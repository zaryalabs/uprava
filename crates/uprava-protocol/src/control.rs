use super::*;

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
