use super::*;

fn task_capability() -> CapabilitySummary {
    CapabilitySummary {
        key: "task_runtime.opensandbox.docker".to_owned(),
        value: CapabilityValue::Extension {
            name: "task_runtime".to_owned(),
            value: JsonValue(serde_json::json!({
                "available": true,
                "configured": true,
                "backend": "opensandbox",
                "mode": "docker",
                "provider": "codex",
                "runtime_image": "uprava/codex-runtime:test"
            })),
        },
    }
}

fn create_request(placement_id: ProjectPlacementId) -> CreateTaskRunRequest {
    CreateTaskRunRequest {
        project_placement_id: placement_id,
        prompt: "Implement the bounded change and run checks".to_owned(),
        base_revision: Some("a".repeat(40)),
        checks: vec![uprava_protocol::TaskCheckSpec {
            label: "make c".to_owned(),
            command: "make".to_owned(),
            args: vec!["c".to_owned()],
            timeout_seconds: 300,
        }],
        artifact_paths: vec!["coverage/report.json".to_owned()],
        timeout_seconds: 3_600,
        ttl_seconds: 7_200,
        resource_limits: uprava_protocol::TaskResourceLimits::default(),
        runtime_image: None,
    }
}

#[tokio::test]
async fn task_run_records_a_distinct_command_without_creating_a_session() {
    let state = test_state().await;
    let (node_id, session, workspace_path) = create_test_session(&state).await;
    set_node_capabilities(&state, &node_id, vec![task_capability()]).await;
    let sessions_before: i64 = sqlx::query_scalar("select count(*) from session_threads")
        .fetch_one(&state.pool)
        .await
        .expect("session count loads");

    let created = create_task_run_route(
        State(state.clone()),
        HeaderMap::new(),
        Json(create_request(
            session.placement.project_placement_id.clone(),
        )),
    )
    .await
    .expect("Task Run creates")
    .0;
    let sessions_after: i64 = sqlx::query_scalar("select count(*) from session_threads")
        .fetch_one(&state.pool)
        .await
        .expect("session count reloads");
    let command_json: String = sqlx::query_scalar(
        "select command_json from commands where command_id = (select command_id from task_runs where task_run_id = ?1)",
    )
    .bind(created.task.task_run_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("task command loads");
    let command: CommandEnvelope =
        serde_json::from_str(&command_json).expect("task command decodes");

    assert_eq!(sessions_before, sessions_after);
    assert_eq!(created.task.state, TaskRunState::Queued);
    assert_eq!(created.task.runtime_image, "uprava/codex-runtime:test");
    assert_eq!(command.kind, CommandKind::RunTask);
    assert_eq!(
        command.target.task_run_id(),
        Some(&created.task.task_run_id)
    );
    let _ = delete_node(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("Node with Task Run history deletes");
    let retained_tasks: i64 = sqlx::query_scalar("select count(*) from task_runs")
        .fetch_one(&state.pool)
        .await
        .expect("Task Run count loads after Node deletion");
    assert_eq!(retained_tasks, 0);
    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn structured_task_result_projects_outcome_and_cleanup_independently() {
    let state = test_state().await;
    let (node_id, session, workspace_path) = create_test_session(&state).await;
    set_node_capabilities(
        &state,
        &node_id,
        vec![
            CapabilitySummary {
                key: "provider.codex".to_owned(),
                value: CapabilityValue::provider(true),
            },
            task_capability(),
        ],
    )
    .await;
    let created = create_task_run_route(
        State(state.clone()),
        HeaderMap::new(),
        Json(create_request(
            session.placement.project_placement_id.clone(),
        )),
    )
    .await
    .expect("Task Run creates")
    .0;
    let command_id: String =
        sqlx::query_scalar("select command_id from task_runs where task_run_id = ?1")
            .bind(created.task.task_run_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command id loads");
    let result = TaskRunResultPackage {
        task_run_id: created.task.task_run_id.clone(),
        state: TaskRunState::Succeeded,
        cleanup_state: TaskCleanupState::Failed,
        summary: "Implementation complete".to_owned(),
        base_revision: created.task.base_revision.clone(),
        final_revision: Some("b".repeat(40)),
        branch: created.task.branch.clone(),
        worktree_path: "/repo/.uprava/runs/task".to_owned(),
        runtime_image: created.task.runtime_image.clone(),
        diff: "diff --git a/a b/a".to_owned(),
        diff_truncated: false,
        checks: vec![],
        artifacts: vec![],
        unresolved_risks: vec!["sandbox cleanup requires reconciliation".to_owned()],
        terminal_reason: None,
    };
    let mut mismatched = result.clone();
    mismatched.task_run_id = TaskRunId::from("task-run-spoofed");
    let error = project_task_command_result(
        &state,
        &CommandId::from(command_id.clone()),
        CommandState::Completed,
        &JsonValue(serde_json::to_value(mismatched).expect("mismatched result serializes")),
    )
    .await
    .expect_err("result for another Task Run rejects");
    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "task_run.result_contract_mismatch",
            ..
        }
    ));
    project_task_command_result(
        &state,
        &CommandId::from(command_id),
        CommandState::Completed,
        &JsonValue(serde_json::to_value(result).expect("result serializes")),
    )
    .await
    .expect("result projects");
    let projected = load_task_run(&state, &created.task.task_run_id)
        .await
        .expect("Task Run reloads");

    assert_eq!(projected.task.state, TaskRunState::Succeeded);
    assert_eq!(projected.task.cleanup_state, TaskCleanupState::Failed);
    assert_eq!(
        projected.task.summary.as_deref(),
        Some("Implementation complete")
    );
    assert!(projected.result.is_some());
    let _ = delete_placement(
        State(state.clone()),
        Path(session.placement.project_placement_id.to_string()),
    )
    .await
    .expect("Placement with Task Run history deletes");
    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
}

#[test]
fn task_request_accepts_only_bounded_resource_presets() {
    let mut request = create_request(ProjectPlacementId::from("placement-limits"));
    validate_task_request(&request).expect("default resource limits validate");

    request.resource_limits.cpu = "32".to_owned();
    let error = validate_task_request(&request).expect_err("unbounded CPU rejects");
    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "task_run.invalid_resource_limits",
            ..
        }
    ));

    request.resource_limits.cpu = "2".to_owned();
    request.resource_limits.memory = "128Gi".to_owned();
    let error = validate_task_request(&request).expect_err("unbounded memory rejects");
    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "task_run.invalid_resource_limits",
            ..
        }
    ));
}

#[tokio::test]
async fn cancelling_task_does_not_regress_on_late_nonterminal_events() {
    let state = test_state().await;
    let (node_id, session, workspace_path) = create_test_session(&state).await;
    set_node_capabilities(
        &state,
        &node_id,
        vec![
            CapabilitySummary {
                key: "provider.codex".to_owned(),
                value: CapabilityValue::provider(true),
            },
            task_capability(),
        ],
    )
    .await;
    let created = create_task_run_route(
        State(state.clone()),
        HeaderMap::new(),
        Json(create_request(
            session.placement.project_placement_id.clone(),
        )),
    )
    .await
    .expect("Task Run creates")
    .0;
    let cancelling = cancel_task_run_route(
        State(state.clone()),
        HeaderMap::new(),
        Path(created.task.task_run_id.to_string()),
    )
    .await
    .expect("Task Run cancellation records")
    .0;
    assert_eq!(cancelling.task.state, TaskRunState::Cancelling);

    for (sequence, task_state) in [(1, TaskRunState::Running), (2, TaskRunState::Cancelled)] {
        let event = EventEnvelope {
            event_id: EventId::from(format!("task-cancel-event-{sequence}")),
            command_id: None,
            correlation_id: None,
            actor_ref: ActorRef::Node {
                node_id: node_id.clone(),
            },
            scope_ref: ScopeRef::TaskRun {
                task_run_id: created.task.task_run_id.clone(),
            },
            node_id: Some(node_id.clone()),
            runtime_session_id: None,
            session_thread_id: None,
            turn_id: None,
            seq: sequence,
            session_projection_seq: None,
            kind: EventKind::TaskRunStateChanged,
            happened_at: Utc::now(),
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: EventPayload::from_json(
                EventKind::TaskRunStateChanged,
                serde_json::json!({
                    "task_run_id": created.task.task_run_id.clone(),
                    "state": task_state,
                    "cleanup_state": "pending"
                }),
            ),
        };
        let mut connection = state.pool.acquire().await.expect("connection acquires");
        project_task_event_on_connection(&mut connection, &event)
            .await
            .expect("task event projects");
        drop(connection);
        let projected = load_task_run(&state, &created.task.task_run_id)
            .await
            .expect("Task Run reloads");
        let expected = if task_state == TaskRunState::Running {
            TaskRunState::Cancelling
        } else {
            TaskRunState::Cancelled
        };
        assert_eq!(projected.task.state, expected);
    }
    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
}
