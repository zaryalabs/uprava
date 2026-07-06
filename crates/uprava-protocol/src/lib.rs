//! Shared Uprava protocol and domain contracts for the V01 control plane.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            #[must_use]
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

id_type!(NodeId);
id_type!(EnrollmentId);
id_type!(ProjectId);
id_type!(ProjectPlacementId);
id_type!(ActorId);
id_type!(SessionThreadId);
id_type!(RuntimeSessionId);
id_type!(TurnId);
id_type!(MessageId);
id_type!(CommandId);
id_type!(TerminalId);
id_type!(EventId);
id_type!(ApprovalId);
id_type!(ArtifactId);
id_type!(BlockId);
id_type!(CorrelationId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentProfile {
    ControlledDev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    Hardened,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorRef {
    LocalUser { actor_id: Option<ActorId> },
    System,
    Node { node_id: NodeId },
    Provider { provider: String },
    Unknown,
}

impl ActorRef {
    #[must_use]
    pub fn local_user() -> Self {
        Self::LocalUser { actor_id: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScopeRef {
    Runtime {
        runtime_session_id: RuntimeSessionId,
    },
    Session {
        session_thread_id: SessionThreadId,
    },
    Node {
        node_id: NodeId,
    },
    Placement {
        project_placement_id: ProjectPlacementId,
    },
    Unknown {
        scope: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentState {
    Unregistered,
    PendingUserApproval,
    Approved,
    Registered,
    Expired,
    Rejected,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodePresence {
    Reachable,
    Stale,
    Offline,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SleepHint {
    Unknown,
    Awake,
    Suspending,
    Sleeping,
    Woke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlChannelState {
    Closed,
    Requested,
    Connecting,
    Open,
    Closing,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementState {
    Pending,
    Validated,
    Missing,
    ReadOnly,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionThreadState {
    Created,
    Active,
    Detached,
    Stopped,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSessionState {
    Starting,
    Ready,
    Running,
    Blocked,
    Stopping,
    Stopped,
    Interrupted,
    Resuming,
    Stale,
    Error,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Created,
    Dispatched,
    Running,
    BlockedOnApproval,
    Completed,
    Interrupted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Recorded,
    PendingDispatch,
    Dispatched,
    Acknowledged,
    Completed,
    Failed,
    Blocked,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Requested,
    Resolved,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningSeverity {
    Info,
    Warning,
    HardBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CommandKind {
    StartRuntime,
    ResumeRuntime,
    SendTurn,
    ResolveApproval,
    InterruptRuntime,
    StopRuntime,
    ValidateWorkspace,
    RefreshResourceSnapshot,
    ListWorkspaceTree,
    ReadWorkspaceFile,
    WriteWorkspaceFile,
    RunWorkspaceCommand,
    ReadWorkspaceDiff,
    OpenWorkspaceTerminal,
    AttachWorkspaceTerminal,
    ResizeWorkspaceTerminal,
    WriteWorkspaceTerminal,
    CloseWorkspaceTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    #[serde(rename = "runtime.starting")]
    RuntimeStarting,
    #[serde(rename = "runtime.ready")]
    RuntimeReady,
    #[serde(rename = "runtime.running")]
    RuntimeRunning,
    #[serde(rename = "runtime.blocked")]
    RuntimeBlocked,
    #[serde(rename = "runtime.expired")]
    RuntimeExpired,
    #[serde(rename = "runtime.resuming")]
    RuntimeResuming,
    #[serde(rename = "runtime.stopped")]
    RuntimeStopped,
    #[serde(rename = "runtime.error")]
    RuntimeError,
    #[serde(rename = "turn.started")]
    TurnStarted,
    #[serde(rename = "turn.completed")]
    TurnCompleted,
    #[serde(rename = "turn.interrupted")]
    TurnInterrupted,
    #[serde(rename = "provider.activity")]
    ProviderActivity,
    #[serde(rename = "provider.output.delta")]
    ProviderOutputDelta,
    #[serde(rename = "provider.message.completed")]
    ProviderMessageCompleted,
    #[serde(rename = "approval.requested")]
    ApprovalRequested,
    #[serde(rename = "approval.resolved")]
    ApprovalResolved,
    #[serde(rename = "coordination.warning_acknowledged")]
    CoordinationWarningAcknowledged,
    #[serde(rename = "workspace.validated")]
    WorkspaceValidated,
    #[serde(rename = "resource.snapshot.updated")]
    ResourceSnapshotUpdated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub target_node_id: NodeId,
    pub actor_ref: ActorRef,
    pub session_thread_id: Option<SessionThreadId>,
    pub runtime_session_id: Option<RuntimeSessionId>,
    pub project_placement_id: Option<ProjectPlacementId>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    pub issued_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
    pub payload: serde_json_value::JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: EventId,
    pub command_id: Option<CommandId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<CorrelationId>,
    pub actor_ref: ActorRef,
    pub scope_ref: ScopeRef,
    pub node_id: Option<NodeId>,
    pub runtime_session_id: Option<RuntimeSessionId>,
    pub session_thread_id: Option<SessionThreadId>,
    pub turn_id: Option<TurnId>,
    pub seq: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_projection_seq: Option<i64>,
    pub kind: EventKind,
    pub happened_at: DateTime<Utc>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub result_refs: Vec<UpravaRef>,
    pub payload: serde_json_value::JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlFrame {
    Hello {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        node_id: NodeId,
        daemon_version: String,
        active_runtime_ids: Vec<RuntimeSessionId>,
    },
    HelloAck {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
    },
    CommandDispatch {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        command: CommandEnvelope,
    },
    CommandAck {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        command_id: CommandId,
        status: CommandState,
    },
    CommandResult {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        command_id: CommandId,
        status: CommandState,
        payload: serde_json_value::JsonValue,
    },
    EventBatch {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        events: Vec<EventEnvelope>,
    },
    EventBatchAck {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        accepted_event_ids: Vec<EventId>,
    },
    WorkspaceTerminalAttach {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
    },
    WorkspaceTerminalInput {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
        data: String,
    },
    WorkspaceTerminalResize {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
        cols: u16,
        rows: u16,
    },
    WorkspaceTerminalClose {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
    },
    WorkspaceTerminalOutput {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
        seq: u64,
        data: String,
    },
    WorkspaceTerminalStatus {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        terminal_id: TerminalId,
        state: WorkspaceTerminalState,
        exit_code: Option<i32>,
        message: Option<String>,
    },
    Ping {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
    },
    Pong {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
    },
    ControlError {
        frame_id: String,
        protocol_version: String,
        sent_at: DateTime<Utc>,
        error: ApiError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    #[serde(default)]
    pub details: serde_json_value::JsonValue,
    pub retryable: bool,
    pub correlation_id: CorrelationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub profile: DeploymentProfile,
    pub security: SecurityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionResponse {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub schema_version: i64,
    pub profile: DeploymentProfile,
    pub security: SecurityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityStatus {
    pub mode: SecurityMode,
    pub web_auth_required: bool,
    pub web_auth_configured: bool,
    pub cookie_secure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub nodes: Vec<NodeSummary>,
    pub placements: Vec<ProjectPlacementSummary>,
    pub sessions: Vec<SessionSummary>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSummary {
    pub node_id: NodeId,
    pub display_name: String,
    pub presence: NodePresence,
    pub sleep_hint: SleepHint,
    pub heartbeat_age_seconds: Option<i64>,
    pub active_runtime_count: i64,
    pub capabilities: Vec<CapabilitySummary>,
    pub diagnostics: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub key: String,
    pub value: serde_json_value::JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPlacementSummary {
    pub project_placement_id: ProjectPlacementId,
    pub project_id: Option<ProjectId>,
    pub node_id: NodeId,
    pub display_name: String,
    pub workspace_path: String,
    pub state: PlacementState,
    pub resource_badges: Vec<ResourceBadge>,
    pub last_validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub display_name: String,
    pub workspace_path: String,
    pub state: PlacementState,
    pub resource_badges: Vec<ResourceBadge>,
    pub last_validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceEntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceEntryStatus {
    Readable,
    Directory,
    Large,
    Binary,
    Ignored,
    Generated,
    PermissionDenied,
    OutsideWorkspace,
    Missing,
    NotFile,
    NotDirectory,
    Symlink,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: String,
    pub kind: WorkspaceEntryKind,
    pub status: WorkspaceEntryStatus,
    pub byte_len: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub children: Vec<WorkspaceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTreeResponse {
    pub placement_id: ProjectPlacementId,
    pub root: WorkspaceEntry,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileContentResponse {
    pub placement_id: ProjectPlacementId,
    pub path: String,
    pub metadata: WorkspaceEntry,
    pub content: Option<String>,
    pub truncated: bool,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileWriteRequest {
    pub path: String,
    pub content: String,
    pub expected_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileWriteResponse {
    pub placement_id: ProjectPlacementId,
    pub path: String,
    pub metadata: WorkspaceEntry,
    pub edit_id: String,
    pub written_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCommandIntent {
    Command,
    Check,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCommandRunRequest {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub intent: WorkspaceCommandIntent,
    pub label: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCommandRunResponse {
    pub placement_id: ProjectPlacementId,
    pub terminal_command_id: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub intent: WorkspaceCommandIntent,
    pub label: Option<String>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiffResponse {
    pub placement_id: ProjectPlacementId,
    pub diff_id: String,
    pub summary: String,
    pub diff: String,
    pub summary_truncated: bool,
    pub diff_truncated: bool,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCommandHistoryItem {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub state: CommandState,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub payload: serde_json_value::JsonValue,
    pub result_payload: Option<serde_json_value::JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCommandHistoryResponse {
    pub placement_id: ProjectPlacementId,
    pub commands: Vec<WorkspaceCommandHistoryItem>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTerminalState {
    Opening,
    Running,
    Detached,
    Exited,
    Closed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalOpenRequest {
    pub shell_profile: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalAttachRequest {
    pub terminal_id: TerminalId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalResizeRequest {
    pub terminal_id: TerminalId,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalWriteRequest {
    pub terminal_id: TerminalId,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalCloseRequest {
    pub terminal_id: TerminalId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalSummary {
    pub placement_id: ProjectPlacementId,
    pub terminal_id: TerminalId,
    pub title: String,
    pub cwd: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub state: WorkspaceTerminalState,
    pub exit_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalOutputFrame {
    pub terminal_id: TerminalId,
    pub seq: u64,
    pub data: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalOpenResponse {
    pub placement_id: ProjectPlacementId,
    pub terminal: WorkspaceTerminalSummary,
    #[serde(default)]
    pub replay: Vec<WorkspaceTerminalOutputFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTerminalListResponse {
    pub placement_id: ProjectPlacementId,
    pub terminals: Vec<WorkspaceTerminalSummary>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceTerminalClientFrame {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
    Close,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceTerminalStreamFrame {
    Output {
        terminal_id: TerminalId,
        seq: u64,
        data: String,
        sent_at: DateTime<Utc>,
    },
    Status {
        terminal_id: TerminalId,
        state: WorkspaceTerminalState,
        exit_code: Option<i32>,
        message: Option<String>,
        sent_at: DateTime<Utc>,
    },
    Pong {
        sent_at: DateTime<Utc>,
    },
    Error {
        terminal_id: TerminalId,
        message: String,
        sent_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBadge {
    pub kind: String,
    pub severity: WarningSeverity,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_thread_id: SessionThreadId,
    pub project_placement_id: ProjectPlacementId,
    pub runtime_session_id: RuntimeSessionId,
    pub title: String,
    pub state: SessionThreadState,
    pub runtime: RuntimeSummary,
    pub message_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSummary {
    pub runtime_session_id: RuntimeSessionId,
    pub provider: String,
    pub state: RuntimeSessionState,
    pub resume_supported: bool,
    pub degraded_reason: Option<String>,
    pub last_runtime_step_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub placement: ProjectPlacementSummary,
    pub messages: Vec<Message>,
    pub events: Vec<EventEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Runtime,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub message_id: MessageId,
    pub session_thread_id: SessionThreadId,
    pub turn_id: Option<TurnId>,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub source_event_id: Option<EventId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePlacementRequest {
    pub node_id: NodeId,
    pub display_name: String,
    pub workspace_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub project_placement_id: ProjectPlacementId,
    pub title: Option<String>,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendTurnRequest {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveApprovalRequest {
    pub approved: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeWarningRequest {
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientLogRequest {
    pub level: ClientLogLevel,
    pub source: String,
    pub message: String,
    pub route: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub detail: serde_json_value::JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientLogResponse {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAuthStatusResponse {
    pub auth_required: bool,
    pub setup_required: bool,
    pub authenticated: bool,
    pub profile: DeploymentProfile,
    pub security: SecurityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAuthSetupRequest {
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAuthLoginRequest {
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAuthResponse {
    pub authenticated: bool,
    pub setup_required: bool,
    pub csrf_token: Option<String>,
    pub security: SecurityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAcceptedResponse {
    pub command_id: CommandId,
    pub session: Option<SessionDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningAcknowledgementResponse {
    pub event_id: EventId,
    pub session: SessionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCreateNodeEnrollmentRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentRequest {
    pub display_name: String,
    pub daemon_version: String,
    pub capabilities: Vec<CapabilitySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentRequestedResponse {
    pub enrollment_id: EnrollmentId,
    pub pairing_code: String,
    pub status: EnrollmentState,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentSummary {
    pub enrollment_id: EnrollmentId,
    pub display_name: String,
    pub status: EnrollmentState,
    pub claimed_node_id: Option<NodeId>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveNodeEnrollmentResponse {
    pub enrollment: NodeEnrollmentSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentClaimRequest {
    pub enrollment_id: EnrollmentId,
    pub pairing_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentClaimResponse {
    pub accepted: bool,
    pub pending: bool,
    pub node_id: Option<NodeId>,
    pub credential: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRevocationResponse {
    pub node_id: NodeId,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCredentialRotationResponse {
    pub node_id: NodeId,
    pub credential: String,
    pub rotated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDeletionResponse {
    pub node_id: NodeId,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementDeletionResponse {
    pub project_placement_id: ProjectPlacementId,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeHeartbeatRequest {
    pub node_id: Option<NodeId>,
    pub display_name: String,
    pub daemon_version: String,
    pub capabilities: Vec<CapabilitySummary>,
    #[serde(default)]
    pub diagnostics: Option<String>,
    pub active_runtime_count: i64,
    pub sleep_hint: SleepHint,
    #[serde(default)]
    pub workspace_summaries: Vec<WorkspaceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeHeartbeatResponse {
    pub accepted: bool,
    pub node_id: NodeId,
    pub open_control_channel: bool,
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpravaRef {
    Node {
        node_id: NodeId,
    },
    Project {
        project_id: ProjectId,
    },
    Placement {
        placement_id: ProjectPlacementId,
    },
    Workspace {
        placement_id: ProjectPlacementId,
    },
    Session {
        session_thread_id: SessionThreadId,
    },
    Runtime {
        runtime_session_id: RuntimeSessionId,
    },
    Turn {
        turn_id: TurnId,
    },
    Message {
        message_id: MessageId,
    },
    Block {
        block_id: BlockId,
    },
    Artifact {
        artifact_id: ArtifactId,
    },
    Event {
        event_id: EventId,
        scope_ref: Box<ScopeRef>,
        seq: i64,
    },
    Command {
        command_id: CommandId,
    },
    Approval {
        approval_id: ApprovalId,
    },
    Warning {
        warning_kind: String,
        command_id: Option<CommandId>,
    },
    ToolCall {
        tool_call_id: String,
    },
    File {
        placement_id: ProjectPlacementId,
        path: String,
        version: Option<String>,
    },
    FileRange {
        placement_id: ProjectPlacementId,
        path: String,
        range: TextRange,
        version: Option<String>,
    },
    Terminal {
        terminal_id: String,
        placement_id: ProjectPlacementId,
    },
    TerminalCommand {
        terminal_command_id: String,
        terminal_id: Option<String>,
    },
    TerminalOutputRange {
        terminal_command_id: String,
        range: TextRange,
    },
    DiffHunk {
        diff_id: String,
        hunk_id: String,
    },
    CheckResult {
        check_run_id: String,
        failure_id: Option<String>,
    },
    WorkspaceEdit {
        edit_id: String,
        placement_id: Option<ProjectPlacementId>,
        path: Option<String>,
    },
    TraceEvent {
        trace_event_id: String,
    },
    ExternalEntity {
        integration_kind: String,
        external_id: String,
    },
    Unknown {
        ref_type: String,
        locator: serde_json_value::JsonValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBlock {
    pub block_id: BlockId,
    #[serde(rename = "type")]
    pub block_type: String,
    pub schema_version: i64,
    pub surface_id: String,
    pub primary_ref: UpravaRef,
    pub parent_ref: Option<UpravaRef>,
    #[serde(default)]
    pub children: Vec<UiBlock>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub related_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub trace_refs: Vec<UpravaRef>,
    pub data: serde_json_value::JsonValue,
    #[serde(default)]
    pub actions: Vec<String>,
    pub fallback_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTreeNode {
    pub artifact_id: ArtifactId,
    pub label: String,
    pub primary_ref: UpravaRef,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub children: Vec<ArtifactTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTree {
    pub session_thread_id: SessionThreadId,
    pub root: ArtifactTreeNode,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProjection {
    pub session_thread_id: SessionThreadId,
    pub project_placement: ProjectPlacementSummary,
    pub runtime_summary: RuntimeSummary,
    pub current_turn: Option<TurnId>,
    #[serde(default)]
    pub pending_approvals: Vec<ApprovalId>,
    #[serde(default)]
    pub active_warnings: Vec<ResourceBadge>,
    #[serde(default)]
    pub recent_turn_summaries: Vec<String>,
    #[serde(default)]
    pub recent_message_refs: Vec<UpravaRef>,
    pub artifact_tree_summary: String,
    #[serde(default)]
    pub available_block_types: Vec<String>,
    #[serde(default)]
    pub available_commands: Vec<String>,
    #[serde(default)]
    pub visible_refs: Vec<UpravaRef>,
    pub source_cause_summary: String,
    pub resume_context: String,
    pub generated_at: DateTime<Utc>,
}

pub mod serde_json_value {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct JsonValue(pub serde_json::Value);

    impl Default for JsonValue {
        fn default() -> Self {
            Self(serde_json::Value::Object(serde_json::Map::new()))
        }
    }

    impl From<serde_json::Value> for JsonValue {
        fn from(value: serde_json::Value) -> Self {
            Self(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn json_payload() -> serde_json_value::JsonValue {
        serde_json_value::JsonValue(serde_json::json!({ "sample": true }))
    }

    #[test]
    fn command_envelope_round_trips_through_json() {
        let command = CommandEnvelope {
            command_id: CommandId::from("command-1"),
            kind: CommandKind::SendTurn,
            target_node_id: NodeId::from("node-1"),
            actor_ref: ActorRef::local_user(),
            session_thread_id: Some(SessionThreadId::from("session-1")),
            runtime_session_id: Some(RuntimeSessionId::from("runtime-1")),
            project_placement_id: Some(ProjectPlacementId::from("placement-1")),
            source_refs: vec![],
            cause_refs: vec![UpravaRef::Session {
                session_thread_id: SessionThreadId::from("session-1"),
            }],
            issued_at: Utc::now(),
            correlation_id: CorrelationId::from("corr-1"),
            payload: json_payload(),
        };

        let encoded = serde_json::to_string(&command).expect("command serializes");
        let decoded: CommandEnvelope =
            serde_json::from_str(&encoded).expect("command deserializes");

        assert_eq!(decoded.kind, CommandKind::SendTurn);
    }

    #[test]
    fn event_envelope_round_trips_through_json() {
        let event = EventEnvelope {
            event_id: EventId::from("event-1"),
            command_id: Some(CommandId::from("command-1")),
            correlation_id: Some(CorrelationId::from("corr-1")),
            actor_ref: ActorRef::Provider {
                provider: "codex".to_owned(),
            },
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: RuntimeSessionId::from("runtime-1"),
            },
            node_id: Some(NodeId::from("node-1")),
            runtime_session_id: Some(RuntimeSessionId::from("runtime-1")),
            session_thread_id: Some(SessionThreadId::from("session-1")),
            turn_id: Some(TurnId::from("turn-1")),
            seq: 1,
            session_projection_seq: Some(1),
            kind: EventKind::ProviderMessageCompleted,
            happened_at: Utc::now(),
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: json_payload(),
        };

        let encoded = serde_json::to_string(&event).expect("event serializes");
        let decoded: EventEnvelope = serde_json::from_str(&encoded).expect("event deserializes");

        assert_eq!(decoded.seq, 1);
        assert_eq!(decoded.correlation_id, Some(CorrelationId::from("corr-1")));
    }

    #[test]
    fn event_envelope_defaults_missing_correlation_id() {
        let encoded = serde_json::json!({
            "event_id": "event-1",
            "command_id": "command-1",
            "actor_ref": { "kind": "system" },
            "scope_ref": { "kind": "unknown", "scope": "test" },
            "node_id": null,
            "runtime_session_id": null,
            "session_thread_id": null,
            "turn_id": null,
            "seq": 1,
            "kind": "runtime.ready",
            "happened_at": Utc::now(),
            "payload": { "sample": true }
        });

        let decoded: EventEnvelope =
            serde_json::from_value(encoded).expect("legacy event deserializes");

        assert_eq!(decoded.correlation_id, None);
    }

    #[test]
    fn event_envelope_preserves_actor_scope_and_causality_refs() {
        let event = EventEnvelope {
            event_id: EventId::from("event-causality-1"),
            command_id: Some(CommandId::from("command-1")),
            correlation_id: Some(CorrelationId::from("corr-2")),
            actor_ref: ActorRef::Node {
                node_id: NodeId::from("node-1"),
            },
            scope_ref: ScopeRef::Runtime {
                runtime_session_id: RuntimeSessionId::from("runtime-1"),
            },
            node_id: Some(NodeId::from("node-1")),
            runtime_session_id: Some(RuntimeSessionId::from("runtime-1")),
            session_thread_id: Some(SessionThreadId::from("session-1")),
            turn_id: Some(TurnId::from("turn-1")),
            seq: 2,
            session_projection_seq: Some(2),
            kind: EventKind::ProviderOutputDelta,
            happened_at: Utc::now(),
            source_refs: vec![UpravaRef::Command {
                command_id: CommandId::from("command-1"),
            }],
            evidence_refs: vec![UpravaRef::FileRange {
                placement_id: ProjectPlacementId::from("placement-1"),
                path: "src/main.rs".to_owned(),
                range: TextRange {
                    start_line: Some(10),
                    end_line: Some(12),
                    start_offset: None,
                    end_offset: None,
                },
                version: Some("git:abc123".to_owned()),
            }],
            cause_refs: vec![UpravaRef::Approval {
                approval_id: ApprovalId::from("approval-1"),
            }],
            result_refs: vec![UpravaRef::Message {
                message_id: MessageId::from("message-1"),
            }],
            payload: json_payload(),
        };

        let encoded = serde_json::to_string(&event).expect("event serializes");
        let decoded: EventEnvelope = serde_json::from_str(&encoded).expect("event deserializes");

        assert_eq!(decoded.actor_ref, event.actor_ref);
        assert_eq!(decoded.scope_ref, event.scope_ref);
        assert_eq!(decoded.source_refs, event.source_refs);
        assert_eq!(decoded.evidence_refs, event.evidence_refs);
        assert_eq!(decoded.cause_refs, event.cause_refs);
        assert_eq!(decoded.result_refs, event.result_refs);
    }

    #[test]
    fn control_frame_round_trips_through_json() {
        let frame = ControlFrame::Ping {
            frame_id: "frame-1".to_owned(),
            protocol_version: "v1".to_owned(),
            sent_at: Utc::now(),
        };

        let encoded = serde_json::to_string(&frame).expect("frame serializes");
        let decoded: ControlFrame = serde_json::from_str(&encoded).expect("frame deserializes");

        assert!(matches!(decoded, ControlFrame::Ping { .. }));
    }

    #[test]
    fn error_envelope_round_trips_through_json() {
        let error = ApiError {
            error_code: "validation.invalid".to_owned(),
            message: "Invalid request".to_owned(),
            details: json_payload(),
            retryable: false,
            correlation_id: CorrelationId::from("corr-1"),
        };

        let encoded = serde_json::to_string(&error).expect("error serializes");
        let decoded: ApiError = serde_json::from_str(&encoded).expect("error deserializes");

        assert_eq!(decoded.error_code, "validation.invalid");
    }

    #[test]
    fn ui_block_reserved_refs_round_trip_through_json() {
        let block = UiBlock {
            block_id: BlockId::from("block-1"),
            block_type: "core.unknown".to_owned(),
            schema_version: 1,
            surface_id: "session.timeline".to_owned(),
            primary_ref: UpravaRef::TerminalCommand {
                terminal_command_id: "terminal-command-1".to_owned(),
                terminal_id: None,
            },
            parent_ref: None,
            children: vec![],
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            related_refs: vec![],
            trace_refs: vec![],
            data: json_payload(),
            actions: vec!["reference.copy".to_owned()],
            fallback_text: Some("Reserved reference unavailable in V01".to_owned()),
        };

        let encoded = serde_json::to_string(&block).expect("block serializes");
        let decoded: UiBlock = serde_json::from_str(&encoded).expect("block deserializes");

        assert_eq!(decoded.block_type, "core.unknown");
    }

    #[test]
    fn uprava_ref_variants_round_trip_through_json() {
        let refs = vec![
            UpravaRef::Node {
                node_id: NodeId::from("node-1"),
            },
            UpravaRef::Project {
                project_id: ProjectId::from("project-1"),
            },
            UpravaRef::Placement {
                placement_id: ProjectPlacementId::from("placement-1"),
            },
            UpravaRef::Workspace {
                placement_id: ProjectPlacementId::from("placement-1"),
            },
            UpravaRef::Session {
                session_thread_id: SessionThreadId::from("session-1"),
            },
            UpravaRef::Runtime {
                runtime_session_id: RuntimeSessionId::from("runtime-1"),
            },
            UpravaRef::Turn {
                turn_id: TurnId::from("turn-1"),
            },
            UpravaRef::Message {
                message_id: MessageId::from("message-1"),
            },
            UpravaRef::Block {
                block_id: BlockId::from("block-1"),
            },
            UpravaRef::Artifact {
                artifact_id: ArtifactId::from("artifact-1"),
            },
            UpravaRef::Event {
                event_id: EventId::from("event-1"),
                scope_ref: Box::new(ScopeRef::Session {
                    session_thread_id: SessionThreadId::from("session-1"),
                }),
                seq: 1,
            },
            UpravaRef::Command {
                command_id: CommandId::from("command-1"),
            },
            UpravaRef::Approval {
                approval_id: ApprovalId::from("approval-1"),
            },
            UpravaRef::Warning {
                warning_kind: "node_offline".to_owned(),
                command_id: Some(CommandId::from("command-1")),
            },
            UpravaRef::ToolCall {
                tool_call_id: "tool-call-1".to_owned(),
            },
            UpravaRef::File {
                placement_id: ProjectPlacementId::from("placement-1"),
                path: "src/main.rs".to_owned(),
                version: Some("git:abc123".to_owned()),
            },
            UpravaRef::FileRange {
                placement_id: ProjectPlacementId::from("placement-1"),
                path: "src/main.rs".to_owned(),
                range: TextRange {
                    start_line: Some(1),
                    end_line: Some(3),
                    start_offset: None,
                    end_offset: None,
                },
                version: None,
            },
            UpravaRef::Terminal {
                terminal_id: "terminal-1".to_owned(),
                placement_id: ProjectPlacementId::from("placement-1"),
            },
            UpravaRef::TerminalCommand {
                terminal_command_id: "terminal-command-1".to_owned(),
                terminal_id: Some("terminal-1".to_owned()),
            },
            UpravaRef::TerminalOutputRange {
                terminal_command_id: "terminal-command-1".to_owned(),
                range: TextRange {
                    start_line: Some(5),
                    end_line: Some(7),
                    start_offset: None,
                    end_offset: None,
                },
            },
            UpravaRef::DiffHunk {
                diff_id: "diff-1".to_owned(),
                hunk_id: "hunk-1".to_owned(),
            },
            UpravaRef::CheckResult {
                check_run_id: "check-1".to_owned(),
                failure_id: Some("failure-1".to_owned()),
            },
            UpravaRef::WorkspaceEdit {
                edit_id: "edit-1".to_owned(),
                placement_id: Some(ProjectPlacementId::from("placement-1")),
                path: Some("src/main.rs".to_owned()),
            },
            UpravaRef::TraceEvent {
                trace_event_id: "trace-event-1".to_owned(),
            },
            UpravaRef::ExternalEntity {
                integration_kind: "github".to_owned(),
                external_id: "pull-1".to_owned(),
            },
            UpravaRef::Unknown {
                ref_type: "future.ref".to_owned(),
                locator: json_payload(),
            },
        ];

        let encoded = serde_json::to_string(&refs).expect("refs serialize");
        let decoded: Vec<UpravaRef> = serde_json::from_str(&encoded).expect("refs deserialize");
        let kinds = serde_json::to_value(&decoded)
            .expect("refs convert to JSON value")
            .as_array()
            .expect("refs encode as array")
            .iter()
            .map(|value| {
                value
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .expect("ref kind is encoded")
                    .to_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(decoded, refs);
        assert_eq!(
            kinds,
            vec![
                "node",
                "project",
                "placement",
                "workspace",
                "session",
                "runtime",
                "turn",
                "message",
                "block",
                "artifact",
                "event",
                "command",
                "approval",
                "warning",
                "tool_call",
                "file",
                "file_range",
                "terminal",
                "terminal_command",
                "terminal_output_range",
                "diff_hunk",
                "check_result",
                "workspace_edit",
                "trace_event",
                "external_entity",
                "unknown",
            ]
        );
    }
}
