export type DeploymentProfile = "controlled_dev";
export type NodePresence = "reachable" | "stale" | "offline" | "revoked";
export type RuntimeSessionState =
  | "starting"
  | "ready"
  | "running"
  | "blocked"
  | "stopping"
  | "stopped"
  | "interrupted"
  | "resuming"
  | "stale"
  | "error"
  | "expired";
export type SessionThreadState =
  | "created"
  | "active"
  | "detached"
  | "stopped"
  | "degraded";
export type PlacementState =
  | "pending"
  | "validated"
  | "missing"
  | "read_only"
  | "error";
export type WarningSeverity = "info" | "warning" | "hard_block";
export type ClientLogLevel = "debug" | "info" | "warn" | "error";
export type CommandState =
  | "recorded"
  | "pending_dispatch"
  | "dispatched"
  | "acknowledged"
  | "completed"
  | "failed"
  | "blocked"
  | "expired";
export type CommandKind =
  | "StartRuntime"
  | "ResumeRuntime"
  | "SendTurn"
  | "ResolveApproval"
  | "InterruptRuntime"
  | "StopRuntime"
  | "ValidateWorkspace"
  | "RefreshResourceSnapshot"
  | "ListWorkspaceTree"
  | "ReadWorkspaceFile"
  | "WriteWorkspaceFile"
  | "RunWorkspaceCommand"
  | "ReadWorkspaceDiff"
  | "OpenWorkspaceTerminal"
  | "AttachWorkspaceTerminal"
  | "ResizeWorkspaceTerminal"
  | "WriteWorkspaceTerminal"
  | "CloseWorkspaceTerminal";
export type MessageRole =
  | "user"
  | "assistant"
  | "system"
  | "runtime"
  | "approval";
export type EnrollmentState =
  | "pending_user_approval"
  | "approved"
  | "registered"
  | "expired"
  | "rejected"
  | "revoked";

export type HealthResponse = {
  status: string;
  profile: DeploymentProfile;
  security?: SecurityStatus;
};

export type VersionResponse = {
  name: string;
  version: string;
  api_version: string;
  schema_version: number;
  profile: DeploymentProfile;
  security?: SecurityStatus;
};

export type SecurityMode = "hardened";

export type SecurityStatus = {
  mode: SecurityMode;
  web_auth_required: boolean;
  web_auth_configured: boolean;
  cookie_secure: boolean;
};

export type ApiError = {
  error_code: string;
  message: string;
  details?: unknown;
  retryable: boolean;
  correlation_id: string;
};

export type CapabilitySummary = {
  key: string;
  value: unknown;
};

export type NodeSummary = {
  node_id: string;
  display_name: string;
  presence: NodePresence;
  sleep_hint: string;
  heartbeat_age_seconds: number | null;
  active_runtime_count: number;
  capabilities: CapabilitySummary[];
  diagnostics: string;
};

export type ResourceBadge = {
  kind: string;
  severity: WarningSeverity;
  label: string;
};

export type ProjectPlacementSummary = {
  project_placement_id: string;
  project_id: string | null;
  node_id: string;
  display_name: string;
  workspace_path: string;
  state: PlacementState;
  resource_badges: ResourceBadge[];
  last_validated_at: string | null;
};

export type WorkspaceEntryKind = "directory" | "file" | "symlink" | "other";

export type WorkspaceEntryStatus =
  | "readable"
  | "directory"
  | "large"
  | "binary"
  | "ignored"
  | "generated"
  | "permission_denied"
  | "outside_workspace"
  | "missing"
  | "not_file"
  | "not_directory"
  | "symlink"
  | "error";

export type WorkspaceEntry = {
  name: string;
  path: string;
  kind: WorkspaceEntryKind;
  status: WorkspaceEntryStatus;
  byte_len: number | null;
  modified_at: string | null;
  children: WorkspaceEntry[];
};

export type WorkspaceTreeResponse = {
  placement_id: string;
  root: WorkspaceEntry;
  generated_at: string;
};

export type WorkspaceFileContentResponse = {
  placement_id: string;
  path: string;
  metadata: WorkspaceEntry;
  content: string | null;
  truncated: boolean;
  generated_at: string;
};

export type WorkspaceFileWriteRequest = {
  path: string;
  content: string;
  expected_content: string | null;
};

export type WorkspaceFileWriteResponse = {
  placement_id: string;
  path: string;
  metadata: WorkspaceEntry;
  edit_id: string;
  written_at: string;
};

export type WorkspaceCommandIntent = "command" | "check";

export type WorkspaceCommandRunRequest = {
  command: string;
  args: string[];
  intent: WorkspaceCommandIntent;
  label: string | null;
  timeout_seconds: number | null;
};

export type WorkspaceCommandRunResponse = {
  placement_id: string;
  terminal_command_id: string;
  command: string;
  args: string[];
  intent: WorkspaceCommandIntent;
  label: string | null;
  exit_code: number | null;
  success: boolean;
  stdout: string;
  stderr: string;
  stdout_truncated: boolean;
  stderr_truncated: boolean;
  duration_ms: number;
  started_at: string;
  completed_at: string;
};

export type WorkspaceDiffResponse = {
  placement_id: string;
  diff_id: string;
  summary: string;
  diff: string;
  summary_truncated: boolean;
  diff_truncated: boolean;
  generated_at: string;
};

export type WorkspaceCommandHistoryItem = {
  command_id: string;
  kind: CommandKind;
  state: CommandState;
  created_at: string;
  completed_at: string | null;
  payload: unknown;
  result_payload: unknown | null;
};

export type WorkspaceCommandHistoryResponse = {
  placement_id: string;
  commands: WorkspaceCommandHistoryItem[];
  generated_at: string;
};

export type WorkspaceTerminalState =
  | "opening"
  | "running"
  | "detached"
  | "exited"
  | "closed"
  | "error";

export type WorkspaceTerminalOpenRequest = {
  shell_profile: string | null;
  cols: number;
  rows: number;
};

export type WorkspaceTerminalSummary = {
  placement_id: string;
  terminal_id: string;
  title: string;
  cwd: string;
  shell: string;
  cols: number;
  rows: number;
  state: WorkspaceTerminalState;
  exit_code: number | null;
  created_at: string;
  updated_at: string;
};

export type WorkspaceTerminalOutputFrame = {
  terminal_id: string;
  seq: number;
  data: string;
  sent_at: string;
};

export type WorkspaceTerminalOpenResponse = {
  placement_id: string;
  terminal: WorkspaceTerminalSummary;
  replay: WorkspaceTerminalOutputFrame[];
};

export type WorkspaceTerminalListResponse = {
  placement_id: string;
  terminals: WorkspaceTerminalSummary[];
  generated_at: string;
};

export type WorkspaceTerminalClientFrame =
  | { kind: "input"; data: string }
  | { kind: "resize"; cols: number; rows: number }
  | { kind: "close" }
  | { kind: "ping" };

export type WorkspaceTerminalStreamFrame =
  | {
      kind: "output";
      terminal_id: string;
      seq: number;
      data: string;
      sent_at: string;
    }
  | {
      kind: "status";
      terminal_id: string;
      state: WorkspaceTerminalState;
      exit_code: number | null;
      message: string | null;
      sent_at: string;
    }
  | { kind: "pong"; sent_at: string }
  | {
      kind: "error";
      terminal_id: string;
      message: string;
      sent_at: string;
    };

export type RuntimeSummary = {
  runtime_session_id: string;
  provider: string;
  state: RuntimeSessionState;
  resume_supported: boolean;
  degraded_reason: string | null;
  last_runtime_step_at: string | null;
};

export type SessionSummary = {
  session_thread_id: string;
  project_placement_id: string;
  runtime_session_id: string;
  title: string;
  state: SessionThreadState;
  runtime: RuntimeSummary;
  message_count: number;
  updated_at: string;
};

export type Message = {
  message_id: string;
  session_thread_id: string;
  turn_id: string | null;
  role: MessageRole;
  content: string;
  created_at: string;
  completed_at: string | null;
  source_event_id: string | null;
};

export type EventEnvelope = {
  event_id: string;
  command_id: string | null;
  correlation_id?: string | null;
  actor_ref: unknown;
  scope_ref: unknown;
  node_id: string | null;
  runtime_session_id: string | null;
  session_thread_id: string | null;
  turn_id: string | null;
  seq: number;
  session_projection_seq?: number | null;
  kind: string;
  happened_at: string;
  source_refs: UpravaRef[];
  evidence_refs: UpravaRef[];
  cause_refs: UpravaRef[];
  result_refs: UpravaRef[];
  payload: unknown;
};

export type UiBlock = {
  block_id: string;
  type: string;
  schema_version: number;
  surface_id: string;
  primary_ref: UpravaRef;
  parent_ref?: UpravaRef | null;
  children: UiBlock[];
  source_refs: UpravaRef[];
  evidence_refs: UpravaRef[];
  cause_refs: UpravaRef[];
  related_refs: UpravaRef[];
  trace_refs: UpravaRef[];
  data: unknown;
  actions: string[];
  fallback_text?: string | null;
};

export type InventorySnapshot = {
  nodes: NodeSummary[];
  placements: ProjectPlacementSummary[];
  sessions: SessionSummary[];
  generated_at: string;
};

export type SessionDetail = {
  session: SessionSummary;
  placement: ProjectPlacementSummary;
  messages: Message[];
  events: EventEnvelope[];
};

export type CreatePlacementRequest = {
  node_id: string;
  display_name: string;
  workspace_path: string;
};

export type CreateSessionRequest = {
  project_placement_id: string;
  title?: string;
  provider: string;
};

export type SendTurnRequest = {
  content: string;
};

export type ResolveApprovalRequest = {
  approved: boolean;
  message?: string | null;
};

export type AcknowledgeWarningRequest = {
  message?: string | null;
};

export type ClientLogRequest = {
  level: ClientLogLevel;
  source: string;
  message: string;
  route?: string | null;
  user_agent?: string | null;
  occurred_at: string;
  detail?: unknown;
};

export type ClientLogResponse = {
  accepted: boolean;
};

export type WebAuthStatusResponse = {
  auth_required: boolean;
  setup_required: boolean;
  authenticated: boolean;
  profile: DeploymentProfile;
  security: SecurityStatus;
};

export type WebAuthSetupRequest = {
  password: string;
};

export type WebAuthLoginRequest = {
  password: string;
};

export type WebAuthResponse = {
  authenticated: boolean;
  setup_required: boolean;
  csrf_token: string | null;
  security: SecurityStatus;
};

export type ClientCreateNodeEnrollmentRequest = {
  display_name: string;
};

export type NodeEnrollmentRequestedResponse = {
  enrollment_id: string;
  pairing_code: string;
  status: EnrollmentState;
  expires_at: string;
};

export type NodeEnrollmentSummary = {
  enrollment_id: string;
  display_name: string;
  status: EnrollmentState;
  claimed_node_id: string | null;
  expires_at: string;
  created_at: string;
  approved_at: string | null;
};

export type ApproveNodeEnrollmentResponse = {
  enrollment: NodeEnrollmentSummary;
};

export type NodeRevocationResponse = {
  node_id: string;
  revoked: boolean;
};

export type NodeCredentialRotationResponse = {
  node_id: string;
  credential: string;
  rotated_at: string;
};

export type NodeDeletionResponse = {
  node_id: string;
  deleted: boolean;
};

export type PlacementDeletionResponse = {
  project_placement_id: string;
  deleted: boolean;
};

export type CommandAcceptedResponse = {
  command_id: string;
  session: SessionDetail | null;
};

export type WarningAcknowledgementResponse = {
  event_id: string;
  session: SessionDetail;
};

export type UpravaRef =
  | { kind: "node"; node_id: string }
  | { kind: "project"; project_id: string }
  | { kind: "placement"; placement_id: string }
  | { kind: "workspace"; placement_id: string }
  | { kind: "session"; session_thread_id: string }
  | { kind: "runtime"; runtime_session_id: string }
  | { kind: "turn"; turn_id: string }
  | { kind: "message"; message_id: string }
  | { kind: "block"; block_id: string }
  | { kind: "artifact"; artifact_id: string }
  | { kind: "event"; event_id: string; scope_ref: unknown; seq: number }
  | { kind: "command"; command_id: string }
  | { kind: "approval"; approval_id: string }
  | { kind: "warning"; warning_kind: string; command_id?: string | null }
  | { kind: "tool_call"; tool_call_id: string }
  | {
      kind: "file";
      placement_id: string;
      path: string;
      version?: string | null;
    }
  | {
      kind: "file_range";
      placement_id: string;
      path: string;
      range: TextRange;
      version?: string | null;
    }
  | { kind: "terminal"; terminal_id: string; placement_id: string }
  | {
      kind: "terminal_command";
      terminal_command_id: string;
      terminal_id?: string | null;
    }
  | {
      kind: "terminal_output_range";
      terminal_command_id: string;
      range: TextRange;
    }
  | { kind: "diff_hunk"; diff_id: string; hunk_id: string }
  | { kind: "check_result"; check_run_id: string; failure_id?: string | null }
  | {
      kind: "workspace_edit";
      edit_id: string;
      placement_id?: string | null;
      path?: string | null;
    }
  | { kind: "trace_event"; trace_event_id: string }
  | { kind: "external_entity"; integration_kind: string; external_id: string }
  | { kind: "unknown"; ref_type: string; locator: Record<string, unknown> }
  | { kind: string; [key: string]: unknown };

export type TextRange = {
  start_line?: number | null;
  end_line?: number | null;
  start_offset?: number | null;
  end_offset?: number | null;
};

export type ArtifactTreeNode = {
  artifact_id: string;
  label: string;
  primary_ref: UpravaRef;
  source_refs: UpravaRef[];
  evidence_refs: UpravaRef[];
  cause_refs: UpravaRef[];
  children: ArtifactTreeNode[];
};

export type ArtifactTree = {
  session_thread_id: string;
  root: ArtifactTreeNode;
  generated_at: string;
};

export type AgentProjection = {
  session_thread_id: string;
  project_placement: ProjectPlacementSummary;
  runtime_summary: RuntimeSummary;
  current_turn: string | null;
  pending_approvals: string[];
  active_warnings: ResourceBadge[];
  recent_turn_summaries: string[];
  recent_message_refs: UpravaRef[];
  artifact_tree_summary: string;
  available_block_types: string[];
  available_commands: string[];
  visible_refs: UpravaRef[];
  source_cause_summary: string;
  resume_context: string;
  generated_at: string;
};
