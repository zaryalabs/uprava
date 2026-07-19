use super::*;

#[test]
fn stream_resume_after_seq_uses_last_event_id_header_when_query_is_absent() {
    let mut headers = HeaderMap::new();
    headers.insert("last-event-id", "7".parse().expect("header value parses"));

    assert_eq!(
        stream_resume_after_seq(&EventsQuery { after_seq: None }, &headers),
        7
    );
}

#[test]
fn stream_resume_after_seq_prefers_query_cursor_over_last_event_id() {
    let mut headers = HeaderMap::new();
    headers.insert("last-event-id", "7".parse().expect("header value parses"));

    assert_eq!(
        stream_resume_after_seq(&EventsQuery { after_seq: Some(3) }, &headers),
        3
    );
}

#[tokio::test]
async fn session_events_endpoint_resumes_after_cursor() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "cursor-provider-delta-1",
            1,
            EventKind::ProviderOutputDelta,
            json!({ "delta": "first" }),
        ),
    )
    .await
    .expect("first event accepts");
    let expected_event_id = EventId::from("cursor-provider-completed-2");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            expected_event_id.as_str(),
            2,
            EventKind::ProviderMessageCompleted,
            json!({ "content": "second" }),
        ),
    )
    .await
    .expect("second event accepts");
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/sessions/{}/events?after_seq=1",
                    detail.session.session_thread_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let events: Vec<EventEnvelope> = serde_json::from_slice(&body).expect("events response parses");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, expected_event_id);
    assert_eq!(events[0].seq, 2);
}

#[tokio::test]
async fn session_events_endpoint_uses_projection_cursor_across_scopes() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "cursor-runtime-raw-5",
            5,
            EventKind::ProviderOutputDelta,
            json!({ "delta": "runtime" }),
        ),
    )
    .await
    .expect("runtime event accepts");
    let session_event = append_event(
        &state,
        NewEvent {
            command_id: None,
            actor_ref: ActorRef::System,
            scope_ref: ScopeRef::Session {
                session_thread_id: detail.session.session_thread_id.clone(),
            },
            node_id: None,
            runtime_session_id: Some(detail.session.runtime.runtime_session_id.clone()),
            session_thread_id: Some(detail.session.session_thread_id.clone()),
            turn_id: None,
            kind: EventKind::CoordinationWarningAcknowledged,
            payload: json!({ "warning_kind": "runtime_degraded" }),
        },
    )
    .await
    .expect("session event appends");
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/sessions/{}/events?after_seq=1",
                    detail.session.session_thread_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let events: Vec<EventEnvelope> = serde_json::from_slice(&body).expect("events response parses");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, session_event.event_id);
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[0].session_projection_seq, Some(2));
}

#[tokio::test]
async fn node_event_sequence_gap_marks_session_and_runtime_degraded() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "provider-gap-1",
        2,
        EventKind::ProviderOutputDelta,
        json!({ "delta": "late event" }),
    );

    accept_node_event(&state, event)
        .await
        .expect("gap event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detail.session.state, SessionThreadState::Degraded);
    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Stale);
    assert_eq!(
        detail.session.runtime.degraded_reason.as_deref(),
        Some("event sequence gap: expected 1, received 2")
    );
}

#[tokio::test]
async fn node_event_payload_mismatch_leaves_no_durable_effect() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let mut event = node_event_fixture(
        &detail,
        node_id,
        "workspace-projection-failure",
        1,
        EventKind::WorkspaceValidated,
        json!({ "state": "validated" }),
    );
    event.scope_ref = ScopeRef::Unknown {
        scope: "missing-placement".to_owned(),
    };
    event.payload = EventPayload::from_json(
        EventKind::RuntimeError,
        json!({ "code": "invalid", "message": "invalid payload kind" }),
    );
    let runtime_id = detail.session.runtime.runtime_session_id.clone();
    let before_step: Option<DateTime<Utc>> = sqlx::query_scalar(
        "select last_runtime_step_at from runtime_sessions where runtime_session_id = ?1",
    )
    .bind(runtime_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("runtime step loads before failure");

    let error = accept_node_event(&state, event.clone())
        .await
        .expect_err("mismatched workspace payload fails");
    let durable_state: (i64, i64, Option<DateTime<Utc>>) = sqlx::query_as(
        r#"
            select
                (select count(*) from events where event_id = ?1),
                (select count(*) from event_publication_outbox where event_id = ?1),
                (select last_runtime_step_at from runtime_sessions where runtime_session_id = ?2)
            "#,
    )
    .bind(event.event_id.as_str())
    .bind(runtime_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("durable state loads after rollback");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "protocol.event_payload_mismatch",
            ..
        }
    ));
    assert_eq!(durable_state, (0, 0, before_step));
}

#[tokio::test]
async fn injected_failure_at_each_event_stage_leaves_no_partial_event() {
    let stages = [
        ("actor", "insert", "actors", EventKind::ProviderActivity),
        ("event", "insert", "events", EventKind::ProviderActivity),
        (
            "runtime",
            "update",
            "runtime_sessions",
            EventKind::RuntimeStarting,
        ),
        (
            "approval",
            "insert",
            "approvals",
            EventKind::ApprovalRequested,
        ),
        (
            "placement",
            "update",
            "project_placements",
            EventKind::WorkspaceValidated,
        ),
        (
            "publication",
            "insert",
            "event_publication_outbox",
            EventKind::ProviderActivity,
        ),
        (
            "message",
            "insert",
            "messages",
            EventKind::ProviderMessageCompleted,
        ),
    ];

    for (stage, operation, table, kind) in stages {
        let state = test_state().await;
        let (node_id, detail, workspace_path) = create_test_session(&state).await;
        let payload = match kind {
            EventKind::ApprovalRequested => json!({
                "approval_id": format!("failure-{stage}"),
                "prompt": "expected failure"
            }),
            EventKind::WorkspaceValidated => json!({
                "placement_id": detail.placement.project_placement_id.as_str(),
                "state": "validated",
                "resource_badges": []
            }),
            EventKind::ProviderMessageCompleted => json!({ "content": "expected failure" }),
            _ => json!({}),
        };
        let mut event = node_event_fixture(
            &detail,
            node_id,
            &format!("failure-{stage}"),
            0,
            kind,
            payload,
        );
        if matches!(event.kind, EventKind::WorkspaceValidated) {
            event.scope_ref = ScopeRef::Placement {
                project_placement_id: detail.placement.project_placement_id.clone(),
            };
        }
        event.seq = next_seq(&state, &scope_key(&event.scope_ref))
            .await
            .expect("failure event sequence allocates");
        let trigger = format!(
                "create temp trigger fail_projection_stage before {operation} on {table} begin select raise(abort, 'injected {stage} failure'); end"
            );
        sqlx::query(&trigger)
            .execute(&state.pool)
            .await
            .unwrap_or_else(|error| panic!("{stage} trigger installs: {error}"));

        assert!(
            accept_node_event(&state, event.clone()).await.is_err(),
            "{stage} failure was not injected"
        );
        let durable_counts: (i64, i64) = sqlx::query_as(
            r#"
                select
                    (select count(*) from events where event_id = ?1),
                    (select count(*) from event_publication_outbox where event_id = ?1)
                "#,
        )
        .bind(event.event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .unwrap_or_else(|error| panic!("{stage} durable counts load: {error}"));
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert_eq!(durable_counts, (0, 0), "stage {stage} left partial state");
    }
}

#[tokio::test]
async fn every_node_event_kind_commits_projection_and_publication_together() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event_kinds = [
        EventKind::RuntimeStarting,
        EventKind::RuntimeReady,
        EventKind::RuntimeRunning,
        EventKind::RuntimeBlocked,
        EventKind::RuntimeExpired,
        EventKind::RuntimeResuming,
        EventKind::RuntimeStopped,
        EventKind::RuntimeError,
        EventKind::TurnStarted,
        EventKind::TurnCompleted,
        EventKind::TurnInterrupted,
        EventKind::ProviderActivity,
        EventKind::ProviderOutputDelta,
        EventKind::ProviderMessageCompleted,
        EventKind::ApprovalRequested,
        EventKind::ApprovalResolved,
        EventKind::CoordinationWarningAcknowledged,
        EventKind::WorkspaceValidated,
        EventKind::ResourceSnapshotUpdated,
    ];
    for (index, kind) in event_kinds.iter().cloned().enumerate() {
        let is_workspace_event = matches!(
            kind,
            EventKind::WorkspaceValidated | EventKind::ResourceSnapshotUpdated
        );
        let payload = match kind {
            EventKind::ApprovalRequested => json!({
                "approval_id": "all-kinds-approval",
                "prompt": "approve all-kinds test"
            }),
            EventKind::ApprovalResolved => json!({
                "approval_id": "all-kinds-approval",
                "approved": true
            }),
            EventKind::ProviderMessageCompleted => json!({ "content": "complete" }),
            EventKind::RuntimeError => json!({ "message": "expected test error" }),
            EventKind::WorkspaceValidated | EventKind::ResourceSnapshotUpdated => json!({
                "placement_id": detail.placement.project_placement_id.as_str(),
                "state": "validated",
                "resource_badges": []
            }),
            _ => json!({}),
        };
        let mut event = node_event_fixture(
            &detail,
            node_id.clone(),
            &format!("all-kinds-{index}"),
            0,
            kind,
            payload,
        );
        if is_workspace_event {
            event.scope_ref = ScopeRef::Placement {
                project_placement_id: detail.placement.project_placement_id.clone(),
            };
        }
        event.seq = next_seq(&state, &scope_key(&event.scope_ref))
            .await
            .expect("next event sequence allocates");

        accept_node_event(&state, event)
            .await
            .unwrap_or_else(|error| panic!("{kind:?} projection failed: {error}"));
    }

    let durable_counts: (i64, i64) = sqlx::query_as(
            r#"
            select
                (select count(*) from events where event_id like 'all-kinds-%' and projection_state = 'projected'),
                (select count(*) from event_publication_outbox where event_id like 'all-kinds-%')
            "#,
        )
        .fetch_one(&state.pool)
        .await
        .expect("all event projection counts load");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        durable_counts,
        (event_kinds.len() as i64, event_kinds.len() as i64)
    );
}

#[tokio::test]
async fn approval_requested_event_creates_approval_message_and_blocks_runtime() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "approval-requested-1",
        1,
        EventKind::ApprovalRequested,
        json!({
            "approval_id": "approval-1",
            "prompt": "Allow test command"
        }),
    );

    accept_node_event(&state, event)
        .await
        .expect("approval event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    let (approval_state, request_payload_json): (String, String) =
        sqlx::query_as("select state, request_payload_json from approvals where approval_id = ?1")
            .bind("approval-1")
            .fetch_one(&state.pool)
            .await
            .expect("approval row loads");
    let request_payload: serde_json::Value =
        serde_json::from_str(&request_payload_json).expect("request payload decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Blocked);
    assert_eq!(approval_state, "requested");
    assert_eq!(
        request_payload
            .get("prompt")
            .and_then(serde_json::Value::as_str),
        Some("Allow test command")
    );
    assert!(detail.messages.iter().any(|message| {
        message.role == MessageRole::Approval && message.content == "Allow test command"
    }));
}

#[tokio::test]
async fn duplicate_approval_requested_event_does_not_duplicate_approval_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "approval-requested-duplicate",
        1,
        EventKind::ApprovalRequested,
        json!({
            "approval_id": "approval-duplicate",
            "prompt": "Allow duplicate command"
        }),
    );

    accept_node_event(&state, event.clone())
        .await
        .expect("first approval event accepts");
    accept_node_event(&state, event)
        .await
        .expect("duplicate approval event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        detail
            .messages
            .iter()
            .filter(|message| {
                message.role == MessageRole::Approval
                    && message.content == "Allow duplicate command"
            })
            .count(),
        1
    );
}

#[tokio::test]
async fn duplicate_approval_resolved_event_does_not_duplicate_resolution_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "approval-resolution-requested",
            1,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-resolution-duplicate",
                "prompt": "Allow resolution test"
            }),
        ),
    )
    .await
    .expect("approval request accepts");
    let event = node_event_fixture(
        &detail,
        node_id,
        "approval-resolution-duplicate",
        2,
        EventKind::ApprovalResolved,
        json!({
            "approval_id": "approval-resolution-duplicate",
            "approved": true,
            "message": "approved once"
        }),
    );

    accept_node_event(&state, event.clone())
        .await
        .expect("first approval resolution accepts");
    accept_node_event(&state, event)
        .await
        .expect("duplicate approval resolution accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        detail
            .messages
            .iter()
            .filter(|message| {
                message.role == MessageRole::Approval && message.content == "approved once"
            })
            .count(),
        1
    );
}

#[tokio::test]
async fn runtime_error_event_creates_runtime_message_and_marks_error() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "runtime-error-1",
        1,
        EventKind::RuntimeError,
        json!({ "message": "boom" }),
    );

    accept_node_event(&state, event)
        .await
        .expect("runtime error event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Error);
    assert!(detail
        .messages
        .iter()
        .any(|message| message.role == MessageRole::Runtime && message.content == "boom"));
}

#[tokio::test]
async fn duplicate_runtime_error_event_does_not_duplicate_runtime_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "runtime-error-duplicate",
        1,
        EventKind::RuntimeError,
        json!({ "message": "boom" }),
    );

    accept_node_event(&state, event.clone())
        .await
        .expect("first runtime error accepts");
    accept_node_event(&state, event)
        .await
        .expect("duplicate runtime error accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        detail
            .messages
            .iter()
            .filter(|message| { message.role == MessageRole::Runtime && message.content == "boom" })
            .count(),
        1
    );
}

#[tokio::test]
async fn resolve_approval_records_routed_command() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "resolve-approval-requested-1",
            1,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-1",
                "prompt": "Allow routed command"
            }),
        ),
    )
    .await
    .expect("approval request accepts");

    let response = resolve_approval(
        State(state.clone()),
        Path((
            detail.session.session_thread_id.to_string(),
            "approval-1".to_owned(),
        )),
        Json(ResolveApprovalRequest {
            approved: true,
            message: Some("approved".to_owned()),
        }),
    )
    .await
    .expect("approval resolve command records")
    .0;
    let command_kind: String =
        sqlx::query_scalar("select kind from commands where command_id = ?1")
            .bind(response.command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command kind loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command_kind, "ResolveApproval");
}

#[tokio::test]
async fn resolve_approval_rejects_non_pending_approval_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let command_count_before = command_count(&state).await;

    let result = resolve_approval(
        State(state.clone()),
        Path((
            detail.session.session_thread_id.to_string(),
            "approval-missing".to_owned(),
        )),
        Json(ResolveApprovalRequest {
            approved: true,
            message: None,
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "approval.not_pending",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
}

#[tokio::test]
async fn detach_and_attach_session_update_state_without_recording_node_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let command_count_before = command_count(&state).await;

    let detached = detach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await
    .expect("session detaches")
    .0;
    let attached = attach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await
    .expect("session attaches")
    .0;
    let command_count_after = command_count(&state).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detached.session.state, SessionThreadState::Detached);
    assert_eq!(attached.session.state, SessionThreadState::Active);
    assert_eq!(command_count_after, command_count_before);
}

#[tokio::test]
async fn runtime_ready_event_preserves_detached_session_state() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let _ = detach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await
    .expect("session detaches");

    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "detached-runtime-ready-1",
            1,
            EventKind::RuntimeReady,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("ready event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detail.session.state, SessionThreadState::Detached);
    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Ready);
}

#[tokio::test]
async fn runtime_ready_event_persists_provider_resume_ref() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;

    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "provider-resume-ref-ready-1",
            1,
            EventKind::RuntimeReady,
            json!({
                "provider": "codex",
                "provider_resume_ref": {
                    "provider_session_id": "codex-session-1",
                    "resume_cursor": "cursor-1"
                }
            }),
        ),
    )
    .await
    .expect("ready event accepts");
    let provider_resume_ref_json: Option<String> = sqlx::query_scalar(
        "select provider_resume_ref_json from runtime_sessions where runtime_session_id = ?1",
    )
    .bind(detail.session.runtime.runtime_session_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("provider resume ref loads");
    let provider_resume_ref: serde_json::Value = serde_json::from_str(
        provider_resume_ref_json
            .as_deref()
            .expect("provider resume ref persisted"),
    )
    .expect("provider resume ref decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        provider_resume_ref
            .get("provider_session_id")
            .and_then(serde_json::Value::as_str),
        Some("codex-session-1")
    );
    assert_eq!(
        provider_resume_ref
            .get("resume_cursor")
            .and_then(serde_json::Value::as_str),
        Some("cursor-1")
    );
}

#[tokio::test]
async fn attach_session_rejects_stopped_session() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    sqlx::query("update session_threads set state = ?1 where session_thread_id = ?2")
        .bind(format_session_state(SessionThreadState::Stopped))
        .bind(detail.session.session_thread_id.as_str())
        .execute(&state.pool)
        .await
        .expect("session stops");

    let result = attach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "session.stopped",
            ..
        })
    ));
}
