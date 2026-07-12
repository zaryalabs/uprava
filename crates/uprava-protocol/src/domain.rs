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

/// Lifecycle of one Core-owned future turn for an existing session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledMessageState {
    Scheduled,
    Sending,
    Sent,
    Failed,
    Cancelled,
}

/// Lifecycle of one concrete execution of a durable background Job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobRunState {
    Queued,
    Starting,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobRunTrigger {
    Manual,
    Scheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOverlapPolicy {
    Skip,
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
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCapability {
    #[serde(rename = "session.attach")]
    SessionAttach,
    #[serde(rename = "session.detach")]
    SessionDetach,
    #[serde(rename = "session.sendTurn")]
    SessionSendTurn,
    #[serde(rename = "runtime.interrupt")]
    RuntimeInterrupt,
    #[serde(rename = "runtime.stop")]
    RuntimeStop,
    #[serde(rename = "runtime.resume")]
    RuntimeResume,
    #[serde(rename = "approval.resolve")]
    ApprovalResolve,
    #[serde(rename = "warning.acknowledge")]
    WarningAcknowledge,
    #[serde(rename = "reference.openInInspector")]
    ReferenceOpenInInspector,
    #[serde(rename = "reference.copy")]
    ReferenceCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(rename = "extension")]
    Extension,
}
