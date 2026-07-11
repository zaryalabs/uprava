use super::*;

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
    pub release_id: String,
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
    pub value: CapabilityValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityValue {
    Provider {
        available: bool,
        configured: bool,
        mode: String,
        timeout_seconds: Option<u64>,
        unavailable_reason: Option<String>,
    },
    WorkspaceValidation {
        mode: String,
    },
    Extension {
        name: String,
        value: serde_json_value::JsonValue,
    },
}

impl CapabilityValue {
    #[must_use]
    pub fn provider(available: bool) -> Self {
        Self::Provider {
            available,
            configured: true,
            mode: "exec".to_owned(),
            timeout_seconds: None,
            unavailable_reason: None,
        }
    }
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
