use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::ErrorKind,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Output},
    time::Duration,
};

use anyhow::Context;
use chrono::Utc;
use cortex_protocol::{
    serde_json_value::JsonValue, ActorRef, ApiError, ApprovalId, CapabilitySummary,
    CommandEnvelope, CommandId, CommandKind, CommandState, ControlFrame, CorrelationId,
    EnrollmentId, EventEnvelope, EventId, EventKind, NodeEnrollmentClaimRequest,
    NodeEnrollmentClaimResponse, NodeEnrollmentRequest, NodeEnrollmentRequestedResponse,
    NodeHeartbeatRequest, NodeHeartbeatResponse, NodeId, PlacementState, ProjectPlacementId,
    ResourceBadge, RuntimeSessionId, RuntimeSessionState, ScopeRef, SessionThreadId, SleepHint,
    TurnId, WarningSeverity, WorkspaceSnapshot,
};
use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::{process::Command as TokioCommand, time::timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message as WsMessage},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

type ControlSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const API_VERSION: &str = "v1";
const MAX_EVENT_OUTBOX_EVENTS: usize = 1024;
const MAX_CODEX_TRANSCRIPT_MESSAGES: usize = 20;
const MAX_CODEX_TRANSCRIPT_CHARS: usize = 12_000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = NodeConfig::from_env()?;
    let client = reqwest::Client::new();
    let mut local_state = NodeLocalState::load(&config.state_path)?;
    let mut control_started = false;
    tracing::info!(
        core_url = %config.core_url,
        display_name = %config.display_name,
        state_path = %config.state_path.display(),
        "starting cortex node"
    );

    loop {
        if !local_state.is_enrolled() {
            match ensure_enrollment(&client, &config, &mut local_state).await {
                Ok(true) => {}
                Ok(false) => {
                    tokio::time::sleep(config.heartbeat_interval).await;
                    continue;
                }
                Err(error) => {
                    tracing::warn!(error = %error, "enrollment step failed");
                    tokio::time::sleep(config.heartbeat_interval).await;
                    continue;
                }
            }
        }

        if control_started {
            match NodeLocalState::load(&config.state_path) {
                Ok(updated_state) if updated_state.is_enrolled() => local_state = updated_state,
                Ok(_) => tracing::warn!("node state refresh returned unenrolled state"),
                Err(error) => tracing::warn!(error = %error, "failed to refresh node state"),
            }
        }

        match send_heartbeat(&client, &config, &local_state).await {
            Ok(response) => {
                tracing::info!(
                    accepted = response.accepted,
                    open_control_channel = response.open_control_channel,
                    node_id = %response.node_id,
                    "heartbeat accepted"
                );
                if response.open_control_channel && !control_started {
                    control_started = true;
                    tokio::spawn(control_channel_loop(config.clone(), local_state.clone()));
                }
            }
            Err(error) => tracing::warn!(error = %error, "heartbeat failed"),
        }
        tokio::time::sleep(config.heartbeat_interval).await;
    }
}

#[derive(Debug, Clone)]
struct NodeConfig {
    core_url: Url,
    display_name: String,
    heartbeat_interval: Duration,
    state_path: PathBuf,
    workspace_paths: Vec<PathBuf>,
    codex_binary: String,
    codex_timeout: Duration,
}

impl NodeConfig {
    fn from_env() -> anyhow::Result<Self> {
        let core_url = std::env::var("CORTEX_CORE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned())
            .parse::<Url>()
            .context("CORTEX_CORE_URL must be a valid URL")?;
        let display_name =
            std::env::var("CORTEX_NODE_DISPLAY_NAME").unwrap_or_else(|_| "Local Node".to_owned());
        let heartbeat_interval = parse_env_duration_seconds("CORTEX_NODE_HEARTBEAT_SECONDS", 5)?;
        let state_path = std::env::var("CORTEX_NODE_STATE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_state_path());
        let workspace_paths = std::env::var("CORTEX_NODE_WORKSPACES")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let codex_binary =
            std::env::var("CORTEX_CODEX_BINARY").unwrap_or_else(|_| "codex".to_owned());
        let codex_timeout = parse_env_duration_seconds("CORTEX_CODEX_TIMEOUT_SECONDS", 120)?;

        Ok(Self {
            core_url,
            display_name,
            heartbeat_interval,
            state_path,
            workspace_paths,
            codex_binary,
            codex_timeout,
        })
    }
}

fn parse_env_duration_seconds(name: &str, fallback_seconds: u64) -> anyhow::Result<Duration> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map(Duration::from_secs)
            .with_context(|| format!("{name} must be an unsigned integer number of seconds")),
        Err(_) => Ok(Duration::from_secs(fallback_seconds)),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct NodeLocalState {
    #[serde(default = "new_daemon_installation_id")]
    daemon_installation_id: String,
    node_id: Option<NodeId>,
    credential: Option<String>,
    enrollment_id: Option<EnrollmentId>,
    pairing_code: Option<String>,
    #[serde(default)]
    command_status: HashMap<String, CommandState>,
    #[serde(default)]
    runtime_seqs: HashMap<String, i64>,
    #[serde(default)]
    event_outbox: Vec<EventEnvelope>,
    #[serde(default)]
    runtime_providers: HashMap<String, String>,
    #[serde(default)]
    runtime_workspace_paths: HashMap<String, String>,
    #[serde(default)]
    runtime_states: HashMap<String, RuntimeSessionState>,
    #[serde(default)]
    runtime_transcripts: HashMap<String, Vec<ProviderTranscriptMessage>>,
    #[serde(default)]
    runtime_provider_resume_refs: HashMap<String, ProviderResumeRef>,
    #[serde(default)]
    placement_seqs: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ProviderTranscriptMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ProviderResumeRef {
    #[serde(default)]
    provider_session_id: Option<String>,
    #[serde(default)]
    resume_cursor: Option<String>,
}

impl Default for NodeLocalState {
    fn default() -> Self {
        Self {
            daemon_installation_id: new_daemon_installation_id(),
            node_id: None,
            credential: None,
            enrollment_id: None,
            pairing_code: None,
            command_status: HashMap::new(),
            runtime_seqs: HashMap::new(),
            event_outbox: Vec::new(),
            runtime_providers: HashMap::new(),
            runtime_workspace_paths: HashMap::new(),
            runtime_states: HashMap::new(),
            runtime_transcripts: HashMap::new(),
            runtime_provider_resume_refs: HashMap::new(),
            placement_seqs: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for NodeLocalState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let runtime_transcript_counts = self
            .runtime_transcripts
            .iter()
            .map(|(runtime_id, transcript)| (runtime_id, transcript.len()))
            .collect::<BTreeMap<_, _>>();
        let runtime_provider_resume_ref_count = self.runtime_provider_resume_refs.len();
        formatter
            .debug_struct("NodeLocalState")
            .field("daemon_installation_id", &self.daemon_installation_id)
            .field("node_id", &self.node_id)
            .field(
                "credential",
                &self.credential.as_ref().map(|_| "[redacted]"),
            )
            .field("enrollment_id", &self.enrollment_id)
            .field(
                "pairing_code",
                &self.pairing_code.as_ref().map(|_| "[redacted]"),
            )
            .field("command_status", &self.command_status)
            .field("runtime_seqs", &self.runtime_seqs)
            .field("event_outbox", &self.event_outbox)
            .field("runtime_providers", &self.runtime_providers)
            .field("runtime_workspace_paths", &self.runtime_workspace_paths)
            .field("runtime_states", &self.runtime_states)
            .field("runtime_transcript_counts", &runtime_transcript_counts)
            .field(
                "runtime_provider_resume_ref_count",
                &runtime_provider_resume_ref_count,
            )
            .field("placement_seqs", &self.placement_seqs)
            .finish()
    }
}

fn new_daemon_installation_id() -> String {
    format!("daemon-{}", Uuid::new_v4())
}

impl NodeLocalState {
    fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read node state {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse node state {}", path.display()))
    }

    fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize node state")?;
        std::fs::write(path, content)
            .with_context(|| format!("failed to write node state {}", path.display()))
    }

    fn is_enrolled(&self) -> bool {
        self.node_id.is_some() && self.credential.is_some()
    }
}

async fn ensure_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
) -> anyhow::Result<bool> {
    if local_state.enrollment_id.is_none() || local_state.pairing_code.is_none() {
        let response = request_enrollment(client, config).await?;
        tracing::info!(
            enrollment_id = %response.enrollment_id,
            expires_at = %response.expires_at,
            "enrollment requested; approve this enrollment in Core"
        );
        local_state.enrollment_id = Some(response.enrollment_id);
        local_state.pairing_code = Some(response.pairing_code);
        local_state.save(&config.state_path)?;
    }

    let claim = claim_enrollment(client, config, local_state).await?;
    if claim.pending {
        tracing::info!("waiting for enrollment approval");
        return Ok(false);
    }
    if !claim.accepted {
        tracing::warn!(message = %claim.message, "enrollment was not accepted");
        local_state.enrollment_id = None;
        local_state.pairing_code = None;
        local_state.save(&config.state_path)?;
        return Ok(false);
    }
    if let (Some(node_id), Some(credential)) = (claim.node_id, claim.credential) {
        local_state.node_id = Some(node_id);
        local_state.credential = Some(credential);
        local_state.enrollment_id = None;
        local_state.pairing_code = None;
        local_state.save(&config.state_path)?;
        tracing::info!("enrollment claimed and credential stored");
        return Ok(true);
    }
    Ok(local_state.is_enrolled())
}

async fn request_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
) -> anyhow::Result<NodeEnrollmentRequestedResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/enrollment-requests")
        .context("enrollment request URL should be valid")?;
    let request = NodeEnrollmentRequest {
        display_name: config.display_name.clone(),
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        capabilities: capabilities(config),
    };
    client
        .post(endpoint)
        .json(&request)
        .send()
        .await
        .context("enrollment request failed")?
        .error_for_status()
        .context("enrollment request returned an error status")?
        .json::<NodeEnrollmentRequestedResponse>()
        .await
        .context("enrollment response was not valid JSON")
}

async fn claim_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &NodeLocalState,
) -> anyhow::Result<NodeEnrollmentClaimResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/enrollment-claims")
        .context("enrollment claim URL should be valid")?;
    let enrollment_id = local_state
        .enrollment_id
        .clone()
        .context("local enrollment id missing")?;
    let pairing_code = local_state
        .pairing_code
        .clone()
        .context("local pairing code missing")?;
    client
        .post(endpoint)
        .json(&NodeEnrollmentClaimRequest {
            enrollment_id,
            pairing_code,
        })
        .send()
        .await
        .context("enrollment claim failed")?
        .error_for_status()
        .context("enrollment claim returned an error status")?
        .json::<NodeEnrollmentClaimResponse>()
        .await
        .context("enrollment claim response was not valid JSON")
}

async fn send_heartbeat(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &NodeLocalState,
) -> anyhow::Result<NodeHeartbeatResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/heartbeat")
        .context("heartbeat URL should be valid")?;
    let request = NodeHeartbeatRequest {
        node_id: local_state.node_id.clone(),
        credential: local_state.credential.clone(),
        display_name: config.display_name.clone(),
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        capabilities: capabilities(config),
        diagnostics: Some(node_diagnostics(local_state)),
        active_runtime_count: active_runtime_count(local_state),
        sleep_hint: SleepHint::Awake,
        workspace_summaries: config
            .workspace_paths
            .iter()
            .map(|path| validate_workspace(path))
            .collect(),
    };

    client
        .post(endpoint)
        .json(&request)
        .send()
        .await
        .context("heartbeat request failed")?
        .error_for_status()
        .context("heartbeat returned an error status")?
        .json::<NodeHeartbeatResponse>()
        .await
        .context("heartbeat response was not valid JSON")
}

fn node_diagnostics(local_state: &NodeLocalState) -> String {
    format!(
        "daemon_installation_id={}; outbox_events={}; cached_commands={}",
        local_state.daemon_installation_id,
        local_state.event_outbox.len(),
        local_state.command_status.len()
    )
}

async fn control_channel_loop(config: NodeConfig, mut local_state: NodeLocalState) {
    loop {
        match run_control_channel(&config, &mut local_state).await {
            Ok(()) => tracing::warn!("control channel closed"),
            Err(error) => tracing::warn!(error = %error, "control channel failed"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_control_channel(
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
) -> anyhow::Result<()> {
    let node_id = local_state
        .node_id
        .clone()
        .context("node id is missing for control channel")?;
    let credential = local_state
        .credential
        .as_deref()
        .context("credential is missing for control channel")?;
    let url = control_url(&config.core_url)?;
    let mut request = url
        .as_str()
        .into_client_request()
        .context("control channel request should build")?;
    request.headers_mut().insert(
        "x-cortex-node-id",
        HeaderValue::from_str(node_id.as_str()).context("node id header should be valid")?,
    );
    request.headers_mut().insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {credential}"))
            .context("authorization header should be valid")?,
    );

    let (mut socket, _) = connect_async(request)
        .await
        .context("control channel connection failed")?;
    tracing::info!(node_id = %node_id, "control channel connected");
    send_frame(
        &mut socket,
        ControlFrame::Hello {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            node_id: node_id.clone(),
            daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
            active_runtime_ids: active_runtime_ids(local_state),
        },
    )
    .await?;

    replay_event_outbox(&mut socket, &local_state.event_outbox).await?;

    while let Some(message) = socket.next().await {
        let message = message.context("control channel read failed")?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let frame = serde_json::from_str::<ControlFrame>(&text)
            .context("control frame was not valid JSON")?;
        if let Some(error_frame) = control_frame_protocol_error(&frame) {
            send_frame(&mut socket, error_frame).await?;
            continue;
        }
        match frame {
            ControlFrame::CommandDispatch { command, .. } => {
                handle_command_dispatch(config, &mut socket, command, local_state).await?;
            }
            ControlFrame::Ping { frame_id, .. } => {
                send_frame(
                    &mut socket,
                    ControlFrame::Pong {
                        frame_id,
                        protocol_version: API_VERSION.to_owned(),
                        sent_at: Utc::now(),
                    },
                )
                .await?;
            }
            ControlFrame::EventBatchAck {
                accepted_event_ids, ..
            } => {
                let removed =
                    remove_acked_events(&mut local_state.event_outbox, &accepted_event_ids);
                if removed > 0 {
                    local_state.save(&config.state_path)?;
                    tracing::info!(
                        removed,
                        remaining = local_state.event_outbox.len(),
                        "control event outbox acked"
                    );
                }
            }
            ControlFrame::HelloAck { .. } => {}
            _ => {}
        }
    }
    Ok(())
}

async fn handle_command_dispatch(
    config: &NodeConfig,
    socket: &mut ControlSocket,
    command: CommandEnvelope,
    local_state: &mut NodeLocalState,
) -> anyhow::Result<()> {
    send_frame(
        socket,
        ControlFrame::CommandAck {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Acknowledged,
        },
    )
    .await?;

    let outcome = prepare_command_dispatch(config, local_state, &command).await;
    if outcome.state_changed {
        local_state.save(&config.state_path)?;
    }

    send_event_batch(socket, outcome.events_to_send).await?;
    send_command_result(socket, &command.command_id, outcome.status).await
}

async fn send_command_result(
    socket: &mut ControlSocket,
    command_id: &CommandId,
    status: CommandState,
) -> anyhow::Result<()> {
    send_frame(
        socket,
        ControlFrame::CommandResult {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command_id.clone(),
            status,
            payload: JsonValue(serde_json::json!({})),
        },
    )
    .await
}

async fn replay_event_outbox(
    socket: &mut ControlSocket,
    events: &[EventEnvelope],
) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    tracing::info!(events = events.len(), "replaying control event outbox");
    send_event_batch(socket, events.to_vec()).await
}

async fn send_event_batch(
    socket: &mut ControlSocket,
    events: Vec<EventEnvelope>,
) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    send_frame(
        socket,
        ControlFrame::EventBatch {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            events,
        },
    )
    .await
}

async fn send_frame(socket: &mut ControlSocket, frame: ControlFrame) -> anyhow::Result<()> {
    let text = serde_json::to_string(&frame).context("control frame should serialize")?;
    socket
        .send(WsMessage::Text(text.into()))
        .await
        .context("control frame send failed")
}

fn control_frame_protocol_error(frame: &ControlFrame) -> Option<ControlFrame> {
    let received_protocol_version = control_frame_protocol_version(frame);
    if received_protocol_version == API_VERSION {
        return None;
    }

    Some(ControlFrame::ControlError {
        frame_id: Uuid::new_v4().to_string(),
        protocol_version: API_VERSION.to_owned(),
        sent_at: Utc::now(),
        error: ApiError {
            error_code: "control.protocol_incompatible".to_owned(),
            message: "Control protocol version is incompatible".to_owned(),
            details: JsonValue(serde_json::json!({
                "expected_protocol_version": API_VERSION,
                "received_protocol_version": received_protocol_version,
            })),
            retryable: false,
            correlation_id: CorrelationId::from(Uuid::new_v4().to_string()),
        },
    })
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

#[derive(Debug)]
struct CommandDispatchOutcome {
    status: CommandState,
    events_to_send: Vec<EventEnvelope>,
    state_changed: bool,
}

async fn prepare_command_dispatch(
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
) -> CommandDispatchOutcome {
    if let Some(status) = local_state
        .command_status
        .get(command.command_id.as_str())
        .copied()
    {
        return CommandDispatchOutcome {
            status,
            events_to_send: outbox_events_for_command(
                &local_state.event_outbox,
                &command.command_id,
            ),
            state_changed: false,
        };
    }

    let events = match command.kind {
        CommandKind::ValidateWorkspace => {
            workspace_validation_events(config, command, &mut local_state.placement_seqs)
        }
        CommandKind::RefreshResourceSnapshot => {
            resource_snapshot_events(config, command, &mut local_state.placement_seqs)
        }
        _ => {
            remember_runtime_metadata(local_state, command);
            let provider_key = provider_for_command(local_state, command);
            let workspace_path = workspace_path_for_command(local_state, command);
            let runtime_manager = RuntimeManager::for_provider(&provider_key, config);
            runtime_manager
                .execute_command(
                    command,
                    &mut local_state.runtime_seqs,
                    workspace_path.as_deref(),
                    &mut local_state.runtime_transcripts,
                    &mut local_state.runtime_provider_resume_refs,
                )
                .await
        }
    };
    apply_runtime_state_projection(local_state, &events);
    local_state.event_outbox.extend(events.iter().cloned());
    let retention_notices = enforce_event_outbox_retention(local_state, MAX_EVENT_OUTBOX_EVENTS);
    let status = command_status_for_events(&events);
    local_state
        .command_status
        .insert(command.command_id.to_string(), status);
    let mut events_to_send = events;
    events_to_send.extend(retention_notices);

    CommandDispatchOutcome {
        status,
        events_to_send,
        state_changed: true,
    }
}

fn command_status_for_events(events: &[EventEnvelope]) -> CommandState {
    if events.is_empty() {
        return CommandState::Failed;
    }
    if events
        .iter()
        .any(|event| event.kind == EventKind::RuntimeError)
    {
        return CommandState::Failed;
    }
    CommandState::Completed
}

fn outbox_events_for_command(
    outbox: &[EventEnvelope],
    command_id: &CommandId,
) -> Vec<EventEnvelope> {
    outbox
        .iter()
        .filter(|event| {
            event
                .command_id
                .as_ref()
                .is_some_and(|event_command_id| event_command_id == command_id)
        })
        .cloned()
        .collect()
}

fn remove_acked_events(outbox: &mut Vec<EventEnvelope>, accepted_event_ids: &[EventId]) -> usize {
    if outbox.is_empty() || accepted_event_ids.is_empty() {
        return 0;
    }
    let accepted_event_ids = accepted_event_ids
        .iter()
        .map(EventId::as_str)
        .collect::<HashSet<_>>();
    let original_len = outbox.len();
    outbox.retain(|event| !accepted_event_ids.contains(event.event_id.as_str()));
    original_len - outbox.len()
}

fn enforce_event_outbox_retention(
    local_state: &mut NodeLocalState,
    max_events: usize,
) -> Vec<EventEnvelope> {
    if max_events == 0 || local_state.event_outbox.len() <= max_events {
        return vec![];
    }

    let overflow = local_state.event_outbox.len() - max_events;
    let dropped = local_state
        .event_outbox
        .drain(0..overflow)
        .collect::<Vec<_>>();
    let mut affected_runtimes =
        BTreeMap::<String, (RuntimeSessionId, Option<SessionThreadId>, Option<NodeId>)>::new();
    for event in dropped {
        let Some(runtime_session_id) = event.runtime_session_id.clone() else {
            continue;
        };
        affected_runtimes
            .entry(runtime_session_id.to_string())
            .or_insert_with(|| {
                (
                    runtime_session_id,
                    event.session_thread_id.clone(),
                    event.node_id.clone(),
                )
            });
    }

    let notices = affected_runtimes
        .into_values()
        .map(|(runtime_session_id, session_thread_id, event_node_id)| {
            runtime_outbox_retention_event(
                local_state,
                runtime_session_id,
                session_thread_id,
                event_node_id,
            )
        })
        .collect::<Vec<_>>();
    if notices.is_empty() {
        trim_event_outbox_to_limit(&mut local_state.event_outbox, max_events);
        return notices;
    }

    apply_runtime_state_projection(local_state, &notices);
    local_state.event_outbox.extend(notices.iter().cloned());
    trim_event_outbox_to_limit(&mut local_state.event_outbox, max_events);
    notices
}

fn trim_event_outbox_to_limit(outbox: &mut Vec<EventEnvelope>, max_events: usize) {
    if outbox.len() > max_events {
        let overflow = outbox.len() - max_events;
        outbox.drain(0..overflow);
    }
}

fn runtime_outbox_retention_event(
    local_state: &mut NodeLocalState,
    runtime_session_id: RuntimeSessionId,
    session_thread_id: Option<SessionThreadId>,
    event_node_id: Option<NodeId>,
) -> EventEnvelope {
    let seq = next_runtime_seq(&mut local_state.runtime_seqs, &runtime_session_id);
    let node_id = local_state.node_id.clone().or(event_node_id);
    EventEnvelope {
        event_id: EventId::new(),
        command_id: None,
        correlation_id: None,
        actor_ref: node_id
            .clone()
            .map(|node_id| ActorRef::Node { node_id })
            .unwrap_or(ActorRef::Unknown),
        scope_ref: ScopeRef::Runtime {
            runtime_session_id: runtime_session_id.clone(),
        },
        node_id,
        runtime_session_id: Some(runtime_session_id),
        session_thread_id,
        turn_id: None,
        seq,
        kind: EventKind::RuntimeError,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: JsonValue(serde_json::json!({
            "code": "node.event_outbox_retention_exceeded",
            "message": "Node dropped unacknowledged runtime events because local outbox retention was exceeded",
        })),
    }
}

fn remember_runtime_metadata(local_state: &mut NodeLocalState, command: &CommandEnvelope) -> bool {
    let Some(runtime_session_id) = &command.runtime_session_id else {
        return false;
    };
    let runtime_key = runtime_session_id.to_string();
    let mut changed = false;

    if matches!(command.kind, CommandKind::StartRuntime) {
        let provider = command_payload_str(command, "provider").unwrap_or("fake");
        changed |= insert_if_changed(
            &mut local_state.runtime_providers,
            runtime_key.clone(),
            provider.to_owned(),
        );
    } else if matches!(command.kind, CommandKind::ResumeRuntime) {
        if let Some(provider) = command_payload_str(command, "provider") {
            changed |= insert_if_changed(
                &mut local_state.runtime_providers,
                runtime_key.clone(),
                provider.to_owned(),
            );
        }
        if let Some(provider_resume_ref) = command_provider_resume_ref(command) {
            changed |= insert_if_changed(
                &mut local_state.runtime_provider_resume_refs,
                runtime_key.clone(),
                provider_resume_ref,
            );
        }
    }

    if matches!(
        command.kind,
        CommandKind::StartRuntime | CommandKind::ResumeRuntime
    ) {
        if let Some(workspace_path) = command_payload_str(command, "workspace_path") {
            changed |= insert_if_changed(
                &mut local_state.runtime_workspace_paths,
                runtime_key,
                workspace_path.to_owned(),
            );
        }
    }

    changed
}

fn provider_for_command(local_state: &NodeLocalState, command: &CommandEnvelope) -> String {
    command_payload_str(command, "provider")
        .map(str::to_owned)
        .or_else(|| {
            command
                .runtime_session_id
                .as_ref()
                .and_then(|runtime_session_id| {
                    local_state
                        .runtime_providers
                        .get(runtime_session_id.as_str())
                        .cloned()
                })
        })
        .unwrap_or_else(|| "fake".to_owned())
}

fn workspace_path_for_command(
    local_state: &NodeLocalState,
    command: &CommandEnvelope,
) -> Option<String> {
    command_payload_str(command, "workspace_path")
        .map(str::to_owned)
        .or_else(|| {
            command
                .runtime_session_id
                .as_ref()
                .and_then(|runtime_session_id| {
                    local_state
                        .runtime_workspace_paths
                        .get(runtime_session_id.as_str())
                        .cloned()
                })
        })
}

fn command_payload_str<'a>(command: &'a CommandEnvelope, key: &str) -> Option<&'a str> {
    command
        .payload
        .0
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn insert_if_changed<T: PartialEq>(map: &mut HashMap<String, T>, key: String, value: T) -> bool {
    if map.get(&key) == Some(&value) {
        return false;
    }
    map.insert(key, value);
    true
}

fn command_provider_resume_ref(command: &CommandEnvelope) -> Option<ProviderResumeRef> {
    command
        .payload
        .0
        .get("provider_resume_ref")
        .and_then(provider_resume_ref_from_json)
}

fn provider_resume_ref_from_json(value: &serde_json::Value) -> Option<ProviderResumeRef> {
    let provider_session_id = value
        .get("provider_session_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| bounded_text(value, 512));
    let resume_cursor = value
        .get("resume_cursor")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| bounded_text(value, 512));
    if provider_session_id.is_none() && resume_cursor.is_none() {
        None
    } else {
        Some(ProviderResumeRef {
            provider_session_id,
            resume_cursor,
        })
    }
}

fn provider_resume_ref_json(resume_ref: &ProviderResumeRef) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    if let Some(provider_session_id) = &resume_ref.provider_session_id {
        value.insert(
            "provider_session_id".to_owned(),
            serde_json::Value::String(provider_session_id.clone()),
        );
    }
    if let Some(resume_cursor) = &resume_ref.resume_cursor {
        value.insert(
            "resume_cursor".to_owned(),
            serde_json::Value::String(resume_cursor.clone()),
        );
    }
    serde_json::Value::Object(value)
}

fn apply_runtime_state_projection(local_state: &mut NodeLocalState, events: &[EventEnvelope]) {
    for event in events {
        let Some(runtime_session_id) = &event.runtime_session_id else {
            continue;
        };
        let Some(state) = runtime_state_for_event(event.kind.clone()) else {
            continue;
        };
        local_state
            .runtime_states
            .insert(runtime_session_id.to_string(), state);
    }
}

fn runtime_state_for_event(kind: EventKind) -> Option<RuntimeSessionState> {
    match kind {
        EventKind::RuntimeStarting => Some(RuntimeSessionState::Starting),
        EventKind::RuntimeReady => Some(RuntimeSessionState::Ready),
        EventKind::RuntimeRunning => Some(RuntimeSessionState::Running),
        EventKind::RuntimeBlocked => Some(RuntimeSessionState::Blocked),
        EventKind::RuntimeExpired => Some(RuntimeSessionState::Expired),
        EventKind::RuntimeResuming => Some(RuntimeSessionState::Resuming),
        EventKind::RuntimeStopped => Some(RuntimeSessionState::Stopped),
        EventKind::RuntimeError => Some(RuntimeSessionState::Error),
        EventKind::TurnInterrupted => Some(RuntimeSessionState::Interrupted),
        _ => None,
    }
}

fn active_runtime_count(local_state: &NodeLocalState) -> i64 {
    active_runtime_ids(local_state).len() as i64
}

fn active_runtime_ids(local_state: &NodeLocalState) -> Vec<RuntimeSessionId> {
    let mut ids = local_state
        .runtime_states
        .iter()
        .filter(|(_, state)| is_active_runtime_state(**state))
        .map(|(runtime_session_id, _)| RuntimeSessionId::from(runtime_session_id.as_str()))
        .collect::<Vec<_>>();
    ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    ids
}

fn is_active_runtime_state(state: RuntimeSessionState) -> bool {
    matches!(
        state,
        RuntimeSessionState::Starting
            | RuntimeSessionState::Ready
            | RuntimeSessionState::Running
            | RuntimeSessionState::Blocked
            | RuntimeSessionState::Stopping
            | RuntimeSessionState::Interrupted
            | RuntimeSessionState::Resuming
    )
}

fn workspace_validation_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    placement_snapshot_events(
        config,
        command,
        placement_seqs,
        EventKind::WorkspaceValidated,
    )
}

fn resource_snapshot_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    placement_snapshot_events(
        config,
        command,
        placement_seqs,
        EventKind::ResourceSnapshotUpdated,
    )
}

fn placement_snapshot_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
    event_kind: EventKind,
) -> Vec<EventEnvelope> {
    let Some(project_placement_id) = command.project_placement_id.clone() else {
        return vec![];
    };
    let workspace_path = command_payload_str(command, "workspace_path").unwrap_or("");
    let display_name = command_payload_str(command, "display_name").unwrap_or("workspace");
    let snapshot = validate_command_workspace(config, display_name, workspace_path);
    let placement_id_payload = project_placement_id.as_str().to_owned();
    vec![placement_event_for_command(
        command,
        placement_seqs,
        project_placement_id,
        event_kind,
        serde_json::json!({
            "placement_id": placement_id_payload,
            "display_name": snapshot.display_name,
            "workspace_path": snapshot.workspace_path,
            "state": snapshot.state,
            "resource_badges": snapshot.resource_badges,
            "last_validated_at": snapshot.last_validated_at,
        }),
    )]
}

fn validate_command_workspace(
    config: &NodeConfig,
    display_name: &str,
    workspace_path: &str,
) -> WorkspaceSnapshot {
    let path = Path::new(workspace_path);
    if !workspace_path_allowed(config, path) {
        return WorkspaceSnapshot {
            display_name: display_name.to_owned(),
            workspace_path: workspace_path.to_owned(),
            state: PlacementState::Error,
            resource_badges: vec![ResourceBadge {
                kind: "workspace_outside_allowed_roots".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace outside allowed roots".to_owned(),
            }],
            last_validated_at: Utc::now(),
        };
    }

    let mut snapshot = validate_workspace(path);
    snapshot.display_name = display_name.to_owned();
    snapshot
}

fn workspace_path_allowed(config: &NodeConfig, path: &Path) -> bool {
    config.workspace_paths.is_empty()
        || config
            .workspace_paths
            .iter()
            .any(|root| path.starts_with(root))
}

fn placement_event_for_command(
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
    project_placement_id: ProjectPlacementId,
    kind: EventKind,
    payload: serde_json::Value,
) -> EventEnvelope {
    let seq = next_placement_seq(placement_seqs, &project_placement_id);
    EventEnvelope {
        event_id: EventId::new(),
        command_id: Some(command.command_id.clone()),
        correlation_id: Some(command.correlation_id.clone()),
        actor_ref: ActorRef::Node {
            node_id: command.target_node_id.clone(),
        },
        scope_ref: ScopeRef::Placement {
            project_placement_id,
        },
        node_id: Some(command.target_node_id.clone()),
        runtime_session_id: None,
        session_thread_id: None,
        turn_id: None,
        seq,
        kind,
        happened_at: Utc::now(),
        source_refs: command.source_refs.clone(),
        evidence_refs: vec![],
        cause_refs: command.cause_refs.clone(),
        result_refs: vec![],
        payload: JsonValue(payload),
    }
}

fn next_placement_seq(
    placement_seqs: &mut HashMap<String, i64>,
    project_placement_id: &ProjectPlacementId,
) -> i64 {
    let entry = placement_seqs
        .entry(project_placement_id.to_string())
        .and_modify(|seq| *seq += 1)
        .or_insert(1);
    *entry
}

#[derive(Debug, Clone)]
enum RuntimeManager {
    Fake(FakeProviderAdapter),
    Codex(CodexProviderAdapter),
    Unsupported(UnsupportedProviderAdapter),
}

impl RuntimeManager {
    fn for_provider(provider_key: &str, config: &NodeConfig) -> Self {
        match provider_key {
            "" | "fake" => Self::Fake(FakeProviderAdapter),
            "codex" => Self::Codex(CodexProviderAdapter::new(config)),
            other => Self::Unsupported(UnsupportedProviderAdapter {
                provider_key: other.to_owned(),
            }),
        }
    }

    async fn execute_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
    ) -> Vec<EventEnvelope> {
        match self {
            Self::Fake(provider) => provider.events_for_command(command, runtime_seqs),
            Self::Codex(provider) => {
                provider
                    .events_for_command(
                        command,
                        runtime_seqs,
                        workspace_path,
                        runtime_transcripts,
                        runtime_provider_resume_refs,
                    )
                    .await
            }
            Self::Unsupported(provider) => provider.events_for_command(command, runtime_seqs),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FakeProviderAdapter;

impl FakeProviderAdapter {
    fn provider_key(&self) -> &'static str {
        "fake"
    }

    fn events_for_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
    ) -> Vec<EventEnvelope> {
        let Some(runtime_session_id) = command.runtime_session_id.clone() else {
            return vec![];
        };
        match command.kind {
            CommandKind::StartRuntime => vec![
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id.clone(),
                    None,
                    EventKind::RuntimeStarting,
                    serde_json::json!({ "provider": self.provider_key() }),
                ),
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    None,
                    EventKind::RuntimeReady,
                    serde_json::json!({ "provider": self.provider_key() }),
                ),
            ],
            CommandKind::ResumeRuntime => vec![
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id.clone(),
                    None,
                    EventKind::RuntimeResuming,
                    serde_json::json!({ "provider": self.provider_key() }),
                ),
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    None,
                    EventKind::RuntimeReady,
                    serde_json::json!({ "provider": self.provider_key() }),
                ),
            ],
            CommandKind::SendTurn => {
                let turn_id = command
                    .payload
                    .0
                    .get("turn_id")
                    .and_then(serde_json::Value::as_str)
                    .map(TurnId::from);
                let content = command
                    .payload
                    .0
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let mut events = vec![
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        None,
                        EventKind::RuntimeRunning,
                        serde_json::json!({ "provider": self.provider_key() }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        turn_id.clone(),
                        EventKind::TurnStarted,
                        serde_json::json!({}),
                    ),
                ];
                let trimmed_content = content.trim();
                if trimmed_content.starts_with("/approval") {
                    let approval_id = ApprovalId::new();
                    let prompt = trimmed_content
                        .strip_prefix("/approval")
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("Fake provider approval requested");
                    events.push(event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        turn_id,
                        EventKind::ApprovalRequested,
                        serde_json::json!({
                            "approval_id": approval_id.as_str(),
                            "prompt": prompt,
                        }),
                    ));
                    events.push(event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeBlocked,
                        serde_json::json!({ "provider": self.provider_key() }),
                    ));
                    return events;
                }
                if trimmed_content.starts_with("/error") {
                    let message = trimmed_content
                        .strip_prefix("/error")
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("Fake provider runtime error");
                    events.push(event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        turn_id,
                        EventKind::RuntimeError,
                        serde_json::json!({ "message": message }),
                    ));
                    return events;
                }
                events.extend([
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        turn_id.clone(),
                        EventKind::ProviderOutputDelta,
                        serde_json::json!({ "delta": format!("Fake provider accepted: {content}") }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        turn_id.clone(),
                        EventKind::ProviderMessageCompleted,
                        serde_json::json!({ "content": format!("Fake provider accepted: {content}") }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        turn_id,
                        EventKind::TurnCompleted,
                        serde_json::json!({}),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeReady,
                        serde_json::json!({ "provider": self.provider_key() }),
                    ),
                ]);
                events
            }
            CommandKind::ResolveApproval => {
                let approved = command
                    .payload
                    .0
                    .get("approved")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let approval_id = command
                    .payload
                    .0
                    .get("approval_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let default_message = if approved {
                    "Approval accepted"
                } else {
                    "Approval denied"
                };
                let message = command
                    .payload
                    .0
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(default_message);
                vec![
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        None,
                        EventKind::ApprovalResolved,
                        serde_json::json!({
                            "approval_id": approval_id,
                            "approved": approved,
                            "message": message,
                        }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeReady,
                        serde_json::json!({ "provider": self.provider_key() }),
                    ),
                ]
            }
            CommandKind::InterruptRuntime => vec![
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id.clone(),
                    None,
                    EventKind::TurnInterrupted,
                    serde_json::json!({}),
                ),
                event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    None,
                    EventKind::RuntimeReady,
                    serde_json::json!({ "provider": self.provider_key() }),
                ),
            ],
            CommandKind::StopRuntime => vec![event_for_command(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id,
                None,
                EventKind::RuntimeStopped,
                serde_json::json!({}),
            )],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct CodexProviderAdapter {
    codex_binary: String,
    timeout: Duration,
}

#[derive(Debug, Clone)]
struct ProviderStartFailure {
    code: &'static str,
    message: String,
}

impl ProviderStartFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl CodexProviderAdapter {
    fn new(config: &NodeConfig) -> Self {
        Self {
            codex_binary: config.codex_binary.clone(),
            timeout: config.codex_timeout,
        }
    }

    fn provider_key(&self) -> &'static str {
        "codex"
    }

    async fn events_for_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
    ) -> Vec<EventEnvelope> {
        let Some(runtime_session_id) = command.runtime_session_id.clone() else {
            return vec![];
        };
        match command.kind {
            CommandKind::StartRuntime => {
                runtime_transcripts.insert(runtime_session_id.to_string(), Vec::new());
                runtime_provider_resume_refs.remove(runtime_session_id.as_str());
                vec![
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        None,
                        EventKind::RuntimeStarting,
                        serde_json::json!({ "provider": self.provider_key(), "mode": "exec" }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeReady,
                        serde_json::json!({
                            "provider": self.provider_key(),
                            "mode": "exec",
                            "resume_source": "node_local_transcript",
                        }),
                    ),
                ]
            }
            CommandKind::ResumeRuntime => {
                let transcript_len = runtime_transcripts
                    .get(runtime_session_id.as_str())
                    .map(Vec::len)
                    .unwrap_or(0);
                let provider_resume_ref = command_provider_resume_ref(command).or_else(|| {
                    runtime_provider_resume_refs
                        .get(runtime_session_id.as_str())
                        .cloned()
                });
                if let Some(provider_resume_ref) = provider_resume_ref {
                    runtime_provider_resume_refs
                        .insert(runtime_session_id.to_string(), provider_resume_ref.clone());
                    return vec![
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id.clone(),
                            None,
                            EventKind::RuntimeResuming,
                            serde_json::json!({
                                "provider": self.provider_key(),
                                "mode": "exec",
                                "resume_source": "provider_resume_ref",
                            }),
                        ),
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id,
                            None,
                            EventKind::RuntimeReady,
                            serde_json::json!({
                                "provider": self.provider_key(),
                                "mode": "exec",
                                "resume_source": "provider_resume_ref",
                                "provider_resume_ref": provider_resume_ref_json(&provider_resume_ref),
                            }),
                        ),
                    ];
                }
                vec![
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        None,
                        EventKind::RuntimeResuming,
                        serde_json::json!({ "provider": self.provider_key(), "mode": "exec" }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeReady,
                        serde_json::json!({
                            "provider": self.provider_key(),
                            "mode": "exec",
                            "resume_source": "node_local_transcript",
                            "transcript_messages": transcript_len,
                        }),
                    ),
                ]
            }
            CommandKind::SendTurn => {
                self.send_turn_events(
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    workspace_path,
                    runtime_transcripts,
                    runtime_provider_resume_refs,
                )
                .await
            }
            CommandKind::ResolveApproval => {
                let approved = command
                    .payload
                    .0
                    .get("approved")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let approval_id = command
                    .payload
                    .0
                    .get("approval_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let default_message = if approved {
                    "Approval accepted"
                } else {
                    "Approval denied"
                };
                let message = command
                    .payload
                    .0
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(default_message);
                vec![
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id.clone(),
                        None,
                        EventKind::ApprovalResolved,
                        serde_json::json!({
                            "approval_id": approval_id,
                            "approved": approved,
                            "message": message,
                        }),
                    ),
                    event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeReady,
                        serde_json::json!({ "provider": self.provider_key(), "mode": "exec" }),
                    ),
                ]
            }
            CommandKind::InterruptRuntime => vec![event_for_command(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id,
                None,
                EventKind::RuntimeError,
                serde_json::json!({
                    "code": "provider.interrupt_unsupported",
                    "message": "Codex interrupt is not supported by the stateless exec adapter yet",
                }),
            )],
            CommandKind::StopRuntime => vec![event_for_command(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id,
                None,
                EventKind::RuntimeStopped,
                serde_json::json!({ "provider": self.provider_key(), "mode": "exec" }),
            )],
            _ => vec![],
        }
    }

    async fn send_turn_events(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        runtime_session_id: RuntimeSessionId,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
    ) -> Vec<EventEnvelope> {
        let turn_id = command
            .payload
            .0
            .get("turn_id")
            .and_then(serde_json::Value::as_str)
            .map(TurnId::from);
        let content = command
            .payload
            .0
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let mut events = vec![
            event_for_command(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id.clone(),
                None,
                EventKind::RuntimeRunning,
                serde_json::json!({ "provider": self.provider_key(), "mode": "exec" }),
            ),
            event_for_command(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id.clone(),
                turn_id.clone(),
                EventKind::TurnStarted,
                serde_json::json!({}),
            ),
        ];

        let Some(workspace_path) = workspace_path.filter(|value| !value.trim().is_empty()) else {
            events.push(runtime_error_event(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id,
                turn_id,
                "provider.workspace_missing",
                "Codex provider requires a workspace path from StartRuntime",
            ));
            return events;
        };

        let last_message_path = codex_last_message_path(&command.command_id);
        let provider_resume_ref = runtime_provider_resume_refs
            .get(runtime_session_id.as_str())
            .cloned();
        let output = if let Some(provider_session_id) = provider_resume_ref
            .as_ref()
            .and_then(|resume_ref| resume_ref.provider_session_id.as_deref())
        {
            self.run_codex_exec_resume(
                workspace_path,
                provider_session_id,
                content,
                &last_message_path,
            )
            .await
        } else {
            let prompt = codex_exec_prompt(
                content,
                runtime_transcripts
                    .get(runtime_session_id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            );
            self.run_codex_exec(workspace_path, &prompt, &last_message_path)
                .await
        };
        let last_message = std::fs::read_to_string(&last_message_path).unwrap_or_default();
        let _ = std::fs::remove_file(&last_message_path);

        match output {
            Ok(output) if output.status.success() => {
                let approval_requests = codex_approval_requests_from_output(&output);
                let provider_resume_ref = codex_resume_ref_from_output(&output);
                if let Some(provider_resume_ref) = provider_resume_ref
                    .as_ref()
                    .and_then(provider_resume_ref_from_json)
                {
                    runtime_provider_resume_refs
                        .insert(runtime_session_id.to_string(), provider_resume_ref);
                }
                if !approval_requests.is_empty() {
                    for approval_request in approval_requests {
                        events.push(event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id.clone(),
                            turn_id.clone(),
                            EventKind::ApprovalRequested,
                            serde_json::json!({
                                "approval_id": approval_request.approval_id.as_str(),
                                "prompt": approval_request.prompt,
                                "provider": self.provider_key(),
                                "provider_event_type": approval_request.provider_event_type,
                                "source": "codex_json_stdout",
                            }),
                        ));
                    }
                    events.push(event_for_command(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        EventKind::RuntimeBlocked,
                        serde_json::json!({
                            "provider": self.provider_key(),
                            "mode": "exec",
                            "reason": "provider_approval_requested",
                        }),
                    ));
                    return events;
                }

                let assistant_content = last_message.trim();
                if assistant_content.is_empty() {
                    events.push(runtime_error_event(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        turn_id,
                        "provider.empty_output",
                        "Codex exec completed without a final assistant message",
                    ));
                } else {
                    record_codex_transcript_turn(
                        runtime_transcripts,
                        &runtime_session_id,
                        content,
                        assistant_content,
                    );
                    events.extend([
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id.clone(),
                            turn_id.clone(),
                            EventKind::ProviderOutputDelta,
                            serde_json::json!({ "delta": assistant_content }),
                        ),
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id.clone(),
                            turn_id.clone(),
                            EventKind::ProviderMessageCompleted,
                            serde_json::json!({ "content": assistant_content }),
                        ),
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id.clone(),
                            turn_id,
                            EventKind::TurnCompleted,
                            serde_json::json!({}),
                        ),
                        event_for_command(
                            self.provider_key(),
                            command,
                            runtime_seqs,
                            runtime_session_id,
                            None,
                            EventKind::RuntimeReady,
                            codex_runtime_ready_payload(self.provider_key(), provider_resume_ref),
                        ),
                    ]);
                }
            }
            Ok(output) => {
                events.push(runtime_error_event(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    turn_id,
                    "provider.exec_failed",
                    codex_failure_message(&output),
                ));
            }
            Err(error) => {
                events.push(runtime_error_event(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    turn_id,
                    error.code,
                    error.message,
                ));
            }
        }
        events
    }

    async fn run_codex_exec(
        &self,
        workspace_path: &str,
        content: &str,
        last_message_path: &Path,
    ) -> Result<Output, ProviderStartFailure> {
        let mut command = TokioCommand::new(&self.codex_binary);
        command
            .arg("exec")
            .arg("--cd")
            .arg(workspace_path)
            .arg("--json")
            .arg("--output-last-message")
            .arg(last_message_path)
            .arg(content)
            .current_dir(workspace_path)
            .kill_on_drop(true);

        match timeout(self.timeout, command.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                let code = if error.kind() == ErrorKind::NotFound {
                    "provider.missing_binary"
                } else {
                    "provider.start_failed"
                };
                Err(ProviderStartFailure::new(
                    code,
                    format!(
                        "Codex exec could not start using `{}`: {error}",
                        self.codex_binary
                    ),
                ))
            }
            Err(_) => Err(ProviderStartFailure::new(
                "provider.start_timeout",
                format!(
                    "Codex exec timed out after {} seconds",
                    self.timeout.as_secs()
                ),
            )),
        }
    }

    async fn run_codex_exec_resume(
        &self,
        workspace_path: &str,
        provider_session_id: &str,
        content: &str,
        last_message_path: &Path,
    ) -> Result<Output, ProviderStartFailure> {
        let mut command = TokioCommand::new(&self.codex_binary);
        command
            .arg("exec")
            .arg("resume")
            .arg("--json")
            .arg("--output-last-message")
            .arg(last_message_path)
            .arg(provider_session_id)
            .arg(content)
            .current_dir(workspace_path)
            .kill_on_drop(true);

        match timeout(self.timeout, command.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                let code = if error.kind() == ErrorKind::NotFound {
                    "provider.missing_binary"
                } else {
                    "provider.start_failed"
                };
                Err(ProviderStartFailure::new(
                    code,
                    format!(
                        "Codex exec resume could not start using `{}`: {error}",
                        self.codex_binary
                    ),
                ))
            }
            Err(_) => Err(ProviderStartFailure::new(
                "provider.start_timeout",
                format!(
                    "Codex exec resume timed out after {} seconds",
                    self.timeout.as_secs()
                ),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexApprovalRequest {
    approval_id: ApprovalId,
    prompt: String,
    provider_event_type: Option<String>,
}

fn codex_approval_requests_from_output(output: &Output) -> Vec<CodexApprovalRequest> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(codex_approval_request_from_json_line)
        .collect()
}

fn codex_approval_request_from_json_line(line: &str) -> Option<CodexApprovalRequest> {
    let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
    if !codex_json_is_approval_request(&value) {
        return None;
    }
    let provider_event_type = top_level_json_string(&value, &["type", "event", "kind"]);
    let approval_id = first_json_string_for_keys(&value, &["approval_id", "request_id", "id"])
        .filter(|value| !value.trim().is_empty())
        .map(ApprovalId::from)
        .unwrap_or_default();
    let prompt = first_json_string_for_keys(
        &value,
        &["prompt", "message", "question", "reason", "description"],
    )
    .map(|value| bounded_text(value.trim(), 1200))
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "Codex requested approval".to_owned());

    Some(CodexApprovalRequest {
        approval_id,
        prompt,
        provider_event_type,
    })
}

fn codex_json_is_approval_request(value: &serde_json::Value) -> bool {
    let Some(event_name) = top_level_json_string(value, &["type", "event", "kind"]) else {
        return first_json_string_for_keys(value, &["approval_id"]).is_some()
            && first_json_string_for_keys(value, &["prompt", "message", "question"]).is_some();
    };
    let normalized = event_name.to_ascii_lowercase();
    (normalized.contains("approval")
        && (normalized.contains("request") || normalized.contains("requested")))
        || (normalized.contains("user")
            && normalized.contains("input")
            && normalized.contains("request"))
}

fn top_level_json_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

fn first_json_string_for_keys(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => {
            for key in keys {
                if let Some(text) = object.get(*key).and_then(serde_json::Value::as_str) {
                    return Some(text.to_owned());
                }
            }
            object
                .values()
                .find_map(|nested| first_json_string_for_keys(nested, keys))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|nested| first_json_string_for_keys(nested, keys)),
        _ => None,
    }
}

fn codex_resume_ref_from_output(output: &Output) -> Option<serde_json::Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|value| {
            let mut resume_ref = serde_json::Map::new();
            if let Some(session_id) = first_json_string_for_keys(
                &value,
                &[
                    "provider_session_id",
                    "session_id",
                    "conversation_id",
                    "thread_id",
                ],
            )
            .filter(|value| !value.trim().is_empty())
            {
                resume_ref.insert(
                    "provider_session_id".to_owned(),
                    serde_json::Value::String(bounded_text(session_id.trim(), 512)),
                );
            }
            if let Some(cursor) = first_json_string_for_keys(&value, &["resume_cursor", "cursor"])
                .filter(|value| !value.trim().is_empty())
            {
                resume_ref.insert(
                    "resume_cursor".to_owned(),
                    serde_json::Value::String(bounded_text(cursor.trim(), 512)),
                );
            }
            if resume_ref.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(resume_ref))
            }
        })
}

fn codex_runtime_ready_payload(
    provider_key: &str,
    provider_resume_ref: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "provider": provider_key,
        "mode": "exec",
    });
    if let Some(provider_resume_ref) = provider_resume_ref {
        payload["provider_resume_ref"] = provider_resume_ref;
    }
    payload
}

fn codex_exec_prompt(content: &str, transcript: &[ProviderTranscriptMessage]) -> String {
    let mut selected = Vec::new();
    let mut context_chars = 0usize;
    for message in transcript.iter().rev() {
        let message_chars = message.role.chars().count() + message.content.chars().count() + 4;
        if !selected.is_empty()
            && context_chars + message_chars + content.chars().count() > MAX_CODEX_TRANSCRIPT_CHARS
        {
            break;
        }
        selected.push(message);
        context_chars += message_chars;
        if selected.len() >= MAX_CODEX_TRANSCRIPT_MESSAGES {
            break;
        }
    }
    selected.reverse();

    if selected.is_empty() {
        return content.to_owned();
    }

    let mut prompt = String::from(
        "Continue this Cortex session. Use the transcript only as prior context, then answer the latest user message.\n\nTranscript:\n",
    );
    for message in selected {
        prompt.push_str(&message.role);
        prompt.push_str(": ");
        prompt.push_str(&message.content);
        prompt.push('\n');
    }
    prompt.push_str("\nLatest user message:\n");
    prompt.push_str(content);
    prompt
}

fn record_codex_transcript_turn(
    runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
    runtime_session_id: &RuntimeSessionId,
    user_content: &str,
    assistant_content: &str,
) {
    let transcript = runtime_transcripts
        .entry(runtime_session_id.to_string())
        .or_default();
    transcript.push(ProviderTranscriptMessage {
        role: "user".to_owned(),
        content: user_content.to_owned(),
    });
    transcript.push(ProviderTranscriptMessage {
        role: "assistant".to_owned(),
        content: assistant_content.to_owned(),
    });
    trim_codex_transcript(transcript);
}

fn trim_codex_transcript(transcript: &mut Vec<ProviderTranscriptMessage>) {
    if transcript.len() > MAX_CODEX_TRANSCRIPT_MESSAGES {
        let overflow = transcript.len() - MAX_CODEX_TRANSCRIPT_MESSAGES;
        transcript.drain(0..overflow);
    }
    while transcript.len() > 2
        && transcript
            .iter()
            .map(|message| message.role.chars().count() + message.content.chars().count())
            .sum::<usize>()
            > MAX_CODEX_TRANSCRIPT_CHARS
    {
        transcript.remove(0);
    }
}

#[derive(Debug, Clone)]
struct UnsupportedProviderAdapter {
    provider_key: String,
}

impl UnsupportedProviderAdapter {
    fn events_for_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
    ) -> Vec<EventEnvelope> {
        let Some(runtime_session_id) = command.runtime_session_id.clone() else {
            return vec![];
        };
        vec![event_for_command(
            &self.provider_key,
            command,
            runtime_seqs,
            runtime_session_id,
            None,
            EventKind::RuntimeError,
            serde_json::json!({
                "code": "provider.unsupported",
                "message": format!("Provider `{}` is not supported by this node", self.provider_key),
            }),
        )]
    }
}

fn runtime_error_event(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    code: &str,
    message: impl Into<String>,
) -> EventEnvelope {
    event_for_command(
        provider_key,
        command,
        runtime_seqs,
        runtime_session_id,
        turn_id,
        EventKind::RuntimeError,
        serde_json::json!({
            "code": code,
            "message": message.into(),
        }),
    )
}

fn codex_last_message_path(command_id: &CommandId) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cortex-codex-{}-{}.txt",
        sanitize_filename_segment(command_id.as_str()),
        Uuid::new_v4()
    ))
}

fn sanitize_filename_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn codex_failure_message(output: &Output) -> String {
    let status = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated_by_signal".to_owned());
    let stderr = bounded_text(&String::from_utf8_lossy(&output.stderr), 1200);
    if !stderr.trim().is_empty() {
        return format!("Codex exec failed with status {status}: {}", stderr.trim());
    }
    let stdout = bounded_text(&String::from_utf8_lossy(&output.stdout), 1200);
    if !stdout.trim().is_empty() {
        return format!("Codex exec failed with status {status}: {}", stdout.trim());
    }
    format!("Codex exec failed with status {status}")
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    let mut text = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        text.push_str("...");
    }
    text
}

fn event_for_command(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    kind: EventKind,
    payload: serde_json::Value,
) -> EventEnvelope {
    let seq = next_runtime_seq(runtime_seqs, &runtime_session_id);
    EventEnvelope {
        event_id: EventId::new(),
        command_id: Some(command.command_id.clone()),
        correlation_id: Some(command.correlation_id.clone()),
        actor_ref: ActorRef::Provider {
            provider: provider_key.to_owned(),
        },
        scope_ref: ScopeRef::Runtime {
            runtime_session_id: runtime_session_id.clone(),
        },
        node_id: Some(command.target_node_id.clone()),
        runtime_session_id: Some(runtime_session_id),
        session_thread_id: command.session_thread_id.clone(),
        turn_id,
        seq,
        kind,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: command.cause_refs.clone(),
        result_refs: vec![],
        payload: JsonValue(payload),
    }
}

fn next_runtime_seq(
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: &RuntimeSessionId,
) -> i64 {
    let entry = runtime_seqs
        .entry(runtime_session_id.to_string())
        .and_modify(|seq| *seq += 1)
        .or_insert(1);
    *entry
}

fn control_url(core_url: &Url) -> anyhow::Result<Url> {
    let mut url = core_url
        .join("/api/v1/node/control")
        .context("control URL should be valid")?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => anyhow::bail!("unsupported Core URL scheme `{other}`"),
    };
    url.set_scheme(scheme)
        .map_err(|_| anyhow::anyhow!("failed to set control URL scheme"))?;
    Ok(url)
}

fn capabilities(config: &NodeConfig) -> Vec<CapabilitySummary> {
    let codex_available = command_available(&config.codex_binary);
    vec![
        CapabilitySummary {
            key: "provider.fake".to_owned(),
            value: JsonValue(serde_json::json!({ "available": true })),
        },
        CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: JsonValue(serde_json::json!({
                "available": codex_available,
                "configured": true,
                "binary": config.codex_binary.as_str(),
                "mode": "exec",
                "timeout_seconds": config.codex_timeout.as_secs(),
                "unavailable_reason": if codex_available { serde_json::Value::Null } else { serde_json::json!("binary_not_found") },
            })),
        },
        CapabilitySummary {
            key: "workspace.validation".to_owned(),
            value: JsonValue(serde_json::json!({ "mode": "explicit_path" })),
        },
    ]
}

fn command_available(binary: &str) -> bool {
    let path = Path::new(binary);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|directory| directory.join(binary).is_file())
}

fn validate_workspace(path: &Path) -> WorkspaceSnapshot {
    let display_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workspace")
        .to_owned();
    let (state, resource_badges) = if !path.exists() {
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
        (PlacementState::Validated, resource_warnings(path))
    };

    WorkspaceSnapshot {
        display_name,
        workspace_path: path.display().to_string(),
        state,
        resource_badges,
        last_validated_at: chrono::Utc::now(),
    }
}

fn resource_warnings(path: &Path) -> Vec<ResourceBadge> {
    let mut badges = Vec::new();
    if !path.join(".git").exists() {
        return badges;
    }

    badges.push(ResourceBadge {
        kind: "git_workspace".to_owned(),
        severity: WarningSeverity::Info,
        label: "Git workspace".to_owned(),
    });
    if let Some(status) = git_status_snapshot(path) {
        badges.extend(git_status_badges(&status));
    } else {
        badges.push(ResourceBadge {
            kind: "git_snapshot_unavailable".to_owned(),
            severity: WarningSeverity::Warning,
            label: "Git snapshot unavailable".to_owned(),
        });
    }
    badges
}

fn git_status_snapshot(path: &Path) -> Option<String> {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--branch")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_status_badges(status: &str) -> Vec<ResourceBadge> {
    let mut branch = None;
    let mut dirty_count = 0usize;
    let mut untracked_count = 0usize;
    let mut ahead_count = 0usize;
    let mut behind_count = 0usize;

    for line in status.lines() {
        if let Some(summary) = line.strip_prefix("## ") {
            branch = git_branch_label(summary);
            ahead_count = git_tracking_count(summary, "ahead");
            behind_count = git_tracking_count(summary, "behind");
        } else if !line.trim().is_empty() {
            dirty_count += 1;
            if line.starts_with("?? ") {
                untracked_count += 1;
            }
        }
    }

    let mut badges = Vec::new();
    if let Some(branch) = branch {
        badges.push(ResourceBadge {
            kind: "git_branch".to_owned(),
            severity: WarningSeverity::Info,
            label: format!("Git branch: {branch}"),
        });
    }
    if dirty_count > 0 {
        badges.push(ResourceBadge {
            kind: "dirty_workspace".to_owned(),
            severity: WarningSeverity::Warning,
            label: dirty_workspace_label(dirty_count, untracked_count),
        });
    }
    if behind_count > 0 {
        badges.push(ResourceBadge {
            kind: "branch_behind".to_owned(),
            severity: WarningSeverity::Warning,
            label: format!("Branch is behind upstream by {behind_count} commit(s)"),
        });
    }
    if ahead_count > 0 {
        badges.push(ResourceBadge {
            kind: "branch_ahead".to_owned(),
            severity: WarningSeverity::Info,
            label: format!("Branch is ahead of upstream by {ahead_count} commit(s)"),
        });
    }
    badges
}

fn git_branch_label(summary: &str) -> Option<String> {
    let without_upstream = summary.split("...").next().unwrap_or(summary);
    let without_tracking = without_upstream
        .split(" [")
        .next()
        .unwrap_or(without_upstream)
        .trim();
    (!without_tracking.is_empty()).then(|| without_tracking.to_owned())
}

fn git_tracking_count(summary: &str, key: &str) -> usize {
    let Some((_, tracking)) = summary.split_once('[') else {
        return 0;
    };
    let tracking = tracking.strip_suffix(']').unwrap_or(tracking);
    tracking
        .split(',')
        .map(str::trim)
        .find_map(|part| {
            part.strip_prefix(key)
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0)
}

fn dirty_workspace_label(dirty_count: usize, untracked_count: usize) -> String {
    if untracked_count == 0 {
        return format!("Git workspace has {dirty_count} changed path(s)");
    }
    format!(
        "Git workspace has {dirty_count} changed path(s), including {untracked_count} untracked"
    )
}

fn default_state_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".local")
        .join("share")
        .join("cortex-node")
        .join("node.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_protocol::{CorrelationId, SessionThreadId};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    const NODE_CONFIG_ENV_VARS: &[&str] = &[
        "CORTEX_CORE_URL",
        "CORTEX_NODE_DISPLAY_NAME",
        "CORTEX_NODE_HEARTBEAT_SECONDS",
        "CORTEX_NODE_STATE_PATH",
        "CORTEX_NODE_WORKSPACES",
        "CORTEX_CODEX_BINARY",
        "CORTEX_CODEX_TIMEOUT_SECONDS",
    ];

    #[test]
    fn node_local_state_loads_legacy_state_without_reliability_fields() {
        let path = std::env::temp_dir().join(format!("cortex-node-{}.json", Uuid::new_v4()));
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
    fn node_local_state_preserves_daemon_installation_id_after_save() {
        let path = std::env::temp_dir().join(format!("cortex-node-{}.json", Uuid::new_v4()));
        let local_state = NodeLocalState::default();
        let installation_id = local_state.daemon_installation_id.clone();
        local_state.save(&path).expect("node state saves");

        let reloaded = NodeLocalState::load(&path).expect("node state reloads");
        std::fs::remove_file(path).expect("node state fixture is removed");

        assert_eq!(reloaded.daemon_installation_id, installation_id);
    }

    #[test]
    fn node_diagnostics_reports_installation_and_queue_counts() {
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
            "daemon_installation_id=daemon-test; outbox_events=0; cached_commands=1"
        );
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
    fn node_config_from_env_uses_documented_defaults() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);

        let config = NodeConfig::from_env().expect("default node config parses");

        assert_eq!(config.core_url.as_str(), "http://127.0.0.1:8080/");
        assert_eq!(config.display_name, "Local Node");
        assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
        assert!(config.workspace_paths.is_empty());
        assert_eq!(config.codex_binary, "codex");
        assert_eq!(config.codex_timeout, Duration::from_secs(120));
    }

    #[test]
    fn node_config_from_env_parses_overrides_and_workspace_list() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);
        let state_path = std::env::temp_dir().join(format!("cortex-node-{}.json", Uuid::new_v4()));
        std::env::set_var("CORTEX_CORE_URL", "http://127.0.0.1:19090");
        std::env::set_var("CORTEX_NODE_DISPLAY_NAME", "Desktop Node");
        std::env::set_var("CORTEX_NODE_HEARTBEAT_SECONDS", "2");
        std::env::set_var("CORTEX_NODE_STATE_PATH", &state_path);
        std::env::set_var("CORTEX_NODE_WORKSPACES", "/tmp/a, ,/tmp/b");
        std::env::set_var("CORTEX_CODEX_BINARY", "/usr/local/bin/codex");
        std::env::set_var("CORTEX_CODEX_TIMEOUT_SECONDS", "7");

        let config = NodeConfig::from_env().expect("overridden node config parses");

        assert_eq!(config.core_url.as_str(), "http://127.0.0.1:19090/");
        assert_eq!(config.display_name, "Desktop Node");
        assert_eq!(config.heartbeat_interval, Duration::from_secs(2));
        assert_eq!(config.state_path, state_path);
        assert_eq!(
            config.workspace_paths,
            vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")]
        );
        assert_eq!(config.codex_binary, "/usr/local/bin/codex");
        assert_eq!(config.codex_timeout, Duration::from_secs(7));
    }

    #[test]
    fn node_config_from_env_rejects_invalid_duration_values() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);
        std::env::set_var("CORTEX_NODE_HEARTBEAT_SECONDS", "soon");

        let error = NodeConfig::from_env().expect_err("invalid heartbeat should fail");

        assert!(error
            .to_string()
            .contains("CORTEX_NODE_HEARTBEAT_SECONDS must be an unsigned integer"));
    }

    #[test]
    fn command_available_returns_false_for_missing_absolute_binary() {
        let missing = std::env::temp_dir().join(format!("missing-codex-{}", Uuid::new_v4()));

        assert!(!command_available(&missing.display().to_string()));
    }

    #[test]
    fn command_available_resolves_binary_from_path() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(&["PATH"]);
        let bin_dir = std::env::temp_dir().join(format!("cortex-node-bin-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&bin_dir).expect("bin dir creates");
        let codex_path = bin_dir.join("codex");
        std::fs::write(&codex_path, "").expect("codex fixture writes");
        std::env::set_var("PATH", &bin_dir);

        let available = command_available("codex");
        std::fs::remove_dir_all(bin_dir).expect("bin dir removes");

        assert!(available);
    }

    #[test]
    fn capabilities_report_codex_unavailable_when_binary_is_missing() {
        let config = config_fixture_with_codex_binary(
            std::env::temp_dir()
                .join(format!("missing-codex-{}", Uuid::new_v4()))
                .display()
                .to_string(),
        );

        let capability = capabilities(&config)
            .into_iter()
            .find(|capability| capability.key == "provider.codex")
            .expect("codex capability exists");

        assert_eq!(
            capability
                .value
                .0
                .get("available")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn compatible_control_frame_has_no_protocol_error() {
        let frame = ControlFrame::HelloAck {
            frame_id: "hello-ack-1".to_owned(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
        };

        assert!(control_frame_protocol_error(&frame).is_none());
    }

    #[test]
    fn incompatible_control_frame_builds_safe_protocol_error() {
        let frame = ControlFrame::CommandDispatch {
            frame_id: "dispatch-1".to_owned(),
            protocol_version: "v0".to_owned(),
            sent_at: Utc::now(),
            command: command_fixture("bad-protocol-command", CommandKind::SendTurn),
        };

        let error_frame = control_frame_protocol_error(&frame).expect("incompatible frame rejects");

        let ControlFrame::ControlError {
            protocol_version,
            error,
            ..
        } = error_frame
        else {
            panic!("expected control error frame");
        };
        assert_eq!(protocol_version, API_VERSION);
        assert_eq!(error.error_code, "control.protocol_incompatible");
        assert!(!error.retryable);
        assert_eq!(
            error
                .details
                .0
                .get("received_protocol_version")
                .and_then(serde_json::Value::as_str),
            Some("v0")
        );
    }

    #[tokio::test]
    async fn prepare_command_dispatch_persists_new_events_and_runtime_sequence() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Completed);
        assert!(outcome.state_changed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::ProviderOutputDelta,
                EventKind::ProviderMessageCompleted,
                EventKind::TurnCompleted,
                EventKind::RuntimeReady,
            ]
        );
        assert_eq!(local_state.event_outbox.len(), 6);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(6));
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Ready)
        );
        assert_eq!(active_runtime_count(&local_state), 1);
        assert_eq!(
            local_state.command_status.get("command-1").copied(),
            Some(CommandState::Completed)
        );
        assert!(outcome.events_to_send.iter().all(|event| event
            .correlation_id
            .as_ref()
            .is_some_and(|correlation_id| correlation_id.as_str() == "correlation-1")));
    }

    #[tokio::test]
    async fn prepare_command_dispatch_replays_outbox_for_duplicate_command() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_event_ids = event_ids(&first.events_to_send);

        let second = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(second.status, CommandState::Completed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(6));
        assert_eq!(local_state.event_outbox.len(), 6);
    }

    #[tokio::test]
    async fn node_local_state_replays_outbox_for_duplicate_command_after_restart() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let path = std::env::temp_dir().join(format!("cortex-node-{}.json", Uuid::new_v4()));
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_event_ids = event_ids(&first.events_to_send);
        local_state
            .save(&path)
            .expect("node state with outbox saves");

        let mut reloaded_state = NodeLocalState::load(&path).expect("node state reloads");
        let second = prepare_command_dispatch(&config, &mut reloaded_state, &command).await;
        std::fs::remove_file(path).expect("node state fixture is removed");

        assert_eq!(second.status, CommandState::Completed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(
            reloaded_state.runtime_seqs.get("runtime-1").copied(),
            Some(6)
        );
        assert_eq!(reloaded_state.event_outbox.len(), 6);
    }

    #[tokio::test]
    async fn remove_acked_events_removes_only_accepted_event_ids() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let accepted_event_id = outcome.events_to_send[1].event_id.clone();

        let removed = remove_acked_events(&mut local_state.event_outbox, &[accepted_event_id]);

        assert_eq!(removed, 1);
        assert_eq!(local_state.event_outbox.len(), 5);
        assert_eq!(event_ids(&local_state.event_outbox).len(), 5);
    }

    #[tokio::test]
    async fn event_outbox_retention_emits_runtime_error_when_runtime_events_are_dropped() {
        let config = config_fixture();
        let command = command_fixture("command-retention", CommandKind::SendTurn);
        let mut local_state = NodeLocalState {
            node_id: Some(NodeId::from("node-1")),
            ..NodeLocalState::default()
        };
        let _ = prepare_command_dispatch(&config, &mut local_state, &command).await;

        let notices = enforce_event_outbox_retention(&mut local_state, 5);

        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, EventKind::RuntimeError);
        assert_eq!(notices[0].seq, 7);
        assert_eq!(
            notices[0]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("node.event_outbox_retention_exceeded")
        );
        assert_eq!(local_state.event_outbox.len(), 5);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(7));
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[tokio::test]
    async fn fake_provider_approval_turn_blocks_runtime() {
        let config = config_fixture();
        let command = command_fixture_with_content(
            "command-approval",
            CommandKind::SendTurn,
            "/approval Allow test command",
        );
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::ApprovalRequested,
                EventKind::RuntimeBlocked,
            ]
        );
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(4));
    }

    #[tokio::test]
    async fn fake_provider_error_turn_marks_runtime_error() {
        let config = config_fixture();
        let command =
            command_fixture_with_content("command-error", CommandKind::SendTurn, "/error boom");
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError,
            ]
        );
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(3));
        assert_eq!(
            local_state.command_status.get("command-error").copied(),
            Some(CommandState::Failed)
        );
    }

    #[tokio::test]
    async fn failed_command_dispatch_replays_failed_status_and_outbox_for_duplicate_command() {
        let config = config_fixture();
        let command =
            command_fixture_with_content("command-error", CommandKind::SendTurn, "/error boom");
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_event_ids = event_ids(&first.events_to_send);

        let second = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(second.status, CommandState::Failed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(3));
    }

    #[tokio::test]
    async fn fake_provider_resolve_approval_returns_runtime_to_ready() {
        let config = config_fixture();
        let mut command = command_fixture("command-resolve", CommandKind::ResolveApproval);
        command.payload = JsonValue(serde_json::json!({
            "approval_id": "approval-1",
            "approved": true,
            "message": "approved"
        }));
        let mut local_state = NodeLocalState::default();
        local_state.runtime_transcripts.insert(
            "runtime-1".to_owned(),
            vec![ProviderTranscriptMessage {
                role: "user".to_owned(),
                content: "stale context".to_owned(),
            }],
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::ApprovalResolved, EventKind::RuntimeReady]
        );
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(2));
    }

    #[tokio::test]
    async fn stop_runtime_marks_runtime_inactive() {
        let config = config_fixture();
        let command = command_fixture("command-stop", CommandKind::StopRuntime);
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_states
            .insert("runtime-1".to_owned(), RuntimeSessionState::Ready);

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::RuntimeStopped]
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Stopped)
        );
        assert_eq!(active_runtime_count(&local_state), 0);
        assert!(active_runtime_ids(&local_state).is_empty());
    }

    #[test]
    fn active_runtime_ids_are_sorted_and_include_only_live_states() {
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_states
            .insert("runtime-b".to_owned(), RuntimeSessionState::Blocked);
        local_state
            .runtime_states
            .insert("runtime-stopped".to_owned(), RuntimeSessionState::Stopped);
        local_state
            .runtime_states
            .insert("runtime-a".to_owned(), RuntimeSessionState::Running);
        local_state
            .runtime_states
            .insert("runtime-error".to_owned(), RuntimeSessionState::Error);

        let ids = active_runtime_ids(&local_state);

        assert_eq!(
            ids.iter().map(RuntimeSessionId::as_str).collect::<Vec<_>>(),
            vec!["runtime-a", "runtime-b"]
        );
    }

    #[tokio::test]
    async fn validate_workspace_command_emits_placement_scoped_event() {
        let config = config_fixture();
        let workspace_path = std::env::temp_dir();
        let command = placement_command_fixture(
            "command-validate",
            "placement-1",
            "workspace",
            &workspace_path.display().to_string(),
        );
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::WorkspaceValidated]
        );
        assert!(matches!(
            &outcome.events_to_send[0].scope_ref,
            ScopeRef::Placement { .. }
        ));
        assert_eq!(
            outcome.events_to_send[0]
                .payload
                .0
                .get("state")
                .and_then(serde_json::Value::as_str),
            Some("validated")
        );
        assert_eq!(
            local_state.placement_seqs.get("placement-1").copied(),
            Some(1)
        );
        assert_eq!(
            outcome.events_to_send[0]
                .correlation_id
                .as_ref()
                .map(CorrelationId::as_str),
            Some("correlation-1")
        );
    }

    #[tokio::test]
    async fn validate_workspace_command_hard_blocks_outside_allowed_roots() {
        let mut config = config_fixture();
        config.workspace_paths =
            vec![std::env::temp_dir().join(format!("allowed-root-{}", Uuid::new_v4()))];
        let workspace_path = std::env::temp_dir().join(format!("outside-root-{}", Uuid::new_v4()));
        let command = placement_command_fixture(
            "command-validate-blocked",
            "placement-2",
            "workspace",
            &workspace_path.display().to_string(),
        );
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(
            outcome.events_to_send[0]
                .payload
                .0
                .get("state")
                .and_then(serde_json::Value::as_str),
            Some("error")
        );
        assert_eq!(
            outcome.events_to_send[0]
                .payload
                .0
                .get("resource_badges")
                .and_then(serde_json::Value::as_array)
                .and_then(|badges| badges.first())
                .and_then(|badge| badge.get("kind"))
                .and_then(serde_json::Value::as_str),
            Some("workspace_outside_allowed_roots")
        );
    }

    #[tokio::test]
    async fn refresh_resource_snapshot_command_emits_placement_scoped_event() {
        let config = config_fixture();
        let workspace_path = std::env::temp_dir();
        let mut command = placement_command_fixture(
            "command-refresh",
            "placement-refresh",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::RefreshResourceSnapshot;
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::ResourceSnapshotUpdated]
        );
        assert!(matches!(
            &outcome.events_to_send[0].scope_ref,
            ScopeRef::Placement { .. }
        ));
        assert_eq!(
            local_state.placement_seqs.get("placement-refresh").copied(),
            Some(1)
        );
    }

    #[test]
    fn git_status_badges_parse_branch_dirty_and_tracking_state() {
        let status = "## main...origin/main [ahead 1, behind 2]\n M src/main.rs\n?? notes.md\n";

        let badges = git_status_badges(status);

        assert_eq!(
            badge_kinds(&badges),
            vec![
                "git_branch",
                "dirty_workspace",
                "branch_behind",
                "branch_ahead",
            ]
        );
        assert_eq!(badges[0].label, "Git branch: main");
        assert_eq!(badges[1].severity, WarningSeverity::Warning);
        assert_eq!(badges[2].severity, WarningSeverity::Warning);
        assert_eq!(badges[3].severity, WarningSeverity::Info);
    }

    #[test]
    fn git_status_badges_report_clean_branch_without_dirty_warning() {
        let status = "## feature/runtime-controls...origin/feature/runtime-controls\n";

        let badges = git_status_badges(status);

        assert_eq!(badge_kinds(&badges), vec!["git_branch"]);
        assert_eq!(badges[0].label, "Git branch: feature/runtime-controls");
    }

    #[test]
    fn resource_warnings_ignore_non_git_workspace() {
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-non-git-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");

        let badges = resource_warnings(&workspace_path);
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert!(badges.is_empty());
    }

    #[tokio::test]
    async fn codex_start_runtime_records_provider_and_workspace_metadata() {
        let config = config_fixture();
        let workspace_path = std::env::temp_dir().display().to_string();
        let mut command = command_fixture("command-codex-start", CommandKind::StartRuntime);
        command.payload = JsonValue(serde_json::json!({
            "provider": "codex",
            "workspace_path": workspace_path.clone(),
        }));
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::RuntimeStarting, EventKind::RuntimeReady]
        );
        assert_eq!(
            local_state
                .runtime_providers
                .get("runtime-1")
                .map(String::as_str),
            Some("codex")
        );
        assert_eq!(
            local_state
                .runtime_workspace_paths
                .get("runtime-1")
                .map(String::as_str),
            Some(workspace_path.as_str())
        );
        assert_eq!(
            local_state
                .runtime_transcripts
                .get("runtime-1")
                .map(Vec::len),
            Some(0)
        );
    }

    #[tokio::test]
    async fn codex_send_turn_maps_missing_binary_to_missing_binary_error() {
        let missing_binary = std::env::temp_dir()
            .join(format!("missing-codex-{}", Uuid::new_v4()))
            .display()
            .to_string();
        let config = config_fixture_with_codex_binary(missing_binary);
        let command = command_fixture("command-codex-send", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state.runtime_workspace_paths.insert(
            "runtime-1".to_owned(),
            std::env::temp_dir().display().to_string(),
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.missing_binary")
        );
        assert_eq!(
            local_state
                .command_status
                .get("command-codex-send")
                .copied(),
            Some(CommandState::Failed)
        );
    }

    #[tokio::test]
    async fn codex_send_turn_maps_missing_workspace_to_workspace_missing_error() {
        let config = config_fixture();
        let command = command_fixture("command-codex-workspace-missing", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.workspace_missing")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_executes_binary_and_emits_completed_assistant_message() {
        let codex_binary = fake_codex_success_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command = command_fixture_with_content(
            "command-codex-success",
            CommandKind::SendTurn,
            "build status",
        );
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::ProviderOutputDelta,
                EventKind::ProviderMessageCompleted,
                EventKind::TurnCompleted,
                EventKind::RuntimeReady,
            ]
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("content")
                .and_then(serde_json::Value::as_str),
            Some("Codex fake accepted")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Ready)
        );
        assert_eq!(
            local_state
                .command_status
                .get("command-codex-success")
                .copied(),
            Some(CommandState::Completed)
        );
        assert_eq!(
            local_state
                .runtime_transcripts
                .get("runtime-1")
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            local_state
                .runtime_provider_resume_refs
                .get("runtime-1")
                .and_then(|resume_ref| resume_ref.provider_session_id.as_deref()),
            Some("codex-session-1")
        );
        assert_eq!(
            outcome.events_to_send[5]
                .payload
                .0
                .get("provider_resume_ref")
                .and_then(|value| value.get("provider_session_id"))
                .and_then(serde_json::Value::as_str),
            Some("codex-session-1")
        );
        assert_eq!(
            outcome.events_to_send[5]
                .payload
                .0
                .get("provider_resume_ref")
                .and_then(|value| value.get("resume_cursor"))
                .and_then(serde_json::Value::as_str),
            Some("cursor-1")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_includes_prior_transcript_context() {
        let capture_path =
            std::env::temp_dir().join(format!("cortex-codex-prompt-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_prompt_capture_binary(&capture_path);
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command = command_fixture_with_content(
            "command-codex-transcript",
            CommandKind::SendTurn,
            "second question",
        );
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());
        local_state.runtime_transcripts.insert(
            "runtime-1".to_owned(),
            vec![
                ProviderTranscriptMessage {
                    role: "user".to_owned(),
                    content: "first question".to_owned(),
                },
                ProviderTranscriptMessage {
                    role: "assistant".to_owned(),
                    content: "first answer".to_owned(),
                },
            ],
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        let captured_prompt =
            std::fs::read_to_string(&capture_path).expect("captured prompt reads");
        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_file(capture_path).expect("prompt capture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert!(captured_prompt.contains("Continue this Cortex session"));
        assert!(captured_prompt.contains("user: first question"));
        assert!(captured_prompt.contains("assistant: first answer"));
        assert!(captured_prompt.contains("Latest user message:\nsecond question"));
        let transcript = local_state
            .runtime_transcripts
            .get("runtime-1")
            .expect("runtime transcript exists");
        assert_eq!(transcript.len(), 4);
        assert_eq!(transcript[2].content, "second question");
        assert_eq!(transcript[3].content, "Codex contextual answer");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_uses_provider_native_resume_when_session_id_exists() {
        let capture_path =
            std::env::temp_dir().join(format!("cortex-codex-resume-args-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_resume_capture_binary(&capture_path);
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command = command_fixture_with_content(
            "command-codex-native-resume",
            CommandKind::SendTurn,
            "third question",
        );
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());
        local_state.runtime_provider_resume_refs.insert(
            "runtime-1".to_owned(),
            ProviderResumeRef {
                provider_session_id: Some("codex-session-1".to_owned()),
                resume_cursor: Some("cursor-1".to_owned()),
            },
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        let captured_args = std::fs::read_to_string(&capture_path).expect("captured args read");
        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_file(capture_path).expect("resume capture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert!(captured_args.contains("exec\nresume\n"));
        assert!(captured_args.contains("\ncodex-session-1\n"));
        assert!(captured_args.contains("\nthird question\n"));
        assert!(!captured_args.contains("Latest user message:"));
        assert_eq!(
            local_state
                .runtime_provider_resume_refs
                .get("runtime-1")
                .and_then(|resume_ref| resume_ref.resume_cursor.as_deref()),
            Some("cursor-2")
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("content")
                .and_then(serde_json::Value::as_str),
            Some("Codex resume accepted")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_maps_stdout_approval_request_to_blocked_runtime() {
        let codex_binary = fake_codex_approval_request_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command = command_fixture_with_content(
            "command-codex-approval",
            CommandKind::SendTurn,
            "change files",
        );
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::ApprovalRequested,
                EventKind::RuntimeBlocked,
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("approval_id")
                .and_then(serde_json::Value::as_str),
            Some("approval-codex-1")
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("prompt")
                .and_then(serde_json::Value::as_str),
            Some("Allow file edit?")
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("source")
                .and_then(serde_json::Value::as_str),
            Some("codex_json_stdout")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Blocked)
        );
        assert!(local_state
            .runtime_transcripts
            .get("runtime-1")
            .map_or(true, Vec::is_empty));
    }

    #[test]
    fn codex_approval_request_parser_accepts_nested_user_input_request() {
        let request = codex_approval_request_from_json_line(
            r#"{"type":"provider.user_input.requested","payload":{"request_id":"input-1","question":"Need confirmation"}}"#,
        )
        .expect("approval request parses");

        assert_eq!(request.approval_id.as_str(), "input-1");
        assert_eq!(request.prompt, "Need confirmation");
        assert_eq!(
            request.provider_event_type.as_deref(),
            Some("provider.user_input.requested")
        );
    }

    #[tokio::test]
    async fn codex_resume_runtime_reports_local_transcript_source() {
        let config = config_fixture_with_codex_binary("codex");
        let command = command_fixture("command-codex-resume", CommandKind::ResumeRuntime);
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state.runtime_transcripts.insert(
            "runtime-1".to_owned(),
            vec![
                ProviderTranscriptMessage {
                    role: "user".to_owned(),
                    content: "first question".to_owned(),
                },
                ProviderTranscriptMessage {
                    role: "assistant".to_owned(),
                    content: "first answer".to_owned(),
                },
            ],
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::RuntimeResuming, EventKind::RuntimeReady]
        );
        assert_eq!(
            outcome.events_to_send[1]
                .payload
                .0
                .get("resume_source")
                .and_then(serde_json::Value::as_str),
            Some("node_local_transcript")
        );
        assert_eq!(
            outcome.events_to_send[1]
                .payload
                .0
                .get("transcript_messages")
                .and_then(serde_json::Value::as_i64),
            Some(2)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_maps_nonzero_exit_to_exec_failed_error() {
        let codex_binary = fake_codex_failing_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command =
            command_fixture_with_content("command-codex-failed", CommandKind::SendTurn, "status");
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError,
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.exec_failed")
        );
        assert!(outcome.events_to_send[2]
            .payload
            .0
            .get("message")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|message| {
                message.contains("status 42") && message.contains("provider crashed")
            }));
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_maps_empty_final_message_to_empty_output_error() {
        let codex_binary = fake_codex_empty_output_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command =
            command_fixture_with_content("command-codex-empty", CommandKind::SendTurn, "status");
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError,
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.empty_output")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_maps_slow_process_to_start_timeout_error() {
        let codex_binary = fake_codex_slow_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("cortex-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let mut config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        config.codex_timeout = Duration::from_millis(50);
        let command =
            command_fixture_with_content("command-codex-timeout", CommandKind::SendTurn, "status");
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::RuntimeError,
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.start_timeout")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[cfg(unix)]
    fn fake_codex_success_binary() -> PathBuf {
        fake_codex_binary(
            r#"#!/bin/sh
output_path=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    output_path="$1"
  fi
  shift
done
if [ -z "$output_path" ]; then
  echo "missing --output-last-message" >&2
  exit 2
fi
printf '%s\n' 'Codex fake accepted' > "$output_path"
printf '%s\n' '{"type":"response.completed","session_id":"codex-session-1","resume_cursor":"cursor-1"}'
"#,
        )
    }

    #[cfg(unix)]
    fn fake_codex_prompt_capture_binary(capture_path: &Path) -> PathBuf {
        fake_codex_binary(&format!(
            r#"#!/bin/sh
output_path=""
last_arg=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    output_path="$1"
  else
    last_arg="$1"
  fi
  shift
done
if [ -z "$output_path" ]; then
  echo "missing --output-last-message" >&2
  exit 2
fi
printf '%s\n' "$last_arg" > '{}'
printf '%s\n' 'Codex contextual answer' > "$output_path"
printf '%s\n' '{{"type":"response.completed"}}'
"#,
            capture_path.display()
        ))
    }

    #[cfg(unix)]
    fn fake_codex_resume_capture_binary(capture_path: &Path) -> PathBuf {
        fake_codex_binary(&format!(
            r#"#!/bin/sh
output_path=""
for arg in "$@"; do
  if [ "$arg" = "--output-last-message" ]; then
    capture_next=1
  elif [ "${{capture_next:-0}}" = "1" ]; then
    output_path="$arg"
    capture_next=0
  fi
done
if [ -z "$output_path" ]; then
  echo "missing --output-last-message" >&2
  exit 2
fi
printf '%s\n' "$@" > '{}'
printf '%s\n' 'Codex resume accepted' > "$output_path"
printf '%s\n' '{{"type":"response.completed","session_id":"codex-session-1","resume_cursor":"cursor-2"}}'
"#,
            capture_path.display()
        ))
    }

    #[cfg(unix)]
    fn fake_codex_approval_request_binary() -> PathBuf {
        fake_codex_binary(
            r#"#!/bin/sh
printf '%s\n' '{"type":"approval.requested","approval_id":"approval-codex-1","prompt":"Allow file edit?"}'
exit 0
"#,
        )
    }

    #[cfg(unix)]
    fn fake_codex_empty_output_binary() -> PathBuf {
        fake_codex_binary(
            r#"#!/bin/sh
exit 0
"#,
        )
    }

    #[cfg(unix)]
    fn fake_codex_failing_binary() -> PathBuf {
        fake_codex_binary(
            r#"#!/bin/sh
echo "provider crashed" >&2
exit 42
"#,
        )
    }

    #[cfg(unix)]
    fn fake_codex_slow_binary() -> PathBuf {
        fake_codex_binary(
            r#"#!/bin/sh
sleep 2
"#,
        )
    }

    #[cfg(unix)]
    fn fake_codex_binary(script: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("fake-codex-{}", Uuid::new_v4()));
        std::fs::write(&path, script).expect("codex fixture writes");
        let mut permissions = std::fs::metadata(&path)
            .expect("codex fixture metadata reads")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&path, permissions).expect("codex fixture executable");
        path
    }

    fn config_fixture() -> NodeConfig {
        config_fixture_with_codex_binary("codex")
    }

    fn config_fixture_with_codex_binary(codex_binary: impl Into<String>) -> NodeConfig {
        NodeConfig {
            core_url: "http://127.0.0.1:8080"
                .parse()
                .expect("test core URL parses"),
            display_name: "Test Node".to_owned(),
            heartbeat_interval: Duration::from_secs(5),
            state_path: std::env::temp_dir().join(format!("cortex-node-{}.json", Uuid::new_v4())),
            workspace_paths: vec![],
            codex_binary: codex_binary.into(),
            codex_timeout: Duration::from_secs(5),
        }
    }

    fn command_fixture(command_id: &str, kind: CommandKind) -> CommandEnvelope {
        command_fixture_with_content(command_id, kind, "hello")
    }

    fn command_fixture_with_content(
        command_id: &str,
        kind: CommandKind,
        content: &str,
    ) -> CommandEnvelope {
        CommandEnvelope {
            command_id: CommandId::from(command_id),
            kind,
            target_node_id: NodeId::from("node-1"),
            actor_ref: ActorRef::local_user(),
            session_thread_id: Some(SessionThreadId::from("session-1")),
            runtime_session_id: Some(RuntimeSessionId::from("runtime-1")),
            project_placement_id: None,
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("correlation-1"),
            payload: JsonValue(serde_json::json!({
                "turn_id": "turn-1",
                "content": content
            })),
        }
    }

    fn placement_command_fixture(
        command_id: &str,
        placement_id: &str,
        display_name: &str,
        workspace_path: &str,
    ) -> CommandEnvelope {
        CommandEnvelope {
            command_id: CommandId::from(command_id),
            kind: CommandKind::ValidateWorkspace,
            target_node_id: NodeId::from("node-1"),
            actor_ref: ActorRef::local_user(),
            session_thread_id: None,
            runtime_session_id: None,
            project_placement_id: Some(ProjectPlacementId::from(placement_id)),
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("correlation-1"),
            payload: JsonValue(serde_json::json!({
                "display_name": display_name,
                "workspace_path": workspace_path,
            })),
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

    fn event_ids(events: &[EventEnvelope]) -> Vec<EventId> {
        events.iter().map(|event| event.event_id.clone()).collect()
    }

    fn event_kinds(events: &[EventEnvelope]) -> Vec<EventKind> {
        events.iter().map(|event| event.kind.clone()).collect()
    }

    fn badge_kinds(badges: &[ResourceBadge]) -> Vec<&str> {
        badges.iter().map(|badge| badge.kind.as_str()).collect()
    }
}
