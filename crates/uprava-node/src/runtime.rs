//! Uprava Node daemon composition root.
//!
//! The daemon owns local state, workspace/process access and PTY lifetimes. It
//! accepts only protocol v2 control traffic and persists command/event progress
//! before acknowledging delivery.

use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, OpenOptions},
    hash::Hash,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command as StdCommand, ExitStatus, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use anyhow::Context;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use pty_process::{Command as PtyCommand, Size as PtySize};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command as TokioCommand,
    sync::{mpsc, oneshot, watch, Mutex, RwLock, Semaphore},
    time::timeout,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message as WsMessage},
};
use uprava_logging::init_tracing;
#[cfg(test)]
use uprava_protocol::CommandTarget;
use uprava_protocol::{
    is_supported_protocol_version, serde_json_value::JsonValue, ActorRef, ApiError, ApprovalId,
    CapabilitySummary, CapabilityValue, CommandEnvelope, CommandId, CommandKind, CommandPayload,
    CommandState, ControlFrame, CorrelationId, DeductionInputPackage, DeductionProviderOutput,
    DeductionProviderResult, EnrollmentId, EventEnvelope, EventId, EventKind, EventPayload,
    NodeEnrollmentClaimRequest, NodeEnrollmentClaimResponse, NodeEnrollmentRequest,
    NodeEnrollmentRequestedResponse, NodeHeartbeatRequest, NodeHeartbeatResponse, NodeId,
    PlacementState, ProjectPlacementId, ResourceBadge, RuntimeSessionId, RuntimeSessionState,
    ScopeRef, SessionThreadId, SleepHint, TerminalId, TextRange, TurnId, UpravaRef,
    WarningSeverity, WorkspaceCommandIntent, WorkspaceCommandRunRequest,
    WorkspaceCommandRunResponse, WorkspaceDiffResponse, WorkspaceEntry,
    WorkspaceEntryClassification, WorkspaceEntryKind, WorkspaceEntryStatus,
    WorkspaceFileContentResponse, WorkspaceFileWriteRequest, WorkspaceFileWriteResponse,
    WorkspaceSnapshot, WorkspaceTerminalOpenRequest, WorkspaceTerminalOpenResponse,
    WorkspaceTerminalOutputFrame, WorkspaceTerminalState, WorkspaceTerminalSummary,
    WorkspaceTreeResponse, CURRENT_PROTOCOL_VERSION as API_VERSION,
};
use uuid::Uuid;

#[path = "config.rs"]
mod config;
#[path = "supervisor.rs"]
mod supervisor;

use config::NodeConfig;
use supervisor::NodeSupervisor;

type ControlFrameSender = mpsc::Sender<ControlFrame>;
type TerminalSenderRoute = Arc<RwLock<Option<ControlFrameSender>>>;

const MAX_EVENT_OUTBOX_EVENTS: usize = 1024;
const MAX_EVENT_OUTBOX_BYTES: usize = 4 * 1024 * 1024;
const MAX_EVENT_OUTBOX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const MAX_RETAINED_COMMANDS: usize = 1024;
const MAX_CODEX_TRANSCRIPT_MESSAGES: usize = 20;
const MAX_WORKSPACE_DIRECTORY_ENTRIES: usize = 100;
const MAX_WORKSPACE_TEXT_BYTES: u64 = 256 * 1024;
const MAX_WORKSPACE_COMMAND_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_WORKSPACE_COMMAND_ARGS: usize = 32;
const MAX_WORKSPACE_COMMAND_ARG_CHARS: usize = 512;
const MAX_WORKSPACE_COMMAND_SECONDS: u64 = 120;
const ALLOWED_WORKSPACE_COMMANDS: &[&str] = &[
    "cargo", "git", "make", "node", "npm", "pnpm", "bun", "rustc",
];
const WORKSPACE_DIFF_STAT_BYTES: usize = 16 * 1024;
const WORKSPACE_DIFF_BYTES: usize = 128 * 1024;
const MAX_WORKSPACE_TERMINAL_REPLAY_FRAMES: usize = 256;
const MAX_WORKSPACE_TERMINAL_REPLAY_BYTES: usize = 256 * 1024;
const MAX_WORKSPACE_TERMINAL_INPUT_CHARS: usize = 16_384;
const WORKSPACE_TERMINAL_READ_BYTES: usize = 4 * 1024;
const WORKSPACE_TERMINAL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const MIN_WORKSPACE_TERMINAL_COLS: u16 = 20;
const MAX_WORKSPACE_TERMINAL_COLS: u16 = 300;
const MIN_WORKSPACE_TERMINAL_ROWS: u16 = 5;
const MAX_WORKSPACE_TERMINAL_ROWS: u16 = 120;
const MAX_CODEX_TRANSCRIPT_CHARS: usize = 12_000;
const MAX_PROVIDER_ACTIVITY_RAW_CHARS: usize = 16_000;
const MAX_PROVIDER_ACTIVITY_SUMMARY_CHARS: usize = 1_200;
const MAX_PROVIDER_ACTIVITY_LINE_CHARS: usize = 4_000;
const MAX_PROVIDER_ACTIVITY_EVENTS: usize = 512;
const MAX_PROVIDER_PROCESS_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_PROVIDER_APPROVAL_REQUESTS: usize = 16;
const MAX_DEDUCTION_RAW_CHARS: usize = 32_000;
const MAX_CANCELLED_DEDUCTION_TOMBSTONES: usize = 1_024;
const DEDUCTION_SCHEMA_VERSION: &str = "uprava.deduction.v1";
const PROVIDER_PROCESS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const NODE_STATE_SLOT: &str = "0.2.0";
const NODE_STATE_SCHEMA_VERSION: u32 = 1;
const NODE_STATE_STORE_QUEUE_CAPACITY: usize = 256;
const CONTROL_WRITER_QUEUE_CAPACITY: usize = 512;
const NODE_COMMAND_DISPATCH_QUEUE_CAPACITY: usize = 128;
const NODE_PRIORITY_COMMAND_DISPATCH_QUEUE_CAPACITY: usize = 64;
const NODE_COMMAND_DISPATCH_CONCURRENCY: usize = 8;

fn daemon_version() -> String {
    match option_env!("UPRAVA_BUILD_GIT_SHA") {
        Some(release_id) if !release_id.is_empty() && release_id != "unknown" => {
            format!("{}+{release_id}", env!("CARGO_PKG_VERSION"))
        }
        _ => env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// Start the composed Node daemon until shutdown.
pub async fn run() -> anyhow::Result<()> {
    let _log_path = init_tracing("node", "UPRAVA_NODE_LOG_FILE", ".local/logs/node.log")?;

    let config = NodeConfig::from_env()?;
    let client = reqwest::Client::new();
    let state_store = NodeStateStore::new(
        NodeLocalState::load_async(&config.state_path).await?,
        config.state_path.clone(),
    );
    tracing::info!("starting uprava node");

    NodeSupervisor::new(config, client, state_store).run().await
}

#[derive(Clone, Serialize, Deserialize)]
struct NodeLocalState {
    #[serde(default = "default_node_state_slot")]
    state_slot: String,
    #[serde(default = "default_node_state_schema_version")]
    schema_version: u32,
    #[serde(default = "new_daemon_installation_id")]
    daemon_installation_id: String,
    node_id: Option<NodeId>,
    credential: Option<String>,
    enrollment_id: Option<EnrollmentId>,
    pairing_code: Option<String>,
    #[serde(default)]
    command_status: HashMap<String, CommandState>,
    #[serde(default)]
    command_result_payloads: HashMap<String, JsonValue>,
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
    cancelled_deductions: HashSet<String>,
    #[serde(default)]
    placement_seqs: HashMap<String, i64>,
    #[serde(default)]
    reconnect_attempts: u64,
    #[serde(default)]
    dropped_event_count: u64,
    #[serde(default)]
    heartbeat_failures: u64,
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
            state_slot: default_node_state_slot(),
            schema_version: default_node_state_schema_version(),
            daemon_installation_id: new_daemon_installation_id(),
            node_id: None,
            credential: None,
            enrollment_id: None,
            pairing_code: None,
            command_status: HashMap::new(),
            command_result_payloads: HashMap::new(),
            runtime_seqs: HashMap::new(),
            event_outbox: Vec::new(),
            runtime_providers: HashMap::new(),
            runtime_workspace_paths: HashMap::new(),
            runtime_states: HashMap::new(),
            runtime_transcripts: HashMap::new(),
            runtime_provider_resume_refs: HashMap::new(),
            cancelled_deductions: HashSet::new(),
            placement_seqs: HashMap::new(),
            reconnect_attempts: 0,
            dropped_event_count: 0,
            heartbeat_failures: 0,
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
            .field("state_slot", &self.state_slot)
            .field("schema_version", &self.schema_version)
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
            .field(
                "command_result_payload_count",
                &self.command_result_payloads.len(),
            )
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
            .field("cancelled_deductions", &self.cancelled_deductions)
            .field("placement_seqs", &self.placement_seqs)
            .field("reconnect_attempts", &self.reconnect_attempts)
            .field("dropped_event_count", &self.dropped_event_count)
            .field("heartbeat_failures", &self.heartbeat_failures)
            .finish()
    }
}

fn new_daemon_installation_id() -> String {
    format!("daemon-{}", Uuid::new_v4())
}

fn default_node_state_slot() -> String {
    NODE_STATE_SLOT.to_owned()
}

fn default_node_state_schema_version() -> u32 {
    NODE_STATE_SCHEMA_VERSION
}

fn merge_changed_map<K, V>(
    owner: &mut HashMap<K, V>,
    baseline: &HashMap<K, V>,
    candidate: &HashMap<K, V>,
) where
    K: Eq + Hash + Clone,
    V: Clone + PartialEq,
{
    let keys = baseline
        .keys()
        .chain(candidate.keys())
        .cloned()
        .collect::<HashSet<_>>();
    for key in keys {
        match (baseline.get(&key), candidate.get(&key)) {
            (Some(previous), Some(next)) if previous != next => {
                owner.insert(key, next.clone());
            }
            (None, Some(next)) => {
                owner.insert(key, next.clone());
            }
            (Some(previous), None) if owner.get(&key) == Some(previous) => {
                owner.remove(&key);
            }
            _ => {}
        }
    }
}

impl NodeLocalState {
    fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            if let Some(legacy_path) = legacy_state_path(path) {
                if legacy_path.exists() {
                    anyhow::bail!(
                        "legacy Uprava Node state found at {}; state slot {} is isolated; move or remove the legacy state and re-enroll",
                        legacy_path.display(),
                        NODE_STATE_SLOT
                    );
                }
            }
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read node state {}", path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse node state {}", path.display()))?;
        if is_versioned_state_path(path) {
            let slot = value.get("state_slot").and_then(serde_json::Value::as_str);
            let schema_version = value
                .get("schema_version")
                .and_then(serde_json::Value::as_u64);
            if slot != Some(NODE_STATE_SLOT)
                || schema_version != Some(NODE_STATE_SCHEMA_VERSION as u64)
            {
                anyhow::bail!(
                    "Node state at {} is not compatible with slot {} schema {}; move it aside and re-enroll",
                    path.display(),
                    NODE_STATE_SLOT,
                    NODE_STATE_SCHEMA_VERSION
                );
            }
        }
        serde_json::from_value(value)
            .with_context(|| format!("failed to decode node state {}", path.display()))
    }

    fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
            set_private_dir_permissions(parent);
        }
        let mut snapshot = self.clone();
        snapshot.compact_for_persistence();
        let content =
            serde_json::to_string_pretty(&snapshot).context("failed to serialize node state")?;
        write_private_file(path, content.as_bytes())
            .with_context(|| format!("failed to write node state {}", path.display()))
    }

    async fn load_async(path: &Path) -> anyhow::Result<Self> {
        if !is_sqlite_state_path(path) {
            return Self::load(path);
        }
        if !path.exists() {
            if let Some(legacy_path) = legacy_state_path(path) {
                if legacy_path.exists() {
                    anyhow::bail!(
                        "legacy Uprava Node state found at {}; state slot {} is isolated; move or remove the legacy state and re-enroll",
                        legacy_path.display(),
                        NODE_STATE_SLOT
                    );
                }
            }
        }
        let pool = open_state_store(path).await?;
        initialize_state_store(&pool).await?;
        let row = sqlx::query(
            "select state_slot, schema_version, snapshot_json from node_state where state_id = 1",
        )
        .fetch_optional(&pool)
        .await?;
        let Some(row) = row else {
            pool.close().await;
            return Ok(Self::default());
        };
        let slot: String = row.try_get("state_slot")?;
        let schema_version: i64 = row.try_get("schema_version")?;
        if slot != NODE_STATE_SLOT || schema_version != NODE_STATE_SCHEMA_VERSION as i64 {
            pool.close().await;
            anyhow::bail!(
                "Node state at {} is not compatible with slot {} schema {}; move it aside and re-enroll",
                path.display(),
                NODE_STATE_SLOT,
                NODE_STATE_SCHEMA_VERSION
            );
        }
        let snapshot: String = row.try_get("snapshot_json")?;
        pool.close().await;
        let mut state: Self = serde_json::from_str(&snapshot)
            .with_context(|| format!("failed to decode node state {}", path.display()))?;
        let pool = open_state_store(path).await?;
        hydrate_from_normalized_tables(&pool, &mut state).await?;
        pool.close().await;
        Ok(state)
    }

    async fn save_async(&self, path: &Path) -> anyhow::Result<()> {
        if !is_sqlite_state_path(path) {
            return self.save(path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
            set_private_dir_permissions(parent);
        }
        let mut snapshot = self.clone();
        snapshot.compact_for_persistence();
        let snapshot_json = serde_json::to_string(&snapshot.sqlite_compatibility_seed())
            .context("failed to serialize node state snapshot")?;
        let pool = open_state_store(path).await?;
        initialize_state_store(&pool).await?;
        let mut transaction = pool.begin().await?;
        sqlx::query(
            r#"
            insert into node_state (state_id, state_slot, schema_version, snapshot_json, updated_at)
            values (1, ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            on conflict(state_id) do update set
                state_slot = excluded.state_slot,
                schema_version = excluded.schema_version,
                snapshot_json = excluded.snapshot_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(NODE_STATE_SLOT)
        .bind(NODE_STATE_SCHEMA_VERSION as i64)
        .bind(snapshot_json)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("delete from node_command_cache")
            .execute(&mut *transaction)
            .await?;
        for (command_id, status) in &snapshot.command_status {
            sqlx::query(
                "insert into node_command_cache (command_id, state, result_payload_json, updated_at) values (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            )
            .bind(command_id)
            .bind(command_state_storage(*status))
            .bind(
                snapshot
                    .command_result_payloads
                    .get(command_id)
                    .map(serde_json::to_string)
                    .transpose()?,
            )
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query("delete from node_event_outbox")
            .execute(&mut *transaction)
            .await?;
        for event in &snapshot.event_outbox {
            sqlx::query(
                "insert into node_event_outbox (event_id, event_json, seq, created_at) values (?1, ?2, ?3, ?4)",
            )
            .bind(event.event_id.as_str())
            .bind(serde_json::to_string(event)?)
            .bind(event.seq)
            .bind(event.happened_at)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query("delete from node_registration")
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "insert into node_registration (state_id, daemon_installation_id, node_id, credential, enrollment_id, pairing_code, updated_at) values (1, ?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(&snapshot.daemon_installation_id)
        .bind(snapshot.node_id.as_ref().map(NodeId::as_str))
        .bind(snapshot.credential.as_deref())
        .bind(snapshot.enrollment_id.as_ref().map(EnrollmentId::as_str))
        .bind(snapshot.pairing_code.as_deref())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            insert into node_metrics (
                state_id, reconnect_attempts, dropped_event_count, heartbeat_failures, updated_at
            )
            values (1, ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            on conflict(state_id) do update set
                reconnect_attempts = excluded.reconnect_attempts,
                dropped_event_count = excluded.dropped_event_count,
                heartbeat_failures = excluded.heartbeat_failures,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(snapshot.reconnect_attempts as i64)
        .bind(snapshot.dropped_event_count as i64)
        .bind(snapshot.heartbeat_failures as i64)
        .execute(&mut *transaction)
        .await?;

        let runtime_ids = snapshot
            .runtime_seqs
            .keys()
            .chain(snapshot.runtime_providers.keys())
            .chain(snapshot.runtime_workspace_paths.keys())
            .chain(snapshot.runtime_states.keys())
            .chain(snapshot.runtime_transcripts.keys())
            .chain(snapshot.runtime_provider_resume_refs.keys())
            .cloned()
            .collect::<HashSet<_>>();
        sqlx::query("delete from node_runtime_metadata")
            .execute(&mut *transaction)
            .await?;
        for runtime_id in runtime_ids {
            sqlx::query(
                "insert into node_runtime_metadata (runtime_session_id, runtime_seq, provider, workspace_path, state_json, transcript_json, resume_ref_json, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            )
            .bind(&runtime_id)
            .bind(snapshot.runtime_seqs.get(&runtime_id).copied())
            .bind(snapshot.runtime_providers.get(&runtime_id))
            .bind(snapshot.runtime_workspace_paths.get(&runtime_id))
            .bind(
                snapshot
                    .runtime_states
                    .get(&runtime_id)
                    .map(serde_json::to_string)
                    .transpose()?,
            )
            .bind(
                snapshot
                    .runtime_transcripts
                    .get(&runtime_id)
                    .map(serde_json::to_string)
                    .transpose()?,
            )
            .bind(
                snapshot
                    .runtime_provider_resume_refs
                    .get(&runtime_id)
                    .map(serde_json::to_string)
                    .transpose()?,
            )
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query("delete from node_placement_sequences")
            .execute(&mut *transaction)
            .await?;
        for (placement_id, seq) in &snapshot.placement_seqs {
            sqlx::query(
                "insert into node_placement_sequences (placement_id, seq, updated_at) values (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            )
            .bind(placement_id)
            .bind(*seq)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        pool.close().await;
        #[cfg(unix)]
        std::fs::set_permissions(path, PermissionsExt::from_mode(0o600))?;
        Ok(())
    }

    fn sqlite_compatibility_seed(&self) -> Self {
        Self {
            state_slot: self.state_slot.clone(),
            schema_version: self.schema_version,
            daemon_installation_id: self.daemon_installation_id.clone(),
            cancelled_deductions: self.cancelled_deductions.clone(),
            ..Self::default()
        }
    }

    fn compact_for_persistence(&mut self) {
        if self.command_status.len() > MAX_RETAINED_COMMANDS {
            let removable = self
                .command_status
                .iter()
                .filter(|(_, status)| {
                    matches!(
                        status,
                        CommandState::Completed
                            | CommandState::Failed
                            | CommandState::Blocked
                            | CommandState::Expired
                    )
                })
                .map(|(command_id, _)| command_id.clone())
                .collect::<Vec<_>>();
            let remove_count = self.command_status.len() - MAX_RETAINED_COMMANDS;
            for command_id in removable.into_iter().take(remove_count) {
                self.command_status.remove(&command_id);
                self.command_result_payloads.remove(&command_id);
            }
        }
        self.command_result_payloads
            .retain(|command_id, _| self.command_status.contains_key(command_id));
    }

    fn is_enrolled(&self) -> bool {
        self.node_id.is_some() && self.credential.is_some()
    }

    fn clear_core_registration(&mut self) {
        self.node_id = None;
        self.credential = None;
        self.enrollment_id = None;
        self.pairing_code = None;
    }

    fn clear_enrollment_attempt(&mut self) {
        self.enrollment_id = None;
        self.pairing_code = None;
    }

    fn merge_command_state_from(&mut self, baseline: &Self, command_state: &Self) {
        // Apply only changes made to the command snapshot. A stale snapshot
        // must never replace an ACKed outbox or newer runtime metadata.
        merge_changed_map(
            &mut self.command_status,
            &baseline.command_status,
            &command_state.command_status,
        );
        merge_changed_map(
            &mut self.command_result_payloads,
            &baseline.command_result_payloads,
            &command_state.command_result_payloads,
        );
        for event in &command_state.event_outbox {
            if !baseline
                .event_outbox
                .iter()
                .any(|old| old.event_id == event.event_id)
                && !self
                    .event_outbox
                    .iter()
                    .any(|old| old.event_id == event.event_id)
            {
                self.event_outbox.push(event.clone());
            }
        }
        for (runtime_id, seq) in &command_state.runtime_seqs {
            if baseline.runtime_seqs.get(runtime_id) != Some(seq) {
                let current = self.runtime_seqs.entry(runtime_id.clone()).or_default();
                *current = (*current).max(*seq);
            }
        }
        merge_changed_map(
            &mut self.runtime_providers,
            &baseline.runtime_providers,
            &command_state.runtime_providers,
        );
        merge_changed_map(
            &mut self.runtime_workspace_paths,
            &baseline.runtime_workspace_paths,
            &command_state.runtime_workspace_paths,
        );
        merge_changed_map(
            &mut self.runtime_states,
            &baseline.runtime_states,
            &command_state.runtime_states,
        );
        merge_changed_map(
            &mut self.runtime_transcripts,
            &baseline.runtime_transcripts,
            &command_state.runtime_transcripts,
        );
        merge_changed_map(
            &mut self.runtime_provider_resume_refs,
            &baseline.runtime_provider_resume_refs,
            &command_state.runtime_provider_resume_refs,
        );
        for deduction_id in command_state
            .cancelled_deductions
            .difference(&baseline.cancelled_deductions)
        {
            remember_cancelled_deduction(&mut self.cancelled_deductions, deduction_id.clone());
        }
        for deduction_id in baseline
            .cancelled_deductions
            .difference(&command_state.cancelled_deductions)
        {
            self.cancelled_deductions.remove(deduction_id);
        }
        for (placement_id, seq) in &command_state.placement_seqs {
            if baseline.placement_seqs.get(placement_id) != Some(seq) {
                let current = self.placement_seqs.entry(placement_id.clone()).or_default();
                *current = (*current).max(*seq);
            }
        }
        self.dropped_event_count = self
            .dropped_event_count
            .max(command_state.dropped_event_count);
        self.heartbeat_failures = self
            .heartbeat_failures
            .max(command_state.heartbeat_failures);
    }
}

fn remember_cancelled_deduction(tombstones: &mut HashSet<String>, deduction_id: String) {
    if tombstones.contains(&deduction_id) {
        return;
    }
    if tombstones.len() >= MAX_CANCELLED_DEDUCTION_TOMBSTONES {
        if let Some(expired) = tombstones.iter().next().cloned() {
            tombstones.remove(&expired);
        }
    }
    tombstones.insert(deduction_id);
}

/// The single owner boundary for durable Node state mutations.
///
/// Runtime tasks may keep a short-lived in-memory snapshot while doing I/O,
/// but every mutation that crosses the control path goes through this store.
#[derive(Clone)]
struct NodeStateStore {
    sender: mpsc::Sender<NodeStateRequest>,
    actor: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

enum NodeStateRequest {
    Snapshot {
        respond_to: oneshot::Sender<anyhow::Result<NodeLocalState>>,
    },
    IsEnrolled {
        respond_to: oneshot::Sender<anyhow::Result<bool>>,
    },
    Mutate {
        mutation: Box<NodeStateMutation>,
        respond_to: oneshot::Sender<anyhow::Result<NodeStateMutationResult>>,
    },
    Shutdown {
        respond_to: oneshot::Sender<()>,
    },
}

enum NodeStateMutation {
    PersistEnrollmentAttempt {
        enrollment_id: EnrollmentId,
        pairing_code: String,
    },
    ClearEnrollmentAttempt,
    PersistEnrollmentIdentity {
        node_id: NodeId,
        credential: String,
    },
    ClearCoreRegistration,
    PersistReconnectAttempt,
    PersistHeartbeatFailure,
    PersistEventAck {
        accepted_event_ids: Vec<EventId>,
    },
    MergeCommandState {
        baseline: Box<NodeLocalState>,
        command_state: Box<NodeLocalState>,
    },
}

enum NodeStateMutationResult {
    Unit,
    RemovedEvents(usize),
}

impl NodeStateStore {
    fn new(state: NodeLocalState, path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel(NODE_STATE_STORE_QUEUE_CAPACITY);
        let actor = tokio::spawn(run_node_state_store(state, path, receiver));
        Self {
            sender,
            actor: Arc::new(Mutex::new(Some(actor))),
        }
    }

    async fn snapshot(&self) -> anyhow::Result<NodeLocalState> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(NodeStateRequest::Snapshot { respond_to })
            .await
            .map_err(|_| anyhow::anyhow!("node state store task stopped"))?;
        response.await.context("node state store task stopped")?
    }

    async fn is_enrolled(&self) -> anyhow::Result<bool> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(NodeStateRequest::IsEnrolled { respond_to })
            .await
            .map_err(|_| anyhow::anyhow!("node state store task stopped"))?;
        response.await.context("node state store task stopped")?
    }

    async fn ensure_enrollment(
        &self,
        client: &reqwest::Client,
        config: &NodeConfig,
    ) -> anyhow::Result<bool> {
        ensure_enrollment(client, config, self).await
    }

    async fn persist_enrollment_attempt(
        &self,
        enrollment_id: EnrollmentId,
        pairing_code: String,
    ) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::PersistEnrollmentAttempt {
            enrollment_id,
            pairing_code,
        })
        .await?;
        Ok(())
    }

    async fn clear_enrollment_attempt(&self) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::ClearEnrollmentAttempt)
            .await?;
        Ok(())
    }

    async fn persist_enrollment_identity(
        &self,
        node_id: NodeId,
        credential: String,
    ) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::PersistEnrollmentIdentity {
            node_id,
            credential,
        })
        .await?;
        Ok(())
    }

    async fn send_heartbeat(
        &self,
        client: &reqwest::Client,
        config: &NodeConfig,
    ) -> anyhow::Result<NodeHeartbeatResponse> {
        let state = self.snapshot().await?;
        send_heartbeat(client, config, &state).await
    }

    async fn clear_core_registration(&self) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::ClearCoreRegistration)
            .await?;
        Ok(())
    }

    async fn persist_reconnect_attempt(&self) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::PersistReconnectAttempt)
            .await?;
        Ok(())
    }

    async fn persist_heartbeat_failure(&self) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::PersistHeartbeatFailure)
            .await?;
        Ok(())
    }

    async fn persist_event_ack(&self, accepted_event_ids: &[EventId]) -> anyhow::Result<usize> {
        match self
            .mutate(NodeStateMutation::PersistEventAck {
                accepted_event_ids: accepted_event_ids.to_vec(),
            })
            .await?
        {
            NodeStateMutationResult::RemovedEvents(removed) => Ok(removed),
            NodeStateMutationResult::Unit => Ok(0),
        }
    }

    async fn merge_command_state(
        &self,
        baseline: &NodeLocalState,
        command_state: &NodeLocalState,
    ) -> anyhow::Result<()> {
        self.mutate(NodeStateMutation::MergeCommandState {
            baseline: Box::new(baseline.clone()),
            command_state: Box::new(command_state.clone()),
        })
        .await?;
        Ok(())
    }

    /// Persist a completed command's status, result payload, and generated
    /// event outbox entries as one owner-boundary mutation.
    async fn persist_command_outcome(
        &self,
        baseline: &NodeLocalState,
        command_state: &NodeLocalState,
    ) -> anyhow::Result<()> {
        self.merge_command_state(baseline, command_state).await
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        let (respond_to, response) = oneshot::channel();
        let _ = self
            .sender
            .send(NodeStateRequest::Shutdown { respond_to })
            .await;
        let _ = response.await;
        if let Some(actor) = self.actor.lock().await.take() {
            actor.await.context("node state store task join failed")?;
        }
        Ok(())
    }

    async fn mutate(&self, mutation: NodeStateMutation) -> anyhow::Result<NodeStateMutationResult> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(NodeStateRequest::Mutate {
                mutation: Box::new(mutation),
                respond_to,
            })
            .await
            .map_err(|_| anyhow::anyhow!("node state store task stopped"))?;
        response.await.context("node state store task stopped")?
    }
}

async fn run_node_state_store(
    mut state: NodeLocalState,
    path: PathBuf,
    mut receiver: mpsc::Receiver<NodeStateRequest>,
) {
    while let Some(request) = receiver.recv().await {
        match request {
            NodeStateRequest::Snapshot { respond_to } => {
                let _ = respond_to.send(Ok(state.clone()));
            }
            NodeStateRequest::IsEnrolled { respond_to } => {
                let _ = respond_to.send(Ok(state.is_enrolled()));
            }
            NodeStateRequest::Mutate {
                mutation,
                respond_to,
            } => {
                let _ =
                    respond_to.send(apply_node_state_mutation(&mut state, &path, *mutation).await);
            }
            NodeStateRequest::Shutdown { respond_to } => {
                let _ = respond_to.send(());
                break;
            }
        }
    }
}

async fn apply_node_state_mutation(
    state: &mut NodeLocalState,
    path: &Path,
    mutation: NodeStateMutation,
) -> anyhow::Result<NodeStateMutationResult> {
    match mutation {
        NodeStateMutation::PersistEnrollmentAttempt {
            enrollment_id,
            pairing_code,
        } => {
            state.enrollment_id = Some(enrollment_id);
            state.pairing_code = Some(pairing_code);
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::ClearEnrollmentAttempt => {
            state.clear_enrollment_attempt();
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::PersistEnrollmentIdentity {
            node_id,
            credential,
        } => {
            state.node_id = Some(node_id);
            state.credential = Some(credential);
            state.enrollment_id = None;
            state.pairing_code = None;
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::ClearCoreRegistration => {
            state.clear_core_registration();
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::PersistReconnectAttempt => {
            state.reconnect_attempts = state.reconnect_attempts.saturating_add(1);
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::PersistHeartbeatFailure => {
            state.heartbeat_failures = state.heartbeat_failures.saturating_add(1);
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
        NodeStateMutation::PersistEventAck { accepted_event_ids } => {
            let removed = remove_acked_events(&mut state.event_outbox, &accepted_event_ids);
            if removed > 0 {
                state.save_async(path).await?;
            }
            Ok(NodeStateMutationResult::RemovedEvents(removed))
        }
        NodeStateMutation::MergeCommandState {
            baseline,
            command_state,
        } => {
            state.merge_command_state_from(&baseline, &command_state);
            state.save_async(path).await?;
            Ok(NodeStateMutationResult::Unit)
        }
    }
}

async fn open_state_store(path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        set_private_dir_permissions(parent);
    }
    SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true),
    )
    .await
    .with_context(|| format!("failed to open node state store {}", path.display()))
}

async fn initialize_state_store(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        create table if not exists node_state (
            state_id integer primary key check (state_id = 1),
            state_slot text not null,
            schema_version integer not null,
            snapshot_json text not null,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_command_cache (
            command_id text primary key,
            state text not null,
            result_payload_json text,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_event_outbox (
            event_id text primary key,
            event_json text not null,
            seq integer not null,
            created_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_registration (
            state_id integer primary key check (state_id = 1),
            daemon_installation_id text not null,
            node_id text,
            credential text,
            enrollment_id text,
            pairing_code text,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_metrics (
            state_id integer primary key check (state_id = 1),
            reconnect_attempts integer not null,
            dropped_event_count integer not null,
            heartbeat_failures integer not null,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_runtime_metadata (
            runtime_session_id text primary key,
            runtime_seq integer,
            provider text,
            workspace_path text,
            state_json text,
            transcript_json text,
            resume_ref_json text,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        create table if not exists node_placement_sequences (
            placement_id text primary key,
            seq integer not null,
            updated_at text not null
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn command_state_storage(state: CommandState) -> &'static str {
    match state {
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

fn command_state_from_storage(value: &str) -> Option<CommandState> {
    Some(match value {
        "recorded" => CommandState::Recorded,
        "pending_dispatch" => CommandState::PendingDispatch,
        "dispatched" => CommandState::Dispatched,
        "acknowledged" => CommandState::Acknowledged,
        "completed" => CommandState::Completed,
        "failed" => CommandState::Failed,
        "blocked" => CommandState::Blocked,
        "expired" => CommandState::Expired,
        _ => return None,
    })
}

async fn hydrate_from_normalized_tables(
    pool: &SqlitePool,
    state: &mut NodeLocalState,
) -> anyhow::Result<()> {
    if let Some(row) = sqlx::query(
        "select daemon_installation_id, node_id, credential, enrollment_id, pairing_code from node_registration where state_id = 1",
    )
    .fetch_optional(pool)
    .await?
    {
        state.daemon_installation_id = row.try_get("daemon_installation_id")?;
        state.node_id = row
            .try_get::<Option<String>, _>("node_id")?
            .map(NodeId::from);
        state.credential = row.try_get("credential")?;
        state.enrollment_id = row
            .try_get::<Option<String>, _>("enrollment_id")?
            .map(EnrollmentId::from);
        state.pairing_code = row.try_get("pairing_code")?;
    }

    if let Some(row) = sqlx::query(
        "select reconnect_attempts, dropped_event_count, heartbeat_failures from node_metrics where state_id = 1",
    )
    .fetch_optional(pool)
    .await?
    {
        state.reconnect_attempts = row
            .try_get::<i64, _>("reconnect_attempts")?
            .try_into()
            .unwrap_or_default();
        state.dropped_event_count = row
            .try_get::<i64, _>("dropped_event_count")?
            .try_into()
            .unwrap_or_default();
        state.heartbeat_failures = row
            .try_get::<i64, _>("heartbeat_failures")?
            .try_into()
            .unwrap_or_default();
    }

    let command_rows =
        sqlx::query("select command_id, state, result_payload_json from node_command_cache")
            .fetch_all(pool)
            .await?;
    if !command_rows.is_empty() {
        state.command_status.clear();
        state.command_result_payloads.clear();
        for row in command_rows {
            let command_id: String = row.try_get("command_id")?;
            let stored_state: String = row.try_get("state")?;
            if let Some(command_state) = command_state_from_storage(&stored_state) {
                state
                    .command_status
                    .insert(command_id.clone(), command_state);
            }
            if let Some(payload) = row.try_get::<Option<String>, _>("result_payload_json")? {
                state
                    .command_result_payloads
                    .insert(command_id, serde_json::from_str(&payload)?);
            }
        }
    }

    let event_rows = sqlx::query("select event_json from node_event_outbox order by rowid")
        .fetch_all(pool)
        .await?;
    if !event_rows.is_empty() {
        state.event_outbox = event_rows
            .into_iter()
            .map(|row| {
                let event_json: String = row.try_get("event_json")?;
                serde_json::from_str(&event_json).map_err(anyhow::Error::from)
            })
            .collect::<anyhow::Result<Vec<EventEnvelope>>>()?;
    }

    let runtime_rows = sqlx::query(
        "select runtime_session_id, runtime_seq, provider, workspace_path, state_json, transcript_json, resume_ref_json from node_runtime_metadata",
    )
    .fetch_all(pool)
    .await?;
    if !runtime_rows.is_empty() {
        state.runtime_seqs.clear();
        state.runtime_providers.clear();
        state.runtime_workspace_paths.clear();
        state.runtime_states.clear();
        state.runtime_transcripts.clear();
        state.runtime_provider_resume_refs.clear();
        for row in runtime_rows {
            let runtime_id: String = row.try_get("runtime_session_id")?;
            if let Some(seq) = row.try_get::<Option<i64>, _>("runtime_seq")? {
                state.runtime_seqs.insert(runtime_id.clone(), seq);
            }
            if let Some(provider) = row.try_get::<Option<String>, _>("provider")? {
                state.runtime_providers.insert(runtime_id.clone(), provider);
            }
            if let Some(path) = row.try_get::<Option<String>, _>("workspace_path")? {
                state
                    .runtime_workspace_paths
                    .insert(runtime_id.clone(), path);
            }
            if let Some(value) = row.try_get::<Option<String>, _>("state_json")? {
                state
                    .runtime_states
                    .insert(runtime_id.clone(), serde_json::from_str(&value)?);
            }
            if let Some(value) = row.try_get::<Option<String>, _>("transcript_json")? {
                state
                    .runtime_transcripts
                    .insert(runtime_id.clone(), serde_json::from_str(&value)?);
            }
            if let Some(value) = row.try_get::<Option<String>, _>("resume_ref_json")? {
                state
                    .runtime_provider_resume_refs
                    .insert(runtime_id, serde_json::from_str(&value)?);
            }
        }
    }

    let placement_rows = sqlx::query("select placement_id, seq from node_placement_sequences")
        .fetch_all(pool)
        .await?;
    if !placement_rows.is_empty() {
        state.placement_seqs.clear();
        for row in placement_rows {
            state
                .placement_seqs
                .insert(row.try_get("placement_id")?, row.try_get("seq")?);
        }
    }
    Ok(())
}

fn write_private_file(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    set_private_dir_permissions(parent);
    let file_name = path
        .file_name()
        .context("private file path must include a file name")?
        .to_string_lossy();
    let temp_path = parent.join(format!(
        ".{file_name}.{}.tmp",
        sanitize_filename_segment(&Uuid::new_v4().to_string())
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    let mut file = options.open(&temp_path)?;
    file.write_all(content)?;
    file.flush()?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(&temp_path, PermissionsExt::from_mode(0o600))?;
    }
    std::fs::rename(&temp_path, path)?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, PermissionsExt::from_mode(0o600))?;
    }
    sync_parent_directory(parent)?;
    Ok(())
}

fn sync_parent_directory(parent: &Path) -> anyhow::Result<()> {
    match fs::File::open(parent) {
        Ok(directory) => {
            directory.sync_all()?;
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::PermissionDenied => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn set_private_dir_permissions(path: &Path) {
    #[cfg(unix)]
    {
        let _ = std::fs::set_permissions(path, PermissionsExt::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn http_error_status(error: &anyhow::Error) -> Option<reqwest::StatusCode> {
    error.chain().find_map(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .and_then(reqwest::Error::status)
    })
}

fn heartbeat_auth_rejected(error: &anyhow::Error) -> bool {
    http_error_status(error) == Some(reqwest::StatusCode::UNAUTHORIZED)
}

fn enrollment_claim_status_invalidates_attempt(status: Option<reqwest::StatusCode>) -> bool {
    matches!(
        status,
        Some(reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::UNAUTHORIZED)
    )
}

fn enrollment_claim_invalidates_attempt(error: &anyhow::Error) -> bool {
    enrollment_claim_status_invalidates_attempt(http_error_status(error))
}

async fn ensure_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
    store: &NodeStateStore,
) -> anyhow::Result<bool> {
    let mut local_state = store.snapshot().await?;
    if local_state.enrollment_id.is_none() || local_state.pairing_code.is_none() {
        let response = request_enrollment(client, config).await?;
        tracing::info!(
            expires_at = %response.expires_at,
            "enrollment requested; approve this enrollment in Core"
        );
        store
            .persist_enrollment_attempt(
                response.enrollment_id.clone(),
                response.pairing_code.clone(),
            )
            .await?;
        local_state.enrollment_id = Some(response.enrollment_id);
        local_state.pairing_code = Some(response.pairing_code);
    }

    let claim = match claim_enrollment(client, config, &local_state).await {
        Ok(claim) => claim,
        Err(error) if enrollment_claim_invalidates_attempt(&error) => {
            tracing::warn!(
                error = %error,
                "enrollment claim was rejected; clearing stale local enrollment"
            );
            store.clear_enrollment_attempt().await?;
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if claim.pending {
        tracing::info!("waiting for enrollment approval");
        return Ok(false);
    }
    if !claim.accepted {
        tracing::warn!(message = %claim.message, "enrollment was not accepted");
        store.clear_enrollment_attempt().await?;
        return Ok(false);
    }
    if let (Some(node_id), Some(credential)) = (claim.node_id, claim.credential) {
        store
            .persist_enrollment_identity(node_id, credential)
            .await?;
        tracing::info!("enrollment claimed and credential stored");
        return Ok(true);
    }
    store.is_enrolled().await
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
        daemon_version: daemon_version(),
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
        display_name: config.display_name.clone(),
        daemon_version: daemon_version(),
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
        .bearer_auth(
            local_state
                .credential
                .as_deref()
                .context("local node credential missing")?,
        )
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
        "outbox_events={}; cached_commands={}; reconnect_attempts={}; dropped_events={}; heartbeat_failures={}; dropped_log_records={}; otlp_export_failures={}",
        local_state.event_outbox.len(),
        local_state.command_status.len(),
        local_state.reconnect_attempts,
        local_state.dropped_event_count,
        local_state.heartbeat_failures,
        uprava_logging::dropped_log_records(),
        uprava_logging::otlp_export_failures(),
    )
}

async fn control_channel_loop(
    config: NodeConfig,
    store: NodeStateStore,
    terminal_supervisor: TerminalSupervisor,
) {
    loop {
        if let Err(error) = store.persist_reconnect_attempt().await {
            tracing::warn!(error = %error, "failed to persist reconnect metric");
        }
        match run_control_channel(&config, &store, &terminal_supervisor).await {
            Ok(()) => tracing::warn!("control channel closed"),
            Err(error) => tracing::warn!(error = %error, "control channel failed"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_control_channel(
    config: &NodeConfig,
    store: &NodeStateStore,
    terminal_supervisor: &TerminalSupervisor,
) -> anyhow::Result<()> {
    let local_state = store.snapshot().await?;
    let node_id = local_state
        .node_id
        .clone()
        .context("node id is missing for control channel")?;
    let credential = local_state
        .credential
        .clone()
        .context("credential is missing for control channel")?;
    let active_runtime_ids = active_runtime_ids(&local_state);
    let event_outbox = local_state.event_outbox.clone();
    let url = control_url(&config.core_url)?;
    let mut request = url
        .as_str()
        .into_client_request()
        .context("control channel request should build")?;
    request.headers_mut().insert(
        "x-uprava-node-id",
        HeaderValue::from_str(node_id.as_str()).context("node id header should be valid")?,
    );
    request.headers_mut().insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {credential}"))
            .context("authorization header should be valid")?,
    );

    let (socket, _) = connect_async(request)
        .await
        .context("control channel connection failed")?;
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (outbound_tx, mut outbound_rx) =
        mpsc::channel::<ControlFrame>(CONTROL_WRITER_QUEUE_CAPACITY);
    let send_task = tokio::spawn(async move {
        while let Some(frame) = outbound_rx.recv().await {
            let Ok(text) = serde_json::to_string(&frame) else {
                continue;
            };
            if socket_sender
                .send(WsMessage::Text(text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    let (dispatch_tx, dispatch_rx) =
        mpsc::channel::<CommandDispatchJob>(NODE_COMMAND_DISPATCH_QUEUE_CAPACITY);
    let (priority_dispatch_tx, priority_dispatch_rx) =
        mpsc::channel::<CommandDispatchJob>(NODE_PRIORITY_COMMAND_DISPATCH_QUEUE_CAPACITY);
    let dispatcher_task = tokio::spawn(run_command_dispatcher(
        config.clone(),
        store.clone(),
        outbound_tx.clone(),
        terminal_supervisor.clone(),
        priority_dispatch_rx,
        dispatch_rx,
    ));
    tracing::info!("control channel connected");
    terminal_supervisor.rebind_sender(&outbound_tx).await;
    send_frame(
        &outbound_tx,
        ControlFrame::Hello {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            node_id: node_id.clone(),
            daemon_version: daemon_version(),
            active_runtime_ids,
        },
    )
    .await?;

    replay_event_outbox(&outbound_tx, &event_outbox).await?;

    while let Some(message) = socket_receiver.next().await {
        let message = message.context("control channel read failed")?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let frame = serde_json::from_str::<ControlFrame>(&text)
            .context("control frame was not valid JSON")?;
        if let Some(error_frame) = control_frame_protocol_error(&frame) {
            send_frame(&outbound_tx, error_frame).await?;
            continue;
        }
        match frame {
            ControlFrame::CommandDispatch { command, .. } => {
                let command = *command;
                let dispatch_result = if is_priority_cancellation_command(&command) {
                    priority_dispatch_tx.try_send(CommandDispatchJob { command })
                } else {
                    dispatch_tx.try_send(CommandDispatchJob { command })
                };
                match dispatch_result {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(job)) => {
                        send_dispatch_busy_result(&outbound_tx, &job.command).await?;
                    }
                    Err(mpsc::error::TrySendError::Closed(job)) => {
                        send_dispatch_closed_result(&outbound_tx, &job.command).await?;
                    }
                }
            }
            ControlFrame::Ping { frame_id, .. } => {
                send_frame(
                    &outbound_tx,
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
                let removed = store.persist_event_ack(&accepted_event_ids).await?;
                if removed > 0 {
                    let local_state = store.snapshot().await?;
                    tracing::info!(
                        removed,
                        remaining = local_state.event_outbox.len(),
                        "control event outbox acked"
                    );
                }
            }
            ControlFrame::HelloAck { .. } => {}
            ControlFrame::WorkspaceTerminalAttach { terminal_id, .. } => {
                terminal_supervisor.attach(&outbound_tx, &terminal_id).await;
            }
            ControlFrame::WorkspaceTerminalInput {
                terminal_id, data, ..
            } => {
                terminal_supervisor.input(&terminal_id, data).await;
            }
            ControlFrame::WorkspaceTerminalResize {
                terminal_id,
                cols,
                rows,
                ..
            } => {
                terminal_supervisor.resize(&terminal_id, cols, rows).await;
            }
            ControlFrame::WorkspaceTerminalClose { terminal_id, .. } => {
                terminal_supervisor.close(&terminal_id).await;
            }
            _ => {}
        }
    }
    terminal_supervisor.detach_sender().await;
    dispatcher_task.abort();
    let _ = dispatcher_task.await;
    send_task.abort();
    let _ = send_task.await;
    Ok(())
}

#[derive(Debug)]
struct CommandDispatchJob {
    command: CommandEnvelope,
}

#[derive(Clone, Default)]
struct CommandExecutionLocks {
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl CommandExecutionLocks {
    async fn lock_for(&self, key: String) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

#[derive(Clone, Default)]
struct ExecutionCancellationRegistry {
    state: Arc<Mutex<ExecutionCancellationState>>,
}

#[derive(Default)]
struct ExecutionCancellationState {
    senders: HashMap<String, watch::Sender<bool>>,
    pending: HashSet<String>,
}

struct ExecutionCancellationGuard {
    key: String,
    sender: watch::Sender<bool>,
    receiver: watch::Receiver<bool>,
}

impl ExecutionCancellationRegistry {
    async fn begin(&self, key: String) -> ExecutionCancellationGuard {
        let mut state = self.state.lock().await;
        let initially_cancelled = state.pending.remove(&key);
        let (sender, receiver) = watch::channel(initially_cancelled);
        state.senders.insert(key.clone(), sender.clone());
        ExecutionCancellationGuard {
            key,
            sender,
            receiver,
        }
    }

    async fn cancel(&self, key: String, remember_if_pending: bool) -> bool {
        let mut state = self.state.lock().await;
        if let Some(sender) = state.senders.get(&key) {
            return sender.send(true).is_ok();
        }
        if remember_if_pending {
            state.pending.insert(key);
        }
        false
    }

    async fn finish(&self, guard: ExecutionCancellationGuard) {
        let mut state = self.state.lock().await;
        if state
            .senders
            .get(&guard.key)
            .is_some_and(|sender| sender.same_channel(&guard.sender))
        {
            state.senders.remove(&guard.key);
        }
    }
}

impl ExecutionCancellationGuard {
    fn receiver(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }
}

async fn run_command_dispatcher(
    config: NodeConfig,
    shared_state: NodeStateStore,
    sender: ControlFrameSender,
    terminal_supervisor: TerminalSupervisor,
    mut priority_receiver: mpsc::Receiver<CommandDispatchJob>,
    mut receiver: mpsc::Receiver<CommandDispatchJob>,
) {
    let shared = CommandDispatcherShared {
        config,
        shared_state,
        sender,
        terminal_supervisor,
        locks: CommandExecutionLocks::default(),
        cancellations: ExecutionCancellationRegistry::default(),
        concurrency: Arc::new(Semaphore::new(NODE_COMMAND_DISPATCH_CONCURRENCY)),
    };
    let mut tasks = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            biased;
            Some(job) = priority_receiver.recv() => {
                spawn_command_dispatch_task(&mut tasks, job, &shared);
            }
            Some(job) = receiver.recv() => {
                spawn_command_dispatch_task(&mut tasks, job, &shared);
            }
            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                if let Err(error) = result {
                    tracing::warn!(error = %error, "command dispatcher task failed");
                }
            }
            else => break,
        }
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::warn!(error = %error, "command dispatcher task failed");
        }
    }
}

#[derive(Clone)]
struct CommandDispatcherShared {
    config: NodeConfig,
    shared_state: NodeStateStore,
    sender: ControlFrameSender,
    terminal_supervisor: TerminalSupervisor,
    locks: CommandExecutionLocks,
    cancellations: ExecutionCancellationRegistry,
    concurrency: Arc<Semaphore>,
}

fn spawn_command_dispatch_task(
    tasks: &mut tokio::task::JoinSet<()>,
    job: CommandDispatchJob,
    shared: &CommandDispatcherShared,
) {
    let shared = shared.clone();
    tasks.spawn(async move {
        let Some(_permit) = prepare_command_dispatch_task(
            &job.command,
            &shared.cancellations,
            shared.concurrency.clone(),
        )
        .await
        else {
            return;
        };
        let execution_lock = shared
            .locks
            .lock_for(command_execution_key(&job.command))
            .await;
        let _guard = execution_lock.lock().await;
        let cancellation_guard = match execution_cancellation_key(&job.command) {
            Some(key) => Some(shared.cancellations.begin(key).await),
            None => None,
        };
        let cancellation_receiver = cancellation_guard
            .as_ref()
            .map(ExecutionCancellationGuard::receiver);
        let baseline = match shared.shared_state.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "command dispatcher could not load state snapshot"
                );
                let _ = send_dispatch_internal_error_result(
                    &shared.sender,
                    &job.command,
                    "node.dispatch_state_unavailable",
                    "Node could not load durable state before command dispatch",
                    false,
                )
                .await;
                return;
            }
        };
        let mut local_state = baseline.clone();
        let context = CommandDispatchContext {
            config: &shared.config,
            sender: &shared.sender,
            terminal_supervisor: &shared.terminal_supervisor,
            shared_state: &shared.shared_state,
            cancellation: cancellation_receiver,
        };
        if let Err(error) =
            handle_command_dispatch(context, job.command, &mut local_state, &baseline).await
        {
            tracing::warn!(error = %error, "command dispatch failed");
        }
        if let Some(guard) = cancellation_guard {
            shared.cancellations.finish(guard).await;
        }
    });
}

async fn prepare_command_dispatch_task(
    command: &CommandEnvelope,
    cancellations: &ExecutionCancellationRegistry,
    concurrency: Arc<Semaphore>,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    if let Some((key, remember_if_pending)) = cancellation_signal(command) {
        let cancelled = cancellations.cancel(key.clone(), remember_if_pending).await;
        tracing::debug!(cancelled, key, "cancellation command signalled provider");
    }
    match concurrency.acquire_owned().await {
        Ok(permit) => Some(permit),
        Err(_) => {
            tracing::warn!("command dispatcher semaphore closed");
            None
        }
    }
}

fn runtime_cancellation_key(runtime_session_id: &RuntimeSessionId) -> String {
    format!("runtime:{}", runtime_session_id.as_str())
}

fn is_priority_cancellation_command(command: &CommandEnvelope) -> bool {
    matches!(
        command.kind,
        CommandKind::InterruptRuntime | CommandKind::StopRuntime | CommandKind::CancelDeduction
    )
}

fn deduction_cancellation_key(deduction_id: &uprava_protocol::DeductionId) -> String {
    format!("deduction:{}", deduction_id.as_str())
}

fn execution_cancellation_key(command: &CommandEnvelope) -> Option<String> {
    match (&command.kind, &command.payload) {
        (CommandKind::SendTurn, _) => command
            .target
            .runtime_session_id()
            .map(runtime_cancellation_key),
        (CommandKind::RequestDeduction, CommandPayload::RequestDeduction { package }) => {
            Some(deduction_cancellation_key(&package.deduction_id))
        }
        _ => None,
    }
}

fn cancellation_signal(command: &CommandEnvelope) -> Option<(String, bool)> {
    match (&command.kind, &command.payload) {
        (CommandKind::InterruptRuntime | CommandKind::StopRuntime, _) => command
            .target
            .runtime_session_id()
            .map(runtime_cancellation_key)
            .map(|key| (key, false)),
        (CommandKind::CancelDeduction, CommandPayload::CancelDeduction { deduction_id }) => {
            Some((deduction_cancellation_key(deduction_id), true))
        }
        _ => None,
    }
}

fn command_execution_key(command: &CommandEnvelope) -> String {
    command
        .target
        .runtime_session_id()
        .map(|runtime_id| format!("runtime:{}", runtime_id.as_str()))
        .or_else(|| {
            command
                .target
                .project_placement_id()
                .map(|placement_id| format!("placement:{}", placement_id.as_str()))
        })
        .unwrap_or_else(|| format!("command:{}", command.command_id.as_str()))
}

struct CommandDispatchContext<'a> {
    config: &'a NodeConfig,
    sender: &'a ControlFrameSender,
    terminal_supervisor: &'a TerminalSupervisor,
    shared_state: &'a NodeStateStore,
    cancellation: Option<watch::Receiver<bool>>,
}

async fn handle_command_dispatch(
    context: CommandDispatchContext<'_>,
    command: CommandEnvelope,
    local_state: &mut NodeLocalState,
    baseline: &NodeLocalState,
) -> anyhow::Result<()> {
    let outcome = prepare_command_dispatch_with_live_socket(
        context.config,
        local_state,
        &command,
        Some(context.sender),
        Some(context.terminal_supervisor),
        context.cancellation,
    )
    .await;
    if outcome.state_changed {
        context
            .shared_state
            .persist_command_outcome(baseline, local_state)
            .await?;
    }

    send_frame(
        context.sender,
        ControlFrame::CommandAck {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Acknowledged,
        },
    )
    .await?;

    send_event_batch(context.sender, outcome.events_to_send).await?;
    send_command_result(
        context.sender,
        &command.command_id,
        outcome.status,
        outcome.result_payload,
    )
    .await
}

async fn send_command_result(
    sender: &ControlFrameSender,
    command_id: &CommandId,
    status: CommandState,
    payload: JsonValue,
) -> anyhow::Result<()> {
    send_frame(
        sender,
        ControlFrame::CommandResult {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command_id.clone(),
            status,
            payload,
        },
    )
    .await
}

async fn replay_event_outbox(
    sender: &ControlFrameSender,
    events: &[EventEnvelope],
) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    tracing::info!(events = events.len(), "replaying control event outbox");
    send_event_batch(sender, events.to_vec()).await
}

async fn send_event_batch(
    sender: &ControlFrameSender,
    events: Vec<EventEnvelope>,
) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    send_frame(
        sender,
        ControlFrame::EventBatch {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            events,
        },
    )
    .await
}

async fn send_dispatch_busy_result(
    sender: &ControlFrameSender,
    command: &CommandEnvelope,
) -> anyhow::Result<()> {
    send_dispatch_internal_error_result(
        sender,
        command,
        "node.dispatch_busy",
        "Node command dispatcher is saturated; retry the command later",
        true,
    )
    .await
}

async fn send_dispatch_closed_result(
    sender: &ControlFrameSender,
    command: &CommandEnvelope,
) -> anyhow::Result<()> {
    send_dispatch_internal_error_result(
        sender,
        command,
        "node.dispatch_closed",
        "Node command dispatcher is unavailable on this control connection",
        true,
    )
    .await
}

async fn send_dispatch_internal_error_result(
    sender: &ControlFrameSender,
    command: &CommandEnvelope,
    error_code: &'static str,
    message: &'static str,
    retryable: bool,
) -> anyhow::Result<()> {
    send_frame(
        sender,
        ControlFrame::CommandAck {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            command_id: command.command_id.clone(),
            status: CommandState::Acknowledged,
        },
    )
    .await?;
    send_command_result(
        sender,
        &command.command_id,
        CommandState::Failed,
        JsonValue(serde_json::json!({
            "error_code": error_code,
            "message": message,
            "retryable": retryable,
        })),
    )
    .await
}

async fn send_frame(sender: &ControlFrameSender, frame: ControlFrame) -> anyhow::Result<()> {
    sender.try_send(frame).context("control frame send failed")
}

fn control_frame_protocol_error(frame: &ControlFrame) -> Option<ControlFrame> {
    let received_protocol_version = control_frame_protocol_version(frame);
    if is_supported_protocol_version(received_protocol_version) {
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
        | ControlFrame::WorkspaceTerminalAttach {
            protocol_version, ..
        }
        | ControlFrame::WorkspaceTerminalInput {
            protocol_version, ..
        }
        | ControlFrame::WorkspaceTerminalResize {
            protocol_version, ..
        }
        | ControlFrame::WorkspaceTerminalClose {
            protocol_version, ..
        }
        | ControlFrame::WorkspaceTerminalOutput {
            protocol_version, ..
        }
        | ControlFrame::WorkspaceTerminalStatus {
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
    result_payload: JsonValue,
    state_changed: bool,
}

#[derive(Clone, Default)]
struct TerminalSupervisor {
    manager: Arc<Mutex<WorkspaceTerminalManager>>,
}

impl TerminalSupervisor {
    async fn rebind_sender(&self, sender: &ControlFrameSender) {
        self.manager.lock().await.rebind_sender(sender).await;
    }

    async fn detach_sender(&self) {
        self.manager.lock().await.detach_sender().await;
    }

    async fn open(
        &self,
        config: &NodeConfig,
        command: &CommandEnvelope,
        sender: &ControlFrameSender,
    ) -> Result<WorkspaceTerminalOpenResponse, WorkspaceInspectError> {
        self.manager
            .lock()
            .await
            .open(config, command, sender)
            .await
    }

    async fn attach(&self, sender: &ControlFrameSender, terminal_id: &TerminalId) {
        self.manager.lock().await.attach(sender, terminal_id).await;
    }

    async fn input(&self, terminal_id: &TerminalId, data: String) {
        self.manager.lock().await.input(terminal_id, data);
    }

    async fn resize(&self, terminal_id: &TerminalId, cols: u16, rows: u16) {
        self.manager.lock().await.resize(terminal_id, cols, rows);
    }

    async fn close(&self, terminal_id: &TerminalId) {
        self.manager.lock().await.close(terminal_id).await;
    }

    async fn shutdown(&self) {
        self.manager.lock().await.shutdown().await;
    }
}

#[derive(Default)]
struct WorkspaceTerminalManager {
    terminals: HashMap<String, WorkspaceTerminalHandle>,
}

struct WorkspaceTerminalHandle {
    replay: Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
    control_tx: mpsc::UnboundedSender<WorkspaceTerminalControl>,
    sender_route: TerminalSenderRoute,
    task: tokio::task::JoinHandle<()>,
}

enum WorkspaceTerminalControl {
    Input(String),
    Resize { cols: u16, rows: u16 },
    Close,
}

impl WorkspaceTerminalManager {
    async fn rebind_sender(&self, sender: &ControlFrameSender) {
        for handle in self.terminals.values() {
            *handle.sender_route.write().await = Some(sender.clone());
        }
    }

    async fn detach_sender(&self) {
        for handle in self.terminals.values() {
            *handle.sender_route.write().await = None;
        }
    }

    async fn open(
        &mut self,
        config: &NodeConfig,
        command: &CommandEnvelope,
        sender: &ControlFrameSender,
    ) -> Result<WorkspaceTerminalOpenResponse, WorkspaceInspectError> {
        let request = workspace_command_payload::<WorkspaceTerminalOpenRequest>(command)?;
        let placement_id = workspace_command_placement_id(command)?;
        let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
        let shell = select_workspace_shell(request.shell_profile.as_deref())?;
        let cols = request
            .cols
            .clamp(MIN_WORKSPACE_TERMINAL_COLS, MAX_WORKSPACE_TERMINAL_COLS);
        let rows = request
            .rows
            .clamp(MIN_WORKSPACE_TERMINAL_ROWS, MAX_WORKSPACE_TERMINAL_ROWS);
        let terminal_id = TerminalId::from(format!("terminal-{}", command.command_id.as_str()));
        let (pty, pts) = pty_process::open()
            .map_err(|error| workspace_terminal_error("workspace_terminal.open_failed", error))?;
        pty.resize(PtySize::new(rows, cols))
            .map_err(|error| workspace_terminal_error("workspace_terminal.resize_failed", error))?;
        let child = PtyCommand::new(&shell)
            .current_dir(&workspace_root)
            .kill_on_drop(true)
            .spawn(pts)
            .map_err(|error| workspace_terminal_error("workspace_terminal.spawn_failed", error))?;
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let replay = Arc::new(RwLock::new(VecDeque::new()));
        let sender_route = Arc::new(RwLock::new(Some(sender.clone())));
        let now = Utc::now();
        let summary = WorkspaceTerminalSummary {
            placement_id: placement_id.clone(),
            terminal_id: terminal_id.clone(),
            title: workspace_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workspace")
                .to_owned(),
            cwd: workspace_root.display().to_string(),
            shell: shell.clone(),
            cols,
            rows,
            state: WorkspaceTerminalState::Running,
            exit_code: None,
            created_at: now,
            updated_at: now,
        };
        let task = tokio::spawn(run_workspace_terminal(
            pty,
            child,
            control_rx,
            sender_route.clone(),
            terminal_id.clone(),
            replay.clone(),
        ));
        self.terminals.insert(
            terminal_id.to_string(),
            WorkspaceTerminalHandle {
                replay: replay.clone(),
                control_tx,
                sender_route: sender_route.clone(),
                task,
            },
        );
        send_terminal_status(
            sender,
            &terminal_id,
            WorkspaceTerminalState::Running,
            None,
            Some("terminal started".to_owned()),
        )
        .await;
        Ok(WorkspaceTerminalOpenResponse {
            placement_id,
            terminal: summary,
            replay: vec![],
        })
    }

    async fn attach(&self, sender: &ControlFrameSender, terminal_id: &TerminalId) {
        let Some(handle) = self.terminals.get(terminal_id.as_str()) else {
            send_terminal_status(
                sender,
                terminal_id,
                WorkspaceTerminalState::Error,
                None,
                Some("terminal not found".to_owned()),
            )
            .await;
            return;
        };
        let replay = handle.replay.read().await;
        if replay.front().is_some_and(|frame| frame.seq > 1) {
            send_terminal_status(
                sender,
                terminal_id,
                WorkspaceTerminalState::Detached,
                None,
                Some("terminal replay gap; older output is no longer retained".to_owned()),
            )
            .await;
        }
        for frame in replay.iter() {
            let _ = send_frame(
                sender,
                ControlFrame::WorkspaceTerminalOutput {
                    frame_id: Uuid::new_v4().to_string(),
                    protocol_version: API_VERSION.to_owned(),
                    sent_at: frame.sent_at,
                    terminal_id: frame.terminal_id.clone(),
                    seq: frame.seq,
                    data: frame.data.clone(),
                },
            )
            .await;
        }
    }

    fn input(&self, terminal_id: &TerminalId, data: String) {
        if data.chars().count() > MAX_WORKSPACE_TERMINAL_INPUT_CHARS {
            return;
        }
        if let Some(handle) = self.terminals.get(terminal_id.as_str()) {
            let _ = handle
                .control_tx
                .send(WorkspaceTerminalControl::Input(data));
        }
    }

    fn resize(&self, terminal_id: &TerminalId, cols: u16, rows: u16) {
        if let Some(handle) = self.terminals.get(terminal_id.as_str()) {
            let _ = handle.control_tx.send(WorkspaceTerminalControl::Resize {
                cols: cols.clamp(MIN_WORKSPACE_TERMINAL_COLS, MAX_WORKSPACE_TERMINAL_COLS),
                rows: rows.clamp(MIN_WORKSPACE_TERMINAL_ROWS, MAX_WORKSPACE_TERMINAL_ROWS),
            });
        }
    }

    async fn close(&mut self, terminal_id: &TerminalId) {
        if let Some(handle) = self.terminals.remove(terminal_id.as_str()) {
            stop_terminal_handle(handle).await;
        }
    }

    async fn shutdown(&mut self) {
        let handles = self
            .terminals
            .drain()
            .map(|(_, handle)| handle)
            .collect::<Vec<_>>();
        for handle in handles {
            stop_terminal_handle(handle).await;
        }
    }
}

async fn run_workspace_terminal(
    mut pty: pty_process::Pty,
    mut child: tokio::process::Child,
    mut control_rx: mpsc::UnboundedReceiver<WorkspaceTerminalControl>,
    sender_route: TerminalSenderRoute,
    terminal_id: TerminalId,
    replay: Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
) {
    let mut seq = 0_u64;
    let mut read_buffer = vec![0_u8; WORKSPACE_TERMINAL_READ_BYTES];
    loop {
        tokio::select! {
            control = control_rx.recv() => {
                let Some(control) = control else {
                    break;
                };
                match control {
                    WorkspaceTerminalControl::Input(data) => {
                        if pty.write_all(data.as_bytes()).await.is_err() {
                            break;
                        }
                        let _ = pty.flush().await;
                    }
                    WorkspaceTerminalControl::Resize { cols, rows } => {
                        if let Err(error) = pty.resize(PtySize::new(rows, cols)) {
                            send_terminal_status_via_route(
                                &sender_route,
                                &terminal_id,
                                WorkspaceTerminalState::Error,
                                None,
                                Some(format!("resize failed: {error}")),
                            )
                            .await;
                        }
                    }
                    WorkspaceTerminalControl::Close => {
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        send_terminal_status_via_route(
                            &sender_route,
                            &terminal_id,
                            WorkspaceTerminalState::Closed,
                            None,
                            Some("terminal closed".to_owned()),
                        )
                        .await;
                        return;
                    }
                }
            }
            read_result = pty.read(&mut read_buffer) => {
                match read_result {
                    Ok(0) => break,
                    Ok(read) => {
                        seq = seq.saturating_add(1);
                        let data = String::from_utf8_lossy(&read_buffer[..read]).into_owned();
                        let sent_at = Utc::now();
                        record_terminal_replay(
                            &replay,
                            WorkspaceTerminalOutputFrame {
                                terminal_id: terminal_id.clone(),
                                seq,
                                data: data.clone(),
                                sent_at,
                            },
                        ).await;
                        let _ = send_terminal_frame(
                            &sender_route,
                            ControlFrame::WorkspaceTerminalOutput {
                                frame_id: Uuid::new_v4().to_string(),
                                protocol_version: API_VERSION.to_owned(),
                                sent_at,
                                terminal_id: terminal_id.clone(),
                                seq,
                                data,
                            },
                        ).await;
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        }
    }
    let exit_code = child.wait().await.ok().and_then(|status| status.code());
    send_terminal_status_via_route(
        &sender_route,
        &terminal_id,
        WorkspaceTerminalState::Exited,
        exit_code,
        Some("terminal exited".to_owned()),
    )
    .await;
}

async fn send_terminal_frame(
    route: &TerminalSenderRoute,
    frame: ControlFrame,
) -> anyhow::Result<()> {
    let sender = route.read().await.clone();
    let Some(sender) = sender else {
        return Ok(());
    };
    send_frame(&sender, frame).await
}

async fn send_terminal_status_via_route(
    route: &TerminalSenderRoute,
    terminal_id: &TerminalId,
    state: WorkspaceTerminalState,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let _ = send_terminal_frame(
        route,
        ControlFrame::WorkspaceTerminalStatus {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            terminal_id: terminal_id.clone(),
            state,
            exit_code,
            message,
        },
    )
    .await;
}

async fn record_terminal_replay(
    replay: &Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
    frame: WorkspaceTerminalOutputFrame,
) {
    let mut replay = replay.write().await;
    replay.push_back(frame);
    while replay.len() > MAX_WORKSPACE_TERMINAL_REPLAY_FRAMES
        || terminal_replay_bytes(&replay) > MAX_WORKSPACE_TERMINAL_REPLAY_BYTES
    {
        replay.pop_front();
    }
}

fn terminal_replay_bytes(replay: &VecDeque<WorkspaceTerminalOutputFrame>) -> usize {
    replay.iter().map(|frame| frame.data.len()).sum()
}

async fn stop_terminal_handle(handle: WorkspaceTerminalHandle) {
    let _ = handle.control_tx.send(WorkspaceTerminalControl::Close);
    join_terminal_task(handle.task).await;
}

async fn join_terminal_task(mut task: tokio::task::JoinHandle<()>) {
    tokio::select! {
        result = &mut task => {
            if let Err(error) = result {
                tracing::warn!(error = %error, "workspace terminal task failed");
            }
        }
        _ = tokio::time::sleep(WORKSPACE_TERMINAL_SHUTDOWN_TIMEOUT) => {
            task.abort();
            let _ = task.await;
            tracing::warn!("workspace terminal task aborted after shutdown timeout");
        }
    }
}

async fn send_terminal_status(
    sender: &ControlFrameSender,
    terminal_id: &TerminalId,
    state: WorkspaceTerminalState,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let _ = send_frame(
        sender,
        ControlFrame::WorkspaceTerminalStatus {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            terminal_id: terminal_id.clone(),
            state,
            exit_code,
            message,
        },
    )
    .await;
}

fn workspace_terminal_error(
    code: &'static str,
    error: impl std::fmt::Display,
) -> WorkspaceInspectError {
    WorkspaceInspectError::new(code, format!("Workspace terminal failed: {error}"))
}

fn select_workspace_shell(profile: Option<&str>) -> Result<String, WorkspaceInspectError> {
    match profile.unwrap_or("default").trim() {
        "" | "default" => Ok(default_workspace_shell()),
        "sh" => Ok(shell_path("sh")),
        "bash" => Ok(shell_path("bash")),
        "zsh" => Ok(shell_path("zsh")),
        _ => Err(WorkspaceInspectError::new(
            "workspace_terminal.shell_denied",
            "Workspace terminal shell profile is not allowed by node policy",
        )),
    }
}

fn default_workspace_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .and_then(|shell| {
            let name = Path::new(&shell).file_name()?.to_str()?;
            matches!(name, "sh" | "bash" | "zsh").then_some(shell)
        })
        .unwrap_or_else(|| shell_path("sh"))
}

fn shell_path(name: &str) -> String {
    ["/bin", "/usr/bin"]
        .iter()
        .map(|prefix| Path::new(prefix).join(name))
        .find(|path| path.exists())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| name.to_owned())
}

struct NodeLiveEventSink<'a> {
    runtime_states: &'a mut HashMap<String, RuntimeSessionState>,
}

impl<'a> NodeLiveEventSink<'a> {
    fn new(runtime_states: &'a mut HashMap<String, RuntimeSessionState>) -> Self {
        Self { runtime_states }
    }

    fn emit(&mut self, event: &EventEnvelope) {
        apply_runtime_state_projection_for_event(self.runtime_states, event);
    }
}

#[cfg(test)]
async fn prepare_command_dispatch(
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
) -> CommandDispatchOutcome {
    prepare_command_dispatch_with_live_socket(config, local_state, command, None, None, None).await
}

async fn prepare_command_dispatch_with_live_socket(
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
    live_sender: Option<&ControlFrameSender>,
    terminal_supervisor: Option<&TerminalSupervisor>,
    cancellation: Option<watch::Receiver<bool>>,
) -> CommandDispatchOutcome {
    if !command.payload.matches_kind(command.kind) {
        return CommandDispatchOutcome {
            status: CommandState::Failed,
            events_to_send: vec![],
            result_payload: JsonValue(serde_json::json!({
                "error_code": "protocol.command_payload_mismatch",
                "message": "Command payload does not match its command kind",
                "retryable": false,
            })),
            state_changed: false,
        };
    }
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
            result_payload: local_state
                .command_result_payloads
                .get(command.command_id.as_str())
                .cloned()
                .unwrap_or_else(|| JsonValue(serde_json::json!({}))),
            state_changed: false,
        };
    }

    let result_payload = JsonValue(serde_json::json!({}));
    let events = match command.kind {
        CommandKind::ValidateWorkspace => {
            workspace_validation_events(config, command, &mut local_state.placement_seqs)
        }
        CommandKind::RefreshResourceSnapshot => {
            resource_snapshot_events(config, command, &mut local_state.placement_seqs)
        }
        CommandKind::ListWorkspaceTree => {
            let (status, payload) = workspace_tree_command_result(config, command);
            record_command_result_payload(local_state, command, status, &payload);
            return CommandDispatchOutcome {
                status,
                events_to_send: vec![],
                result_payload: payload,
                state_changed: true,
            };
        }
        CommandKind::ReadWorkspaceFile => {
            let (status, payload) = workspace_file_command_result(config, command);
            record_command_result_payload(local_state, command, status, &payload);
            return CommandDispatchOutcome {
                status,
                events_to_send: vec![],
                result_payload: payload,
                state_changed: true,
            };
        }
        CommandKind::WriteWorkspaceFile => {
            let (status, payload) = workspace_file_write_command_result(config, command);
            let events =
                causal_workspace_events(command, status, &payload, &mut local_state.placement_seqs);
            return workspace_command_outcome(local_state, command, status, payload, events);
        }
        CommandKind::RunWorkspaceCommand => {
            let (status, payload) = workspace_command_run_command_result(config, command).await;
            let events =
                causal_workspace_events(command, status, &payload, &mut local_state.placement_seqs);
            return workspace_command_outcome(local_state, command, status, payload, events);
        }
        CommandKind::ReadWorkspaceDiff => {
            let (status, payload) = workspace_diff_command_result(config, command).await;
            let events =
                causal_workspace_events(command, status, &payload, &mut local_state.placement_seqs);
            return workspace_command_outcome(local_state, command, status, payload, events);
        }
        CommandKind::OpenWorkspaceTerminal => {
            let (status, payload) = workspace_terminal_open_command_result(
                config,
                command,
                live_sender,
                terminal_supervisor,
            )
            .await;
            record_command_result_payload(local_state, command, status, &payload);
            return CommandDispatchOutcome {
                status,
                events_to_send: vec![],
                result_payload: payload,
                state_changed: true,
            };
        }
        CommandKind::RequestDeduction => {
            let deduction_id = match &command.payload {
                CommandPayload::RequestDeduction { package } => package.deduction_id.to_string(),
                _ => String::new(),
            };
            if local_state.cancelled_deductions.remove(&deduction_id) {
                let (status, payload) = deduction_error_payload(
                    command,
                    "cancelled",
                    "deduction.cancelled",
                    "Deduction was cancelled before provider execution",
                );
                record_command_result_payload(local_state, command, status, &payload);
                return CommandDispatchOutcome {
                    status,
                    events_to_send: vec![],
                    result_payload: payload,
                    state_changed: true,
                };
            }
            if cancellation
                .as_ref()
                .is_some_and(|receiver| *receiver.borrow())
            {
                let (status, payload) = deduction_error_payload(
                    command,
                    "cancelled",
                    "deduction.cancelled",
                    "Deduction was cancelled before provider execution",
                );
                record_command_result_payload(local_state, command, status, &payload);
                return CommandDispatchOutcome {
                    status,
                    events_to_send: vec![],
                    result_payload: payload,
                    state_changed: true,
                };
            }
            let provider_key =
                provider_for_command(local_state, command).unwrap_or_else(|| "unknown".to_owned());
            let workspace_path = workspace_path_for_command(local_state, command);
            let (status, payload) = RuntimeManager::for_provider(&provider_key, config)
                .execute_deduction(command, workspace_path.as_deref(), cancellation)
                .await;
            record_command_result_payload(local_state, command, status, &payload);
            return CommandDispatchOutcome {
                status,
                events_to_send: vec![],
                result_payload: payload,
                state_changed: true,
            };
        }
        CommandKind::CancelDeduction => {
            let CommandPayload::CancelDeduction { deduction_id } = &command.payload else {
                unreachable!("command payload kind was validated before dispatch")
            };
            remember_cancelled_deduction(
                &mut local_state.cancelled_deductions,
                deduction_id.to_string(),
            );
            let payload = JsonValue(serde_json::json!({
                "deduction_id": deduction_id.as_str(),
                "cancelled": true,
            }));
            record_command_result_payload(local_state, command, CommandState::Completed, &payload);
            return CommandDispatchOutcome {
                status: CommandState::Completed,
                events_to_send: vec![],
                result_payload: payload,
                state_changed: true,
            };
        }
        _ => {
            let provider_key =
                provider_for_command(local_state, command).unwrap_or_else(|| "unknown".to_owned());
            if let Err(error) = remember_runtime_metadata(config, local_state, command) {
                runtime_workspace_error_events(
                    &provider_key,
                    command,
                    &mut local_state.runtime_seqs,
                    error,
                )
            } else {
                let provider_key = provider_for_command(local_state, command);
                let workspace_path = workspace_path_for_command(local_state, command);
                let mut live_event_sink =
                    live_sender.map(|_| NodeLiveEventSink::new(&mut local_state.runtime_states));
                let events = if let Some(provider_key) = provider_key {
                    RuntimeManager::for_provider(&provider_key, config)
                        .execute_command(
                            command,
                            &mut local_state.runtime_seqs,
                            workspace_path.as_deref(),
                            &mut local_state.runtime_transcripts,
                            &mut local_state.runtime_provider_resume_refs,
                            live_event_sink.as_mut(),
                            cancellation,
                        )
                        .await
                } else {
                    missing_provider_events_for_command(command, &mut local_state.runtime_seqs)
                };
                events
            }
        }
    };
    let unsent_events = events.clone();
    apply_runtime_state_projection(local_state, &unsent_events);
    local_state
        .event_outbox
        .extend(unsent_events.iter().cloned());
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
        result_payload,
        state_changed: true,
    }
}

fn record_command_result_payload(
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
    status: CommandState,
    payload: &JsonValue,
) {
    local_state
        .command_status
        .insert(command.command_id.to_string(), status);
    local_state
        .command_result_payloads
        .insert(command.command_id.to_string(), payload.clone());
}

fn workspace_command_outcome(
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
    status: CommandState,
    payload: JsonValue,
    events: Vec<EventEnvelope>,
) -> CommandDispatchOutcome {
    record_command_result_payload(local_state, command, status, &payload);
    local_state.event_outbox.extend(events.iter().cloned());
    let retention_notices = enforce_event_outbox_retention(local_state, MAX_EVENT_OUTBOX_EVENTS);
    let mut events_to_send = events;
    events_to_send.extend(retention_notices);
    CommandDispatchOutcome {
        status,
        events_to_send,
        result_payload: payload,
        state_changed: true,
    }
}

fn causal_workspace_events(
    command: &CommandEnvelope,
    status: CommandState,
    payload: &JsonValue,
    placement_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    if status != CommandState::Completed {
        return vec![];
    }
    let mut event = match command.kind {
        CommandKind::WriteWorkspaceFile => {
            let Ok(result) =
                serde_json::from_value::<WorkspaceFileWriteResponse>(payload.0.clone())
            else {
                return vec![];
            };
            let mut event = placement_event_for_command(
                command,
                placement_seqs,
                result.placement_id.clone(),
                EventKind::WorkspaceFileWritten,
                payload.0.clone(),
            );
            event.result_refs = vec![
                UpravaRef::WorkspaceEdit {
                    edit_id: result.edit_id,
                    placement_id: Some(result.placement_id.clone()),
                    path: Some(result.path.clone()),
                },
                UpravaRef::File {
                    placement_id: result.placement_id,
                    path: result.path,
                    version: result.metadata.modified_at.map(|value| value.to_rfc3339()),
                },
            ];
            event
        }
        CommandKind::RunWorkspaceCommand => {
            let Ok(result) =
                serde_json::from_value::<WorkspaceCommandRunResponse>(payload.0.clone())
            else {
                return vec![];
            };
            let event_kind = if result.intent == WorkspaceCommandIntent::Check {
                EventKind::WorkspaceCheckCompleted
            } else {
                EventKind::WorkspaceCommandCompleted
            };
            let mut event = placement_event_for_command(
                command,
                placement_seqs,
                result.placement_id,
                event_kind,
                payload.0.clone(),
            );
            let terminal_command_ref = UpravaRef::TerminalCommand {
                terminal_command_id: result.terminal_command_id.clone(),
                terminal_id: None,
            };
            event.evidence_refs = vec![UpravaRef::TerminalOutputRange {
                terminal_command_id: result.terminal_command_id.clone(),
                range: TextRange {
                    start_line: Some(1),
                    end_line: None,
                    start_offset: Some(0),
                    end_offset: Some(
                        result
                            .stdout
                            .chars()
                            .count()
                            .saturating_add(result.stderr.chars().count())
                            as i64,
                    ),
                },
            }];
            event.result_refs = vec![terminal_command_ref];
            if result.intent == WorkspaceCommandIntent::Check {
                event.result_refs.push(UpravaRef::CheckResult {
                    check_run_id: result.terminal_command_id,
                    failure_id: (!result.success).then(|| "command_failed".to_owned()),
                });
            }
            event
        }
        CommandKind::ReadWorkspaceDiff => {
            let Ok(result) = serde_json::from_value::<WorkspaceDiffResponse>(payload.0.clone())
            else {
                return vec![];
            };
            let mut event = placement_event_for_command(
                command,
                placement_seqs,
                result.placement_id.clone(),
                EventKind::WorkspaceDiffObserved,
                payload.0.clone(),
            );
            event.result_refs = vec![UpravaRef::WorkspaceDiff {
                diff_id: result.diff_id,
                placement_id: result.placement_id,
            }];
            event
        }
        _ => return vec![],
    };
    event.cause_refs.push(UpravaRef::Command {
        command_id: command.command_id.clone(),
    });
    vec![event]
}

fn command_status_for_events(events: &[EventEnvelope]) -> CommandState {
    if events.is_empty() {
        return CommandState::Failed;
    }
    if events.iter().any(|event| {
        matches!(
            event.kind,
            EventKind::RuntimeError | EventKind::TurnInterrupted
        )
    }) {
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
    enforce_event_outbox_retention_with_limits(
        local_state,
        max_events,
        MAX_EVENT_OUTBOX_AGE,
        MAX_EVENT_OUTBOX_BYTES,
    )
}

fn enforce_event_outbox_retention_with_limits(
    local_state: &mut NodeLocalState,
    max_events: usize,
    max_age: Duration,
    max_bytes: usize,
) -> Vec<EventEnvelope> {
    let drop_count = event_outbox_retention_drop_count(
        &local_state.event_outbox,
        max_events,
        max_age,
        max_bytes,
    );
    if drop_count == 0 {
        return vec![];
    }

    local_state.dropped_event_count = local_state
        .dropped_event_count
        .saturating_add(drop_count as u64);
    let dropped = local_state
        .event_outbox
        .drain(0..drop_count)
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
        trim_event_outbox_to_limits(&mut local_state.event_outbox, max_events, max_bytes);
        return notices;
    }

    apply_runtime_state_projection(local_state, &notices);
    local_state.event_outbox.extend(notices.iter().cloned());
    trim_event_outbox_to_limits(&mut local_state.event_outbox, max_events, max_bytes);
    notices
}

fn event_outbox_retention_drop_count(
    outbox: &[EventEnvelope],
    max_events: usize,
    max_age: Duration,
    max_bytes: usize,
) -> usize {
    if outbox.is_empty() {
        return 0;
    }
    let cutoff = if max_age.is_zero() {
        None
    } else {
        chrono::Duration::from_std(max_age)
            .ok()
            .map(|age| Utc::now() - age)
    };
    let event_sizes = outbox.iter().map(serialized_event_len).collect::<Vec<_>>();
    let mut retained_bytes = event_sizes.iter().sum::<usize>();
    let mut dropped = 0usize;
    while dropped < outbox.len() {
        let retained_count = outbox.len() - dropped;
        let count_exceeded = max_events > 0 && retained_count > max_events;
        let age_exceeded = cutoff
            .as_ref()
            .is_some_and(|cutoff| outbox[dropped].happened_at < *cutoff);
        let bytes_exceeded = max_bytes > 0 && retained_bytes > max_bytes;
        if !(count_exceeded || age_exceeded || bytes_exceeded) {
            break;
        }
        retained_bytes = retained_bytes.saturating_sub(event_sizes[dropped]);
        dropped += 1;
    }
    dropped
}

fn trim_event_outbox_to_limits(
    outbox: &mut Vec<EventEnvelope>,
    max_events: usize,
    max_bytes: usize,
) {
    loop {
        let count_exceeded = max_events > 0 && outbox.len() > max_events;
        let bytes_exceeded =
            max_bytes > 0 && outbox.iter().map(serialized_event_len).sum::<usize>() > max_bytes;
        if !(count_exceeded || bytes_exceeded) {
            break;
        }
        if outbox.is_empty() {
            break;
        }
        outbox.remove(0);
    }
}

fn serialized_event_len(event: &EventEnvelope) -> usize {
    serde_json::to_vec(event)
        .map(|value| value.len())
        .unwrap_or(usize::MAX)
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
        session_projection_seq: None,
        kind: EventKind::RuntimeError,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: vec![],
        result_refs: vec![],
        payload: EventPayload::from_json(
            EventKind::RuntimeError,
            serde_json::json!({
                "code": "node.event_outbox_retention_exceeded",
                "message": "Node dropped unacknowledged runtime events because local outbox retention was exceeded",
            }),
        ),
    }
}

fn remember_runtime_metadata(
    config: &NodeConfig,
    local_state: &mut NodeLocalState,
    command: &CommandEnvelope,
) -> Result<bool, WorkspaceInspectError> {
    let Some(runtime_session_id) = command.target.runtime_session_id() else {
        return Ok(false);
    };
    let runtime_key = runtime_session_id.to_string();
    let mut changed = false;
    let canonical_workspace_path = if matches!(
        command.kind,
        CommandKind::StartRuntime | CommandKind::ResumeRuntime
    ) {
        match command_payload_str(command, "workspace_path") {
            Some(workspace_path) => Some(canonical_workspace_root(config, workspace_path)?),
            None => None,
        }
    } else {
        None
    };

    if matches!(command.kind, CommandKind::StartRuntime) {
        if let Some(provider) = command_payload_str(command, "provider") {
            changed |= insert_if_changed(
                &mut local_state.runtime_providers,
                runtime_key.clone(),
                provider.to_owned(),
            );
        }
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
        if let Some(workspace_path) = canonical_workspace_path {
            changed |= insert_if_changed(
                &mut local_state.runtime_workspace_paths,
                runtime_key,
                workspace_path.display().to_string(),
            );
        }
    }

    Ok(changed)
}

fn provider_for_command(local_state: &NodeLocalState, command: &CommandEnvelope) -> Option<String> {
    command_payload_str(command, "provider")
        .map(str::to_owned)
        .or_else(|| {
            command
                .target
                .runtime_session_id()
                .and_then(|runtime_session_id| {
                    local_state
                        .runtime_providers
                        .get(runtime_session_id.as_str())
                        .cloned()
                })
        })
}

fn workspace_path_for_command(
    local_state: &NodeLocalState,
    command: &CommandEnvelope,
) -> Option<String> {
    command_payload_str(command, "workspace_path")
        .map(str::to_owned)
        .or_else(|| {
            command
                .target
                .runtime_session_id()
                .and_then(|runtime_session_id| {
                    local_state
                        .runtime_workspace_paths
                        .get(runtime_session_id.as_str())
                        .cloned()
                })
        })
}

fn command_payload_str<'a>(command: &'a CommandEnvelope, key: &str) -> Option<&'a str> {
    match key {
        "provider" => command.payload.provider(),
        "workspace_path" => command.payload.workspace_path(),
        "display_name" => command.payload.display_name(),
        "path" => command.payload.path(),
        _ => None,
    }
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
        .provider_resume_ref()
        .map(|value| &value.0)
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
        apply_runtime_state_projection_for_event(&mut local_state.runtime_states, event);
    }
}

fn apply_runtime_state_projection_for_event(
    runtime_states: &mut HashMap<String, RuntimeSessionState>,
    event: &EventEnvelope,
) {
    let Some(runtime_session_id) = &event.runtime_session_id else {
        return;
    };
    let Some(state) = runtime_state_for_event(event.kind) else {
        return;
    };
    runtime_states.insert(runtime_session_id.to_string(), state);
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
    let Some(project_placement_id) = command.target.project_placement_id().cloned() else {
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
    if let Ok(canonical_path) = std::fs::canonicalize(path) {
        return canonical_workspace_path_allowed(config, &canonical_path);
    }
    path.ancestors()
        .skip(1)
        .find_map(|ancestor| std::fs::canonicalize(ancestor).ok())
        .is_some_and(|ancestor| canonical_workspace_path_allowed(config, &ancestor))
}

fn workspace_tree_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_tree_response(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

fn workspace_file_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_file_response(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

fn workspace_file_write_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match write_workspace_file(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

async fn workspace_command_run_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match run_workspace_command(config, command).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

async fn workspace_diff_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_diff_response(config, command).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

async fn workspace_terminal_open_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
    live_sender: Option<&ControlFrameSender>,
    terminal_supervisor: Option<&TerminalSupervisor>,
) -> (CommandState, JsonValue) {
    let Some(sender) = live_sender else {
        return (
            CommandState::Failed,
            WorkspaceInspectError::new(
                "workspace_terminal.control_unavailable",
                "Workspace terminal requires a live node control channel",
            )
            .into_payload(),
        );
    };
    let Some(terminal_supervisor) = terminal_supervisor else {
        return (
            CommandState::Failed,
            WorkspaceInspectError::new(
                "workspace_terminal.manager_unavailable",
                "Workspace terminal manager is unavailable",
            )
            .into_payload(),
        );
    };
    match terminal_supervisor.open(config, command, sender).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

fn workspace_success_payload<T: Serialize>(value: T) -> (CommandState, JsonValue) {
    match serde_json::to_value(value) {
        Ok(value) => (CommandState::Completed, JsonValue(value)),
        Err(error) => (
            CommandState::Failed,
            JsonValue(serde_json::json!({
                "error_code": "workspace.serialization_failed",
                "message": error.to_string(),
            })),
        ),
    }
}

fn write_workspace_file(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceFileWriteResponse, WorkspaceInspectError> {
    let request = workspace_command_payload::<WorkspaceFileWriteRequest>(command)?;
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let relative_path = safe_workspace_relative_path(&request.path)?;
    if relative_path.as_os_str().is_empty() {
        return Err(WorkspaceInspectError::new(
            "workspace.path_required",
            "Workspace file write requires a file path",
        ));
    }
    if let Some(status) = generated_or_ignored_status(&relative_path) {
        return Err(WorkspaceInspectError::new(
            "workspace.protected_path",
            format!(
                "Workspace file writes cannot target {} paths",
                workspace_entry_status_label(status)
            ),
        ));
    }
    if request.content.len() > MAX_WORKSPACE_TEXT_BYTES as usize {
        return Err(WorkspaceInspectError::new(
            "workspace.write_too_large",
            format!(
                "Workspace file writes are limited to {} bytes",
                MAX_WORKSPACE_TEXT_BYTES
            ),
        ));
    }
    if request.content.as_bytes().contains(&0) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_content",
            "Workspace file writes only accept text content",
        ));
    }

    let parent_relative = relative_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = relative_path.file_name().ok_or_else(|| {
        WorkspaceInspectError::new(
            "workspace.path_required",
            "Workspace file write requires a file name",
        )
    })?;
    let parent_path = resolve_existing_workspace_path(&workspace_root, parent_relative)?;
    let parent_metadata = fs::symlink_metadata(&parent_path).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.parent_metadata_failed",
            format!("Failed to inspect parent directory: {error}"),
        )
    })?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(WorkspaceInspectError::new(
            "workspace.parent_not_directory",
            "Workspace file write parent is not a directory",
        ));
    }
    let target_path = parent_path.join(file_name);
    if binary_extension(&relative_path) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes do not edit binary file types",
        ));
    }

    let mut file = open_workspace_write_target(
        &target_path,
        &relative_path,
        request.expected_content.as_deref(),
    )?;
    file.set_len(0).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to truncate {}: {error}", relative_path.display()),
        )
    })?;
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to seek {}: {error}", relative_path.display()),
        )
    })?;
    file.write_all(request.content.as_bytes())
        .map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.write_failed",
                format!("Failed to write {}: {error}", relative_path.display()),
            )
        })?;
    file.sync_all().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to sync {}: {error}", relative_path.display()),
        )
    })?;

    let metadata = file.metadata().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.metadata_failed",
            format!(
                "Failed to inspect written file {}: {error}",
                relative_path.display()
            ),
        )
    })?;
    Ok(WorkspaceFileWriteResponse {
        placement_id,
        path: relative_path_string(&relative_path),
        metadata: WorkspaceEntry {
            name: workspace_entry_name(&relative_path),
            path: relative_path_string(&relative_path),
            kind: workspace_entry_kind(&metadata),
            status: workspace_status_for_entry(&relative_path, &metadata),
            classification: workspace_entry_classification(&relative_path),
            expandable: false,
            byte_len: metadata.is_file().then_some(metadata.len()),
            modified_at: metadata_modified_at(&metadata),
            children: vec![],
        },
        edit_id: format!("workspace-edit-{}", command.command_id.as_str()),
        written_at: Utc::now(),
    })
}

fn open_workspace_write_target(
    target_path: &Path,
    relative_path: &Path,
    expected_content: Option<&str>,
) -> Result<fs::File, WorkspaceInspectError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    set_no_follow(&mut options);
    match options.open(target_path) {
        Ok(mut file) => {
            validate_opened_write_target(&mut file, relative_path, expected_content)?;
            Ok(file)
        }
        Err(error) if error.kind() == ErrorKind::NotFound && expected_content.is_none() => {
            let mut create_options = OpenOptions::new();
            create_options.read(true).write(true).create_new(true);
            set_no_follow(&mut create_options);
            create_options.open(target_path).map_err(|error| {
                if is_symlink_open_error(&error) {
                    WorkspaceInspectError::new(
                        "workspace.write_symlink",
                        "Workspace file writes do not follow symlinks",
                    )
                } else if error.kind() == ErrorKind::AlreadyExists {
                    WorkspaceInspectError::new(
                        "workspace.write_conflict",
                        "Workspace file changed before save; reload before writing",
                    )
                } else {
                    WorkspaceInspectError::new(
                        "workspace.write_failed",
                        format!(
                            "Failed to create {} for writing: {error}",
                            relative_path.display()
                        ),
                    )
                }
            })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(WorkspaceInspectError::new(
            "workspace.write_conflict",
            "Workspace file changed before save: target is now missing",
        )),
        Err(error) if is_symlink_open_error(&error) => Err(WorkspaceInspectError::new(
            "workspace.write_symlink",
            "Workspace file writes do not follow symlinks",
        )),
        Err(error) => Err(WorkspaceInspectError::new(
            "workspace.write_failed",
            format!(
                "Failed to open {} for writing: {error}",
                relative_path.display()
            ),
        )),
    }
}

fn validate_opened_write_target(
    file: &mut fs::File,
    relative_path: &Path,
    expected_content: Option<&str>,
) -> Result<(), WorkspaceInspectError> {
    let metadata = file.metadata().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.metadata_failed",
            format!("Failed to inspect {}: {error}", relative_path.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(WorkspaceInspectError::new(
            "workspace.write_not_file",
            "Workspace file write target is not a file",
        ));
    }
    if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Err(WorkspaceInspectError::new(
            "workspace.write_large_file",
            "Workspace file write target is too large for lightweight editing",
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.read_failed",
            format!(
                "Failed to seek {} before writing: {error}",
                relative_path.display()
            ),
        )
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.read_failed",
            format!(
                "Failed to read {} before writing: {error}",
                relative_path.display()
            ),
        )
    })?;
    if bytes.contains(&0) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes do not edit binary content",
        ));
    }
    let current_content = String::from_utf8(bytes).map_err(|_| {
        WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes only edit UTF-8 text",
        )
    })?;
    if let Some(expected_content) = expected_content {
        if current_content != expected_content {
            return Err(WorkspaceInspectError::new(
                "workspace.write_conflict",
                "Workspace file changed before save; reload before writing",
            ));
        }
    }
    Ok(())
}

fn set_no_follow(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    {
        let _ = options;
    }
}

fn is_symlink_open_error(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::ELOOP)
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

async fn run_workspace_command(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceCommandRunResponse, WorkspaceInspectError> {
    let request = workspace_command_payload::<WorkspaceCommandRunRequest>(command)?;
    validate_workspace_command_request(&request)?;
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let timeout_seconds = request
        .timeout_seconds
        .unwrap_or(30)
        .clamp(1, MAX_WORKSPACE_COMMAND_SECONDS);
    let output = run_workspace_process(
        &workspace_root,
        request.command.trim(),
        &request.args,
        Duration::from_secs(timeout_seconds),
        MAX_WORKSPACE_COMMAND_OUTPUT_BYTES,
        MAX_WORKSPACE_COMMAND_OUTPUT_BYTES,
    )
    .await;
    Ok(WorkspaceCommandRunResponse {
        placement_id,
        terminal_command_id: format!("terminal-command-{}", command.command_id.as_str()),
        command: request.command.trim().to_owned(),
        args: request.args,
        intent: request.intent,
        label: request.label,
        exit_code: output.exit_code,
        success: output.success,
        stdout: output.stdout,
        stderr: output.stderr,
        stdout_truncated: output.stdout_truncated,
        stderr_truncated: output.stderr_truncated,
        duration_ms: output.duration_ms,
        started_at: output.started_at,
        completed_at: output.completed_at,
    })
}

async fn build_workspace_diff_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceDiffResponse, WorkspaceInspectError> {
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let inside = run_workspace_process(
        &workspace_root,
        "git",
        &["rev-parse".to_owned(), "--is-inside-work-tree".to_owned()],
        Duration::from_secs(10),
        1_024,
        4_096,
    )
    .await;
    if !inside.success || inside.stdout.trim() != "true" {
        return Ok(WorkspaceDiffResponse {
            placement_id,
            diff_id: format!("workspace-diff-{}", command.command_id.as_str()),
            summary: "Workspace is not a git worktree".to_owned(),
            diff: inside.stderr,
            summary_truncated: false,
            diff_truncated: inside.stderr_truncated,
            generated_at: Utc::now(),
        });
    }
    let summary = run_workspace_process(
        &workspace_root,
        "git",
        &["diff".to_owned(), "--stat".to_owned()],
        Duration::from_secs(10),
        WORKSPACE_DIFF_STAT_BYTES,
        4_096,
    )
    .await;
    let diff = run_workspace_process(
        &workspace_root,
        "git",
        &["diff".to_owned(), "--".to_owned()],
        Duration::from_secs(10),
        WORKSPACE_DIFF_BYTES,
        4_096,
    )
    .await;
    let mut summary_text = summary.stdout;
    if !summary.stderr.trim().is_empty() {
        if !summary_text.is_empty() {
            summary_text.push('\n');
        }
        summary_text.push_str(&summary.stderr);
    }
    let mut diff_text = diff.stdout;
    if !diff.stderr.trim().is_empty() {
        if !diff_text.is_empty() {
            diff_text.push('\n');
        }
        diff_text.push_str(&diff.stderr);
    }
    Ok(WorkspaceDiffResponse {
        placement_id,
        diff_id: format!("workspace-diff-{}", command.command_id.as_str()),
        summary: if summary_text.trim().is_empty() {
            "No unstaged diff".to_owned()
        } else {
            summary_text
        },
        diff: diff_text,
        summary_truncated: summary.stdout_truncated || summary.stderr_truncated,
        diff_truncated: diff.stdout_truncated || diff.stderr_truncated,
        generated_at: Utc::now(),
    })
}

fn workspace_command_payload<T: for<'de> Deserialize<'de>>(
    command: &CommandEnvelope,
) -> Result<T, WorkspaceInspectError> {
    command.payload.workspace_request().ok_or_else(|| {
        WorkspaceInspectError::new(
            "workspace.invalid_payload",
            "Workspace command payload does not match its command kind",
        )
    })
}

fn validate_workspace_command_request(
    request: &WorkspaceCommandRunRequest,
) -> Result<(), WorkspaceInspectError> {
    let command = request.command.trim();
    if command.is_empty() {
        return Err(WorkspaceInspectError::new(
            "workspace.command_required",
            "Workspace command requires an executable name",
        ));
    }
    if command.chars().count() > MAX_WORKSPACE_COMMAND_ARG_CHARS
        || command.contains('\0')
        || command.contains('/')
        || command.contains('\\')
    {
        return Err(WorkspaceInspectError::new(
            "workspace.command_invalid",
            "Workspace command executable must be a program name, not a path",
        ));
    }
    if !ALLOWED_WORKSPACE_COMMANDS.contains(&command) {
        return Err(WorkspaceInspectError::new(
            "workspace.command_not_allowed",
            format!("Workspace command `{command}` is not allowed by node policy"),
        ));
    }
    if request.args.len() > MAX_WORKSPACE_COMMAND_ARGS {
        return Err(WorkspaceInspectError::new(
            "workspace.command_too_many_args",
            format!(
                "Workspace commands accept at most {} arguments",
                MAX_WORKSPACE_COMMAND_ARGS
            ),
        ));
    }
    if request
        .args
        .iter()
        .any(|arg| arg.contains('\0') || arg.chars().count() > MAX_WORKSPACE_COMMAND_ARG_CHARS)
    {
        return Err(WorkspaceInspectError::new(
            "workspace.command_arg_invalid",
            format!(
                "Workspace command arguments are limited to {} characters",
                MAX_WORKSPACE_COMMAND_ARG_CHARS
            ),
        ));
    }
    Ok(())
}

struct WorkspaceProcessOutput {
    exit_code: Option<i32>,
    success: bool,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    duration_ms: u64,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
}

async fn run_workspace_process(
    workspace_root: &Path,
    command_name: &str,
    args: &[String],
    timeout_duration: Duration,
    stdout_cap: usize,
    stderr_cap: usize,
) -> WorkspaceProcessOutput {
    let started_at = Utc::now();
    let started = Instant::now();
    let mut command = TokioCommand::new(command_name);
    command
        .args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return WorkspaceProcessOutput {
                exit_code: None,
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to start `{command_name}`: {error}"),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: duration_millis(started),
                started_at,
                completed_at: Utc::now(),
            };
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_task = tokio::spawn(read_capped_process_output(stdout, stdout_cap));
    let stderr_task = tokio::spawn(read_capped_process_output(stderr, stderr_cap));
    let wait_result = timeout(timeout_duration, child.wait()).await;
    let completed_at = Utc::now();
    let duration_ms = duration_millis(started);
    match wait_result {
        Ok(Ok(status)) => {
            let (stdout, stdout_truncated) = join_capped_output(stdout_task).await;
            let (stderr, stderr_truncated) = join_capped_output(stderr_task).await;
            WorkspaceProcessOutput {
                exit_code: status.code(),
                success: status.success(),
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                duration_ms,
                started_at,
                completed_at,
            }
        }
        Ok(Err(error)) => WorkspaceProcessOutput {
            exit_code: None,
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to start `{command_name}`: {error}"),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms,
            started_at,
            completed_at,
        },
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let (stdout, stdout_truncated) = join_capped_output(stdout_task).await;
            let (mut stderr, stderr_truncated) = join_capped_output(stderr_task).await;
            let timeout_message = format!(
                "`{command_name}` timed out after {} seconds",
                timeout_duration.as_secs()
            );
            if stderr.trim().is_empty() {
                stderr = timeout_message;
            } else {
                stderr.push('\n');
                stderr.push_str(&timeout_message);
            }
            WorkspaceProcessOutput {
                exit_code: None,
                success: false,
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                duration_ms,
                started_at,
                completed_at,
            }
        }
    }
}

async fn read_capped_process_output<R>(
    reader: Option<R>,
    cap: usize,
) -> std::io::Result<(String, bool)>
where
    R: AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Ok((String::new(), false));
    };
    let mut buffer = [0_u8; 8192];
    let mut collected = Vec::with_capacity(cap.min(8192));
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = cap.saturating_sub(collected.len());
        if remaining > 0 {
            let keep = read.min(remaining);
            collected.extend_from_slice(&buffer[..keep]);
            truncated |= keep < read;
        } else {
            truncated = true;
        }
    }
    Ok((String::from_utf8_lossy(&collected).into_owned(), truncated))
}

async fn join_capped_output(
    task: tokio::task::JoinHandle<std::io::Result<(String, bool)>>,
) -> (String, bool) {
    match task.await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => (format!("failed to read process output: {error}"), false),
        Err(error) => (
            format!("failed to join process output reader: {error}"),
            false,
        ),
    }
}

fn duration_millis(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        millis as u64
    }
}

#[derive(Debug)]
struct WorkspaceInspectError {
    code: &'static str,
    message: String,
}

impl WorkspaceInspectError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn into_payload(self) -> JsonValue {
        JsonValue(serde_json::json!({
            "error_code": self.code,
            "message": self.message,
        }))
    }
}

fn build_workspace_tree_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceTreeResponse, WorkspaceInspectError> {
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let requested_path = command_payload_str(command, "path").unwrap_or(".");
    let relative_path = safe_workspace_relative_path(requested_path)?;
    let root_path = match resolve_existing_workspace_path(&workspace_root, &relative_path) {
        Ok(path) => path,
        Err(error) if error.code == "workspace.path_missing" => workspace_root.join(&relative_path),
        Err(error) => return Err(error),
    };
    let mut root = workspace_tree_entry(&root_path, &relative_path);
    let mut truncated = false;
    let mut total_entries = None;
    if root.kind == WorkspaceEntryKind::Directory && root.status == WorkspaceEntryStatus::Directory
    {
        match std::fs::read_dir(&root_path) {
            Ok(read_dir) => {
                let mut entries = read_dir.filter_map(Result::ok).collect::<Vec<_>>();
                entries.sort_by(|left, right| {
                    let left_is_dir = left.file_type().is_ok_and(|kind| kind.is_dir());
                    let right_is_dir = right.file_type().is_ok_and(|kind| kind.is_dir());
                    right_is_dir
                        .cmp(&left_is_dir)
                        .then_with(|| left.file_name().cmp(&right.file_name()))
                });
                total_entries = Some(entries.len() as u64);
                truncated = entries.len() > MAX_WORKSPACE_DIRECTORY_ENTRIES;
                root.children = entries
                    .into_iter()
                    .take(MAX_WORKSPACE_DIRECTORY_ENTRIES)
                    .map(|entry| {
                        let child_relative_path = relative_path.join(entry.file_name());
                        workspace_tree_entry(&entry.path(), &child_relative_path)
                    })
                    .collect();
            }
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                root.status = WorkspaceEntryStatus::PermissionDenied;
                root.expandable = false;
            }
            Err(_) => {
                root.status = WorkspaceEntryStatus::Error;
                root.expandable = false;
            }
        }
    }
    Ok(WorkspaceTreeResponse {
        placement_id,
        root,
        truncated,
        total_entries,
        generated_at: Utc::now(),
    })
}

fn build_workspace_file_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceFileContentResponse, WorkspaceInspectError> {
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let requested_path = command_payload_str(command, "path").unwrap_or(".");
    let relative_path = safe_workspace_relative_path(requested_path)?;
    let response_path = relative_path_string(&relative_path);
    let target_path = match resolve_existing_workspace_path(&workspace_root, &relative_path) {
        Ok(path) => path,
        Err(error) if error.code == "workspace.path_missing" => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
                None,
            ));
        }
        Err(error) => return Err(error),
    };
    let metadata = match std::fs::symlink_metadata(&target_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::PermissionDenied,
                None,
                None,
                None,
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
                None,
            ));
        }
        Err(error) => {
            return Err(WorkspaceInspectError::new(
                "workspace.metadata_failed",
                format!("Failed to inspect {response_path}: {error}"),
            ));
        }
    };
    let kind = workspace_entry_kind(&metadata);
    let modified_at = metadata_modified_at(&metadata);
    if metadata.file_type().is_symlink() {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Symlink,
            None,
            modified_at,
            None,
        ));
    }
    if !metadata.is_file() {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::NotFile,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if generated_or_ignored_status(&relative_path).is_some() {
        let status =
            generated_or_ignored_status(&relative_path).unwrap_or(WorkspaceEntryStatus::Generated);
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            status,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Large,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if binary_extension(&relative_path) {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Binary,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }

    let bytes = match std::fs::read(&target_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                kind,
                WorkspaceEntryStatus::PermissionDenied,
                Some(metadata.len()),
                modified_at,
                None,
            ));
        }
        Err(error) => {
            return Err(WorkspaceInspectError::new(
                "workspace.read_failed",
                format!("Failed to read {response_path}: {error}"),
            ));
        }
    };
    if bytes.contains(&0) {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Binary,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                kind,
                WorkspaceEntryStatus::Binary,
                Some(metadata.len()),
                modified_at,
                None,
            ));
        }
    };

    Ok(workspace_file_status_response(
        placement_id,
        relative_path,
        kind,
        WorkspaceEntryStatus::Readable,
        Some(metadata.len()),
        modified_at,
        Some(content),
    ))
}

fn workspace_file_status_response(
    placement_id: ProjectPlacementId,
    relative_path: PathBuf,
    kind: WorkspaceEntryKind,
    status: WorkspaceEntryStatus,
    byte_len: Option<u64>,
    modified_at: Option<DateTime<Utc>>,
    content: Option<String>,
) -> WorkspaceFileContentResponse {
    let path = relative_path_string(&relative_path);
    WorkspaceFileContentResponse {
        placement_id,
        path: path.clone(),
        metadata: WorkspaceEntry {
            name: workspace_entry_name(&relative_path),
            path,
            kind,
            status,
            classification: workspace_entry_classification(&relative_path),
            expandable: false,
            byte_len,
            modified_at,
            children: vec![],
        },
        content,
        truncated: false,
        generated_at: Utc::now(),
    }
}

fn workspace_command_placement_id(
    command: &CommandEnvelope,
) -> Result<ProjectPlacementId, WorkspaceInspectError> {
    command
        .target
        .project_placement_id()
        .cloned()
        .ok_or_else(|| {
            WorkspaceInspectError::new(
                "workspace.placement_missing",
                "Workspace inspector command is missing a placement id",
            )
        })
}

fn workspace_command_path(command: &CommandEnvelope) -> Result<&str, WorkspaceInspectError> {
    command_payload_str(command, "workspace_path")
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| {
            WorkspaceInspectError::new(
                "workspace.path_required",
                "Workspace inspector command is missing a workspace path",
            )
        })
}

fn canonical_workspace_root(
    config: &NodeConfig,
    workspace_path: &str,
) -> Result<PathBuf, WorkspaceInspectError> {
    canonical_workspace_root_for_allowed_paths(&config.workspace_paths, workspace_path)
}

fn canonical_workspace_root_for_allowed_paths(
    allowed_paths: &[PathBuf],
    workspace_path: &str,
) -> Result<PathBuf, WorkspaceInspectError> {
    let root = std::fs::canonicalize(workspace_path).map_err(|error| {
        let code = if error.kind() == ErrorKind::NotFound {
            "workspace.root_missing"
        } else if error.kind() == ErrorKind::PermissionDenied {
            "workspace.root_permission_denied"
        } else {
            "workspace.root_invalid"
        };
        WorkspaceInspectError::new(
            code,
            format!("Workspace root {workspace_path} is not readable: {error}"),
        )
    })?;
    if !root.is_dir() {
        return Err(WorkspaceInspectError::new(
            "workspace.root_not_directory",
            "Workspace root is not a directory",
        ));
    }
    if !canonical_workspace_path_allowed_roots(allowed_paths, &root) {
        return Err(WorkspaceInspectError::new(
            "workspace.outside_allowed_roots",
            "Workspace root is outside the node allowed roots",
        ));
    }
    Ok(root)
}

fn canonical_workspace_path_allowed(config: &NodeConfig, workspace_root: &Path) -> bool {
    canonical_workspace_path_allowed_roots(&config.workspace_paths, workspace_root)
}

fn canonical_workspace_path_allowed_roots(
    allowed_paths: &[PathBuf],
    workspace_root: &Path,
) -> bool {
    !allowed_paths.is_empty()
        && allowed_paths.iter().any(|allowed_root| {
            std::fs::canonicalize(allowed_root)
                .map(|root| workspace_root.starts_with(root))
                .unwrap_or(false)
        })
}

fn safe_workspace_relative_path(path: &str) -> Result<PathBuf, WorkspaceInspectError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(PathBuf::new());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(WorkspaceInspectError::new(
            "workspace.absolute_path",
            "Workspace inspector paths must be relative",
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(WorkspaceInspectError::new(
                    "workspace.path_escape",
                    "Workspace inspector paths cannot leave the workspace",
                ));
            }
        }
    }
    Ok(normalized)
}

fn resolve_existing_workspace_path(
    workspace_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, WorkspaceInspectError> {
    let mut current = workspace_root.to_path_buf();
    if relative_path.as_os_str().is_empty() {
        return Ok(current);
    }
    for component in relative_path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(WorkspaceInspectError::new(
                "workspace.path_escape",
                "Workspace inspector paths cannot leave the workspace",
            ));
        };
        current.push(segment);
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| {
            let code = if error.kind() == ErrorKind::NotFound {
                "workspace.path_missing"
            } else if error.kind() == ErrorKind::PermissionDenied {
                "workspace.permission_denied"
            } else {
                "workspace.metadata_failed"
            };
            WorkspaceInspectError::new(
                code,
                format!("Failed to inspect {}: {error}", relative_path.display()),
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Ok(current);
        }
        let canonical = std::fs::canonicalize(&current).map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.canonicalize_failed",
                format!("Failed to resolve {}: {error}", relative_path.display()),
            )
        })?;
        if !canonical.starts_with(workspace_root) {
            return Err(WorkspaceInspectError::new(
                "workspace.path_escape",
                "Workspace inspector path resolves outside the workspace",
            ));
        }
        current = canonical;
    }
    Ok(current)
}

fn workspace_tree_entry(path: &Path, relative_path: &Path) -> WorkspaceEntry {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::PermissionDenied,
                None,
                None,
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
            );
        }
        Err(_) => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Error,
                None,
                None,
            );
        }
    };
    let kind = workspace_entry_kind(&metadata);
    let status = workspace_status_for_entry(relative_path, &metadata);

    WorkspaceEntry {
        name: workspace_entry_name(relative_path),
        path: relative_path_string(relative_path),
        kind,
        status,
        classification: workspace_entry_classification(relative_path),
        expandable: metadata.is_dir(),
        byte_len: metadata.is_file().then_some(metadata.len()),
        modified_at: metadata_modified_at(&metadata),
        children: vec![],
    }
}

fn workspace_entry_for_status(
    relative_path: &Path,
    kind: WorkspaceEntryKind,
    status: WorkspaceEntryStatus,
    byte_len: Option<u64>,
    modified_at: Option<DateTime<Utc>>,
) -> WorkspaceEntry {
    WorkspaceEntry {
        name: workspace_entry_name(relative_path),
        path: relative_path_string(relative_path),
        kind,
        status,
        classification: WorkspaceEntryClassification::Normal,
        expandable: false,
        byte_len,
        modified_at,
        children: vec![],
    }
}

fn workspace_entry_kind(metadata: &std::fs::Metadata) -> WorkspaceEntryKind {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        WorkspaceEntryKind::Directory
    } else if file_type.is_file() {
        WorkspaceEntryKind::File
    } else if file_type.is_symlink() {
        WorkspaceEntryKind::Symlink
    } else {
        WorkspaceEntryKind::Other
    }
}

fn workspace_status_for_entry(
    relative_path: &Path,
    metadata: &std::fs::Metadata,
) -> WorkspaceEntryStatus {
    if metadata.file_type().is_symlink() {
        return WorkspaceEntryStatus::Symlink;
    }
    if metadata.is_dir() {
        return WorkspaceEntryStatus::Directory;
    }
    if metadata.is_file() {
        if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
            return WorkspaceEntryStatus::Large;
        }
        if binary_extension(relative_path) {
            return WorkspaceEntryStatus::Binary;
        }
        return WorkspaceEntryStatus::Readable;
    }
    WorkspaceEntryStatus::Error
}

fn workspace_entry_classification(relative_path: &Path) -> WorkspaceEntryClassification {
    match generated_or_ignored_status(relative_path) {
        Some(WorkspaceEntryStatus::Generated) => WorkspaceEntryClassification::Generated,
        Some(WorkspaceEntryStatus::Ignored) => WorkspaceEntryClassification::Ignored,
        _ => WorkspaceEntryClassification::Normal,
    }
}

fn generated_or_ignored_status(relative_path: &Path) -> Option<WorkspaceEntryStatus> {
    let mut generated = None;
    for component in relative_path.components() {
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        let Some(name) = value.to_str() else {
            continue;
        };
        match name {
            ".git" | ".hg" | ".svn" | ".DS_Store" => return Some(WorkspaceEntryStatus::Ignored),
            ".local" | "node_modules" | "target" | "dist" | "build" | "coverage" | ".next"
            | ".turbo" | ".vite" => generated = Some(WorkspaceEntryStatus::Generated),
            _ => {}
        }
    }
    generated
}

fn workspace_entry_status_label(status: WorkspaceEntryStatus) -> &'static str {
    match status {
        WorkspaceEntryStatus::Readable => "readable",
        WorkspaceEntryStatus::Directory => "directory",
        WorkspaceEntryStatus::Large => "large",
        WorkspaceEntryStatus::Binary => "binary",
        WorkspaceEntryStatus::Ignored => "ignored",
        WorkspaceEntryStatus::Generated => "generated",
        WorkspaceEntryStatus::PermissionDenied => "permission-denied",
        WorkspaceEntryStatus::OutsideWorkspace => "outside-workspace",
        WorkspaceEntryStatus::Missing => "missing",
        WorkspaceEntryStatus::NotFile => "not-file",
        WorkspaceEntryStatus::NotDirectory => "not-directory",
        WorkspaceEntryStatus::Symlink => "symlink",
        WorkspaceEntryStatus::Error => "error",
    }
}

fn binary_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "pdf"
            | "zip"
            | "gz"
            | "tar"
            | "tgz"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "wasm"
            | "sqlite"
            | "db"
            | "bin"
            | "exe"
            | "dylib"
            | "so"
            | "dll"
    )
}

fn workspace_entry_name(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        return ".".to_owned();
    }
    relative_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_owned())
}

fn relative_path_string(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        return ".".to_owned();
    }
    relative_path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn metadata_modified_at(metadata: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    metadata.modified().ok().map(DateTime::<Utc>::from)
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
            node_id: command.target.node_id().clone(),
        },
        scope_ref: ScopeRef::Placement {
            project_placement_id,
        },
        node_id: Some(command.target.node_id().clone()),
        runtime_session_id: None,
        session_thread_id: None,
        turn_id: None,
        seq,
        session_projection_seq: None,
        kind,
        happened_at: Utc::now(),
        source_refs: command.source_refs.clone(),
        evidence_refs: vec![],
        cause_refs: command.cause_refs.clone(),
        result_refs: vec![],
        payload: EventPayload::from_json(kind, payload),
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
    Codex(CodexProviderAdapter),
    Unsupported(UnsupportedProviderAdapter),
}

impl RuntimeManager {
    fn for_provider(provider_key: &str, config: &NodeConfig) -> Self {
        match provider_key {
            "codex" => Self::Codex(CodexProviderAdapter::new(config)),
            other => Self::Unsupported(UnsupportedProviderAdapter {
                provider_key: other.to_owned(),
            }),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "runtime execution bridges durable runtime maps, workspace context, live events, and cancellation"
    )]
    async fn execute_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
        live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> Vec<EventEnvelope> {
        match self {
            Self::Codex(provider) => {
                provider
                    .events_for_command(
                        command,
                        runtime_seqs,
                        workspace_path,
                        runtime_transcripts,
                        runtime_provider_resume_refs,
                        live_event_sink,
                        cancellation,
                    )
                    .await
            }
            Self::Unsupported(provider) => provider.events_for_command(command, runtime_seqs),
        }
    }

    async fn execute_deduction(
        &self,
        command: &CommandEnvelope,
        workspace_path: Option<&str>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> (CommandState, JsonValue) {
        match self {
            Self::Codex(provider) => {
                provider
                    .execute_deduction(command, workspace_path, cancellation)
                    .await
            }
            Self::Unsupported(provider) => deduction_error_payload(
                command,
                &provider.provider_key,
                "deduction.provider_unsupported",
                format!(
                    "Provider `{}` cannot execute structured deductions",
                    provider.provider_key
                ),
            ),
        }
    }
}

#[derive(Debug, Clone)]
struct CodexProviderAdapter {
    codex_binary: String,
    ignore_user_config: bool,
    timeout: Duration,
    workspace_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct ProviderStartFailure {
    code: &'static str,
    message: String,
}

struct CodexProcessOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
    dropped_activity_count: usize,
    approval_requests: Vec<CodexApprovalRequest>,
    provider_resume_ref: Option<serde_json::Value>,
    activity_events: Vec<EventEnvelope>,
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
            ignore_user_config: config.codex_ignore_user_config,
            timeout: config.codex_timeout,
            workspace_paths: config.workspace_paths.clone(),
        }
    }

    fn provider_key(&self) -> &'static str {
        "codex"
    }

    fn authorized_workspace_path(
        &self,
        workspace_path: &str,
    ) -> Result<String, WorkspaceInspectError> {
        canonical_workspace_root_for_allowed_paths(&self.workspace_paths, workspace_path)
            .map(|path| path.display().to_string())
    }

    async fn execute_deduction(
        &self,
        command_context: &CommandEnvelope,
        workspace_path: Option<&str>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> (CommandState, JsonValue) {
        let CommandPayload::RequestDeduction { package } = &command_context.payload else {
            return deduction_error_payload(
                command_context,
                self.provider_key(),
                "protocol.command_payload_mismatch",
                "RequestDeduction payload does not match its command kind",
            );
        };
        let Some(runtime_session_id) = command_context.target.runtime_session_id().cloned() else {
            return deduction_error_payload(
                command_context,
                self.provider_key(),
                "deduction.runtime_missing",
                "Deduction command is missing a runtime session target",
            );
        };
        let Some(workspace_path) = workspace_path.filter(|value| !value.trim().is_empty()) else {
            return deduction_error_payload(
                command_context,
                self.provider_key(),
                "deduction.workspace_missing",
                "Deduction requires the session workspace path",
            );
        };
        let workspace_path = match self.authorized_workspace_path(workspace_path) {
            Ok(path) => path,
            Err(error) => {
                return deduction_error_payload(
                    command_context,
                    self.provider_key(),
                    error.code,
                    error.message,
                );
            }
        };

        let last_message_path = codex_last_message_path(&command_context.command_id);
        let schema_path = codex_deduction_schema_path(&command_context.command_id);
        let schema = deduction_output_schema(package);
        if let Err(error) = std::fs::write(
            &schema_path,
            serde_json::to_vec_pretty(&schema).unwrap_or_default(),
        ) {
            return deduction_error_payload(
                command_context,
                self.provider_key(),
                "deduction.schema_write_failed",
                format!("Could not write the temporary deduction schema: {error}"),
            );
        }
        let prompt = deduction_prompt(package);
        let mut command = TokioCommand::new(&self.codex_binary);
        command.arg("exec");
        if self.ignore_user_config {
            command.arg("--ignore-user-config");
        }
        command
            .arg("--cd")
            .arg(&workspace_path)
            .arg("--ephemeral")
            .arg("--sandbox")
            .arg("read-only")
            .arg("--skip-git-repo-check")
            .arg("--json")
            .arg("--output-schema")
            .arg(&schema_path)
            .arg("--output-last-message")
            .arg(&last_message_path)
            .arg(prompt)
            .current_dir(&workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_provider_process(&mut command);

        let mut isolated_seqs = HashMap::new();
        let output = self
            .run_codex_command(
                command,
                "Codex deduction",
                command_context,
                &mut isolated_seqs,
                &runtime_session_id,
                None,
                None,
                cancellation,
            )
            .await;
        let raw_text = std::fs::read_to_string(&last_message_path).unwrap_or_default();
        let _ = std::fs::remove_file(&last_message_path);
        let _ = std::fs::remove_file(&schema_path);
        let raw_truncated = raw_text.chars().count() > MAX_DEDUCTION_RAW_CHARS;
        let raw_text = bounded_text(&raw_text, MAX_DEDUCTION_RAW_CHARS);

        match output {
            Ok(output) if output.status.success() => {
                let parsed = serde_json::from_str::<DeductionProviderResult>(raw_text.trim());
                let (result, error_code, error_message) = match parsed {
                    Ok(result) => (Some(result), None, None),
                    Err(error) => (
                        None,
                        Some("deduction.output_invalid_json".to_owned()),
                        Some(format!(
                            "Structured deduction output could not be parsed: {error}"
                        )),
                    ),
                };
                deduction_output_payload(
                    package,
                    self.provider_key(),
                    result,
                    raw_text,
                    raw_truncated,
                    error_code,
                    error_message,
                    CommandState::Completed,
                )
            }
            Ok(output) => deduction_output_payload(
                package,
                self.provider_key(),
                None,
                raw_text,
                raw_truncated,
                Some("deduction.provider_failed".to_owned()),
                Some(codex_failure_message(&output)),
                CommandState::Failed,
            ),
            Err(error) => deduction_output_payload(
                package,
                self.provider_key(),
                None,
                raw_text,
                raw_truncated,
                Some(error.code.to_owned()),
                Some(error.message),
                CommandState::Failed,
            ),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "provider command execution updates runtime sequence, transcript, resume, live event, and cancellation state"
    )]
    async fn events_for_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
        live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> Vec<EventEnvelope> {
        let Some(runtime_session_id) = command.target.runtime_session_id().cloned() else {
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
                    live_event_sink,
                    cancellation,
                )
                .await
            }
            CommandKind::ResolveApproval => {
                let CommandPayload::ResolveApproval {
                    approval_id,
                    approved,
                    message,
                } = &command.payload
                else {
                    return vec![runtime_error_event(
                        self.provider_key(),
                        command,
                        runtime_seqs,
                        runtime_session_id,
                        None,
                        "protocol.command_payload_mismatch",
                        "ResolveApproval payload does not match its command kind",
                    )];
                };
                let default_message = if *approved {
                    "Approval accepted"
                } else {
                    "Approval denied"
                };
                let message = message.as_deref().unwrap_or(default_message);
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
                            "approved": *approved,
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

    #[expect(
        clippy::too_many_arguments,
        reason = "provider turn execution carries command, runtime, transcript, resume, and live stream state"
    )]
    async fn send_turn_events(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        runtime_session_id: RuntimeSessionId,
        workspace_path: Option<&str>,
        runtime_transcripts: &mut HashMap<String, Vec<ProviderTranscriptMessage>>,
        runtime_provider_resume_refs: &mut HashMap<String, ProviderResumeRef>,
        live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> Vec<EventEnvelope> {
        let CommandPayload::SendTurn { content, turn_id } = &command.payload else {
            return vec![runtime_error_event(
                self.provider_key(),
                command,
                runtime_seqs,
                runtime_session_id,
                None,
                "protocol.command_payload_mismatch",
                "SendTurn payload does not match its command kind",
            )];
        };
        let turn_id = Some(turn_id.clone());
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
        let workspace_path = match self.authorized_workspace_path(workspace_path) {
            Ok(path) => path,
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
                return events;
            }
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
                &workspace_path,
                provider_session_id,
                content,
                &last_message_path,
                command,
                runtime_seqs,
                &runtime_session_id,
                turn_id.clone(),
                live_event_sink,
                cancellation,
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
            self.run_codex_exec(
                &workspace_path,
                &prompt,
                &last_message_path,
                command,
                runtime_seqs,
                &runtime_session_id,
                turn_id.clone(),
                live_event_sink,
                cancellation,
            )
            .await
        };
        let last_message = std::fs::read_to_string(&last_message_path).unwrap_or_default();
        let _ = std::fs::remove_file(&last_message_path);

        match output {
            Ok(output) if output.status.success() => {
                events.extend(output.activity_events.iter().cloned());
                append_codex_process_limit_events(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id.clone(),
                    turn_id.clone(),
                    &output,
                    &mut events,
                );
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
                                "source": "codex.exec.jsonl",
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
                events.extend(output.activity_events.iter().cloned());
                append_codex_process_limit_events(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id.clone(),
                    turn_id.clone(),
                    &output,
                    &mut events,
                );
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
            Err(error) if error.code == "provider.cancelled" => {
                events.push(event_for_command(
                    self.provider_key(),
                    command,
                    runtime_seqs,
                    runtime_session_id,
                    turn_id,
                    EventKind::TurnInterrupted,
                    serde_json::json!({
                        "provider": self.provider_key(),
                        "code": error.code,
                        "message": error.message,
                    }),
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

    #[expect(
        clippy::too_many_arguments,
        reason = "Codex process launch needs workspace, output file, runtime, and live stream context"
    )]
    async fn run_codex_exec(
        &self,
        workspace_path: &str,
        content: &str,
        last_message_path: &Path,
        command_context: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        runtime_session_id: &RuntimeSessionId,
        turn_id: Option<TurnId>,
        live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> Result<CodexProcessOutput, ProviderStartFailure> {
        let mut command = TokioCommand::new(&self.codex_binary);
        command.arg("exec");
        if self.ignore_user_config {
            command.arg("--ignore-user-config");
        }
        command
            .arg("--cd")
            .arg(workspace_path)
            .arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("--json")
            .arg("--output-last-message")
            .arg(last_message_path)
            .arg(content)
            .current_dir(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_provider_process(&mut command);

        self.run_codex_command(
            command,
            "Codex exec",
            command_context,
            runtime_seqs,
            runtime_session_id,
            turn_id,
            live_event_sink,
            cancellation,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Codex resume launch needs resume id plus the same runtime and live stream context"
    )]
    async fn run_codex_exec_resume(
        &self,
        workspace_path: &str,
        provider_session_id: &str,
        content: &str,
        last_message_path: &Path,
        command_context: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        runtime_session_id: &RuntimeSessionId,
        turn_id: Option<TurnId>,
        live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        cancellation: Option<watch::Receiver<bool>>,
    ) -> Result<CodexProcessOutput, ProviderStartFailure> {
        let mut command = TokioCommand::new(&self.codex_binary);
        command.arg("exec");
        if self.ignore_user_config {
            command.arg("--ignore-user-config");
        }
        command
            .arg("resume")
            .arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("--json")
            .arg("--output-last-message")
            .arg(last_message_path)
            .arg(provider_session_id)
            .arg(content)
            .current_dir(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_provider_process(&mut command);

        self.run_codex_command(
            command,
            "Codex exec resume",
            command_context,
            runtime_seqs,
            runtime_session_id,
            turn_id,
            live_event_sink,
            cancellation,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "shared child-process reader carries command, runtime sequence, turn, and live sink context"
    )]
    async fn run_codex_command(
        &self,
        mut command: TokioCommand,
        command_label: &'static str,
        command_context: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
        runtime_session_id: &RuntimeSessionId,
        turn_id: Option<TurnId>,
        mut live_event_sink: Option<&mut NodeLiveEventSink<'_>>,
        mut cancellation: Option<watch::Receiver<bool>>,
    ) -> Result<CodexProcessOutput, ProviderStartFailure> {
        let mut child = command.spawn().map_err(|error| {
            let code = if error.kind() == ErrorKind::NotFound {
                "provider.missing_binary"
            } else {
                "provider.start_failed"
            };
            ProviderStartFailure::new(
                code,
                format!(
                    "{command_label} could not start using `{}`: {error}",
                    self.codex_binary
                ),
            )
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ProviderStartFailure::new(
                "provider.start_failed",
                format!("{command_label} did not expose stdout"),
            )
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ProviderStartFailure::new(
                "provider.start_failed",
                format!("{command_label} did not expose stderr"),
            )
        })?;

        let run = async {
            let mut stdout_lines = BufReader::new(stdout).lines();
            let mut stderr_lines = BufReader::new(stderr).lines();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut stdout_truncated = false;
            let mut stderr_truncated = false;
            let mut activity_events = Vec::new();
            let mut approval_requests = Vec::new();
            let mut provider_resume_ref = None;
            let mut dropped_activity_count = 0usize;
            let mut stdout_done = false;
            let mut stderr_done = false;
            let mut status = None;
            let wait = child.wait();
            tokio::pin!(wait);

            loop {
                if status.is_some() && stdout_done && stderr_done {
                    break;
                }

                tokio::select! {
                    line = stdout_lines.next_line(), if !stdout_done => {
                        match line {
                            Ok(Some(line)) => {
                                if approval_requests.len() < MAX_PROVIDER_APPROVAL_REQUESTS {
                                    if let Some(approval_request) =
                                        codex_approval_request_from_json_line(&line)
                                    {
                                        approval_requests.push(approval_request);
                                    }
                                }
                                if provider_resume_ref.is_none() {
                                    provider_resume_ref = codex_resume_ref_from_json_line(&line);
                                }
                                append_capped_process_line(
                                    &mut stdout,
                                    &line,
                                    MAX_PROVIDER_PROCESS_OUTPUT_BYTES,
                                    &mut stdout_truncated,
                                );
                                let event = codex_stdout_activity_event(
                                    self.provider_key(),
                                    command_context,
                                    runtime_seqs,
                                    runtime_session_id.clone(),
                                    turn_id.clone(),
                                    &line,
                                );
                                emit_codex_activity_event(
                                    &mut activity_events,
                                    &mut dropped_activity_count,
                                    event,
                                    &mut live_event_sink,
                                )
                                .await?;
                            }
                            Ok(None) => stdout_done = true,
                            Err(error) => {
                                return Err(ProviderStartFailure::new(
                                    "provider.stdout_read_failed",
                                    format!("{command_label} stdout could not be read: {error}"),
                                ));
                            }
                        }
                    }
                    line = stderr_lines.next_line(), if !stderr_done => {
                        match line {
                            Ok(Some(line)) => {
                                append_capped_process_line(
                                    &mut stderr,
                                    &line,
                                    MAX_PROVIDER_PROCESS_OUTPUT_BYTES,
                                    &mut stderr_truncated,
                                );
                                let event = codex_stderr_activity_event(
                                    self.provider_key(),
                                    command_context,
                                    runtime_seqs,
                                    runtime_session_id.clone(),
                                    turn_id.clone(),
                                    &line,
                                );
                                emit_codex_activity_event(
                                    &mut activity_events,
                                    &mut dropped_activity_count,
                                    event,
                                    &mut live_event_sink,
                                )
                                .await?;
                            }
                            Ok(None) => stderr_done = true,
                            Err(error) => {
                                return Err(ProviderStartFailure::new(
                                    "provider.stderr_read_failed",
                                    format!("{command_label} stderr could not be read: {error}"),
                                ));
                            }
                        }
                    }
                    wait_result = &mut wait, if status.is_none() => {
                        status = Some(wait_result.map_err(|error| {
                            ProviderStartFailure::new(
                                "provider.wait_failed",
                                format!("{command_label} wait failed: {error}"),
                            )
                        })?);
                    }
                    _ = wait_for_runtime_cancellation(&mut cancellation), if cancellation.is_some() => {
                        return Err(ProviderStartFailure::new(
                            "provider.cancelled",
                            format!("{command_label} was cancelled by a runtime control command"),
                        ));
                    }
                }
            }

            let status = status.ok_or_else(|| {
                ProviderStartFailure::new(
                    "provider.wait_failed",
                    format!("{command_label} exited without a status"),
                )
            })?;

            Ok(CodexProcessOutput {
                status,
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                dropped_activity_count,
                approval_requests,
                provider_resume_ref,
                activity_events,
            })
        };

        match timeout(self.timeout, run).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                if error.code == "provider.cancelled" {
                    terminate_provider_process(&mut child).await;
                }
                Err(error)
            }
            Err(_) => {
                terminate_provider_process(&mut child).await;
                Err(ProviderStartFailure::new(
                    "provider.execution_timeout",
                    format!(
                        "{command_label} timed out after {} seconds",
                        self.timeout.as_secs()
                    ),
                ))
            }
        }
    }
}

#[cfg(unix)]
fn configure_provider_process(command: &mut TokioCommand) {
    command.as_std_mut().process_group(0);
}

#[cfg(not(unix))]
fn configure_provider_process(_command: &mut TokioCommand) {}

async fn terminate_provider_process(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        signal_provider_process_group(pid, "-TERM").await;
    }
    #[cfg(not(unix))]
    let _ = child.start_kill();

    if timeout(PROVIDER_PROCESS_SHUTDOWN_TIMEOUT, child.wait())
        .await
        .is_ok()
    {
        return;
    }

    #[cfg(unix)]
    if let Some(pid) = child.id() {
        signal_provider_process_group(pid, "-KILL").await;
    }
    #[cfg(not(unix))]
    let _ = child.start_kill();
    let _ = timeout(PROVIDER_PROCESS_SHUTDOWN_TIMEOUT, child.wait()).await;
}

#[cfg(unix)]
async fn signal_provider_process_group(pid: u32, signal: &str) {
    let _ = TokioCommand::new("/bin/kill")
        .arg(signal)
        .arg("--")
        .arg(format!("-{pid}"))
        .status()
        .await;
}

async fn wait_for_runtime_cancellation(cancellation: &mut Option<watch::Receiver<bool>>) {
    let Some(receiver) = cancellation.as_mut() else {
        std::future::pending::<()>().await;
        return;
    };
    loop {
        if *receiver.borrow_and_update() {
            return;
        }
        if receiver.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

async fn emit_codex_activity_event(
    activity_events: &mut Vec<EventEnvelope>,
    dropped_activity_count: &mut usize,
    event: EventEnvelope,
    live_event_sink: &mut Option<&mut NodeLiveEventSink<'_>>,
) -> Result<(), ProviderStartFailure> {
    if let Some(sink) = live_event_sink.as_mut() {
        sink.emit(&event);
    }
    if activity_events.len() < MAX_PROVIDER_ACTIVITY_EVENTS {
        activity_events.push(event);
    } else {
        *dropped_activity_count = dropped_activity_count.saturating_add(1);
    }
    Ok(())
}

fn append_capped_process_line(
    output: &mut Vec<u8>,
    line: &str,
    max_bytes: usize,
    truncated: &mut bool,
) {
    if max_bytes == 0 {
        *truncated = true;
        return;
    }
    let remaining = max_bytes.saturating_sub(output.len());
    if remaining == 0 {
        *truncated = true;
        return;
    }
    let line_bytes = line.as_bytes();
    let copied = line_bytes.len().min(remaining);
    output.extend_from_slice(&line_bytes[..copied]);
    if copied < line_bytes.len() {
        *truncated = true;
        return;
    }
    if output.len() < max_bytes {
        output.push(b'\n');
    } else {
        *truncated = true;
    }
}

fn append_codex_process_limit_events(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    output: &CodexProcessOutput,
    events: &mut Vec<EventEnvelope>,
) {
    if !output.stdout_truncated && !output.stderr_truncated && output.dropped_activity_count == 0 {
        return;
    }
    events.push(provider_activity_event(
        provider_key,
        command,
        runtime_seqs,
        runtime_session_id,
        turn_id,
        serde_json::json!({
            "provider": provider_key,
            "source": "codex.exec.limits",
            "provider_event_type": "output_truncated",
            "phase": "warning",
            "status": "warning",
            "summary": "Codex provider output exceeded node retention limits",
            "stdout_truncated": output.stdout_truncated,
            "stderr_truncated": output.stderr_truncated,
            "dropped_activity_count": output.dropped_activity_count,
            "max_process_output_bytes": MAX_PROVIDER_PROCESS_OUTPUT_BYTES,
            "max_activity_events": MAX_PROVIDER_ACTIVITY_EVENTS,
        }),
    ));
}

fn codex_stdout_activity_event(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    line: &str,
) -> EventEnvelope {
    match serde_json::from_str::<serde_json::Value>(line) {
        Ok(value) => provider_activity_event(
            provider_key,
            command,
            runtime_seqs,
            runtime_session_id,
            turn_id,
            codex_activity_payload_from_json(provider_key, value),
        ),
        Err(error) => provider_activity_event(
            provider_key,
            command,
            runtime_seqs,
            runtime_session_id,
            turn_id,
            serde_json::json!({
                "provider": provider_key,
                "source": "codex.exec.jsonl",
                "provider_event_type": "parse_error",
                "phase": "error",
                "status": "error",
                "summary": format!("Codex JSONL parse error: {error}"),
                "raw_line_preview": bounded_text(line, MAX_PROVIDER_ACTIVITY_LINE_CHARS),
                "raw_line_truncated": line.chars().count() > MAX_PROVIDER_ACTIVITY_LINE_CHARS,
                "raw_line_original_chars": line.chars().count(),
                "parse_error": error.to_string(),
            }),
        ),
    }
}

fn codex_stderr_activity_event(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    line: &str,
) -> EventEnvelope {
    provider_activity_event(
        provider_key,
        command,
        runtime_seqs,
        runtime_session_id,
        turn_id,
        serde_json::json!({
            "provider": provider_key,
            "source": "codex.exec.stderr",
            "provider_event_type": "stderr",
            "phase": "warning",
            "status": "warning",
            "summary": bounded_text(line.trim(), MAX_PROVIDER_ACTIVITY_SUMMARY_CHARS),
            "raw_event": {
                "stream": "stderr",
                "line": bounded_text(line, MAX_PROVIDER_ACTIVITY_LINE_CHARS),
            },
            "raw_event_truncated": line.chars().count() > MAX_PROVIDER_ACTIVITY_LINE_CHARS,
            "raw_event_original_chars": line.chars().count(),
        }),
    )
}

fn provider_activity_event(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    runtime_session_id: RuntimeSessionId,
    turn_id: Option<TurnId>,
    payload: serde_json::Value,
) -> EventEnvelope {
    event_for_command(
        provider_key,
        command,
        runtime_seqs,
        runtime_session_id,
        turn_id,
        EventKind::ProviderActivity,
        payload,
    )
}

fn codex_activity_payload_from_json(
    provider_key: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    let provider_event_type = top_level_json_string(&value, &["type", "event", "kind"])
        .unwrap_or_else(|| "unknown".to_owned());
    let provider_item = value.get("item");
    let provider_item_id = provider_item
        .and_then(|item| item.get("id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("item_id").and_then(serde_json::Value::as_str))
        .map(|text| bounded_text(text, 512));
    let provider_item_type = provider_item
        .and_then(|item| item.get("type"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("item_type").and_then(serde_json::Value::as_str))
        .map(|text| bounded_text(text, 512));
    let status = first_json_string_for_keys(&value, &["status", "state"])
        .map(|text| bounded_text(text.trim(), MAX_PROVIDER_ACTIVITY_SUMMARY_CHARS));
    let phase = codex_activity_phase(&provider_event_type, status.as_deref());
    let summary = codex_activity_summary(&value).unwrap_or_else(|| provider_event_type.clone());

    let mut payload = serde_json::json!({
        "provider": provider_key,
        "source": "codex.exec.jsonl",
        "provider_event_type": provider_event_type,
        "phase": phase,
        "summary": summary,
    });
    if let Some(provider_item_id) = provider_item_id {
        payload["provider_item_id"] = serde_json::Value::String(provider_item_id);
    }
    if let Some(provider_item_type) = provider_item_type {
        payload["provider_item_type"] = serde_json::Value::String(provider_item_type);
    }
    if let Some(status) = status {
        payload["status"] = serde_json::Value::String(status);
    }
    append_bounded_raw_event(&mut payload, value);
    payload
}

fn codex_activity_phase(provider_event_type: &str, status: Option<&str>) -> String {
    let normalized_type = provider_event_type.to_ascii_lowercase();
    if normalized_type.contains("failed") || normalized_type.contains("error") {
        return "error".to_owned();
    }
    if normalized_type.contains("completed") || normalized_type.contains("done") {
        return "completed".to_owned();
    }
    if normalized_type.contains("started") || normalized_type.contains("created") {
        return "started".to_owned();
    }
    if normalized_type.contains("delta") || normalized_type.contains("output") {
        return "running".to_owned();
    }
    if let Some(status) = status {
        let normalized_status = status.to_ascii_lowercase();
        if normalized_status.contains("failed") || normalized_status.contains("error") {
            return "error".to_owned();
        }
        if normalized_status.contains("completed") || normalized_status.contains("done") {
            return "completed".to_owned();
        }
        if normalized_status.contains("running") || normalized_status.contains("started") {
            return "running".to_owned();
        }
    }
    "observed".to_owned()
}

fn codex_activity_summary(value: &serde_json::Value) -> Option<String> {
    first_json_string_for_keys(
        value,
        &[
            "command",
            "summary",
            "message",
            "text",
            "content",
            "delta",
            "reason",
            "description",
            "path",
        ],
    )
    .map(|text| bounded_text(text.trim(), MAX_PROVIDER_ACTIVITY_SUMMARY_CHARS))
    .filter(|text| !text.is_empty())
}

fn append_bounded_raw_event(payload: &mut serde_json::Value, raw_event: serde_json::Value) {
    let raw_event_chars = serde_json::to_string(&raw_event)
        .map(|text| text.chars().count())
        .unwrap_or(0);
    if raw_event_chars <= MAX_PROVIDER_ACTIVITY_RAW_CHARS {
        payload["raw_event"] = raw_event;
        payload["raw_event_truncated"] = serde_json::Value::Bool(false);
        return;
    }
    let preview = serde_json::to_string(&raw_event)
        .map(|text| bounded_text(&text, MAX_PROVIDER_ACTIVITY_RAW_CHARS))
        .unwrap_or_else(|_| "<unserializable provider event>".to_owned());
    payload["raw_event_truncated"] = serde_json::Value::Bool(true);
    payload["raw_event_original_chars"] =
        serde_json::Value::Number(serde_json::Number::from(raw_event_chars));
    payload["raw_event_preview"] = serde_json::Value::String(preview);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexApprovalRequest {
    approval_id: ApprovalId,
    prompt: String,
    provider_event_type: Option<String>,
}

fn codex_approval_requests_from_output(output: &CodexProcessOutput) -> Vec<CodexApprovalRequest> {
    output.approval_requests.clone()
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

fn codex_resume_ref_from_output(output: &CodexProcessOutput) -> Option<serde_json::Value> {
    output.provider_resume_ref.clone()
}

fn codex_resume_ref_from_json_line(line: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(codex_resume_ref_from_json)
}

fn codex_resume_ref_from_json(value: serde_json::Value) -> Option<serde_json::Value> {
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
        "Continue this Uprava session. Use the transcript only as prior context, then answer the latest user message.\n\nTranscript:\n",
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

fn missing_provider_events_for_command(
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    let Some(runtime_session_id) = command.target.runtime_session_id().cloned() else {
        return vec![];
    };
    vec![event_for_command(
        "unknown",
        command,
        runtime_seqs,
        runtime_session_id,
        None,
        EventKind::RuntimeError,
        serde_json::json!({
            "code": "provider.missing",
            "message": "Runtime command is missing provider metadata",
        }),
    )]
}

impl UnsupportedProviderAdapter {
    fn events_for_command(
        &self,
        command: &CommandEnvelope,
        runtime_seqs: &mut HashMap<String, i64>,
    ) -> Vec<EventEnvelope> {
        let Some(runtime_session_id) = command.target.runtime_session_id().cloned() else {
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

fn runtime_workspace_error_events(
    provider_key: &str,
    command: &CommandEnvelope,
    runtime_seqs: &mut HashMap<String, i64>,
    error: WorkspaceInspectError,
) -> Vec<EventEnvelope> {
    let Some(runtime_session_id) = command.target.runtime_session_id().cloned() else {
        return vec![];
    };
    vec![runtime_error_event(
        provider_key,
        command,
        runtime_seqs,
        runtime_session_id,
        None,
        error.code,
        error.message,
    )]
}

fn codex_last_message_path(command_id: &CommandId) -> PathBuf {
    std::env::temp_dir().join(format!(
        "uprava-codex-{}-{}.txt",
        sanitize_filename_segment(command_id.as_str()),
        Uuid::new_v4()
    ))
}

fn codex_deduction_schema_path(command_id: &CommandId) -> PathBuf {
    std::env::temp_dir().join(format!(
        "uprava-deduction-schema-{}-{}.json",
        sanitize_filename_segment(command_id.as_str()),
        Uuid::new_v4()
    ))
}

fn deduction_output_schema(package: &DeductionInputPackage) -> serde_json::Value {
    let allowed_refs = package
        .allowed_refs
        .iter()
        .filter_map(|reference| serde_json::to_value(reference).ok())
        .collect::<Vec<_>>();
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": [
            "title", "conclusion", "certainty", "steps", "assumptions", "unknowns", "alternatives"
        ],
        "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 240 },
            "conclusion": { "type": "string", "minLength": 1, "maxLength": 4000 },
            "certainty": { "type": "string", "enum": ["high", "medium", "low", "unknown"] },
            "steps": {
                "type": "array",
                "minItems": 1,
                "maxItems": 100,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["step_id", "classification", "summary", "support_refs"],
                    "properties": {
                        "step_id": { "type": "string", "minLength": 1, "maxLength": 120 },
                        "classification": {
                            "type": "string",
                            "enum": ["observed", "inference", "assumption", "unknown", "alternative"]
                        },
                        "summary": { "type": "string", "minLength": 1, "maxLength": 1600 },
                        "support_refs": {
                            "type": "array",
                            "maxItems": 100,
                            "items": { "enum": allowed_refs }
                        }
                    }
                }
            },
            "assumptions": {
                "type": "array",
                "maxItems": 100,
                "items": { "type": "string", "maxLength": 1000 }
            },
            "unknowns": {
                "type": "array",
                "maxItems": 100,
                "items": { "type": "string", "maxLength": 1000 }
            },
            "alternatives": {
                "type": "array",
                "maxItems": 100,
                "items": { "type": "string", "maxLength": 1000 }
            }
        }
    })
}

fn deduction_prompt(package: &DeductionInputPackage) -> String {
    let package_json = serde_json::to_string_pretty(package).unwrap_or_else(|_| "{}".to_owned());
    format!(
        "You are Uprava's isolated causality analyst. Analyze only the bounded evidence package below.\n\
         Do not edit files, run commands, continue the interactive agent session, or invent references.\n\
         Classify direct facts as observed, derived claims as inference, unsupported premises as assumption,\n\
         missing information as unknown, and competing explanations as alternative.\n\
         Every observed step must cite one or more exact support_refs copied from allowed_refs.\n\
         Never cite a reference outside allowed_refs. Return only JSON matching the supplied schema.\n\n\
         EVIDENCE PACKAGE:\n{package_json}"
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "deduction provider envelope keeps output, fallback, error, and command state explicit"
)]
fn deduction_output_payload(
    package: &DeductionInputPackage,
    provider: &str,
    result: Option<DeductionProviderResult>,
    raw_text: String,
    raw_truncated: bool,
    error_code: Option<String>,
    error_message: Option<String>,
    status: CommandState,
) -> (CommandState, JsonValue) {
    let output = DeductionProviderOutput {
        deduction_id: package.deduction_id.clone(),
        provider: provider.to_owned(),
        model: None,
        schema_version: DEDUCTION_SCHEMA_VERSION.to_owned(),
        evidence_snapshot_hash: package.evidence_snapshot_hash.clone(),
        result,
        raw_text,
        raw_truncated,
        error_code,
        error_message,
    };
    match serde_json::to_value(output) {
        Ok(value) => (status, JsonValue(value)),
        Err(error) => (
            CommandState::Failed,
            JsonValue(serde_json::json!({
                "error_code": "deduction.output_serialization_failed",
                "message": error.to_string(),
            })),
        ),
    }
}

fn deduction_error_payload(
    command: &CommandEnvelope,
    provider: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> (CommandState, JsonValue) {
    if let CommandPayload::RequestDeduction { package } = &command.payload {
        return deduction_output_payload(
            package,
            provider,
            None,
            String::new(),
            false,
            Some(code.into()),
            Some(message.into()),
            CommandState::Failed,
        );
    }
    (
        CommandState::Failed,
        JsonValue(serde_json::json!({
            "error_code": code.into(),
            "message": message.into(),
        })),
    )
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

fn codex_failure_message(output: &CodexProcessOutput) -> String {
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
        node_id: Some(command.target.node_id().clone()),
        runtime_session_id: Some(runtime_session_id),
        session_thread_id: command.target.session_thread_id().cloned(),
        turn_id,
        seq,
        session_projection_seq: None,
        kind,
        happened_at: Utc::now(),
        source_refs: vec![],
        evidence_refs: vec![],
        cause_refs: command.cause_refs.clone(),
        result_refs: vec![],
        payload: EventPayload::from_json(kind, payload),
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
            key: "provider.codex".to_owned(),
            value: CapabilityValue::Provider {
                available: codex_available,
                configured: true,
                mode: "exec".to_owned(),
                timeout_seconds: Some(config.codex_timeout.as_secs()),
                unavailable_reason: (!codex_available).then(|| "binary_not_found".to_owned()),
            },
        },
        CapabilitySummary {
            key: "workspace.validation".to_owned(),
            value: CapabilityValue::WorkspaceValidation {
                mode: "explicit_path".to_owned(),
            },
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
    command_available_in_search_path(binary, &paths)
}

fn command_available_in_search_path(binary: &str, search_path: &std::ffi::OsStr) -> bool {
    std::env::split_paths(search_path).any(|directory| directory.join(binary).is_file())
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
        .join("uprava-node")
        .join(NODE_STATE_SLOT)
        .join("node.sqlite")
}

fn is_sqlite_state_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "sqlite")
}

fn legacy_state_path(path: &Path) -> Option<PathBuf> {
    let slot_dir = path.parent()?;
    if slot_dir.file_name()?.to_string_lossy() != NODE_STATE_SLOT {
        return None;
    }
    Some(slot_dir.parent()?.join("node.json"))
}

fn is_versioned_state_path(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.to_string_lossy() == NODE_STATE_SLOT)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use uprava_protocol::{CorrelationId, SessionThreadId};

    const NODE_CONFIG_ENV_VARS: &[&str] = &[
        "UPRAVA_CORE_URL",
        "UPRAVA_NODE_DISPLAY_NAME",
        "UPRAVA_NODE_HEARTBEAT_SECONDS",
        "UPRAVA_NODE_STATE_PATH",
        "UPRAVA_NODE_WORKSPACES",
        "UPRAVA_CODEX_BINARY",
        "UPRAVA_CODEX_IGNORE_USER_CONFIG",
        "UPRAVA_CODEX_TIMEOUT_SECONDS",
    ];

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
        let placement_count: i64 =
            sqlx::query_scalar("select count(*) from node_placement_sequences")
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
            .persist_enrollment_identity(
                NodeId::from("node-reopen"),
                "credential-reopen".to_owned(),
            )
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
        let path =
            std::env::temp_dir().join(format!("uprava-node-metrics-{}.sqlite", Uuid::new_v4()));
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
        let path =
            std::env::temp_dir().join(format!("uprava-node-shutdown-{}.sqlite", Uuid::new_v4()));
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
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
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

    #[test]
    fn node_config_from_env_requires_workspace_roots() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);

        let error = NodeConfig::from_env().expect_err("missing workspace roots should fail");

        assert!(error
            .to_string()
            .contains("UPRAVA_NODE_WORKSPACES must list one or more allowed workspace roots"));
    }

    #[test]
    fn node_config_from_env_defaults_codex_timeout_to_one_day() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);
        std::env::set_var("UPRAVA_NODE_WORKSPACES", std::env::temp_dir());

        let config = NodeConfig::from_env().expect("default node config parses");

        assert_eq!(config.codex_timeout, Duration::from_secs(24 * 60 * 60));
    }

    #[test]
    fn node_config_from_env_parses_overrides_and_workspace_list() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);
        let state_path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
        std::env::set_var("UPRAVA_CORE_URL", "http://127.0.0.1:19090");
        std::env::set_var("UPRAVA_NODE_DISPLAY_NAME", "Desktop Node");
        std::env::set_var("UPRAVA_NODE_HEARTBEAT_SECONDS", "2");
        std::env::set_var("UPRAVA_NODE_STATE_PATH", &state_path);
        std::env::set_var("UPRAVA_NODE_WORKSPACES", "/tmp/a, ,/tmp/b");
        std::env::set_var("UPRAVA_CODEX_BINARY", "/usr/local/bin/codex");
        std::env::set_var("UPRAVA_CODEX_IGNORE_USER_CONFIG", "true");
        std::env::set_var("UPRAVA_CODEX_TIMEOUT_SECONDS", "7");

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
        assert!(config.codex_ignore_user_config);
        assert_eq!(config.codex_timeout, Duration::from_secs(7));
    }

    #[test]
    fn node_config_from_env_rejects_invalid_duration_values() {
        let _lock = env_lock();
        let _env = EnvGuard::cleared(NODE_CONFIG_ENV_VARS);
        std::env::set_var("UPRAVA_NODE_WORKSPACES", std::env::temp_dir());
        std::env::set_var("UPRAVA_NODE_HEARTBEAT_SECONDS", "soon");

        let error = NodeConfig::from_env().expect_err("invalid heartbeat should fail");

        assert!(error
            .to_string()
            .contains("UPRAVA_NODE_HEARTBEAT_SECONDS must be an unsigned integer"));
    }

    #[test]
    fn command_available_returns_false_for_missing_absolute_binary() {
        let missing = std::env::temp_dir().join(format!("missing-codex-{}", Uuid::new_v4()));

        assert!(!command_available(&missing.display().to_string()));
    }

    #[test]
    fn command_available_resolves_binary_from_path() {
        let bin_dir = std::env::temp_dir().join(format!("uprava-node-bin-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&bin_dir).expect("bin dir creates");
        let codex_path = bin_dir.join("codex");
        std::fs::write(&codex_path, "").expect("codex fixture writes");

        let available = command_available_in_search_path("codex", bin_dir.as_os_str());
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

        assert!(matches!(
            capability.value,
            CapabilityValue::Provider {
                available: false,
                ..
            }
        ));
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
            command: Box::new(command_fixture(
                "bad-protocol-command",
                CommandKind::SendTurn,
            )),
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
    async fn prepare_command_dispatch_rejects_command_without_provider_metadata() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Failed);
        assert!(outcome.state_changed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::RuntimeError]
        );
        assert_eq!(local_state.event_outbox.len(), 1);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(1));
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
        assert_eq!(active_runtime_count(&local_state), 0);
        assert_eq!(
            local_state.command_status.get("command-1").copied(),
            Some(CommandState::Failed)
        );
        assert_eq!(
            outcome.events_to_send[0]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.missing")
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

        assert_eq!(second.status, CommandState::Failed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(1));
        assert_eq!(local_state.event_outbox.len(), 1);
    }

    #[tokio::test]
    async fn node_local_state_replays_outbox_for_duplicate_command_after_restart() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_event_ids = event_ids(&first.events_to_send);
        local_state
            .save(&path)
            .expect("node state with outbox saves");

        let mut reloaded_state = NodeLocalState::load(&path).expect("node state reloads");
        let second = prepare_command_dispatch(&config, &mut reloaded_state, &command).await;
        std::fs::remove_file(path).expect("node state fixture is removed");

        assert_eq!(second.status, CommandState::Failed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(
            reloaded_state.runtime_seqs.get("runtime-1").copied(),
            Some(1)
        );
        assert_eq!(reloaded_state.event_outbox.len(), 1);
    }

    #[tokio::test]
    async fn remove_acked_events_removes_only_accepted_event_ids() {
        let config = config_fixture();
        let command = command_fixture("command-1", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let accepted_event_id = outcome.events_to_send[0].event_id.clone();

        let removed = remove_acked_events(&mut local_state.event_outbox, &[accepted_event_id]);

        assert_eq!(removed, 1);
        assert_eq!(local_state.event_outbox.len(), 0);
        assert!(event_ids(&local_state.event_outbox).is_empty());
    }

    #[tokio::test]
    async fn event_outbox_retention_emits_runtime_error_when_runtime_events_are_dropped() {
        let command = command_fixture("command-retention", CommandKind::SendTurn);
        let mut local_state = NodeLocalState {
            node_id: Some(NodeId::from("node-1")),
            ..NodeLocalState::default()
        };
        let runtime_session_id = RuntimeSessionId::from("runtime-1");
        for _ in 0..6 {
            let event = event_for_command(
                "codex",
                &command,
                &mut local_state.runtime_seqs,
                runtime_session_id.clone(),
                None,
                EventKind::RuntimeRunning,
                serde_json::json!({ "provider": "codex" }),
            );
            local_state.event_outbox.push(event);
        }

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
        assert_eq!(local_state.dropped_event_count, 1);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(7));
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Error)
        );
    }

    #[tokio::test]
    async fn event_outbox_retention_drops_old_runtime_events_by_age() {
        let command = command_fixture("command-retention-age", CommandKind::SendTurn);
        let mut local_state = NodeLocalState {
            node_id: Some(NodeId::from("node-1")),
            ..NodeLocalState::default()
        };
        let runtime_session_id = RuntimeSessionId::from("runtime-age");
        let mut old_event = event_for_command(
            "codex",
            &command,
            &mut local_state.runtime_seqs,
            runtime_session_id.clone(),
            None,
            EventKind::RuntimeRunning,
            serde_json::json!({ "provider": "codex" }),
        );
        old_event.happened_at = Utc::now() - chrono::Duration::seconds(10);
        let recent_event = event_for_command(
            "codex",
            &command,
            &mut local_state.runtime_seqs,
            runtime_session_id,
            None,
            EventKind::RuntimeRunning,
            serde_json::json!({ "provider": "codex" }),
        );
        local_state.event_outbox.push(old_event);
        local_state.event_outbox.push(recent_event);

        let notices = enforce_event_outbox_retention_with_limits(
            &mut local_state,
            10,
            Duration::from_secs(1),
            usize::MAX,
        );

        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, EventKind::RuntimeError);
        assert_eq!(local_state.dropped_event_count, 1);
        assert!(local_state
            .event_outbox
            .iter()
            .all(|event| event.happened_at >= Utc::now() - chrono::Duration::seconds(11)));
    }

    #[tokio::test]
    async fn event_outbox_retention_drops_oldest_events_by_serialized_bytes() {
        let command = command_fixture("command-retention-bytes", CommandKind::SendTurn);
        let mut local_state = NodeLocalState {
            node_id: Some(NodeId::from("node-1")),
            ..NodeLocalState::default()
        };
        for index in 0..3 {
            let mut event = event_for_command(
                "codex",
                &command,
                &mut local_state.runtime_seqs,
                RuntimeSessionId::from("runtime-bytes"),
                None,
                EventKind::ProviderActivity,
                serde_json::json!({ "data": "x".repeat(1024), "index": index }),
            );
            event.runtime_session_id = None;
            event.scope_ref = ScopeRef::Node {
                node_id: NodeId::from("node-1"),
            };
            local_state.event_outbox.push(event);
        }
        let max_bytes = serialized_event_len(&local_state.event_outbox[2]) + 1;

        let notices = enforce_event_outbox_retention_with_limits(
            &mut local_state,
            10,
            Duration::ZERO,
            max_bytes,
        );
        let retained_bytes = local_state
            .event_outbox
            .iter()
            .map(serialized_event_len)
            .sum::<usize>();

        assert!(notices.is_empty());
        assert!(retained_bytes <= max_bytes);
        assert!(local_state.event_outbox.len() <= 1);
        assert!(local_state.dropped_event_count >= 2);
    }

    #[tokio::test]
    async fn failed_command_dispatch_replays_failed_status_and_outbox_for_duplicate_command() {
        let config = config_fixture();
        let command = command_fixture("command-error", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_event_ids = event_ids(&first.events_to_send);

        let second = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(second.status, CommandState::Failed);
        assert!(!second.state_changed);
        assert_eq!(event_ids(&second.events_to_send), first_event_ids);
        assert_eq!(local_state.runtime_seqs.get("runtime-1").copied(), Some(1));
    }

    #[tokio::test]
    async fn codex_resolve_approval_returns_runtime_to_ready() {
        let config = config_fixture();
        let mut command = command_fixture("command-resolve", CommandKind::ResolveApproval);
        command.payload = CommandPayload::ResolveApproval {
            approval_id: ApprovalId::from("approval-1"),
            approved: true,
            message: Some("approved".to_owned()),
        };
        let mut local_state = NodeLocalState::default();
        local_state.runtime_transcripts.insert(
            "runtime-1".to_owned(),
            vec![ProviderTranscriptMessage {
                role: "user".to_owned(),
                content: "stale context".to_owned(),
            }],
        );
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());

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
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());

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

    #[cfg(unix)]
    #[tokio::test]
    async fn validate_workspace_command_rejects_symlink_escape_from_allowed_root() {
        let allowed_root = std::env::temp_dir().join(format!("uprava-allowed-{}", Uuid::new_v4()));
        let outside_root = std::env::temp_dir().join(format!("uprava-outside-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&allowed_root).expect("allowed root creates");
        std::fs::create_dir_all(&outside_root).expect("outside root creates");
        let escaped_workspace = allowed_root.join("escaped");
        std::os::unix::fs::symlink(&outside_root, &escaped_workspace)
            .expect("escaped workspace symlink creates");
        let mut config = config_fixture();
        config.workspace_paths = vec![allowed_root.clone()];
        let command = placement_command_fixture(
            "command-validate-symlink-escape",
            "placement-symlink-escape",
            "workspace",
            &escaped_workspace.display().to_string(),
        );
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_dir_all(&allowed_root).expect("allowed root removes");
        std::fs::remove_dir_all(&outside_root).expect("outside root removes");
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

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_start_runtime_rejects_symlink_workspace_escape() {
        let allowed_root = std::env::temp_dir().join(format!("uprava-allowed-{}", Uuid::new_v4()));
        let outside_root = std::env::temp_dir().join(format!("uprava-outside-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&allowed_root).expect("allowed root creates");
        std::fs::create_dir_all(&outside_root).expect("outside root creates");
        let escaped_workspace = allowed_root.join("escaped");
        std::os::unix::fs::symlink(&outside_root, &escaped_workspace)
            .expect("escaped workspace symlink creates");
        let mut config = config_fixture();
        config.workspace_paths = vec![allowed_root.clone()];
        let mut command = command_fixture(
            "command-codex-start-symlink-escape",
            CommandKind::StartRuntime,
        );
        command.payload = CommandPayload::StartRuntime {
            provider: "codex".to_owned(),
            workspace_path: escaped_workspace.display().to_string(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_dir_all(&allowed_root).expect("allowed root removes");
        std::fs::remove_dir_all(&outside_root).expect("outside root removes");
        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::RuntimeError]
        );
        assert!(local_state.runtime_workspace_paths.is_empty());
        assert_eq!(
            outcome.events_to_send[0]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("workspace.outside_allowed_roots")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_rechecks_cached_workspace_against_allowed_roots() {
        let allowed_root = std::env::temp_dir().join(format!("uprava-allowed-{}", Uuid::new_v4()));
        let outside_root = std::env::temp_dir().join(format!("uprava-outside-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&allowed_root).expect("allowed root creates");
        std::fs::create_dir_all(&outside_root).expect("outside root creates");
        let escaped_workspace = allowed_root.join("escaped");
        std::os::unix::fs::symlink(&outside_root, &escaped_workspace)
            .expect("escaped workspace symlink creates");
        let mut config = config_fixture();
        config.workspace_paths = vec![allowed_root.clone()];
        let command = command_fixture("command-codex-send-symlink-escape", CommandKind::SendTurn);
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state.runtime_workspace_paths.insert(
            "runtime-1".to_owned(),
            escaped_workspace.display().to_string(),
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        std::fs::remove_dir_all(&allowed_root).expect("allowed root removes");
        std::fs::remove_dir_all(&outside_root).expect("outside root removes");
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
            Some("workspace.outside_allowed_roots")
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
        command.payload = CommandPayload::RefreshResourceSnapshot {
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.display().to_string(),
        };
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

    #[tokio::test]
    async fn read_workspace_file_command_returns_text_content() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::fs::write(workspace_path.join("README.md"), "hello inspector")
            .expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-read-file",
            "placement-read-file",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ReadWorkspaceFile;
        command.payload = CommandPayload::ReadWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            path: "README.md".to_owned(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response =
            serde_json::from_value::<WorkspaceFileContentResponse>(outcome.result_payload.0)
                .expect("workspace file response decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(response.metadata.status, WorkspaceEntryStatus::Readable);
        assert_eq!(response.content.as_deref(), Some("hello inspector"));
    }

    #[tokio::test]
    async fn read_workspace_file_command_replays_payload_after_restart() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        let state_path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::fs::write(workspace_path.join("README.md"), "hello inspector")
            .expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-read-file-replay",
            "placement-read-file-replay",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ReadWorkspaceFile;
        command.payload = CommandPayload::ReadWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            path: "README.md".to_owned(),
        };
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_payload = first.result_payload.clone();
        local_state
            .save(&state_path)
            .expect("node state with result payload saves");

        let mut reloaded_state = NodeLocalState::load(&state_path).expect("node state reloads");
        let second = prepare_command_dispatch(&config, &mut reloaded_state, &command).await;
        let response =
            serde_json::from_value::<WorkspaceFileContentResponse>(second.result_payload.0.clone())
                .expect("replayed workspace file response decodes");

        std::fs::remove_file(&state_path).expect("state fixture removes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");
        assert_eq!(second.status, CommandState::Completed);
        assert!(!second.state_changed);
        assert_eq!(second.result_payload.0, first_payload.0);
        assert_eq!(response.content.as_deref(), Some("hello inspector"));
    }

    #[tokio::test]
    async fn list_workspace_tree_marks_and_allows_generated_directories() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(workspace_path.join("target/debug"))
            .expect("generated fixture creates");
        std::fs::write(workspace_path.join("target/debug/app"), "compiled")
            .expect("generated file fixture writes");
        let mut command = placement_command_fixture(
            "command-list-tree",
            "placement-list-tree",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ListWorkspaceTree;
        command.payload = CommandPayload::ListWorkspaceTree {
            workspace_path: workspace_path.display().to_string(),
            path: ".".to_owned(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response = serde_json::from_value::<WorkspaceTreeResponse>(outcome.result_payload.0)
            .expect("workspace tree response decodes");
        let target = response
            .root
            .children
            .iter()
            .find(|entry| entry.name == "target")
            .expect("target entry appears");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(target.status, WorkspaceEntryStatus::Directory);
        assert_eq!(
            target.classification,
            WorkspaceEntryClassification::Generated
        );
        assert!(target.expandable);
        assert!(target.children.is_empty());
    }

    #[tokio::test]
    async fn list_workspace_tree_shows_dotfiles_and_limits_sorted_children() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-tree-limit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(workspace_path.join(".github")).expect("dot directory creates");
        std::fs::write(workspace_path.join(".env"), "visible").expect("dot file creates");
        for index in 0..100 {
            std::fs::write(
                workspace_path.join(format!("file-{index:03}.txt")),
                "fixture",
            )
            .expect("limit fixture creates");
        }
        let mut command = placement_command_fixture(
            "command-list-limited-tree",
            "placement-list-limited-tree",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ListWorkspaceTree;
        command.payload = CommandPayload::ListWorkspaceTree {
            workspace_path: workspace_path.display().to_string(),
            path: ".".to_owned(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response = serde_json::from_value::<WorkspaceTreeResponse>(outcome.result_payload.0)
            .expect("workspace tree response decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(response.total_entries, Some(102));
        assert!(response.truncated);
        assert_eq!(response.root.children.len(), 100);
        assert_eq!(response.root.children[0].name, ".github");
        assert_eq!(response.root.children[1].name, ".env");
    }

    #[tokio::test]
    async fn read_workspace_file_command_rejects_parent_path_escape() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let mut command = placement_command_fixture(
            "command-read-escape",
            "placement-read-escape",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ReadWorkspaceFile;
        command.payload = CommandPayload::ReadWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            path: "../secret.txt".to_owned(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let error_code = outcome
            .result_payload
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(error_code.as_deref(), Some("workspace.path_escape"));
    }

    #[tokio::test]
    async fn write_workspace_file_command_updates_text_when_expected_content_matches() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::fs::write(workspace_path.join("README.md"), "before").expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-write-file",
            "placement-write-file",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::WriteWorkspaceFile;
        command.payload = CommandPayload::WriteWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceFileWriteRequest {
                path: "README.md".to_owned(),
                content: "after".to_owned(),
                expected_content: Some("before".to_owned()),
            },
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response =
            serde_json::from_value::<WorkspaceFileWriteResponse>(outcome.result_payload.0)
                .expect("workspace write response decodes");
        let written =
            std::fs::read_to_string(workspace_path.join("README.md")).expect("written file reads");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert_eq!(response.path, "README.md");
        assert_eq!(written, "after");
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![EventKind::WorkspaceFileWritten]
        );
        assert!(outcome.events_to_send[0]
            .cause_refs
            .iter()
            .any(|reference| matches!(
                reference,
                UpravaRef::Command { command_id } if command_id == &command.command_id
            )));
        assert!(outcome.events_to_send[0]
            .result_refs
            .iter()
            .any(|reference| matches!(reference, UpravaRef::WorkspaceEdit { .. })));
    }

    #[tokio::test]
    async fn write_workspace_file_command_replays_payload_after_restart_without_rewriting() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        let state_path = std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::fs::write(workspace_path.join("README.md"), "before").expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-write-file-replay",
            "placement-write-file-replay",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::WriteWorkspaceFile;
        command.payload = CommandPayload::WriteWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceFileWriteRequest {
                path: "README.md".to_owned(),
                content: "after".to_owned(),
                expected_content: Some("before".to_owned()),
            },
        };
        let mut local_state = NodeLocalState::default();
        let first = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let first_payload = first.result_payload.clone();
        local_state
            .save(&state_path)
            .expect("node state with write result payload saves");
        std::fs::write(workspace_path.join("README.md"), "external change")
            .expect("post-command file fixture changes");

        let mut reloaded_state = NodeLocalState::load(&state_path).expect("node state reloads");
        let second = prepare_command_dispatch(&config, &mut reloaded_state, &command).await;
        let written =
            std::fs::read_to_string(workspace_path.join("README.md")).expect("written file reads");

        std::fs::remove_file(&state_path).expect("state fixture removes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");
        assert_eq!(second.status, CommandState::Completed);
        assert!(!second.state_changed);
        assert_eq!(second.result_payload.0, first_payload.0);
        assert_eq!(written, "external change");
    }

    #[tokio::test]
    async fn write_workspace_file_command_rejects_stale_expected_content() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::fs::write(workspace_path.join("README.md"), "current").expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-write-conflict",
            "placement-write-conflict",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::WriteWorkspaceFile;
        command.payload = CommandPayload::WriteWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceFileWriteRequest {
                path: "README.md".to_owned(),
                content: "after".to_owned(),
                expected_content: Some("before".to_owned()),
            },
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let error_code = outcome
            .result_payload
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(error_code.as_deref(), Some("workspace.write_conflict"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_workspace_file_command_rejects_symlink_target() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        let outside_path =
            std::env::temp_dir().join(format!("uprava-node-outside-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        std::os::unix::fs::symlink(&outside_path, workspace_path.join("link.txt"))
            .expect("symlink fixture creates");
        let mut command = placement_command_fixture(
            "command-write-symlink",
            "placement-write-symlink",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::WriteWorkspaceFile;
        command.payload = CommandPayload::WriteWorkspaceFile {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceFileWriteRequest {
                path: "link.txt".to_owned(),
                content: "after".to_owned(),
                expected_content: None,
            },
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let error_code = outcome
            .result_payload
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(error_code.as_deref(), Some("workspace.write_symlink"));
        assert!(!outside_path.exists());
    }

    #[tokio::test]
    async fn run_workspace_command_captures_stdout_and_exit_status() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let mut command = placement_command_fixture(
            "command-run-workspace",
            "placement-run-workspace",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::RunWorkspaceCommand;
        command.payload = CommandPayload::RunWorkspaceCommand {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceCommandRunRequest {
                command: "rustc".to_owned(),
                args: vec!["--version".to_owned()],
                intent: WorkspaceCommandIntent::Command,
                label: None,
                timeout_seconds: Some(30),
            },
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response =
            serde_json::from_value::<WorkspaceCommandRunResponse>(outcome.result_payload.0)
                .expect("workspace command response decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert!(response.success, "stderr: {}", response.stderr);
        assert!(response.stdout.contains("rustc"));
    }

    #[tokio::test]
    async fn run_workspace_command_rejects_disallowed_executable() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let mut command = placement_command_fixture(
            "command-run-disallowed",
            "placement-run-disallowed",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::RunWorkspaceCommand;
        command.payload = CommandPayload::RunWorkspaceCommand {
            workspace_path: workspace_path.display().to_string(),
            request: WorkspaceCommandRunRequest {
                command: "sh".to_owned(),
                args: vec!["-c".to_owned(), "echo blocked".to_owned()],
                intent: WorkspaceCommandIntent::Command,
                label: None,
                timeout_seconds: Some(30),
            },
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let error_code = outcome
            .result_payload
            .0
            .get("error_code")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(error_code.as_deref(), Some("workspace.command_not_allowed"));
    }

    #[tokio::test]
    async fn run_workspace_process_caps_stdout_during_execution() {
        let output = run_workspace_process(
            &std::env::temp_dir(),
            "rustc",
            &["--print".to_owned(), "target-list".to_owned()],
            Duration::from_secs(30),
            64,
            64,
        )
        .await;

        assert!(output.success, "stderr: {}", output.stderr);
        assert!(output.stdout.len() <= 64);
        assert!(output.stdout_truncated);
    }

    #[tokio::test]
    async fn read_workspace_diff_command_returns_git_diff() {
        let config = config_fixture();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-node-inspector-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        StdCommand::new("git")
            .arg("init")
            .current_dir(&workspace_path)
            .status()
            .expect("git init starts");
        std::fs::write(workspace_path.join("README.md"), "before\n").expect("text fixture writes");
        StdCommand::new("git")
            .args(["add", "README.md"])
            .current_dir(&workspace_path)
            .status()
            .expect("git add starts");
        StdCommand::new("git")
            .args(["-c", "user.email=test@example.invalid"])
            .args(["-c", "user.name=Uprava Test"])
            .args(["commit", "-m", "initial"])
            .current_dir(&workspace_path)
            .status()
            .expect("git commit starts");
        std::fs::write(workspace_path.join("README.md"), "after\n").expect("text fixture writes");
        let mut command = placement_command_fixture(
            "command-read-diff",
            "placement-read-diff",
            "workspace",
            &workspace_path.display().to_string(),
        );
        command.kind = CommandKind::ReadWorkspaceDiff;
        command.payload = CommandPayload::ReadWorkspaceDiff {
            workspace_path: workspace_path.display().to_string(),
        };
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let response = serde_json::from_value::<WorkspaceDiffResponse>(outcome.result_payload.0)
            .expect("workspace diff response decodes");
        std::fs::remove_dir_all(&workspace_path).expect("workspace fixture removes");

        assert_eq!(outcome.status, CommandState::Completed);
        assert!(response.diff.contains("-before"));
        assert!(response.diff.contains("+after"));
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
            std::env::temp_dir().join(format!("uprava-non-git-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");

        let badges = resource_warnings(&workspace_path);
        std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

        assert!(badges.is_empty());
    }

    #[tokio::test]
    async fn codex_start_runtime_records_provider_and_workspace_metadata() {
        let config = config_fixture();
        let workspace_path_buf = std::env::temp_dir();
        let workspace_path = workspace_path_buf.display().to_string();
        let canonical_workspace_path = std::fs::canonicalize(&workspace_path_buf)
            .expect("temp dir canonicalizes")
            .display()
            .to_string();
        let mut command = command_fixture("command-codex-start", CommandKind::StartRuntime);
        command.payload = CommandPayload::StartRuntime {
            provider: "codex".to_owned(),
            workspace_path: workspace_path.clone(),
        };
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
            Some(canonical_workspace_path.as_str())
        );
        assert_eq!(
            local_state
                .runtime_transcripts
                .get("runtime-1")
                .map(Vec::len),
            Some(0)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_deduction_is_ephemeral_read_only_and_returns_structured_output() {
        let capture_path =
            std::env::temp_dir().join(format!("uprava-deduction-args-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_deduction_binary(&capture_path);
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let scope_ref = UpravaRef::Session {
            session_thread_id: SessionThreadId::from("session-1"),
        };
        let package = DeductionInputPackage {
            deduction_id: uprava_protocol::DeductionId::from("deduction-1"),
            session_thread_id: SessionThreadId::from("session-1"),
            scope_ref: scope_ref.clone(),
            question: "What caused the result?".to_owned(),
            evidence_snapshot_hash: "snapshot-hash-1".to_owned(),
            trace_steps: vec![],
            events: vec![],
            allowed_refs: vec![scope_ref],
            truncated: false,
            generated_at: Utc::now(),
        };
        let mut command = command_fixture("command-deduction", CommandKind::RequestDeduction);
        command.payload = CommandPayload::RequestDeduction {
            package: Box::new(package),
        };
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state.runtime_workspace_paths.insert(
            "runtime-1".to_owned(),
            std::env::temp_dir().display().to_string(),
        );

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;
        let output = serde_json::from_value::<DeductionProviderOutput>(outcome.result_payload.0)
            .expect("deduction output decodes");
        let args = std::fs::read_to_string(&capture_path).expect("deduction args captured");

        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_file(capture_path).expect("capture fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert!(outcome.events_to_send.is_empty());
        assert_eq!(output.deduction_id.as_str(), "deduction-1");
        assert_eq!(output.schema_version, DEDUCTION_SCHEMA_VERSION);
        assert!(output.result.is_some());
        assert!(args.lines().any(|arg| arg == "--ephemeral"));
        assert!(args.lines().any(|arg| arg == "read-only"));
        assert!(args.lines().any(|arg| arg == "--output-schema"));
        assert!(!args.contains("dangerously-bypass-approvals-and-sandbox"));
        assert!(!args.lines().any(|arg| arg == "resume"));
        assert!(local_state.runtime_transcripts.is_empty());
        assert!(local_state.runtime_provider_resume_refs.is_empty());
    }

    #[tokio::test]
    async fn cancelled_deduction_tombstone_prevents_late_provider_start() {
        let config = config_fixture_with_codex_binary("missing-codex-that-must-not-start");
        let mut cancel = command_fixture("command-cancel-deduction", CommandKind::CancelDeduction);
        cancel.payload = CommandPayload::CancelDeduction {
            deduction_id: uprava_protocol::DeductionId::from("deduction-cancelled"),
        };
        let scope_ref = UpravaRef::Session {
            session_thread_id: SessionThreadId::from("session-1"),
        };
        let mut request = command_fixture("command-late-deduction", CommandKind::RequestDeduction);
        request.payload = CommandPayload::RequestDeduction {
            package: Box::new(DeductionInputPackage {
                deduction_id: uprava_protocol::DeductionId::from("deduction-cancelled"),
                session_thread_id: SessionThreadId::from("session-1"),
                scope_ref: scope_ref.clone(),
                question: "Why?".to_owned(),
                evidence_snapshot_hash: "snapshot-hash".to_owned(),
                trace_steps: vec![],
                events: vec![],
                allowed_refs: vec![scope_ref],
                truncated: false,
                generated_at: Utc::now(),
            }),
        };
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state.runtime_workspace_paths.insert(
            "runtime-1".to_owned(),
            std::env::temp_dir().display().to_string(),
        );

        let cancelled = prepare_command_dispatch(&config, &mut local_state, &cancel).await;
        let late = prepare_command_dispatch(&config, &mut local_state, &request).await;
        let output = serde_json::from_value::<DeductionProviderOutput>(late.result_payload.0)
            .expect("cancelled provider output decodes");

        assert_eq!(cancelled.status, CommandState::Completed);
        assert_eq!(late.status, CommandState::Failed);
        assert_eq!(output.error_code.as_deref(), Some("deduction.cancelled"));
        assert!(!local_state
            .cancelled_deductions
            .contains("deduction-cancelled"));
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
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
        assert_eq!(
            outcome.status,
            CommandState::Completed,
            "Codex success fixture returned unexpected events: {:#?}",
            outcome.events_to_send
        );
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::ProviderActivity,
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
            outcome.events_to_send[2]
                .payload
                .0
                .get("provider_event_type")
                .and_then(serde_json::Value::as_str),
            Some("response.completed")
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("raw_event")
                .and_then(|value| value.get("session_id"))
                .and_then(serde_json::Value::as_str),
            Some("codex-session-1")
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
    async fn codex_send_turn_includes_required_noninteractive_flags() {
        let capture_path =
            std::env::temp_dir().join(format!("uprava-codex-args-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_args_capture_binary(&capture_path);
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        let command =
            command_fixture_with_content("command-codex-args", CommandKind::SendTurn, "status");
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        let captured_args = std::fs::read_to_string(&capture_path).expect("captured args read");
        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_file(capture_path).expect("args capture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Completed);
        assert_codex_launch_flags(&captured_args);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_includes_prior_transcript_context() {
        let capture_path =
            std::env::temp_dir().join(format!("uprava-codex-prompt-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_prompt_capture_binary(&capture_path);
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
        assert!(captured_prompt.contains("Continue this Uprava session"));
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
            std::env::temp_dir().join(format!("uprava-codex-resume-args-{}", Uuid::new_v4()));
        let codex_binary = fake_codex_resume_capture_binary(&capture_path);
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
        assert_codex_launch_flags(&captured_args);
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
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
                EventKind::ProviderActivity,
                EventKind::ApprovalRequested,
                EventKind::RuntimeBlocked,
            ]
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("approval_id")
                .and_then(serde_json::Value::as_str),
            Some("approval-codex-1")
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("prompt")
                .and_then(serde_json::Value::as_str),
            Some("Allow file edit?")
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("source")
                .and_then(serde_json::Value::as_str),
            Some("codex.exec.jsonl")
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("raw_event")
                .and_then(|value| value.get("approval_id"))
                .and_then(serde_json::Value::as_str),
            Some("approval-codex-1")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Blocked)
        );
        assert!(local_state
            .runtime_transcripts
            .get("runtime-1")
            .is_none_or(Vec::is_empty));
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

    #[test]
    fn codex_activity_payload_preserves_unknown_raw_json_fields() {
        let payload = codex_activity_payload_from_json(
            "codex",
            serde_json::json!({
                "type": "item.completed",
                "item": {
                    "id": "item-1",
                    "type": "command_execution",
                    "command": "make c",
                    "status": "completed",
                    "future_field": { "kept": true }
                }
            }),
        );

        assert_eq!(
            payload
                .get("provider_event_type")
                .and_then(serde_json::Value::as_str),
            Some("item.completed")
        );
        assert_eq!(
            payload
                .get("provider_item_type")
                .and_then(serde_json::Value::as_str),
            Some("command_execution")
        );
        assert_eq!(
            payload
                .get("raw_event")
                .and_then(|raw| raw.get("item"))
                .and_then(|item| item.get("future_field"))
                .and_then(|field| field.get("kept"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn codex_stdout_activity_event_records_malformed_jsonl_as_parse_error() {
        let command = command_fixture("command-codex-parse-error", CommandKind::SendTurn);
        let mut runtime_seqs = HashMap::new();

        let event = codex_stdout_activity_event(
            "codex",
            &command,
            &mut runtime_seqs,
            RuntimeSessionId::from("runtime-1"),
            Some(TurnId::from("turn-1")),
            "{bad json",
        );

        assert_eq!(event.kind, EventKind::ProviderActivity);
        assert_eq!(
            event
                .payload
                .0
                .get("provider_event_type")
                .and_then(serde_json::Value::as_str),
            Some("parse_error")
        );
        assert_eq!(runtime_seqs.get("runtime-1").copied(), Some(1));
    }

    #[cfg(unix)]
    fn assert_codex_launch_flags(captured_args: &str) {
        for flag in [
            "--skip-git-repo-check",
            "--dangerously-bypass-approvals-and-sandbox",
        ] {
            assert!(
                captured_args.contains(&format!("\n{flag}\n")),
                "captured args did not include {flag}: {captured_args}"
            );
        }
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
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
                EventKind::ProviderActivity,
                EventKind::RuntimeError,
            ]
        );
        assert_eq!(
            outcome.events_to_send[3]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.exec_failed")
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("provider_event_type")
                .and_then(serde_json::Value::as_str),
            Some("stderr")
        );
        assert!(outcome.events_to_send[3]
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
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
        let event_kinds = event_kinds(&outcome.events_to_send);
        assert!(event_kinds.len() >= 3);
        assert_eq!(event_kinds.first(), Some(&EventKind::RuntimeRunning));
        assert_eq!(event_kinds.get(1), Some(&EventKind::TurnStarted));
        assert!(event_kinds[2..event_kinds.len() - 1]
            .iter()
            .all(|kind| *kind == EventKind::ProviderActivity));
        assert_eq!(event_kinds.last(), Some(&EventKind::RuntimeError));
        let runtime_error = outcome
            .events_to_send
            .last()
            .expect("timeout emits a runtime error");
        assert_eq!(
            runtime_error
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
    async fn codex_send_turn_cancellation_interrupts_active_process() {
        let codex_binary = fake_codex_slow_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_path).expect("workspace fixture creates");
        let mut config = config_fixture_with_codex_binary(codex_binary.display().to_string());
        config.codex_timeout = Duration::from_secs(5);
        let command =
            command_fixture_with_content("command-codex-cancel", CommandKind::SendTurn, "status");
        let mut local_state = NodeLocalState::default();
        local_state
            .runtime_providers
            .insert("runtime-1".to_owned(), "codex".to_owned());
        local_state
            .runtime_workspace_paths
            .insert("runtime-1".to_owned(), workspace_path.display().to_string());
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let cancel_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = cancel_tx.send(true);
        });

        let outcome = prepare_command_dispatch_with_live_socket(
            &config,
            &mut local_state,
            &command,
            None,
            None,
            Some(cancel_rx),
        )
        .await;

        cancel_task.await.expect("cancel task joins");
        std::fs::remove_file(codex_binary).expect("codex fixture removes");
        std::fs::remove_dir_all(workspace_path).expect("workspace fixture removes");
        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            event_kinds(&outcome.events_to_send),
            vec![
                EventKind::RuntimeRunning,
                EventKind::TurnStarted,
                EventKind::TurnInterrupted,
            ]
        );
        assert_eq!(
            outcome.events_to_send[2]
                .payload
                .0
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("provider.cancelled")
        );
        assert_eq!(
            local_state.runtime_states.get("runtime-1").copied(),
            Some(RuntimeSessionState::Interrupted)
        );
    }

    #[tokio::test]
    async fn runtime_cancellation_signals_before_dispatch_capacity_is_available() {
        let cancellations = ExecutionCancellationRegistry::default();
        let guard = cancellations
            .begin(runtime_cancellation_key(&RuntimeSessionId::from(
                "runtime-1",
            )))
            .await;
        let mut cancellation = guard.receiver();
        let command = command_fixture("command-cancel-saturated", CommandKind::InterruptRuntime);
        let concurrency = Arc::new(Semaphore::new(0));
        let task = tokio::spawn({
            let cancellations = cancellations.clone();
            let concurrency = concurrency.clone();
            async move { prepare_command_dispatch_task(&command, &cancellations, concurrency).await }
        });

        timeout(Duration::from_secs(1), cancellation.changed())
            .await
            .expect("cancellation is not blocked by dispatch capacity")
            .expect("cancellation sender remains available");

        task.abort();
        let _ = task.await;
        cancellations.finish(guard).await;
        assert!(*cancellation.borrow());
    }

    #[tokio::test]
    async fn deduction_cancellation_is_scoped_away_from_live_turns() {
        let cancellations = ExecutionCancellationRegistry::default();
        let live_guard = cancellations
            .begin(runtime_cancellation_key(&RuntimeSessionId::from(
                "runtime-1",
            )))
            .await;
        let deduction_guard = cancellations
            .begin(deduction_cancellation_key(
                &uprava_protocol::DeductionId::from("deduction-1"),
            ))
            .await;
        let live_cancellation = live_guard.receiver();
        let mut deduction_cancellation = deduction_guard.receiver();
        let mut command = command_fixture("command-cancel-deduction", CommandKind::CancelDeduction);
        command.payload = CommandPayload::CancelDeduction {
            deduction_id: uprava_protocol::DeductionId::from("deduction-1"),
        };

        let permit =
            prepare_command_dispatch_task(&command, &cancellations, Arc::new(Semaphore::new(1)))
                .await;

        assert!(permit.is_some());
        assert!(!live_cancellation.has_changed().expect("live sender exists"));
        deduction_cancellation
            .changed()
            .await
            .expect("deduction sender remains available");
        assert!(*deduction_cancellation.borrow());
        cancellations.finish(live_guard).await;
        cancellations.finish(deduction_guard).await;

        let mut early_cancel = command_fixture(
            "command-cancel-early-deduction",
            CommandKind::CancelDeduction,
        );
        early_cancel.payload = CommandPayload::CancelDeduction {
            deduction_id: uprava_protocol::DeductionId::from("deduction-early"),
        };
        let permit = prepare_command_dispatch_task(
            &early_cancel,
            &cancellations,
            Arc::new(Semaphore::new(1)),
        )
        .await;
        assert!(permit.is_some());
        let early_guard = cancellations
            .begin(deduction_cancellation_key(
                &uprava_protocol::DeductionId::from("deduction-early"),
            ))
            .await;
        assert!(*early_guard.receiver().borrow());
        cancellations.finish(early_guard).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codex_send_turn_maps_slow_process_to_execution_timeout_error() {
        let codex_binary = fake_codex_slow_binary();
        let workspace_path =
            std::env::temp_dir().join(format!("uprava-codex-workspace-{}", Uuid::new_v4()));
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
            Some("provider.execution_timeout")
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
    fn fake_codex_args_capture_binary(capture_path: &Path) -> PathBuf {
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
printf '%s\n' 'Codex args accepted' > "$output_path"
printf '%s\n' '{{"type":"response.completed","session_id":"codex-session-1","resume_cursor":"cursor-1"}}'
"#,
            capture_path.display()
        ))
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
    fn fake_codex_deduction_binary(capture_path: &Path) -> PathBuf {
        fake_codex_binary(&format!(
            r#"#!/bin/sh
output_path=""
schema_path=""
capture_next=""
for arg in "$@"; do
  if [ "$capture_next" = "output" ]; then
    output_path="$arg"
    capture_next=""
  elif [ "$capture_next" = "schema" ]; then
    schema_path="$arg"
    capture_next=""
  elif [ "$arg" = "--output-last-message" ]; then
    capture_next="output"
  elif [ "$arg" = "--output-schema" ]; then
    capture_next="schema"
  fi
done
if [ -z "$output_path" ] || [ -z "$schema_path" ] || [ ! -f "$schema_path" ]; then
  echo "missing structured output paths" >&2
  exit 2
fi
printf '%s\n' "$@" > '{}'
printf '%s\n' '{{"title":"Root cause","conclusion":"The session evidence supports the result.","certainty":"high","steps":[{{"step_id":"step-1","classification":"observed","summary":"The session is the bounded scope.","support_refs":[{{"kind":"session","session_thread_id":"session-1"}}]}}],"assumptions":[],"unknowns":[],"alternatives":[]}}' > "$output_path"
printf '%s\n' '{{"type":"response.completed"}}'
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
            state_path: std::env::temp_dir().join(format!("uprava-node-{}.json", Uuid::new_v4())),
            workspace_paths: vec![std::env::temp_dir()],
            codex_binary: codex_binary.into(),
            codex_ignore_user_config: false,
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
        let payload = match kind {
            CommandKind::StartRuntime => CommandPayload::StartRuntime {
                provider: "codex".to_owned(),
                workspace_path: std::env::temp_dir().display().to_string(),
            },
            CommandKind::ResumeRuntime => CommandPayload::ResumeRuntime {
                provider: "codex".to_owned(),
                workspace_path: std::env::temp_dir().display().to_string(),
                provider_resume_ref: None,
            },
            CommandKind::SendTurn => CommandPayload::SendTurn {
                turn_id: TurnId::from("turn-1"),
                content: content.to_owned(),
            },
            CommandKind::ResolveApproval => CommandPayload::ResolveApproval {
                approval_id: ApprovalId::from("approval-1"),
                approved: true,
                message: Some("approved".to_owned()),
            },
            CommandKind::InterruptRuntime => CommandPayload::InterruptRuntime,
            CommandKind::StopRuntime => CommandPayload::StopRuntime,
            _ => CommandPayload::Extension {
                name: "test.fixture".to_owned(),
                value: JsonValue(serde_json::json!({})),
            },
        };
        CommandEnvelope {
            command_id: CommandId::from(command_id),
            kind,
            target: CommandTarget::SessionRuntime {
                node_id: NodeId::from("node-1"),
                project_placement_id: ProjectPlacementId::from("placement-1"),
                session_thread_id: SessionThreadId::from("session-1"),
                runtime_session_id: RuntimeSessionId::from("runtime-1"),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("correlation-1"),
            payload,
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
            target: CommandTarget::Placement {
                node_id: NodeId::from("node-1"),
                project_placement_id: ProjectPlacementId::from(placement_id),
            },
            actor_ref: ActorRef::local_user(),
            source_refs: vec![],
            cause_refs: vec![],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("correlation-1"),
            payload: CommandPayload::ValidateWorkspace {
                display_name: display_name.to_owned(),
                workspace_path: workspace_path.to_owned(),
            },
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
        events.iter().map(|event| event.kind).collect()
    }

    fn badge_kinds(badges: &[ResourceBadge]) -> Vec<&str> {
        badges.iter().map(|badge| badge.kind.as_str()).collect()
    }

    #[tokio::test]
    async fn terminal_manager_rebinds_routes_without_dropping_pty_handles() {
        let (old_sender, _old_receiver) = mpsc::channel(4);
        let (new_sender, _new_receiver) = mpsc::channel(4);
        let route = Arc::new(RwLock::new(Some(old_sender)));
        let control_tx = mpsc::unbounded_channel().0;
        let mut manager = WorkspaceTerminalManager::default();
        manager.terminals.insert(
            "terminal-1".to_owned(),
            WorkspaceTerminalHandle {
                replay: Arc::new(RwLock::new(VecDeque::new())),
                control_tx,
                sender_route: route.clone(),
                task: tokio::spawn(async {}),
            },
        );

        manager.rebind_sender(&new_sender).await;
        assert!(route.read().await.is_some());
        assert_eq!(manager.terminals.len(), 1);

        manager.detach_sender().await;
        assert!(route.read().await.is_none());
        assert_eq!(manager.terminals.len(), 1);
    }

    #[tokio::test]
    async fn terminal_replay_is_bounded_by_bytes() {
        let replay = Arc::new(RwLock::new(VecDeque::new()));
        let terminal_id = TerminalId::from("terminal-replay");
        for seq in 1..=4 {
            record_terminal_replay(
                &replay,
                WorkspaceTerminalOutputFrame {
                    terminal_id: terminal_id.clone(),
                    seq,
                    data: "x".repeat(MAX_WORKSPACE_TERMINAL_REPLAY_BYTES / 2),
                    sent_at: Utc::now(),
                },
            )
            .await;
        }

        let replay = replay.read().await;
        assert!(terminal_replay_bytes(&replay) <= MAX_WORKSPACE_TERMINAL_REPLAY_BYTES);
        assert!(replay.front().is_some_and(|frame| frame.seq > 1));
    }

    #[tokio::test]
    async fn terminal_manager_shutdown_sends_close_and_joins_tasks() {
        let (control_tx, mut control_rx) = mpsc::unbounded_channel();
        let (closed_tx, closed_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            while let Some(control) = control_rx.recv().await {
                if matches!(control, WorkspaceTerminalControl::Close) {
                    let _ = closed_tx.send(());
                    break;
                }
            }
        });
        let route = Arc::new(RwLock::new(None));
        let mut manager = WorkspaceTerminalManager::default();
        manager.terminals.insert(
            "terminal-shutdown".to_owned(),
            WorkspaceTerminalHandle {
                replay: Arc::new(RwLock::new(VecDeque::new())),
                control_tx,
                sender_route: route,
                task,
            },
        );

        manager.shutdown().await;

        closed_rx.await.expect("terminal task observed close");
        assert!(manager.terminals.is_empty());
    }

    #[test]
    fn command_execution_key_prefers_runtime_then_placement() {
        let runtime_command = command_fixture("command-runtime", CommandKind::SendTurn);
        assert_eq!(command_execution_key(&runtime_command), "runtime:runtime-1");

        let mut placement_command =
            command_fixture("command-placement", CommandKind::RunWorkspaceCommand);
        placement_command.target = CommandTarget::Placement {
            node_id: NodeId::from("node-1"),
            project_placement_id: ProjectPlacementId::from("placement-1"),
        };
        assert_eq!(
            command_execution_key(&placement_command),
            "placement:placement-1"
        );

        let mut command = command_fixture("command-standalone", CommandKind::ValidateWorkspace);
        command.target = CommandTarget::Node {
            node_id: NodeId::from("node-1"),
        };
        assert_eq!(
            command_execution_key(&command),
            "command:command-standalone"
        );
    }

    #[tokio::test]
    async fn command_dispatch_rejects_payload_kind_mismatch_before_execution() {
        let config = config_fixture();
        let mut command = command_fixture("command-mismatch", CommandKind::SendTurn);
        command.payload = CommandPayload::StopRuntime;
        let mut local_state = NodeLocalState::default();

        let outcome = prepare_command_dispatch(&config, &mut local_state, &command).await;

        assert_eq!(outcome.status, CommandState::Failed);
        assert_eq!(
            outcome.result_payload.0["error_code"],
            "protocol.command_payload_mismatch"
        );
        assert!(!outcome.state_changed);
    }

    #[tokio::test]
    async fn dispatch_busy_result_is_retryable_failed_command_result() {
        let (sender, mut receiver) = mpsc::channel(4);
        let command = command_fixture("command-busy", CommandKind::SendTurn);

        send_dispatch_busy_result(&sender, &command)
            .await
            .expect("busy result sends");

        let ack = receiver.recv().await.expect("ack frame");
        let ControlFrame::CommandAck {
            command_id, status, ..
        } = ack
        else {
            panic!("expected command ack");
        };
        assert_eq!(command_id, command.command_id);
        assert_eq!(status, CommandState::Acknowledged);

        let result = receiver.recv().await.expect("result frame");
        let ControlFrame::CommandResult {
            command_id,
            status,
            payload,
            ..
        } = result
        else {
            panic!("expected command result");
        };
        assert_eq!(command_id, command.command_id);
        assert_eq!(status, CommandState::Failed);
        assert_eq!(
            payload
                .0
                .get("error_code")
                .and_then(serde_json::Value::as_str),
            Some("node.dispatch_busy")
        );
        assert_eq!(
            payload
                .0
                .get("retryable")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[tokio::test]
    async fn send_frame_reports_saturated_writer_queue() {
        let (sender, _receiver) = mpsc::channel(1);
        send_frame(
            &sender,
            ControlFrame::Pong {
                frame_id: "frame-1".to_owned(),
                protocol_version: API_VERSION.to_owned(),
                sent_at: Utc::now(),
            },
        )
        .await
        .expect("first frame fits queue");

        let error = send_frame(
            &sender,
            ControlFrame::Pong {
                frame_id: "frame-2".to_owned(),
                protocol_version: API_VERSION.to_owned(),
                sent_at: Utc::now(),
            },
        )
        .await
        .expect_err("second frame reports saturation");
        assert!(error.to_string().contains("control frame send failed"));
    }

    #[test]
    fn append_capped_process_line_bounds_output_bytes() {
        let mut output = Vec::new();
        let mut truncated = false;

        append_capped_process_line(&mut output, "abcdef", 5, &mut truncated);

        assert_eq!(output, b"abcde");
        assert!(truncated);

        append_capped_process_line(&mut output, "ignored", 5, &mut truncated);

        assert_eq!(output, b"abcde");
    }

    #[cfg(unix)]
    #[test]
    fn codex_process_limit_event_records_output_truncation() {
        use std::os::unix::process::ExitStatusExt;

        let command = command_fixture("command-limits", CommandKind::SendTurn);
        let output = CodexProcessOutput {
            status: ExitStatus::from_raw(0),
            stdout: b"partial".to_vec(),
            stderr: Vec::new(),
            stdout_truncated: true,
            stderr_truncated: false,
            dropped_activity_count: 7,
            approval_requests: vec![],
            provider_resume_ref: None,
            activity_events: vec![],
        };
        let mut runtime_seqs = HashMap::new();
        let mut events = Vec::new();

        append_codex_process_limit_events(
            "codex",
            &command,
            &mut runtime_seqs,
            RuntimeSessionId::from("runtime-1"),
            Some(TurnId::from("turn-1")),
            &output,
            &mut events,
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::ProviderActivity);
        assert_eq!(
            events[0]
                .payload
                .0
                .get("provider_event_type")
                .and_then(serde_json::Value::as_str),
            Some("output_truncated")
        );
        assert_eq!(
            events[0]
                .payload
                .0
                .get("dropped_activity_count")
                .and_then(serde_json::Value::as_i64),
            Some(7)
        );
    }

    #[cfg(unix)]
    #[test]
    fn codex_resume_ref_uses_incremental_parse_result() {
        use std::os::unix::process::ExitStatusExt;

        let output = CodexProcessOutput {
            status: ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_truncated: true,
            stderr_truncated: false,
            dropped_activity_count: 0,
            approval_requests: vec![],
            provider_resume_ref: Some(serde_json::json!({
                "provider_session_id": "session-incremental",
            })),
            activity_events: vec![],
        };

        assert_eq!(
            codex_resume_ref_from_output(&output)
                .and_then(|value| value.get("provider_session_id").cloned())
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("session-incremental".to_owned())
        );
    }

    #[test]
    fn command_state_merge_preserves_registration_and_replaces_runtime_fields() {
        let mut shared = NodeLocalState {
            node_id: Some(NodeId::from("node-owner")),
            credential: Some("credential-owner".to_owned()),
            ..NodeLocalState::default()
        };
        let command_state = NodeLocalState {
            node_id: Some(NodeId::from("stale-command-copy")),
            credential: Some("stale-credential".to_owned()),
            runtime_seqs: HashMap::from([("runtime-1".to_owned(), 4)]),
            ..NodeLocalState::default()
        };

        let baseline = shared.clone();
        shared.merge_command_state_from(&baseline, &command_state);

        assert_eq!(shared.node_id, Some(NodeId::from("node-owner")));
        assert_eq!(shared.credential.as_deref(), Some("credential-owner"));
        assert_eq!(shared.runtime_seqs.get("runtime-1"), Some(&4));
    }

    #[tokio::test]
    async fn state_store_serializes_command_merges_and_preserves_registration() {
        let path =
            std::env::temp_dir().join(format!("uprava-node-store-{}.sqlite", Uuid::new_v4()));
        let mut initial = NodeLocalState {
            node_id: Some(NodeId::from("node-owner")),
            credential: Some("credential-owner".to_owned()),
            ..NodeLocalState::default()
        };
        let event = runtime_outbox_retention_event(
            &mut initial,
            RuntimeSessionId::from("runtime-store"),
            None,
            None,
        );
        let event_id = event.event_id.clone();
        initial.event_outbox.push(event);
        initial
            .save_async(&path)
            .await
            .expect("initial state persists");
        let store = NodeStateStore::new(initial, path.clone());
        let baseline = store.snapshot().await.expect("state snapshot");
        let first = NodeLocalState {
            node_id: Some(NodeId::from("stale-first-copy")),
            command_status: HashMap::from([("command-1".to_owned(), CommandState::Completed)]),
            event_outbox: baseline.event_outbox.clone(),
            ..NodeLocalState::default()
        };
        let second = NodeLocalState {
            node_id: Some(NodeId::from("stale-second-copy")),
            command_status: HashMap::from([("command-2".to_owned(), CommandState::Failed)]),
            event_outbox: baseline.event_outbox.clone(),
            ..NodeLocalState::default()
        };

        let (first_result, second_result) = tokio::join!(
            store.merge_command_state(&baseline, &first),
            store.merge_command_state(&baseline, &second)
        );
        first_result.expect("first command merge persists");
        second_result.expect("second command merge persists");

        let merged = store.snapshot().await.expect("state snapshot");
        assert_eq!(merged.node_id, Some(NodeId::from("node-owner")));
        assert_eq!(merged.credential.as_deref(), Some("credential-owner"));
        assert!(merged.command_status.contains_key("command-1"));
        assert!(merged.command_status.contains_key("command-2"));
        assert!(merged
            .event_outbox
            .iter()
            .any(|event| event.event_id == event_id));

        let stale = baseline.clone();
        let (ack_result, merge_result) = tokio::join!(
            store.persist_event_ack(std::slice::from_ref(&event_id)),
            store.merge_command_state(&baseline, &stale)
        );
        ack_result.expect("event ACK persists");
        merge_result.expect("stale command merge persists");
        let merged = store.snapshot().await.expect("state snapshot");
        assert!(merged
            .event_outbox
            .iter()
            .all(|event| event.event_id != event_id));
        assert!(merged.command_status.contains_key("command-1"));
        let reopened = NodeLocalState::load_async(&path)
            .await
            .expect("sqlite state reopens");
        assert!(reopened
            .event_outbox
            .iter()
            .all(|event| event.event_id != event_id));
        let pool = open_state_store(&path).await.expect("sqlite store opens");
        let outbox_rows: i64 = sqlx::query_scalar("select count(*) from node_event_outbox")
            .fetch_one(&pool)
            .await
            .expect("outbox rows query");
        pool.close().await;
        assert_eq!(outbox_rows, 0);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn state_store_persists_command_outcome_before_result_delivery() {
        let path = std::env::temp_dir().join(format!(
            "uprava-node-command-outcome-{}.sqlite",
            Uuid::new_v4()
        ));
        let baseline = NodeLocalState {
            node_id: Some(NodeId::from("node-owner")),
            credential: Some("credential-owner".to_owned()),
            ..NodeLocalState::default()
        };
        baseline
            .clone()
            .save_async(&path)
            .await
            .expect("baseline state persists");
        let store = NodeStateStore::new(baseline.clone(), path.clone());
        let mut command_state = baseline.clone();
        let command_id = "command-outcome";
        command_state
            .command_status
            .insert(command_id.to_owned(), CommandState::Completed);
        command_state.command_result_payloads.insert(
            command_id.to_owned(),
            JsonValue(serde_json::json!({"ok": true})),
        );
        let event = runtime_outbox_retention_event(
            &mut command_state,
            RuntimeSessionId::from("runtime-command-outcome"),
            None,
            None,
        );
        let event_id = event.event_id.clone();
        command_state.event_outbox.push(event);

        store
            .persist_command_outcome(&baseline, &command_state)
            .await
            .expect("command outcome persists");

        let reloaded = NodeLocalState::load_async(&path)
            .await
            .expect("persisted outcome reloads");
        assert_eq!(
            reloaded.command_status.get(command_id),
            Some(&CommandState::Completed)
        );
        assert_eq!(
            reloaded.command_result_payloads.get(command_id),
            Some(&JsonValue(serde_json::json!({"ok": true})))
        );
        assert!(reloaded
            .event_outbox
            .iter()
            .any(|event| event.event_id == event_id));
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn state_store_command_merge_propagates_removals_without_clobbering_newer_values() {
        let baseline = NodeLocalState {
            runtime_provider_resume_refs: HashMap::from([(
                "runtime-remove".to_owned(),
                ProviderResumeRef {
                    provider_session_id: Some("session-old".to_owned()),
                    resume_cursor: Some("cursor-old".to_owned()),
                },
            )]),
            ..NodeLocalState::default()
        };
        let command_state = NodeLocalState {
            // The command snapshot explicitly removed the resume reference.
            ..baseline.clone()
        };
        let mut command_state = command_state;
        command_state.runtime_provider_resume_refs.clear();

        let removal_path = std::env::temp_dir().join(format!(
            "uprava-node-store-removal-{}.sqlite",
            Uuid::new_v4()
        ));
        baseline
            .clone()
            .save_async(&removal_path)
            .await
            .expect("baseline removal state persists");
        let removal_store = NodeStateStore::new(baseline.clone(), removal_path.clone());
        removal_store
            .merge_command_state(&baseline, &command_state)
            .await
            .expect("removal merge persists");
        assert!(!removal_store
            .snapshot()
            .await
            .expect("state snapshot")
            .runtime_provider_resume_refs
            .contains_key("runtime-remove"));
        let removal_reopened = NodeLocalState::load_async(&removal_path)
            .await
            .expect("removal state reopens");
        assert!(!removal_reopened
            .runtime_provider_resume_refs
            .contains_key("runtime-remove"));

        let newer_path = std::env::temp_dir().join(format!(
            "uprava-node-store-newer-ref-{}.sqlite",
            Uuid::new_v4()
        ));
        let newer_ref = ProviderResumeRef {
            provider_session_id: Some("session-new".to_owned()),
            resume_cursor: Some("cursor-new".to_owned()),
        };
        let newer_owner = NodeLocalState {
            runtime_provider_resume_refs: HashMap::from([(
                "runtime-remove".to_owned(),
                newer_ref.clone(),
            )]),
            ..NodeLocalState::default()
        };
        newer_owner
            .clone()
            .save_async(&newer_path)
            .await
            .expect("newer owner state persists");
        let newer_store = NodeStateStore::new(newer_owner, newer_path.clone());
        newer_store
            .merge_command_state(&baseline, &command_state)
            .await
            .expect("stale removal merge persists");
        assert_eq!(
            newer_store
                .snapshot()
                .await
                .expect("state snapshot")
                .runtime_provider_resume_refs
                .get("runtime-remove"),
            Some(&newer_ref)
        );
        let newer_reopened = NodeLocalState::load_async(&newer_path)
            .await
            .expect("newer state reopens");
        assert_eq!(
            newer_reopened
                .runtime_provider_resume_refs
                .get("runtime-remove"),
            Some(&newer_ref)
        );

        let _ = std::fs::remove_file(removal_path);
        let _ = std::fs::remove_file(newer_path);
    }

    #[test]
    fn live_event_sink_only_records_until_durable_dispatch_phase() {
        let mut local_state = NodeLocalState::default();
        let event = runtime_outbox_retention_event(
            &mut local_state,
            RuntimeSessionId::from("runtime-durable-live-event"),
            None,
            None,
        );
        let mut runtime_states = HashMap::new();
        let mut sink = NodeLiveEventSink::new(&mut runtime_states);

        sink.emit(&event);

        assert_eq!(
            runtime_states.get("runtime-durable-live-event"),
            Some(&RuntimeSessionState::Error)
        );
    }
}
