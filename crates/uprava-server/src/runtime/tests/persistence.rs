use super::*;

#[tokio::test]
async fn record_command_persists_queryable_envelope_fields() {
    let state = test_state().await;
    let node_id = NodeId::from("node-queryable-command");
    let command_id = CommandId::from("command-queryable");
    let mut command = command_fixture(command_id.clone(), node_id.clone());
    command.actor_ref = ActorRef::System;
    command.source_refs = vec![UpravaRef::Node {
        node_id: node_id.clone(),
    }];
    command.cause_refs = vec![UpravaRef::Command {
        command_id: CommandId::from("command-cause"),
    }];
    command.payload = CommandPayload::Extension {
        name: "test.queryable".to_owned(),
        value: JsonValue(json!({ "reason": "queryable fields" })),
    };
    command.kind = CommandKind::Extension;

    record_command(&state, command)
        .await
        .expect("command records");
    let (
        actor_ref_json,
        correlation_id,
        source_refs_json,
        cause_refs_json,
        payload_json,
        dedupe_key,
    ): (String, String, String, String, String, String) = sqlx::query_as(
        r#"
            select actor_ref_json, correlation_id, source_refs_json,
                   cause_refs_json, payload_json, dedupe_key
            from commands
            where command_id = ?1
            "#,
    )
    .bind(command_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("command row loads");
    let actor_ref =
        serde_json::from_str::<ActorRef>(&actor_ref_json).expect("actor ref json decodes");
    let source_refs = serde_json::from_str::<Vec<UpravaRef>>(&source_refs_json)
        .expect("source refs json decodes");
    let cause_refs =
        serde_json::from_str::<Vec<UpravaRef>>(&cause_refs_json).expect("cause refs json decodes");
    let payload = serde_json::from_str::<JsonValue>(&payload_json).expect("payload json decodes");

    assert_eq!(actor_ref, ActorRef::System);
    assert_eq!(correlation_id, "correlation-1");
    assert_eq!(
        source_refs,
        vec![UpravaRef::Node {
            node_id: node_id.clone()
        }]
    );
    assert_eq!(
        cause_refs,
        vec![UpravaRef::Command {
            command_id: CommandId::from("command-cause")
        }]
    );
    assert_eq!(
        payload
            .0
            .get("value")
            .and_then(|value| value.get("reason"))
            .and_then(serde_json::Value::as_str),
        Some("queryable fields")
    );
    assert_eq!(dedupe_key, command_id.as_str());
}

#[tokio::test]
async fn record_command_rejects_payload_kind_mismatch_before_persistence() {
    let state = test_state().await;
    let command_id = CommandId::from("command-payload-mismatch");
    let mut command = command_fixture(command_id.clone(), NodeId::from("node-payload-mismatch"));
    command.kind = CommandKind::SendTurn;

    let error = record_command(&state, command)
        .await
        .expect_err("payload mismatch rejects");
    let persisted: i64 = sqlx::query_scalar("select count(*) from commands where command_id = ?1")
        .bind(command_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("command count loads");

    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "protocol.command_payload_mismatch",
            ..
        }
    ));
    assert_eq!(persisted, 0);
}

#[tokio::test]
async fn accept_node_event_persists_queryable_envelope_fields() {
    let state = test_state().await;
    let node_id = NodeId::from("node-queryable-event");
    let runtime_session_id = RuntimeSessionId::from("runtime-queryable-event");
    let scope_ref = ScopeRef::Runtime {
        runtime_session_id: runtime_session_id.clone(),
    };
    let source_ref = UpravaRef::Node {
        node_id: node_id.clone(),
    };
    let result_ref = UpravaRef::Runtime {
        runtime_session_id: runtime_session_id.clone(),
    };

    accept_node_event(
        &state,
        EventEnvelope {
            event_id: EventId::from("event-queryable"),
            command_id: None,
            correlation_id: Some(CorrelationId::from("correlation-event")),
            actor_ref: ActorRef::Provider {
                provider: "codex".to_owned(),
            },
            scope_ref: scope_ref.clone(),
            node_id: Some(node_id),
            runtime_session_id: None,
            session_thread_id: None,
            turn_id: None,
            seq: 1,
            session_projection_seq: None,
            kind: EventKind::ProviderMessageCompleted,
            happened_at: Utc::now(),
            source_refs: vec![source_ref.clone()],
            evidence_refs: vec![],
            cause_refs: vec![UpravaRef::Command {
                command_id: CommandId::from("command-cause"),
            }],
            result_refs: vec![result_ref.clone()],
            payload: EventPayload::from_json(
                EventKind::ProviderMessageCompleted,
                json!({ "content": "queryable event" }),
            ),
        },
    )
    .await
    .expect("event accepts");
    let (
        actor_ref_json,
        scope_ref_json,
        correlation_id,
        source_refs_json,
        result_refs_json,
        payload_json,
    ): (String, String, String, String, String, String) = sqlx::query_as(
        r#"
            select actor_ref_json, scope_ref_json, correlation_id,
                   source_refs_json, result_refs_json, payload_json
            from events
            where event_id = ?1
            "#,
    )
    .bind("event-queryable")
    .fetch_one(&state.pool)
    .await
    .expect("event row loads");
    let actor_ref =
        serde_json::from_str::<ActorRef>(&actor_ref_json).expect("actor ref json decodes");
    let persisted_scope_ref =
        serde_json::from_str::<ScopeRef>(&scope_ref_json).expect("scope ref json decodes");
    let source_refs = serde_json::from_str::<Vec<UpravaRef>>(&source_refs_json)
        .expect("source refs json decodes");
    let result_refs = serde_json::from_str::<Vec<UpravaRef>>(&result_refs_json)
        .expect("result refs json decodes");
    let payload = serde_json::from_str::<JsonValue>(&payload_json).expect("payload json decodes");

    assert_eq!(
        actor_ref,
        ActorRef::Provider {
            provider: "codex".to_owned()
        }
    );
    assert_eq!(persisted_scope_ref, scope_ref);
    assert_eq!(correlation_id, "correlation-event");
    assert_eq!(source_refs, vec![source_ref]);
    assert_eq!(result_refs, vec![result_ref]);
    assert_eq!(
        payload.0.get("content").and_then(serde_json::Value::as_str),
        Some("queryable event")
    );
}

#[tokio::test]
async fn accept_node_event_rejects_payload_kind_mismatch_before_persistence() {
    let state = test_state().await;
    let event_id = EventId::from("event-payload-mismatch");
    let mut event = EventEnvelope {
        event_id: event_id.clone(),
        command_id: None,
        correlation_id: None,
        actor_ref: ActorRef::System,
        scope_ref: ScopeRef::Unknown {
            scope: "payload-mismatch".to_owned(),
        },
        node_id: None,
        runtime_session_id: None,
        session_thread_id: None,
        turn_id: None,
        seq: 1,
        session_projection_seq: None,
        kind: EventKind::RuntimeReady,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: EventPayload::from_json(
            EventKind::RuntimeError,
            json!({ "code": "failed", "message": "failure" }),
        ),
    };
    event.kind = EventKind::RuntimeReady;

    let error = accept_node_event(&state, event)
        .await
        .expect_err("event payload mismatch rejects");
    let persisted: i64 = sqlx::query_scalar("select count(*) from events where event_id = ?1")
        .bind(event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("event count loads");

    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "protocol.event_payload_mismatch",
            ..
        }
    ));
    assert_eq!(persisted, 0);
}

#[tokio::test]
async fn accept_node_event_backfills_correlation_id_from_command() {
    let state = test_state().await;
    let command_id = CommandId::from("command-correlation");
    let node_id = NodeId::from("node-1");
    let runtime_session_id = RuntimeSessionId::from("runtime-correlation");
    record_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");

    accept_node_event(
        &state,
        EventEnvelope {
            event_id: EventId::from("event-correlation"),
            command_id: Some(command_id),
            correlation_id: None,
            actor_ref: ActorRef::Provider {
                provider: "codex".to_owned(),
            },
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: runtime_session_id.clone(),
            },
            node_id: Some(node_id),
            runtime_session_id: Some(runtime_session_id),
            session_thread_id: None,
            turn_id: None,
            seq: 1,
            session_projection_seq: None,
            kind: EventKind::RuntimeReady,
            happened_at: Utc::now(),
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: EventPayload::from_json(EventKind::RuntimeReady, json!({})),
        },
    )
    .await
    .expect("event accepts");
    let event_json: String =
        sqlx::query_scalar("select event_json from events where event_id = ?1")
            .bind("event-correlation")
            .fetch_one(&state.pool)
            .await
            .expect("event json loads");
    let event: EventEnvelope = serde_json::from_str(&event_json).expect("event json decodes");

    assert_eq!(
        event.correlation_id,
        Some(CorrelationId::from("correlation-1"))
    );
    let actor_count: i64 = sqlx::query_scalar(
        "select count(*) from actors where actor_key in ('local_user', 'provider:codex')",
    )
    .fetch_one(&state.pool)
    .await
    .expect("actors count loads");

    assert_eq!(actor_count, 2);
}

#[tokio::test]
async fn event_append_uses_monotonic_scope_sequence() {
    let state = test_state().await;
    let runtime_id = RuntimeSessionId::from("runtime-test");
    let first = append_event(
        &state,
        NewEvent {
            command_id: None,
            actor_ref: ActorRef::System,
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: runtime_id.clone(),
            },
            node_id: None,
            runtime_session_id: Some(runtime_id.clone()),
            session_thread_id: None,
            turn_id: None,
            kind: EventKind::RuntimeReady,
            payload: json!({}),
        },
    )
    .await
    .expect("first event appends");
    let second = append_event(
        &state,
        NewEvent {
            command_id: None,
            actor_ref: ActorRef::System,
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: runtime_id,
            },
            node_id: None,
            runtime_session_id: None,
            session_thread_id: None,
            turn_id: None,
            kind: EventKind::RuntimeRunning,
            payload: json!({}),
        },
    )
    .await
    .expect("second event appends");

    assert_eq!(second.seq, first.seq + 1);
}
