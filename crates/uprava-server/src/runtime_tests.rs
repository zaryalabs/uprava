use std::sync::{Mutex, MutexGuard, OnceLock};

use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower::ServiceExt;

use super::*;

const CORE_CONFIG_ENV_VARS: &[&str] = &[
    "UPRAVA_CORE_BIND",
    "UPRAVA_ALLOWED_ORIGINS",
    "UPRAVA_DATABASE_URL",
    "UPRAVA_DEPLOYMENT_PROFILE",
    "UPRAVA_HEARTBEAT_STALE_SECONDS",
    "UPRAVA_HEARTBEAT_OFFLINE_SECONDS",
    "UPRAVA_ENROLLMENT_TTL_SECONDS",
    "UPRAVA_MAX_PENDING_ENROLLMENTS",
    "UPRAVA_RUNTIME_EXPIRY_SECONDS",
    "UPRAVA_AUTO_APPROVE_ENROLLMENTS",
    "UPRAVA_AUTO_APPROVE_NODE_NAME",
    "UPRAVA_CLIENT_LOG_FILE",
    "UPRAVA_WEB_AUTH",
    "UPRAVA_WEB_SESSION_TTL_SECONDS",
    "UPRAVA_COOKIE_SECURE",
    "UPRAVA_CORE_SHUTDOWN_TIMEOUT_SECONDS",
];

#[test]
fn public_peer_rate_policy_keeps_sensitive_budgets_independent() {
    assert_eq!(public_peer_rate_policy("/api/v1/auth/status"), ("auth", 30));
    assert_eq!(
        public_peer_rate_policy("/api/v1/node-enrollments"),
        ("enrollment", 30)
    );
    assert_eq!(
        public_peer_rate_policy("/api/v1/client/logs"),
        ("client_logs", 120)
    );
    assert_eq!(
        public_peer_rate_policy("/api/v1/inventory"),
        ("general", PUBLIC_PEER_RATE_LIMIT)
    );
}

async fn test_state() -> Arc<AppState> {
    test_state_with_runtime_expiry(86_400).await
}

async fn activate_test_connection(
    state: &AppState,
    node_id: NodeId,
) -> (NodeContext, mpsc::Receiver<ControlFrame>) {
    let (sender, receiver) = mpsc::channel(CONTROL_QUEUE_CAPACITY);
    let context = state.control_connections.context(node_id, sender);
    state.control_connections.activate(&context).await;
    (context, receiver)
}

async fn test_state_with_runtime_expiry(runtime_expiry_seconds: i64) -> Arc<AppState> {
    let pool = memory_pool().await;
    AppState::new(test_config(runtime_expiry_seconds), pool)
        .await
        .expect("state migrates")
}

async fn test_state_with_web_auth() -> Arc<AppState> {
    let pool = memory_pool().await;
    let mut config = test_config(86_400);
    config.profile = DeploymentProfile::ControlledDev;
    config.web_auth_required = true;
    AppState::new(config, pool).await.expect("state migrates")
}

async fn memory_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true),
        )
        .await
        .expect("sqlite opens");
    pool
}

async fn sqlite_file_pool(path: &std::path::Path) -> SqlitePool {
    sqlite_file_pool_with_connections(path, 1).await
}

async fn sqlite_file_pool_with_connections(
    path: &std::path::Path,
    max_connections: u32,
) -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await
        .expect("sqlite file opens");
    pool
}

fn remove_sqlite_file_set(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(std::path::PathBuf::from(format!(
        "{}-shm",
        path.to_string_lossy()
    )));
    let _ = std::fs::remove_file(std::path::PathBuf::from(format!(
        "{}-wal",
        path.to_string_lossy()
    )));
}

fn test_config(runtime_expiry_seconds: i64) -> AppConfig {
    AppConfig {
        bind_address: "127.0.0.1:0".to_owned(),
        database_url: "sqlite::memory:".to_owned(),
        profile: DeploymentProfile::ControlledDev,
        allowed_origins: default_allowed_origins(),
        stale_after_seconds: 15,
        offline_after_seconds: 45,
        enrollment_ttl_seconds: 600,
        max_pending_enrollments: 100,
        runtime_expiry_seconds,
        auto_approve_enrollments: false,
        auto_approve_node_name: None,
        client_log_file: std::env::temp_dir()
            .join(format!("uprava-client-log-{}.jsonl", Uuid::new_v4())),
        web_auth_required: false,
        web_session_ttl_seconds: 86_400,
        cookie_secure: false,
        core_shutdown_timeout_seconds: 5,
    }
}

fn set_cookie_header(response: &axum::response::Response) -> String {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split_once(';').map(|(cookie, _)| cookie.to_owned()))
        .collect::<Vec<_>>()
        .join("; ")
}

fn csrf_from_cookie_header(cookie_header: &str) -> String {
    cookie_header
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(name, value)| (name == CSRF_COOKIE).then(|| value.to_owned()))
        .expect("csrf cookie exists")
}

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock is not poisoned")
}

struct EnvGuard {
    values: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn cleared(names: &[&'static str]) -> Self {
        let values = names
            .iter()
            .map(|name| {
                let value = std::env::var(name).ok();
                std::env::remove_var(name);
                (*name, value)
            })
            .collect();
        Self { values }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in self.values.drain(..) {
            if let Some(value) = value {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}

#[test]
fn app_config_from_env_uses_documented_defaults() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);

    let config = AppConfig::from_env().expect("default core config parses");

    assert_eq!(config.bind_address, "127.0.0.1:8080");
    assert_eq!(
        config.database_url,
        "sqlite://.local/state/core/core.sqlite"
    );
    assert_eq!(config.profile, DeploymentProfile::ControlledDev);
    assert_eq!(
        config
            .allowed_origins
            .iter()
            .map(|origin| origin.to_str().expect("origin is utf8"))
            .collect::<Vec<_>>(),
        vec!["http://127.0.0.1:5173", "http://localhost:5173"]
    );
    assert_eq!(config.stale_after_seconds, 15);
    assert_eq!(config.offline_after_seconds, 45);
    assert_eq!(config.enrollment_ttl_seconds, 600);
    assert_eq!(config.runtime_expiry_seconds, 86_400);
    assert!(!config.auto_approve_enrollments);
    assert_eq!(config.auto_approve_node_name, None);
    assert_eq!(
        config.client_log_file,
        PathBuf::from(".local/logs/client.log")
    );
    assert!(config.web_auth_required);
    assert_eq!(config.web_session_ttl_seconds, 86_400);
    assert!(!config.cookie_secure);
    assert_eq!(config.core_shutdown_timeout_seconds, 5);
}

#[test]
fn app_config_from_env_parses_overrides() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_CORE_BIND", "127.0.0.1:19080");
    std::env::set_var(
        "UPRAVA_ALLOWED_ORIGINS",
        "http://127.0.0.1:5173, http://localhost:4173",
    );
    std::env::set_var("UPRAVA_DATABASE_URL", "sqlite:///tmp/uprava-test.sqlite");
    std::env::set_var("UPRAVA_DEPLOYMENT_PROFILE", "controlled_dev");
    std::env::set_var("UPRAVA_HEARTBEAT_STALE_SECONDS", "3");
    std::env::set_var("UPRAVA_HEARTBEAT_OFFLINE_SECONDS", "9");
    std::env::set_var("UPRAVA_ENROLLMENT_TTL_SECONDS", "30");
    std::env::set_var("UPRAVA_RUNTIME_EXPIRY_SECONDS", "120");
    std::env::set_var("UPRAVA_CLIENT_LOG_FILE", "/tmp/uprava-client.jsonl");
    std::env::set_var("UPRAVA_WEB_SESSION_TTL_SECONDS", "3600");
    std::env::set_var("UPRAVA_COOKIE_SECURE", "true");
    std::env::set_var("UPRAVA_CORE_SHUTDOWN_TIMEOUT_SECONDS", "2");
    std::env::set_var("UPRAVA_AUTO_APPROVE_NODE_NAME", " Zarya Server ");

    let config = AppConfig::from_env().expect("overridden core config parses");

    assert_eq!(config.bind_address, "127.0.0.1:19080");
    assert_eq!(config.database_url, "sqlite:///tmp/uprava-test.sqlite");
    assert_eq!(config.profile, DeploymentProfile::ControlledDev);
    assert_eq!(
        config
            .allowed_origins
            .iter()
            .map(|origin| origin.to_str().expect("origin is utf8"))
            .collect::<Vec<_>>(),
        vec!["http://127.0.0.1:5173", "http://localhost:4173"]
    );
    assert_eq!(config.stale_after_seconds, 3);
    assert_eq!(config.offline_after_seconds, 9);
    assert_eq!(config.enrollment_ttl_seconds, 30);
    assert_eq!(config.runtime_expiry_seconds, 120);
    assert!(!config.auto_approve_enrollments);
    assert_eq!(
        config.auto_approve_node_name.as_deref(),
        Some("Zarya Server")
    );
    assert_eq!(
        config.client_log_file,
        PathBuf::from("/tmp/uprava-client.jsonl")
    );
    assert!(config.web_auth_required);
    assert_eq!(config.web_session_ttl_seconds, 3600);
    assert!(config.cookie_secure);
    assert_eq!(config.core_shutdown_timeout_seconds, 2);
}

#[test]
fn app_config_from_env_rejects_invalid_profile() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_DEPLOYMENT_PROFILE", "production");

    let error = AppConfig::from_env().expect_err("invalid profile should fail");

    assert!(matches!(error, ConfigError::InvalidProfile(profile) if profile == "production"));
}

#[test]
fn app_config_from_env_rejects_local_trusted_profile() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_DEPLOYMENT_PROFILE", "local_trusted");

    let error = AppConfig::from_env().expect_err("local_trusted profile should fail");

    assert!(matches!(error, ConfigError::InvalidProfile(profile) if profile == "local_trusted"));
}

#[test]
fn app_config_from_env_rejects_disabled_web_auth() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_WEB_AUTH", "disabled");

    let error = AppConfig::from_env().expect_err("disabled web auth should fail");

    assert!(matches!(error, ConfigError::InvalidWebAuthMode(mode) if mode == "disabled"));
}

#[test]
fn app_config_from_env_rejects_auto_approve_enrollments() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_AUTO_APPROVE_ENROLLMENTS", "yes");

    let error = AppConfig::from_env().expect_err("auto approval should fail");

    assert!(matches!(error, ConfigError::AutoApproveEnrollments));
}

#[test]
fn app_config_from_env_rejects_invalid_integer() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_HEARTBEAT_STALE_SECONDS", "fast");

    let error = AppConfig::from_env().expect_err("invalid integer should fail");

    assert!(matches!(
        error,
        ConfigError::InvalidInteger { name, .. }
            if name == "UPRAVA_HEARTBEAT_STALE_SECONDS"
    ));
}

#[test]
fn app_config_from_env_rejects_wildcard_cors_origin() {
    let _lock = env_lock();
    let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
    std::env::set_var("UPRAVA_ALLOWED_ORIGINS", "*");

    let error = AppConfig::from_env().expect_err("wildcard origin should fail");

    assert!(matches!(error, ConfigError::WildcardOrigin));
}

#[tokio::test]
async fn migration_creates_baseline_schema_from_empty_database() {
    let state = test_state().await;

    let table_count: i64 = sqlx::query_scalar(
        r#"
            select count(*)
            from sqlite_master
            where type = 'table'
              and name in (
                  'nodes',
                  'node_enrollments',
                  'node_capabilities',
                  'actors',
                  'projects',
                  'project_placements',
                  'session_threads',
                  'runtime_sessions',
                  'turns',
                  'approvals',
                  'messages',
                  'commands',
                  'events',
                  'warning_acknowledgements',
                  'event_publication_outbox',
                  'command_dispatch_outbox'
              )
            "#,
    )
    .fetch_one(&state.pool)
    .await
    .expect("baseline tables count loads");

    assert_eq!(table_count, 16);

    let applied_versions: Vec<i64> =
        sqlx::query_scalar("select version from schema_migrations order by version")
            .fetch_all(&state.pool)
            .await
            .expect("migration versions load");
    assert_eq!(applied_versions, vec![1, 2, 3, 4, 5, 6, 7]);

    let metadata: (String, i64) =
        sqlx::query_as("select slot, schema_version from core_schema_meta")
            .fetch_one(&state.pool)
            .await
            .expect("core schema metadata loads");
    assert_eq!(metadata, (CORE_STATE_SLOT.to_owned(), SCHEMA_VERSION));
}

#[tokio::test]
async fn migration_runner_is_idempotent_and_does_not_duplicate_versions() {
    let pool = memory_pool().await;
    let state = AppState::new(test_config(86_400), pool)
        .await
        .expect("state migrates");
    state
        .migrate()
        .await
        .expect("second migration run succeeds");

    let migration_count: i64 = sqlx::query_scalar("select count(*) from schema_migrations")
        .fetch_one(&state.pool)
        .await
        .expect("migration count loads");
    assert_eq!(migration_count, 7);
}

#[tokio::test]
async fn migration_runner_rejects_changed_applied_migration() {
    let state = test_state().await;
    sqlx::query("update schema_migrations set checksum = 'tampered' where version = 1")
        .execute(&state.pool)
        .await
        .expect("migration metadata updates");

    let error = state
        .migrate()
        .await
        .expect_err("tampered migration must be rejected");
    assert!(error.to_string().contains("checksum mismatch"));
}

#[tokio::test]
async fn migration_adds_core_identity_constraints() {
    let state = test_state().await;
    let index_count: i64 = sqlx::query_scalar(
            "select count(*) from sqlite_master where type = 'index' and name in ('project_placements_identity_idx', 'runtime_sessions_session_thread_idx', 'approvals_turn_idx', 'messages_source_event_idx')",
        )
        .fetch_one(&state.pool)
        .await
        .expect("invariant indexes load");
    assert_eq!(index_count, 4);

    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let duplicate = sqlx::query(
        r#"
            insert into runtime_sessions (
                runtime_session_id, session_thread_id, provider, state,
                resume_supported, degraded_reason, created_at, updated_at
            ) values (?1, ?2, 'codex', 'starting', 1, null, ?3, ?3)
            "#,
    )
    .bind("duplicate-runtime-session")
    .bind(detail.session.session_thread_id.as_str())
    .bind(Utc::now())
    .execute(&state.pool)
    .await;
    assert!(duplicate.is_err());

    let duplicate_placement = sqlx::query(
        r#"
            insert into project_placements (
                project_placement_id, project_id, node_id, display_name, workspace_path,
                state, resource_badges_json, last_validated_at, created_at, updated_at
            ) values (?1, ?2, ?3, 'duplicate', ?4, 'pending', '[]', null, ?5, ?5)
            "#,
    )
    .bind("duplicate-placement")
    .bind("different-project")
    .bind(detail.placement.node_id.as_str())
    .bind(detail.placement.workspace_path.as_str())
    .bind(Utc::now())
    .execute(&state.pool)
    .await;
    assert!(duplicate_placement.is_err());

    let now = Utc::now();
    for approval_id in ["approval-1", "approval-2"] {
        let result = sqlx::query(
            r#"
                insert into approvals (
                    approval_id, session_thread_id, turn_id, state,
                    request_payload_json, created_at, updated_at
                ) values (?1, ?2, 'turn-unique', 'requested', '{}', ?3, ?3)
                "#,
        )
        .bind(approval_id)
        .bind(detail.session.session_thread_id.as_str())
        .bind(now)
        .execute(&state.pool)
        .await;
        if approval_id == "approval-1" {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    for message_id in ["message-1", "message-2"] {
        let result = sqlx::query(
            r#"
                insert into messages (
                    message_id, session_thread_id, role, content, source_event_id, created_at
                ) values (?1, ?2, 'assistant', 'duplicate source', 'event-unique', ?3)
                "#,
        )
        .bind(message_id)
        .bind(detail.session.session_thread_id.as_str())
        .bind(now)
        .execute(&state.pool)
        .await;
        if message_id == "message-1" {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn migration_rejects_mismatched_state_slot_before_schema_writes() {
    let pool = memory_pool().await;
    sqlx::query(
            "create table core_schema_meta (slot text primary key, schema_version integer not null, created_at text not null, updated_at text not null)",
        )
        .execute(&pool)
        .await
        .expect("schema metadata table creates");
    sqlx::query(
            "insert into core_schema_meta (slot, schema_version, created_at, updated_at) values ('0.1.8', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("legacy metadata inserts");

    let error = match AppState::new(test_config(86_400), pool.clone()).await {
        Ok(_) => panic!("mismatched state slot must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("expected slot 0.2.0"));
    let migration_table_count: i64 = sqlx::query_scalar(
        "select count(*) from sqlite_master where type = 'table' and name = 'schema_migrations'",
    )
    .fetch_one(&pool)
    .await
    .expect("migration table count loads");
    assert_eq!(migration_table_count, 0);
}

#[tokio::test]
async fn migration_rejects_partial_unversioned_schema_before_writes() {
    let pool = memory_pool().await;
    sqlx::query("create table legacy_partial_state (value text)")
        .execute(&pool)
        .await
        .expect("partial legacy table creates");

    let error = match AppState::new(test_config(86_400), pool.clone()).await {
        Ok(_) => panic!("partial unversioned state must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("retained 0.1.x Core state"));
    let migration_table_count: i64 = sqlx::query_scalar(
        "select count(*) from sqlite_master where type = 'table' and name = 'schema_migrations'",
    )
    .fetch_one(&pool)
    .await
    .expect("migration table count loads");
    assert_eq!(migration_table_count, 0);
}

#[tokio::test]
async fn migration_rejects_multiple_core_schema_metadata_rows() {
    let pool = memory_pool().await;
    sqlx::query(
            "create table core_schema_meta (slot text primary key, schema_version integer not null, created_at text not null, updated_at text not null)",
        )
        .execute(&pool)
        .await
        .expect("schema metadata table creates");
    sqlx::query(
            "insert into core_schema_meta (slot, schema_version, created_at, updated_at) values ('0.2.0', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("first metadata row inserts");
    sqlx::query(
            "insert into core_schema_meta (slot, schema_version, created_at, updated_at) values ('0.2.0-duplicate', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("second metadata row inserts");

    let error = match AppState::new(test_config(86_400), pool).await {
        Ok(_) => panic!("multiple metadata rows must be rejected"),
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("exactly one Core schema metadata"));
}

#[tokio::test]
async fn migration_concurrent_file_backed_starts_share_one_numbered_history() {
    let db_path = std::env::temp_dir().join(format!("uprava-migrations-{}.sqlite", Uuid::new_v4()));
    remove_sqlite_file_set(&db_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await
        .expect("migration pool opens");
    let config = test_config(86_400);
    let (first, second) = tokio::join!(
        AppState::new(config.clone(), pool.clone()),
        AppState::new(config, pool.clone()),
    );
    let first = first.expect("first migration succeeds");
    let second = second.expect("second migration succeeds");
    let count: i64 = sqlx::query_scalar("select count(*) from schema_migrations")
        .fetch_one(&pool)
        .await
        .expect("migration count loads");
    assert_eq!(count, 7);
    drop(first);
    drop(second);
    pool.close().await;
    remove_sqlite_file_set(&db_path);
}

#[tokio::test]
async fn migration_rejects_unversioned_previous_dev_nodes_table_without_mutation() {
    let pool = memory_pool().await;
    sqlx::query(
        r#"
            create table nodes (
                node_id text primary key,
                display_name text not null,
                presence text not null,
                sleep_hint text not null,
                last_heartbeat_at text,
                daemon_version text not null,
                active_runtime_count integer not null default 0,
                capabilities_json text not null,
                diagnostics text not null,
                created_at text not null,
                updated_at text not null
            )
            "#,
    )
    .execute(&pool)
    .await
    .expect("legacy nodes table creates");

    let error = match AppState::new(test_config(86_400), pool.clone()).await {
        Ok(_) => panic!("unversioned state must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("retained 0.1.x Core state"));
    let column_count: i64 = sqlx::query_scalar(
        "select count(*) from pragma_table_info('nodes') where name = 'credential_hash'",
    )
    .fetch_one(&pool)
    .await
    .expect("nodes columns load");

    assert_eq!(column_count, 0);
}

#[tokio::test]
async fn core_state_survives_sqlite_reopen() {
    let db_path = std::env::temp_dir().join(format!("uprava-core-{}.sqlite", Uuid::new_v4()));
    remove_sqlite_file_set(&db_path);
    let state = AppState::new(test_config(86_400), sqlite_file_pool(&db_path).await)
        .await
        .expect("file state migrates");
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let placement_id = detail.placement.project_placement_id.clone();
    let session_id = detail.session.session_thread_id.clone();
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
    state.pool.close().await;
    drop(state);

    let reopened = AppState::new(test_config(86_400), sqlite_file_pool(&db_path).await)
        .await
        .expect("file state remigrates");
    let inventory = load_inventory(&reopened)
        .await
        .expect("inventory loads after reopen");
    let node_persisted = inventory.nodes.iter().any(|node| node.node_id == node_id);
    let placement_persisted = inventory
        .placements
        .iter()
        .any(|placement| placement.project_placement_id == placement_id);
    let session_persisted = inventory
        .sessions
        .iter()
        .any(|session| session.session_thread_id == session_id);
    reopened.pool.close().await;
    drop(reopened);
    remove_sqlite_file_set(&db_path);

    assert!(node_persisted);
    assert!(placement_persisted);
    assert!(session_persisted);
}

#[tokio::test]
async fn session_history_and_read_models_survive_sqlite_reopen() {
    let db_path = std::env::temp_dir().join(format!("uprava-core-{}.sqlite", Uuid::new_v4()));
    remove_sqlite_file_set(&db_path);
    let state = AppState::new(test_config(86_400), sqlite_file_pool(&db_path).await)
        .await
        .expect("file state migrates");
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let session_id = detail.session.session_thread_id.clone();
    let event_id = EventId::from("persisted-provider-completed-1");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            event_id.as_str(),
            1,
            EventKind::ProviderMessageCompleted,
            json!({ "content": "persisted assistant" }),
        ),
    )
    .await
    .expect("provider event accepts");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
    state.pool.close().await;
    drop(state);

    let reopened = AppState::new(test_config(86_400), sqlite_file_pool(&db_path).await)
        .await
        .expect("file state remigrates");
    let detail = load_session_detail(&reopened, &session_id)
        .await
        .expect("session detail reloads after reopen");
    let evidence_projection = build_session_evidence_projection(&reopened, &session_id)
        .await
        .expect("evidence projection rebuilds after reopen");
    let projection = build_agent_projection(&reopened, &session_id)
        .await
        .expect("agent projection rebuilds after reopen");
    reopened.pool.close().await;
    drop(reopened);
    remove_sqlite_file_set(&db_path);

    let assistant_message = detail
        .messages
        .iter()
        .find(|message| {
            message.role == MessageRole::Assistant
                && message.content == "persisted assistant"
                && message.source_event_id.as_ref() == Some(&event_id)
        })
        .expect("assistant message persisted");
    assert!(detail.events.iter().any(|event| event.event_id == event_id));
    assert!(evidence_projection
        .root
        .children
        .iter()
        .any(|node| matches!(
            &node.primary_ref,
            UpravaRef::Message { message_id } if message_id == &assistant_message.message_id
        )));
    assert!(evidence_projection
        .root
        .children
        .iter()
        .any(|node| matches!(
            &node.primary_ref,
            UpravaRef::Event {
                event_id: artifact_event_id,
                ..
            } if artifact_event_id == &event_id
        )));
    assert!(projection
        .recent_message_refs
        .iter()
        .any(|reference| matches!(
                reference,
                UpravaRef::Message { message_id } if message_id == &assistant_message.message_id
        )));
    assert!(projection
        .evidence_projection_summary
        .contains("1 messages, 1 events"));
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header(CORRELATION_ID_HEADER, "health-correlation")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CORRELATION_ID_HEADER),
        Some(&HeaderValue::from_static("health-correlation"))
    );
}

#[tokio::test]
async fn public_rate_limit_does_not_starve_health() {
    let app = build_router(test_state().await);
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/status")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let limited = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/status")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

    let health = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(health.status(), StatusCode::OK);
}

#[tokio::test]
async fn general_local_traffic_does_not_consume_enrollment_budget() {
    let app = build_router(test_state().await);
    for _ in 0..31 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/inventory")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let enrollment = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/node-enrollments")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(enrollment.status(), StatusCode::OK);
}

#[tokio::test]
async fn response_header_and_error_body_share_correlation_id() {
    let response = build_router(test_state().await)
        .oneshot(
            Request::builder()
                .uri("/api/v1/nodes/missing")
                .header(CORRELATION_ID_HEADER, "correlation-response")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get(CORRELATION_ID_HEADER),
        Some(&HeaderValue::from_static("correlation-response"))
    );
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("error body loads");
    let error: ApiError = serde_json::from_slice(&body).expect("error body decodes");
    assert_eq!(error.correlation_id.as_str(), "correlation-response");
}

#[tokio::test]
async fn metrics_endpoint_exposes_bounded_core_counters() {
    let state = test_state().await;
    state
        .core_metrics
        .accepted_events
        .store(3, Ordering::Relaxed);
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/metrics")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("metrics body loads");
    let body = String::from_utf8(body.to_vec()).expect("metrics are utf8");
    assert!(body.contains("uprava_core_events_accepted_total 3"));
    assert!(body.contains("uprava_core_command_results_total 0"));
    assert!(body.contains("uprava_core_auth_failures_total 0"));
    assert!(body.contains("uprava_core_log_records_dropped_total"));
}

#[tokio::test]
async fn router_rejects_oversized_request_body() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/client/logs")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b'a'; MAX_HTTP_REQUEST_BODY_BYTES + 1]))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn client_logs_endpoint_appends_local_jsonl_record() {
    let state = test_state().await;
    let log_path = state.config.client_log_file.clone();
    let app = build_router(state);
    let request = ClientLogRequest {
        level: ClientLogLevel::Error,
        source: "web.global_error".to_owned(),
        message: "render failed".to_owned(),
        route: Some("/nodes".to_owned()),
        user_agent: Some("vitest".to_owned()),
        occurred_at: Utc::now(),
        detail: JsonValue(json!({ "component": "NodesRoute" })),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/client/logs")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let log_content = std::fs::read_to_string(&log_path).expect("client log file reads");
    let first_line = log_content.lines().next().expect("client log line exists");
    let record: serde_json::Value =
        serde_json::from_str(first_line).expect("client log json parses");
    let _ = std::fs::remove_file(log_path);

    assert_eq!(status, StatusCode::OK);
    assert!(
        serde_json::from_slice::<ClientLogResponse>(&body)
            .expect("response parses")
            .accepted
    );
    assert_eq!(record["level"], "error");
    assert_eq!(record["source"], "web.global_error");
    assert_eq!(record["message"], "render failed");
    assert!(record["detail"]
        .as_str()
        .expect("detail is bounded string")
        .contains("NodesRoute"));
}

#[tokio::test]
async fn client_log_retention_rotates_a_full_file() {
    let path =
        std::env::temp_dir().join(format!("uprava-client-rotation-{}.jsonl", Uuid::new_v4()));
    let file = std::fs::File::create(&path).expect("client log creates");
    file.set_len(MAX_CLIENT_LOG_BYTES)
        .expect("client log fills");

    append_jsonl_log(path.clone(), "{\"message\":\"next\"}".to_owned())
        .await
        .expect("client log rotates");

    assert!(path.exists());
    assert!(PathBuf::from(format!("{}.1", path.display())).exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(PathBuf::from(format!("{}.1", path.display())));
}

#[tokio::test]
async fn cors_preflight_allows_configured_loopback_origin() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/health")
                .header("origin", "http://127.0.0.1:5173")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("http://127.0.0.1:5173")
    );
}

#[tokio::test]
async fn cors_preflight_rejects_unknown_origin() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/health")
                .header("origin", "https://example.com")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn hardened_web_auth_requires_setup_before_client_routes() {
    let app = build_router(test_state_with_web_auth().await);

    let protected = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inventory")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/status")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let body = to_bytes(status.into_body(), 64 * 1024)
        .await
        .expect("status body loads");
    let auth_status: WebAuthStatusResponse =
        serde_json::from_slice(&body).expect("auth status parses");

    assert_eq!(protected.status(), StatusCode::UNAUTHORIZED);
    assert!(auth_status.auth_required);
    assert!(auth_status.setup_required);
    assert!(!auth_status.authenticated);
}

#[test]
fn password_hash_uses_argon2id_and_rejects_legacy_sha256_records() {
    let password = "very-secure-local-password";
    let hash = hash_password(password).expect("Argon2id hash creates");
    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password(&hash, password));
    assert!(!verify_password(&hash, "wrong-password"));
    assert!(!verify_password("pwd-sha256:salt:digest", password));
}

#[tokio::test]
async fn hardened_web_auth_sets_session_and_enforces_csrf_for_mutations() {
    let app = build_router(test_state_with_web_auth().await);
    let setup = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/setup")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&WebAuthSetupRequest {
                        password: "very-secure-local-password".to_owned(),
                    })
                    .expect("setup serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let cookie_header = set_cookie_header(&setup);
    let csrf_token = csrf_from_cookie_header(&cookie_header);

    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inventory")
                .header(COOKIE, cookie_header.as_str())
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let rejected_mutation = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node-enrollments")
                .header(COOKIE, cookie_header.as_str())
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientCreateNodeEnrollmentRequest {
                        display_name: "Secure node".to_owned(),
                    })
                    .expect("enrollment serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let accepted_mutation = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node-enrollments")
                .header(COOKIE, cookie_header.as_str())
                .header(CSRF_HEADER, csrf_token)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientCreateNodeEnrollmentRequest {
                        display_name: "Secure node".to_owned(),
                    })
                    .expect("enrollment serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(setup.status(), StatusCode::OK);
    assert!(!cookie_header.is_empty());
    assert_eq!(read.status(), StatusCode::OK);
    assert_eq!(rejected_mutation.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(accepted_mutation.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_resource_api_error_uses_safe_envelope_with_correlation_id() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nodes/missing-node")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let envelope: ApiError = serde_json::from_slice(&body).expect("api error envelope parses");

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(envelope.error_code, "node.not_found");
    assert_eq!(envelope.message, "Node not found");
    assert!(!envelope.retryable);
    assert!(!envelope.correlation_id.as_str().is_empty());
}

#[tokio::test]
async fn delete_node_removes_inventory_dependents() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;
    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/nodes/{node_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let deletion: NodeDeletionResponse =
        serde_json::from_slice(&body).expect("node deletion response parses");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(deletion.node_id, node_id);
    assert!(deletion.deleted);
    assert!(!inventory.nodes.iter().any(|node| node.node_id == node_id));
    assert!(!inventory
        .placements
        .iter()
        .any(|placement| placement.node_id == node_id));
    assert!(!inventory
        .sessions
        .iter()
        .any(|session| { session.session_thread_id == detail.session.session_thread_id }));
}

#[tokio::test]
async fn delete_node_removes_deleted_workspace_tombstones() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;

    let placement_deletion = delete_placement(
        State(state.clone()),
        Path(detail.placement.project_placement_id.to_string()),
    )
    .await
    .expect("placement delete succeeds")
    .0;
    let tombstone_count: i64 =
        sqlx::query_scalar("select count(*) from deleted_workspace_bindings where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("tombstone count loads");

    let node_deletion = delete_node(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("node delete succeeds")
        .0;
    let remaining_tombstones: i64 =
        sqlx::query_scalar("select count(*) from deleted_workspace_bindings where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("remaining tombstone count loads");

    assert!(placement_deletion.deleted);
    assert_eq!(tombstone_count, 1);
    assert!(node_deletion.deleted);
    assert_eq!(remaining_tombstones, 0);
}

#[tokio::test]
async fn delete_placement_removes_inventory_dependents_but_keeps_node() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;
    let placement_id = detail.placement.project_placement_id.clone();
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/placements/{placement_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let deletion: PlacementDeletionResponse =
        serde_json::from_slice(&body).expect("placement deletion response parses");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(deletion.project_placement_id, placement_id);
    assert!(deletion.deleted);
    assert!(inventory.nodes.iter().any(|node| node.node_id == node_id));
    assert!(!inventory
        .placements
        .iter()
        .any(|placement| { placement.project_placement_id == deletion.project_placement_id }));
    assert!(!inventory
        .sessions
        .iter()
        .any(|session| { session.session_thread_id == detail.session.session_thread_id }));
}

#[tokio::test]
async fn heartbeat_appears_in_inventory() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let credential = claim.credential.clone();
    let request = NodeHeartbeatRequest {
        node_id: claim.node_id.clone(),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(true),
        }],
        diagnostics: Some("daemon_installation_id=daemon-test".to_owned()),
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };

    let response = node_heartbeat(State(state.clone()), credential.as_deref(), Json(request))
        .await
        .expect("heartbeat accepted");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(response.0.accepted, !inventory.nodes.is_empty());
    assert!(inventory.nodes.iter().any(|node| node
        .diagnostics
        .contains("daemon_installation_id=daemon-test")));
}

#[tokio::test]
async fn heartbeat_route_rejects_credential_in_body_without_bearer() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node/heartbeat")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "node_id": node_id,
                        "credential": claim.credential,
                        "display_name": "Test node",
                        "daemon_version": "0.1.0",
                        "capabilities": [],
                        "diagnostics": null,
                        "active_runtime_count": 0,
                        "sleep_hint": "awake",
                        "workspace_summaries": []
                    }))
                    .expect("heartbeat serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn heartbeat_replaces_normalized_node_capabilities() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let first = NodeHeartbeatRequest {
        node_id: Some(node_id.clone()),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(true),
        }],
        diagnostics: None,
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };
    let _ = node_heartbeat(State(state.clone()), credential.as_deref(), Json(first))
        .await
        .expect("first heartbeat accepted");
    let second = NodeHeartbeatRequest {
        node_id: Some(node_id.clone()),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(false),
        }],
        diagnostics: None,
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };

    let _ = node_heartbeat(State(state.clone()), credential.as_deref(), Json(second))
        .await
        .expect("second heartbeat accepted");
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
            select capability_key, value_json
            from node_capabilities
            where node_id = ?1
            order by capability_key
            "#,
    )
    .bind(node_id.as_str())
    .fetch_all(&state.pool)
    .await
    .expect("capability rows load");

    assert_eq!(rows.len(), 1);
    let codex_capability =
        serde_json::from_str::<JsonValue>(&rows[0].1).expect("capability value json decodes");

    assert_eq!(rows[0].0, "provider.codex");
    assert_eq!(
        codex_capability
            .0
            .get("available")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert!(!node_supports_provider(&state, &node_id, "codex")
        .await
        .expect("provider support checks"));
}

#[tokio::test]
async fn inventory_lists_multiple_heartbeat_nodes_with_activity_counts() {
    let state = test_state().await;
    let first = enroll_test_node(&state).await;
    let second = enroll_test_node(&state).await;
    let first_node_id = first.node_id.expect("first node id returned");
    let second_node_id = second.node_id.expect("second node id returned");
    heartbeat_node(
        &state,
        first_node_id.clone(),
        first.credential,
        "Node one",
        SleepHint::Awake,
        2,
    )
    .await;
    heartbeat_node(
        &state,
        second_node_id.clone(),
        second.credential,
        "Node two",
        SleepHint::Sleeping,
        0,
    )
    .await;

    let inventory = load_inventory(&state).await.expect("inventory loads");
    let first_summary = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == first_node_id)
        .expect("first node visible");
    let second_summary = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == second_node_id)
        .expect("second node visible");

    assert_eq!(first_summary.active_runtime_count, 2);
    assert_eq!(second_summary.sleep_hint, SleepHint::Sleeping);
}

#[tokio::test]
async fn stale_node_keeps_sleep_hint_and_accepts_commands() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_node(
        &state,
        node_id.clone(),
        claim.credential,
        "Sleepy node",
        SleepHint::Sleeping,
        0,
    )
    .await;
    age_node_heartbeat(&state, &node_id, state.config.stale_after_seconds + 1).await;

    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: "/tmp/uprava-stale-node-workspace".to_owned(),
        }),
    )
    .await
    .expect("stale node can still accept commands")
    .0;
    let inventory = load_inventory(&state).await.expect("inventory loads");
    let node = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .expect("node visible");

    assert_eq!(node.presence, NodePresence::Stale);
    assert_eq!(node.sleep_hint, SleepHint::Sleeping);
    assert_eq!(placement.state, PlacementState::Pending);
}

#[tokio::test]
async fn revoked_node_cannot_heartbeat() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let _ = revoke_node(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("node revokes");

    let result = node_heartbeat(
        State(state),
        claim.credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;

    assert!(matches!(result, Err(AppError::Auth { .. })));
}

#[tokio::test]
async fn rotated_node_credential_replaces_previous_credential() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let old_credential = claim.credential.clone();

    let rotation = rotate_node_credential(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("credential rotates")
        .0;
    let old_result = node_heartbeat(
        State(state.clone()),
        old_credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;
    let new_result = node_heartbeat(
        State(state),
        Some(rotation.credential.as_str()),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;

    assert!(matches!(old_result, Err(AppError::Auth { .. })));
    assert!(new_result.is_ok());
}

#[tokio::test]
async fn repeated_invalid_node_credentials_are_rate_limited() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");

    for _ in 0..AUTH_FAILURE_LIMIT {
        let _ = node_heartbeat(
            State(state.clone()),
            Some("wrong-credential"),
            Json(NodeHeartbeatRequest {
                node_id: Some(node_id.clone()),
                display_name: "Test node".to_owned(),
                daemon_version: "0.1.0".to_owned(),
                capabilities: vec![],
                diagnostics: None,
                active_runtime_count: 0,
                sleep_hint: SleepHint::Awake,
                workspace_summaries: vec![],
            }),
        )
        .await;
    }
    let error = node_heartbeat(
        State(state),
        claim.credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await
    .expect_err("valid credential is temporarily rate limited");

    assert!(matches!(
        error,
        AppError::RateLimited {
            code: "auth.rate_limited",
            ..
        }
    ));
}

#[tokio::test]
async fn pending_enrollment_count_is_bounded() {
    let mut config = test_config(86_400);
    config.max_pending_enrollments = 1;
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");
    create_enrollment(&state, "first", None, vec![])
        .await
        .expect("first enrollment is created");

    let error = create_enrollment(&state, "second", None, vec![])
        .await
        .expect_err("pending enrollment limit is enforced");
    assert!(matches!(
        error,
        AppError::RateLimited {
            code: "node_enrollment.pending_limit",
            ..
        }
    ));
}

#[tokio::test]
async fn unapproved_enrollment_claim_remains_pending() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Pending node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id,
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("claim returns pending");

    assert!(claim.pending);
}

#[tokio::test]
async fn matching_production_node_name_is_auto_approved() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");

    let requested = create_enrollment(&state, " Zarya Server ", Some("0.2.2"), vec![])
        .await
        .expect("matching enrollment creates");

    assert_eq!(requested.status, EnrollmentState::Approved);
}

#[tokio::test]
async fn non_matching_node_name_still_requires_approval() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");

    let requested = create_enrollment(&state, "Unexpected Node", Some("0.2.2"), vec![])
        .await
        .expect("non-matching enrollment creates");

    assert_eq!(requested.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn duplicate_production_node_name_is_not_auto_approved() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");
    create_enrollment(&state, "Zarya Server", Some("0.2.2"), vec![])
        .await
        .expect("first enrollment creates");

    let duplicate = create_enrollment(&state, "Zarya Server", Some("0.2.2"), vec![])
        .await
        .expect("duplicate enrollment creates");

    assert_eq!(duplicate.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn approval_moves_enrollment_to_approved_state() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Approved node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");

    let response = approve_node_enrollment(State(state), Path(requested.enrollment_id.to_string()))
        .await
        .expect("enrollment approves")
        .0;

    assert_eq!(response.enrollment.status, EnrollmentState::Approved);
    assert!(response.enrollment.approved_at.is_some());
}

#[tokio::test]
async fn approved_enrollment_claim_registers_node() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Approved node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let _ = approve_node_enrollment(
        State(state.clone()),
        Path(requested.enrollment_id.to_string()),
    )
    .await
    .expect("enrollment approves");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("approved claim registers");
    let enrollment = load_enrollment(&state, &requested.enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(claim.accepted);
    assert!(!claim.pending);
    assert!(claim.node_id.is_some());
    assert!(claim.credential.is_some());
    assert_eq!(enrollment.status, EnrollmentState::Registered);
    let audit_count: i64 = sqlx::query_scalar(
        "select count(*) from security_audit_events where kind = 'node.enrollment.claimed'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("claim audit loads");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn legacy_approved_pending_enrollment_claim_registers_node() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Legacy node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    sqlx::query(
        r#"
            update node_enrollments
            set approved_at = ?1
            where enrollment_id = ?2
            "#,
    )
    .bind(Utc::now())
    .bind(requested.enrollment_id.as_str())
    .execute(&state.pool)
    .await
    .expect("legacy approval stores");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("legacy approved claim registers");

    assert!(claim.accepted);
    assert!(claim.credential.is_some());
}

#[tokio::test]
async fn expired_enrollment_claim_marks_enrollment_expired_without_credential() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Expired node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let enrollment_id = requested.enrollment_id.clone();
    sqlx::query(
        r#"
            update node_enrollments
            set expires_at = ?1
            where enrollment_id = ?2
            "#,
    )
    .bind(Utc::now() - chrono::Duration::seconds(1))
    .bind(enrollment_id.as_str())
    .execute(&state.pool)
    .await
    .expect("enrollment expiry rewinds");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("expired claim returns safe response");
    let enrollment = load_enrollment(&state, &enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(!claim.accepted);
    assert!(!claim.pending);
    assert_eq!(claim.node_id, None);
    assert_eq!(claim.credential, None);
    assert_eq!(claim.message, "Enrollment expired");
    assert_eq!(enrollment.status, EnrollmentState::Expired);
}

#[tokio::test]
async fn invalid_pairing_code_rejects_claim_with_safe_error() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Invalid code node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let enrollment_id = requested.enrollment_id.clone();

    let error = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: enrollment_id.clone(),
            pairing_code: "wrong-pairing-code".to_owned(),
        },
    )
    .await
    .expect_err("invalid pairing code rejects");
    let enrollment = load_enrollment(&state, &enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(matches!(
        error,
        AppError::Auth {
            code: "auth_dev.invalid_pairing_code",
            message
        } if message == "Pairing code is invalid"
    ));
    assert_eq!(enrollment.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn heartbeat_upserts_node_reported_workspace() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let credential = claim.credential.clone();
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id,
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path.display().to_string(),
                PlacementState::Pending,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    let placement = inventory
        .placements
        .iter()
        .find(|placement| placement.workspace_path == workspace_path.display().to_string())
        .expect("heartbeat workspace placement appears in inventory");
    let project_id = placement
        .project_id
        .clone()
        .expect("heartbeat workspace placement has project id");
    let project_display_name: String =
        sqlx::query_scalar("select display_name from projects where project_id = ?1")
            .bind(project_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("heartbeat project row loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(project_display_name, "workspace");
}

#[tokio::test]
async fn heartbeat_updates_a_manually_created_workspace_binding() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let workspace_path_buf = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let workspace_path = workspace_path_buf.display().to_string();
    let existing_placement_id = ProjectPlacementId::new();
    let existing_project_id = ProjectId::new();
    let now = Utc::now();
    std::fs::create_dir_all(&workspace_path_buf).expect("workspace dir creates");

    sqlx::query(
        r#"
        insert into projects (project_id, display_name, repo_id, created_at, updated_at)
        values (?1, 'Manual workspace', null, ?2, ?2)
        "#,
    )
    .bind(existing_project_id.as_str())
    .bind(now)
    .execute(&state.pool)
    .await
    .expect("manual project inserts");
    sqlx::query(
        r#"
        insert into project_placements (
            project_placement_id, project_id, node_id, display_name, workspace_path,
            state, resource_badges_json, last_validated_at, created_at, updated_at
        ) values (?1, ?2, ?3, 'Manual workspace', ?4, 'pending', '[]', ?5, ?5, ?5)
        "#,
    )
    .bind(existing_placement_id.as_str())
    .bind(existing_project_id.as_str())
    .bind(node_id.as_str())
    .bind(&workspace_path)
    .bind(now)
    .execute(&state.pool)
    .await
    .expect("manual workspace binding inserts");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "Node workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepts an existing workspace binding");

    let placement = load_placement(&state, &existing_placement_id)
        .await
        .expect("manual workspace binding remains available");
    std::fs::remove_dir_all(&workspace_path_buf).expect("workspace dir removes");

    assert_eq!(placement.display_name, "Node workspace");
    assert_eq!(placement.state, PlacementState::Validated);
}

#[tokio::test]
async fn delete_placement_tombstones_node_reported_workspace_until_explicit_validate() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let workspace_path_buf = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let workspace_path = workspace_path_buf.display().to_string();
    std::fs::create_dir_all(&workspace_path_buf).expect("workspace dir creates");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id.clone(),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let placement = load_inventory(&state)
        .await
        .expect("inventory loads")
        .placements
        .into_iter()
        .find(|placement| placement.workspace_path == workspace_path)
        .expect("heartbeat placement appears");
    let app = build_router(state.clone());

    let delete_response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/placements/{}",
                    placement.project_placement_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id,
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let inventory_after_heartbeat = load_inventory(&state).await.expect("inventory loads");

    let explicit_placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id,
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.clone(),
        }),
    )
    .await
    .expect("explicit validation recreates placement")
    .0;
    std::fs::remove_dir_all(&workspace_path_buf).expect("workspace dir removes");

    assert_eq!(delete_response.status(), StatusCode::OK);
    assert!(!inventory_after_heartbeat
        .placements
        .iter()
        .any(|placement| placement.workspace_path == workspace_path));
    assert_eq!(explicit_placement.workspace_path, workspace_path);
}

#[tokio::test]
async fn validate_placement_records_node_command_and_pending_state() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));

    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validation command records")
    .0;
    let reused = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "renamed workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("existing placement is reused")
    .0;
    assert_eq!(reused.project_placement_id, placement.project_placement_id);
    let (command_kind, command_json): (String, String) = sqlx::query_as(
            "select kind, command_json from commands where target_node_id = ?1 order by created_at desc limit 1",
        )
        .bind(node_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("command loads");
    let command =
        serde_json::from_str::<CommandEnvelope>(&command_json).expect("command json decodes");
    let project_id = placement
        .project_id
        .clone()
        .expect("placement has project id");
    let project_display_name: String =
        sqlx::query_scalar("select display_name from projects where project_id = ?1")
            .bind(project_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("project row loads");

    assert_eq!(placement.state, PlacementState::Pending);
    assert_eq!(project_display_name, "workspace");
    assert_eq!(command_kind, "ValidateWorkspace");
    assert_eq!(
        command.target.project_placement_id().cloned(),
        Some(placement.project_placement_id.clone())
    );
    assert!(should_open_control_channel(&state, &node_id)
        .await
        .expect("channel request evaluates"));
}

#[tokio::test]
async fn concurrent_validate_placement_reuses_canonical_workspace_identity() {
    let db_path = std::env::temp_dir().join(format!("uprava-test-{}.sqlite", Uuid::new_v4()));
    let pool = sqlite_file_pool_with_connections(&db_path, 4).await;
    let state = AppState::new(test_config(86_400), pool)
        .await
        .expect("state migrates");
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let request = CreatePlacementRequest {
        node_id: node_id.clone(),
        display_name: "workspace".to_owned(),
        workspace_path: workspace_path.display().to_string(),
    };

    let (first, second) = tokio::join!(
        validate_placement(State(state.clone()), Json(request.clone())),
        validate_placement(
            State(state.clone()),
            Json(CreatePlacementRequest {
                display_name: "renamed workspace".to_owned(),
                ..request
            })
        ),
    );
    let first = first.expect("first placement validates").0;
    let second = second.expect("second placement validates").0;
    let placement_count: i64 = sqlx::query_scalar(
        "select count(*) from project_placements where node_id = ?1 and workspace_path = ?2",
    )
    .bind(node_id.as_str())
    .bind(&first.workspace_path)
    .fetch_one(&state.pool)
    .await
    .expect("placement count loads");
    let project_count: i64 = sqlx::query_scalar("select count(*) from projects")
        .fetch_one(&state.pool)
        .await
        .expect("project count loads");
    state.pool.close().await;
    remove_sqlite_file_set(&db_path);

    assert_eq!(first.project_placement_id, second.project_placement_id);
    assert_eq!(placement_count, 1);
    assert_eq!(project_count, 1);
}

#[tokio::test]
async fn command_api_uses_request_correlation_id_header() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/project-placements/validate")
                .header(CONTENT_TYPE, "application/json")
                .header(CORRELATION_ID_HEADER, "corr-http-1")
                .body(Body::from(
                    serde_json::to_vec(&CreatePlacementRequest {
                        node_id: node_id.clone(),
                        display_name: "workspace".to_owned(),
                        workspace_path: workspace_path.display().to_string(),
                    })
                    .expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let command_json: String = sqlx::query_scalar(
            "select command_json from commands where target_node_id = ?1 order by created_at desc limit 1",
        )
        .bind(node_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("command loads");
    let command =
        serde_json::from_str::<CommandEnvelope>(&command_json).expect("command json decodes");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(command.correlation_id.as_str(), "corr-http-1");
}

#[tokio::test]
async fn validate_placement_rejects_offline_node() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));

    let result = validate_placement(
        State(state),
        Json(CreatePlacementRequest {
            node_id,
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "node.offline",
            ..
        })
    ));
}

#[tokio::test]
async fn workspace_validated_event_updates_pending_placement() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validation command records")
    .0;

    accept_workspace_validation_event(
        &state,
        &placement,
        node_id,
        PlacementState::Validated,
        vec![ResourceBadge {
            kind: "git_workspace".to_owned(),
            severity: WarningSeverity::Info,
            label: "Git workspace".to_owned(),
        }],
    )
    .await;
    let placement = load_placement(&state, &placement.project_placement_id)
        .await
        .expect("placement reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(placement.state, PlacementState::Validated);
    assert_eq!(placement.resource_badges[0].kind, "git_workspace");
}

#[tokio::test]
async fn placement_projection_warns_when_workspace_has_active_session() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;

    let placement = load_placement(&state, &detail.placement.project_placement_id)
        .await
        .expect("placement reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(placement.resource_badges.iter().any(|badge| {
        badge.kind == "same_workspace_active" && badge.severity == WarningSeverity::Warning
    }));
}

#[tokio::test]
async fn session_detail_does_not_warn_about_its_own_active_runtime() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;

    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(!detail
        .placement
        .resource_badges
        .iter()
        .any(|badge| badge.kind == "same_workspace_active"));
}

#[tokio::test]
async fn second_session_on_same_workspace_is_allowed_and_warns() {
    let state = test_state().await;
    let (_node_id, first_detail, workspace_path) = create_test_session(&state).await;

    let second_detail = create_session(
        State(state.clone()),
        Json(CreateSessionRequest {
            project_placement_id: first_detail.placement.project_placement_id,
            title: Some("Second session".to_owned()),
            provider: "codex".to_owned(),
        }),
    )
    .await
    .expect("second session starts with warning")
    .0;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(second_detail
        .placement
        .resource_badges
        .iter()
        .any(|badge| badge.kind == "same_workspace_active"));
}

#[tokio::test]
async fn refresh_resource_snapshot_records_node_command() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validation command records")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id,
        PlacementState::Validated,
        vec![],
    )
    .await;

    let response = refresh_resource_snapshot(
        State(state.clone()),
        Path(placement.project_placement_id.to_string()),
    )
    .await
    .expect("resource snapshot refresh records")
    .0;
    let (command_kind, command_json): (String, String) =
        sqlx::query_as("select kind, command_json from commands where command_id = ?1")
            .bind(response.command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command loads");
    let command =
        serde_json::from_str::<CommandEnvelope>(&command_json).expect("command json decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command_kind, "RefreshResourceSnapshot");
    assert_eq!(
        command.target.project_placement_id().cloned(),
        Some(placement.project_placement_id)
    );
}

#[tokio::test]
async fn create_session_rejects_missing_provider_capability_without_recording_command() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validation command records")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id,
        PlacementState::Validated,
        vec![],
    )
    .await;
    let command_count_before = command_count(&state).await;

    let result = create_session(
        State(state.clone()),
        Json(CreateSessionRequest {
            project_placement_id: placement.project_placement_id,
            title: Some("Unsupported session".to_owned()),
            provider: "opencode".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "node.capability_missing",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
}

#[tokio::test]
async fn resource_snapshot_event_updates_placement_state() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validation command records")
    .0;
    let warning = ResourceBadge {
        kind: "dirty_workspace".to_owned(),
        severity: WarningSeverity::Warning,
        label: "Dirty workspace".to_owned(),
    };

    accept_placement_snapshot_event(
        &state,
        &placement,
        node_id,
        EventKind::ResourceSnapshotUpdated,
        PlacementState::Validated,
        vec![warning],
    )
    .await;
    let placement = load_placement(&state, &placement.project_placement_id)
        .await
        .expect("placement reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(placement.state, PlacementState::Validated);
    assert_eq!(placement.resource_badges[0].kind, "dirty_workspace");
}

#[tokio::test]
async fn node_provider_completed_event_creates_assistant_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "provider-completed-1",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "from node" }),
    );

    accept_node_event(&state, event)
        .await
        .expect("node event accepts");
    let messages = load_messages(&state, &detail.session.session_thread_id)
        .await
        .expect("messages load");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(messages
        .iter()
        .any(|message| message.role == MessageRole::Assistant && message.content == "from node"));
}

#[tokio::test]
async fn provider_activity_event_persists_without_creating_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "provider-activity-1",
        1,
        EventKind::ProviderActivity,
        json!({
            "provider": "codex",
            "source": "codex.exec.jsonl",
            "provider_event_type": "item.completed",
            "raw_event": {
                "type": "item.completed",
                "unknown_future_field": true
            }
        }),
    );

    accept_node_event(&state, event)
        .await
        .expect("provider activity event accepts");
    let persisted_events: i64 =
        sqlx::query_scalar("select count(*) from events where event_id = 'provider-activity-1'")
            .fetch_one(&state.pool)
            .await
            .expect("provider activity event count loads");
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(persisted_events, 1);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn pending_command_requests_control_channel_and_dispatches_after_connect() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let command_id = CommandId::from("pending-command-1");

    record_and_dispatch_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");

    assert!(should_open_control_channel(&state, &node_id)
        .await
        .expect("channel request evaluates"));

    let (_context, mut rx) = activate_test_connection(&state, node_id.clone()).await;
    dispatch_pending_commands(&state, &node_id)
        .await
        .expect("pending command dispatches");
    let frame = rx.recv().await.expect("dispatch frame is sent");
    let command_state: String =
        sqlx::query_scalar("select state from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command state loads");

    assert!(matches!(
        frame,
        ControlFrame::CommandDispatch { command, .. }
            if command.command_id == command_id
    ));
    assert_eq!(command_state, "dispatched");
    let attempts: i64 =
        sqlx::query_scalar("select attempts from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("outbox attempts load");
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn command_dispatch_outbox_is_idempotent_and_clears_on_terminal_result() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let command_id = CommandId::from("outbox-idempotent-command-1");
    let command = command_fixture(command_id.clone(), node_id.clone());

    record_command(&state, command.clone())
        .await
        .expect("command records");
    let duplicate = record_command(&state, command).await;
    assert!(duplicate.is_err(), "command id remains database-idempotent");
    let outbox_count: i64 =
        sqlx::query_scalar("select count(*) from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("outbox count loads");
    assert_eq!(outbox_count, 1);

    update_command_result(
        &state,
        &command_id,
        CommandState::Completed,
        &JsonValue(json!({"ok": true})),
    )
    .await
    .expect("terminal result stores");
    let remaining: i64 =
        sqlx::query_scalar("select count(*) from command_dispatch_outbox where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("outbox cleanup loads");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn acknowledged_command_without_result_dispatches_after_reconnect() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let command_id = CommandId::from("acknowledged-command-1");

    record_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");
    update_command_state(&state, &command_id, CommandState::Acknowledged)
        .await
        .expect("command acknowledges");

    assert!(should_open_control_channel(&state, &node_id)
        .await
        .expect("channel request evaluates"));

    let (_context, mut rx) = activate_test_connection(&state, node_id.clone()).await;
    dispatch_pending_commands(&state, &node_id)
        .await
        .expect("acknowledged command redispatches");
    let frame = rx.recv().await.expect("dispatch frame is sent");

    assert!(matches!(
        frame,
        ControlFrame::CommandDispatch { command, .. }
            if command.command_id == command_id
    ));
}

#[tokio::test]
async fn workspace_file_route_dispatches_node_read_and_decodes_payload() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    sqlx::query("delete from commands")
        .execute(&state.pool)
        .await
        .expect("setup commands clear");
    let (context, mut rx) = activate_test_connection(&state, node_id.clone()).await;
    let state_for_route = state.clone();
    let placement_id = placement.project_placement_id.clone();
    let route_task = tokio::spawn(async move {
        workspace_file_with_correlation(
            &state_for_route,
            placement_id,
            "README.md".to_owned(),
            CorrelationId::from("correlation-workspace-file"),
        )
        .await
    });

    let dispatched = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("workspace command dispatch is sent")
        .expect("channel stays open");
    let ControlFrame::CommandDispatch { command, .. } = dispatched else {
        panic!("expected command dispatch");
    };
    let response_payload = JsonValue(json!({
        "placement_id": placement.project_placement_id.as_str(),
        "path": "README.md",
        "metadata": {
            "name": "README.md",
            "path": "README.md",
            "kind": "file",
            "status": "readable",
            "byte_len": 5,
            "modified_at": null,
            "children": []
        },
        "content": "hello",
        "truncated": false,
        "generated_at": "2026-06-17T00:00:00Z"
    }));
    handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandResult {
            frame_id: "workspace-result-frame".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Completed,
            payload: response_payload,
        },
    )
    .await
    .expect("node command result accepts");
    let response = route_task
        .await
        .expect("route task joins")
        .expect("workspace file route succeeds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command.kind, CommandKind::ReadWorkspaceFile);
    assert!(matches!(
        command.payload,
        CommandPayload::ReadWorkspaceFile { ref path, .. } if path == "README.md"
    ));
    assert_eq!(response.content.as_deref(), Some("hello"));
}

#[tokio::test]
async fn workspace_command_route_persists_result_payload_for_history() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    sqlx::query("delete from commands")
        .execute(&state.pool)
        .await
        .expect("setup commands clear");
    let (context, mut rx) = activate_test_connection(&state, node_id.clone()).await;
    let state_for_route = state.clone();
    let placement_id = placement.project_placement_id.clone();
    let route_task = tokio::spawn(async move {
        workspace_command_run_with_correlation(
            &state_for_route,
            placement_id,
            WorkspaceCommandRunRequest {
                command: "rustc".to_owned(),
                args: vec!["--version".to_owned()],
                intent: uprava_protocol::WorkspaceCommandIntent::Command,
                label: None,
                timeout_seconds: Some(30),
            },
            CorrelationId::from("correlation-workspace-command"),
        )
        .await
    });

    let dispatched = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("workspace command dispatch is sent")
        .expect("channel stays open");
    let ControlFrame::CommandDispatch { command, .. } = dispatched else {
        panic!("expected command dispatch");
    };
    let response_payload = JsonValue(json!({
        "placement_id": placement.project_placement_id.as_str(),
        "terminal_command_id": "terminal-command-test",
        "command": "rustc",
        "args": ["--version"],
        "intent": "command",
        "label": null,
        "exit_code": 0,
        "success": true,
        "stdout": "rustc 1.0.0\n",
        "stderr": "",
        "stdout_truncated": false,
        "stderr_truncated": false,
        "duration_ms": 10,
        "started_at": "2026-06-17T00:00:00Z",
        "completed_at": "2026-06-17T00:00:01Z"
    }));
    handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandResult {
            frame_id: "workspace-command-result-frame".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Completed,
            payload: response_payload,
        },
    )
    .await
    .expect("node command result accepts");
    let response = route_task
        .await
        .expect("route task joins")
        .expect("workspace command route succeeds");
    let history =
        workspace_command_history(&state, placement.project_placement_id.clone(), Some(10))
            .await
            .expect("workspace command history loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command.kind, CommandKind::RunWorkspaceCommand);
    assert_eq!(response.stdout, "rustc 1.0.0\n");
    assert_eq!(history.commands.len(), 1);
    assert_eq!(history.commands[0].command_id, command.command_id);
    assert!(history.commands[0].result_payload.is_some());
}

#[tokio::test]
async fn workspace_command_async_resource_reports_progress_and_terminal_result() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    sqlx::query("delete from commands")
        .execute(&state.pool)
        .await
        .expect("setup commands clear");
    let (context, mut rx) = activate_test_connection(&state, node_id.clone()).await;
    let app = build_router(state.clone());

    let accepted_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async",
                    placement.project_placement_id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&WorkspaceCommandRunRequest {
                        command: "rustc".to_owned(),
                        args: vec!["--version".to_owned()],
                        intent: uprava_protocol::WorkspaceCommandIntent::Command,
                        label: None,
                        timeout_seconds: Some(30),
                    })
                    .expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(accepted_response.status(), StatusCode::ACCEPTED);
    let accepted_body = to_bytes(accepted_response.into_body(), 64 * 1024)
        .await
        .expect("accepted body loads");
    let accepted = serde_json::from_slice::<CommandAcceptedResponse>(&accepted_body)
        .expect("accepted decodes");

    let dispatched = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("workspace command dispatch is sent")
        .expect("channel stays open");
    let ControlFrame::CommandDispatch { command, .. } = dispatched else {
        panic!("expected command dispatch");
    };
    let progress_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async/{}",
                    placement.project_placement_id, accepted.command_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(progress_response.status(), StatusCode::ACCEPTED);

    let response_payload = JsonValue(json!({
        "placement_id": placement.project_placement_id.as_str(),
        "terminal_command_id": "terminal-command-async-test",
        "command": "rustc",
        "args": ["--version"],
        "intent": "command",
        "label": null,
        "exit_code": 0,
        "success": true,
        "stdout": "rustc 1.0.0\n",
        "stderr": "",
        "stdout_truncated": false,
        "stderr_truncated": false,
        "duration_ms": 10,
        "started_at": "2026-06-17T00:00:00Z",
        "completed_at": "2026-06-17T00:00:01Z"
    }));
    handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandResult {
            frame_id: "workspace-command-async-result-frame".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Completed,
            payload: response_payload,
        },
    )
    .await
    .expect("node command result accepts");

    let terminal_response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async/{}",
                    placement.project_placement_id, accepted.command_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(terminal_response.status(), StatusCode::OK);
    let terminal_body = to_bytes(terminal_response.into_body(), 64 * 1024)
        .await
        .expect("terminal body loads");
    let item = serde_json::from_slice::<WorkspaceCommandHistoryItem>(&terminal_body)
        .expect("resource decodes");
    let result_payload = item.result_payload.expect("result payload persists");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(accepted.command_id, command.command_id);
    assert_eq!(item.state, CommandState::Completed);
    assert_eq!(result_payload.0["stdout"], "rustc 1.0.0\n");
}

#[tokio::test]
async fn workspace_command_async_resource_cancels_and_expires_nonterminal_commands() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    sqlx::query("delete from commands")
        .execute(&state.pool)
        .await
        .expect("setup commands clear");
    let (_context, mut rx) = activate_test_connection(&state, node_id).await;
    let app = build_router(state.clone());

    let cancel_accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async",
                    placement.project_placement_id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&WorkspaceCommandRunRequest {
                        command: "sleep".to_owned(),
                        args: vec!["10".to_owned()],
                        intent: uprava_protocol::WorkspaceCommandIntent::Command,
                        label: None,
                        timeout_seconds: Some(30),
                    })
                    .expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let cancel_body = to_bytes(cancel_accepted.into_body(), 64 * 1024)
        .await
        .expect("cancel accepted body loads");
    let cancel_command =
        serde_json::from_slice::<CommandAcceptedResponse>(&cancel_body).expect("accepted decodes");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("cancel command dispatch is sent")
        .expect("channel stays open");

    let cancel_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async/{}",
                    placement.project_placement_id, cancel_command.command_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let cancel_body = to_bytes(cancel_response.into_body(), 64 * 1024)
        .await
        .expect("cancel body loads");
    let cancelled = serde_json::from_slice::<WorkspaceCommandHistoryItem>(&cancel_body)
        .expect("cancel decodes");

    let expire_accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async",
                    placement.project_placement_id
                ))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&WorkspaceCommandRunRequest {
                        command: "sleep".to_owned(),
                        args: vec!["10".to_owned()],
                        intent: uprava_protocol::WorkspaceCommandIntent::Command,
                        label: None,
                        timeout_seconds: Some(1),
                    })
                    .expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let expire_body = to_bytes(expire_accepted.into_body(), 64 * 1024)
        .await
        .expect("expire accepted body loads");
    let expire_command =
        serde_json::from_slice::<CommandAcceptedResponse>(&expire_body).expect("accepted decodes");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("expire command dispatch is sent")
        .expect("channel stays open");
    sqlx::query("update commands set created_at = ?1 where command_id = ?2")
        .bind(Utc::now() - chrono::Duration::seconds(20))
        .bind(expire_command.command_id.as_str())
        .execute(&state.pool)
        .await
        .expect("command age rewinds");

    let expired_response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/placements/{}/workspace/commands/async/{}",
                    placement.project_placement_id, expire_command.command_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(expired_response.status(), StatusCode::OK);
    let expired_body = to_bytes(expired_response.into_body(), 64 * 1024)
        .await
        .expect("expired body loads");
    let expired = serde_json::from_slice::<WorkspaceCommandHistoryItem>(&expired_body)
        .expect("expire decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(cancelled.state, CommandState::Expired);
    assert_eq!(
        cancelled
            .result_payload
            .expect("cancel payload")
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str),
        Some("workspace.command_cancelled")
    );
    assert_eq!(expired.state, CommandState::Expired);
    assert_eq!(
        expired
            .result_payload
            .expect("expiry payload")
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str),
        Some("workspace.command_expired")
    );
}

#[tokio::test]
async fn workspace_command_timeout_cleans_waiter_registry() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        &state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    let placement = load_placement(&state, &placement.project_placement_id)
        .await
        .expect("placement reloads");
    let (_context, _rx) = activate_test_connection(&state, node_id).await;

    let result = dispatch_workspace_command::<WorkspaceCommandRunResponse>(
        &state,
        &placement,
        CommandKind::RunWorkspaceCommand,
        serde_json::to_value(WorkspaceCommandRunRequest {
            command: "sleep".to_owned(),
            args: vec!["10".to_owned()],
            intent: uprava_protocol::WorkspaceCommandIntent::Command,
            label: None,
            timeout_seconds: Some(1),
        })
        .expect("request serializes"),
        vec![UpravaRef::Workspace {
            placement_id: placement.project_placement_id.clone(),
        }],
        CorrelationId::from("correlation-timeout-cleanup"),
        std::time::Duration::from_millis(1),
    )
    .await;
    let waiters_empty = lock_command_waiters(&state)
        .expect("waiters lock")
        .is_empty();
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(
        matches!(
            result,
            Err(AppError::BadRequest {
                code: "workspace.command_timeout",
                ..
            }) | Err(AppError::BadRequest {
                code: "workspace.command_result_unavailable",
                ..
            })
        ),
        "unexpected workspace command result: {result:?}"
    );
    assert!(waiters_empty);
}

#[tokio::test]
async fn compatible_control_hello_acknowledges_and_dispatches_pending_command() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let command_id = CommandId::from("hello-dispatch-command-1");
    record_and_dispatch_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");
    let (tx, mut rx) = mpsc::channel(CONTROL_QUEUE_CAPACITY);
    let context = state.control_connections.context(node_id.clone(), tx);
    assert!(!state.control_connections.contains(&node_id).await);

    handle_node_control_frame(
        &state,
        &context,
        ControlFrame::Hello {
            frame_id: "hello-frame-1".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            node_id: node_id.clone(),
            daemon_version: "0.1.0".to_owned(),
            active_runtime_ids: vec![],
        },
    )
    .await
    .expect("compatible hello accepts");
    assert!(state.control_connections.contains(&node_id).await);
    let hello_ack = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("hello ack is sent")
        .expect("channel stays open");
    let dispatch = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("command dispatch is sent")
        .expect("channel stays open");

    assert!(matches!(hello_ack, ControlFrame::HelloAck { .. }));
    assert!(matches!(
        dispatch,
        ControlFrame::CommandDispatch { command, .. }
            if command.command_id == command_id
    ));
}

#[tokio::test]
async fn overlapping_control_connections_keep_newest_generation_active() {
    let state = test_state().await;
    let node_id = NodeId::from("overlapping-control-node");
    let (old_sender, _old_receiver) = mpsc::channel(CONTROL_QUEUE_CAPACITY);
    let old_context = state
        .control_connections
        .context(node_id.clone(), old_sender);
    state.control_connections.activate(&old_context).await;
    let (new_sender, _new_receiver) = mpsc::channel(CONTROL_QUEUE_CAPACITY);
    let new_context = state
        .control_connections
        .context(node_id.clone(), new_sender);
    state.control_connections.activate(&new_context).await;

    let old_removed = state
        .control_connections
        .remove_if_active(&old_context)
        .await;
    let stale_result = handle_node_control_frame(
        &state,
        &old_context,
        ControlFrame::CommandAck {
            frame_id: "stale-ack".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: CommandId::from("stale-command"),
            status: CommandState::Acknowledged,
        },
    )
    .await;
    let stale_hello = handle_node_control_frame(
        &state,
        &old_context,
        ControlFrame::Hello {
            frame_id: "stale-hello".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            node_id: old_context.node_id.clone(),
            daemon_version: "test".to_owned(),
            active_runtime_ids: vec![],
        },
    )
    .await;

    assert!(!old_removed);
    assert!(state.control_connections.is_active(&new_context).await);
    assert!(matches!(
        stale_result,
        Err(AppError::Auth {
            code: "control.stale_generation",
            ..
        })
    ));
    assert!(matches!(
        stale_hello,
        Err(AppError::Auth {
            code: "control.stale_generation",
            ..
        })
    ));
}

#[tokio::test]
async fn saturated_control_queue_rejects_frame_and_increments_metric() {
    let state = test_state().await;
    let node_id = NodeId::from("saturated-control-node");
    let (_context, _receiver) = activate_test_connection(&state, node_id.clone()).await;
    for index in 0..CONTROL_QUEUE_CAPACITY {
        assert!(
            send_control_frame(
                &state,
                &node_id,
                ControlFrame::Ping {
                    frame_id: format!("fill-{index}"),
                    protocol_version: API_VERSION.to_owned(),
                    sent_at: Utc::now(),
                },
            )
            .await
        );
    }

    let overflow = try_send_control_frame(
        &state,
        &node_id,
        ControlFrame::Ping {
            frame_id: "overflow".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
        },
    )
    .await;

    assert_eq!(overflow, Err(ControlSendError::Saturated));
    assert_eq!(
        state
            .core_metrics
            .control_queue_rejections
            .load(Ordering::Relaxed),
        1
    );
}

#[tokio::test]
async fn cross_node_command_ack_is_rejected_without_state_change() {
    let state = test_state().await;
    let owner = enroll_test_node(&state)
        .await
        .node_id
        .expect("owner node id returned");
    let attacker = enroll_test_node(&state)
        .await
        .node_id
        .expect("attacker node id returned");
    let command_id = CommandId::from("cross-node-command");
    record_command(&state, command_fixture(command_id.clone(), owner))
        .await
        .expect("owned command records");
    update_command_state(&state, &command_id, CommandState::Dispatched)
        .await
        .expect("owned command dispatches");
    let (context, _receiver) = activate_test_connection(&state, attacker).await;

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandAck {
            frame_id: "cross-node-ack".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command_id.clone(),
            status: CommandState::Acknowledged,
        },
    )
    .await;
    let stored_state: String =
        sqlx::query_scalar("select state from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command state loads");

    assert!(matches!(
        result,
        Err(AppError::Auth {
            code: "control.command_owner_mismatch",
            ..
        })
    ));
    assert_eq!(stored_state, "dispatched");
}

#[tokio::test]
async fn conflicting_duplicate_command_result_is_rejected() {
    let state = test_state().await;
    let node_id = enroll_test_node(&state)
        .await
        .node_id
        .expect("node id returned");
    let command_id = CommandId::from("conflicting-result-command");
    record_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");
    update_command_state(&state, &command_id, CommandState::Dispatched)
        .await
        .expect("command dispatches");
    let (context, _receiver) = activate_test_connection(&state, node_id).await;
    let result_frame = |payload| ControlFrame::CommandResult {
        frame_id: Uuid::new_v4().to_string(),
        protocol_version: API_VERSION.to_owned(),
        sent_at: Utc::now(),
        command_id: command_id.clone(),
        status: CommandState::Completed,
        payload: JsonValue(payload),
    };
    handle_node_control_frame(&state, &context, result_frame(json!({"value": 1})))
        .await
        .expect("first terminal result accepts");

    let duplicate =
        handle_node_control_frame(&state, &context, result_frame(json!({"value": 2}))).await;

    assert!(matches!(
        duplicate,
        Err(AppError::BadRequest {
            code: "control.command_result_conflict",
            ..
        })
    ));
}

#[tokio::test]
async fn workspace_result_with_wrong_placement_echo_is_rejected() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let command_id = CommandId::from("wrong-placement-result");
    let mut command = command_fixture(command_id.clone(), node_id.clone());
    command.kind = CommandKind::ReadWorkspaceFile;
    command.target = CommandTarget::Placement {
        node_id: detail.placement.node_id.clone(),
        project_placement_id: detail.placement.project_placement_id.clone(),
    };
    command.payload = CommandPayload::ReadWorkspaceFile {
        workspace_path: detail.placement.workspace_path.clone(),
        path: "README.md".to_owned(),
    };
    record_command(&state, command)
        .await
        .expect("workspace command records");
    update_command_state(&state, &command_id, CommandState::Dispatched)
        .await
        .expect("workspace command dispatches");
    let (context, _receiver) = activate_test_connection(&state, node_id).await;

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandResult {
            frame_id: "wrong-placement-result-frame".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id,
            status: CommandState::Completed,
            payload: JsonValue(json!({
                "placement_id": "another-placement",
                "path": "README.md",
                "metadata": {
                    "name": "README.md",
                    "path": "README.md",
                    "kind": "file",
                    "status": "readable",
                    "byte_len": 1,
                    "modified_at": null,
                    "children": []
                },
                "content": "x",
                "truncated": false,
                "generated_at": "2026-07-10T00:00:00Z"
            })),
        },
    )
    .await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "control.command_result_target_mismatch",
            ..
        })
    ));
}

#[tokio::test]
async fn oversized_event_batch_is_rejected_before_event_validation() {
    let state = test_state().await;
    let node_id = NodeId::from("oversized-batch-node");
    let (context, _receiver) = activate_test_connection(&state, node_id.clone()).await;
    let event = EventEnvelope {
        event_id: EventId::from("oversized-event"),
        command_id: None,
        correlation_id: None,
        actor_ref: ActorRef::Node {
            node_id: node_id.clone(),
        },
        scope_ref: ScopeRef::Node { node_id },
        node_id: Some(context.node_id.clone()),
        runtime_session_id: None,
        session_thread_id: None,
        turn_id: None,
        seq: 1,
        session_projection_seq: None,
        kind: EventKind::ProviderActivity,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: EventPayload::from_json(EventKind::ProviderActivity, json!({})),
    };

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::EventBatch {
            frame_id: "oversized-event-batch".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            events: vec![event; MAX_EVENT_BATCH_ITEMS + 1],
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "control.event_batch_too_large",
            ..
        })
    ));
}

#[tokio::test]
async fn deeply_nested_control_payload_is_rejected_before_command_lookup() {
    let state = test_state().await;
    let node_id = NodeId::from("deep-control-node");
    let (context, _receiver) = activate_test_connection(&state, node_id).await;
    let mut nested = json!(null);
    for _ in 0..=MAX_CONTROL_JSON_DEPTH {
        nested = json!({ "nested": nested });
    }

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::CommandResult {
            frame_id: "deep-control-frame".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: CommandId::from("not-looked-up"),
            status: CommandState::Completed,
            payload: JsonValue(nested),
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "control.frame_too_deep",
            ..
        })
    ));
}

#[tokio::test]
async fn cross_node_runtime_event_is_rejected_before_persistence() {
    let state = test_state().await;
    let (owner, detail, workspace_path) = create_test_session(&state).await;
    let attacker = enroll_test_node(&state)
        .await
        .node_id
        .expect("attacker node id returned");
    let (context, _receiver) = activate_test_connection(&state, attacker.clone()).await;
    let mut forged = node_event_fixture(
        &detail,
        attacker,
        "cross-node-runtime-event",
        1,
        EventKind::ProviderActivity,
        json!({}),
    );
    forged.actor_ref = ActorRef::Provider {
        provider: "codex".to_owned(),
    };

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::EventBatch {
            frame_id: "cross-node-event-batch".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            events: vec![forged.clone()],
        },
    )
    .await;
    let event_count: i64 = sqlx::query_scalar("select count(*) from events where event_id = ?1")
        .bind(forged.event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("forged event count loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_ne!(owner, context.node_id);
    assert!(matches!(
        result,
        Err(AppError::Auth {
            code: "control.event_runtime_mismatch",
            ..
        })
    ));
    assert_eq!(event_count, 0);
}

#[tokio::test]
async fn cross_node_terminal_output_is_rejected_before_broadcast() {
    let state = test_state().await;
    let (_owner, detail, workspace_path) = create_test_session(&state).await;
    let attacker = enroll_test_node(&state)
        .await
        .node_id
        .expect("attacker node id returned");
    let terminal_id = TerminalId::from("cross-node-terminal");
    state.workspace_terminals.write().await.insert(
        terminal_id.to_string(),
        WorkspaceTerminalSummary {
            placement_id: detail.placement.project_placement_id.clone(),
            terminal_id: terminal_id.clone(),
            title: "test".to_owned(),
            cwd: "/tmp".to_owned(),
            shell: "sh".to_owned(),
            cols: 80,
            rows: 24,
            state: WorkspaceTerminalState::Running,
            exit_code: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    let (context, _receiver) = activate_test_connection(&state, attacker).await;
    let mut terminal_rx = state.terminal_hub.subscribe(&terminal_id).await;

    let result = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::WorkspaceTerminalOutput {
            frame_id: "cross-node-terminal-output".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            terminal_id,
            seq: 1,
            data: "forged".to_owned(),
        },
    )
    .await;
    let broadcast = terminal_rx.try_recv();
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::Auth {
            code: "control.terminal_owner_mismatch",
            ..
        })
    ));
    assert!(matches!(
        broadcast,
        Err(broadcast::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn incompatible_control_hello_sends_error_and_leaves_command_pending() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let command_id = CommandId::from("bad-hello-command-1");
    record_and_dispatch_command(&state, command_fixture(command_id.clone(), node_id.clone()))
        .await
        .expect("command records");
    let (tx, mut rx) = mpsc::channel(CONTROL_QUEUE_CAPACITY);
    let context = state.control_connections.context(node_id.clone(), tx);

    let error = handle_node_control_frame(
        &state,
        &context,
        ControlFrame::Hello {
            frame_id: "bad-hello-frame-1".to_owned(),
            protocol_version: "v0".to_owned(),
            sent_at: Utc::now(),
            node_id: node_id.clone(),
            daemon_version: "0.1.0".to_owned(),
            active_runtime_ids: vec![],
        },
    )
    .await
    .expect_err("incompatible hello rejects");
    let frame = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("control error is sent")
        .expect("channel stays open");
    let command_state: String =
        sqlx::query_scalar("select state from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command state loads");

    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "control.protocol_incompatible",
            ..
        }
    ));
    assert!(matches!(
        frame,
        ControlFrame::ControlError { error, .. }
            if error.error_code == "control.protocol_incompatible" && !error.retryable
    ));
    assert_eq!(command_state, "pending_dispatch");
}

#[tokio::test]
async fn duplicate_node_event_does_not_duplicate_assistant_message() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "provider-completed-duplicate",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "deduped" }),
    );
    let event_id = event.event_id.clone();

    accept_node_event(&state, event.clone())
        .await
        .expect("first event accepts");
    sqlx::query(
        "update events set projection_state = 'pending', projected_at = null where event_id = ?1",
    )
    .bind(event_id.as_str())
    .execute(&state.pool)
    .await
    .expect("projection is reset for replay");
    let mut conflicting_replay = event;
    conflicting_replay.payload = EventPayload::from_json(
        EventKind::ProviderMessageCompleted,
        json!({ "content": "must not replace original" }),
    );
    accept_node_event(&state, conflicting_replay)
        .await
        .expect("pending duplicate replays persisted event");
    let projection_state: (String, i64) = sqlx::query_as(
        "select projection_state, projection_attempts from events where event_id = ?1",
    )
    .bind(event_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("projection state loads");
    let messages = load_messages(&state, &detail.session.session_thread_id)
        .await
        .expect("messages load");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    let duplicate_count = messages
        .iter()
        .filter(|message| message.role == MessageRole::Assistant && message.content == "deduped")
        .count();
    assert_eq!(duplicate_count, 1);
    assert_eq!(projection_state, ("projected".to_owned(), 1));
}

#[tokio::test]
async fn projection_completion_couples_state_and_publication_enqueue() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "projection-boundary-provider-completed",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "boundary" }),
    );
    let scope_key = scope_key(&event.scope_ref);
    insert_event_record(&state, &scope_key, &event)
        .await
        .expect("event record inserts");

    let pending: (String, i64) = sqlx::query_as(
            "select projection_state, (select count(*) from event_publication_outbox where event_id = events.event_id) from events where event_id = ?1",
        )
        .bind(event.event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("pending boundary loads");
    // Node ingest must not expose an event for publication while its
    // projection is still pending.  The completion boundary below is the
    // first operation allowed to enqueue it.
    assert_eq!(pending, ("pending".to_owned(), 0));

    complete_event_projection(&state, &event)
        .await
        .expect("projection completion commits");
    let projected: (String, i64) = sqlx::query_as(
            "select projection_state, (select count(*) from event_publication_outbox where event_id = events.event_id) from events where event_id = ?1",
        )
        .bind(event.event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("projected boundary loads");
    assert_eq!(projected, ("projected".to_owned(), 1));

    // Repeating the completion is idempotent and does not duplicate the
    // publication row.
    complete_event_projection(&state, &event)
        .await
        .expect("repeated projection completion commits");
    let outbox_count: i64 =
        sqlx::query_scalar("select count(*) from event_publication_outbox where event_id = ?1")
            .bind(event.event_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("outbox count loads");
    assert_eq!(outbox_count, 1);
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn accepted_node_event_is_published_to_session_stream_bus() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "published-provider-completed-1",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "published" }),
    );
    let expected_event_id = event.event_id.clone();
    let mut event_rx = state.event_tx.subscribe();
    drain_event_publication_outbox(&state)
        .await
        .expect("pre-existing outbox rows drain");
    while event_rx.try_recv().is_ok() {}

    accept_node_event(&state, event)
        .await
        .expect("node event accepts");
    let published = tokio::time::timeout(std::time::Duration::from_secs(1), event_rx.recv())
        .await
        .expect("event is published")
        .expect("event bus stays open");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(published.event_id, expected_event_id);
    assert!(event_matches_session_after_seq(
        &published,
        &detail.session.session_thread_id,
        0
    ));
}

#[tokio::test]
async fn event_publication_outbox_retries_without_duplicate_delivery() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let mut event_rx = state.event_tx.subscribe();
    drain_event_publication_outbox(&state)
        .await
        .expect("pre-existing outbox rows drain");
    while event_rx.try_recv().is_ok() {}
    drop(event_rx);
    let event = node_event_fixture(
        &detail,
        node_id,
        "outbox-retry-provider-completed",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "outbox" }),
    );
    let event_id = event.event_id.clone();

    // With no subscribers the durable row remains pending and records the
    // failed publication attempt.
    accept_node_event(&state, event.clone())
        .await
        .expect("event accepts without subscribers");
    let pending: (i64, Option<String>) = sqlx::query_as(
        "select attempts, published_at from event_publication_outbox where event_id = ?1",
    )
    .bind(event_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("pending outbox row loads");
    assert_eq!(pending.0, 1);
    assert!(pending.1.is_none());

    let mut event_rx = state.event_tx.subscribe();
    drain_event_publication_outbox(&state)
        .await
        .expect("outbox drains after subscriber joins");
    let published = tokio::time::timeout(std::time::Duration::from_secs(1), event_rx.recv())
        .await
        .expect("retry publishes")
        .expect("event bus stays open");
    assert_eq!(published.event_id, event_id);

    let published_at: Option<String> =
        sqlx::query_scalar("select published_at from event_publication_outbox where event_id = ?1")
            .bind(event_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("published outbox row loads");
    assert!(published_at.is_some());

    // A projected duplicate is idempotent: its existing published row is
    // not re-enqueued and no second broadcast is emitted.
    accept_node_event(&state, event)
        .await
        .expect("duplicate event is idempotent");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), event_rx.recv())
            .await
            .is_err()
    );
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

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

#[tokio::test]
async fn send_turn_persists_durable_turn_and_user_message() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;

    let response = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "persist this turn".to_owned(),
        }),
    )
    .await
    .expect("turn sends")
    .0;
    let (turn_state, content, user_message_count): (String, String, i64) = sqlx::query_as(
        r#"
            select t.state, t.content, count(m.message_id)
            from turns t
            left join messages m on m.turn_id = t.turn_id and m.role = 'user'
            where t.command_id = ?1
            group by t.turn_id, t.state, t.content
            "#,
    )
    .bind(response.command_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("turn row loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(turn_state, "created");
    assert_eq!(content, "persist this turn");
    assert_eq!(user_message_count, 1);
}

#[tokio::test]
async fn turn_events_update_durable_turn_state_and_blocked_approval() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    let response = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "needs approval".to_owned(),
        }),
    )
    .await
    .expect("turn sends")
    .0;
    let turn_id = turn_id_for_command(&state, &response.command_id).await;
    let mut started = node_event_fixture(
        &detail,
        node_id.clone(),
        "turn-state-started-1",
        1,
        EventKind::TurnStarted,
        json!({}),
    );
    started.turn_id = Some(turn_id.clone());
    accept_node_event(&state, started)
        .await
        .expect("started event accepts");
    let mut approval = node_event_fixture(
        &detail,
        node_id,
        "turn-state-approval-2",
        2,
        EventKind::ApprovalRequested,
        json!({
            "approval_id": "approval-turn-state",
            "prompt": "Allow state change"
        }),
    );
    approval.turn_id = Some(turn_id.clone());
    accept_node_event(&state, approval)
        .await
        .expect("approval event accepts");
    let (turn_state, blocked_approval_id): (String, Option<String>) =
        sqlx::query_as("select state, blocked_approval_id from turns where turn_id = ?1")
            .bind(turn_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("turn row loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(turn_state, "blocked_on_approval");
    assert_eq!(blocked_approval_id.as_deref(), Some("approval-turn-state"));
}

#[tokio::test]
async fn send_turn_rejects_offline_node_without_recording_command() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    mark_node_offline(&state, &node_id).await;
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "hello".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "node.offline",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn send_turn_rejects_detached_session_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let _ = detach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await
    .expect("session detaches");
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "hello".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "session.detached",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn send_turn_rejects_runtime_state_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Stopped).await;
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "hello".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "runtime.command_not_allowed",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn interrupt_runtime_rejects_ready_runtime_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    let command_count_before = command_count(&state).await;

    let result = interrupt_runtime(
        State(state.clone()),
        Path(detail.session.runtime.runtime_session_id.to_string()),
    )
    .await;
    let command_count_after = command_count(&state).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "runtime.command_not_allowed",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
}

#[tokio::test]
async fn resume_runtime_rejects_ready_runtime_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    let command_count_before = command_count(&state).await;

    let result = resume_runtime(
        State(state.clone()),
        Path(detail.session.runtime.runtime_session_id.to_string()),
    )
    .await;
    let command_count_after = command_count(&state).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "runtime.command_not_allowed",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
}

#[tokio::test]
async fn resume_runtime_accepts_stopped_runtime_with_blank_resume_ref() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    sqlx::query(
        r#"
            update runtime_sessions
            set state = ?1, provider_resume_ref_json = ''
            where runtime_session_id = ?2
            "#,
    )
    .bind(format_runtime_state(RuntimeSessionState::Stopped))
    .bind(detail.session.runtime.runtime_session_id.as_str())
    .execute(&state.pool)
    .await
    .expect("runtime stores blank resume ref");
    sqlx::query("update session_threads set state = ?1 where session_thread_id = ?2")
        .bind(format_session_state(SessionThreadState::Stopped))
        .bind(detail.session.session_thread_id.as_str())
        .execute(&state.pool)
        .await
        .expect("session stops");

    let response = resume_runtime(
        State(state.clone()),
        Path(detail.session.runtime.runtime_session_id.to_string()),
    )
    .await
    .expect("stopped runtime resumes without provider ref")
    .0;
    let command_json: String =
        sqlx::query_scalar("select command_json from commands where command_id = ?1")
            .bind(response.command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command json loads");
    let command: CommandEnvelope = serde_json::from_str(&command_json).expect("command decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command.kind, CommandKind::ResumeRuntime);
    assert!(matches!(
        command.payload,
        CommandPayload::ResumeRuntime {
            ref workspace_path,
            provider_resume_ref: None,
            ..
        } if workspace_path == &detail.placement.workspace_path
    ));
}

#[tokio::test]
async fn send_turn_rejects_placement_hard_block_without_recording_command() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    let hard_block = ResourceBadge {
        kind: "read_only_workspace".to_owned(),
        severity: WarningSeverity::HardBlock,
        label: "Read-only workspace".to_owned(),
    };
    sqlx::query(
        "update project_placements set resource_badges_json = ?1 where project_placement_id = ?2",
    )
    .bind(serde_json::to_string(&vec![hard_block]).expect("badge serializes"))
    .bind(detail.placement.project_placement_id.as_str())
    .execute(&state.pool)
    .await
    .expect("placement hard block stores");
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "blocked".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "placement.hard_blocked",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn send_turn_rejects_missing_provider_capability_without_recording_command() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    set_node_capabilities(&state, &node_id, vec![]).await;
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "hello".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "node.capability_missing",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn agent_projection_warns_for_offline_node_and_suppresses_node_commands() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-ready-before-offline",
            1,
            EventKind::RuntimeReady,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("ready event accepts");
    mark_node_offline(&state, &node_id).await;

    let projection = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(projection.active_warnings.iter().any(|warning| {
        warning.kind == "node_offline" && warning.severity == WarningSeverity::HardBlock
    }));
    assert!(!projection
        .available_commands
        .contains(&ActionCapability::SessionSendTurn));
    assert!(!projection
        .available_commands
        .contains(&ActionCapability::RuntimeStop));
}

#[tokio::test]
async fn agent_projection_warns_for_missing_provider_and_suppresses_provider_commands() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-ready-before-provider-missing",
            1,
            EventKind::RuntimeReady,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("ready event accepts");
    set_node_capabilities(&state, &node_id, vec![]).await;

    let projection = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(projection.active_warnings.iter().any(|warning| {
        warning.kind == "provider_unavailable" && warning.severity == WarningSeverity::HardBlock
    }));
    assert!(!projection
        .available_commands
        .contains(&ActionCapability::SessionSendTurn));
    assert!(projection
        .available_commands
        .contains(&ActionCapability::RuntimeStop));
}

#[tokio::test]
async fn agent_projection_switches_between_detach_and_attach_commands() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let before = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds before detach");

    let _ = detach_session(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
    )
    .await
    .expect("session detaches");
    let after = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds after detach");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(before
        .available_commands
        .contains(&ActionCapability::SessionDetach));
    assert!(!before
        .available_commands
        .contains(&ActionCapability::SessionAttach));
    assert!(after
        .available_commands
        .contains(&ActionCapability::SessionAttach));
    assert!(!after
        .available_commands
        .contains(&ActionCapability::SessionDetach));
    assert!(!after
        .available_commands
        .contains(&ActionCapability::SessionSendTurn));
}

#[tokio::test]
async fn agent_projection_tracks_pending_approval_and_current_turn() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-running-1",
            1,
            EventKind::RuntimeRunning,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("running event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-turn-started-1",
            2,
            EventKind::TurnStarted,
            json!({}),
        ),
    )
    .await
    .expect("turn event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-approval-1",
            3,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-1",
                "prompt": "Allow projection test"
            }),
        ),
    )
    .await
    .expect("approval event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "projection-blocked-1",
            4,
            EventKind::RuntimeBlocked,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("blocked event accepts");

    let projection = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(projection.current_turn, Some(TurnId::from("turn-1")));
    assert_eq!(
        projection.pending_approvals,
        vec![ApprovalId::from("approval-1")]
    );
    assert!(projection
        .available_commands
        .contains(&ActionCapability::ApprovalResolve));
}

#[tokio::test]
async fn agent_projection_requires_blocked_runtime_for_approval_resolution() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-approval-ready-1",
            1,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-ready",
                "prompt": "Allow projection test"
            }),
        ),
    )
    .await
    .expect("approval event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "projection-runtime-ready-after-approval",
            2,
            EventKind::RuntimeReady,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("ready event accepts");

    let projection = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        projection.pending_approvals,
        vec![ApprovalId::from("approval-ready")]
    );
    assert!(!projection
        .available_commands
        .contains(&ActionCapability::ApprovalResolve));
}

#[tokio::test]
async fn agent_projection_clears_pending_approval_after_resolution() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-approval-requested-2",
            1,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-2",
                "prompt": "Allow projection test"
            }),
        ),
    )
    .await
    .expect("approval event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "projection-approval-resolved-2",
            2,
            EventKind::ApprovalResolved,
            json!({
                "approval_id": "approval-2",
                "approved": true,
                "message": "approved"
            }),
        ),
    )
    .await
    .expect("approval resolution event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "projection-ready-2",
            3,
            EventKind::RuntimeReady,
            json!({ "provider": "codex" }),
        ),
    )
    .await
    .expect("ready event accepts");

    let projection = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds");
    let (approval_state, response_payload_json): (String, Option<String>) =
        sqlx::query_as("select state, response_payload_json from approvals where approval_id = ?1")
            .bind("approval-2")
            .fetch_one(&state.pool)
            .await
            .expect("approval row loads");
    let response_payload: serde_json::Value =
        serde_json::from_str(&response_payload_json.expect("response payload stores"))
            .expect("response payload decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(projection.pending_approvals.is_empty());
    assert_eq!(approval_state, "resolved");
    assert_eq!(
        response_payload
            .get("message")
            .and_then(serde_json::Value::as_str),
        Some("approved")
    );
    assert!(!projection
        .available_commands
        .contains(&ActionCapability::ApprovalResolve));
    assert!(projection
        .available_commands
        .contains(&ActionCapability::SessionSendTurn));
}

#[tokio::test]
async fn acknowledge_warning_persists_event_and_suppresses_projection_warning() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let warning = ResourceBadge {
        kind: "dirty_workspace".to_owned(),
        severity: WarningSeverity::Warning,
        label: "Dirty workspace".to_owned(),
    };
    sqlx::query(
        "update project_placements set resource_badges_json = ?1 where project_placement_id = ?2",
    )
    .bind(serde_json::to_string(&vec![warning]).expect("warning serializes"))
    .bind(detail.placement.project_placement_id.as_str())
    .execute(&state.pool)
    .await
    .expect("placement warning stores");
    let before = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds before ack");

    let response = acknowledge_warning(
        State(state.clone()),
        Path((
            detail.session.session_thread_id.to_string(),
            "dirty_workspace".to_owned(),
        )),
        Json(AcknowledgeWarningRequest {
            message: Some("reviewed".to_owned()),
        }),
    )
    .await
    .expect("warning acknowledges")
    .0;
    let row_count: i64 =
        sqlx::query_scalar("select count(*) from warning_acknowledgements where warning_kind = ?1")
            .bind("dirty_workspace")
            .fetch_one(&state.pool)
            .await
            .expect("warning acknowledgement count loads");
    let event_kind: String = sqlx::query_scalar("select kind from events where event_id = ?1")
        .bind(response.event_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("ack event loads");
    let after = build_agent_projection(&state, &detail.session.session_thread_id)
        .await
        .expect("projection builds after ack");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(before.active_warnings.len(), 1);
    assert!(before
        .available_commands
        .contains(&ActionCapability::WarningAcknowledge));
    assert_eq!(row_count, 1);
    assert_eq!(event_kind, "CoordinationWarningAcknowledged");
    assert!(after.active_warnings.is_empty());
    assert!(!after
        .available_commands
        .contains(&ActionCapability::WarningAcknowledge));
}

#[tokio::test]
async fn evidence_projection_uses_approval_ref_for_approval_event() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "artifact-approval-1",
            1,
            EventKind::ApprovalRequested,
            json!({
                "approval_id": "approval-artifact-1",
                "prompt": "Allow artifact test"
            }),
        ),
    )
    .await
    .expect("approval event accepts");

    let evidence_projection =
        build_session_evidence_projection(&state, &detail.session.session_thread_id)
            .await
            .expect("evidence projection builds");
    let rebuilt_projection =
        build_session_evidence_projection(&state, &detail.session.session_thread_id)
            .await
            .expect("evidence projection rebuilds");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        evidence_projection.root.evidence_id,
        rebuilt_projection.root.evidence_id
    );
    assert_eq!(
        evidence_projection
            .root
            .children
            .iter()
            .map(|node| node.evidence_id.clone())
            .collect::<Vec<_>>(),
        rebuilt_projection
            .root
            .children
            .iter()
            .map(|node| node.evidence_id.clone())
            .collect::<Vec<_>>()
    );
    assert!(evidence_projection
        .root
        .children
        .iter()
        .any(|node| matches!(
            &node.primary_ref,
            UpravaRef::Approval { approval_id } if approval_id.as_str() == "approval-artifact-1"
        )));
}

#[tokio::test]
async fn runtime_scoped_provider_event_updates_last_runtime_step() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let event = node_event_fixture(
        &detail,
        node_id,
        "runtime-step-provider-completed-1",
        1,
        EventKind::ProviderMessageCompleted,
        json!({ "content": "step" }),
    );
    let happened_at = event.happened_at;

    accept_node_event(&state, event)
        .await
        .expect("provider event accepts");
    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(
        detail.session.runtime.last_runtime_step_at,
        Some(happened_at)
    );
}

#[tokio::test]
async fn load_session_detail_expires_idle_runtime_with_durable_event() {
    let state = test_state_with_runtime_expiry(1).await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2)).await;

    let detail = load_session_detail(&state, &detail.session.session_thread_id)
        .await
        .expect("session reloads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Expired);
    assert_eq!(detail.session.state, SessionThreadState::Degraded);
    assert!(detail.events.iter().any(|event| {
        event.kind == EventKind::RuntimeExpired
            && event
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str)
                == Some("runtime.idle_expired")
    }));
}

#[tokio::test]
async fn send_turn_rejects_idle_expired_runtime_without_recording_command() {
    let state = test_state_with_runtime_expiry(1).await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2)).await;
    let command_count_before = command_count(&state).await;
    let message_count_before = session_message_count(&state, &detail).await;

    let result = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "after expiry".to_owned(),
        }),
    )
    .await;
    let command_count_after = command_count(&state).await;
    let message_count_after = session_message_count(&state, &detail).await;
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "runtime.command_not_allowed",
            ..
        })
    ));
    assert_eq!(command_count_after, command_count_before);
    assert_eq!(message_count_after, message_count_before);
}

#[tokio::test]
async fn resume_runtime_accepts_idle_expired_runtime() {
    let state = test_state_with_runtime_expiry(1).await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;
    set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2)).await;
    sqlx::query(
        "update runtime_sessions set provider_resume_ref_json = ?1 where runtime_session_id = ?2",
    )
    .bind(
        json!({
            "provider_session_id": "codex-session-1",
            "resume_cursor": "cursor-1",
        })
        .to_string(),
    )
    .bind(detail.session.runtime.runtime_session_id.as_str())
    .execute(&state.pool)
    .await
    .expect("provider resume ref stores");

    let response = resume_runtime(
        State(state.clone()),
        Path(detail.session.runtime.runtime_session_id.to_string()),
    )
    .await
    .expect("expired runtime resumes")
    .0;
    let command_kind: String =
        sqlx::query_scalar("select kind from commands where command_id = ?1")
            .bind(response.command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command kind loads");
    let command_json: String =
        sqlx::query_scalar("select command_json from commands where command_id = ?1")
            .bind(response.command_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("command json loads");
    let command: CommandEnvelope = serde_json::from_str(&command_json).expect("command decodes");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(command_kind, "ResumeRuntime");
    let CommandPayload::ResumeRuntime {
        workspace_path,
        provider_resume_ref: Some(provider_resume_ref),
        ..
    } = command.payload
    else {
        panic!("expected typed resume payload");
    };
    assert_eq!(
        provider_resume_ref
            .0
            .get("provider_session_id")
            .and_then(serde_json::Value::as_str),
        Some("codex-session-1")
    );
    assert_eq!(workspace_path, detail.placement.workspace_path);
}

#[tokio::test]
async fn ready_event_clears_degraded_reason_after_sequence_gap() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id.clone(),
            "runtime-gap-before-ready",
            2,
            EventKind::ProviderOutputDelta,
            json!({ "delta": "late" }),
        ),
    )
    .await
    .expect("gap event accepts");
    accept_node_event(
        &state,
        node_event_fixture(
            &detail,
            node_id,
            "runtime-ready-after-gap",
            3,
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

    assert_eq!(detail.session.runtime.state, RuntimeSessionState::Ready);
    assert_eq!(detail.session.runtime.degraded_reason, None);
    assert_eq!(detail.session.state, SessionThreadState::Active);
}

async fn enroll_test_node(state: &Arc<AppState>) -> NodeEnrollmentClaimResponse {
    let requested = create_enrollment(
        state,
        "Test node",
        Some("0.1.0"),
        vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(true),
        }],
    )
    .await
    .expect("enrollment creates");
    let _ = approve_node_enrollment(
        State(state.clone()),
        Path(requested.enrollment_id.to_string()),
    )
    .await
    .expect("enrollment approves");
    claim_enrollment(
        state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id,
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("enrollment claims")
}

async fn create_test_session(state: &Arc<AppState>) -> (NodeId, SessionDetail, std::path::PathBuf) {
    let claim = enroll_test_node(state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    heartbeat_test_node(state, node_id.clone(), claim.credential.clone()).await;
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");
    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        }),
    )
    .await
    .expect("placement validates")
    .0;
    accept_workspace_validation_event(
        state,
        &placement,
        node_id.clone(),
        PlacementState::Validated,
        vec![],
    )
    .await;
    let detail = create_session(
        State(state.clone()),
        Json(CreateSessionRequest {
            project_placement_id: placement.project_placement_id,
            title: Some("Session".to_owned()),
            provider: "codex".to_owned(),
        }),
    )
    .await
    .expect("session creates")
    .0;
    (node_id, detail, workspace_path)
}

async fn heartbeat_test_node(state: &Arc<AppState>, node_id: NodeId, credential: Option<String>) {
    heartbeat_node(state, node_id, credential, "Test node", SleepHint::Awake, 0).await;
}

async fn heartbeat_node(
    state: &Arc<AppState>,
    node_id: NodeId,
    credential: Option<String>,
    display_name: &str,
    sleep_hint: SleepHint,
    active_runtime_count: i64,
) {
    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: display_name.to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![CapabilitySummary {
                key: "provider.codex".to_owned(),
                value: CapabilityValue::provider(true),
            }],
            diagnostics: None,
            active_runtime_count,
            sleep_hint,
            workspace_summaries: vec![],
        }),
    )
    .await
    .expect("heartbeat accepted");
}

async fn mark_node_offline(state: &Arc<AppState>, node_id: &NodeId) {
    age_node_heartbeat(state, node_id, state.config.offline_after_seconds + 1).await;
}

async fn age_node_heartbeat(state: &Arc<AppState>, node_id: &NodeId, age_seconds: i64) {
    let last_heartbeat_at = Utc::now() - chrono::Duration::seconds(age_seconds);
    sqlx::query(
            "update nodes set presence = 'reachable', last_heartbeat_at = ?1, updated_at = ?2 where node_id = ?3",
        )
        .bind(last_heartbeat_at)
        .bind(Utc::now())
        .bind(node_id.as_str())
        .execute(&state.pool)
        .await
        .expect("node heartbeat is aged");
}

async fn set_node_capabilities(
    state: &Arc<AppState>,
    node_id: &NodeId,
    capabilities: Vec<CapabilitySummary>,
) {
    let now = Utc::now();
    sqlx::query("update nodes set capabilities_json = ?1, updated_at = ?2 where node_id = ?3")
        .bind(serde_json::to_string(&capabilities).expect("capabilities serialize"))
        .bind(now)
        .bind(node_id.as_str())
        .execute(&state.pool)
        .await
        .expect("node capabilities update");
    replace_node_capabilities(state, node_id, &capabilities, now)
        .await
        .expect("normalized node capabilities update");
}

async fn set_session_runtime_state(
    state: &Arc<AppState>,
    detail: &SessionDetail,
    runtime_state: RuntimeSessionState,
) {
    sqlx::query("update runtime_sessions set state = ?1 where runtime_session_id = ?2")
        .bind(format_runtime_state(runtime_state))
        .bind(detail.session.runtime.runtime_session_id.as_str())
        .execute(&state.pool)
        .await
        .expect("runtime state updates");
}

async fn set_session_runtime_last_step(
    state: &Arc<AppState>,
    detail: &SessionDetail,
    last_runtime_step_at: DateTime<Utc>,
) {
    sqlx::query(
        "update runtime_sessions set last_runtime_step_at = ?1 where runtime_session_id = ?2",
    )
    .bind(last_runtime_step_at)
    .bind(detail.session.runtime.runtime_session_id.as_str())
    .execute(&state.pool)
    .await
    .expect("runtime last step updates");
}

async fn command_count(state: &Arc<AppState>) -> i64 {
    sqlx::query_scalar("select count(*) from commands")
        .fetch_one(&state.pool)
        .await
        .expect("command count loads")
}

async fn turn_id_for_command(state: &Arc<AppState>, command_id: &CommandId) -> TurnId {
    let turn_id: String = sqlx::query_scalar("select turn_id from turns where command_id = ?1")
        .bind(command_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("turn id loads");
    TurnId::from(turn_id)
}

async fn session_message_count(state: &Arc<AppState>, detail: &SessionDetail) -> i64 {
    sqlx::query_scalar("select count(*) from messages where session_thread_id = ?1")
        .bind(detail.session.session_thread_id.as_str())
        .fetch_one(&state.pool)
        .await
        .expect("message count loads")
}

async fn accept_workspace_validation_event(
    state: &Arc<AppState>,
    placement: &ProjectPlacementSummary,
    node_id: NodeId,
    placement_state: PlacementState,
    resource_badges: Vec<ResourceBadge>,
) {
    accept_placement_snapshot_event(
        state,
        placement,
        node_id,
        EventKind::WorkspaceValidated,
        placement_state,
        resource_badges,
    )
    .await;
}

async fn accept_placement_snapshot_event(
    state: &Arc<AppState>,
    placement: &ProjectPlacementSummary,
    node_id: NodeId,
    kind: EventKind,
    placement_state: PlacementState,
    resource_badges: Vec<ResourceBadge>,
) {
    accept_node_event(
        state,
        EventEnvelope {
            event_id: EventId::new(),
            command_id: None,
            correlation_id: None,
            actor_ref: ActorRef::Node {
                node_id: node_id.clone(),
            },
            scope_ref: ScopeRef::Placement {
                project_placement_id: placement.project_placement_id.clone(),
            },
            node_id: Some(node_id),
            runtime_session_id: None,
            session_thread_id: None,
            turn_id: None,
            seq: 1,
            session_projection_seq: None,
            kind,
            happened_at: Utc::now(),
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: EventPayload::from_json(
                kind,
                json!({
                    "placement_id": placement.project_placement_id.as_str(),
                    "display_name": placement.display_name.as_str(),
                    "workspace_path": placement.workspace_path.as_str(),
                    "state": placement_state,
                    "resource_badges": resource_badges,
                }),
            ),
        },
    )
    .await
    .expect("workspace validation event accepts");
}

fn command_fixture(command_id: CommandId, node_id: NodeId) -> CommandEnvelope {
    CommandEnvelope {
        command_id,
        kind: CommandKind::RefreshResourceSnapshot,
        target: CommandTarget::Node { node_id },
        actor_ref: ActorRef::local_user(),
        source_refs: vec![],
        cause_refs: vec![],
        issued_at: Utc::now(),
        correlation_id: CorrelationId::from("correlation-1"),
        payload: CommandPayload::RefreshResourceSnapshot {
            display_name: "fixture".to_owned(),
            workspace_path: "/workspace".to_owned(),
        },
    }
}

fn node_event_fixture(
    detail: &SessionDetail,
    node_id: NodeId,
    event_id: &str,
    seq: i64,
    kind: EventKind,
    payload: serde_json::Value,
) -> EventEnvelope {
    let runtime_session_id = detail.session.runtime.runtime_session_id.clone();
    EventEnvelope {
        event_id: EventId::from(event_id),
        command_id: None,
        correlation_id: None,
        actor_ref: ActorRef::Provider {
            provider: "codex".to_owned(),
        },
        scope_ref: ScopeRef::Runtime {
            runtime_session_id: runtime_session_id.clone(),
        },
        node_id: Some(node_id),
        runtime_session_id: Some(runtime_session_id),
        session_thread_id: Some(detail.session.session_thread_id.clone()),
        turn_id: Some(TurnId::from("turn-1")),
        seq,
        session_projection_seq: None,
        kind,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: EventPayload::from_json(kind, payload),
    }
}

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
