use super::*;

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
    pub scheduled_messages: Vec<ScheduledSessionMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledSessionMessage {
    pub scheduled_message_id: String,
    pub session_thread_id: SessionThreadId,
    pub content: String,
    pub due_at: DateTime<Utc>,
    /// IANA timezone selected by the person who scheduled the message.
    pub timezone: String,
    pub state: ScheduledMessageState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sending_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub command_id: Option<CommandId>,
    pub turn_id: Option<TurnId>,
    pub failure: Option<ScheduledMessageFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledMessageFailure {
    pub code: String,
    pub message: String,
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
pub struct CreateScheduledMessageRequest {
    pub content: String,
    pub due_at: DateTime<Utc>,
    pub timezone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateScheduledMessageRequest {
    pub content: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub timezone: Option<String>,
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
