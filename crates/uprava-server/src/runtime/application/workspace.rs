//! Placement and workspace application use cases.

use super::super::*;

pub(crate) async fn validate_placement_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreatePlacementRequest>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    validate_placement_with_correlation(&state, request, request_correlation_id(&headers))
        .await
        .map(Json)
}

#[cfg(test)]
pub(crate) async fn validate_placement(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePlacementRequest>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    validate_placement_with_correlation(&state, request, CorrelationId::new())
        .await
        .map(Json)
}

pub(crate) async fn validate_placement_with_correlation(
    state: &AppState,
    request: CreatePlacementRequest,
    correlation_id: CorrelationId,
) -> Result<ProjectPlacementSummary, AppError> {
    if request.display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.display_name_required",
            "Display name is required",
        ));
    }
    let placement_identity =
        PlacementIdentity::try_new(request.node_id.as_str(), request.workspace_path.as_str())
            .map_err(|error| match error {
                domain::PlacementIdentityError::MissingNode => {
                    AppError::bad_request("validation.node_required", "Node identity is required")
                }
                domain::PlacementIdentityError::MissingWorkspace => AppError::bad_request(
                    "validation.workspace_path_required",
                    "Workspace path is required",
                ),
            })?;
    let node_id = NodeId::from(placement_identity.node_id().to_owned());
    ensure_node_commandable(state, &node_id).await?;

    if let Some(existing_placement_id) = sqlx::query_scalar::<_, String>(
        "select project_placement_id from project_placements where node_id = ?1 and workspace_path = ?2",
    )
    .bind(node_id.as_str())
    .bind(placement_identity.workspace_path())
    .fetch_optional(&state.pool)
    .await?
    {
        return load_placement(
            state,
            &ProjectPlacementId::from(existing_placement_id),
        )
        .await;
    }

    let now = Utc::now();
    let project_id = ProjectId::new();
    let placement_id = ProjectPlacementId::new();
    let display_name = request.display_name.trim().to_owned();
    let workspace_path = placement_identity.workspace_path().to_owned();
    let mut placement_transaction = state.pool.begin().await?;
    sqlx::query(
        "delete from deleted_workspace_bindings where node_id = ?1 and workspace_path = ?2",
    )
    .bind(node_id.as_str())
    .bind(&workspace_path)
    .execute(&mut *placement_transaction)
    .await?;
    sqlx::query(
        r#"
        insert into projects (project_id, display_name, repo_id, created_at, updated_at)
        values (?1, ?2, null, ?3, ?3)
        on conflict(project_id) do update set
            display_name = excluded.display_name,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(project_id.as_str())
    .bind(&display_name)
    .bind(now)
    .execute(&mut *placement_transaction)
    .await?;
    let placement_insert = sqlx::query(
        r#"
        insert into project_placements (
            project_placement_id, project_id, node_id, display_name, workspace_path,
            state, resource_badges_json, last_validated_at, created_at, updated_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
        "#,
    )
    .bind(placement_id.as_str())
    .bind(project_id.as_str())
    .bind(node_id.as_str())
    .bind(&display_name)
    .bind(&workspace_path)
    .bind(format_placement_state(PlacementState::Pending))
    .bind(serde_json::to_string(&Vec::<ResourceBadge>::new())?)
    .bind(Option::<DateTime<Utc>>::None)
    .bind(now)
    .execute(&mut *placement_transaction)
    .await;
    if let Err(error) = placement_insert {
        if is_workspace_identity_conflict(&error) {
            placement_transaction.rollback().await?;
            let existing_placement_id: String = sqlx::query_scalar(
                "select project_placement_id from project_placements where node_id = ?1 and workspace_path = ?2",
            )
            .bind(node_id.as_str())
            .bind(&workspace_path)
            .fetch_one(&state.pool)
            .await?;
            return load_placement(state, &ProjectPlacementId::from(existing_placement_id)).await;
        }
        return Err(error.into());
    }
    let command = CommandEnvelope {
        command_id: CommandId::new(),
        kind: CommandKind::ValidateWorkspace,
        target: CommandTarget::Placement {
            node_id,
            project_placement_id: placement_id.clone(),
        },
        actor_ref: ActorRef::local_user(),
        source_refs: vec![UpravaRef::Placement {
            placement_id: placement_id.clone(),
        }],
        cause_refs: vec![],
        issued_at: now,
        correlation_id,
        payload: CommandPayload::ValidateWorkspace {
            display_name,
            workspace_path,
        },
    };
    record_command_on_connection(&mut placement_transaction, &command).await?;
    placement_transaction.commit().await?;
    dispatch_pending_commands(state, command.target.node_id()).await?;

    load_placement(state, &placement_id).await
}

pub(crate) async fn placement_detail(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    load_placement(&state, &ProjectPlacementId::from(placement_id))
        .await
        .map(Json)
}

pub(crate) async fn delete_placement(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
) -> Result<Json<PlacementDeletionResponse>, AppError> {
    let placement_id = ProjectPlacementId::from(placement_id);
    let mut transaction = state.pool.begin().await?;
    let placement_row = sqlx::query(
        "select node_id, workspace_path from project_placements where project_placement_id = ?1",
    )
    .bind(placement_id.as_str())
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(placement_row) = placement_row else {
        return Err(AppError::not_found(
            "placement.not_found",
            "Placement not found",
        ));
    };
    let node_id: String = placement_row.try_get("node_id")?;
    let workspace_path: String = placement_row.try_get("workspace_path")?;
    let now = Utc::now();

    let deleted_sessions = sqlx::query_scalar::<_, i64>(
        "select count(*) from session_threads where project_placement_id = ?1",
    )
    .bind(placement_id.as_str())
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        insert into deleted_workspace_bindings (node_id, workspace_path, deleted_at)
        values (?1, ?2, ?3)
        on conflict(node_id, workspace_path) do update set
            deleted_at = excluded.deleted_at
        "#,
    )
    .bind(&node_id)
    .bind(&workspace_path)
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    for statement in [
        r#"
        delete from events
        where command_id in (
            select command_id
            from commands
            where project_placement_id = ?1
               or session_thread_id in (
                    select session_thread_id
                    from session_threads
                    where project_placement_id = ?1
               )
               or runtime_session_id in (
                    select runtime_session_id
                    from runtime_sessions
                    where session_thread_id in (
                        select session_thread_id
                        from session_threads
                        where project_placement_id = ?1
                    )
               )
        )
           or runtime_session_id in (
                select runtime_session_id
                from runtime_sessions
                where session_thread_id in (
                    select session_thread_id
                    from session_threads
                    where project_placement_id = ?1
                )
           )
           or session_thread_id in (
                select session_thread_id
                from session_threads
                where project_placement_id = ?1
           )
        "#,
        r#"
        delete from warning_acknowledgements
        where session_thread_id in (
            select session_thread_id
            from session_threads
            where project_placement_id = ?1
        )
        "#,
        r#"
        delete from approvals
        where session_thread_id in (
            select session_thread_id
            from session_threads
            where project_placement_id = ?1
        )
        "#,
        r#"
        delete from messages
        where session_thread_id in (
            select session_thread_id
            from session_threads
            where project_placement_id = ?1
        )
        "#,
        r#"
        delete from turns
        where session_thread_id in (
            select session_thread_id
            from session_threads
            where project_placement_id = ?1
        )
        "#,
        "delete from task_runs where project_placement_id = ?1",
        r#"
        delete from commands
        where project_placement_id = ?1
           or session_thread_id in (
                select session_thread_id
                from session_threads
                where project_placement_id = ?1
           )
           or runtime_session_id in (
                select runtime_session_id
                from runtime_sessions
                where session_thread_id in (
                    select session_thread_id
                    from session_threads
                    where project_placement_id = ?1
                )
           )
        "#,
        r#"
        delete from runtime_sessions
        where session_thread_id in (
            select session_thread_id
            from session_threads
            where project_placement_id = ?1
        )
        "#,
        "delete from session_threads where project_placement_id = ?1",
    ] {
        sqlx::query(statement)
            .bind(placement_id.as_str())
            .execute(&mut *transaction)
            .await?;
    }

    let deleted = sqlx::query("delete from project_placements where project_placement_id = ?1")
        .bind(placement_id.as_str())
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    transaction.commit().await?;

    if deleted == 0 {
        return Err(AppError::not_found(
            "placement.not_found",
            "Placement not found",
        ));
    }
    tracing::warn!(deleted_sessions, "placement deleted");
    Ok(Json(PlacementDeletionResponse {
        project_placement_id: placement_id,
        deleted: true,
    }))
}

pub(crate) async fn refresh_resource_snapshot_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    refresh_resource_snapshot_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
pub(crate) async fn refresh_resource_snapshot(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    refresh_resource_snapshot_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

pub(crate) async fn refresh_resource_snapshot_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    correlation_id: CorrelationId,
) -> Result<CommandAcceptedResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_node_commandable(state, &placement.node_id).await?;
    let now = Utc::now();
    let command_id = CommandId::new();

    record_and_dispatch_command(
        state,
        CommandEnvelope {
            command_id: command_id.clone(),
            kind: CommandKind::RefreshResourceSnapshot,
            target: CommandTarget::Placement {
                node_id: placement.node_id.clone(),
                project_placement_id: placement.project_placement_id.clone(),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![UpravaRef::Placement {
                placement_id: placement.project_placement_id.clone(),
            }],
            cause_refs: vec![],
            issued_at: now,
            correlation_id,
            payload: CommandPayload::RefreshResourceSnapshot {
                display_name: placement.display_name,
                workspace_path: placement.workspace_path,
            },
        },
    )
    .await?;

    Ok(CommandAcceptedResponse {
        command_id,
        session: None,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkspacePathQuery {
    pub(crate) path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkspaceHistoryQuery {
    pub(crate) limit: Option<i64>,
}

pub(crate) async fn workspace_tree_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Query(query): Query<WorkspacePathQuery>,
) -> Result<Json<WorkspaceTreeResponse>, AppError> {
    workspace_tree_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        query.path.unwrap_or_else(|| ".".to_owned()),
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_file_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Query(query): Query<WorkspacePathQuery>,
) -> Result<Json<WorkspaceFileContentResponse>, AppError> {
    workspace_file_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        query.path.unwrap_or_else(|| ".".to_owned()),
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_file_write_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Json(request): Json<WorkspaceFileWriteRequest>,
) -> Result<Json<WorkspaceFileWriteResponse>, AppError> {
    workspace_file_write_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_command_run_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Json(request): Json<WorkspaceCommandRunRequest>,
) -> Result<Json<WorkspaceCommandRunResponse>, AppError> {
    workspace_command_run_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_command_accept_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Json(request): Json<WorkspaceCommandRunRequest>,
) -> Result<(StatusCode, Json<CommandAcceptedResponse>), AppError> {
    let placement = load_placement(&state, &ProjectPlacementId::from(placement_id)).await?;
    ensure_placement_intervention_allowed(&placement)?;
    ensure_node_commandable(&state, &placement.node_id).await?;
    let command_id = CommandId::new();
    let project_placement_id = placement.project_placement_id.clone();
    record_and_dispatch_command(
        &state,
        CommandEnvelope {
            command_id: command_id.clone(),
            kind: CommandKind::RunWorkspaceCommand,
            target: CommandTarget::Placement {
                node_id: placement.node_id,
                project_placement_id: project_placement_id.clone(),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![UpravaRef::Workspace {
                placement_id: project_placement_id,
            }],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: request_correlation_id(&headers),
            payload: CommandPayload::RunWorkspaceCommand {
                workspace_path: placement.workspace_path,
                request,
            },
        },
    )
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(CommandAcceptedResponse {
            command_id,
            session: None,
        }),
    ))
}

pub(crate) async fn workspace_command_resource_route(
    State(state): State<Arc<AppState>>,
    Path((placement_id, command_id)): Path<(String, String)>,
) -> Result<axum::response::Response, AppError> {
    let placement = load_placement(&state, &ProjectPlacementId::from(placement_id)).await?;
    ensure_placement_inspectable(&placement)?;
    let (status, item) =
        load_workspace_command_resource(&state, &placement, &CommandId::from(command_id), true)
            .await?;
    Ok((status, Json(item)).into_response())
}

pub(crate) async fn workspace_command_cancel_route(
    State(state): State<Arc<AppState>>,
    Path((placement_id, command_id)): Path<(String, String)>,
) -> Result<Json<WorkspaceCommandHistoryItem>, AppError> {
    let placement = load_placement(&state, &ProjectPlacementId::from(placement_id)).await?;
    ensure_placement_intervention_allowed(&placement)?;
    let command_id = CommandId::from(command_id);
    let _ = load_workspace_command_resource(&state, &placement, &command_id, false).await?;
    mark_command_terminal_if_nonterminal(
        &state,
        &command_id,
        CommandState::Expired,
        &JsonValue(json!({
            "error_code": "workspace.command_cancelled",
            "message": "Workspace command was cancelled by the client"
        })),
        Utc::now(),
    )
    .await?;
    let (_status, item) =
        load_workspace_command_resource(&state, &placement, &command_id, false).await?;
    Ok(Json(item))
}

pub(crate) async fn load_workspace_command_resource(
    state: &AppState,
    placement: &ProjectPlacementSummary,
    command_id: &CommandId,
    expire_if_due: bool,
) -> Result<(StatusCode, WorkspaceCommandHistoryItem), AppError> {
    let row = sqlx::query(
        r#"
        select command_id, kind, state, payload_json, result_payload_json,
               created_at, completed_at
        from commands
        where command_id = ?1 and project_placement_id = ?2
        "#,
    )
    .bind(command_id.as_str())
    .bind(placement.project_placement_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::not_found("workspace.command_not_found", "Workspace command not found")
    })?;
    let state_value: String = row.try_get("state")?;
    let mut item = WorkspaceCommandHistoryItem {
        command_id: CommandId::from(row.try_get::<String, _>("command_id")?),
        kind: parse_command_kind(&row.try_get::<String, _>("kind")?),
        state: parse_command_state(&state_value),
        created_at: row.try_get("created_at")?,
        completed_at: row.try_get("completed_at")?,
        payload: serde_json::from_str(&row.try_get::<String, _>("payload_json")?)?,
        result_payload: row
            .try_get::<Option<String>, _>("result_payload_json")?
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?,
    };
    if expire_if_due {
        if let Some((completed_at, result_payload)) =
            expire_workspace_command_if_due(state, &item).await?
        {
            item.state = CommandState::Expired;
            item.completed_at = Some(completed_at);
            item.result_payload = Some(result_payload);
        }
    }
    let status = if is_terminal_command_state(item.state) {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    Ok((status, item))
}

pub(crate) async fn expire_workspace_command_if_due(
    state: &AppState,
    item: &WorkspaceCommandHistoryItem,
) -> Result<Option<(DateTime<Utc>, JsonValue)>, AppError> {
    if is_terminal_command_state(item.state) {
        return Ok(None);
    }
    let timeout_seconds = item
        .payload
        .0
        .get("request")
        .and_then(|request| request.get("timeout_seconds"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(WORKSPACE_INTERVENTION_TIMEOUT.as_secs() as i64)
        .clamp(1, WORKSPACE_INTERVENTION_TIMEOUT.as_secs() as i64);
    let Some(deadline) = item
        .created_at
        .checked_add_signed(chrono::Duration::seconds(timeout_seconds + 10))
    else {
        return Ok(None);
    };
    let now = Utc::now();
    if now < deadline {
        return Ok(None);
    }
    let result_payload = JsonValue(json!({
        "error_code": "workspace.command_expired",
        "message": "Workspace command expired before a terminal node result arrived"
    }));
    if mark_command_terminal_if_nonterminal(
        state,
        &item.command_id,
        CommandState::Expired,
        &result_payload,
        now,
    )
    .await?
    {
        return Ok(Some((now, result_payload)));
    }
    Ok(None)
}

pub(crate) async fn mark_command_terminal_if_nonterminal(
    state: &AppState,
    command_id: &CommandId,
    command_state: CommandState,
    result_payload: &JsonValue,
    completed_at: DateTime<Utc>,
) -> Result<bool, AppError> {
    debug_assert!(is_terminal_command_state(command_state));
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        r#"
        update commands
        set state = ?1,
            completed_at = coalesce(completed_at, ?2),
            result_payload_json = coalesce(result_payload_json, ?3)
        where command_id = ?4
          and state not in ('completed', 'failed', 'blocked', 'expired')
        "#,
    )
    .bind(format_command_state(command_state))
    .bind(completed_at)
    .bind(serde_json::to_string(result_payload)?)
    .bind(command_id.as_str())
    .execute(&mut *transaction)
    .await?
    .rows_affected()
        > 0;
    if updated {
        sqlx::query("delete from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    if updated {
        let notice = CommandResultNotice {
            command_id: command_id.clone(),
            status: command_state,
            payload: result_payload.clone(),
        };
        let _ = state.command_result_tx.send(notice.clone());
        let waiter = {
            let mut waiters = lock_command_waiters(state)?;
            waiters.remove(command_id.as_str())
        };
        if let Some(waiter) = waiter {
            let _ = waiter.send(notice);
        }
    }
    Ok(updated)
}

pub(crate) async fn workspace_diff_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Query(request): Query<WorkspaceDiffRequest>,
) -> Result<Json<WorkspaceDiffResponse>, AppError> {
    workspace_diff_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_review_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Query(request): Query<WorkspaceDiffRequest>,
) -> Result<Json<WorkspaceReviewProjection>, AppError> {
    let placement_id = ProjectPlacementId::from(placement_id);
    let correlation_id = request_correlation_id(&headers);
    let (diff, checks) = tokio::try_join!(
        workspace_diff_with_correlation(&state, placement_id.clone(), request, correlation_id,),
        workspace_check_history(&state, placement_id.clone(), Some(20)),
    )?;
    Ok(Json(WorkspaceReviewProjection {
        placement_id,
        git_snapshot: diff.git_snapshot.clone(),
        diff,
        checks,
        generated_at: Utc::now(),
    }))
}

pub(crate) async fn workspace_command_history_route(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
    Query(query): Query<WorkspaceHistoryQuery>,
) -> Result<Json<WorkspaceCommandHistoryResponse>, AppError> {
    workspace_command_history(&state, ProjectPlacementId::from(placement_id), query.limit)
        .await
        .map(Json)
}

pub(crate) async fn workspace_terminal_open_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(placement_id): Path<String>,
    Json(request): Json<WorkspaceTerminalOpenRequest>,
) -> Result<Json<WorkspaceTerminalOpenResponse>, AppError> {
    workspace_terminal_open_with_correlation(
        &state,
        ProjectPlacementId::from(placement_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn workspace_terminal_list_route(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
) -> Result<Json<WorkspaceTerminalListResponse>, AppError> {
    workspace_terminal_list(&state, ProjectPlacementId::from(placement_id))
        .await
        .map(Json)
}

pub(crate) async fn workspace_terminal_stream_route(
    State(state): State<Arc<AppState>>,
    Path((placement_id, terminal_id)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let placement_id = ProjectPlacementId::from(placement_id);
    let terminal_id = TerminalId::from(terminal_id);
    let placement = load_placement(&state, &placement_id).await?;
    ensure_placement_intervention_allowed(&placement)?;
    ensure_node_commandable(&state, &placement.node_id).await?;
    ensure_terminal_belongs_to_placement(&state, &terminal_id, &placement_id).await?;
    Ok(ws.on_upgrade(move |socket| {
        handle_workspace_terminal_stream(state, placement.node_id, terminal_id, socket)
    }))
}

pub(crate) async fn workspace_tree_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    path: String,
    correlation_id: CorrelationId,
) -> Result<WorkspaceTreeResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    dispatch_workspace_command(
        state,
        &placement,
        CommandKind::ListWorkspaceTree,
        json!({
            "path": path,
        }),
        vec![UpravaRef::Workspace {
            placement_id: placement.project_placement_id.clone(),
        }],
        correlation_id,
        WORKSPACE_COMMAND_TIMEOUT,
    )
    .await
}

pub(crate) async fn workspace_file_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    path: String,
    correlation_id: CorrelationId,
) -> Result<WorkspaceFileContentResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    dispatch_workspace_command(
        state,
        &placement,
        CommandKind::ReadWorkspaceFile,
        json!({
            "path": path,
        }),
        vec![
            UpravaRef::Workspace {
                placement_id: placement.project_placement_id.clone(),
            },
            UpravaRef::File {
                placement_id: placement.project_placement_id.clone(),
                path,
                version: None,
            },
        ],
        correlation_id,
        WORKSPACE_COMMAND_TIMEOUT,
    )
    .await
}

pub(crate) async fn workspace_file_write_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    request: WorkspaceFileWriteRequest,
    correlation_id: CorrelationId,
) -> Result<WorkspaceFileWriteResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_intervention_allowed(&placement)?;
    let path = request.path.clone();
    dispatch_workspace_command(
        state,
        &placement,
        CommandKind::WriteWorkspaceFile,
        serde_json::to_value(request)?,
        vec![
            UpravaRef::Workspace {
                placement_id: placement.project_placement_id.clone(),
            },
            UpravaRef::File {
                placement_id: placement.project_placement_id.clone(),
                path,
                version: None,
            },
        ],
        correlation_id,
        WORKSPACE_INTERVENTION_TIMEOUT,
    )
    .await
}

pub(crate) async fn workspace_command_run_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    request: WorkspaceCommandRunRequest,
    correlation_id: CorrelationId,
) -> Result<WorkspaceCommandRunResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_intervention_allowed(&placement)?;
    dispatch_workspace_command(
        state,
        &placement,
        CommandKind::RunWorkspaceCommand,
        serde_json::to_value(request)?,
        vec![UpravaRef::Workspace {
            placement_id: placement.project_placement_id.clone(),
        }],
        correlation_id,
        WORKSPACE_INTERVENTION_TIMEOUT,
    )
    .await
}

pub(crate) async fn workspace_diff_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    request: WorkspaceDiffRequest,
    correlation_id: CorrelationId,
) -> Result<WorkspaceDiffResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_inspectable(&placement)?;
    let response = dispatch_workspace_command(
        state,
        &placement,
        CommandKind::ReadWorkspaceDiff,
        serde_json::to_value(request)?,
        vec![UpravaRef::Workspace {
            placement_id: placement.project_placement_id.clone(),
        }],
        correlation_id,
        WORKSPACE_INTERVENTION_TIMEOUT,
    )
    .await?;
    let response: WorkspaceDiffResponse = response;
    sqlx::query(
        "update project_placements set git_snapshot_json = ?1, updated_at = ?2 where project_placement_id = ?3",
    )
    .bind(serde_json::to_string(&response.git_snapshot)?)
    .bind(response.git_snapshot.generated_at)
    .bind(placement.project_placement_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(response)
}

pub(crate) async fn workspace_terminal_open_with_correlation(
    state: &AppState,
    placement_id: ProjectPlacementId,
    request: WorkspaceTerminalOpenRequest,
    correlation_id: CorrelationId,
) -> Result<WorkspaceTerminalOpenResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_intervention_allowed(&placement)?;
    let response = dispatch_workspace_command(
        state,
        &placement,
        CommandKind::OpenWorkspaceTerminal,
        serde_json::to_value(request)?,
        vec![UpravaRef::Workspace {
            placement_id: placement.project_placement_id.clone(),
        }],
        correlation_id,
        WORKSPACE_INTERVENTION_TIMEOUT,
    )
    .await?;
    let response: WorkspaceTerminalOpenResponse = response;
    state.workspace_terminals.write().await.insert(
        response.terminal.terminal_id.to_string(),
        response.terminal.clone(),
    );
    state
        .core_metrics
        .pty_opened
        .fetch_add(1, Ordering::Relaxed);
    Ok(response)
}

pub(crate) async fn workspace_terminal_list(
    state: &AppState,
    placement_id: ProjectPlacementId,
) -> Result<WorkspaceTerminalListResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_inspectable(&placement)?;
    let mut terminals = state
        .workspace_terminals
        .read()
        .await
        .values()
        .filter(|terminal| terminal.placement_id == placement.project_placement_id)
        .cloned()
        .collect::<Vec<_>>();
    terminals.sort_by_key(|terminal| std::cmp::Reverse(terminal.created_at));
    Ok(WorkspaceTerminalListResponse {
        placement_id: placement.project_placement_id,
        terminals,
        generated_at: Utc::now(),
    })
}

pub(crate) async fn ensure_terminal_belongs_to_placement(
    state: &AppState,
    terminal_id: &TerminalId,
    placement_id: &ProjectPlacementId,
) -> Result<(), AppError> {
    let terminals = state.workspace_terminals.read().await;
    let Some(terminal) = terminals.get(terminal_id.as_str()) else {
        return Err(AppError::not_found(
            "workspace_terminal.not_found",
            "Workspace terminal was not found",
        ));
    };
    if terminal.placement_id != *placement_id {
        return Err(AppError::bad_request(
            "workspace_terminal.placement_mismatch",
            "Workspace terminal does not belong to this placement",
        ));
    }
    Ok(())
}

pub(crate) async fn workspace_command_history(
    state: &AppState,
    placement_id: ProjectPlacementId,
    limit: Option<i64>,
) -> Result<WorkspaceCommandHistoryResponse, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_inspectable(&placement)?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let rows = sqlx::query(
        r#"
        select command_id, kind, state, payload_json, result_payload_json, created_at, completed_at
        from commands
        where project_placement_id = ?1
          and kind in ('WriteWorkspaceFile', 'RunWorkspaceCommand', 'ReadWorkspaceDiff')
        order by created_at desc
        limit ?2
        "#,
    )
    .bind(placement.project_placement_id.as_str())
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    let mut commands = Vec::with_capacity(rows.len());
    for row in rows {
        let kind: String = row.try_get("kind")?;
        let state_value: String = row.try_get("state")?;
        let payload_json: String = row.try_get("payload_json")?;
        let result_payload_json: Option<String> = row.try_get("result_payload_json")?;
        commands.push(WorkspaceCommandHistoryItem {
            command_id: CommandId::from(row.try_get::<String, _>("command_id")?),
            kind: parse_command_kind(&kind),
            state: parse_command_state(&state_value),
            created_at: row.try_get("created_at")?,
            completed_at: row.try_get("completed_at")?,
            payload: serde_json::from_str(&payload_json)?,
            result_payload: result_payload_json
                .map(|payload| serde_json::from_str(&payload))
                .transpose()?,
        });
    }
    Ok(WorkspaceCommandHistoryResponse {
        placement_id: placement.project_placement_id,
        commands,
        generated_at: Utc::now(),
    })
}

pub(crate) async fn workspace_check_history(
    state: &AppState,
    placement_id: ProjectPlacementId,
    limit: Option<i64>,
) -> Result<Vec<WorkspaceCheckRunSummary>, AppError> {
    let placement = load_placement(state, &placement_id).await?;
    ensure_placement_inspectable(&placement)?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let rows = sqlx::query(
        r#"
        select command_id, state, payload_json, result_payload_json, created_at, completed_at
        from commands
        where project_placement_id = ?1
          and kind = 'RunWorkspaceCommand'
          and json_extract(payload_json, '$.request.intent') = 'check'
        order by created_at desc
        limit ?2
        "#,
    )
    .bind(placement.project_placement_id.as_str())
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    let mut checks = Vec::new();
    for row in rows {
        let payload: CommandPayload =
            serde_json::from_str(&row.try_get::<String, _>("payload_json")?)?;
        let CommandPayload::RunWorkspaceCommand { request, .. } = payload else {
            continue;
        };
        if request.intent != WorkspaceCommandIntent::Check {
            continue;
        }
        let result = row
            .try_get::<Option<String>, _>("result_payload_json")?
            .as_deref()
            .and_then(|raw| serde_json::from_str::<WorkspaceCommandRunResponse>(raw).ok());
        checks.push(WorkspaceCheckRunSummary {
            command_id: CommandId::from(row.try_get::<String, _>("command_id")?),
            state: parse_command_state(&row.try_get::<String, _>("state")?),
            command: request.command,
            args: request.args,
            label: request.label,
            success: result.as_ref().map(|result| result.success),
            exit_code: result.as_ref().and_then(|result| result.exit_code),
            stdout: result.as_ref().map(|result| result.stdout.clone()),
            stderr: result.as_ref().map(|result| result.stderr.clone()),
            stdout_truncated: result
                .as_ref()
                .is_some_and(|result| result.stdout_truncated),
            stderr_truncated: result
                .as_ref()
                .is_some_and(|result| result.stderr_truncated),
            duration_ms: result.as_ref().map(|result| result.duration_ms),
            created_at: row.try_get("created_at")?,
            completed_at: row.try_get("completed_at")?,
        });
    }
    Ok(checks)
}

pub(crate) async fn dispatch_workspace_command<T>(
    state: &AppState,
    placement: &ProjectPlacementSummary,
    kind: CommandKind,
    payload: serde_json::Value,
    source_refs: Vec<UpravaRef>,
    correlation_id: CorrelationId,
    timeout: Duration,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    ensure_placement_inspectable(placement)?;
    ensure_node_commandable(state, &placement.node_id).await?;
    let command_id = CommandId::new();
    let (result_sender, result_receiver) = oneshot::channel();
    lock_command_waiters(state)?.insert(command_id.to_string(), result_sender);
    let _waiter_guard = CommandWaiterGuard::new(state, command_id.clone());
    let payload = typed_workspace_command_payload(kind, placement.workspace_path.clone(), payload)?;
    record_and_dispatch_command(
        state,
        CommandEnvelope {
            command_id: command_id.clone(),
            kind,
            target: CommandTarget::Placement {
                node_id: placement.node_id.clone(),
                project_placement_id: placement.project_placement_id.clone(),
            },
            actor_ref: ActorRef::local_user(),
            source_refs,
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id,
            payload,
        },
    )
    .await?;
    let result = wait_for_command_result(state, result_receiver, &command_id, timeout).await?;
    if result.status == CommandState::Completed {
        return serde_json::from_value::<T>(result.payload.0).map_err(AppError::from);
    }
    Err(workspace_command_failed(result))
}

pub(crate) fn typed_workspace_command_payload(
    kind: CommandKind,
    workspace_path: String,
    payload: serde_json::Value,
) -> Result<CommandPayload, AppError> {
    let path = || {
        payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::bad_request(
                    "workspace.path_required",
                    "Workspace command requires a path",
                )
            })
    };
    match kind {
        CommandKind::ListWorkspaceTree => Ok(CommandPayload::ListWorkspaceTree {
            workspace_path,
            path: path()?,
        }),
        CommandKind::ReadWorkspaceFile => Ok(CommandPayload::ReadWorkspaceFile {
            workspace_path,
            path: path()?,
        }),
        CommandKind::WriteWorkspaceFile => Ok(CommandPayload::WriteWorkspaceFile {
            workspace_path,
            request: serde_json::from_value(payload)?,
        }),
        CommandKind::RunWorkspaceCommand => Ok(CommandPayload::RunWorkspaceCommand {
            workspace_path,
            request: serde_json::from_value(payload)?,
        }),
        CommandKind::ReadWorkspaceDiff => Ok(CommandPayload::ReadWorkspaceDiff {
            workspace_path,
            request: serde_json::from_value(payload)?,
        }),
        CommandKind::OpenWorkspaceTerminal => Ok(CommandPayload::OpenWorkspaceTerminal {
            workspace_path,
            request: serde_json::from_value(payload)?,
        }),
        _ => Err(AppError::bad_request(
            "workspace.command_kind_invalid",
            "Command kind is not a workspace request",
        )),
    }
}

pub(crate) fn ensure_placement_inspectable(
    placement: &ProjectPlacementSummary,
) -> Result<(), AppError> {
    if matches!(
        placement.state,
        PlacementState::Validated | PlacementState::ReadOnly
    ) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "placement.not_inspectable",
        "Workspace placement is not ready for inspection",
    ))
}

pub(crate) fn ensure_placement_intervention_allowed(
    placement: &ProjectPlacementSummary,
) -> Result<(), AppError> {
    if placement.state == PlacementState::Validated {
        return Ok(());
    }
    Err(AppError::bad_request(
        "placement.not_writable",
        "Workspace placement is not writable",
    ))
}

pub(crate) async fn wait_for_command_result(
    state: &AppState,
    result_receiver: oneshot::Receiver<CommandResultNotice>,
    command_id: &CommandId,
    timeout_duration: Duration,
) -> Result<CommandResultNotice, AppError> {
    match tokio::time::timeout(timeout_duration, result_receiver).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => {
            if let Some(result) = load_durable_command_result(state, command_id).await? {
                return Ok(result);
            }
            Err(AppError::bad_request(
                "workspace.command_result_unavailable",
                "Workspace command result channel closed",
            ))
        }
        Err(_) => {
            if let Some(result) = load_durable_command_result(state, command_id).await? {
                return Ok(result);
            }
            Err(AppError::bad_request(
                "workspace.command_timeout",
                "Timed out waiting for the node workspace inspector",
            ))
        }
    }
}

pub(crate) fn is_workspace_identity_conflict(error: &sqlx::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("unique constraint")
        && (message.contains("project_placements") || message.contains("node_id, workspace_path"))
}

pub(crate) async fn load_durable_command_result(
    state: &AppState,
    command_id: &CommandId,
) -> Result<Option<CommandResultNotice>, AppError> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("select state, result_payload_json from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_optional(&state.pool)
            .await?;
    let Some((state_value, payload)) = row else {
        return Ok(None);
    };
    let status = parse_command_state(&state_value);
    if !matches!(
        status,
        CommandState::Completed
            | CommandState::Failed
            | CommandState::Blocked
            | CommandState::Expired
    ) {
        return Ok(None);
    }
    let payload = payload
        .map(|value| serde_json::from_str(&value))
        .transpose()?
        .unwrap_or_else(|| JsonValue(json!({})));
    Ok(Some(CommandResultNotice {
        command_id: command_id.clone(),
        status,
        payload,
    }))
}

pub(crate) fn workspace_command_failed(result: CommandResultNotice) -> AppError {
    let node_code = result
        .payload
        .0
        .get("error_code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("workspace.command_failed");
    let message = result
        .payload
        .0
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Node workspace inspector command failed");
    AppError::bad_request(
        "workspace.command_failed",
        format!("{message} ({node_code})"),
    )
}
