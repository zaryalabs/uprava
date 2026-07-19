//! Runtime lifecycle, command dispatch and atomic event-ingest orchestration.

use super::super::*;

pub(crate) async fn resolve_approval_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((session_thread_id, approval_id)): Path<(String, String)>,
    Json(request): Json<ResolveApprovalRequest>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    resolve_approval_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        ApprovalId::from(approval_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
pub(crate) async fn resolve_approval(
    State(state): State<Arc<AppState>>,
    Path((session_thread_id, approval_id)): Path<(String, String)>,
    Json(request): Json<ResolveApprovalRequest>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    resolve_approval_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        ApprovalId::from(approval_id),
        request,
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

pub(crate) async fn resolve_approval_with_correlation(
    state: &AppState,
    session_id: SessionThreadId,
    approval_id: ApprovalId,
    request: ResolveApprovalRequest,
    correlation_id: CorrelationId,
) -> Result<CommandAcceptedResponse, AppError> {
    let detail = load_session_detail(state, &session_id).await?;
    ensure_pending_approval(&detail, &approval_id)?;
    ensure_session_commandable(state, &detail, CommandKind::ResolveApproval).await?;
    let command_id = CommandId::new();

    record_and_dispatch_command(
        state,
        CommandEnvelope {
            command_id: command_id.clone(),
            kind: CommandKind::ResolveApproval,
            target: CommandTarget::SessionRuntime {
                node_id: detail.placement.node_id.clone(),
                project_placement_id: detail.placement.project_placement_id.clone(),
                session_thread_id: session_id.clone(),
                runtime_session_id: detail.session.runtime.runtime_session_id.clone(),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![UpravaRef::Approval {
                approval_id: approval_id.clone(),
            }],
            cause_refs: vec![UpravaRef::Session {
                session_thread_id: session_id.clone(),
            }],
            issued_at: Utc::now(),
            correlation_id,
            payload: CommandPayload::ResolveApproval {
                approval_id,
                approved: request.approved,
                message: request.message,
            },
        },
    )
    .await?;

    let session = load_session_detail(state, &session_id).await?;
    Ok(CommandAcceptedResponse {
        command_id,
        session: Some(session),
    })
}

pub(crate) async fn acknowledge_warning_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((session_thread_id, warning_kind)): Path<(String, String)>,
    Json(request): Json<AcknowledgeWarningRequest>,
) -> Result<Json<WarningAcknowledgementResponse>, AppError> {
    acknowledge_warning_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        warning_kind,
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
pub(crate) async fn acknowledge_warning(
    State(state): State<Arc<AppState>>,
    Path((session_thread_id, warning_kind)): Path<(String, String)>,
    Json(request): Json<AcknowledgeWarningRequest>,
) -> Result<Json<WarningAcknowledgementResponse>, AppError> {
    acknowledge_warning_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        warning_kind,
        request,
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

pub(crate) async fn acknowledge_warning_with_correlation(
    state: &AppState,
    session_id: SessionThreadId,
    warning_kind: String,
    request: AcknowledgeWarningRequest,
    correlation_id: CorrelationId,
) -> Result<WarningAcknowledgementResponse, AppError> {
    if warning_kind.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.warning_kind_required",
            "Warning kind is required",
        ));
    }
    let detail = load_session_detail(state, &session_id).await?;
    let acknowledged = acknowledged_warning_kinds(state, &session_id).await?;
    let active_warnings =
        active_warnings(&detail.placement, &detail.session.runtime, &acknowledged);
    if !active_warnings
        .iter()
        .any(|warning| warning.kind == warning_kind)
    {
        return Err(AppError::bad_request(
            "warning.not_active",
            "Warning is not currently active for this session",
        ));
    }

    let event = record_warning_acknowledgement(
        state,
        &detail,
        warning_kind,
        request.message,
        correlation_id,
    )
    .await?;
    let session = load_session_detail(state, &session_id).await?;
    Ok(WarningAcknowledgementResponse {
        event_id: event.event_id,
        session,
    })
}

pub(crate) async fn interrupt_runtime_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(runtime_session_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    lifecycle_command(
        &state,
        RuntimeSessionId::from(runtime_session_id),
        CommandKind::InterruptRuntime,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
pub(crate) async fn interrupt_runtime(
    State(state): State<Arc<AppState>>,
    Path(runtime_session_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    lifecycle_command(
        &state,
        RuntimeSessionId::from(runtime_session_id),
        CommandKind::InterruptRuntime,
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

pub(crate) async fn stop_runtime_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(runtime_session_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    lifecycle_command(
        &state,
        RuntimeSessionId::from(runtime_session_id),
        CommandKind::StopRuntime,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

pub(crate) async fn resume_runtime_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(runtime_session_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    lifecycle_command(
        &state,
        RuntimeSessionId::from(runtime_session_id),
        CommandKind::ResumeRuntime,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
pub(crate) async fn resume_runtime(
    State(state): State<Arc<AppState>>,
    Path(runtime_session_id): Path<String>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    lifecycle_command(
        &state,
        RuntimeSessionId::from(runtime_session_id),
        CommandKind::ResumeRuntime,
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

pub(crate) async fn lifecycle_command(
    state: &AppState,
    runtime_session_id: RuntimeSessionId,
    kind: CommandKind,
    correlation_id: CorrelationId,
) -> Result<CommandAcceptedResponse, AppError> {
    let session_id = find_session_for_runtime(state, &runtime_session_id).await?;
    let detail = load_session_detail(state, &session_id).await?;
    ensure_session_commandable(state, &detail, kind).await?;
    let command_id = CommandId::new();
    let payload = lifecycle_command_payload(state, &detail, kind).await?;
    record_and_dispatch_command(
        state,
        CommandEnvelope {
            command_id: command_id.clone(),
            kind,
            target: CommandTarget::SessionRuntime {
                node_id: detail.placement.node_id.clone(),
                project_placement_id: detail.placement.project_placement_id.clone(),
                session_thread_id: session_id.clone(),
                runtime_session_id: runtime_session_id.clone(),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id,
            payload,
        },
    )
    .await?;

    Ok(CommandAcceptedResponse {
        command_id,
        session: Some(load_session_detail(state, &session_id).await?),
    })
}

pub(crate) async fn lifecycle_command_payload(
    state: &AppState,
    detail: &SessionDetail,
    kind: CommandKind,
) -> Result<CommandPayload, AppError> {
    if kind == CommandKind::InterruptRuntime {
        return Ok(CommandPayload::InterruptRuntime);
    }
    if kind == CommandKind::StopRuntime {
        return Ok(CommandPayload::StopRuntime);
    }
    if kind != CommandKind::ResumeRuntime {
        return Err(AppError::bad_request(
            "runtime.command_kind_invalid",
            "Unsupported runtime lifecycle command",
        ));
    }

    let provider_resume_ref =
        runtime_provider_resume_ref_json(state, &detail.session.runtime.runtime_session_id).await?;
    Ok(CommandPayload::ResumeRuntime {
        provider: detail.session.runtime.provider.clone(),
        workspace_path: detail.placement.workspace_path.clone(),
        provider_resume_ref: provider_resume_ref.map(JsonValue),
    })
}

pub(crate) async fn runtime_provider_resume_ref_json(
    state: &AppState,
    runtime_session_id: &RuntimeSessionId,
) -> Result<Option<serde_json::Value>, AppError> {
    let provider_resume_ref_json: Option<String> = sqlx::query_scalar(
        "select provider_resume_ref_json from runtime_sessions where runtime_session_id = ?1",
    )
    .bind(runtime_session_id.as_str())
    .fetch_optional(&state.pool)
    .await?;
    let Some(provider_resume_ref_json) = provider_resume_ref_json else {
        return Ok(None);
    };
    let provider_resume_ref_json = provider_resume_ref_json.trim();
    if provider_resume_ref_json.is_empty() {
        return Ok(None);
    }
    let value = serde_json::from_str::<serde_json::Value>(provider_resume_ref_json)?;
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(value))
}

pub(crate) async fn record_and_dispatch_command(
    state: &AppState,
    command: CommandEnvelope,
) -> Result<(), AppError> {
    let command_id = command.command_id.clone();
    let target_node_id = command.target.node_id().clone();
    record_command(state, command).await?;
    update_command_state(state, &command_id, CommandState::PendingDispatch).await?;
    dispatch_pending_commands(state, &target_node_id).await
}

pub(crate) async fn dispatch_pending_commands(
    state: &AppState,
    node_id: &NodeId,
) -> Result<(), AppError> {
    let rows = sqlx::query(
        r#"
        select o.command_id, o.command_json
        from command_dispatch_outbox o
        join commands c on c.command_id = o.command_id
        where o.target_node_id = ?1
          and c.state in ('recorded', 'pending_dispatch', 'dispatched', 'acknowledged')
        order by o.enqueued_at asc, o.command_id asc
        "#,
    )
    .bind(node_id.as_str())
    .fetch_all(&state.pool)
    .await?;
    let pending_count = rows.len();
    let Some(channel) = state.control_connections.sender(node_id).await else {
        if pending_count > 0 {
            tracing::debug!(
                pending_count,
                "pending commands waiting for node control channel"
            );
        }
        return Ok(());
    };

    for row in rows {
        let command_id = CommandId::from(row.try_get::<String, _>("command_id")?);
        let command_json: String = row.try_get("command_json")?;
        let command = serde_json::from_str::<CommandEnvelope>(&command_json)?;
        let command_kind = command.kind;
        let correlation_id = command.correlation_id.clone();
        if channel
            .try_send(ControlFrame::CommandDispatch {
                frame_id: Uuid::new_v4().to_string(),
                protocol_version: API_VERSION.to_owned(),
                sent_at: Utc::now(),
                command: Box::new(command),
            })
            .is_ok()
        {
            sqlx::query(
                "update command_dispatch_outbox set attempts = attempts + 1, last_attempt_at = ?1 where command_id = ?2",
            )
            .bind(Utc::now())
            .bind(command_id.as_str())
            .execute(&state.pool)
            .await?;
            tracing::info!(
                command_kind = ?command_kind,
                correlation_id = %correlation_id,
                "command dispatched"
            );
            update_command_state(state, &command_id, CommandState::Dispatched).await?;
        } else {
            tracing::warn!(
                command_kind = ?command_kind,
                correlation_id = %correlation_id,
                "command dispatch channel closed"
            );
        }
    }
    Ok(())
}

pub(crate) async fn should_open_control_channel(
    state: &AppState,
    node_id: &NodeId,
) -> Result<bool, AppError> {
    if state.control_connections.contains(node_id).await {
        return Ok(false);
    }
    let pending: i64 = sqlx::query_scalar(
        "select count(*) from commands where target_node_id = ?1 and state in ('recorded', 'pending_dispatch', 'dispatched', 'acknowledged')",
    )
    .bind(node_id.as_str())
    .fetch_one(&state.pool)
    .await?;
    Ok(pending > 0)
}

pub(crate) async fn update_command_state(
    state: &AppState,
    command_id: &CommandId,
    command_state: CommandState,
) -> Result<(), AppError> {
    let completed_at = matches!(
        command_state,
        CommandState::Completed
            | CommandState::Failed
            | CommandState::Blocked
            | CommandState::Expired
    )
    .then(Utc::now);
    let mut transaction = state.pool.begin().await?;
    sqlx::query("update commands set state = ?1, completed_at = coalesce(?2, completed_at) where command_id = ?3")
        .bind(format_command_state(command_state))
        .bind(completed_at)
        .bind(command_id.as_str())
        .execute(&mut *transaction)
        .await?;
    if completed_at.is_some() {
        sqlx::query("delete from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    tracing::debug!(
        command_state = ?command_state,
        "command state updated"
    );
    Ok(())
}

pub(crate) async fn update_command_result(
    state: &AppState,
    command_id: &CommandId,
    command_state: CommandState,
    result_payload: &JsonValue,
) -> Result<(), AppError> {
    let completed_at = matches!(
        command_state,
        CommandState::Completed
            | CommandState::Failed
            | CommandState::Blocked
            | CommandState::Expired
    )
    .then(Utc::now);
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        r#"
        update commands
        set state = ?1,
            completed_at = coalesce(?2, completed_at),
            result_payload_json = ?3
        where command_id = ?4
        "#,
    )
    .bind(format_command_state(command_state))
    .bind(completed_at)
    .bind(serde_json::to_string(result_payload)?)
    .bind(command_id.as_str())
    .execute(&mut *transaction)
    .await?;
    if matches!(
        command_state,
        CommandState::Completed
            | CommandState::Failed
            | CommandState::Blocked
            | CommandState::Expired
    ) {
        sqlx::query("delete from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    tracing::debug!(
        command_state = ?command_state,
        "command result stored"
    );
    Ok(())
}

pub(crate) async fn accept_node_event(
    state: &AppState,
    event: EventEnvelope,
) -> Result<(), AppError> {
    if !event.payload.matches_kind(event.kind) {
        return Err(AppError::bad_request(
            "protocol.event_payload_mismatch",
            "Event payload does not match its event kind",
        ));
    }
    // SQLite serializes the durable unit of work; this lock also keeps cursor
    // allocation deterministic before the connection-level projection API is
    // moved into a dedicated module.
    let _ingest_guard = state.event_ingest_lock.lock().await;
    let mut event = event;
    let mut transaction = state.pool.begin().await?;
    let existing_event: Option<(String, String)> =
        sqlx::query_as("select projection_state, event_json from events where event_id = ?1")
            .bind(event.event_id.as_str())
            .fetch_optional(&mut *transaction)
            .await?;
    if let Some((projection_state, persisted_event_json)) = existing_event {
        let persisted_event: EventEnvelope = serde_json::from_str(&persisted_event_json)?;
        if projection_state == "projected" {
            tracing::debug!(
                event_kind = ?&persisted_event.kind,
                seq = persisted_event.seq,
                "duplicate projected node event ignored"
            );
            enqueue_event_publication_on_connection(&mut transaction, &persisted_event).await?;
            transaction.commit().await?;
            drain_event_publication_outbox(state).await?;
            return Ok(());
        }
        sqlx::query(
            "update events set projection_attempts = projection_attempts + 1 where event_id = ?1",
        )
        .bind(event.event_id.as_str())
        .execute(&mut *transaction)
        .await?;
        apply_event_projection_on_connection(&mut transaction, &persisted_event).await?;
        complete_event_projection_on_connection(&mut transaction, &persisted_event).await?;
        transaction.commit().await?;
        state
            .core_metrics
            .accepted_events
            .fetch_add(1, Ordering::Relaxed);
        record_event_metrics(state, &persisted_event);
        drain_event_publication_outbox(state).await?;
        return Ok(());
    }
    if sqlx::query_scalar::<_, i64>("select count(*) from events where event_id = ?1")
        .bind(event.event_id.as_str())
        .fetch_one(&mut *transaction)
        .await?
        > 0
    {
        tracing::debug!(
            event_kind = ?&event.kind,
            seq = event.seq,
            "duplicate node event ignored"
        );
        return Ok(());
    }
    let scope_key = scope_key(&event.scope_ref);
    let seq_conflict: Option<String> =
        sqlx::query_scalar("select event_id from events where scope_key = ?1 and seq = ?2")
            .bind(&scope_key)
            .bind(event.seq)
            .fetch_optional(&mut *transaction)
            .await?;
    if let Some(conflict) = seq_conflict {
        tracing::warn!(
            event_kind = ?&event.kind,
            seq = event.seq,
            "event sequence conflict rejected"
        );
        return Err(AppError::bad_request(
            "event.seq_conflict",
            format!("Event seq conflicts with {conflict}"),
        ));
    }
    let max_seq: Option<i64> =
        sqlx::query_scalar("select max(seq) from events where scope_key = ?1")
            .bind(&scope_key)
            .fetch_one(&mut *transaction)
            .await?;
    let expected_seq = max_seq.unwrap_or(0) + 1;
    let stream_gap = (event.seq > expected_seq).then_some(expected_seq);
    if let Some(expected_seq) = stream_gap {
        tracing::warn!(
            event_kind = ?&event.kind,
            expected_seq,
            received_seq = event.seq,
            "event stream gap detected"
        );
    }
    if event.correlation_id.is_none() {
        event.correlation_id =
            command_correlation_id_on_connection(&mut transaction, event.command_id.as_ref())
                .await?;
    }
    if let Some(session_id) = event.session_thread_id.clone() {
        event.session_projection_seq =
            Some(next_session_projection_seq_on_connection(&mut transaction, &session_id).await?);
    } else {
        event.session_projection_seq = None;
    }
    upsert_actor_on_connection(&mut transaction, &event.actor_ref, event.happened_at).await?;
    insert_event_record_on_connection(&mut transaction, &scope_key, &event).await?;
    apply_event_projection_on_connection(&mut transaction, &event).await?;
    if let Some(expected_seq) = stream_gap {
        mark_event_stream_gap_on_connection(&mut transaction, &event, expected_seq).await?;
    }
    complete_event_projection_on_connection(&mut transaction, &event).await?;
    transaction.commit().await?;
    state
        .core_metrics
        .accepted_events
        .fetch_add(1, Ordering::Relaxed);
    record_event_metrics(state, &event);
    drain_event_publication_outbox(state).await?;
    log_event_appended(&event, stream_gap);
    Ok(())
}

pub(crate) fn record_event_metrics(state: &AppState, event: &EventEnvelope) {
    let is_truncation = event.kind == EventKind::ProviderActivity
        && event
            .payload
            .0
            .get("provider_event_type")
            .and_then(serde_json::Value::as_str)
            == Some("output_truncated");
    if is_truncation {
        state
            .core_metrics
            .provider_truncations
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
pub(crate) async fn insert_event_record(
    state: &AppState,
    scope_key: &str,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    let mut transaction = state.pool.begin().await?;
    insert_event_record_on_connection(&mut transaction, scope_key, event).await?;
    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn insert_event_record_on_connection(
    connection: &mut SqliteConnection,
    scope_key: &str,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        insert into events (
            event_id, scope_key, seq, kind, node_id, runtime_session_id,
            session_thread_id, session_projection_seq, command_id, actor_ref_json,
            scope_ref_json, correlation_id, source_refs_json, evidence_refs_json,
            cause_refs_json, result_refs_json, payload_json, event_json, happened_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
    )
    .bind(event.event_id.as_str())
    .bind(scope_key)
    .bind(event.seq)
    .bind(format!("{:?}", event.kind))
    .bind(event.node_id.as_ref().map(NodeId::as_str))
    .bind(
        event
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str),
    )
    .bind(
        event
            .session_thread_id
            .as_ref()
            .map(SessionThreadId::as_str),
    )
    .bind(event.session_projection_seq)
    .bind(event.command_id.as_ref().map(CommandId::as_str))
    .bind(serde_json::to_string(&event.actor_ref)?)
    .bind(serde_json::to_string(&event.scope_ref)?)
    .bind(event.correlation_id.as_ref().map(CorrelationId::as_str))
    .bind(serde_json::to_string(&event.source_refs)?)
    .bind(serde_json::to_string(&event.evidence_refs)?)
    .bind(serde_json::to_string(&event.cause_refs)?)
    .bind(serde_json::to_string(&event.result_refs)?)
    .bind(serde_json::to_string(&event.payload)?)
    .bind(serde_json::to_string(event)?)
    .bind(event.happened_at)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn complete_event_projection(
    state: &AppState,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    let mut transaction = state.pool.begin().await?;
    complete_event_projection_on_connection(&mut transaction, event).await?;
    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn complete_event_projection_on_connection(
    connection: &mut SqliteConnection,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    sqlx::query(
        "update events set projection_state = 'projected', projected_at = ?2 where event_id = ?1 and projection_state <> 'projected'",
    )
    .bind(event.event_id.as_str())
    .bind(event.happened_at)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "insert into event_publication_outbox (event_id, event_json, enqueued_at) values (?1, ?2, ?3) on conflict(event_id) do nothing",
    )
    .bind(event.event_id.as_str())
    .bind(serde_json::to_string(event)?)
    .bind(event.happened_at)
    .execute(&mut *connection)
    .await?;
    let durable_message = match event.kind {
        EventKind::ProviderMessageCompleted => Some((
            MessageRole::Assistant,
            event
                .payload
                .0
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Provider completed a message"),
        )),
        EventKind::RuntimeError => Some((
            MessageRole::Runtime,
            event
                .payload
                .0
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Runtime error"),
        )),
        EventKind::ApprovalRequested => Some((
            MessageRole::Approval,
            event
                .payload
                .0
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Approval requested"),
        )),
        EventKind::ApprovalResolved => Some((
            MessageRole::Approval,
            event
                .payload
                .0
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Approval resolved"),
        )),
        _ => None,
    };
    if let (Some((role, content)), Some(session_thread_id)) =
        (durable_message, event.session_thread_id.as_ref())
    {
        let message_id = MessageId::new();
        sqlx::query(
            r#"
                insert into messages (
                    message_id, session_thread_id, turn_id, role, content,
                    created_at, completed_at, source_event_id
                )
                values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                on conflict do nothing
                "#,
        )
        .bind(message_id.as_str())
        .bind(session_thread_id.as_str())
        .bind(event.turn_id.as_ref().map(TurnId::as_str))
        .bind(format_message_role(role))
        .bind(content)
        .bind(event.happened_at)
        .bind(event.happened_at)
        .bind(event.event_id.as_str())
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub(crate) async fn next_session_projection_seq(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<i64, AppError> {
    let max_seq: Option<i64> = sqlx::query_scalar(
        "select max(session_projection_seq) from events where session_thread_id = ?1",
    )
    .bind(session_id.as_str())
    .fetch_one(&state.pool)
    .await?;
    Ok(max_seq.unwrap_or(0) + 1)
}

pub(crate) async fn next_session_projection_seq_on_connection(
    connection: &mut SqliteConnection,
    session_id: &SessionThreadId,
) -> Result<i64, AppError> {
    let max_seq: Option<i64> = sqlx::query_scalar(
        "select max(session_projection_seq) from events where session_thread_id = ?1",
    )
    .bind(session_id.as_str())
    .fetch_one(&mut *connection)
    .await?;
    Ok(max_seq.unwrap_or(0) + 1)
}

pub(crate) async fn command_correlation_id_on_connection(
    connection: &mut SqliteConnection,
    command_id: Option<&CommandId>,
) -> Result<Option<CorrelationId>, AppError> {
    let Some(command_id) = command_id else {
        return Ok(None);
    };
    let command_json: Option<String> =
        sqlx::query_scalar("select command_json from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_optional(&mut *connection)
            .await?;
    command_json
        .map(|command_json| serde_json::from_str::<CommandEnvelope>(&command_json))
        .transpose()
        .map(|command| command.map(|command| command.correlation_id))
        .map_err(AppError::from)
}

pub(crate) async fn apply_event_projection_on_connection(
    connection: &mut SqliteConnection,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    if let Some(runtime_session_id) = &event.runtime_session_id {
        touch_runtime_step_on_connection(connection, runtime_session_id, event.happened_at).await?;
    }
    update_turn_from_event_on_connection(connection, event).await?;
    update_approval_from_event_on_connection(connection, event).await?;

    match event.kind {
        EventKind::RuntimeStarting => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Starting,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::RuntimeReady => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Ready,
                    event.happened_at,
                )
                .await?;
                update_runtime_provider_resume_ref_on_connection(
                    connection,
                    runtime_session_id,
                    event,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::RuntimeResuming => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Resuming,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::RuntimeRunning => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Running,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::RuntimeBlocked => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Blocked,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::RuntimeExpired => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Expired,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Degraded,
            )
            .await?;
        }
        EventKind::RuntimeStopped => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Stopped,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Stopped,
            )
            .await?;
        }
        EventKind::RuntimeError => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Error,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Degraded,
            )
            .await?;
        }
        EventKind::TurnInterrupted => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Interrupted,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::ProviderMessageCompleted => {}
        EventKind::ApprovalRequested => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state_on_connection(
                    connection,
                    runtime_session_id,
                    RuntimeSessionState::Blocked,
                    event.happened_at,
                )
                .await?;
            }
            update_session_state_from_event_on_connection(
                connection,
                event,
                SessionThreadState::Active,
            )
            .await?;
        }
        EventKind::ApprovalResolved => {}
        EventKind::WorkspaceValidated | EventKind::ResourceSnapshotUpdated => {
            update_placement_from_workspace_event_on_connection(connection, event).await?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) async fn update_placement_from_workspace_event_on_connection(
    connection: &mut SqliteConnection,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    let placement_id = match &event.scope_ref {
        ScopeRef::Placement {
            project_placement_id,
        } => project_placement_id.clone(),
        _ => event
            .payload
            .0
            .get("placement_id")
            .and_then(serde_json::Value::as_str)
            .map(ProjectPlacementId::from)
            .ok_or_else(|| {
                AppError::bad_request(
                    "placement.missing_ref",
                    "Workspace validation event is missing placement ref",
                )
            })?,
    };
    let state_value = event
        .payload
        .0
        .get("state")
        .and_then(serde_json::Value::as_str)
        .map(parse_placement_state)
        .unwrap_or(PlacementState::Validated);
    let resource_badges = event
        .payload
        .0
        .get("resource_badges")
        .cloned()
        .map(serde_json::from_value::<Vec<ResourceBadge>>)
        .transpose()?
        .unwrap_or_default();
    let git_snapshot = event
        .payload
        .0
        .get("git_snapshot")
        .cloned()
        .map(serde_json::from_value::<Option<GitWorkspaceSnapshot>>)
        .transpose()?
        .flatten();

    sqlx::query(
        r#"
        update project_placements
        set display_name = coalesce(?1, display_name),
            workspace_path = coalesce(?2, workspace_path),
            state = ?3,
            resource_badges_json = ?4,
            git_snapshot_json = ?5,
            last_validated_at = ?6,
            updated_at = ?6
        where project_placement_id = ?7
        "#,
    )
    .bind(
        event
            .payload
            .0
            .get("display_name")
            .and_then(serde_json::Value::as_str),
    )
    .bind(
        event
            .payload
            .0
            .get("workspace_path")
            .and_then(serde_json::Value::as_str),
    )
    .bind(format_placement_state(state_value))
    .bind(serde_json::to_string(&resource_badges)?)
    .bind(
        git_snapshot
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?,
    )
    .bind(event.happened_at)
    .bind(placement_id.as_str())
    .execute(&mut *connection)
    .await?;

    Ok(())
}

pub(crate) async fn record_warning_acknowledgement(
    state: &AppState,
    detail: &SessionDetail,
    warning_kind: String,
    message: Option<String>,
    correlation_id: CorrelationId,
) -> Result<EventEnvelope, AppError> {
    let scope_ref = ScopeRef::Session {
        session_thread_id: detail.session.session_thread_id.clone(),
    };
    let scope_key = scope_key(&scope_ref);
    let seq = next_seq(state, &scope_key).await?;
    let happened_at = Utc::now();
    let affected_refs = vec![
        UpravaRef::Warning {
            warning_kind: warning_kind.clone(),
            command_id: None,
        },
        UpravaRef::Session {
            session_thread_id: detail.session.session_thread_id.clone(),
        },
        UpravaRef::Placement {
            placement_id: detail.placement.project_placement_id.clone(),
        },
    ];
    let event = EventEnvelope {
        event_id: EventId::new(),
        command_id: None,
        correlation_id: Some(correlation_id),
        actor_ref: ActorRef::local_user(),
        scope_ref,
        node_id: Some(detail.placement.node_id.clone()),
        runtime_session_id: Some(detail.session.runtime.runtime_session_id.clone()),
        session_thread_id: Some(detail.session.session_thread_id.clone()),
        turn_id: None,
        seq,
        session_projection_seq: Some(
            next_session_projection_seq(state, &detail.session.session_thread_id).await?,
        ),
        kind: EventKind::CoordinationWarningAcknowledged,
        happened_at,
        source_refs: vec![UpravaRef::Warning {
            warning_kind: warning_kind.clone(),
            command_id: None,
        }],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: affected_refs.clone(),
        payload: EventPayload::from_json(
            EventKind::CoordinationWarningAcknowledged,
            json!({
                "warning_kind": warning_kind,
                "message": message,
                "affected_refs": affected_refs,
            }),
        ),
    };

    let mut transaction = state.pool.begin().await?;
    upsert_actor_on_connection(&mut transaction, &event.actor_ref, event.happened_at).await?;
    insert_event_record_on_connection(&mut transaction, &scope_key, &event).await?;

    sqlx::query(
        r#"
        insert into warning_acknowledgements (
            event_id, session_thread_id, actor_ref_json, warning_kind,
            command_id, affected_refs_json, acknowledged_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(event.event_id.as_str())
    .bind(detail.session.session_thread_id.as_str())
    .bind(serde_json::to_string(&event.actor_ref)?)
    .bind(
        event
            .payload
            .0
            .get("warning_kind")
            .and_then(serde_json::Value::as_str),
    )
    .bind(event.command_id.as_ref().map(CommandId::as_str))
    .bind(serde_json::to_string(&event.result_refs)?)
    .bind(event.happened_at)
    .execute(&mut *transaction)
    .await?;
    complete_event_projection_on_connection(&mut transaction, &event).await?;
    transaction.commit().await?;
    drain_event_publication_outbox(state).await?;
    log_event_appended(&event, None);
    Ok(event)
}
