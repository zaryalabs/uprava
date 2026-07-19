use super::*;

#[test]
fn node_local_state_loads_legacy_state_without_reliability_fields() {
    let path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
    std::fs::write(
        &path,
        r#"{"node_id":"node-1","credential":"development-secret"}"#,
    )
    .expect("legacy state fixture writes");

    let local_state = NodeLocalState::load(&path).expect("legacy state loads");
    std::fs::remove_file(path).expect("legacy state fixture is removed");

    assert!(local_state.daemon_installation_id.starts_with("daemon-"));
    assert!(local_state.command_status.is_empty());
    assert!(local_state.runtime_seqs.is_empty());
    assert!(local_state.event_outbox.is_empty());
    assert!(local_state.runtime_providers.is_empty());
    assert!(local_state.runtime_workspace_paths.is_empty());
    assert!(local_state.runtime_states.is_empty());
    assert!(local_state.runtime_transcripts.is_empty());
    assert!(local_state.placement_seqs.is_empty());
}

#[test]
fn versioned_state_slot_rejects_unmarked_legacy_json() {
    let dir = std::env::temp_dir().join(format!("uprava-node-slot-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.json");
    std::fs::create_dir_all(path.parent().expect("slot parent exists"))
        .expect("slot directory creates");
    std::fs::write(&path, r#"{"node_id":"old","credential":"legacy"}"#)
        .expect("legacy state fixture writes");

    let error = NodeLocalState::load(&path).expect_err("legacy slot state must be rejected");
    assert!(error.to_string().contains("not compatible with slot 0.2.0"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn node_local_state_preserves_daemon_installation_id_after_save() {
    let path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
    let local_state = NodeLocalState::default();
    let installation_id = local_state.daemon_installation_id.clone();
    local_state.save(&path).expect("node state saves");

    let reloaded = NodeLocalState::load(&path).expect("node state reloads");
    std::fs::remove_file(path).expect("node state fixture is removed");

    assert_eq!(reloaded.daemon_installation_id, installation_id);
}

#[test]
fn node_local_state_compacts_completed_command_cache_on_save() {
    let path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
    let mut local_state = NodeLocalState::default();
    local_state
        .command_status
        .insert("active".to_owned(), CommandState::Dispatched);
    for index in 0..(MAX_RETAINED_COMMANDS + 20) {
        let command_id = format!("completed-{index}");
        local_state
            .command_status
            .insert(command_id.clone(), CommandState::Completed);
        local_state
            .command_result_payloads
            .insert(command_id, JsonValue(serde_json::json!({ "ok": true })));
    }

    local_state.save(&path).expect("state saves");
    let reloaded = NodeLocalState::load(&path).expect("state reloads");
    let _ = std::fs::remove_file(path);

    assert!(reloaded.command_status.len() <= MAX_RETAINED_COMMANDS);
    assert_eq!(
        reloaded.command_status.get("active"),
        Some(&CommandState::Dispatched)
    );
    assert!(reloaded
        .command_result_payloads
        .keys()
        .all(|command_id| reloaded.command_status.contains_key(command_id)));
}

#[tokio::test]
async fn sqlite_state_store_round_trips_versioned_snapshot_transactionally() {
    let dir = std::env::temp_dir().join(format!("uprava-node-sqlite-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.sqlite");
    let mut state = NodeLocalState {
        node_id: Some(NodeId::from("node-sqlite")),
        ..NodeLocalState::default()
    };
    state
        .command_status
        .insert("command-sqlite".to_owned(), CommandState::Completed);
    state.runtime_seqs.insert("runtime-sqlite".to_owned(), 7);
    state
        .runtime_providers
        .insert("runtime-sqlite".to_owned(), "codex".to_owned());
    state
        .runtime_states
        .insert("runtime-sqlite".to_owned(), RuntimeSessionState::Running);
    state
        .placement_seqs
        .insert("placement-sqlite".to_owned(), 3);

    state.save_async(&path).await.expect("sqlite state saves");
    let pool = open_state_store(&path).await.expect("store opens");
    sqlx::query("update node_state set snapshot_json = ?1 where state_id = 1")
            .bind(
                r#"{"state_slot":"0.2.0","schema_version":1,"daemon_installation_id":"snapshot-fallback"}"#,
            )
            .execute(&pool)
            .await
            .expect("snapshot is replaceable for hydration test");
    pool.close().await;
    let reloaded = NodeLocalState::load_async(&path)
        .await
        .expect("sqlite state reloads");
    assert_eq!(reloaded.node_id, Some(NodeId::from("node-sqlite")));
    assert_eq!(
        reloaded.command_status.get("command-sqlite"),
        Some(&CommandState::Completed)
    );

    let pool = open_state_store(&path).await.expect("store reopens");
    let row: (String, i64) =
        sqlx::query_as("select state_slot, schema_version from node_state where state_id = 1")
            .fetch_one(&pool)
            .await
            .expect("version metadata persists");
    assert_eq!(
        row,
        (NODE_STATE_SLOT.to_owned(), NODE_STATE_SCHEMA_VERSION as i64)
    );
    let command_count: i64 = sqlx::query_scalar("select count(*) from node_command_cache")
        .fetch_one(&pool)
        .await
        .expect("command cache persists");
    let outbox_count: i64 = sqlx::query_scalar("select count(*) from node_event_outbox")
        .fetch_one(&pool)
        .await
        .expect("event outbox persists");
    let registration_count: i64 = sqlx::query_scalar("select count(*) from node_registration")
        .fetch_one(&pool)
        .await
        .expect("registration persists");
    let runtime_count: i64 = sqlx::query_scalar("select count(*) from node_runtime_metadata")
        .fetch_one(&pool)
        .await
        .expect("runtime metadata persists");
    let placement_count: i64 = sqlx::query_scalar("select count(*) from node_placement_sequences")
        .fetch_one(&pool)
        .await
        .expect("placement sequences persist");
    pool.close().await;
    assert_eq!(command_count, 1);
    assert_eq!(outbox_count, 0);
    assert_eq!(registration_count, 1);
    assert_eq!(runtime_count, 1);
    assert_eq!(placement_count, 1);
    std::fs::remove_dir_all(dir).expect("sqlite state fixture removes");
}

#[tokio::test]
async fn open_state_store_creates_missing_parent_directory() {
    let dir = std::env::temp_dir().join(format!("uprava-node-store-parent-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.sqlite");

    let pool = open_state_store(&path)
        .await
        .expect("store opens below a missing parent directory");
    pool.close().await;

    assert!(path.exists());
    std::fs::remove_dir_all(dir).expect("state store fixture removes");
}

#[tokio::test]
async fn sqlite_state_store_snapshot_is_only_a_compatibility_seed() {
    let dir = std::env::temp_dir().join(format!("uprava-node-seed-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.sqlite");
    let mut state = NodeLocalState {
        node_id: Some(NodeId::from("node-seed")),
        reconnect_attempts: 4,
        ..NodeLocalState::default()
    };
    state
        .command_status
        .insert("command-seed".to_owned(), CommandState::Completed);
    state.runtime_seqs.insert("runtime-seed".to_owned(), 9);
    state.runtime_provider_resume_refs.insert(
        "runtime-seed".to_owned(),
        ProviderResumeRef {
            provider_session_id: Some("provider-session".to_owned()),
            resume_cursor: Some("cursor".to_owned()),
        },
    );

    state.save_async(&path).await.expect("sqlite state saves");
    let pool = open_state_store(&path).await.expect("store opens");
    let snapshot_json: String =
        sqlx::query_scalar("select snapshot_json from node_state where state_id = 1")
            .fetch_one(&pool)
            .await
            .expect("snapshot loads");
    pool.close().await;
    let seed = serde_json::from_str::<NodeLocalState>(&snapshot_json).expect("seed decodes");
    let reloaded = NodeLocalState::load_async(&path)
        .await
        .expect("sqlite state reloads");
    std::fs::remove_dir_all(dir).expect("sqlite state fixture removes");

    assert!(seed.command_status.is_empty());
    assert!(seed.runtime_seqs.is_empty());
    assert!(seed.runtime_provider_resume_refs.is_empty());
    assert_eq!(
        reloaded.command_status.get("command-seed"),
        Some(&CommandState::Completed)
    );
    assert_eq!(reloaded.runtime_seqs.get("runtime-seed").copied(), Some(9));
    assert_eq!(reloaded.reconnect_attempts, 4);
}

#[tokio::test]
async fn enrollment_owner_api_reopens_attempt_and_identity() {
    let dir = std::env::temp_dir().join(format!("uprava-node-enrollment-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.sqlite");
    let store = NodeStateStore::new(NodeLocalState::default(), path.clone());

    store
        .persist_enrollment_attempt(
            EnrollmentId::from("enrollment-reopen"),
            "pairing-reopen".to_owned(),
        )
        .await
        .expect("enrollment attempt persists");
    let reopened = NodeLocalState::load_async(&path)
        .await
        .expect("enrollment attempt reopens");
    assert_eq!(
        reopened.enrollment_id,
        Some(EnrollmentId::from("enrollment-reopen"))
    );
    assert_eq!(reopened.pairing_code.as_deref(), Some("pairing-reopen"));

    let reopened_store = NodeStateStore::new(reopened, path.clone());
    reopened_store
        .persist_enrollment_identity(NodeId::from("node-reopen"), "credential-reopen".to_owned())
        .await
        .expect("enrollment identity persists");
    let enrolled = NodeLocalState::load_async(&path)
        .await
        .expect("enrollment identity reopens");
    assert_eq!(enrolled.node_id, Some(NodeId::from("node-reopen")));
    assert_eq!(enrolled.credential.as_deref(), Some("credential-reopen"));
    assert_eq!(enrolled.enrollment_id, None);
    assert_eq!(enrolled.pairing_code, None);

    std::fs::remove_dir_all(dir).expect("enrollment state fixture removes");
}

#[tokio::test]
async fn sqlite_state_store_reconnect_replays_pending_work_without_duplication() {
    let dir = std::env::temp_dir().join(format!("uprava-node-reconnect-{}", Uuid::new_v4()));
    let path = dir.join(NODE_STATE_SLOT).join("node.sqlite");
    let config = config_fixture();
    let command = command_fixture("command-reconnect", CommandKind::SendTurn);
    let mut initial = NodeLocalState {
        node_id: Some(NodeId::from("node-reconnect")),
        credential: Some("credential-reconnect".to_owned()),
        ..NodeLocalState::default()
    };
    let first = prepare_command_dispatch(&config, &mut initial, &command).await;
    let first_event_ids = event_ids(&first.events_to_send);
    assert_eq!(first.status, CommandState::Failed);
    assert_eq!(first_event_ids.len(), 1);

    initial
        .save_async(&path)
        .await
        .expect("initial reconnect state persists");

    // Simulate a daemon restart followed by a reconnect attempt. The
    // SQLite normalized tables, rather than only the snapshot, must retain
    // both the command result and the unacknowledged event.
    let reopened = NodeLocalState::load_async(&path)
        .await
        .expect("reconnect state reopens");
    let store = NodeStateStore::new(reopened, path.clone());
    store
        .persist_reconnect_attempt()
        .await
        .expect("first reconnect attempt persists");
    store
        .persist_reconnect_attempt()
        .await
        .expect("second reconnect attempt persists");

    let after_reconnect = store.snapshot().await.expect("state snapshot");
    assert_eq!(after_reconnect.reconnect_attempts, 2);
    assert_eq!(
        after_reconnect
            .command_status
            .get(command.command_id.as_str()),
        Some(&CommandState::Failed)
    );
    assert_eq!(event_ids(&after_reconnect.event_outbox), first_event_ids);

    // A duplicate command received after reconnect replays the cached
    // result/event and must not append a second outbox entry.
    let mut duplicate_snapshot = after_reconnect.clone();
    let duplicate = prepare_command_dispatch(&config, &mut duplicate_snapshot, &command).await;
    assert_eq!(duplicate.status, CommandState::Failed);
    assert!(!duplicate.state_changed);
    assert_eq!(event_ids(&duplicate.events_to_send), first_event_ids);
    assert_eq!(event_ids(&duplicate_snapshot.event_outbox), first_event_ids);

    let reopened_again = NodeLocalState::load_async(&path)
        .await
        .expect("reconnect state survives a second reopen");
    assert_eq!(reopened_again.reconnect_attempts, 2);
    assert_eq!(
        reopened_again
            .command_status
            .get(command.command_id.as_str()),
        Some(&CommandState::Failed)
    );
    assert_eq!(event_ids(&reopened_again.event_outbox), first_event_ids);

    store
        .persist_event_ack(&first_event_ids)
        .await
        .expect("event ACK persists after replay");
    let acknowledged = NodeLocalState::load_async(&path)
        .await
        .expect("acknowledged reconnect state reopens");
    assert!(acknowledged.event_outbox.is_empty());
    assert_eq!(
        acknowledged.command_status.get(command.command_id.as_str()),
        Some(&CommandState::Failed)
    );

    std::fs::remove_dir_all(dir).expect("reconnect state fixture removes");
}

#[cfg(unix)]
#[test]
fn node_local_state_save_uses_private_file_permissions() {
    let dir = std::env::temp_dir().join(format!("uprava-node-{}", Uuid::new_v4()));
    let path = dir.join("node.json");
    let local_state = NodeLocalState::default();
    local_state.save(&path).expect("node state saves");

    let file_mode = std::fs::metadata(&path)
        .expect("state metadata loads")
        .permissions()
        .mode()
        & 0o777;
    let dir_mode = std::fs::metadata(&dir)
        .expect("state dir metadata loads")
        .permissions()
        .mode()
        & 0o777;
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);

    assert_eq!(file_mode, 0o600);
    assert_eq!(dir_mode, 0o700);
}

#[test]
fn node_diagnostics_reports_bounded_operational_counts_without_ids() {
    let mut local_state = NodeLocalState {
        daemon_installation_id: "daemon-test".to_owned(),
        ..NodeLocalState::default()
    };
    local_state
        .command_status
        .insert("command-1".to_owned(), CommandState::Completed);

    let diagnostics = node_diagnostics(&local_state);

    assert_eq!(
            diagnostics,
            "outbox_events=0; cached_commands=1; reconnect_attempts=0; dropped_events=0; heartbeat_failures=0; dropped_log_records=0; otlp_export_failures=0"
        );
    assert!(!diagnostics.contains("daemon-test"));
}

#[tokio::test]
async fn state_store_persists_heartbeat_failures_and_merges_concurrently() {
    let path = std::env::temp_dir().join(format!(
        "uprava-node-heartbeat-metric-{}.sqlite",
        Uuid::new_v4()
    ));
    let state = NodeLocalState::default();
    state.save_async(&path).await.expect("state persists");
    let store = NodeStateStore::new(state, path.clone());

    let (first, second) = tokio::join!(
        store.persist_heartbeat_failure(),
        store.persist_heartbeat_failure()
    );
    first.expect("first heartbeat failure persists");
    second.expect("second heartbeat failure persists");

    assert_eq!(
        store
            .snapshot()
            .await
            .expect("state snapshot")
            .heartbeat_failures,
        2
    );
    let reopened = NodeLocalState::load_async(&path)
        .await
        .expect("heartbeat metric reloads");
    assert_eq!(reopened.heartbeat_failures, 2);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn sqlite_state_store_hydrates_metrics_from_normalized_table() {
    let path = std::env::temp_dir().join(format!("uprava-node-metrics-{}.sqlite", Uuid::new_v4()));
    let state = NodeLocalState {
        reconnect_attempts: 7,
        dropped_event_count: 5,
        heartbeat_failures: 3,
        ..NodeLocalState::default()
    };
    state.save_async(&path).await.expect("state persists");
    let stale_snapshot =
        serde_json::to_string(&NodeLocalState::default()).expect("stale snapshot serializes");
    let pool = open_state_store(&path).await.expect("store opens");
    sqlx::query("update node_state set snapshot_json = ?1 where state_id = 1")
        .bind(stale_snapshot)
        .execute(&pool)
        .await
        .expect("snapshot rewrites");
    pool.close().await;

    let reloaded = NodeLocalState::load_async(&path)
        .await
        .expect("state reloads");
    let _ = std::fs::remove_file(path);

    assert_eq!(reloaded.reconnect_attempts, 7);
    assert_eq!(reloaded.dropped_event_count, 5);
    assert_eq!(reloaded.heartbeat_failures, 3);
}

#[tokio::test]
async fn state_store_shutdown_joins_actor_and_rejects_later_requests() {
    let path = std::env::temp_dir().join(format!("uprava-node-shutdown-{}.sqlite", Uuid::new_v4()));
    let state = NodeLocalState::default();
    state.save_async(&path).await.expect("state persists");
    let store = NodeStateStore::new(state, path.clone());
    store
        .persist_heartbeat_failure()
        .await
        .expect("mutation before shutdown persists");

    store.shutdown().await.expect("state actor joins");
    let error = store
        .persist_heartbeat_failure()
        .await
        .expect_err("mutation after shutdown fails");
    let _ = std::fs::remove_file(path);

    assert!(error.to_string().contains("state store task stopped"));
}

#[test]
fn node_local_state_debug_redacts_secret_fields() {
    let local_state = NodeLocalState {
        credential: Some("development-secret".to_owned()),
        pairing_code: Some("pair-secret".to_owned()),
        runtime_transcripts: HashMap::from([(
            "runtime-1".to_owned(),
            vec![ProviderTranscriptMessage {
                role: "user".to_owned(),
                content: "secret prompt content".to_owned(),
            }],
        )]),
        ..NodeLocalState::default()
    };

    let formatted = format!("{local_state:?}");

    assert!(!formatted.contains("development-secret"));
    assert!(!formatted.contains("pair-secret"));
    assert!(!formatted.contains("secret prompt content"));
    assert!(formatted.contains("runtime_transcript_counts"));
    assert!(formatted.contains("[redacted]"));
}

#[test]
fn clear_core_registration_removes_pairing_and_node_identity_only() {
    let mut local_state = NodeLocalState {
        node_id: Some(NodeId::from("node-1")),
        credential: Some("development-secret".to_owned()),
        enrollment_id: Some(EnrollmentId::from("enrollment-1")),
        pairing_code: Some("pair-secret".to_owned()),
        command_status: HashMap::from([("command-1".to_owned(), CommandState::Completed)]),
        ..NodeLocalState::default()
    };

    local_state.clear_core_registration();

    assert_eq!(local_state.node_id, None);
    assert_eq!(local_state.credential, None);
    assert_eq!(local_state.enrollment_id, None);
    assert_eq!(local_state.pairing_code, None);
    assert_eq!(
        local_state.command_status.get("command-1"),
        Some(&CommandState::Completed)
    );
}

#[test]
fn clear_enrollment_attempt_preserves_node_identity_and_cached_work() {
    let mut local_state = NodeLocalState {
        node_id: Some(NodeId::from("node-1")),
        credential: Some("development-secret".to_owned()),
        enrollment_id: Some(EnrollmentId::from("enrollment-1")),
        pairing_code: Some("pair-secret".to_owned()),
        command_status: HashMap::from([("command-1".to_owned(), CommandState::Completed)]),
        ..NodeLocalState::default()
    };

    local_state.clear_enrollment_attempt();

    assert_eq!(local_state.node_id, Some(NodeId::from("node-1")));
    assert_eq!(
        local_state.credential,
        Some("development-secret".to_owned())
    );
    assert_eq!(local_state.enrollment_id, None);
    assert_eq!(local_state.pairing_code, None);
    assert_eq!(
        local_state.command_status.get("command-1"),
        Some(&CommandState::Completed)
    );
}

#[test]
fn enrollment_claim_status_invalidates_only_stale_local_attempts() {
    assert!(enrollment_claim_status_invalidates_attempt(Some(
        reqwest::StatusCode::NOT_FOUND
    )));
    assert!(enrollment_claim_status_invalidates_attempt(Some(
        reqwest::StatusCode::UNAUTHORIZED
    )));
    assert!(!enrollment_claim_status_invalidates_attempt(Some(
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    )));
    assert!(!enrollment_claim_status_invalidates_attempt(None));
}

#[tokio::test]
async fn ensure_enrollment_clears_stale_local_attempt_after_not_found_claim() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server binds");
    let address = listener.local_addr().expect("test server address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("request accepted");
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer).await.expect("request read");
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .await
            .expect("response written");
    });
    let state_path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
    let mut config = config_fixture();
    config.core_url = format!("http://{address}")
        .parse()
        .expect("test core URL parses");
    config.state_path = state_path.clone();
    let local_state = NodeLocalState {
        enrollment_id: Some(EnrollmentId::from("stale-enrollment")),
        pairing_code: Some("stale-pairing-code".to_owned()),
        ..NodeLocalState::default()
    };

    let store = NodeStateStore::new(local_state, state_path.clone());
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test client builds");
    let enrolled = store
        .ensure_enrollment(&client, &config)
        .await
        .expect("stale enrollment clears");
    server.await.expect("test server finishes");
    let saved_state = NodeLocalState::load(&state_path).expect("state reloads");
    let current_state = store.snapshot().await.expect("state snapshot");
    std::fs::remove_file(state_path).expect("node state fixture is removed");

    assert!(!enrolled);
    assert_eq!(current_state.enrollment_id, None);
    assert_eq!(current_state.pairing_code, None);
    assert_eq!(saved_state.enrollment_id, None);
    assert_eq!(saved_state.pairing_code, None);
}
