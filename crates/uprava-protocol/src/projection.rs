use super::*;

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
pub struct SessionEvidenceProjectionNode {
    pub evidence_id: EvidenceId,
    pub label: String,
    pub primary_ref: UpravaRef,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub children: Vec<SessionEvidenceProjectionNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEvidenceProjection {
    pub session_thread_id: SessionThreadId,
    pub root: SessionEvidenceProjectionNode,
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
    pub evidence_projection_summary: String,
    #[serde(default)]
    pub available_block_types: Vec<String>,
    #[serde(default)]
    pub available_commands: Vec<ActionCapability>,
    #[serde(default)]
    pub visible_refs: Vec<UpravaRef>,
    pub source_cause_summary: String,
    pub resume_context: String,
    pub generated_at: DateTime<Utc>,
}
