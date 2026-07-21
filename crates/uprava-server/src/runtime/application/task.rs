//! Task-oriented isolated sandbox orchestration.

use super::super::*;

const TASK_RUNTIME_CAPABILITY: &str = "task_runtime.opensandbox.docker";
const MAX_TASK_PROMPT_CHARS: usize = 65_536;
const MAX_TASK_CHECKS: usize = 32;
const MAX_TASK_ARTIFACTS: usize = 64;
const MAX_TASK_PATH_CHARS: usize = 1_024;
const MAX_TASK_CHECK_LABEL_CHARS: usize = 256;
const MAX_TASK_CHECK_ARGS: usize = 64;
const MAX_TASK_CHECK_ARG_CHARS: usize = 4_096;
const MAX_TASK_CHECK_COMMAND_CHARS: usize = 16_384;
const MIN_TASK_TIMEOUT_SECONDS: u64 = 60;
const MAX_TASK_TIMEOUT_SECONDS: u64 = 24 * 60 * 60;
const ALLOWED_TASK_CPUS: [&str; 5] = ["0.5", "1", "2", "4", "8"];
const ALLOWED_TASK_MEMORY: [&str; 6] = ["512Mi", "1Gi", "2Gi", "4Gi", "8Gi", "16Gi"];

#[derive(Debug, Default, Deserialize)]
pub(crate) struct TaskRunListQuery {
    project_placement_id: Option<String>,
}

pub(crate) async fn list_task_runs_route(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TaskRunListQuery>,
) -> Result<Json<TaskRunListResponse>, AppError> {
    let rows = sqlx::query(
        r#"
        select tr.*, pp.display_name as placement_name
        from task_runs tr
        join project_placements pp on pp.project_placement_id = tr.project_placement_id
        where (?1 is null or tr.project_placement_id = ?1)
        order by tr.queued_at desc, tr.task_run_id desc
        "#,
    )
    .bind(query.project_placement_id)
    .fetch_all(&state.pool)
    .await?;
    let items = rows
        .iter()
        .map(task_summary_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(TaskRunListResponse { items }))
}

pub(crate) async fn create_task_run_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateTaskRunRequest>,
) -> Result<Json<TaskRunDetail>, AppError> {
    validate_task_request(&request)?;
    let placement = load_placement(&state, &request.project_placement_id).await?;
    ensure_node_commandable(&state, &placement.node_id).await?;
    ensure_placement_startable(&placement)?;
    let advertised_runtime_image =
        ensure_node_supports_task_runtime(&state, &placement.node_id).await?;

    let base_revision = match request.base_revision.as_deref().map(str::trim) {
        Some(revision) => {
            validate_immutable_revision(revision)?;
            revision.to_owned()
        }
        None => placement
            .git_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.commit.clone())
            .filter(|revision| validate_immutable_revision(revision).is_ok())
            .ok_or_else(|| {
                AppError::bad_request(
                    "task_run.base_revision_unavailable",
                    "Placement has no immutable Git HEAD; refresh the resource snapshot or provide a commit",
                )
            })?,
    };
    let task_run_id = TaskRunId::new();
    let branch = format!("uprava/task/{}", task_run_id.as_str());
    let resource_limits = uprava_protocol::TaskResourceLimits {
        cpu: request.resource_limits.cpu.trim().to_owned(),
        memory: request.resource_limits.memory.trim().to_owned(),
    };
    let runtime_image = match request
        .runtime_image
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(requested) if requested != advertised_runtime_image => {
            return Err(AppError::bad_request(
                "task_run.runtime_image_not_advertised",
                "Task runtime image must match the immutable image advertised by the Node",
            ));
        }
        Some(requested) => requested.to_owned(),
        None => advertised_runtime_image,
    };
    let spec = TaskRunSpec {
        task_run_id: task_run_id.clone(),
        project_placement_id: request.project_placement_id.clone(),
        provider: "codex".to_owned(),
        prompt: request.prompt.trim().to_owned(),
        base_revision: base_revision.clone(),
        branch: branch.clone(),
        checks: request.checks.clone(),
        artifact_paths: request.artifact_paths.clone(),
        timeout_seconds: request.timeout_seconds,
        ttl_seconds: request.ttl_seconds,
        resource_limits: resource_limits.clone(),
        runtime_image: runtime_image.clone(),
    };
    let now = Utc::now();
    let command = CommandEnvelope {
        command_id: CommandId::new(),
        kind: CommandKind::RunTask,
        target: CommandTarget::TaskRun {
            node_id: placement.node_id.clone(),
            project_placement_id: request.project_placement_id.clone(),
            task_run_id: task_run_id.clone(),
        },
        actor_ref: ActorRef::local_user(),
        source_refs: vec![UpravaRef::Placement {
            placement_id: request.project_placement_id.clone(),
        }],
        cause_refs: vec![],
        issued_at: now,
        correlation_id: request_correlation_id(&headers),
        payload: CommandPayload::RunTask {
            workspace_path: placement.workspace_path,
            spec: Box::new(spec),
        },
    };

    let mut transaction = state.pool.begin().await?;
    record_command_on_connection(&mut transaction, &command).await?;
    sqlx::query(
        r#"
        insert into task_runs (
            task_run_id, project_placement_id, node_id, provider, state, cleanup_state,
            prompt, base_revision, branch, runtime_image, checks_json, artifact_paths_json,
            timeout_seconds, ttl_seconds, resource_limits_json, command_id, cancel_command_id,
            worktree_path, result_json, summary, terminal_code, terminal_message,
            queued_at, started_at, finished_at, updated_at
        ) values (
            ?1, ?2, ?3, 'codex', 'queued', 'pending', ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, null, null, null, null, null, null, ?14, null, null, ?14
        )
        "#,
    )
    .bind(task_run_id.as_str())
    .bind(request.project_placement_id.as_str())
    .bind(placement.node_id.as_str())
    .bind(request.prompt.trim())
    .bind(&base_revision)
    .bind(&branch)
    .bind(&runtime_image)
    .bind(serde_json::to_string(&request.checks)?)
    .bind(serde_json::to_string(&request.artifact_paths)?)
    .bind(request.timeout_seconds as i64)
    .bind(request.ttl_seconds as i64)
    .bind(serde_json::to_string(&resource_limits)?)
    .bind(command.command_id.as_str())
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    dispatch_pending_commands(&state, &placement.node_id).await?;
    load_task_run(&state, &task_run_id).await.map(Json)
}

pub(crate) async fn task_run_detail_route(
    State(state): State<Arc<AppState>>,
    Path(task_run_id): Path<String>,
) -> Result<Json<TaskRunDetail>, AppError> {
    load_task_run(&state, &TaskRunId::from(task_run_id))
        .await
        .map(Json)
}

pub(crate) async fn cancel_task_run_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(task_run_id): Path<String>,
) -> Result<Json<TaskRunDetail>, AppError> {
    let task_run_id = TaskRunId::from(task_run_id);
    let detail = load_task_run(&state, &task_run_id).await?;
    if detail.task.state.is_terminal() || detail.task.state == TaskRunState::Cancelling {
        return Err(AppError::bad_request(
            "task_run.not_cancellable",
            "Only an active Task Run can be cancelled",
        ));
    }
    let command = CommandEnvelope {
        command_id: CommandId::new(),
        kind: CommandKind::CancelTaskRun,
        target: CommandTarget::TaskRun {
            node_id: detail.task.node_id.clone(),
            project_placement_id: detail.task.project_placement_id.clone(),
            task_run_id: task_run_id.clone(),
        },
        actor_ref: ActorRef::local_user(),
        source_refs: vec![UpravaRef::TaskRun {
            task_run_id: task_run_id.clone(),
        }],
        cause_refs: vec![],
        issued_at: Utc::now(),
        correlation_id: request_correlation_id(&headers),
        payload: CommandPayload::CancelTaskRun {
            task_run_id: task_run_id.clone(),
        },
    };
    let mut transaction = state.pool.begin().await?;
    record_command_on_connection(&mut transaction, &command).await?;
    let updated = sqlx::query(
        "update task_runs set state = 'cancelling', cancel_command_id = ?1, updated_at = ?2 where task_run_id = ?3 and state not in ('succeeded', 'failed', 'cancelled', 'timed_out', 'cancelling')",
    )
    .bind(command.command_id.as_str())
    .bind(command.issued_at)
    .bind(task_run_id.as_str())
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        transaction.rollback().await?;
        return Err(AppError::bad_request(
            "task_run.not_cancellable",
            "Task Run became terminal before cancellation was recorded",
        ));
    }
    transaction.commit().await?;
    dispatch_pending_commands(&state, &detail.task.node_id).await?;
    load_task_run(&state, &task_run_id).await.map(Json)
}

pub(crate) async fn project_task_command_result(
    state: &AppState,
    command_id: &CommandId,
    status: CommandState,
    payload: &JsonValue,
) -> Result<(), AppError> {
    let command_json: Option<String> =
        sqlx::query_scalar("select command_json from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_optional(&state.pool)
            .await?;
    let Some(command_json) = command_json else {
        return Ok(());
    };
    let command: CommandEnvelope = serde_json::from_str(&command_json)?;
    let CommandPayload::RunTask { spec, .. } = command.payload else {
        return Ok(());
    };
    if let Ok(result) = serde_json::from_value::<TaskRunResultPackage>(payload.0.clone()) {
        if status != CommandState::Completed
            || !result.state.is_terminal()
            || result.task_run_id != spec.task_run_id
            || result.base_revision != spec.base_revision
            || result.branch != spec.branch
            || result.runtime_image != spec.runtime_image
            || result
                .final_revision
                .as_deref()
                .is_some_and(|revision| validate_immutable_revision(revision).is_err())
        {
            return Err(AppError::bad_request(
                "task_run.result_contract_mismatch",
                "Task result must be terminal and match the immutable dispatched Task Run spec",
            ));
        }
        sqlx::query(
            r#"
            update task_runs
            set state = ?1, cleanup_state = ?2, worktree_path = ?3, result_json = ?4,
                summary = ?5, terminal_code = ?6, terminal_message = ?7,
                started_at = coalesce(started_at, ?8),
                finished_at = ?8,
                updated_at = ?8
            where task_run_id = ?9
            "#,
        )
        .bind(format_task_state(result.state))
        .bind(format_task_cleanup_state(result.cleanup_state))
        .bind(&result.worktree_path)
        .bind(serde_json::to_string(&result)?)
        .bind(&result.summary)
        .bind(result.terminal_reason.as_ref().map(|reason| &reason.code))
        .bind(
            result
                .terminal_reason
                .as_ref()
                .map(|reason| &reason.message),
        )
        .bind(Utc::now())
        .bind(spec.task_run_id.as_str())
        .execute(&state.pool)
        .await?;
    } else if status == CommandState::Failed {
        let code = payload
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("task_run.execution_failed");
        let message = payload
            .0
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Task Run command failed before evidence was produced");
        sqlx::query(
            "update task_runs set state = 'failed', terminal_code = ?1, terminal_message = ?2, finished_at = ?3, updated_at = ?3 where task_run_id = ?4",
        )
        .bind(code)
        .bind(message)
        .bind(Utc::now())
        .bind(spec.task_run_id.as_str())
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

pub(crate) async fn project_task_event_on_connection(
    connection: &mut SqliteConnection,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    if event.kind != EventKind::TaskRunStateChanged {
        return Ok(());
    }
    let task_run_id = event
        .payload
        .0
        .get("task_run_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::bad_request(
                "task_run.event_missing_id",
                "Task Run state event is missing task_run_id",
            )
        })?;
    let state = event
        .payload
        .0
        .get("state")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::bad_request(
                "task_run.event_missing_state",
                "Task Run state event is missing state",
            )
        })?;
    let cleanup_state = event
        .payload
        .0
        .get("cleanup_state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("pending");
    sqlx::query(
        r#"
        update task_runs
        set state = ?1, cleanup_state = ?2,
            started_at = case when ?1 in ('preparing_workspace', 'starting_runtime', 'running', 'checking', 'collecting_evidence') then coalesce(started_at, ?3) else started_at end,
            updated_at = ?3
        where task_run_id = ?4
          and state not in ('succeeded', 'failed', 'cancelled', 'timed_out')
          and (state <> 'cancelling' or ?1 in ('succeeded', 'failed', 'cancelled', 'timed_out'))
        "#,
    )
    .bind(state)
    .bind(cleanup_state)
    .bind(event.happened_at)
    .bind(task_run_id)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(crate) async fn load_task_run(
    state: &AppState,
    task_run_id: &TaskRunId,
) -> Result<TaskRunDetail, AppError> {
    let row = sqlx::query(
        r#"
        select tr.*, pp.display_name as placement_name
        from task_runs tr
        join project_placements pp on pp.project_placement_id = tr.project_placement_id
        where tr.task_run_id = ?1
        "#,
    )
    .bind(task_run_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("task_run.not_found", "Task Run not found"))?;

    let summary = task_summary_from_row(&row)?;
    let result_json: Option<String> = row.try_get("result_json")?;
    Ok(TaskRunDetail {
        task: summary,
        prompt: row.try_get("prompt")?,
        checks: serde_json::from_str(&row.try_get::<String, _>("checks_json")?)?,
        artifact_paths: serde_json::from_str(&row.try_get::<String, _>("artifact_paths_json")?)?,
        timeout_seconds: row.try_get::<i64, _>("timeout_seconds")? as u64,
        ttl_seconds: row.try_get::<i64, _>("ttl_seconds")? as u64,
        resource_limits: serde_json::from_str(&row.try_get::<String, _>("resource_limits_json")?)?,
        worktree_path: row.try_get("worktree_path")?,
        result: result_json
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
    })
}

fn task_summary_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<TaskRunSummary, AppError> {
    let terminal_code: Option<String> = row.try_get("terminal_code")?;
    let terminal_message: Option<String> = row.try_get("terminal_message")?;
    Ok(TaskRunSummary {
        task_run_id: TaskRunId::from(row.try_get::<String, _>("task_run_id")?),
        project_placement_id: ProjectPlacementId::from(
            row.try_get::<String, _>("project_placement_id")?,
        ),
        placement_name: row.try_get("placement_name")?,
        node_id: NodeId::from(row.try_get::<String, _>("node_id")?),
        provider: row.try_get("provider")?,
        state: parse_task_state(&row.try_get::<String, _>("state")?)?,
        cleanup_state: parse_task_cleanup_state(&row.try_get::<String, _>("cleanup_state")?)?,
        base_revision: row.try_get("base_revision")?,
        branch: row.try_get("branch")?,
        runtime_image: row.try_get("runtime_image")?,
        queued_at: row.try_get("queued_at")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        summary: row.try_get("summary")?,
        terminal_reason: terminal_code
            .zip(terminal_message)
            .map(|(code, message)| ScheduledMessageFailure { code, message }),
    })
}

pub(crate) fn validate_task_request(request: &CreateTaskRunRequest) -> Result<(), AppError> {
    if request.prompt.trim().is_empty() || request.prompt.chars().count() > MAX_TASK_PROMPT_CHARS {
        return Err(AppError::bad_request(
            "task_run.invalid_prompt",
            "Task prompt must be non-empty and within the supported limit",
        ));
    }
    if !(MIN_TASK_TIMEOUT_SECONDS..=MAX_TASK_TIMEOUT_SECONDS).contains(&request.timeout_seconds)
        || request.ttl_seconds < request.timeout_seconds
        || request.ttl_seconds > MAX_TASK_TIMEOUT_SECONDS
    {
        return Err(AppError::bad_request(
            "task_run.invalid_timeout",
            "Task timeout must be 60-86400 seconds and TTL must be at least the timeout",
        ));
    }
    if request.checks.len() > MAX_TASK_CHECKS || request.artifact_paths.len() > MAX_TASK_ARTIFACTS {
        return Err(AppError::bad_request(
            "task_run.too_many_evidence_items",
            "Task Run declares too many checks or artifact paths",
        ));
    }
    for check in &request.checks {
        let command_chars = check.command.chars().count()
            + check
                .args
                .iter()
                .map(|arg| arg.chars().count())
                .sum::<usize>();
        if check.label.trim().is_empty()
            || check.label.chars().count() > MAX_TASK_CHECK_LABEL_CHARS
            || check.command.trim().is_empty()
            || check.args.len() > MAX_TASK_CHECK_ARGS
            || check
                .args
                .iter()
                .any(|arg| arg.chars().count() > MAX_TASK_CHECK_ARG_CHARS)
            || command_chars > MAX_TASK_CHECK_COMMAND_CHARS
            || check.timeout_seconds == 0
            || check.timeout_seconds > request.timeout_seconds
        {
            return Err(AppError::bad_request(
                "task_run.invalid_check",
                "Each task check requires a label, command and bounded timeout",
            ));
        }
    }
    for path in &request.artifact_paths {
        validate_relative_evidence_path(path)?;
    }
    if !ALLOWED_TASK_CPUS.contains(&request.resource_limits.cpu.trim())
        || !ALLOWED_TASK_MEMORY.contains(&request.resource_limits.memory.trim())
    {
        return Err(AppError::bad_request(
            "task_run.invalid_resource_limits",
            "Task limits must use an allowed CPU (0.5-8) and memory (512Mi-16Gi) value",
        ));
    }
    Ok(())
}

fn validate_relative_evidence_path(path: &str) -> Result<(), AppError> {
    let candidate = std::path::Path::new(path);
    let invalid_component = candidate.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    });
    if path.trim().is_empty()
        || path.chars().count() > MAX_TASK_PATH_CHARS
        || candidate.is_absolute()
        || invalid_component
        || path.contains(['*', '?', '[', ']'])
    {
        return Err(AppError::bad_request(
            "task_run.invalid_artifact_path",
            "Task artifact paths must be explicit relative paths without traversal or globs",
        ));
    }
    Ok(())
}

fn validate_immutable_revision(revision: &str) -> Result<(), AppError> {
    if (40..=64).contains(&revision.len()) && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Ok(());
    }
    Err(AppError::bad_request(
        "task_run.mutable_base_revision",
        "Task base_revision must be a full immutable Git commit hash",
    ))
}

async fn ensure_node_supports_task_runtime(
    state: &AppState,
    node_id: &NodeId,
) -> Result<String, AppError> {
    let value_json: Option<String> = sqlx::query_scalar(
        "select value_json from node_capabilities where node_id = ?1 and capability_key = ?2",
    )
    .bind(node_id.as_str())
    .bind(TASK_RUNTIME_CAPABILITY)
    .fetch_optional(&state.pool)
    .await?;
    let runtime = value_json
        .as_deref()
        .map(serde_json::from_str::<CapabilityValue>)
        .transpose()?
        .and_then(|value| match value {
            CapabilityValue::Extension { value, .. }
                if value
                    .0
                    .get("available")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true) =>
            {
                (value.0.get("provider").and_then(serde_json::Value::as_str) == Some("codex"))
                    .then(|| {
                        value
                            .0
                            .get("runtime_image")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|image| !image.is_empty())
                            .map(str::to_owned)
                    })
                    .flatten()
            }
            _ => None,
        });
    if let Some(runtime_image) = runtime {
        return Ok(runtime_image);
    }
    Err(AppError::bad_request(
        "node.task_runtime_unavailable",
        "Node does not advertise an available Docker/OpenSandbox task runtime",
    ))
}

fn parse_task_state(value: &str) -> Result<TaskRunState, AppError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(AppError::from)
}

fn parse_task_cleanup_state(value: &str) -> Result<TaskCleanupState, AppError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(AppError::from)
}

fn format_task_state(value: TaskRunState) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "failed".to_owned())
}

fn format_task_cleanup_state(value: TaskCleanupState) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "failed".to_owned())
}
