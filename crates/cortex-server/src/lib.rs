use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Arc,
};

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{
        header::{HeaderName, HeaderValue, InvalidHeaderValue, AUTHORIZATION, CONTENT_TYPE},
        HeaderMap, Method, StatusCode,
    },
    response::{IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use cortex_protocol::{
    serde_json_value::JsonValue, AcknowledgeWarningRequest, ActorRef, AgentProjection, ApiError,
    ApprovalId, ApprovalState, ApproveNodeEnrollmentResponse, ArtifactId, ArtifactTree,
    ArtifactTreeNode, CapabilitySummary, ClientCreateNodeEnrollmentRequest, ClientLogLevel,
    ClientLogRequest, ClientLogResponse, CommandAcceptedResponse, CommandEnvelope, CommandId,
    CommandKind, CommandState, ControlFrame, CorrelationId, CortexRef, CreatePlacementRequest,
    CreateSessionRequest, DeploymentProfile, EnrollmentId, EnrollmentState, EventEnvelope, EventId,
    EventKind, HealthResponse, InventorySnapshot, Message, MessageId, MessageRole,
    NodeDeletionResponse, NodeEnrollmentClaimRequest, NodeEnrollmentClaimResponse,
    NodeEnrollmentRequest, NodeEnrollmentRequestedResponse, NodeEnrollmentSummary,
    NodeHeartbeatRequest, NodeHeartbeatResponse, NodeId, NodePresence, NodeRevocationResponse,
    PlacementDeletionResponse, PlacementState, ProjectId, ProjectPlacementId,
    ProjectPlacementSummary, ResolveApprovalRequest, ResourceBadge, RuntimeSessionId,
    RuntimeSessionState, RuntimeSummary, ScopeRef, SendTurnRequest, SessionDetail, SessionSummary,
    SessionThreadId, SessionThreadState, SleepHint, TurnId, TurnState, VersionResponse,
    WarningAcknowledgementResponse, WarningSeverity, WorkspaceSnapshot,
};
use futures_util::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use tokio::sync::{broadcast, mpsc, RwLock};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

const API_VERSION: &str = "v1";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const SCHEMA_VERSION: i64 = 1;
const CORRELATION_ID_HEADER: &str = "x-correlation-id";
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_CLIENT_LOG_FIELD_CHARS: usize = 2_000;
const MAX_CLIENT_LOG_DETAIL_CHARS: usize = 8_000;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_address: String,
    pub database_url: String,
    pub profile: DeploymentProfile,
    pub allowed_origins: Vec<HeaderValue>,
    pub stale_after_seconds: i64,
    pub offline_after_seconds: i64,
    pub enrollment_ttl_seconds: i64,
    pub runtime_expiry_seconds: i64,
    pub auto_approve_enrollments: bool,
    pub client_log_file: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_address: std::env::var("CORTEX_CORE_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_owned()),
            database_url: std::env::var("CORTEX_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://.local/state/core.sqlite".to_owned()),
            profile: parse_profile(std::env::var("CORTEX_DEPLOYMENT_PROFILE").ok())?,
            allowed_origins: parse_allowed_origins(std::env::var("CORTEX_ALLOWED_ORIGINS").ok())?,
            stale_after_seconds: parse_env_i64("CORTEX_HEARTBEAT_STALE_SECONDS", 15)?,
            offline_after_seconds: parse_env_i64("CORTEX_HEARTBEAT_OFFLINE_SECONDS", 45)?,
            enrollment_ttl_seconds: parse_env_i64("CORTEX_ENROLLMENT_TTL_SECONDS", 600)?,
            runtime_expiry_seconds: parse_env_i64("CORTEX_RUNTIME_EXPIRY_SECONDS", 86_400)?,
            auto_approve_enrollments: parse_env_bool("CORTEX_AUTO_APPROVE_ENROLLMENTS", false),
            client_log_file: std::env::var("CORTEX_CLIENT_LOG_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".local/logs/client.log")),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid deployment profile `{0}`")]
    InvalidProfile(String),
    #[error("invalid CORS origin `{origin}`")]
    InvalidOrigin {
        origin: String,
        source: InvalidHeaderValue,
    },
    #[error("wildcard CORS origin is not allowed in trusted development profile")]
    WildcardOrigin,
    #[error("invalid integer environment variable `{name}`")]
    InvalidInteger {
        name: String,
        source: std::num::ParseIntError,
    },
}

fn parse_profile(value: Option<String>) -> Result<DeploymentProfile, ConfigError> {
    match value.as_deref() {
        Some("controlled_dev") => Ok(DeploymentProfile::ControlledDev),
        Some("local_trusted") | None => Ok(DeploymentProfile::LocalTrusted),
        Some(other) => Err(ConfigError::InvalidProfile(other.to_owned())),
    }
}

fn parse_allowed_origins(value: Option<String>) -> Result<Vec<HeaderValue>, ConfigError> {
    let Some(value) = value else {
        return Ok(default_allowed_origins());
    };
    let origins = value
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            if origin == "*" {
                return Err(ConfigError::WildcardOrigin);
            }
            HeaderValue::from_str(origin).map_err(|source| ConfigError::InvalidOrigin {
                origin: origin.to_owned(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if origins.is_empty() {
        Ok(default_allowed_origins())
    } else {
        Ok(origins)
    }
}

fn default_allowed_origins() -> Vec<HeaderValue> {
    vec![
        HeaderValue::from_static("http://127.0.0.1:5173"),
        HeaderValue::from_static("http://localhost:5173"),
    ]
}

fn parse_env_i64(name: &str, fallback: i64) -> Result<i64, ConfigError> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<i64>()
            .map_err(|source| ConfigError::InvalidInteger {
                name: name.to_owned(),
                source,
            }),
        Err(_) => Ok(fallback),
    }
}

fn parse_env_bool(name: &str, fallback: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(fallback)
}

pub struct AppState {
    config: AppConfig,
    pool: SqlitePool,
    control_channels: RwLock<HashMap<String, mpsc::UnboundedSender<ControlFrame>>>,
    event_tx: broadcast::Sender<EventEnvelope>,
}

impl AppState {
    pub async fn new(config: AppConfig, pool: SqlitePool) -> Result<Arc<Self>, AppError> {
        let state = Arc::new(Self {
            config,
            pool,
            control_channels: RwLock::new(HashMap::new()),
            event_tx: broadcast::channel(256).0,
        });
        state.migrate().await?;
        Ok(state)
    }

    async fn migrate(&self) -> Result<(), AppError> {
        for statement in MIGRATIONS {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        self.ensure_optional_schema().await?;
        Ok(())
    }

    async fn ensure_optional_schema(&self) -> Result<(), AppError> {
        let _ = sqlx::query("alter table nodes add column credential_hash text")
            .execute(&self.pool)
            .await;
        for statement in ["alter table runtime_sessions add column provider_resume_ref_json text"] {
            let _ = sqlx::query(statement).execute(&self.pool).await;
        }
        for statement in [
            "alter table commands add column actor_ref_json text",
            "alter table commands add column correlation_id text",
            "alter table commands add column source_refs_json text",
            "alter table commands add column cause_refs_json text",
            "alter table commands add column payload_json text",
            "alter table commands add column dedupe_key text",
        ] {
            let _ = sqlx::query(statement).execute(&self.pool).await;
        }
        for statement in [
            "alter table events add column actor_ref_json text",
            "alter table events add column scope_ref_json text",
            "alter table events add column correlation_id text",
            "alter table events add column source_refs_json text",
            "alter table events add column evidence_refs_json text",
            "alter table events add column cause_refs_json text",
            "alter table events add column result_refs_json text",
            "alter table events add column payload_json text",
        ] {
            let _ = sqlx::query(statement).execute(&self.pool).await;
        }
        Ok(())
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = cors_layer(&state.config);
    let api = Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/client/logs", post(client_logs))
        .route("/inventory", get(inventory))
        .route("/nodes", get(nodes))
        .route("/nodes/{node_id}", get(node_detail).delete(delete_node))
        .route("/nodes/{node_id}/revoke", post(revoke_node))
        .route(
            "/node-enrollments",
            get(node_enrollments).post(create_client_node_enrollment),
        )
        .route(
            "/node-enrollments/{enrollment_id}/approve",
            post(approve_node_enrollment),
        )
        .route("/node/enrollment-requests", post(node_enrollment_request))
        .route("/node/enrollment-claims", post(node_enrollment_claim))
        .route("/node/heartbeat", post(node_heartbeat))
        .route("/node/control", get(node_control))
        .route(
            "/project-placements/validate",
            post(validate_placement_route),
        )
        .route(
            "/placements/{placement_id}",
            get(placement_detail).delete(delete_placement),
        )
        .route(
            "/placements/{placement_id}/resource-snapshot/refresh",
            post(refresh_resource_snapshot_route),
        )
        .route("/sessions", post(create_session_route))
        .route("/sessions/{session_thread_id}", get(session_detail))
        .route("/sessions/{session_thread_id}/attach", post(attach_session))
        .route("/sessions/{session_thread_id}/detach", post(detach_session))
        .route(
            "/sessions/{session_thread_id}/messages",
            get(session_messages),
        )
        .route("/sessions/{session_thread_id}/events", get(session_events))
        .route("/sessions/{session_thread_id}/stream", get(session_stream))
        .route(
            "/sessions/{session_thread_id}/artifact-tree",
            get(session_artifact_tree),
        )
        .route(
            "/sessions/{session_thread_id}/agent-projection",
            get(session_agent_projection),
        )
        .route("/sessions/{session_thread_id}/turns", post(send_turn_route))
        .route(
            "/sessions/{session_thread_id}/approvals/{approval_id}/resolve",
            post(resolve_approval_route),
        )
        .route(
            "/sessions/{session_thread_id}/warnings/{warning_kind}/acknowledge",
            post(acknowledge_warning_route),
        )
        .route(
            "/runtime-sessions/{runtime_session_id}/interrupt",
            post(interrupt_runtime_route),
        )
        .route(
            "/runtime-sessions/{runtime_session_id}/stop",
            post(stop_runtime_route),
        )
        .route(
            "/runtime-sessions/{runtime_session_id}/resume",
            post(resume_runtime_route),
        );

    Router::new()
        .nest("/api/v1", api)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

fn cors_layer(config: &AppConfig) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(config.allowed_origins.clone()))
        .allow_methods([Method::DELETE, Method::GET, Method::POST])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("last-event-id"),
            HeaderName::from_static(CORRELATION_ID_HEADER),
            HeaderName::from_static(REQUEST_ID_HEADER),
        ])
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            let _ = signal.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        profile: state.config.profile,
    })
}

async fn version(State(state): State<Arc<AppState>>) -> Json<VersionResponse> {
    Json(VersionResponse {
        name: "cortex-core".to_owned(),
        version: APP_VERSION.to_owned(),
        api_version: API_VERSION.to_owned(),
        schema_version: SCHEMA_VERSION,
        profile: state.config.profile,
    })
}

async fn client_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ClientLogRequest>,
) -> Result<Json<ClientLogResponse>, AppError> {
    let accepted_at = Utc::now();
    let user_agent = request
        .user_agent
        .filter(|value| !value.trim().is_empty())
        .or_else(|| header_value(&headers, "user-agent"))
        .map(|value| truncate_chars(value.trim(), MAX_CLIENT_LOG_FIELD_CHARS));
    let record = json!({
        "accepted_at": accepted_at,
        "occurred_at": request.occurred_at,
        "level": format_client_log_level(request.level),
        "source": truncate_chars(request.source.trim(), MAX_CLIENT_LOG_FIELD_CHARS),
        "message": truncate_chars(request.message.trim(), MAX_CLIENT_LOG_FIELD_CHARS),
        "route": request
            .route
            .filter(|value| !value.trim().is_empty())
            .map(|value| truncate_chars(value.trim(), MAX_CLIENT_LOG_FIELD_CHARS)),
        "user_agent": user_agent,
        "detail": truncate_chars(
            &serde_json::to_string(&request.detail.0).unwrap_or_else(|_| "null".to_owned()),
            MAX_CLIENT_LOG_DETAIL_CHARS,
        ),
    });
    append_jsonl_log(
        state.config.client_log_file.clone(),
        serde_json::to_string(&record)?,
    )
    .await?;
    tracing::debug!(
        level = format_client_log_level(request.level),
        source = %request.source,
        "client log accepted"
    );
    Ok(Json(ClientLogResponse { accepted: true }))
}

async fn inventory(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InventorySnapshot>, AppError> {
    Ok(Json(load_inventory(&state).await?))
}

async fn nodes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<cortex_protocol::NodeSummary>>, AppError> {
    Ok(Json(load_nodes(&state).await?))
}

async fn node_detail(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Result<Json<cortex_protocol::NodeSummary>, AppError> {
    let node_id = NodeId::from(node_id);
    load_nodes(&state)
        .await?
        .into_iter()
        .find(|node| node.node_id == node_id)
        .map(Json)
        .ok_or_else(|| AppError::not_found("node.not_found", "Node not found"))
}

async fn node_enrollments(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NodeEnrollmentSummary>>, AppError> {
    load_enrollments(&state).await.map(Json)
}

async fn create_client_node_enrollment(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ClientCreateNodeEnrollmentRequest>,
) -> Result<Json<NodeEnrollmentRequestedResponse>, AppError> {
    create_enrollment(&state, &request.display_name, None, Vec::new())
        .await
        .map(Json)
}

async fn node_enrollment_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NodeEnrollmentRequest>,
) -> Result<Json<NodeEnrollmentRequestedResponse>, AppError> {
    create_enrollment(
        &state,
        &request.display_name,
        Some(&request.daemon_version),
        request.capabilities,
    )
    .await
    .map(Json)
}

async fn approve_node_enrollment(
    State(state): State<Arc<AppState>>,
    Path(enrollment_id): Path<String>,
) -> Result<Json<ApproveNodeEnrollmentResponse>, AppError> {
    let now = Utc::now();
    let enrollment_id = EnrollmentId::from(enrollment_id);
    let updated = sqlx::query(
        r#"
        update node_enrollments
        set status = 'approved', approved_at = ?1, updated_at = ?1
        where enrollment_id = ?2
          and status = 'pending_user_approval'
          and approved_at is null
          and expires_at > ?1
        "#,
    )
    .bind(now)
    .bind(enrollment_id.as_str())
    .execute(&state.pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::bad_request(
            "node_enrollment.not_approvable",
            "Enrollment is missing, expired or already claimed",
        ));
    }
    let enrollment = load_enrollment(&state, &enrollment_id).await?;
    tracing::info!(
        enrollment_id = %enrollment_id,
        "node enrollment approved"
    );
    Ok(Json(ApproveNodeEnrollmentResponse { enrollment }))
}

async fn node_enrollment_claim(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NodeEnrollmentClaimRequest>,
) -> Result<Json<NodeEnrollmentClaimResponse>, AppError> {
    claim_enrollment(&state, &request).await.map(Json)
}

async fn revoke_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Result<Json<NodeRevocationResponse>, AppError> {
    let node_id = NodeId::from(node_id);
    let updated = sqlx::query(
        r#"
        update nodes
        set presence = 'revoked', credential_hash = null, updated_at = ?1
        where node_id = ?2
        "#,
    )
    .bind(Utc::now())
    .bind(node_id.as_str())
    .execute(&state.pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::not_found("node.not_found", "Node not found"));
    }
    tracing::warn!(node_id = %node_id, "node revoked");
    Ok(Json(NodeRevocationResponse {
        node_id,
        revoked: true,
    }))
}

async fn delete_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Result<Json<NodeDeletionResponse>, AppError> {
    let node_id = NodeId::from(node_id);
    let mut transaction = state.pool.begin().await?;
    let exists = sqlx::query_scalar::<_, i64>("select 1 from nodes where node_id = ?1")
        .bind(node_id.as_str())
        .fetch_optional(&mut *transaction)
        .await?
        .is_some();
    if !exists {
        return Err(AppError::not_found("node.not_found", "Node not found"));
    }

    let deleted_sessions = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from session_threads st
        join project_placements pp on pp.project_placement_id = st.project_placement_id
        where pp.node_id = ?1
        "#,
    )
    .bind(node_id.as_str())
    .fetch_one(&mut *transaction)
    .await?;
    let deleted_placements =
        sqlx::query_scalar::<_, i64>("select count(*) from project_placements where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_one(&mut *transaction)
            .await?;

    for statement in [
        r#"
        delete from events
        where node_id = ?1
           or runtime_session_id in (
                select rs.runtime_session_id
                from runtime_sessions rs
                join session_threads st on st.session_thread_id = rs.session_thread_id
                join project_placements pp on pp.project_placement_id = st.project_placement_id
                where pp.node_id = ?1
           )
           or session_thread_id in (
                select st.session_thread_id
                from session_threads st
                join project_placements pp on pp.project_placement_id = st.project_placement_id
                where pp.node_id = ?1
           )
        "#,
        r#"
        delete from warning_acknowledgements
        where session_thread_id in (
            select st.session_thread_id
            from session_threads st
            join project_placements pp on pp.project_placement_id = st.project_placement_id
            where pp.node_id = ?1
        )
        "#,
        r#"
        delete from approvals
        where session_thread_id in (
            select st.session_thread_id
            from session_threads st
            join project_placements pp on pp.project_placement_id = st.project_placement_id
            where pp.node_id = ?1
        )
        "#,
        r#"
        delete from messages
        where session_thread_id in (
            select st.session_thread_id
            from session_threads st
            join project_placements pp on pp.project_placement_id = st.project_placement_id
            where pp.node_id = ?1
        )
        "#,
        r#"
        delete from turns
        where session_thread_id in (
            select st.session_thread_id
            from session_threads st
            join project_placements pp on pp.project_placement_id = st.project_placement_id
            where pp.node_id = ?1
        )
        "#,
        r#"
        delete from runtime_sessions
        where session_thread_id in (
            select st.session_thread_id
            from session_threads st
            join project_placements pp on pp.project_placement_id = st.project_placement_id
            where pp.node_id = ?1
        )
        "#,
        r#"
        delete from session_threads
        where project_placement_id in (
            select project_placement_id from project_placements where node_id = ?1
        )
        "#,
        "delete from commands where target_node_id = ?1",
        "delete from project_placements where node_id = ?1",
        "delete from node_capabilities where node_id = ?1",
        "delete from node_enrollments where claimed_node_id = ?1",
    ] {
        sqlx::query(statement)
            .bind(node_id.as_str())
            .execute(&mut *transaction)
            .await?;
    }

    let deleted = sqlx::query("delete from nodes where node_id = ?1")
        .bind(node_id.as_str())
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    transaction.commit().await?;
    state
        .control_channels
        .write()
        .await
        .remove(node_id.as_str());

    if deleted == 0 {
        return Err(AppError::not_found("node.not_found", "Node not found"));
    }
    tracing::warn!(
        node_id = %node_id,
        deleted_placements,
        deleted_sessions,
        "node deleted"
    );
    Ok(Json(NodeDeletionResponse {
        node_id,
        deleted: true,
    }))
}

async fn node_heartbeat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NodeHeartbeatRequest>,
) -> Result<Json<NodeHeartbeatResponse>, AppError> {
    let now = Utc::now();
    let node_id = request
        .node_id
        .ok_or_else(|| AppError::auth("auth_dev.node_id_required", "Node id is required"))?;
    verify_node_credential(&state, &node_id, request.credential.as_deref()).await?;
    let display_name = request.display_name;
    let daemon_version = request.daemon_version;
    let active_runtime_count = request.active_runtime_count;
    let capabilities = request.capabilities;
    let diagnostics = request
        .diagnostics
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "heartbeat accepted".to_owned());
    let workspace_summaries = request.workspace_summaries;
    let workspace_count = workspace_summaries.len();
    let capabilities_json = serde_json::to_string(&capabilities)?;
    sqlx::query(
        r#"
        insert into nodes (
            node_id, display_name, presence, sleep_hint, last_heartbeat_at,
            daemon_version, active_runtime_count, capabilities_json, diagnostics, created_at, updated_at
        )
        values (?1, ?2, 'reachable', ?3, ?4, ?5, ?6, ?7, ?8, ?4, ?4)
        on conflict(node_id) do update set
            display_name = excluded.display_name,
            presence = 'reachable',
            sleep_hint = excluded.sleep_hint,
            last_heartbeat_at = excluded.last_heartbeat_at,
            daemon_version = excluded.daemon_version,
            active_runtime_count = excluded.active_runtime_count,
            capabilities_json = excluded.capabilities_json,
            diagnostics = excluded.diagnostics,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(node_id.as_str())
    .bind(display_name)
    .bind(format_sleep_hint(request.sleep_hint))
    .bind(now)
    .bind(daemon_version)
    .bind(active_runtime_count)
    .bind(&capabilities_json)
    .bind(diagnostics)
    .execute(&state.pool)
    .await?;

    replace_node_capabilities(&state, &node_id, &capabilities, now).await?;
    upsert_heartbeat_workspaces(&state, &node_id, workspace_summaries).await?;
    let open_control_channel = should_open_control_channel(&state, &node_id).await?;
    tracing::debug!(
        node_id = %node_id,
        active_runtime_count,
        workspace_count,
        open_control_channel,
        "node heartbeat accepted"
    );

    Ok(Json(NodeHeartbeatResponse {
        accepted: true,
        node_id: node_id.clone(),
        open_control_channel,
        server_time: now,
    }))
}

async fn node_control(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let node_id = header_value(&headers, "x-cortex-node-id")
        .map(NodeId::from)
        .ok_or_else(|| AppError::auth("auth_dev.node_id_required", "Node id is required"))?;
    let credential = bearer_token(&headers).ok_or_else(|| {
        AppError::auth(
            "auth_dev.credential_required",
            "Node credential is required",
        )
    })?;
    verify_node_credential(&state, &node_id, Some(&credential)).await?;

    Ok(ws.on_upgrade(move |socket| handle_control_socket(state, node_id, socket)))
}

async fn handle_control_socket(state: Arc<AppState>, node_id: NodeId, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ControlFrame>();
    state
        .control_channels
        .write()
        .await
        .insert(node_id.to_string(), tx.clone());
    tracing::info!(node_id = %node_id, "node control channel connected");

    let send_task = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&frame) else {
                continue;
            };
            if sender.send(WsMessage::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = receiver.next().await {
        let Ok(WsMessage::Text(text)) = message else {
            continue;
        };
        match serde_json::from_str::<ControlFrame>(&text) {
            Ok(frame) => {
                if let Err(error) = handle_node_control_frame(&state, &node_id, frame).await {
                    tracing::warn!(node_id = %node_id, error = %error, "control frame failed");
                }
            }
            Err(error) => {
                tracing::warn!(node_id = %node_id, error = %error, "invalid control frame");
            }
        }
    }

    state
        .control_channels
        .write()
        .await
        .remove(node_id.as_str());
    tracing::info!(node_id = %node_id, "node control channel disconnected");
    send_task.abort();
}

async fn handle_node_control_frame(
    state: &AppState,
    node_id: &NodeId,
    frame: ControlFrame,
) -> Result<(), AppError> {
    if !matches!(frame, ControlFrame::Hello { .. })
        && control_frame_protocol_version(&frame) != API_VERSION
    {
        send_control_error(
            state,
            node_id,
            "control.protocol_incompatible",
            "Control protocol version is incompatible",
            false,
        )
        .await;
        return Err(AppError::bad_request(
            "control.protocol_incompatible",
            "Control protocol version is incompatible",
        ));
    }

    match frame {
        ControlFrame::Hello {
            protocol_version,
            node_id: hello_node_id,
            ..
        } => {
            if hello_node_id != *node_id {
                send_control_error(
                    state,
                    node_id,
                    "control.node_mismatch",
                    "Control hello node id does not match authenticated node",
                    false,
                )
                .await;
                return Err(AppError::auth(
                    "control.node_mismatch",
                    "Control hello node id does not match authenticated node",
                ));
            }
            if protocol_version != API_VERSION {
                send_control_error(
                    state,
                    node_id,
                    "control.protocol_incompatible",
                    "Control protocol version is incompatible",
                    false,
                )
                .await;
                return Err(AppError::bad_request(
                    "control.protocol_incompatible",
                    "Control protocol version is incompatible",
                ));
            }
            send_control_frame(
                state,
                node_id,
                ControlFrame::HelloAck {
                    frame_id: Uuid::new_v4().to_string(),
                    protocol_version: API_VERSION.to_owned(),
                    sent_at: Utc::now(),
                },
            )
            .await;
            dispatch_pending_commands(state, node_id).await
        }
        ControlFrame::CommandAck {
            command_id, status, ..
        } => {
            tracing::debug!(
                node_id = %node_id,
                command_id = %command_id,
                command_state = ?status,
                "node command acknowledged"
            );
            update_command_state(state, &command_id, status).await
        }
        ControlFrame::CommandResult {
            command_id, status, ..
        } => {
            tracing::info!(
                node_id = %node_id,
                command_id = %command_id,
                command_state = ?status,
                "node command result received"
            );
            update_command_state(state, &command_id, status).await
        }
        ControlFrame::EventBatch { events, .. } => {
            let event_count = events.len();
            let mut accepted_event_ids = Vec::with_capacity(events.len());
            for event in events {
                let event_id = event.event_id.clone();
                accept_node_event(state, event).await?;
                accepted_event_ids.push(event_id);
            }
            if let Some(channel) = state.control_channels.read().await.get(node_id.as_str()) {
                let _ = channel.send(ControlFrame::EventBatchAck {
                    frame_id: Uuid::new_v4().to_string(),
                    protocol_version: API_VERSION.to_owned(),
                    sent_at: Utc::now(),
                    accepted_event_ids,
                });
            }
            tracing::info!(
                node_id = %node_id,
                event_count,
                "node event batch accepted"
            );
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn send_control_error(
    state: &AppState,
    node_id: &NodeId,
    error_code: &str,
    message: &str,
    retryable: bool,
) {
    send_control_frame(
        state,
        node_id,
        ControlFrame::ControlError {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            error: ApiError {
                error_code: error_code.to_owned(),
                message: message.to_owned(),
                details: JsonValue(json!({})),
                retryable,
                correlation_id: CorrelationId::from(Uuid::new_v4().to_string()),
            },
        },
    )
    .await;
}

async fn send_control_frame(state: &AppState, node_id: &NodeId, frame: ControlFrame) -> bool {
    let channel = state
        .control_channels
        .read()
        .await
        .get(node_id.as_str())
        .cloned();
    channel
        .map(|channel| channel.send(frame).is_ok())
        .unwrap_or(false)
}

fn control_frame_protocol_version(frame: &ControlFrame) -> &str {
    match frame {
        ControlFrame::Hello {
            protocol_version, ..
        }
        | ControlFrame::HelloAck {
            protocol_version, ..
        }
        | ControlFrame::CommandDispatch {
            protocol_version, ..
        }
        | ControlFrame::CommandAck {
            protocol_version, ..
        }
        | ControlFrame::CommandResult {
            protocol_version, ..
        }
        | ControlFrame::EventBatch {
            protocol_version, ..
        }
        | ControlFrame::EventBatchAck {
            protocol_version, ..
        }
        | ControlFrame::Ping {
            protocol_version, ..
        }
        | ControlFrame::Pong {
            protocol_version, ..
        }
        | ControlFrame::ControlError {
            protocol_version, ..
        } => protocol_version,
    }
}

async fn validate_placement_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreatePlacementRequest>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    validate_placement_with_correlation(&state, request, request_correlation_id(&headers))
        .await
        .map(Json)
}

#[cfg(test)]
async fn validate_placement(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePlacementRequest>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    validate_placement_with_correlation(&state, request, CorrelationId::new())
        .await
        .map(Json)
}

async fn validate_placement_with_correlation(
    state: &AppState,
    request: CreatePlacementRequest,
    correlation_id: CorrelationId,
) -> Result<ProjectPlacementSummary, AppError> {
    ensure_node_commandable(state, &request.node_id).await?;
    if request.display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.display_name_required",
            "Display name is required",
        ));
    }
    if request.workspace_path.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.workspace_path_required",
            "Workspace path is required",
        ));
    }

    let now = Utc::now();
    let project_id = ProjectId::new();
    let placement_id = ProjectPlacementId::new();
    let display_name = request.display_name.trim().to_owned();
    let workspace_path = request.workspace_path.trim().to_owned();
    sqlx::query(
        "delete from deleted_workspace_bindings where node_id = ?1 and workspace_path = ?2",
    )
    .bind(request.node_id.as_str())
    .bind(&workspace_path)
    .execute(&state.pool)
    .await?;
    upsert_project(state, &project_id, &display_name, now).await?;
    sqlx::query(
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
    .bind(request.node_id.as_str())
    .bind(&display_name)
    .bind(&workspace_path)
    .bind(format_placement_state(PlacementState::Pending))
    .bind(serde_json::to_string(&Vec::<ResourceBadge>::new())?)
    .bind(Option::<DateTime<Utc>>::None)
    .bind(now)
    .execute(&state.pool)
    .await?;

    record_and_dispatch_command(
        state,
        CommandEnvelope {
            command_id: CommandId::new(),
            kind: CommandKind::ValidateWorkspace,
            target_node_id: request.node_id.clone(),
            actor_ref: ActorRef::local_user(),
            session_thread_id: None,
            runtime_session_id: None,
            project_placement_id: Some(placement_id.clone()),
            source_refs: vec![CortexRef::Placement {
                placement_id: placement_id.clone(),
            }],
            cause_refs: vec![],
            issued_at: now,
            correlation_id,
            payload: JsonValue(json!({
                "display_name": display_name,
                "workspace_path": workspace_path,
            })),
        },
    )
    .await?;

    load_placement(state, &placement_id).await
}

async fn placement_detail(
    State(state): State<Arc<AppState>>,
    Path(placement_id): Path<String>,
) -> Result<Json<ProjectPlacementSummary>, AppError> {
    load_placement(&state, &ProjectPlacementId::from(placement_id))
        .await
        .map(Json)
}

async fn delete_placement(
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
    tracing::warn!(
        placement_id = %placement_id,
        node_id,
        workspace_path,
        deleted_sessions,
        "placement deleted"
    );
    Ok(Json(PlacementDeletionResponse {
        project_placement_id: placement_id,
        deleted: true,
    }))
}

async fn refresh_resource_snapshot_route(
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
async fn refresh_resource_snapshot(
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

async fn refresh_resource_snapshot_with_correlation(
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
            target_node_id: placement.node_id.clone(),
            actor_ref: ActorRef::local_user(),
            session_thread_id: None,
            runtime_session_id: None,
            project_placement_id: Some(placement.project_placement_id.clone()),
            source_refs: vec![CortexRef::Placement {
                placement_id: placement.project_placement_id.clone(),
            }],
            cause_refs: vec![],
            issued_at: now,
            correlation_id,
            payload: JsonValue(json!({
                "display_name": placement.display_name,
                "workspace_path": placement.workspace_path,
            })),
        },
    )
    .await?;

    Ok(CommandAcceptedResponse {
        command_id,
        session: None,
    })
}

async fn create_session_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<SessionDetail>, AppError> {
    create_session_with_correlation(&state, request, request_correlation_id(&headers))
        .await
        .map(Json)
}

#[cfg(test)]
async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<SessionDetail>, AppError> {
    create_session_with_correlation(&state, request, CorrelationId::new())
        .await
        .map(Json)
}

async fn create_session_with_correlation(
    state: &AppState,
    request: CreateSessionRequest,
    correlation_id: CorrelationId,
) -> Result<SessionDetail, AppError> {
    let placement = load_placement(state, &request.project_placement_id).await?;
    let provider = request.provider.unwrap_or_else(|| "fake".to_owned());
    ensure_node_commandable(state, &placement.node_id).await?;
    ensure_placement_startable(&placement)?;
    ensure_node_supports_provider(state, &placement.node_id, &provider).await?;
    let now = Utc::now();
    let session_thread_id = SessionThreadId::new();
    let runtime_session_id = RuntimeSessionId::new();
    let title = request
        .title
        .unwrap_or_else(|| format!("Session for {}", placement.display_name));

    sqlx::query(
        r#"
        insert into session_threads (
            session_thread_id, project_placement_id, runtime_session_id, title,
            state, provider, created_at, updated_at
        )
        values (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?6)
        "#,
    )
    .bind(session_thread_id.as_str())
    .bind(request.project_placement_id.as_str())
    .bind(runtime_session_id.as_str())
    .bind(title)
    .bind(&provider)
    .bind(now)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        r#"
        insert into runtime_sessions (
            runtime_session_id, session_thread_id, provider, state,
            resume_supported, provider_resume_ref_json, degraded_reason,
            last_runtime_step_at, created_at, updated_at
        )
        values (?1, ?2, ?3, 'starting', 1, null, null, ?4, ?4, ?4)
        "#,
    )
    .bind(runtime_session_id.as_str())
    .bind(session_thread_id.as_str())
    .bind(&provider)
    .bind(now)
    .execute(&state.pool)
    .await?;

    let command = CommandEnvelope {
        command_id: CommandId::new(),
        kind: CommandKind::StartRuntime,
        target_node_id: placement.node_id.clone(),
        actor_ref: ActorRef::local_user(),
        session_thread_id: Some(session_thread_id.clone()),
        runtime_session_id: Some(runtime_session_id.clone()),
        project_placement_id: Some(request.project_placement_id.clone()),
        source_refs: vec![],
        cause_refs: vec![],
        issued_at: now,
        correlation_id,
        payload: JsonValue(json!({
            "provider": provider.clone(),
            "workspace_path": placement.workspace_path,
        })),
    };
    record_and_dispatch_command(state, command).await?;

    load_session_detail(state, &session_thread_id).await
}

async fn session_detail(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<SessionDetail>, AppError> {
    load_session_detail(&state, &SessionThreadId::from(session_thread_id))
        .await
        .map(Json)
}

async fn attach_session(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<SessionDetail>, AppError> {
    let session_id = SessionThreadId::from(session_thread_id);
    update_session_attachment_state(&state, &session_id, SessionThreadState::Active).await?;
    load_session_detail(&state, &session_id).await.map(Json)
}

async fn detach_session(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<SessionDetail>, AppError> {
    let session_id = SessionThreadId::from(session_thread_id);
    update_session_attachment_state(&state, &session_id, SessionThreadState::Detached).await?;
    load_session_detail(&state, &session_id).await.map(Json)
}

async fn session_messages(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<Vec<Message>>, AppError> {
    load_messages(&state, &SessionThreadId::from(session_thread_id))
        .await
        .map(Json)
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after_seq: Option<i64>,
}

async fn session_events(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<EventEnvelope>>, AppError> {
    load_events(
        &state,
        &SessionThreadId::from(session_thread_id),
        query.after_seq.unwrap_or(0),
    )
    .await
    .map(Json)
}

async fn session_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_thread_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>>, AppError> {
    let session_id = SessionThreadId::from(session_thread_id);
    let after_seq = stream_resume_after_seq(&query, &headers);
    let mut event_rx = state.event_tx.subscribe();
    let events = load_events(&state, &session_id, after_seq).await?;

    let stream = async_stream::stream! {
        let mut last_seq = after_seq;
        for event in events {
            last_seq = last_seq.max(event.seq);
            yield Ok(sse_event_for_event(&event));
        }
        loop {
            match event_rx.recv().await {
                Ok(event) if event_matches_session_after_seq(&event, &session_id, last_seq) => {
                    last_seq = event.seq;
                    yield Ok(sse_event_for_event(&event));
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(axum::response::sse::Event::default()
                        .event("cortex.reload")
                        .data(r#"{"reason":"stream_lagged"}"#));
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream))
}

fn stream_resume_after_seq(query: &EventsQuery, headers: &HeaderMap) -> i64 {
    query
        .after_seq
        .or_else(|| last_event_id_after_seq(headers))
        .unwrap_or(0)
}

fn last_event_id_after_seq(headers: &HeaderMap) -> Option<i64> {
    header_value(headers, "last-event-id").and_then(|value| {
        value
            .parse::<i64>()
            .ok()
            .filter(|after_seq| *after_seq >= 0)
    })
}

fn sse_event_for_event(event: &EventEnvelope) -> axum::response::sse::Event {
    let data = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_owned());
    axum::response::sse::Event::default()
        .id(event.seq.to_string())
        .event("cortex.event")
        .data(data)
}

fn event_matches_session_after_seq(
    event: &EventEnvelope,
    session_id: &SessionThreadId,
    after_seq: i64,
) -> bool {
    event
        .session_thread_id
        .as_ref()
        .is_some_and(|event_session_id| event_session_id == session_id)
        && event.seq > after_seq
}

async fn session_artifact_tree(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<ArtifactTree>, AppError> {
    build_artifact_tree(&state, &SessionThreadId::from(session_thread_id))
        .await
        .map(Json)
}

async fn session_agent_projection(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
) -> Result<Json<AgentProjection>, AppError> {
    build_agent_projection(&state, &SessionThreadId::from(session_thread_id))
        .await
        .map(Json)
}

async fn send_turn_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_thread_id): Path<String>,
    Json(request): Json<SendTurnRequest>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    send_turn_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        request,
        request_correlation_id(&headers),
    )
    .await
    .map(Json)
}

#[cfg(test)]
async fn send_turn(
    State(state): State<Arc<AppState>>,
    Path(session_thread_id): Path<String>,
    Json(request): Json<SendTurnRequest>,
) -> Result<Json<CommandAcceptedResponse>, AppError> {
    send_turn_with_correlation(
        &state,
        SessionThreadId::from(session_thread_id),
        request,
        CorrelationId::new(),
    )
    .await
    .map(Json)
}

async fn send_turn_with_correlation(
    state: &AppState,
    session_id: SessionThreadId,
    request: SendTurnRequest,
    correlation_id: CorrelationId,
) -> Result<CommandAcceptedResponse, AppError> {
    if request.content.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.empty_turn",
            "Turn content cannot be empty",
        ));
    }

    let detail = load_session_detail(state, &session_id).await?;
    ensure_session_commandable(state, &detail, CommandKind::SendTurn).await?;
    let now = Utc::now();
    let command_id = CommandId::new();
    let turn_id = TurnId::new();
    let user_message_id = MessageId::new();
    let content = request.content;

    let command = CommandEnvelope {
        command_id: command_id.clone(),
        kind: CommandKind::SendTurn,
        target_node_id: detail.placement.node_id.clone(),
        actor_ref: ActorRef::local_user(),
        session_thread_id: Some(session_id.clone()),
        runtime_session_id: Some(detail.session.runtime.runtime_session_id.clone()),
        project_placement_id: Some(detail.placement.project_placement_id.clone()),
        source_refs: vec![],
        cause_refs: vec![CortexRef::Session {
            session_thread_id: session_id.clone(),
        }],
        issued_at: now,
        correlation_id,
        payload: JsonValue(json!({
            "content": content.clone(),
            "turn_id": turn_id.as_str(),
        })),
    };
    record_command(state, command).await?;
    insert_turn(state, &turn_id, &session_id, &command_id, &content, now).await?;

    insert_message(
        state,
        &Message {
            message_id: user_message_id,
            session_thread_id: session_id.clone(),
            turn_id: Some(turn_id.clone()),
            role: MessageRole::User,
            content,
            created_at: now,
            completed_at: Some(now),
            source_event_id: None,
        },
    )
    .await?;
    update_command_state(state, &command_id, CommandState::PendingDispatch).await?;
    dispatch_pending_commands(state, &detail.placement.node_id).await?;

    let session = load_session_detail(state, &session_id).await?;
    Ok(CommandAcceptedResponse {
        command_id,
        session: Some(session),
    })
}

async fn resolve_approval_route(
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
async fn resolve_approval(
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

async fn resolve_approval_with_correlation(
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
            target_node_id: detail.placement.node_id.clone(),
            actor_ref: ActorRef::local_user(),
            session_thread_id: Some(session_id.clone()),
            runtime_session_id: Some(detail.session.runtime.runtime_session_id.clone()),
            project_placement_id: Some(detail.placement.project_placement_id),
            source_refs: vec![CortexRef::Approval {
                approval_id: approval_id.clone(),
            }],
            cause_refs: vec![CortexRef::Session {
                session_thread_id: session_id.clone(),
            }],
            issued_at: Utc::now(),
            correlation_id,
            payload: JsonValue(json!({
                "approval_id": approval_id.as_str(),
                "approved": request.approved,
                "message": request.message,
            })),
        },
    )
    .await?;

    let session = load_session_detail(state, &session_id).await?;
    Ok(CommandAcceptedResponse {
        command_id,
        session: Some(session),
    })
}

async fn acknowledge_warning_route(
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
async fn acknowledge_warning(
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

async fn acknowledge_warning_with_correlation(
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

async fn interrupt_runtime_route(
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
async fn interrupt_runtime(
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

async fn stop_runtime_route(
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

async fn resume_runtime_route(
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
async fn resume_runtime(
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

async fn lifecycle_command(
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
            target_node_id: detail.placement.node_id.clone(),
            actor_ref: ActorRef::local_user(),
            session_thread_id: Some(session_id.clone()),
            runtime_session_id: Some(runtime_session_id.clone()),
            project_placement_id: Some(detail.placement.project_placement_id),
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id,
            payload: JsonValue(payload),
        },
    )
    .await?;

    Ok(CommandAcceptedResponse {
        command_id,
        session: Some(load_session_detail(state, &session_id).await?),
    })
}

async fn lifecycle_command_payload(
    state: &AppState,
    detail: &SessionDetail,
    kind: CommandKind,
) -> Result<serde_json::Value, AppError> {
    if kind != CommandKind::ResumeRuntime {
        return Ok(json!({}));
    }

    let provider_resume_ref =
        runtime_provider_resume_ref_json(state, &detail.session.runtime.runtime_session_id).await?;
    let mut payload = json!({
        "provider": detail.session.runtime.provider.as_str(),
        "workspace_path": detail.placement.workspace_path.as_str(),
    });
    if let Some(provider_resume_ref) = provider_resume_ref {
        payload["provider_resume_ref"] = provider_resume_ref;
    }
    Ok(payload)
}

async fn runtime_provider_resume_ref_json(
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

async fn record_and_dispatch_command(
    state: &AppState,
    command: CommandEnvelope,
) -> Result<(), AppError> {
    let command_id = command.command_id.clone();
    let target_node_id = command.target_node_id.clone();
    record_command(state, command).await?;
    update_command_state(state, &command_id, CommandState::PendingDispatch).await?;
    dispatch_pending_commands(state, &target_node_id).await
}

async fn dispatch_pending_commands(state: &AppState, node_id: &NodeId) -> Result<(), AppError> {
    let rows = sqlx::query(
        r#"
        select command_id, command_json
        from commands
        where target_node_id = ?1 and state in ('recorded', 'pending_dispatch', 'dispatched')
        order by created_at asc
        "#,
    )
    .bind(node_id.as_str())
    .fetch_all(&state.pool)
    .await?;
    let pending_count = rows.len();
    let Some(channel) = state
        .control_channels
        .read()
        .await
        .get(node_id.as_str())
        .cloned()
    else {
        if pending_count > 0 {
            tracing::debug!(
                node_id = %node_id,
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
            .send(ControlFrame::CommandDispatch {
                frame_id: Uuid::new_v4().to_string(),
                protocol_version: API_VERSION.to_owned(),
                sent_at: Utc::now(),
                command,
            })
            .is_ok()
        {
            tracing::info!(
                node_id = %node_id,
                command_id = %command_id,
                command_kind = ?command_kind,
                correlation_id = %correlation_id,
                "command dispatched"
            );
            update_command_state(state, &command_id, CommandState::Dispatched).await?;
        } else {
            tracing::warn!(
                node_id = %node_id,
                command_id = %command_id,
                command_kind = ?command_kind,
                correlation_id = %correlation_id,
                "command dispatch channel closed"
            );
        }
    }
    Ok(())
}

async fn should_open_control_channel(state: &AppState, node_id: &NodeId) -> Result<bool, AppError> {
    if state
        .control_channels
        .read()
        .await
        .contains_key(node_id.as_str())
    {
        return Ok(false);
    }
    let pending: i64 = sqlx::query_scalar(
        "select count(*) from commands where target_node_id = ?1 and state in ('recorded', 'pending_dispatch', 'dispatched')",
    )
    .bind(node_id.as_str())
    .fetch_one(&state.pool)
    .await?;
    Ok(pending > 0)
}

async fn update_command_state(
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
    sqlx::query("update commands set state = ?1, completed_at = coalesce(?2, completed_at) where command_id = ?3")
        .bind(format_command_state(command_state))
        .bind(completed_at)
        .bind(command_id.as_str())
        .execute(&state.pool)
        .await?;
    tracing::debug!(
        command_id = %command_id,
        command_state = ?command_state,
        "command state updated"
    );
    Ok(())
}

async fn accept_node_event(state: &AppState, event: EventEnvelope) -> Result<(), AppError> {
    let mut event = event;
    let exists: i64 = sqlx::query_scalar("select count(*) from events where event_id = ?1")
        .bind(event.event_id.as_str())
        .fetch_one(&state.pool)
        .await?;
    if exists > 0 {
        tracing::debug!(
            event_id = %event.event_id,
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
            .fetch_optional(&state.pool)
            .await?;
    if let Some(conflict) = seq_conflict {
        tracing::warn!(
            event_id = %event.event_id,
            event_kind = ?&event.kind,
            seq = event.seq,
            conflict_event_id = %conflict,
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
            .fetch_one(&state.pool)
            .await?;
    let expected_seq = max_seq.unwrap_or(0) + 1;
    let stream_gap = (event.seq > expected_seq).then_some(expected_seq);
    if let Some(expected_seq) = stream_gap {
        tracing::warn!(
            event_id = %event.event_id,
            event_kind = ?&event.kind,
            expected_seq,
            received_seq = event.seq,
            "event stream gap detected"
        );
    }
    if event.correlation_id.is_none() {
        event.correlation_id = command_correlation_id(state, event.command_id.as_ref()).await?;
    }
    upsert_actor(state, &event.actor_ref, event.happened_at).await?;

    insert_event_record(state, &scope_key, &event).await?;

    apply_event_projection(state, &event).await?;
    if let Some(expected_seq) = stream_gap {
        mark_event_stream_gap(state, &event, expected_seq).await?;
    }
    publish_event(state, &event);
    log_event_appended(&event, stream_gap);
    Ok(())
}

async fn insert_event_record(
    state: &AppState,
    scope_key: &str,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        insert into events (
            event_id, scope_key, seq, kind, node_id, runtime_session_id,
            session_thread_id, command_id, actor_ref_json, scope_ref_json,
            correlation_id, source_refs_json, evidence_refs_json, cause_refs_json,
            result_refs_json, payload_json, event_json, happened_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
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
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn command_correlation_id(
    state: &AppState,
    command_id: Option<&CommandId>,
) -> Result<Option<CorrelationId>, AppError> {
    let Some(command_id) = command_id else {
        return Ok(None);
    };
    let command_json: Option<String> =
        sqlx::query_scalar("select command_json from commands where command_id = ?1")
            .bind(command_id.as_str())
            .fetch_optional(&state.pool)
            .await?;
    command_json
        .map(|command_json| serde_json::from_str::<CommandEnvelope>(&command_json))
        .transpose()
        .map(|command| command.map(|command| command.correlation_id))
        .map_err(AppError::from)
}

async fn apply_event_projection(state: &AppState, event: &EventEnvelope) -> Result<(), AppError> {
    if let Some(runtime_session_id) = &event.runtime_session_id {
        touch_runtime_step(state, runtime_session_id, event.happened_at).await?;
    }
    update_turn_from_event(state, event).await?;
    update_approval_from_event(state, event).await?;

    match event.kind {
        EventKind::RuntimeStarting => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Starting)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::RuntimeReady => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Ready).await?;
                update_runtime_provider_resume_ref(state, runtime_session_id, event).await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::RuntimeResuming => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Resuming)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::RuntimeRunning => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Running)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::RuntimeBlocked => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Blocked)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::RuntimeExpired => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Expired)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Degraded).await?;
        }
        EventKind::RuntimeStopped => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Stopped)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Stopped).await?;
        }
        EventKind::RuntimeError => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Error).await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Degraded).await?;
            insert_event_message(
                state,
                event,
                MessageRole::Runtime,
                event
                    .payload
                    .0
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Runtime error"),
            )
            .await?;
        }
        EventKind::TurnInterrupted => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Interrupted)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
        }
        EventKind::ProviderMessageCompleted => {
            insert_event_message(
                state,
                event,
                MessageRole::Assistant,
                event
                    .payload
                    .0
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Provider completed a message"),
            )
            .await?;
        }
        EventKind::ApprovalRequested => {
            if let Some(runtime_session_id) = &event.runtime_session_id {
                update_runtime_state(state, runtime_session_id, RuntimeSessionState::Blocked)
                    .await?;
            }
            update_session_state_from_event(state, event, SessionThreadState::Active).await?;
            insert_event_message(
                state,
                event,
                MessageRole::Approval,
                event
                    .payload
                    .0
                    .get("prompt")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Approval requested"),
            )
            .await?;
        }
        EventKind::ApprovalResolved => {
            insert_event_message(
                state,
                event,
                MessageRole::Approval,
                event
                    .payload
                    .0
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Approval resolved"),
            )
            .await?;
        }
        EventKind::WorkspaceValidated | EventKind::ResourceSnapshotUpdated => {
            update_placement_from_workspace_event(state, event).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn update_placement_from_workspace_event(
    state: &AppState,
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

    sqlx::query(
        r#"
        update project_placements
        set display_name = coalesce(?1, display_name),
            workspace_path = coalesce(?2, workspace_path),
            state = ?3,
            resource_badges_json = ?4,
            last_validated_at = ?5,
            updated_at = ?5
        where project_placement_id = ?6
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
    .bind(event.happened_at)
    .bind(placement_id.as_str())
    .execute(&state.pool)
    .await?;

    Ok(())
}

async fn insert_event_message(
    state: &AppState,
    event: &EventEnvelope,
    role: MessageRole,
    content: &str,
) -> Result<(), AppError> {
    if let Some(session_thread_id) = &event.session_thread_id {
        let message = Message {
            message_id: MessageId::new(),
            session_thread_id: session_thread_id.clone(),
            turn_id: event.turn_id.clone(),
            role,
            content: content.to_owned(),
            created_at: event.happened_at,
            completed_at: Some(event.happened_at),
            source_event_id: Some(event.event_id.clone()),
        };
        insert_message(state, &message).await?;
    }
    Ok(())
}

async fn record_warning_acknowledgement(
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
        CortexRef::Warning {
            warning_kind: warning_kind.clone(),
            command_id: None,
        },
        CortexRef::Session {
            session_thread_id: detail.session.session_thread_id.clone(),
        },
        CortexRef::Placement {
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
        kind: EventKind::CoordinationWarningAcknowledged,
        happened_at,
        source_refs: vec![CortexRef::Warning {
            warning_kind: warning_kind.clone(),
            command_id: None,
        }],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: affected_refs.clone(),
        payload: JsonValue(json!({
            "warning_kind": warning_kind,
            "message": message,
            "affected_refs": affected_refs,
        })),
    };

    upsert_actor(state, &event.actor_ref, event.happened_at).await?;
    insert_event_record(state, &scope_key, &event).await?;

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
    .execute(&state.pool)
    .await?;

    publish_event(state, &event);
    log_event_appended(&event, None);
    Ok(event)
}

async fn create_enrollment(
    state: &AppState,
    display_name: &str,
    daemon_version: Option<&str>,
    capabilities: Vec<CapabilitySummary>,
) -> Result<NodeEnrollmentRequestedResponse, AppError> {
    if display_name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation.display_name_required",
            "Display name is required",
        ));
    }

    let now = Utc::now();
    let enrollment_id = EnrollmentId::new();
    let pairing_code = new_secret("pair");
    let expires_at = now + chrono::Duration::seconds(state.config.enrollment_ttl_seconds);
    let approved_at = state.config.auto_approve_enrollments.then_some(now);
    let status = if approved_at.is_some() {
        EnrollmentState::Approved
    } else {
        EnrollmentState::PendingUserApproval
    };
    sqlx::query(
        r#"
        insert into node_enrollments (
            enrollment_id, display_name, daemon_version, capabilities_json,
            pairing_code_hash, status, expires_at, claimed_node_id,
            created_at, updated_at, approved_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, null, ?8, ?8, ?9)
        "#,
    )
    .bind(enrollment_id.as_str())
    .bind(display_name.trim())
    .bind(daemon_version)
    .bind(serde_json::to_string(&capabilities)?)
    .bind(hash_secret(&pairing_code))
    .bind(format_enrollment_state(&status))
    .bind(expires_at)
    .bind(now)
    .bind(approved_at)
    .execute(&state.pool)
    .await?;
    tracing::info!(
        enrollment_id = %enrollment_id,
        display_name = %display_name.trim(),
        daemon_version = daemon_version.unwrap_or("client"),
        capabilities = capabilities.len(),
        auto_approved = approved_at.is_some(),
        expires_at = %expires_at,
        "node enrollment created"
    );

    Ok(NodeEnrollmentRequestedResponse {
        enrollment_id,
        pairing_code,
        status,
        expires_at,
    })
}

async fn claim_enrollment(
    state: &AppState,
    request: &NodeEnrollmentClaimRequest,
) -> Result<NodeEnrollmentClaimResponse, AppError> {
    let row = sqlx::query(
        r#"
        select enrollment_id, display_name, daemon_version, capabilities_json,
               pairing_code_hash, status, expires_at, claimed_node_id,
               approved_at, created_at
        from node_enrollments
        where enrollment_id = ?1
        "#,
    )
    .bind(request.enrollment_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("node_enrollment.not_found", "Enrollment not found"))?;

    let now = Utc::now();
    let status = parse_enrollment_state(row.try_get::<String, _>("status")?.as_str());
    let expires_at: DateTime<Utc> = row.try_get("expires_at")?;
    if expires_at <= now {
        expire_enrollment(state, &request.enrollment_id, now).await?;
        tracing::warn!(
            enrollment_id = %request.enrollment_id,
            "node enrollment claim rejected because enrollment expired"
        );
        return Ok(NodeEnrollmentClaimResponse {
            accepted: false,
            pending: false,
            node_id: None,
            credential: None,
            message: "Enrollment expired".to_owned(),
        });
    }
    let stored_pairing_hash: String = row.try_get("pairing_code_hash")?;
    if stored_pairing_hash != hash_secret(&request.pairing_code) {
        tracing::warn!(
            enrollment_id = %request.enrollment_id,
            "node enrollment claim rejected because pairing code was invalid"
        );
        return Err(AppError::auth(
            "auth_dev.invalid_pairing_code",
            "Pairing code is invalid",
        ));
    }
    let approved_at: Option<DateTime<Utc>> = row.try_get("approved_at")?;
    if status == EnrollmentState::Registered {
        let claimed_node_id: Option<String> = row.try_get("claimed_node_id")?;
        tracing::info!(
            enrollment_id = %request.enrollment_id,
            claimed_node_id = claimed_node_id.as_deref().unwrap_or("none"),
            "node enrollment claim replayed after registration"
        );
        return Ok(NodeEnrollmentClaimResponse {
            accepted: true,
            pending: false,
            node_id: claimed_node_id.map(NodeId::from),
            credential: None,
            message: "Enrollment already claimed; existing credential is not returned".to_owned(),
        });
    }
    if matches!(
        status,
        EnrollmentState::Expired | EnrollmentState::Rejected | EnrollmentState::Revoked
    ) {
        tracing::warn!(
            enrollment_id = %request.enrollment_id,
            status = ?status,
            "node enrollment claim rejected because enrollment is terminal"
        );
        return Ok(NodeEnrollmentClaimResponse {
            accepted: false,
            pending: false,
            node_id: None,
            credential: None,
            message: format!("Enrollment is {}", format_enrollment_state(&status)),
        });
    }
    let approved = approved_at.is_some()
        || status == EnrollmentState::Approved
        || state.config.auto_approve_enrollments;
    if !approved {
        tracing::debug!(
            enrollment_id = %request.enrollment_id,
            "node enrollment claim waiting for user approval"
        );
        return Ok(NodeEnrollmentClaimResponse {
            accepted: false,
            pending: true,
            node_id: None,
            credential: None,
            message: "Enrollment is waiting for approval".to_owned(),
        });
    }

    let node_id = NodeId::new();
    let credential = new_secret("node");
    let credential_hash = hash_secret(&credential);
    let display_name: String = row.try_get("display_name")?;
    let daemon_version: Option<String> = row.try_get("daemon_version")?;
    let capabilities_json: String = row.try_get("capabilities_json")?;
    let capabilities = serde_json::from_str::<Vec<CapabilitySummary>>(&capabilities_json)?;

    sqlx::query(
        r#"
        insert into nodes (
            node_id, display_name, presence, sleep_hint, last_heartbeat_at,
            daemon_version, active_runtime_count, capabilities_json, diagnostics,
            credential_hash, created_at, updated_at
        )
        values (?1, ?2, 'offline', 'unknown', null, ?3, 0, ?4, 'enrolled; waiting for heartbeat', ?5, ?6, ?6)
        "#,
    )
    .bind(node_id.as_str())
    .bind(display_name)
    .bind(daemon_version.unwrap_or_else(|| "unknown".to_owned()))
    .bind(capabilities_json)
    .bind(credential_hash)
    .bind(now)
    .execute(&state.pool)
    .await?;

    replace_node_capabilities(state, &node_id, &capabilities, now).await?;

    sqlx::query(
        r#"
        update node_enrollments
        set status = 'registered', claimed_node_id = ?1, updated_at = ?2
        where enrollment_id = ?3
        "#,
    )
    .bind(node_id.as_str())
    .bind(now)
    .bind(request.enrollment_id.as_str())
    .execute(&state.pool)
    .await?;
    tracing::info!(
        enrollment_id = %request.enrollment_id,
        node_id = %node_id,
        "node enrollment claimed"
    );

    Ok(NodeEnrollmentClaimResponse {
        accepted: true,
        pending: false,
        node_id: Some(node_id),
        credential: Some(credential),
        message: "Enrollment claimed".to_owned(),
    })
}

async fn expire_enrollment(
    state: &AppState,
    enrollment_id: &EnrollmentId,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        update node_enrollments
        set status = 'expired', updated_at = ?1
        where enrollment_id = ?2 and status in ('pending_user_approval', 'approved')
        "#,
    )
    .bind(now)
    .bind(enrollment_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn load_enrollments(state: &AppState) -> Result<Vec<NodeEnrollmentSummary>, AppError> {
    let rows = sqlx::query(
        r#"
        select enrollment_id, display_name, status, claimed_node_id,
               expires_at, created_at, approved_at
        from node_enrollments
        order by created_at desc
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter().map(row_to_enrollment).collect()
}

async fn load_enrollment(
    state: &AppState,
    enrollment_id: &EnrollmentId,
) -> Result<NodeEnrollmentSummary, AppError> {
    let row = sqlx::query(
        r#"
        select enrollment_id, display_name, status, claimed_node_id,
               expires_at, created_at, approved_at
        from node_enrollments
        where enrollment_id = ?1
        "#,
    )
    .bind(enrollment_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("node_enrollment.not_found", "Enrollment not found"))?;
    row_to_enrollment(row)
}

fn row_to_enrollment(row: sqlx::sqlite::SqliteRow) -> Result<NodeEnrollmentSummary, AppError> {
    let claimed_node_id: Option<String> = row.try_get("claimed_node_id")?;
    let approved_at: Option<DateTime<Utc>> = row.try_get("approved_at")?;
    let mut status = parse_enrollment_state(row.try_get::<String, _>("status")?.as_str());
    if status == EnrollmentState::PendingUserApproval && approved_at.is_some() {
        status = EnrollmentState::Approved;
    }
    Ok(NodeEnrollmentSummary {
        enrollment_id: EnrollmentId::from(row.try_get::<String, _>("enrollment_id")?),
        display_name: row.try_get("display_name")?,
        status,
        claimed_node_id: claimed_node_id.map(NodeId::from),
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        approved_at,
    })
}

async fn verify_node_credential(
    state: &AppState,
    node_id: &NodeId,
    credential: Option<&str>,
) -> Result<(), AppError> {
    let credential = credential
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::auth(
                "auth_dev.credential_required",
                "Node credential is required",
            )
        })?;
    let row = sqlx::query("select presence, credential_hash from nodes where node_id = ?1")
        .bind(node_id.as_str())
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::auth("auth_dev.node_unknown", "Node is not enrolled"))?;
    if parse_presence(row.try_get::<String, _>("presence")?.as_str()) == NodePresence::Revoked {
        return Err(AppError::auth(
            "auth_dev.node_revoked",
            "Node has been revoked",
        ));
    }
    let Some(credential_hash) = row.try_get::<Option<String>, _>("credential_hash")? else {
        return Err(AppError::auth(
            "auth_dev.credential_missing",
            "Node credential is missing",
        ));
    };
    if credential_hash != hash_secret(credential) {
        return Err(AppError::auth(
            "auth_dev.credential_invalid",
            "Node credential is invalid",
        ));
    }
    Ok(())
}

async fn upsert_heartbeat_workspaces(
    state: &AppState,
    node_id: &NodeId,
    workspaces: Vec<WorkspaceSnapshot>,
) -> Result<(), AppError> {
    for workspace in workspaces {
        if workspace_binding_deleted(state, node_id, &workspace.workspace_path).await? {
            continue;
        }
        let placement_id = stable_placement_id(node_id, &workspace.workspace_path);
        let project_id = stable_project_id(node_id, &workspace.workspace_path);
        upsert_project(
            state,
            &project_id,
            &workspace.display_name,
            workspace.last_validated_at,
        )
        .await?;
        sqlx::query(
            r#"
            insert into project_placements (
                project_placement_id, project_id, node_id, display_name, workspace_path,
                state, resource_badges_json, last_validated_at, created_at, updated_at
            )
            values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8)
            on conflict(project_placement_id) do update set
                project_id = excluded.project_id,
                display_name = excluded.display_name,
                workspace_path = excluded.workspace_path,
                state = excluded.state,
                resource_badges_json = excluded.resource_badges_json,
                last_validated_at = excluded.last_validated_at,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(placement_id.as_str())
        .bind(project_id.as_str())
        .bind(node_id.as_str())
        .bind(workspace.display_name)
        .bind(workspace.workspace_path)
        .bind(format_placement_state(workspace.state))
        .bind(serde_json::to_string(&workspace.resource_badges)?)
        .bind(workspace.last_validated_at)
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

async fn workspace_binding_deleted(
    state: &AppState,
    node_id: &NodeId,
    workspace_path: &str,
) -> Result<bool, AppError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        select 1
        from deleted_workspace_bindings
        where node_id = ?1 and workspace_path = ?2
        "#,
    )
    .bind(node_id.as_str())
    .bind(workspace_path)
    .fetch_optional(&state.pool)
    .await
    .map(|row| row.is_some())
    .map_err(AppError::from)
}

async fn replace_node_capabilities(
    state: &AppState,
    node_id: &NodeId,
    capabilities: &[CapabilitySummary],
    updated_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query("delete from node_capabilities where node_id = ?1")
        .bind(node_id.as_str())
        .execute(&state.pool)
        .await?;
    for capability in capabilities {
        sqlx::query(
            r#"
            insert into node_capabilities (
                node_id, capability_key, value_json, updated_at
            )
            values (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(node_id.as_str())
        .bind(&capability.key)
        .bind(serde_json::to_string(&capability.value)?)
        .bind(updated_at)
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

async fn upsert_project(
    state: &AppState,
    project_id: &ProjectId,
    display_name: &str,
    updated_at: DateTime<Utc>,
) -> Result<(), AppError> {
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
    .bind(display_name)
    .bind(updated_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn effective_node_presence(
    state: &AppState,
    node_id: &NodeId,
) -> Result<NodePresence, AppError> {
    let row = sqlx::query("select presence, last_heartbeat_at from nodes where node_id = ?1")
        .bind(node_id.as_str())
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("node.not_found", "Node not found"))?;
    let last_heartbeat_at: Option<DateTime<Utc>> = row.try_get("last_heartbeat_at")?;
    let heartbeat_age_seconds = last_heartbeat_at.map(|timestamp| {
        Utc::now()
            .signed_duration_since(timestamp)
            .num_seconds()
            .max(0)
    });
    Ok(derive_presence(
        parse_presence(row.try_get::<String, _>("presence")?.as_str()),
        heartbeat_age_seconds,
        state.config.stale_after_seconds,
        state.config.offline_after_seconds,
    ))
}

async fn ensure_node_commandable(state: &AppState, node_id: &NodeId) -> Result<(), AppError> {
    match effective_node_presence(state, node_id).await? {
        NodePresence::Reachable | NodePresence::Stale => Ok(()),
        NodePresence::Offline => Err(AppError::bad_request(
            "node.offline",
            "Node is offline and cannot accept commands",
        )),
        NodePresence::Revoked => Err(AppError::bad_request(
            "node.revoked",
            "Node has been revoked and cannot accept commands",
        )),
    }
}

async fn ensure_session_commandable(
    state: &AppState,
    detail: &SessionDetail,
    command_kind: CommandKind,
) -> Result<(), AppError> {
    if command_requires_attached_session(command_kind)
        && detail.session.state == SessionThreadState::Detached
    {
        return Err(AppError::bad_request(
            "session.detached",
            "Session is detached; attach before sending interactive commands",
        ));
    }
    ensure_runtime_accepts_command(&detail.session.runtime, command_kind)?;
    ensure_node_commandable(state, &detail.placement.node_id).await?;
    if command_requires_startable_placement(command_kind) {
        ensure_placement_startable(&detail.placement)?;
    }
    if command_requires_provider_capability(command_kind) {
        ensure_node_supports_provider(
            state,
            &detail.placement.node_id,
            &detail.session.runtime.provider,
        )
        .await?;
    }
    Ok(())
}

fn command_requires_startable_placement(command_kind: CommandKind) -> bool {
    matches!(
        command_kind,
        CommandKind::StartRuntime | CommandKind::SendTurn | CommandKind::ResumeRuntime
    )
}

fn ensure_runtime_accepts_command(
    runtime: &RuntimeSummary,
    command_kind: CommandKind,
) -> Result<(), AppError> {
    let accepts = match command_kind {
        CommandKind::SendTurn => matches!(
            runtime.state,
            RuntimeSessionState::Ready | RuntimeSessionState::Running
        ),
        CommandKind::ResolveApproval => runtime.state == RuntimeSessionState::Blocked,
        CommandKind::InterruptRuntime => matches!(
            runtime.state,
            RuntimeSessionState::Running | RuntimeSessionState::Blocked
        ),
        CommandKind::StopRuntime => !matches!(
            runtime.state,
            RuntimeSessionState::Stopped | RuntimeSessionState::Expired
        ),
        CommandKind::ResumeRuntime => matches!(
            runtime.state,
            RuntimeSessionState::Stopped
                | RuntimeSessionState::Expired
                | RuntimeSessionState::Stale
                | RuntimeSessionState::Error
                | RuntimeSessionState::Interrupted
        ),
        CommandKind::StartRuntime
        | CommandKind::ValidateWorkspace
        | CommandKind::RefreshResourceSnapshot => true,
    };
    if accepts {
        return Ok(());
    }
    Err(AppError::bad_request(
        "runtime.command_not_allowed",
        format!(
            "Runtime state `{}` cannot accept `{command_kind:?}`",
            format_runtime_state(runtime.state)
        ),
    ))
}

fn ensure_pending_approval(
    detail: &SessionDetail,
    approval_id: &ApprovalId,
) -> Result<(), AppError> {
    if pending_approvals(&detail.events)
        .iter()
        .any(|pending| pending == approval_id)
    {
        return Ok(());
    }
    Err(AppError::bad_request(
        "approval.not_pending",
        "Approval is not pending for this session",
    ))
}

fn command_requires_attached_session(command_kind: CommandKind) -> bool {
    matches!(
        command_kind,
        CommandKind::SendTurn | CommandKind::ResolveApproval
    )
}

fn command_requires_provider_capability(command_kind: CommandKind) -> bool {
    matches!(
        command_kind,
        CommandKind::StartRuntime
            | CommandKind::ResumeRuntime
            | CommandKind::SendTurn
            | CommandKind::ResolveApproval
    )
}

fn ensure_placement_startable(placement: &ProjectPlacementSummary) -> Result<(), AppError> {
    if placement.state != PlacementState::Validated {
        return Err(AppError::bad_request(
            "placement.not_startable",
            "Workspace placement is not startable",
        ));
    }
    if placement_has_hard_block(placement) {
        return Err(AppError::bad_request(
            "placement.hard_blocked",
            "Workspace placement has a hard-blocking resource badge",
        ));
    }
    Ok(())
}

fn placement_has_hard_block(placement: &ProjectPlacementSummary) -> bool {
    placement
        .resource_badges
        .iter()
        .any(|badge| badge.severity == WarningSeverity::HardBlock)
}

async fn ensure_node_supports_provider(
    state: &AppState,
    node_id: &NodeId,
    provider: &str,
) -> Result<(), AppError> {
    if node_supports_provider(state, node_id, provider).await? {
        return Ok(());
    }
    Err(AppError::bad_request(
        "node.capability_missing",
        format!("Node does not advertise provider capability `{provider}`"),
    ))
}

async fn node_supports_provider(
    state: &AppState,
    node_id: &NodeId,
    provider: &str,
) -> Result<bool, AppError> {
    let provider_key = format!("provider.{provider}");
    let capability_json: Option<String> = sqlx::query_scalar(
        "select value_json from node_capabilities where node_id = ?1 and capability_key = ?2",
    )
    .bind(node_id.as_str())
    .bind(&provider_key)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(capability_json) = capability_json {
        let value = serde_json::from_str::<JsonValue>(&capability_json)?;
        return Ok(capability_is_available(&CapabilitySummary {
            key: provider_key,
            value,
        }));
    }

    let capabilities_json: String =
        sqlx::query_scalar("select capabilities_json from nodes where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::not_found("node.not_found", "Node not found"))?;
    let capabilities = serde_json::from_str::<Vec<CapabilitySummary>>(&capabilities_json)?;
    Ok(capabilities
        .iter()
        .any(|capability| capability.key == provider_key && capability_is_available(capability)))
}

fn capability_is_available(capability: &CapabilitySummary) -> bool {
    capability
        .value
        .0
        .get("available")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

async fn load_inventory(state: &AppState) -> Result<InventorySnapshot, AppError> {
    expire_idle_runtimes(state).await?;
    Ok(InventorySnapshot {
        nodes: load_nodes(state).await?,
        placements: load_placements(state).await?,
        sessions: load_sessions(state).await?,
        generated_at: Utc::now(),
    })
}

async fn load_nodes(state: &AppState) -> Result<Vec<cortex_protocol::NodeSummary>, AppError> {
    let rows = sqlx::query(
        r#"
        select node_id, display_name, presence, sleep_hint, last_heartbeat_at,
               active_runtime_count, capabilities_json, diagnostics
        from nodes
        order by updated_at desc
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    let now = Utc::now();

    rows.into_iter()
        .map(|row| {
            let last_heartbeat_at: Option<DateTime<Utc>> = row.try_get("last_heartbeat_at")?;
            let heartbeat_age_seconds = last_heartbeat_at
                .map(|timestamp| now.signed_duration_since(timestamp).num_seconds().max(0));
            let presence = derive_presence(
                parse_presence(row.try_get::<String, _>("presence")?.as_str()),
                heartbeat_age_seconds,
                state.config.stale_after_seconds,
                state.config.offline_after_seconds,
            );
            let capabilities_json: String = row.try_get("capabilities_json")?;
            let capabilities = serde_json::from_str::<Vec<CapabilitySummary>>(&capabilities_json)?;
            Ok(cortex_protocol::NodeSummary {
                node_id: NodeId::from(row.try_get::<String, _>("node_id")?),
                display_name: row.try_get("display_name")?,
                presence,
                sleep_hint: parse_sleep_hint(row.try_get::<String, _>("sleep_hint")?.as_str()),
                heartbeat_age_seconds,
                active_runtime_count: row.try_get("active_runtime_count")?,
                capabilities,
                diagnostics: row.try_get("diagnostics")?,
            })
        })
        .collect()
}

async fn load_placements(state: &AppState) -> Result<Vec<ProjectPlacementSummary>, AppError> {
    expire_idle_runtimes(state).await?;
    let rows = sqlx::query(
        r#"
        select project_placement_id, project_id, node_id, display_name, workspace_path,
               state, resource_badges_json, last_validated_at
        from project_placements
        order by updated_at desc
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    let mut placements = Vec::with_capacity(rows.len());
    for row in rows {
        let placement = row_to_placement(row)?;
        placements.push(add_core_resource_badges(state, placement, None).await?);
    }
    Ok(placements)
}

async fn load_placement(
    state: &AppState,
    placement_id: &ProjectPlacementId,
) -> Result<ProjectPlacementSummary, AppError> {
    load_placement_with_excluded_session(state, placement_id, None).await
}

async fn load_placement_for_session(
    state: &AppState,
    placement_id: &ProjectPlacementId,
    session_id: &SessionThreadId,
) -> Result<ProjectPlacementSummary, AppError> {
    load_placement_with_excluded_session(state, placement_id, Some(session_id)).await
}

async fn load_placement_with_excluded_session(
    state: &AppState,
    placement_id: &ProjectPlacementId,
    excluded_session_id: Option<&SessionThreadId>,
) -> Result<ProjectPlacementSummary, AppError> {
    expire_idle_runtimes(state).await?;
    let row = sqlx::query(
        r#"
        select project_placement_id, project_id, node_id, display_name, workspace_path,
               state, resource_badges_json, last_validated_at
        from project_placements
        where project_placement_id = ?1
        "#,
    )
    .bind(placement_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("placement.not_found", "Placement not found"))?;
    let placement = row_to_placement(row)?;
    add_core_resource_badges(state, placement, excluded_session_id).await
}

fn row_to_placement(row: sqlx::sqlite::SqliteRow) -> Result<ProjectPlacementSummary, AppError> {
    let badges_json: String = row.try_get("resource_badges_json")?;
    let project_id: Option<String> = row.try_get("project_id")?;
    Ok(ProjectPlacementSummary {
        project_placement_id: ProjectPlacementId::from(
            row.try_get::<String, _>("project_placement_id")?,
        ),
        project_id: project_id.map(ProjectId::from),
        node_id: NodeId::from(row.try_get::<String, _>("node_id")?),
        display_name: row.try_get("display_name")?,
        workspace_path: row.try_get("workspace_path")?,
        state: parse_placement_state(row.try_get::<String, _>("state")?.as_str()),
        resource_badges: serde_json::from_str(&badges_json)?,
        last_validated_at: row.try_get("last_validated_at")?,
    })
}

async fn add_core_resource_badges(
    state: &AppState,
    mut placement: ProjectPlacementSummary,
    excluded_session_id: Option<&SessionThreadId>,
) -> Result<ProjectPlacementSummary, AppError> {
    placement
        .resource_badges
        .retain(|badge| badge.kind != "same_workspace_active");
    let active_count =
        active_workspace_session_count(state, &placement, excluded_session_id).await?;
    if active_count > 0 {
        placement.resource_badges.push(ResourceBadge {
            kind: "same_workspace_active".to_owned(),
            severity: WarningSeverity::Warning,
            label: format!("Workspace already has {active_count} active session(s)"),
        });
    }
    Ok(placement)
}

async fn active_workspace_session_count(
    state: &AppState,
    placement: &ProjectPlacementSummary,
    excluded_session_id: Option<&SessionThreadId>,
) -> Result<i64, AppError> {
    sqlx::query_scalar(
        r#"
        select count(*)
        from session_threads st
        join project_placements pp on pp.project_placement_id = st.project_placement_id
        join runtime_sessions rs on rs.runtime_session_id = st.runtime_session_id
        where pp.node_id = ?1
          and pp.workspace_path = ?2
          and (?3 is null or st.session_thread_id != ?3)
          and st.state in ('active', 'detached', 'degraded')
          and rs.state in (
              'starting', 'ready', 'running', 'blocked',
              'stopping', 'interrupted', 'resuming', 'stale'
          )
        "#,
    )
    .bind(placement.node_id.as_str())
    .bind(&placement.workspace_path)
    .bind(excluded_session_id.map(SessionThreadId::as_str))
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::from)
}

async fn expire_idle_runtimes(state: &AppState) -> Result<(), AppError> {
    if state.config.runtime_expiry_seconds <= 0 {
        return Ok(());
    }
    let now = Utc::now();
    let cutoff = now - chrono::Duration::seconds(state.config.runtime_expiry_seconds);
    let rows = sqlx::query(
        r#"
        select rs.runtime_session_id, rs.session_thread_id, pp.node_id
        from runtime_sessions rs
        join session_threads st on st.session_thread_id = rs.session_thread_id
        join project_placements pp on pp.project_placement_id = st.project_placement_id
        where rs.state in ('ready', 'running', 'blocked', 'stale')
          and coalesce(rs.last_runtime_step_at, rs.created_at) <= ?1
        "#,
    )
    .bind(cutoff)
    .fetch_all(&state.pool)
    .await?;

    for row in rows {
        let runtime_session_id =
            RuntimeSessionId::from(row.try_get::<String, _>("runtime_session_id")?);
        let session_thread_id =
            SessionThreadId::from(row.try_get::<String, _>("session_thread_id")?);
        let node_id = NodeId::from(row.try_get::<String, _>("node_id")?);
        let seq = next_seq(
            state,
            &scope_key(&ScopeRef::Runtime {
                runtime_session_id: runtime_session_id.clone(),
            }),
        )
        .await?;
        accept_node_event(
            state,
            EventEnvelope {
                event_id: EventId::new(),
                command_id: None,
                correlation_id: None,
                actor_ref: ActorRef::System,
                scope_ref: ScopeRef::Runtime {
                    runtime_session_id: runtime_session_id.clone(),
                },
                node_id: Some(node_id),
                runtime_session_id: Some(runtime_session_id.clone()),
                session_thread_id: Some(session_thread_id),
                turn_id: None,
                seq,
                kind: EventKind::RuntimeExpired,
                happened_at: now,
                source_refs: vec![CortexRef::Runtime { runtime_session_id }],
                evidence_refs: vec![],
                cause_refs: vec![],
                result_refs: vec![],
                payload: JsonValue(json!({
                    "code": "runtime.idle_expired",
                    "message": format!(
                        "Runtime expired after {} seconds without runtime activity",
                        state.config.runtime_expiry_seconds
                    ),
                    "expiry_seconds": state.config.runtime_expiry_seconds,
                })),
            },
        )
        .await?;
    }
    Ok(())
}

async fn load_sessions(state: &AppState) -> Result<Vec<SessionSummary>, AppError> {
    expire_idle_runtimes(state).await?;
    let rows = sqlx::query(
        r#"
        select st.session_thread_id, st.project_placement_id, st.runtime_session_id, st.title,
               st.state as session_state, st.updated_at, rs.provider, rs.state as runtime_state,
               rs.resume_supported, rs.degraded_reason, rs.last_runtime_step_at,
               (select count(*) from messages m where m.session_thread_id = st.session_thread_id) as message_count
        from session_threads st
        join runtime_sessions rs on rs.runtime_session_id = st.runtime_session_id
        order by st.updated_at desc
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter().map(row_to_session).collect()
}

fn row_to_session(row: sqlx::sqlite::SqliteRow) -> Result<SessionSummary, AppError> {
    let runtime_session_id =
        RuntimeSessionId::from(row.try_get::<String, _>("runtime_session_id")?);
    Ok(SessionSummary {
        session_thread_id: SessionThreadId::from(row.try_get::<String, _>("session_thread_id")?),
        project_placement_id: ProjectPlacementId::from(
            row.try_get::<String, _>("project_placement_id")?,
        ),
        runtime_session_id: runtime_session_id.clone(),
        title: row.try_get("title")?,
        state: parse_session_state(row.try_get::<String, _>("session_state")?.as_str()),
        runtime: RuntimeSummary {
            runtime_session_id,
            provider: row.try_get("provider")?,
            state: parse_runtime_state(row.try_get::<String, _>("runtime_state")?.as_str()),
            resume_supported: row.try_get::<i64, _>("resume_supported")? != 0,
            degraded_reason: row.try_get("degraded_reason")?,
            last_runtime_step_at: row.try_get("last_runtime_step_at")?,
        },
        message_count: row.try_get("message_count")?,
        updated_at: row.try_get("updated_at")?,
    })
}

async fn load_session_detail(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<SessionDetail, AppError> {
    let session = load_sessions(state)
        .await?
        .into_iter()
        .find(|candidate| candidate.session_thread_id == *session_id)
        .ok_or_else(|| AppError::not_found("session.not_found", "Session not found"))?;
    let placement =
        load_placement_for_session(state, &session.project_placement_id, session_id).await?;
    let messages = load_messages(state, session_id).await?;
    let events = load_events(state, session_id, 0).await?;
    Ok(SessionDetail {
        session,
        placement,
        messages,
        events,
    })
}

async fn load_messages(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<Vec<Message>, AppError> {
    let rows = sqlx::query(
        r#"
        select message_id, session_thread_id, turn_id, role, content,
               created_at, completed_at, source_event_id
        from messages
        where session_thread_id = ?1
        order by created_at asc
        "#,
    )
    .bind(session_id.as_str())
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let turn_id: Option<String> = row.try_get("turn_id")?;
            let source_event_id: Option<String> = row.try_get("source_event_id")?;
            Ok(Message {
                message_id: MessageId::from(row.try_get::<String, _>("message_id")?),
                session_thread_id: SessionThreadId::from(
                    row.try_get::<String, _>("session_thread_id")?,
                ),
                turn_id: turn_id.map(TurnId::from),
                role: parse_message_role(row.try_get::<String, _>("role")?.as_str()),
                content: row.try_get("content")?,
                created_at: row.try_get("created_at")?,
                completed_at: row.try_get("completed_at")?,
                source_event_id: source_event_id.map(EventId::from),
            })
        })
        .collect()
}

async fn load_events(
    state: &AppState,
    session_id: &SessionThreadId,
    after_seq: i64,
) -> Result<Vec<EventEnvelope>, AppError> {
    let rows = sqlx::query(
        r#"
        select event_json
        from events
        where session_thread_id = ?1 and seq > ?2
        order by seq asc
        "#,
    )
    .bind(session_id.as_str())
    .bind(after_seq)
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let event_json: String = row.try_get("event_json")?;
            Ok(serde_json::from_str::<EventEnvelope>(&event_json)?)
        })
        .collect()
}

async fn build_artifact_tree(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<ArtifactTree, AppError> {
    let detail = load_session_detail(state, session_id).await?;
    let mut children = Vec::with_capacity(detail.messages.len() + detail.events.len());
    for message in &detail.messages {
        children.push(ArtifactTreeNode {
            artifact_id: ArtifactId::new(),
            label: format!("{:?} message", message.role),
            primary_ref: CortexRef::Message {
                message_id: message.message_id.clone(),
            },
            source_refs: message_source_refs(message, &detail.events),
            evidence_refs: vec![],
            cause_refs: vec![],
            children: vec![],
        });
    }
    for event in &detail.events {
        children.push(ArtifactTreeNode {
            artifact_id: ArtifactId::new(),
            label: artifact_label_for_event(event),
            primary_ref: primary_ref_for_event(event),
            source_refs: event.source_refs.clone(),
            evidence_refs: event.evidence_refs.clone(),
            cause_refs: event.cause_refs.clone(),
            children: vec![],
        });
    }

    Ok(ArtifactTree {
        session_thread_id: session_id.clone(),
        root: ArtifactTreeNode {
            artifact_id: ArtifactId::new(),
            label: detail.session.title,
            primary_ref: CortexRef::Session {
                session_thread_id: session_id.clone(),
            },
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            children,
        },
        generated_at: Utc::now(),
    })
}

async fn build_agent_projection(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<AgentProjection, AppError> {
    let detail = load_session_detail(state, session_id).await?;
    let node_presence = effective_node_presence(state, &detail.placement.node_id).await?;
    let provider_available = node_supports_provider(
        state,
        &detail.placement.node_id,
        &detail.session.runtime.provider,
    )
    .await?;
    let current_turn = current_turn(&detail.events);
    let pending_approvals = pending_approvals(&detail.events);
    let acknowledged = acknowledged_warning_kinds(state, session_id).await?;
    let mut active_warnings =
        active_warnings(&detail.placement, &detail.session.runtime, &acknowledged);
    if let Some(warning) = node_presence_warning(node_presence) {
        if !acknowledged.contains(&warning.kind) {
            active_warnings.push(warning);
        }
    }
    if !provider_available && !acknowledged.contains("provider_unavailable") {
        active_warnings.push(ResourceBadge {
            kind: "provider_unavailable".to_owned(),
            severity: WarningSeverity::HardBlock,
            label: format!(
                "Provider `{}` is not advertised by this node",
                detail.session.runtime.provider
            ),
        });
    }
    let recent_turn_summaries = recent_turn_summaries(&detail.messages);
    let recent_message_refs = detail
        .messages
        .iter()
        .rev()
        .take(5)
        .map(|message| CortexRef::Message {
            message_id: message.message_id.clone(),
        })
        .collect::<Vec<_>>();
    let visible_refs = visible_refs(
        &detail,
        &pending_approvals,
        &active_warnings,
        &recent_message_refs,
    );
    let source_cause_summary = source_cause_summary(&detail.events);
    let resume_context = resume_context(
        &detail,
        current_turn.as_ref(),
        &pending_approvals,
        &recent_turn_summaries,
    );
    let artifact_tree_summary = format!(
        "Session-local index: {} messages, {} events, {} pending approvals",
        detail.messages.len(),
        detail.events.len(),
        pending_approvals.len()
    );
    let available_commands = available_commands(
        detail.session.state,
        detail.session.runtime.state,
        !pending_approvals.is_empty(),
        !active_warnings.is_empty(),
        node_accepts_commands(node_presence),
        placement_has_hard_block(&detail.placement),
        provider_available,
    );
    Ok(AgentProjection {
        session_thread_id: session_id.clone(),
        project_placement: detail.placement,
        runtime_summary: detail.session.runtime,
        current_turn,
        pending_approvals,
        active_warnings,
        recent_turn_summaries,
        recent_message_refs,
        artifact_tree_summary,
        available_block_types: vec![
            "core.user-message".to_owned(),
            "core.assistant-message".to_owned(),
            "core.provider-output-stream".to_owned(),
            "core.approval-request".to_owned(),
            "core.runtime-event".to_owned(),
            "core.workspace-validation".to_owned(),
            "core.resource-snapshot".to_owned(),
            "core.warning".to_owned(),
            "core.error".to_owned(),
            "core.agent-projection-summary".to_owned(),
            "core.unknown".to_owned(),
        ],
        available_commands,
        visible_refs,
        source_cause_summary,
        resume_context,
        generated_at: Utc::now(),
    })
}

fn message_source_refs(message: &Message, events: &[EventEnvelope]) -> Vec<CortexRef> {
    let Some(source_event_id) = &message.source_event_id else {
        return vec![];
    };
    events
        .iter()
        .find(|event| event.event_id == *source_event_id)
        .map(|event| {
            vec![CortexRef::Event {
                event_id: event.event_id.clone(),
                scope_ref: Box::new(event.scope_ref.clone()),
                seq: event.seq,
            }]
        })
        .unwrap_or_else(|| {
            vec![CortexRef::Event {
                event_id: source_event_id.clone(),
                scope_ref: Box::new(ScopeRef::Session {
                    session_thread_id: message.session_thread_id.clone(),
                }),
                seq: 0,
            }]
        })
}

fn artifact_label_for_event(event: &EventEnvelope) -> String {
    match event.kind {
        EventKind::ApprovalRequested => {
            let prompt = event
                .payload
                .0
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("approval requested");
            format!("Approval requested: {}", snippet(prompt, 80))
        }
        EventKind::ApprovalResolved => "Approval resolved".to_owned(),
        EventKind::RuntimeError => {
            let message = event
                .payload
                .0
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("runtime error");
            format!("Runtime error: {}", snippet(message, 80))
        }
        _ => format!("{:?} #{}", event.kind, event.seq),
    }
}

fn primary_ref_for_event(event: &EventEnvelope) -> CortexRef {
    if matches!(
        event.kind,
        EventKind::ApprovalRequested | EventKind::ApprovalResolved
    ) {
        if let Some(approval_id) = event_approval_id(event) {
            return CortexRef::Approval { approval_id };
        }
    }
    CortexRef::Event {
        event_id: event.event_id.clone(),
        scope_ref: Box::new(event.scope_ref.clone()),
        seq: event.seq,
    }
}

fn current_turn(events: &[EventEnvelope]) -> Option<TurnId> {
    let mut active_turns = Vec::<TurnId>::new();
    for event in events {
        match event.kind {
            EventKind::TurnStarted => {
                if let Some(turn_id) = &event.turn_id {
                    active_turns.retain(|candidate| candidate != turn_id);
                    active_turns.push(turn_id.clone());
                }
            }
            EventKind::TurnCompleted | EventKind::TurnInterrupted | EventKind::RuntimeError => {
                if let Some(turn_id) = &event.turn_id {
                    active_turns.retain(|candidate| candidate != turn_id);
                }
            }
            _ => {}
        }
    }
    active_turns.last().cloned()
}

fn pending_approvals(events: &[EventEnvelope]) -> Vec<ApprovalId> {
    let mut pending = Vec::<ApprovalId>::new();
    for event in events {
        match event.kind {
            EventKind::ApprovalRequested => {
                if let Some(approval_id) = event_approval_id(event) {
                    pending.retain(|candidate| candidate != &approval_id);
                    pending.push(approval_id);
                }
            }
            EventKind::ApprovalResolved => {
                if let Some(approval_id) = event_approval_id(event) {
                    pending.retain(|candidate| candidate != &approval_id);
                }
            }
            _ => {}
        }
    }
    pending
}

fn event_approval_id(event: &EventEnvelope) -> Option<ApprovalId> {
    event
        .payload
        .0
        .get("approval_id")
        .and_then(serde_json::Value::as_str)
        .filter(|approval_id| !approval_id.is_empty())
        .map(ApprovalId::from)
}

fn active_warnings(
    placement: &ProjectPlacementSummary,
    runtime: &RuntimeSummary,
    acknowledged: &HashSet<String>,
) -> Vec<ResourceBadge> {
    let mut warnings = placement
        .resource_badges
        .iter()
        .filter(|badge| badge.severity != WarningSeverity::Info)
        .filter(|badge| !acknowledged.contains(&badge.kind))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(reason) = &runtime.degraded_reason {
        if !acknowledged.contains("runtime_degraded") {
            warnings.push(ResourceBadge {
                kind: "runtime_degraded".to_owned(),
                severity: WarningSeverity::Warning,
                label: reason.clone(),
            });
        }
    }
    warnings
}

fn node_presence_warning(presence: NodePresence) -> Option<ResourceBadge> {
    match presence {
        NodePresence::Reachable => None,
        NodePresence::Stale => Some(ResourceBadge {
            kind: "node_stale".to_owned(),
            severity: WarningSeverity::Warning,
            label: "Node heartbeat is stale".to_owned(),
        }),
        NodePresence::Offline => Some(ResourceBadge {
            kind: "node_offline".to_owned(),
            severity: WarningSeverity::HardBlock,
            label: "Node is offline".to_owned(),
        }),
        NodePresence::Revoked => Some(ResourceBadge {
            kind: "node_revoked".to_owned(),
            severity: WarningSeverity::HardBlock,
            label: "Node is revoked".to_owned(),
        }),
    }
}

fn node_accepts_commands(presence: NodePresence) -> bool {
    matches!(presence, NodePresence::Reachable | NodePresence::Stale)
}

async fn acknowledged_warning_kinds(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<HashSet<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        "select warning_kind from warning_acknowledgements where session_thread_id = ?1",
    )
    .bind(session_id.as_str())
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().collect())
}

fn recent_turn_summaries(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .rev()
        .filter(|message| {
            matches!(
                message.role,
                MessageRole::User
                    | MessageRole::Assistant
                    | MessageRole::Approval
                    | MessageRole::Runtime
            )
        })
        .take(6)
        .map(|message| format!("{:?}: {}", message.role, snippet(&message.content, 140)))
        .collect()
}

fn visible_refs(
    detail: &SessionDetail,
    pending_approvals: &[ApprovalId],
    active_warnings: &[ResourceBadge],
    recent_message_refs: &[CortexRef],
) -> Vec<CortexRef> {
    let mut refs = vec![
        CortexRef::Session {
            session_thread_id: detail.session.session_thread_id.clone(),
        },
        CortexRef::Runtime {
            runtime_session_id: detail.session.runtime.runtime_session_id.clone(),
        },
        CortexRef::Placement {
            placement_id: detail.placement.project_placement_id.clone(),
        },
    ];
    refs.extend(recent_message_refs.iter().cloned());
    refs.extend(
        detail
            .events
            .iter()
            .rev()
            .take(5)
            .map(|event| CortexRef::Event {
                event_id: event.event_id.clone(),
                scope_ref: Box::new(event.scope_ref.clone()),
                seq: event.seq,
            }),
    );
    refs.extend(
        pending_approvals
            .iter()
            .cloned()
            .map(|approval_id| CortexRef::Approval { approval_id }),
    );
    refs.extend(active_warnings.iter().map(|warning| CortexRef::Warning {
        warning_kind: warning.kind.clone(),
        command_id: None,
    }));
    dedupe_refs(refs)
}

fn dedupe_refs(refs: Vec<CortexRef>) -> Vec<CortexRef> {
    let mut seen = HashSet::<String>::new();
    refs.into_iter()
        .filter(|reference| seen.insert(ref_key(reference)))
        .collect()
}

fn ref_key(reference: &CortexRef) -> String {
    serde_json::to_string(reference).unwrap_or_else(|_| format!("{reference:?}"))
}

fn source_cause_summary(events: &[EventEnvelope]) -> String {
    let source_count = events
        .iter()
        .filter(|event| !event.source_refs.is_empty())
        .count();
    let evidence_count = events
        .iter()
        .filter(|event| !event.evidence_refs.is_empty())
        .count();
    let cause_count = events
        .iter()
        .filter(|event| !event.cause_refs.is_empty())
        .count();
    format!(
        "{} events; {source_count} with source refs, {evidence_count} with evidence refs, {cause_count} with cause refs. Missing causality remains explicit.",
        events.len()
    )
}

fn available_commands(
    session_state: SessionThreadState,
    runtime_state: RuntimeSessionState,
    has_pending_approvals: bool,
    has_active_warnings: bool,
    node_accepts_commands: bool,
    placement_has_hard_block: bool,
    provider_available: bool,
) -> Vec<String> {
    let mut commands = Vec::new();
    let session_is_attached = session_state != SessionThreadState::Detached;
    if session_state == SessionThreadState::Detached {
        commands.push("session.attach".to_owned());
    } else if session_state != SessionThreadState::Stopped {
        commands.push("session.detach".to_owned());
    }
    let can_start_or_continue_runtime =
        node_accepts_commands && !placement_has_hard_block && provider_available;
    if matches!(
        runtime_state,
        RuntimeSessionState::Ready | RuntimeSessionState::Running
    ) && can_start_or_continue_runtime
        && session_is_attached
    {
        commands.push("session.sendTurn".to_owned());
    }
    if matches!(
        runtime_state,
        RuntimeSessionState::Running | RuntimeSessionState::Blocked
    ) && node_accepts_commands
    {
        commands.push("runtime.interrupt".to_owned());
    }
    if !matches!(
        runtime_state,
        RuntimeSessionState::Stopped | RuntimeSessionState::Expired
    ) && node_accepts_commands
    {
        commands.push("runtime.stop".to_owned());
    }
    if matches!(
        runtime_state,
        RuntimeSessionState::Stopped
            | RuntimeSessionState::Expired
            | RuntimeSessionState::Stale
            | RuntimeSessionState::Error
            | RuntimeSessionState::Interrupted
    ) && can_start_or_continue_runtime
    {
        commands.push("runtime.resume".to_owned());
    }
    if has_pending_approvals && node_accepts_commands && provider_available && session_is_attached {
        commands.push("approval.resolve".to_owned());
    }
    if has_active_warnings {
        commands.push("warning.acknowledge".to_owned());
    }
    commands.push("reference.openInInspector".to_owned());
    commands.push("reference.copy".to_owned());
    commands
}

fn resume_context(
    detail: &SessionDetail,
    current_turn: Option<&TurnId>,
    pending_approvals: &[ApprovalId],
    recent_turn_summaries: &[String],
) -> String {
    let mut parts = vec![
        format!("runtime_state={:?}", detail.session.runtime.state),
        format!("provider={}", detail.session.runtime.provider),
    ];
    if let Some(turn_id) = current_turn {
        parts.push(format!("current_turn={turn_id}"));
    }
    if !pending_approvals.is_empty() {
        parts.push(format!(
            "pending_approvals={}",
            pending_approvals
                .iter()
                .map(ApprovalId::as_str)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if let Some(reason) = &detail.session.runtime.degraded_reason {
        parts.push(format!("degraded_reason={}", snippet(reason, 160)));
    }
    if !recent_turn_summaries.is_empty() {
        parts.push(format!(
            "recent={}",
            recent_turn_summaries
                .iter()
                .map(|summary| snippet(summary, 100))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    parts.join("; ")
}

fn snippet(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut result = normalized.chars().take(max_chars).collect::<String>();
    result.push_str("...");
    result
}

async fn upsert_actor(
    state: &AppState,
    actor_ref: &ActorRef,
    seen_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let (actor_key, actor_kind, display_name) = actor_identity(actor_ref);
    sqlx::query(
        r#"
        insert into actors (
            actor_key, actor_kind, display_name, actor_ref_json, first_seen_at, last_seen_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?5)
        on conflict(actor_key) do update set
            actor_ref_json = excluded.actor_ref_json,
            display_name = excluded.display_name,
            last_seen_at = excluded.last_seen_at
        "#,
    )
    .bind(actor_key)
    .bind(actor_kind)
    .bind(display_name)
    .bind(serde_json::to_string(actor_ref)?)
    .bind(seen_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn actor_identity(actor_ref: &ActorRef) -> (String, &'static str, String) {
    match actor_ref {
        ActorRef::LocalUser { actor_id } => {
            let actor_key = actor_id
                .as_ref()
                .map(|actor_id| format!("local_user:{actor_id}"))
                .unwrap_or_else(|| "local_user".to_owned());
            (actor_key, "local_user", "Local user".to_owned())
        }
        ActorRef::System => ("system".to_owned(), "system", "System".to_owned()),
        ActorRef::Node { node_id } => {
            (format!("node:{node_id}"), "node", format!("Node {node_id}"))
        }
        ActorRef::Provider { provider } => (
            format!("provider:{provider}"),
            "provider",
            format!("Provider {provider}"),
        ),
        ActorRef::Unknown => ("unknown".to_owned(), "unknown", "Unknown".to_owned()),
    }
}

async fn record_command(state: &AppState, command: CommandEnvelope) -> Result<(), AppError> {
    upsert_actor(state, &command.actor_ref, command.issued_at).await?;
    tracing::info!(
        command_id = %command.command_id,
        command_kind = ?command.kind,
        target_node_id = %command.target_node_id,
        session_thread_id = command
            .session_thread_id
            .as_ref()
            .map(SessionThreadId::as_str)
            .unwrap_or("none"),
        runtime_session_id = command
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str)
            .unwrap_or("none"),
        project_placement_id = command
            .project_placement_id
            .as_ref()
            .map(ProjectPlacementId::as_str)
            .unwrap_or("none"),
        correlation_id = %command.correlation_id,
        "command recorded"
    );
    sqlx::query(
        r#"
        insert into commands (
            command_id, kind, state, target_node_id, session_thread_id,
            runtime_session_id, project_placement_id, actor_ref_json, correlation_id,
            source_refs_json, cause_refs_json, payload_json, dedupe_key, command_json,
            created_at, completed_at
        )
        values (?1, ?2, 'recorded', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, null)
        "#,
    )
    .bind(command.command_id.as_str())
    .bind(format!("{:?}", command.kind))
    .bind(command.target_node_id.as_str())
    .bind(
        command
            .session_thread_id
            .as_ref()
            .map(SessionThreadId::as_str),
    )
    .bind(
        command
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str),
    )
    .bind(
        command
            .project_placement_id
            .as_ref()
            .map(ProjectPlacementId::as_str),
    )
    .bind(serde_json::to_string(&command.actor_ref)?)
    .bind(command.correlation_id.as_str())
    .bind(serde_json::to_string(&command.source_refs)?)
    .bind(serde_json::to_string(&command.cause_refs)?)
    .bind(serde_json::to_string(&command.payload)?)
    .bind(command.command_id.as_str())
    .bind(serde_json::to_string(&command)?)
    .bind(command.issued_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn insert_message(state: &AppState, message: &Message) -> Result<(), AppError> {
    sqlx::query(
        r#"
        insert into messages (
            message_id, session_thread_id, turn_id, role, content,
            created_at, completed_at, source_event_id
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(message.message_id.as_str())
    .bind(message.session_thread_id.as_str())
    .bind(message.turn_id.as_ref().map(TurnId::as_str))
    .bind(format_message_role(message.role.clone()))
    .bind(&message.content)
    .bind(message.created_at)
    .bind(message.completed_at)
    .bind(message.source_event_id.as_ref().map(EventId::as_str))
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn insert_turn(
    state: &AppState,
    turn_id: &TurnId,
    session_thread_id: &SessionThreadId,
    command_id: &CommandId,
    content: &str,
    created_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let turn_index: i64 = sqlx::query_scalar(
        "select coalesce(max(turn_index), 0) + 1 from turns where session_thread_id = ?1",
    )
    .bind(session_thread_id.as_str())
    .fetch_one(&state.pool)
    .await?;
    sqlx::query(
        r#"
        insert into turns (
            turn_id, session_thread_id, command_id, turn_index, state, content,
            blocked_approval_id, created_at, updated_at, completed_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, null, ?7, ?7, null)
        "#,
    )
    .bind(turn_id.as_str())
    .bind(session_thread_id.as_str())
    .bind(command_id.as_str())
    .bind(turn_index)
    .bind(format_turn_state(TurnState::Created))
    .bind(content)
    .bind(created_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn update_turn_from_event(state: &AppState, event: &EventEnvelope) -> Result<(), AppError> {
    let Some(turn_id) = &event.turn_id else {
        return Ok(());
    };
    let Some(turn_state) = turn_state_for_event(event) else {
        return Ok(());
    };
    let completed_at = matches!(
        turn_state,
        TurnState::Completed | TurnState::Interrupted | TurnState::Failed
    )
    .then_some(event.happened_at);
    let blocked_approval_id = if turn_state == TurnState::BlockedOnApproval {
        event_approval_id(event)
    } else {
        None
    };

    sqlx::query(
        r#"
        update turns
        set state = ?1,
            blocked_approval_id = ?2,
            completed_at = coalesce(?3, completed_at),
            updated_at = ?4
        where turn_id = ?5
        "#,
    )
    .bind(format_turn_state(turn_state))
    .bind(blocked_approval_id.as_ref().map(ApprovalId::as_str))
    .bind(completed_at)
    .bind(event.happened_at)
    .bind(turn_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn turn_state_for_event(event: &EventEnvelope) -> Option<TurnState> {
    match event.kind {
        EventKind::TurnStarted => Some(TurnState::Running),
        EventKind::TurnCompleted => Some(TurnState::Completed),
        EventKind::TurnInterrupted => Some(TurnState::Interrupted),
        EventKind::ApprovalRequested => Some(TurnState::BlockedOnApproval),
        EventKind::RuntimeError => Some(TurnState::Failed),
        _ => None,
    }
}

async fn update_approval_from_event(
    state: &AppState,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    match event.kind {
        EventKind::ApprovalRequested => record_approval_request(state, event).await,
        EventKind::ApprovalResolved => record_approval_resolution(state, event).await,
        _ => Ok(()),
    }
}

async fn record_approval_request(state: &AppState, event: &EventEnvelope) -> Result<(), AppError> {
    let Some(approval_id) = event_approval_id(event) else {
        return Ok(());
    };
    let Some(session_thread_id) = &event.session_thread_id else {
        return Ok(());
    };

    sqlx::query(
        r#"
        insert into approvals (
            approval_id, session_thread_id, runtime_session_id, turn_id, state,
            request_payload_json, response_payload_json, request_command_id,
            resolve_command_id, requested_event_id, resolved_event_id,
            created_at, updated_at, resolved_at
        )
        values (?1, ?2, ?3, ?4, ?5, ?6, null, ?7, null, ?8, null, ?9, ?9, null)
        on conflict(approval_id) do update set
            session_thread_id = excluded.session_thread_id,
            runtime_session_id = excluded.runtime_session_id,
            turn_id = excluded.turn_id,
            state = excluded.state,
            request_payload_json = excluded.request_payload_json,
            request_command_id = excluded.request_command_id,
            requested_event_id = excluded.requested_event_id,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(approval_id.as_str())
    .bind(session_thread_id.as_str())
    .bind(
        event
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str),
    )
    .bind(event.turn_id.as_ref().map(TurnId::as_str))
    .bind(format_approval_state(ApprovalState::Requested))
    .bind(serde_json::to_string(&event.payload)?)
    .bind(event.command_id.as_ref().map(CommandId::as_str))
    .bind(event.event_id.as_str())
    .bind(event.happened_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn record_approval_resolution(
    state: &AppState,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    let Some(approval_id) = event_approval_id(event) else {
        return Ok(());
    };
    let Some(session_thread_id) = &event.session_thread_id else {
        return Ok(());
    };

    sqlx::query(
        r#"
        insert into approvals (
            approval_id, session_thread_id, runtime_session_id, turn_id, state,
            request_payload_json, response_payload_json, request_command_id,
            resolve_command_id, requested_event_id, resolved_event_id,
            created_at, updated_at, resolved_at
        )
        values (?1, ?2, ?3, ?4, ?5, '{}', ?6, null, ?7, null, ?8, ?9, ?9, ?9)
        on conflict(approval_id) do update set
            session_thread_id = excluded.session_thread_id,
            runtime_session_id = excluded.runtime_session_id,
            turn_id = coalesce(excluded.turn_id, approvals.turn_id),
            state = excluded.state,
            response_payload_json = excluded.response_payload_json,
            resolve_command_id = excluded.resolve_command_id,
            resolved_event_id = excluded.resolved_event_id,
            resolved_at = excluded.resolved_at,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(approval_id.as_str())
    .bind(session_thread_id.as_str())
    .bind(
        event
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str),
    )
    .bind(event.turn_id.as_ref().map(TurnId::as_str))
    .bind(format_approval_state(ApprovalState::Resolved))
    .bind(serde_json::to_string(&event.payload)?)
    .bind(event.command_id.as_ref().map(CommandId::as_str))
    .bind(event.event_id.as_str())
    .bind(event.happened_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

#[cfg(test)]
struct NewEvent {
    command_id: Option<CommandId>,
    actor_ref: ActorRef,
    scope_ref: ScopeRef,
    node_id: Option<NodeId>,
    runtime_session_id: Option<RuntimeSessionId>,
    session_thread_id: Option<SessionThreadId>,
    turn_id: Option<TurnId>,
    kind: EventKind,
    payload: serde_json::Value,
}

#[cfg(test)]
async fn append_event(state: &AppState, new_event: NewEvent) -> Result<EventEnvelope, AppError> {
    let scope_key = scope_key(&new_event.scope_ref);
    let seq = next_seq(state, &scope_key).await?;
    let now = Utc::now();
    let event = EventEnvelope {
        event_id: EventId::new(),
        command_id: new_event.command_id,
        correlation_id: None,
        actor_ref: new_event.actor_ref,
        scope_ref: new_event.scope_ref,
        node_id: new_event.node_id,
        runtime_session_id: new_event.runtime_session_id,
        session_thread_id: new_event.session_thread_id,
        turn_id: new_event.turn_id,
        seq,
        kind: new_event.kind,
        happened_at: now,
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: JsonValue(new_event.payload),
    };
    insert_event_record(state, &scope_key, &event).await?;
    publish_event(state, &event);
    Ok(event)
}

fn publish_event(state: &AppState, event: &EventEnvelope) {
    let _ = state.event_tx.send(event.clone());
}

fn log_event_appended(event: &EventEnvelope, stream_gap_expected_seq: Option<i64>) {
    tracing::info!(
        event_id = %event.event_id,
        event_kind = ?&event.kind,
        seq = event.seq,
        node_id = event.node_id.as_ref().map(NodeId::as_str).unwrap_or("none"),
        runtime_session_id = event
            .runtime_session_id
            .as_ref()
            .map(RuntimeSessionId::as_str)
            .unwrap_or("none"),
        session_thread_id = event
            .session_thread_id
            .as_ref()
            .map(SessionThreadId::as_str)
            .unwrap_or("none"),
        command_id = event
            .command_id
            .as_ref()
            .map(CommandId::as_str)
            .unwrap_or("none"),
        correlation_id = event
            .correlation_id
            .as_ref()
            .map(CorrelationId::as_str)
            .unwrap_or("none"),
        stream_gap = stream_gap_expected_seq.is_some(),
        stream_gap_expected_seq = stream_gap_expected_seq.unwrap_or(0),
        "event appended"
    );
}

async fn next_seq(state: &AppState, scope_key: &str) -> Result<i64, AppError> {
    let max_seq: Option<i64> =
        sqlx::query_scalar("select max(seq) from events where scope_key = ?1")
            .bind(scope_key)
            .fetch_one(&state.pool)
            .await?;
    Ok(max_seq.unwrap_or(0) + 1)
}

async fn find_session_for_runtime(
    state: &AppState,
    runtime_session_id: &RuntimeSessionId,
) -> Result<SessionThreadId, AppError> {
    let session_id: Option<String> = sqlx::query_scalar(
        "select session_thread_id from runtime_sessions where runtime_session_id = ?1",
    )
    .bind(runtime_session_id.as_str())
    .fetch_optional(&state.pool)
    .await?;
    session_id
        .map(SessionThreadId::from)
        .ok_or_else(|| AppError::not_found("runtime.not_found", "Runtime not found"))
}

async fn update_runtime_state(
    state: &AppState,
    runtime_session_id: &RuntimeSessionId,
    runtime_state: RuntimeSessionState,
) -> Result<(), AppError> {
    let clears_degraded_reason = matches!(
        runtime_state,
        RuntimeSessionState::Starting
            | RuntimeSessionState::Ready
            | RuntimeSessionState::Running
            | RuntimeSessionState::Resuming
    );
    if clears_degraded_reason {
        sqlx::query(
            r#"
            update runtime_sessions
            set state = ?1, degraded_reason = null, updated_at = ?2
            where runtime_session_id = ?3
            "#,
        )
        .bind(format_runtime_state(runtime_state))
        .bind(Utc::now())
        .bind(runtime_session_id.as_str())
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query(
            r#"
            update runtime_sessions
            set state = ?1, updated_at = ?2
            where runtime_session_id = ?3
            "#,
        )
        .bind(format_runtime_state(runtime_state))
        .bind(Utc::now())
        .bind(runtime_session_id.as_str())
        .execute(&state.pool)
        .await?;
    }
    tracing::info!(
        runtime_session_id = %runtime_session_id,
        runtime_state = ?runtime_state,
        "runtime state updated"
    );
    Ok(())
}

async fn update_runtime_provider_resume_ref(
    state: &AppState,
    runtime_session_id: &RuntimeSessionId,
    event: &EventEnvelope,
) -> Result<(), AppError> {
    let Some(provider_resume_ref_json) = provider_resume_ref_json(event)? else {
        return Ok(());
    };
    sqlx::query(
        r#"
        update runtime_sessions
        set provider_resume_ref_json = ?1, updated_at = ?2
        where runtime_session_id = ?3
        "#,
    )
    .bind(provider_resume_ref_json)
    .bind(Utc::now())
    .bind(runtime_session_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn provider_resume_ref_json(event: &EventEnvelope) -> Result<Option<String>, AppError> {
    if let Some(provider_resume_ref) = event
        .payload
        .0
        .get("provider_resume_ref")
        .filter(|value| !value.is_null())
    {
        return serde_json::to_string(provider_resume_ref)
            .map(Some)
            .map_err(AppError::from);
    }

    let mut resume_ref = serde_json::Map::new();
    if let Some(provider_session_id) = event
        .payload
        .0
        .get("provider_session_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        resume_ref.insert(
            "provider_session_id".to_owned(),
            serde_json::Value::String(snippet(provider_session_id, 512)),
        );
    }
    if let Some(resume_cursor) = event
        .payload
        .0
        .get("resume_cursor")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        resume_ref.insert(
            "resume_cursor".to_owned(),
            serde_json::Value::String(snippet(resume_cursor, 512)),
        );
    }
    if resume_ref.is_empty() {
        Ok(None)
    } else {
        serde_json::to_string(&serde_json::Value::Object(resume_ref))
            .map(Some)
            .map_err(AppError::from)
    }
}

async fn touch_runtime_step(
    state: &AppState,
    runtime_session_id: &RuntimeSessionId,
    happened_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        update runtime_sessions
        set last_runtime_step_at = ?1, updated_at = ?2
        where runtime_session_id = ?3
        "#,
    )
    .bind(happened_at)
    .bind(Utc::now())
    .bind(runtime_session_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn update_session_state_from_event(
    state: &AppState,
    event: &EventEnvelope,
    session_state: SessionThreadState,
) -> Result<(), AppError> {
    if let Some(session_thread_id) = &event.session_thread_id {
        if session_state == SessionThreadState::Active
            && load_session_state(state, session_thread_id).await?
                == Some(SessionThreadState::Detached)
        {
            return Ok(());
        }
        sqlx::query(
            "update session_threads set state = ?1, updated_at = ?2 where session_thread_id = ?3",
        )
        .bind(format_session_state(session_state))
        .bind(Utc::now())
        .bind(session_thread_id.as_str())
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

async fn load_session_state(
    state: &AppState,
    session_id: &SessionThreadId,
) -> Result<Option<SessionThreadState>, AppError> {
    let state_value: Option<String> =
        sqlx::query_scalar("select state from session_threads where session_thread_id = ?1")
            .bind(session_id.as_str())
            .fetch_optional(&state.pool)
            .await?;
    Ok(state_value.map(|value| parse_session_state(&value)))
}

async fn update_session_attachment_state(
    state: &AppState,
    session_id: &SessionThreadId,
    target_state: SessionThreadState,
) -> Result<(), AppError> {
    let current_state: String =
        sqlx::query_scalar("select state from session_threads where session_thread_id = ?1")
            .bind(session_id.as_str())
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::not_found("session.not_found", "Session not found"))?;
    let current_state = parse_session_state(&current_state);
    if current_state == SessionThreadState::Stopped {
        return Err(AppError::bad_request(
            "session.stopped",
            "Stopped sessions cannot be attached or detached",
        ));
    }
    if current_state == target_state {
        return Ok(());
    }

    sqlx::query(
        "update session_threads set state = ?1, updated_at = ?2 where session_thread_id = ?3",
    )
    .bind(format_session_state(target_state))
    .bind(Utc::now())
    .bind(session_id.as_str())
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn mark_event_stream_gap(
    state: &AppState,
    event: &EventEnvelope,
    expected_seq: i64,
) -> Result<(), AppError> {
    let reason = format!(
        "event sequence gap: expected {expected_seq}, received {}",
        event.seq
    );
    if let Some(runtime_session_id) = &event.runtime_session_id {
        sqlx::query(
            r#"
            update runtime_sessions
            set state = ?1, degraded_reason = ?2, updated_at = ?3
            where runtime_session_id = ?4
            "#,
        )
        .bind(format_runtime_state(RuntimeSessionState::Stale))
        .bind(&reason)
        .bind(Utc::now())
        .bind(runtime_session_id.as_str())
        .execute(&state.pool)
        .await?;
    }
    if let Some(session_thread_id) = &event.session_thread_id {
        sqlx::query(
            "update session_threads set state = ?1, updated_at = ?2 where session_thread_id = ?3",
        )
        .bind(format_session_state(SessionThreadState::Degraded))
        .bind(Utc::now())
        .bind(session_thread_id.as_str())
        .execute(&state.pool)
        .await?;
    }
    tracing::warn!(
        event_id = %event.event_id,
        event_kind = ?&event.kind,
        expected_seq,
        received_seq = event.seq,
        "event stream marked degraded"
    );
    Ok(())
}

fn scope_key(scope_ref: &ScopeRef) -> String {
    match scope_ref {
        ScopeRef::Runtime { runtime_session_id } => format!("runtime:{}", runtime_session_id),
        ScopeRef::Session { session_thread_id } => format!("session:{}", session_thread_id),
        ScopeRef::Node { node_id } => format!("node:{}", node_id),
        ScopeRef::Placement {
            project_placement_id,
        } => {
            format!("placement:{project_placement_id}")
        }
        ScopeRef::Unknown { scope } => format!("unknown:{scope}"),
    }
}

#[cfg(test)]
fn compute_resource_badges(path: &str) -> Vec<ResourceBadge> {
    let mut badges = Vec::new();
    if path.contains("dirty") {
        badges.push(ResourceBadge {
            kind: "dirty_workspace".to_owned(),
            severity: WarningSeverity::Warning,
            label: "Dirty workspace".to_owned(),
        });
    }
    if path.contains("readonly") {
        badges.push(ResourceBadge {
            kind: "read_only_workspace".to_owned(),
            severity: WarningSeverity::HardBlock,
            label: "Read-only workspace".to_owned(),
        });
    }
    badges
}

#[cfg(test)]
fn workspace_snapshot_from_request(
    display_name: &str,
    workspace_path: &str,
    fallback_state: PlacementState,
) -> WorkspaceSnapshot {
    let path = std::path::Path::new(workspace_path);
    let (state, mut badges) = if !path.exists() {
        (
            PlacementState::Missing,
            vec![ResourceBadge {
                kind: "missing_workspace".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace missing".to_owned(),
            }],
        )
    } else if !path.is_dir() {
        (
            PlacementState::Missing,
            vec![ResourceBadge {
                kind: "workspace_not_directory".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace is not a directory".to_owned(),
            }],
        )
    } else if std::fs::metadata(path)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(false)
    {
        (
            PlacementState::ReadOnly,
            vec![ResourceBadge {
                kind: "read_only_workspace".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Read-only workspace".to_owned(),
            }],
        )
    } else {
        (
            PlacementState::Validated,
            compute_resource_badges(workspace_path),
        )
    };
    if state == PlacementState::Validated
        && badges.is_empty()
        && fallback_state != PlacementState::Pending
    {
        badges.push(ResourceBadge {
            kind: "validated".to_owned(),
            severity: WarningSeverity::Info,
            label: "Validated".to_owned(),
        });
    }
    WorkspaceSnapshot {
        display_name: display_name.to_owned(),
        workspace_path: workspace_path.to_owned(),
        state,
        resource_badges: badges,
        last_validated_at: Utc::now(),
    }
}

fn stable_placement_id(node_id: &NodeId, workspace_path: &str) -> ProjectPlacementId {
    let digest = Sha256::digest(format!("{}:{workspace_path}", node_id.as_str()).as_bytes());
    ProjectPlacementId::from(format!("placement-{}", hex_prefix(&digest, 16)))
}

fn stable_project_id(node_id: &NodeId, workspace_path: &str) -> ProjectId {
    let digest = Sha256::digest(format!("{}:{workspace_path}", node_id.as_str()).as_bytes());
    ProjectId::from(format!("project-{}", hex_prefix(&digest, 16)))
}

fn new_secret(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex_prefix(&digest, digest.len())
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .take(len)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn derive_presence(
    stored: NodePresence,
    heartbeat_age_seconds: Option<i64>,
    stale_after_seconds: i64,
    offline_after_seconds: i64,
) -> NodePresence {
    if stored == NodePresence::Revoked {
        return NodePresence::Revoked;
    }
    match heartbeat_age_seconds {
        Some(age) if age >= offline_after_seconds => NodePresence::Offline,
        Some(age) if age >= stale_after_seconds => NodePresence::Stale,
        Some(_) => NodePresence::Reachable,
        None => stored,
    }
}

fn parse_presence(value: &str) -> NodePresence {
    match value {
        "stale" => NodePresence::Stale,
        "offline" => NodePresence::Offline,
        "revoked" => NodePresence::Revoked,
        _ => NodePresence::Reachable,
    }
}

fn parse_enrollment_state(value: &str) -> EnrollmentState {
    match value {
        "approved" => EnrollmentState::Approved,
        "registered" => EnrollmentState::Registered,
        "expired" => EnrollmentState::Expired,
        "rejected" => EnrollmentState::Rejected,
        "revoked" => EnrollmentState::Revoked,
        "unregistered" => EnrollmentState::Unregistered,
        _ => EnrollmentState::PendingUserApproval,
    }
}

fn format_enrollment_state(value: &EnrollmentState) -> &'static str {
    match value {
        EnrollmentState::Unregistered => "unregistered",
        EnrollmentState::PendingUserApproval => "pending_user_approval",
        EnrollmentState::Approved => "approved",
        EnrollmentState::Registered => "registered",
        EnrollmentState::Expired => "expired",
        EnrollmentState::Rejected => "rejected",
        EnrollmentState::Revoked => "revoked",
    }
}

fn format_client_log_level(value: ClientLogLevel) -> &'static str {
    match value {
        ClientLogLevel::Debug => "debug",
        ClientLogLevel::Info => "info",
        ClientLogLevel::Warn => "warn",
        ClientLogLevel::Error => "error",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

async fn append_jsonl_log(path: PathBuf, line: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()
    })
    .await??;
    Ok(())
}

fn format_sleep_hint(value: SleepHint) -> &'static str {
    match value {
        SleepHint::Unknown => "unknown",
        SleepHint::Awake => "awake",
        SleepHint::Suspending => "suspending",
        SleepHint::Sleeping => "sleeping",
        SleepHint::Woke => "woke",
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn request_correlation_id(headers: &HeaderMap) -> CorrelationId {
    header_value(headers, CORRELATION_ID_HEADER)
        .or_else(|| header_value(headers, REQUEST_ID_HEADER))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(CorrelationId::from)
        .unwrap_or_default()
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let authorization = header_value(headers, "authorization")?;
    authorization
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_sleep_hint(value: &str) -> SleepHint {
    match value {
        "awake" => SleepHint::Awake,
        "suspending" => SleepHint::Suspending,
        "sleeping" => SleepHint::Sleeping,
        "woke" => SleepHint::Woke,
        _ => SleepHint::Unknown,
    }
}

fn parse_placement_state(value: &str) -> cortex_protocol::PlacementState {
    match value {
        "validated" => cortex_protocol::PlacementState::Validated,
        "missing" => cortex_protocol::PlacementState::Missing,
        "read_only" => cortex_protocol::PlacementState::ReadOnly,
        "error" => cortex_protocol::PlacementState::Error,
        _ => cortex_protocol::PlacementState::Pending,
    }
}

fn format_placement_state(value: PlacementState) -> &'static str {
    match value {
        PlacementState::Pending => "pending",
        PlacementState::Validated => "validated",
        PlacementState::Missing => "missing",
        PlacementState::ReadOnly => "read_only",
        PlacementState::Error => "error",
    }
}

fn parse_session_state(value: &str) -> SessionThreadState {
    match value {
        "active" => SessionThreadState::Active,
        "detached" => SessionThreadState::Detached,
        "stopped" => SessionThreadState::Stopped,
        "degraded" => SessionThreadState::Degraded,
        _ => SessionThreadState::Created,
    }
}

fn format_session_state(value: SessionThreadState) -> &'static str {
    match value {
        SessionThreadState::Created => "created",
        SessionThreadState::Active => "active",
        SessionThreadState::Detached => "detached",
        SessionThreadState::Stopped => "stopped",
        SessionThreadState::Degraded => "degraded",
    }
}

fn parse_runtime_state(value: &str) -> RuntimeSessionState {
    match value {
        "ready" => RuntimeSessionState::Ready,
        "running" => RuntimeSessionState::Running,
        "blocked" => RuntimeSessionState::Blocked,
        "stopping" => RuntimeSessionState::Stopping,
        "stopped" => RuntimeSessionState::Stopped,
        "interrupted" => RuntimeSessionState::Interrupted,
        "resuming" => RuntimeSessionState::Resuming,
        "stale" => RuntimeSessionState::Stale,
        "error" => RuntimeSessionState::Error,
        "expired" => RuntimeSessionState::Expired,
        _ => RuntimeSessionState::Starting,
    }
}

fn format_runtime_state(value: RuntimeSessionState) -> &'static str {
    match value {
        RuntimeSessionState::Starting => "starting",
        RuntimeSessionState::Ready => "ready",
        RuntimeSessionState::Running => "running",
        RuntimeSessionState::Blocked => "blocked",
        RuntimeSessionState::Stopping => "stopping",
        RuntimeSessionState::Stopped => "stopped",
        RuntimeSessionState::Interrupted => "interrupted",
        RuntimeSessionState::Resuming => "resuming",
        RuntimeSessionState::Stale => "stale",
        RuntimeSessionState::Error => "error",
        RuntimeSessionState::Expired => "expired",
    }
}

fn format_turn_state(value: TurnState) -> &'static str {
    match value {
        TurnState::Created => "created",
        TurnState::Dispatched => "dispatched",
        TurnState::Running => "running",
        TurnState::BlockedOnApproval => "blocked_on_approval",
        TurnState::Completed => "completed",
        TurnState::Interrupted => "interrupted",
        TurnState::Failed => "failed",
    }
}

fn format_approval_state(value: ApprovalState) -> &'static str {
    match value {
        ApprovalState::Requested => "requested",
        ApprovalState::Resolved => "resolved",
        ApprovalState::Expired => "expired",
        ApprovalState::Cancelled => "cancelled",
    }
}

fn format_command_state(value: CommandState) -> &'static str {
    match value {
        CommandState::Recorded => "recorded",
        CommandState::PendingDispatch => "pending_dispatch",
        CommandState::Dispatched => "dispatched",
        CommandState::Acknowledged => "acknowledged",
        CommandState::Completed => "completed",
        CommandState::Failed => "failed",
        CommandState::Blocked => "blocked",
        CommandState::Expired => "expired",
    }
}

fn parse_message_role(value: &str) -> MessageRole {
    match value {
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        "runtime" => MessageRole::Runtime,
        "approval" => MessageRole::Approval,
        _ => MessageRole::User,
    }
}

fn format_message_role(value: MessageRole) -> &'static str {
    match value {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
        MessageRole::Runtime => "runtime",
        MessageRole::Approval => "approval",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("background task error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error("not found: {message}")]
    NotFound { code: &'static str, message: String },
    #[error("bad request: {message}")]
    BadRequest { code: &'static str, message: String },
    #[error("auth error: {message}")]
    Auth { code: &'static str, message: String },
}

impl AppError {
    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            message: message.into(),
        }
    }

    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    fn auth(code: &'static str, message: impl Into<String>) -> Self {
        Self::Auth {
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let correlation_id = CorrelationId::from(Uuid::new_v4().to_string());
        let (status, error_code, message, retryable) = match self {
            Self::Database(error) => {
                tracing::error!(%correlation_id, error = %error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal.database",
                    "Core database operation failed".to_owned(),
                    true,
                )
            }
            Self::Serialization(error) => {
                tracing::error!(%correlation_id, error = %error, "serialization error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal.serialization",
                    "Core serialization failed".to_owned(),
                    false,
                )
            }
            Self::Io(error) => {
                tracing::error!(%correlation_id, error = %error, "io error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal.io",
                    "Core IO operation failed".to_owned(),
                    true,
                )
            }
            Self::TaskJoin(error) => {
                tracing::error!(%correlation_id, error = %error, "background task error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal.task_join",
                    "Core background task failed".to_owned(),
                    true,
                )
            }
            Self::NotFound { code, message } => (StatusCode::NOT_FOUND, code, message, false),
            Self::BadRequest { code, message } => (StatusCode::BAD_REQUEST, code, message, false),
            Self::Auth { code, message } => (StatusCode::UNAUTHORIZED, code, message, false),
        };
        let body = ApiError {
            error_code: error_code.to_owned(),
            message,
            details: JsonValue::default(),
            retryable,
            correlation_id,
        };
        (status, Json(body)).into_response()
    }
}

const MIGRATIONS: &[&str] = &[
    "pragma foreign_keys = on",
    "pragma journal_mode = wal",
    "pragma busy_timeout = 5000",
    r#"
    create table if not exists nodes (
        node_id text primary key,
        display_name text not null,
        presence text not null,
        sleep_hint text not null,
        last_heartbeat_at text,
        daemon_version text not null,
        active_runtime_count integer not null default 0,
        capabilities_json text not null,
        diagnostics text not null,
        credential_hash text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists node_enrollments (
        enrollment_id text primary key,
        display_name text not null,
        daemon_version text,
        capabilities_json text not null,
        pairing_code_hash text not null,
        status text not null,
        expires_at text not null,
        claimed_node_id text,
        created_at text not null,
        updated_at text not null,
        approved_at text
    )
    "#,
    r#"
    create table if not exists node_capabilities (
        node_id text not null references nodes(node_id),
        capability_key text not null,
        value_json text not null,
        updated_at text not null,
        primary key (node_id, capability_key)
    )
    "#,
    r#"
    create table if not exists actors (
        actor_key text primary key,
        actor_kind text not null,
        display_name text not null,
        actor_ref_json text not null,
        first_seen_at text not null,
        last_seen_at text not null
    )
    "#,
    r#"
    create table if not exists projects (
        project_id text primary key,
        display_name text not null,
        repo_id text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists project_placements (
        project_placement_id text primary key,
        project_id text references projects(project_id),
        node_id text not null references nodes(node_id),
        display_name text not null,
        workspace_path text not null,
        state text not null,
        resource_badges_json text not null,
        last_validated_at text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists deleted_workspace_bindings (
        node_id text not null references nodes(node_id),
        workspace_path text not null,
        deleted_at text not null,
        primary key(node_id, workspace_path)
    )
    "#,
    r#"
    create table if not exists session_threads (
        session_thread_id text primary key,
        project_placement_id text not null references project_placements(project_placement_id),
        runtime_session_id text not null unique,
        title text not null,
        state text not null,
        provider text not null,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists runtime_sessions (
        runtime_session_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        provider text not null,
        state text not null,
        resume_supported integer not null,
        provider_resume_ref_json text,
        degraded_reason text,
        last_runtime_step_at text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists turns (
        turn_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        command_id text not null unique,
        turn_index integer not null,
        state text not null,
        content text not null,
        blocked_approval_id text,
        created_at text not null,
        updated_at text not null,
        completed_at text,
        unique(session_thread_id, turn_index)
    )
    "#,
    r#"
    create table if not exists approvals (
        approval_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        runtime_session_id text,
        turn_id text,
        state text not null,
        request_payload_json text not null,
        response_payload_json text,
        request_command_id text,
        resolve_command_id text,
        requested_event_id text,
        resolved_event_id text,
        created_at text not null,
        updated_at text not null,
        resolved_at text
    )
    "#,
    r#"
    create table if not exists messages (
        message_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        turn_id text,
        role text not null,
        content text not null,
        created_at text not null,
        completed_at text,
        source_event_id text
    )
    "#,
    r#"
    create table if not exists commands (
        command_id text primary key,
        kind text not null,
        state text not null,
        target_node_id text not null,
        session_thread_id text,
        runtime_session_id text,
        project_placement_id text,
        actor_ref_json text not null,
        correlation_id text not null,
        source_refs_json text not null,
        cause_refs_json text not null,
        payload_json text not null,
        dedupe_key text,
        command_json text not null,
        created_at text not null,
        completed_at text
    )
    "#,
    r#"
    create table if not exists events (
        event_id text primary key,
        scope_key text not null,
        seq integer not null,
        kind text not null,
        node_id text,
        runtime_session_id text,
        session_thread_id text,
        command_id text,
        actor_ref_json text not null,
        scope_ref_json text not null,
        correlation_id text,
        source_refs_json text not null,
        evidence_refs_json text not null,
        cause_refs_json text not null,
        result_refs_json text not null,
        payload_json text not null,
        event_json text not null,
        happened_at text not null,
        unique(scope_key, seq)
    )
    "#,
    r#"
    create table if not exists warning_acknowledgements (
        event_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        actor_ref_json text not null,
        warning_kind text not null,
        command_id text,
        affected_refs_json text not null,
        acknowledged_at text not null
    )
    "#,
];

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tower::ServiceExt;

    use super::*;

    const CORE_CONFIG_ENV_VARS: &[&str] = &[
        "CORTEX_CORE_BIND",
        "CORTEX_ALLOWED_ORIGINS",
        "CORTEX_DATABASE_URL",
        "CORTEX_DEPLOYMENT_PROFILE",
        "CORTEX_HEARTBEAT_STALE_SECONDS",
        "CORTEX_HEARTBEAT_OFFLINE_SECONDS",
        "CORTEX_ENROLLMENT_TTL_SECONDS",
        "CORTEX_RUNTIME_EXPIRY_SECONDS",
        "CORTEX_AUTO_APPROVE_ENROLLMENTS",
        "CORTEX_CLIENT_LOG_FILE",
    ];

    async fn test_state() -> Arc<AppState> {
        test_state_with_runtime_expiry(86_400).await
    }

    async fn test_state_with_runtime_expiry(runtime_expiry_seconds: i64) -> Arc<AppState> {
        let pool = memory_pool().await;
        AppState::new(test_config(runtime_expiry_seconds), pool)
            .await
            .expect("state migrates")
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
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
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
            profile: DeploymentProfile::LocalTrusted,
            allowed_origins: default_allowed_origins(),
            stale_after_seconds: 15,
            offline_after_seconds: 45,
            enrollment_ttl_seconds: 600,
            runtime_expiry_seconds,
            auto_approve_enrollments: false,
            client_log_file: std::env::temp_dir()
                .join(format!("cortex-client-log-{}.jsonl", Uuid::new_v4())),
        }
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
        assert_eq!(config.database_url, "sqlite://.local/state/core.sqlite");
        assert_eq!(config.profile, DeploymentProfile::LocalTrusted);
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
        assert_eq!(
            config.client_log_file,
            PathBuf::from(".local/logs/client.log")
        );
    }

    #[test]
    fn app_config_from_env_parses_overrides() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
        std::env::set_var("CORTEX_CORE_BIND", "127.0.0.1:19080");
        std::env::set_var(
            "CORTEX_ALLOWED_ORIGINS",
            "http://127.0.0.1:5173, http://localhost:4173",
        );
        std::env::set_var("CORTEX_DATABASE_URL", "sqlite:///tmp/cortex-test.sqlite");
        std::env::set_var("CORTEX_DEPLOYMENT_PROFILE", "controlled_dev");
        std::env::set_var("CORTEX_HEARTBEAT_STALE_SECONDS", "3");
        std::env::set_var("CORTEX_HEARTBEAT_OFFLINE_SECONDS", "9");
        std::env::set_var("CORTEX_ENROLLMENT_TTL_SECONDS", "30");
        std::env::set_var("CORTEX_RUNTIME_EXPIRY_SECONDS", "120");
        std::env::set_var("CORTEX_AUTO_APPROVE_ENROLLMENTS", "yes");
        std::env::set_var("CORTEX_CLIENT_LOG_FILE", "/tmp/cortex-client.jsonl");

        let config = AppConfig::from_env().expect("overridden core config parses");

        assert_eq!(config.bind_address, "127.0.0.1:19080");
        assert_eq!(config.database_url, "sqlite:///tmp/cortex-test.sqlite");
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
        assert!(config.auto_approve_enrollments);
        assert_eq!(
            config.client_log_file,
            PathBuf::from("/tmp/cortex-client.jsonl")
        );
    }

    #[test]
    fn app_config_from_env_rejects_invalid_profile() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
        std::env::set_var("CORTEX_DEPLOYMENT_PROFILE", "production");

        let error = AppConfig::from_env().expect_err("invalid profile should fail");

        assert!(matches!(error, ConfigError::InvalidProfile(profile) if profile == "production"));
    }

    #[test]
    fn app_config_from_env_rejects_invalid_integer() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
        std::env::set_var("CORTEX_HEARTBEAT_STALE_SECONDS", "fast");

        let error = AppConfig::from_env().expect_err("invalid integer should fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger { name, .. }
                if name == "CORTEX_HEARTBEAT_STALE_SECONDS"
        ));
    }

    #[test]
    fn app_config_from_env_rejects_wildcard_cors_origin() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(CORE_CONFIG_ENV_VARS);
        std::env::set_var("CORTEX_ALLOWED_ORIGINS", "*");

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
                  'warning_acknowledgements'
              )
            "#,
        )
        .fetch_one(&state.pool)
        .await
        .expect("baseline tables count loads");

        assert_eq!(table_count, 14);
    }

    #[tokio::test]
    async fn migration_adds_credential_hash_to_previous_dev_nodes_table() {
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

        let state = AppState::new(test_config(86_400), pool)
            .await
            .expect("legacy state migrates");
        let column_count: i64 = sqlx::query_scalar(
            "select count(*) from pragma_table_info('nodes') where name = 'credential_hash'",
        )
        .fetch_one(&state.pool)
        .await
        .expect("nodes columns load");

        assert_eq!(column_count, 1);
    }

    #[tokio::test]
    async fn core_state_survives_sqlite_reopen() {
        let db_path = std::env::temp_dir().join(format!("cortex-core-{}.sqlite", Uuid::new_v4()));
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
        let db_path = std::env::temp_dir().join(format!("cortex-core-{}.sqlite", Uuid::new_v4()));
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
        let artifact_tree = build_artifact_tree(&reopened, &session_id)
            .await
            .expect("artifact tree rebuilds after reopen");
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
        assert!(artifact_tree.root.children.iter().any(|node| matches!(
            &node.primary_ref,
            CortexRef::Message { message_id } if message_id == &assistant_message.message_id
        )));
        assert!(artifact_tree.root.children.iter().any(|node| matches!(
            &node.primary_ref,
            CortexRef::Event {
                event_id: artifact_event_id,
                ..
            } if artifact_event_id == &event_id
        )));
        assert!(projection
            .recent_message_refs
            .iter()
            .any(|reference| matches!(
                reference,
                CortexRef::Message { message_id } if message_id == &assistant_message.message_id
            )));
        assert!(projection
            .artifact_tree_summary
            .contains("1 messages, 1 events"));
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = build_router(test_state().await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");

        assert_eq!(response.status(), StatusCode::OK);
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
        let request = NodeHeartbeatRequest {
            node_id: claim.node_id.clone(),
            credential: claim.credential.clone(),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![CapabilitySummary {
                key: "provider.fake".to_owned(),
                value: JsonValue(json!({ "available": true })),
            }],
            diagnostics: Some("daemon_installation_id=daemon-test".to_owned()),
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        };

        let response = node_heartbeat(State(state.clone()), Json(request))
            .await
            .expect("heartbeat accepted");
        let inventory = load_inventory(&state).await.expect("inventory loads");

        assert_eq!(response.0.accepted, !inventory.nodes.is_empty());
        assert!(inventory.nodes.iter().any(|node| node
            .diagnostics
            .contains("daemon_installation_id=daemon-test")));
    }

    #[tokio::test]
    async fn heartbeat_replaces_normalized_node_capabilities() {
        let state = test_state().await;
        let claim = enroll_test_node(&state).await;
        let node_id = claim.node_id.clone().expect("node id returned");
        let first = NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            credential: claim.credential.clone(),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![
                CapabilitySummary {
                    key: "provider.fake".to_owned(),
                    value: JsonValue(json!({ "available": true })),
                },
                CapabilitySummary {
                    key: "provider.codex".to_owned(),
                    value: JsonValue(json!({ "available": true })),
                },
            ],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        };
        let _ = node_heartbeat(State(state.clone()), Json(first))
            .await
            .expect("first heartbeat accepted");
        let second = NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            credential: claim.credential,
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![CapabilitySummary {
                key: "provider.fake".to_owned(),
                value: JsonValue(json!({ "available": false })),
            }],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        };

        let _ = node_heartbeat(State(state.clone()), Json(second))
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
        let fake_capability =
            serde_json::from_str::<JsonValue>(&rows[0].1).expect("capability value json decodes");

        assert_eq!(rows[0].0, "provider.fake");
        assert_eq!(
            fake_capability
                .0
                .get("available")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(!node_supports_provider(&state, &node_id, "fake")
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
                workspace_path: "/tmp/cortex-stale-node-workspace".to_owned(),
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
            Json(NodeHeartbeatRequest {
                node_id: Some(node_id),
                credential: claim.credential,
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
    async fn approval_moves_enrollment_to_approved_state() {
        let state = test_state().await;
        let requested = create_enrollment(&state, "Approved node", Some("0.1.0"), vec![])
            .await
            .expect("enrollment creates");

        let response =
            approve_node_enrollment(State(state), Path(requested.enrollment_id.to_string()))
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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");

        let _ = node_heartbeat(
            State(state.clone()),
            Json(NodeHeartbeatRequest {
                node_id: claim.node_id,
                credential: claim.credential,
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
    async fn delete_placement_tombstones_node_reported_workspace_until_explicit_validate() {
        let state = test_state().await;
        let claim = enroll_test_node(&state).await;
        let node_id = claim.node_id.clone().expect("node id returned");
        let workspace_path_buf =
            std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
        let workspace_path = workspace_path_buf.display().to_string();
        std::fs::create_dir_all(&workspace_path_buf).expect("workspace dir creates");

        let _ = node_heartbeat(
            State(state.clone()),
            Json(NodeHeartbeatRequest {
                node_id: claim.node_id.clone(),
                credential: claim.credential.clone(),
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
            Json(NodeHeartbeatRequest {
                node_id: claim.node_id,
                credential: claim.credential,
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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));

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
            command.project_placement_id,
            Some(placement.project_placement_id.clone())
        );
        assert!(should_open_control_channel(&state, &node_id)
            .await
            .expect("channel request evaluates"));
    }

    #[tokio::test]
    async fn command_api_uses_request_correlation_id_header() {
        let state = test_state().await;
        let claim = enroll_test_node(&state).await;
        let node_id = claim.node_id.expect("node id returned");
        heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));

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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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
                provider: Some("fake".to_owned()),
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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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
            command.project_placement_id,
            Some(placement.project_placement_id)
        );
    }

    #[tokio::test]
    async fn create_session_rejects_missing_provider_capability_without_recording_command() {
        let state = test_state().await;
        let claim = enroll_test_node(&state).await;
        let node_id = claim.node_id.expect("node id returned");
        heartbeat_test_node(&state, node_id.clone(), claim.credential.clone()).await;
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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
                title: Some("Codex session".to_owned()),
                provider: Some("codex".to_owned()),
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
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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

        assert!(messages.iter().any(
            |message| message.role == MessageRole::Assistant && message.content == "from node"
        ));
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

        let (tx, mut rx) = mpsc::unbounded_channel();
        state
            .control_channels
            .write()
            .await
            .insert(node_id.to_string(), tx);
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
        let (tx, mut rx) = mpsc::unbounded_channel();
        state
            .control_channels
            .write()
            .await
            .insert(node_id.to_string(), tx);

        handle_node_control_frame(
            &state,
            &node_id,
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
    async fn incompatible_control_hello_sends_error_and_leaves_command_pending() {
        let state = test_state().await;
        let claim = enroll_test_node(&state).await;
        let node_id = claim.node_id.expect("node id returned");
        let command_id = CommandId::from("bad-hello-command-1");
        record_and_dispatch_command(&state, command_fixture(command_id.clone(), node_id.clone()))
            .await
            .expect("command records");
        let (tx, mut rx) = mpsc::unbounded_channel();
        state
            .control_channels
            .write()
            .await
            .insert(node_id.to_string(), tx);

        let error = handle_node_control_frame(
            &state,
            &node_id,
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

        accept_node_event(&state, event.clone())
            .await
            .expect("first event accepts");
        accept_node_event(&state, event)
            .await
            .expect("duplicate event accepts");
        let messages = load_messages(&state, &detail.session.session_thread_id)
            .await
            .expect("messages load");
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        let duplicate_count = messages
            .iter()
            .filter(|message| {
                message.role == MessageRole::Assistant && message.content == "deduped"
            })
            .count();
        assert_eq!(duplicate_count, 1);
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
        let events: Vec<EventEnvelope> =
            serde_json::from_slice(&body).expect("events response parses");
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, expected_event_id);
        assert_eq!(events[0].seq, 2);
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
        let (approval_state, request_payload_json): (String, String) = sqlx::query_as(
            "select state, request_payload_json from approvals where approval_id = ?1",
        )
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
                json!({ "provider": "fake" }),
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
        let command: CommandEnvelope =
            serde_json::from_str(&command_json).expect("command decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert_eq!(command.kind, CommandKind::ResumeRuntime);
        assert_eq!(
            command
                .payload
                .0
                .get("workspace_path")
                .and_then(serde_json::Value::as_str),
            Some(detail.placement.workspace_path.as_str())
        );
        assert!(command.payload.0.get("provider_resume_ref").is_none());
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
                json!({ "provider": "fake" }),
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
            .contains(&"session.sendTurn".to_owned()));
        assert!(!projection
            .available_commands
            .contains(&"runtime.stop".to_owned()));
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
                json!({ "provider": "fake" }),
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
            .contains(&"session.sendTurn".to_owned()));
        assert!(projection
            .available_commands
            .contains(&"runtime.stop".to_owned()));
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
            .contains(&"session.detach".to_owned()));
        assert!(!before
            .available_commands
            .contains(&"session.attach".to_owned()));
        assert!(after
            .available_commands
            .contains(&"session.attach".to_owned()));
        assert!(!after
            .available_commands
            .contains(&"session.detach".to_owned()));
        assert!(!after
            .available_commands
            .contains(&"session.sendTurn".to_owned()));
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
                json!({ "provider": "fake" }),
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
                json!({ "provider": "fake" }),
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
            .contains(&"approval.resolve".to_owned()));
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
                json!({ "provider": "fake" }),
            ),
        )
        .await
        .expect("ready event accepts");

        let projection = build_agent_projection(&state, &detail.session.session_thread_id)
            .await
            .expect("projection builds");
        let (approval_state, response_payload_json): (String, Option<String>) = sqlx::query_as(
            "select state, response_payload_json from approvals where approval_id = ?1",
        )
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
            .contains(&"approval.resolve".to_owned()));
        assert!(projection
            .available_commands
            .contains(&"session.sendTurn".to_owned()));
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
        let row_count: i64 = sqlx::query_scalar(
            "select count(*) from warning_acknowledgements where warning_kind = ?1",
        )
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
            .contains(&"warning.acknowledge".to_owned()));
        assert_eq!(row_count, 1);
        assert_eq!(event_kind, "CoordinationWarningAcknowledged");
        assert!(after.active_warnings.is_empty());
        assert!(!after
            .available_commands
            .contains(&"warning.acknowledge".to_owned()));
    }

    #[tokio::test]
    async fn artifact_tree_uses_approval_ref_for_approval_event() {
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

        let artifact_tree = build_artifact_tree(&state, &detail.session.session_thread_id)
            .await
            .expect("artifact tree builds");
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert!(artifact_tree.root.children.iter().any(|node| matches!(
            &node.primary_ref,
            CortexRef::Approval { approval_id } if approval_id.as_str() == "approval-artifact-1"
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
        set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2))
            .await;

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
        set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2))
            .await;
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
        set_session_runtime_last_step(&state, &detail, Utc::now() - chrono::Duration::seconds(2))
            .await;
        sqlx::query(
            "update runtime_sessions set provider_resume_ref_json = ?1 where runtime_session_id = ?2",
        )
        .bind(json!({
            "provider_session_id": "codex-session-1",
            "resume_cursor": "cursor-1",
        }).to_string())
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
        let command: CommandEnvelope =
            serde_json::from_str(&command_json).expect("command decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert_eq!(command_kind, "ResumeRuntime");
        assert_eq!(
            command
                .payload
                .0
                .get("provider_resume_ref")
                .and_then(|value| value.get("provider_session_id"))
                .and_then(serde_json::Value::as_str),
            Some("codex-session-1")
        );
        assert_eq!(
            command
                .payload
                .0
                .get("workspace_path")
                .and_then(serde_json::Value::as_str),
            Some(detail.placement.workspace_path.as_str())
        );
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
                json!({ "provider": "fake" }),
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
                key: "provider.fake".to_owned(),
                value: JsonValue(json!({ "available": true })),
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

    async fn create_test_session(
        state: &Arc<AppState>,
    ) -> (NodeId, SessionDetail, std::path::PathBuf) {
        let claim = enroll_test_node(state).await;
        let node_id = claim.node_id.clone().expect("node id returned");
        heartbeat_test_node(state, node_id.clone(), claim.credential.clone()).await;
        let workspace_path = std::env::temp_dir().join(format!("cortex-test-{}", Uuid::new_v4()));
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
                provider: Some("fake".to_owned()),
            }),
        )
        .await
        .expect("session creates")
        .0;
        (node_id, detail, workspace_path)
    }

    async fn heartbeat_test_node(
        state: &Arc<AppState>,
        node_id: NodeId,
        credential: Option<String>,
    ) {
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
            Json(NodeHeartbeatRequest {
                node_id: Some(node_id),
                credential,
                display_name: display_name.to_owned(),
                daemon_version: "0.1.0".to_owned(),
                capabilities: vec![CapabilitySummary {
                    key: "provider.fake".to_owned(),
                    value: JsonValue(json!({ "available": true })),
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
                kind,
                happened_at: Utc::now(),
                source_refs: vec![],
                evidence_refs: vec![],
                cause_refs: vec![],
                result_refs: vec![],
                payload: JsonValue(json!({
                    "placement_id": placement.project_placement_id.as_str(),
                    "display_name": placement.display_name.as_str(),
                    "workspace_path": placement.workspace_path.as_str(),
                    "state": placement_state,
                    "resource_badges": resource_badges,
                })),
            },
        )
        .await
        .expect("workspace validation event accepts");
    }

    fn command_fixture(command_id: CommandId, node_id: NodeId) -> CommandEnvelope {
        CommandEnvelope {
            command_id,
            kind: CommandKind::RefreshResourceSnapshot,
            target_node_id: node_id,
            actor_ref: ActorRef::local_user(),
            session_thread_id: None,
            runtime_session_id: None,
            project_placement_id: None,
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("correlation-1"),
            payload: JsonValue(json!({})),
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
                provider: "fake".to_owned(),
            },
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: runtime_session_id.clone(),
            },
            node_id: Some(node_id),
            runtime_session_id: Some(runtime_session_id),
            session_thread_id: Some(detail.session.session_thread_id.clone()),
            turn_id: Some(TurnId::from("turn-1")),
            seq,
            kind,
            happened_at: Utc::now(),
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: JsonValue(payload),
        }
    }

    #[tokio::test]
    async fn record_command_persists_queryable_envelope_fields() {
        let state = test_state().await;
        let node_id = NodeId::from("node-queryable-command");
        let command_id = CommandId::from("command-queryable");
        let mut command = command_fixture(command_id.clone(), node_id.clone());
        command.actor_ref = ActorRef::System;
        command.source_refs = vec![CortexRef::Node {
            node_id: node_id.clone(),
        }];
        command.cause_refs = vec![CortexRef::Command {
            command_id: CommandId::from("command-cause"),
        }];
        command.payload = JsonValue(json!({ "reason": "queryable fields" }));

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
        let source_refs = serde_json::from_str::<Vec<CortexRef>>(&source_refs_json)
            .expect("source refs json decodes");
        let cause_refs = serde_json::from_str::<Vec<CortexRef>>(&cause_refs_json)
            .expect("cause refs json decodes");
        let payload =
            serde_json::from_str::<JsonValue>(&payload_json).expect("payload json decodes");

        assert_eq!(actor_ref, ActorRef::System);
        assert_eq!(correlation_id, "correlation-1");
        assert_eq!(
            source_refs,
            vec![CortexRef::Node {
                node_id: node_id.clone()
            }]
        );
        assert_eq!(
            cause_refs,
            vec![CortexRef::Command {
                command_id: CommandId::from("command-cause")
            }]
        );
        assert_eq!(
            payload.0.get("reason").and_then(serde_json::Value::as_str),
            Some("queryable fields")
        );
        assert_eq!(dedupe_key, command_id.as_str());
    }

    #[tokio::test]
    async fn accept_node_event_persists_queryable_envelope_fields() {
        let state = test_state().await;
        let node_id = NodeId::from("node-queryable-event");
        let runtime_session_id = RuntimeSessionId::from("runtime-queryable-event");
        let scope_ref = ScopeRef::Runtime {
            runtime_session_id: runtime_session_id.clone(),
        };
        let source_ref = CortexRef::Node {
            node_id: node_id.clone(),
        };
        let result_ref = CortexRef::Runtime {
            runtime_session_id: runtime_session_id.clone(),
        };

        accept_node_event(
            &state,
            EventEnvelope {
                event_id: EventId::from("event-queryable"),
                command_id: None,
                correlation_id: Some(CorrelationId::from("correlation-event")),
                actor_ref: ActorRef::Provider {
                    provider: "fake".to_owned(),
                },
                scope_ref: scope_ref.clone(),
                node_id: Some(node_id),
                runtime_session_id: None,
                session_thread_id: None,
                turn_id: None,
                seq: 1,
                kind: EventKind::ProviderMessageCompleted,
                happened_at: Utc::now(),
                source_refs: vec![source_ref.clone()],
                evidence_refs: vec![],
                cause_refs: vec![CortexRef::Command {
                    command_id: CommandId::from("command-cause"),
                }],
                result_refs: vec![result_ref.clone()],
                payload: JsonValue(json!({ "content": "queryable event" })),
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
        let source_refs = serde_json::from_str::<Vec<CortexRef>>(&source_refs_json)
            .expect("source refs json decodes");
        let result_refs = serde_json::from_str::<Vec<CortexRef>>(&result_refs_json)
            .expect("result refs json decodes");
        let payload =
            serde_json::from_str::<JsonValue>(&payload_json).expect("payload json decodes");

        assert_eq!(
            actor_ref,
            ActorRef::Provider {
                provider: "fake".to_owned()
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
                    provider: "fake".to_owned(),
                },
                scope_ref: ScopeRef::Runtime {
                    runtime_session_id: runtime_session_id.clone(),
                },
                node_id: Some(node_id),
                runtime_session_id: Some(runtime_session_id),
                session_thread_id: None,
                turn_id: None,
                seq: 1,
                kind: EventKind::RuntimeReady,
                happened_at: Utc::now(),
                source_refs: vec![],
                evidence_refs: vec![],
                cause_refs: vec![],
                result_refs: vec![],
                payload: JsonValue(json!({})),
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
            "select count(*) from actors where actor_key in ('local_user', 'provider:fake')",
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
}
