use super::*;

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
