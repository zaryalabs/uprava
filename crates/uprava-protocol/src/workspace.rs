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
    #[serde(default)]
    pub git_snapshot: Option<GitWorkspaceSnapshot>,
    pub last_validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub display_name: String,
    pub workspace_path: String,
    pub state: PlacementState,
    pub resource_badges: Vec<ResourceBadge>,
    #[serde(default)]
    pub git_snapshot: Option<GitWorkspaceSnapshot>,
    pub last_validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitRepositoryState {
    Ready,
    NotRepository,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHeadState {
    Branch,
    Detached,
    Unborn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitWorktreeKind {
    Primary,
    Linked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Unmerged,
    TypeChanged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitChangedFile {
    pub path: String,
    pub previous_path: Option<String>,
    pub index_status: Option<GitChangeKind>,
    pub worktree_status: Option<GitChangeKind>,
    pub conflicted: bool,
    pub binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorkspaceSnapshot {
    pub state: GitRepositoryState,
    pub repo_id: Option<String>,
    pub head_state: Option<GitHeadState>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub worktree_kind: Option<GitWorktreeKind>,
    pub operation: Option<GitOperation>,
    #[serde(default)]
    pub changed_files: Vec<GitChangedFile>,
    pub staged_count: u64,
    pub unstaged_count: u64,
    pub untracked_count: u64,
    pub conflicted_count: u64,
    pub truncated: bool,
    pub generated_at: DateTime<Utc>,
}

impl Default for GitWorkspaceSnapshot {
    fn default() -> Self {
        Self {
            state: GitRepositoryState::Unavailable,
            repo_id: None,
            head_state: None,
            branch: None,
            commit: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            worktree_kind: None,
            operation: None,
            changed_files: vec![],
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            conflicted_count: 0,
            truncated: false,
            generated_at: DateTime::<Utc>::UNIX_EPOCH,
        }
    }
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceEntryClassification {
    #[default]
    Normal,
    Generated,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: String,
    pub kind: WorkspaceEntryKind,
    pub status: WorkspaceEntryStatus,
    #[serde(default)]
    pub classification: WorkspaceEntryClassification,
    #[serde(default)]
    pub expandable: bool,
    pub byte_len: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub children: Vec<WorkspaceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTreeResponse {
    pub placement_id: ProjectPlacementId,
    pub root: WorkspaceEntry,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub total_entries: Option<u64>,
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
    #[serde(default)]
    pub git_snapshot: GitWorkspaceSnapshot,
    pub summary: String,
    pub diff: String,
    #[serde(default)]
    pub scope: WorkspaceDiffScope,
    pub path: Option<String>,
    #[serde(default)]
    pub changed_files: Vec<GitChangedFile>,
    #[serde(default)]
    pub hunks: Vec<WorkspaceDiffHunk>,
    pub original: Option<String>,
    pub modified: Option<String>,
    #[serde(default)]
    pub binary: bool,
    pub summary_truncated: bool,
    pub diff_truncated: bool,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiffHunk {
    pub hunk_id: String,
    pub header: String,
    pub patch: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDiffScope {
    #[default]
    All,
    Staged,
    Unstaged,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiffRequest {
    #[serde(default)]
    pub scope: WorkspaceDiffScope,
    pub path: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCheckRunSummary {
    pub command_id: CommandId,
    pub state: CommandState,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub label: Option<String>,
    pub success: Option<bool>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceReviewProjection {
    pub placement_id: ProjectPlacementId,
    pub git_snapshot: GitWorkspaceSnapshot,
    pub diff: WorkspaceDiffResponse,
    #[serde(default)]
    pub checks: Vec<WorkspaceCheckRunSummary>,
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
