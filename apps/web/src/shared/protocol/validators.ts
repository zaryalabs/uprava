import { z } from "zod";

import {
  COMMAND_KIND_VALUES,
  COMMAND_STATE_VALUES,
  EVENT_KIND_VALUES,
  INTEGRATION_AUTH_STATE_VALUES,
  INTEGRATION_DESIRED_STATE_VALUES,
  MCP_DEPENDENCY_ACTUAL_STATE_VALUES,
  MESSAGE_ROLE_VALUES,
  OBSERVED_CAPABILITY_STATE_VALUES,
  PLACEMENT_STATE_VALUES,
  POLICY_DECISION_VALUES,
  TOOL_AVAILABILITY_STATE_VALUES,
  TOOL_CALL_STATE_VALUES,
  TOOL_DEFINITION_STATE_VALUES,
  TOOL_EXECUTION_ERROR_CODE_VALUES,
  TOOL_EXECUTION_KIND_VALUES,
  TOOL_INVOCATION_MODE_VALUES,
  TOOL_RISK_LEVEL_VALUES,
  TOOL_SOURCE_KIND_VALUES,
  TOOL_UNAVAILABLE_REASON_VALUES,
  WARNING_SEVERITY_VALUES,
  WORKSPACE_COMMAND_INTENT_VALUES,
  WORKSPACE_TERMINAL_STATE_VALUES,
} from "./literals";
import type {
  ActorRef,
  CommandAcceptedResponse,
  CommandKind,
  CommandState,
  EventEnvelope,
  EventKind,
  EventPayload,
  ExecuteToolRequest,
  ExecuteToolResponse,
  InspectToolRequest,
  InspectToolResponse,
  IntegrationConnectRequest,
  IntegrationConnectResponse,
  IntegrationConnectionSummary,
  IntegrationConnectionsResponse,
  IntegrationDisconnectRequest,
  IntegrationDisconnectResponse,
  McpAccessLeaseClaims,
  McpDependencyStatus,
  McpDependencyStatusesResponse,
  MessageRole,
  ObservedCapabilitiesResponse,
  ObservedCapability,
  SearchToolsRequest,
  SearchToolsResponse,
  ToolAvailability,
  ToolAvailabilityResponse,
  ToolCallDetail,
  ToolCallSummary,
  ToolCallsResponse,
  ToolDefinition,
  ToolDefinitionsResponse,
  ToolExecutionError,
  ToolResultEnvelope,
  ToolScope,
  ToolingCommandV1,
  ToolingContractFixture,
  ToolingEventV1,
  WorkspaceCommandHistoryItem,
  WorkspaceCommandHistoryResponse,
  WorkspaceCommandIntent,
  WorkspaceCommandRunResponse,
  WorkspaceDiffResponse,
  WorkspaceReviewProjection,
  WorkspaceTerminalListResponse,
  WorkspaceTerminalOpenResponse,
  WorkspaceTerminalOutputFrame,
  WorkspaceTerminalState,
  WorkspaceTerminalStreamFrame,
  WorkspaceTerminalSummary,
} from "./types";

export type ProtocolSchema<T> = {
  safeParse: (
    value: unknown,
  ) =>
    | { success: true; data: T }
    | { success: false; error: { issues: ProtocolValidationIssue[] } };
};

export type ProtocolValidationIssue = {
  path: PropertyKey[];
  message: string;
};

export const commandStateSchema = z.enum(
  COMMAND_STATE_VALUES,
) satisfies z.ZodType<CommandState>;

export const commandKindSchema = z.enum(
  COMMAND_KIND_VALUES,
) satisfies z.ZodType<CommandKind>;

export const eventKindSchema = z.enum(
  EVENT_KIND_VALUES,
) satisfies z.ZodType<EventKind>;

export const messageRoleSchema = z.enum(
  MESSAGE_ROLE_VALUES,
) satisfies z.ZodType<MessageRole>;

export const workspaceCommandIntentSchema = z.enum(
  WORKSPACE_COMMAND_INTENT_VALUES,
) satisfies z.ZodType<WorkspaceCommandIntent>;

export const workspaceTerminalStateSchema = z.enum(
  WORKSPACE_TERMINAL_STATE_VALUES,
) satisfies z.ZodType<WorkspaceTerminalState>;

const nullableString = z.string().nullable();
const nullableNumber = z.number().nullable();
const protocolRefSchema = z.object({ kind: z.string() }).passthrough();
const actorRefSchema = z.discriminatedUnion("kind", [
  z
    .object({ kind: z.literal("local_user"), actor_id: nullableString })
    .strict(),
  z.object({ kind: z.literal("system") }).strict(),
  z.object({ kind: z.literal("node"), node_id: z.string() }).strict(),
  z.object({ kind: z.literal("provider"), provider: z.string() }).strict(),
  z.object({ kind: z.literal("unknown") }).strict(),
]) satisfies z.ZodType<ActorRef>;
const toolSourceKindSchema = z.enum(TOOL_SOURCE_KIND_VALUES);
const toolExecutionKindSchema = z.enum(TOOL_EXECUTION_KIND_VALUES);
const toolRiskLevelSchema = z.enum(TOOL_RISK_LEVEL_VALUES);
const toolDefinitionStateSchema = z.enum(TOOL_DEFINITION_STATE_VALUES);
const toolAvailabilityStateSchema = z.enum(TOOL_AVAILABILITY_STATE_VALUES);
const toolUnavailableReasonSchema = z.enum(TOOL_UNAVAILABLE_REASON_VALUES);
const observedCapabilityStateSchema = z.enum(OBSERVED_CAPABILITY_STATE_VALUES);
const integrationDesiredStateSchema = z.enum(INTEGRATION_DESIRED_STATE_VALUES);
const integrationAuthStateSchema = z.enum(INTEGRATION_AUTH_STATE_VALUES);
const mcpDependencyActualStateSchema = z.enum(
  MCP_DEPENDENCY_ACTUAL_STATE_VALUES,
);
const policyDecisionSchema = z.enum(POLICY_DECISION_VALUES);
const toolCallStateSchema = z.enum(TOOL_CALL_STATE_VALUES);
const toolInvocationModeSchema = z.enum(TOOL_INVOCATION_MODE_VALUES);
const toolExecutionErrorCodeSchema = z.enum(TOOL_EXECUTION_ERROR_CODE_VALUES);
const upravaRefSchema = protocolRefSchema as z.ZodType<
  import("./types").UpravaRef
>;

export const toolScopeSchema = z
  .object({
    actor_ref: actorRefSchema,
    node_id: nullableString,
    project_id: nullableString,
    project_placement_id: nullableString,
    session_thread_id: nullableString,
  })
  .strict() satisfies z.ZodType<ToolScope>;

export const toolDefinitionSchema = z
  .object({
    tool_id: z.string(),
    source_id: z.string(),
    source_kind: toolSourceKindSchema,
    source_tool_name: z.string(),
    version: z.number().int().nonnegative(),
    display_name: z.string(),
    short_description: z.string(),
    documentation_url: nullableString,
    input_schema: z.unknown(),
    output_schema: z.unknown(),
    schema_hash: z.string(),
    risk_level: toolRiskLevelSchema,
    required_permissions: z.array(z.string()),
    execution_kind: toolExecutionKindSchema,
    approval_policy: policyDecisionSchema,
    redaction: z
      .object({
        argument_json_pointers: z.array(z.string()),
        result_json_pointers: z.array(z.string()),
        redact_all_arguments: z.boolean(),
        redact_all_result: z.boolean(),
        max_summary_bytes: z.number().int().nonnegative(),
      })
      .strict(),
    state: toolDefinitionStateSchema,
    created_at: z.string(),
    updated_at: z.string(),
  })
  .strict() satisfies z.ZodType<ToolDefinition>;

export const toolAvailabilitySchema = z
  .object({
    tool_id: z.string(),
    scope: toolScopeSchema,
    state: toolAvailabilityStateSchema,
    reason: toolUnavailableReasonSchema.nullable(),
    backend_ref: nullableString,
    dependency_instance_id: nullableString,
    schema_hash: z.string(),
    policy_version: z.string(),
    observed_at: z.string(),
  })
  .strict() satisfies z.ZodType<ToolAvailability>;

export const observedCapabilitySchema = z
  .object({
    node_id: z.string(),
    capability_key: z.string(),
    display_name: z.string(),
    state: observedCapabilityStateSchema,
    version: nullableString,
    safe_authentication_state: nullableString,
    observed_at: z.string(),
  })
  .strict() satisfies z.ZodType<ObservedCapability>;

export const integrationConnectionSummarySchema = z
  .object({
    integration_id: z.string(),
    source_id: z.string(),
    provider: z.string(),
    display_name: z.string(),
    desired_state: integrationDesiredStateSchema,
    auth_state: integrationAuthStateSchema,
    node_id: nullableString,
    authenticated_actor_label: nullableString,
    connected_at: nullableString,
    updated_at: z.string(),
    error_code: nullableString,
  })
  .strict() satisfies z.ZodType<IntegrationConnectionSummary>;

export const mcpDependencyStatusSchema = z
  .object({
    dependency_instance_id: z.string(),
    integration_id: z.string(),
    node_id: z.string(),
    desired_state: integrationDesiredStateSchema,
    actual_state: mcpDependencyActualStateSchema,
    runtime_name: z.string(),
    runtime_version: nullableString,
    upstream_identity: nullableString,
    schema_set_hash: nullableString,
    error_code: nullableString,
    observed_at: z.string(),
  })
  .strict() satisfies z.ZodType<McpDependencyStatus>;

const toolSearchFiltersSchema = z
  .object({
    source_kinds: z.array(toolSourceKindSchema),
    risk_levels: z.array(toolRiskLevelSchema),
    availability_states: z.array(toolAvailabilityStateSchema),
  })
  .strict();

export const searchToolsRequestSchema = z
  .object({
    scope: toolScopeSchema,
    query: z.string(),
    filters: toolSearchFiltersSchema,
    cursor: nullableString,
    limit: z.number().int().positive().nullable(),
  })
  .strict() satisfies z.ZodType<SearchToolsRequest>;

const toolSearchResultSchema = z
  .object({
    tool_id: z.string(),
    display_name: z.string(),
    short_description: z.string(),
    source_kind: toolSourceKindSchema,
    risk_level: toolRiskLevelSchema,
    availability_state: toolAvailabilityStateSchema,
    unavailable_reason: toolUnavailableReasonSchema.nullable(),
    schema_hash: z.string(),
  })
  .strict();

export const searchToolsResponseSchema = z
  .object({
    items: z.array(toolSearchResultSchema),
    next_cursor: nullableString,
  })
  .strict() satisfies z.ZodType<SearchToolsResponse>;

export const inspectToolRequestSchema = z
  .object({ scope: toolScopeSchema, tool_id: z.string() })
  .strict() satisfies z.ZodType<InspectToolRequest>;

export const inspectToolResponseSchema = z
  .object({
    definition: toolDefinitionSchema,
    availability: toolAvailabilitySchema,
    invocation_mode: toolInvocationModeSchema,
  })
  .strict() satisfies z.ZodType<InspectToolResponse>;

export const executeToolRequestSchema = z
  .object({
    scope: toolScopeSchema,
    tool_id: z.string(),
    arguments: z.unknown(),
  })
  .strict() satisfies z.ZodType<ExecuteToolRequest>;

const toolExecutionErrorSchema = z
  .object({
    code: toolExecutionErrorCodeSchema,
    message: z.string(),
    retryable: z.boolean(),
    redacted_details: z.unknown(),
  })
  .strict() satisfies z.ZodType<ToolExecutionError>;

const toolResultEnvelopeSchema = z
  .object({
    content: z.unknown(),
    summary: nullableString,
    truncated: z.boolean(),
    original_size_bytes: nullableNumber,
    artifact_refs: z.array(upravaRefSchema),
  })
  .strict() satisfies z.ZodType<ToolResultEnvelope>;

export const executeToolResponseSchema = z
  .object({
    tool_call_id: z.string(),
    state: toolCallStateSchema,
    result: toolResultEnvelopeSchema.nullable(),
    error: toolExecutionErrorSchema.nullable(),
  })
  .strict() satisfies z.ZodType<ExecuteToolResponse>;

const toolCallSummarySchema = z
  .object({
    tool_call_id: z.string(),
    tool_id: z.string(),
    schema_hash: z.string(),
    actor_ref: actorRefSchema,
    scope: toolScopeSchema,
    source_kind: toolSourceKindSchema,
    state: toolCallStateSchema,
    policy_decision: policyDecisionSchema,
    route: z.string(),
    requested_at: z.string(),
    started_at: nullableString,
    completed_at: nullableString,
    correlation_id: z.string(),
  })
  .strict() satisfies z.ZodType<ToolCallSummary>;

export const toolCallDetailSchema = z
  .object({
    summary: toolCallSummarySchema,
    command_id: nullableString,
    integration_id: nullableString,
    dependency_instance_id: nullableString,
    policy_version: z.string(),
    redacted_arguments_summary: nullableString,
    redacted_result_summary: nullableString,
    argument_hash: nullableString,
    result_hash: nullableString,
    result_size_bytes: nullableNumber,
    trace_refs: z.array(upravaRefSchema),
    result_refs: z.array(upravaRefSchema),
    error: toolExecutionErrorSchema.nullable(),
  })
  .strict() satisfies z.ZodType<ToolCallDetail>;

const toolingCommandPayloadSchema = z.discriminatedUnion("type", [
  z
    .object({
      type: z.literal("execute_external_tool"),
      tool_call_id: z.string(),
      tool_id: z.string(),
      schema_hash: z.string(),
      integration_id: z.string(),
      dependency_instance_id: z.string(),
      scope: toolScopeSchema,
      arguments: z.unknown(),
      deadline_at: z.string(),
      max_result_bytes: z.number().int().nonnegative(),
    })
    .strict(),
  z
    .object({
      type: z.literal("cancel_tool_call"),
      tool_call_id: z.string(),
      reason: nullableString,
    })
    .strict(),
  z
    .object({
      type: z.literal("update_dependency_desired_state"),
      dependency_instance_id: z.string(),
      integration_id: z.string(),
      desired_state: integrationDesiredStateSchema,
      credential_ref: nullableString,
    })
    .strict(),
]);

export const toolingCommandV1Schema = z
  .object({
    contract_version: z.literal(1),
    payload: toolingCommandPayloadSchema,
  })
  .strict() satisfies z.ZodType<ToolingCommandV1>;

const toolingEventPayloadSchema = z.discriminatedUnion("type", [
  z
    .object({
      type: z.literal("dependency_actual_state_reported"),
      status: mcpDependencyStatusSchema,
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_definitions_discovered"),
      dependency_instance_id: z.string(),
      definitions: z.array(toolDefinitionSchema),
      schema_set_hash: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_call_started"),
      tool_call_id: z.string(),
      started_at: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_call_completed"),
      tool_call_id: z.string(),
      result: toolResultEnvelopeSchema,
      completed_at: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_call_failed"),
      tool_call_id: z.string(),
      error: toolExecutionErrorSchema,
      failed_at: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_call_denied"),
      tool_call_id: z.string(),
      error: toolExecutionErrorSchema,
      denied_at: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("tool_availability_changed"),
      availability: toolAvailabilitySchema,
    })
    .strict(),
]);

export const toolingEventV1Schema = z
  .object({
    contract_version: z.literal(1),
    payload: toolingEventPayloadSchema,
  })
  .strict() satisfies z.ZodType<ToolingEventV1>;

const mcpAccessLeaseClaimsSchema = z
  .object({
    lease_id: z.string(),
    audience: z.string(),
    actor_ref: actorRefSchema,
    session_thread_id: z.string(),
    project_id: nullableString,
    project_placement_id: z.string(),
    node_id: z.string(),
    issued_at: z.string(),
    expires_at: z.string(),
    credential_version: z.number().int().nonnegative(),
  })
  .strict() satisfies z.ZodType<McpAccessLeaseClaims>;

export const toolDefinitionsResponseSchema = z
  .object({
    items: z.array(toolDefinitionSchema),
    next_cursor: nullableString,
  })
  .strict() satisfies z.ZodType<ToolDefinitionsResponse>;

export const toolAvailabilityResponseSchema = z
  .object({
    items: z.array(toolAvailabilitySchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<ToolAvailabilityResponse>;

export const observedCapabilitiesResponseSchema = z
  .object({
    items: z.array(observedCapabilitySchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<ObservedCapabilitiesResponse>;

export const integrationConnectionsResponseSchema = z
  .object({ items: z.array(integrationConnectionSummarySchema) })
  .strict() satisfies z.ZodType<IntegrationConnectionsResponse>;

export const mcpDependencyStatusesResponseSchema = z
  .object({
    items: z.array(mcpDependencyStatusSchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<McpDependencyStatusesResponse>;

export const toolCallsResponseSchema = z
  .object({
    items: z.array(toolCallSummarySchema),
    next_cursor: nullableString,
  })
  .strict() satisfies z.ZodType<ToolCallsResponse>;

export const integrationConnectRequestSchema = z
  .object({
    integration_id: z.string(),
    project_id: nullableString,
    node_id: z.string(),
  })
  .strict() satisfies z.ZodType<IntegrationConnectRequest>;

export const integrationConnectResponseSchema = z
  .object({
    connection: integrationConnectionSummarySchema,
    authorization_url: z.string(),
    expires_at: z.string(),
  })
  .strict() satisfies z.ZodType<IntegrationConnectResponse>;

export const integrationDisconnectRequestSchema = z
  .object({ revoke_remote: z.boolean() })
  .strict() satisfies z.ZodType<IntegrationDisconnectRequest>;

export const integrationDisconnectResponseSchema = z
  .object({
    connection: integrationConnectionSummarySchema,
    remote_revocation_confirmed: z.boolean(),
  })
  .strict() satisfies z.ZodType<IntegrationDisconnectResponse>;

export const toolingContractFixtureSchema = z
  .object({
    tool_definition: toolDefinitionSchema,
    availability: toolAvailabilitySchema,
    observed_capability: observedCapabilitySchema,
    integration: integrationConnectionSummarySchema,
    dependency: mcpDependencyStatusSchema,
    search_request: searchToolsRequestSchema,
    search_response: searchToolsResponseSchema,
    inspect_request: inspectToolRequestSchema,
    inspect_response: inspectToolResponseSchema,
    execute_request: executeToolRequestSchema,
    execute_response: executeToolResponseSchema,
    tool_call_detail: toolCallDetailSchema,
    node_command: toolingCommandV1Schema,
    node_event: toolingEventV1Schema,
    lease_claims: mcpAccessLeaseClaimsSchema,
    tool_definitions: toolDefinitionsResponseSchema,
    tool_availability: toolAvailabilityResponseSchema,
    observed_capabilities: observedCapabilitiesResponseSchema,
    integration_connections: integrationConnectionsResponseSchema,
    dependency_statuses: mcpDependencyStatusesResponseSchema,
    tool_calls: toolCallsResponseSchema,
    integration_connect_request: integrationConnectRequestSchema,
    integration_connect_response: integrationConnectResponseSchema,
    integration_disconnect_request: integrationDisconnectRequestSchema,
    integration_disconnect_response: integrationDisconnectResponseSchema,
  })
  .strict() satisfies z.ZodType<ToolingContractFixture>;
const gitChangeKindSchema = z.enum([
  "added",
  "modified",
  "deleted",
  "renamed",
  "copied",
  "untracked",
  "unmerged",
  "type_changed",
  "unknown",
]);
const gitChangedFileSchema = z
  .object({
    path: z.string(),
    previous_path: nullableString,
    index_status: gitChangeKindSchema.nullable(),
    worktree_status: gitChangeKindSchema.nullable(),
    conflicted: z.boolean(),
    binary: z.boolean(),
  })
  .strict();
const gitWorkspaceSnapshotSchema = z
  .object({
    state: z.enum(["ready", "not_repository", "unavailable"]),
    repo_id: nullableString,
    head_state: z.enum(["branch", "detached", "unborn"]).nullable(),
    branch: nullableString,
    commit: nullableString,
    upstream: nullableString,
    ahead: z.number().int().nonnegative(),
    behind: z.number().int().nonnegative(),
    worktree_kind: z.enum(["primary", "linked"]).nullable(),
    operation: z
      .enum(["merge", "rebase", "cherry_pick", "revert", "bisect"])
      .nullable(),
    changed_files: z.array(gitChangedFileSchema),
    staged_count: z.number().int().nonnegative(),
    unstaged_count: z.number().int().nonnegative(),
    untracked_count: z.number().int().nonnegative(),
    conflicted_count: z.number().int().nonnegative(),
    truncated: z.boolean(),
    generated_at: z.string(),
  })
  .strict();
const runtimeStateEventPayloadSchema = z
  .object({
    type: z.enum([
      "runtime_starting",
      "runtime_ready",
      "runtime_running",
      "runtime_blocked",
      "runtime_expired",
      "runtime_resuming",
      "runtime_stopped",
    ]),
    provider: nullableString,
    mode: nullableString,
    resume_source: nullableString,
    provider_resume_ref: z.unknown().nullable(),
    transcript_messages: nullableNumber,
    reason: nullableString,
    code: nullableString,
    message: nullableString,
    expiry_seconds: nullableNumber,
  })
  .strict();
const providerActivityEventPayloadSchema = z
  .object({
    type: z.literal("provider_activity"),
    provider: nullableString,
    source: nullableString,
    provider_event_type: nullableString,
    provider_item_id: nullableString,
    provider_item_type: nullableString,
    phase: nullableString,
    status: nullableString,
    summary: nullableString,
    raw_event: z.unknown().nullable(),
    raw_event_truncated: z.boolean().nullable(),
    raw_event_original_chars: nullableNumber,
    raw_event_preview: nullableString,
    dropped_count: nullableNumber,
    stream: nullableString,
    limit_bytes: nullableNumber,
    stdout_truncated: z.boolean().nullable(),
    stderr_truncated: z.boolean().nullable(),
    dropped_activity_count: nullableNumber,
    max_process_output_bytes: nullableNumber,
    max_activity_events: nullableNumber,
    extension: z.unknown().nullable(),
  })
  .strict();
const workspaceSnapshotEventFields = {
  placement_id: z.string(),
  display_name: z.string(),
  workspace_path: z.string(),
  state: z.enum(PLACEMENT_STATE_VALUES),
  resource_badges: z.array(
    z
      .object({
        kind: z.string(),
        severity: z.enum(WARNING_SEVERITY_VALUES),
        label: z.string(),
      })
      .strict(),
  ),
  git_snapshot: gitWorkspaceSnapshotSchema.nullable().optional(),
};
export const eventPayloadSchema = z.union([
  runtimeStateEventPayloadSchema,
  z
    .object({
      type: z.literal("runtime_error"),
      code: z.string(),
      message: z.string(),
    })
    .strict(),
  z.object({ type: z.literal("turn_started") }).strict(),
  z.object({ type: z.literal("turn_completed") }).strict(),
  z
    .object({
      type: z.literal("turn_interrupted"),
      provider: nullableString,
      code: nullableString,
      message: nullableString,
    })
    .strict(),
  providerActivityEventPayloadSchema,
  z
    .object({ type: z.literal("provider_output_delta"), content: z.string() })
    .strict(),
  z
    .object({
      type: z.literal("provider_message_completed"),
      content: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("approval_requested"),
      approval_id: z.string(),
      prompt: z.string(),
      provider: nullableString,
      provider_event_type: nullableString,
      source: nullableString,
    })
    .strict(),
  z
    .object({
      type: z.literal("approval_resolved"),
      approval_id: z.string(),
      approved: z.boolean(),
      message: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("coordination_warning_acknowledged"),
      warning_kind: z.string(),
      message: nullableString,
      affected_refs: z.array(protocolRefSchema),
    })
    .strict(),
  z
    .object({
      type: z.literal("workspace_validated"),
      ...workspaceSnapshotEventFields,
    })
    .strict(),
  z
    .object({
      type: z.literal("resource_snapshot_updated"),
      ...workspaceSnapshotEventFields,
    })
    .strict(),
  z
    .object({
      type: z.literal("workspace_file_written"),
      placement_id: z.string(),
      path: z.string(),
      edit_id: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("workspace_command_completed"),
      placement_id: z.string(),
      terminal_command_id: z.string(),
      success: z.boolean(),
      exit_code: nullableNumber,
      stdout_truncated: z.boolean(),
      stderr_truncated: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal("workspace_check_completed"),
      placement_id: z.string(),
      check_run_id: z.string(),
      success: z.boolean(),
      exit_code: nullableNumber,
      stdout_truncated: z.boolean(),
      stderr_truncated: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal("workspace_diff_observed"),
      placement_id: z.string(),
      diff_id: z.string(),
      summary_truncated: z.boolean(),
      diff_truncated: z.boolean(),
    })
    .strict(),
  z
    .object({
      type: z.literal("deduction_requested"),
      deduction_id: z.string(),
      scope_ref: protocolRefSchema,
      question: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("deduction_completed"),
      deduction_id: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("deduction_cancelled"),
      deduction_id: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.enum(["deduction_invalid", "deduction_failed"]),
      deduction_id: z.string(),
      code: z.string(),
      message: z.string(),
    })
    .strict(),
  z
    .object({
      type: z.literal("extension"),
      name: z.string(),
      value: z.unknown(),
    })
    .strict(),
]) as z.ZodType<EventPayload>;

const payloadTypeByEventKind: Record<EventKind, EventPayload["type"]> = {
  "runtime.starting": "runtime_starting",
  "runtime.ready": "runtime_ready",
  "runtime.running": "runtime_running",
  "runtime.blocked": "runtime_blocked",
  "runtime.expired": "runtime_expired",
  "runtime.resuming": "runtime_resuming",
  "runtime.stopped": "runtime_stopped",
  "runtime.error": "runtime_error",
  "turn.started": "turn_started",
  "turn.completed": "turn_completed",
  "turn.interrupted": "turn_interrupted",
  "provider.activity": "provider_activity",
  "provider.output.delta": "provider_output_delta",
  "provider.message.completed": "provider_message_completed",
  "approval.requested": "approval_requested",
  "approval.resolved": "approval_resolved",
  "coordination.warning_acknowledged": "coordination_warning_acknowledged",
  "workspace.validated": "workspace_validated",
  "resource.snapshot.updated": "resource_snapshot_updated",
  "workspace.file.written": "workspace_file_written",
  "workspace.command.completed": "workspace_command_completed",
  "workspace.check.completed": "workspace_check_completed",
  "workspace.diff.observed": "workspace_diff_observed",
  "deduction.requested": "deduction_requested",
  "deduction.completed": "deduction_completed",
  "deduction.invalid": "deduction_invalid",
  "deduction.failed": "deduction_failed",
  "deduction.cancelled": "deduction_cancelled",
  extension: "extension",
};

export function eventPayloadTypeForKind(kind: EventKind): EventPayload["type"] {
  return payloadTypeByEventKind[kind];
}
const commandAcceptedSessionSchema = z.custom<
  CommandAcceptedResponse["session"]
>((value) => value === null || (typeof value === "object" && value !== null));

export const commandAcceptedResponseSchema = z
  .object({
    command_id: z.string(),
    session: commandAcceptedSessionSchema,
  })
  .strict() satisfies z.ZodType<CommandAcceptedResponse>;

export const workspaceCommandRunResponseSchema = z
  .object({
    placement_id: z.string(),
    terminal_command_id: z.string(),
    command: z.string(),
    args: z.array(z.string()),
    intent: workspaceCommandIntentSchema,
    label: nullableString,
    exit_code: nullableNumber,
    success: z.boolean(),
    stdout: z.string(),
    stderr: z.string(),
    stdout_truncated: z.boolean(),
    stderr_truncated: z.boolean(),
    duration_ms: z.number(),
    started_at: z.string(),
    completed_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceCommandRunResponse>;

export const workspaceDiffResponseSchema = z
  .object({
    placement_id: z.string(),
    diff_id: z.string(),
    git_snapshot: gitWorkspaceSnapshotSchema,
    summary: z.string(),
    diff: z.string(),
    scope: z.enum(["all", "staged", "unstaged"]),
    path: nullableString,
    changed_files: z.array(gitChangedFileSchema),
    hunks: z.array(
      z
        .object({
          hunk_id: z.string(),
          header: z.string(),
          patch: z.string(),
        })
        .strict(),
    ),
    original: nullableString,
    modified: nullableString,
    binary: z.boolean(),
    summary_truncated: z.boolean(),
    diff_truncated: z.boolean(),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceDiffResponse>;

const workspaceCheckRunSummarySchema = z
  .object({
    command_id: z.string(),
    state: commandStateSchema,
    command: z.string(),
    args: z.array(z.string()),
    label: nullableString,
    success: z.boolean().nullable(),
    exit_code: nullableNumber,
    stdout: nullableString,
    stderr: nullableString,
    stdout_truncated: z.boolean(),
    stderr_truncated: z.boolean(),
    duration_ms: nullableNumber,
    created_at: z.string(),
    completed_at: nullableString,
  })
  .strict();

export const workspaceReviewProjectionSchema = z
  .object({
    placement_id: z.string(),
    git_snapshot: gitWorkspaceSnapshotSchema,
    diff: workspaceDiffResponseSchema,
    checks: z.array(workspaceCheckRunSummarySchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceReviewProjection>;

export const workspaceCommandHistoryItemSchema = z
  .object({
    command_id: z.string(),
    kind: commandKindSchema,
    state: commandStateSchema,
    created_at: z.string(),
    completed_at: nullableString,
    payload: z.unknown(),
    result_payload: z.unknown().nullable(),
  })
  .strict() satisfies z.ZodType<WorkspaceCommandHistoryItem>;

export const workspaceCommandHistoryResponseSchema = z
  .object({
    placement_id: z.string(),
    commands: z.array(workspaceCommandHistoryItemSchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceCommandHistoryResponse>;

export const workspaceTerminalOutputFrameSchema = z
  .object({
    terminal_id: z.string(),
    seq: z.number().int().nonnegative(),
    data: z.string(),
    sent_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceTerminalOutputFrame>;

export const workspaceTerminalSummarySchema = z
  .object({
    placement_id: z.string(),
    terminal_id: z.string(),
    title: z.string(),
    cwd: z.string(),
    shell: z.string(),
    cols: z.number().int().nonnegative(),
    rows: z.number().int().nonnegative(),
    state: workspaceTerminalStateSchema,
    exit_code: nullableNumber,
    created_at: z.string(),
    updated_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceTerminalSummary>;

export const workspaceTerminalOpenResponseSchema = z
  .object({
    placement_id: z.string(),
    terminal: workspaceTerminalSummarySchema,
    replay: z.array(workspaceTerminalOutputFrameSchema),
  })
  .strict() satisfies z.ZodType<WorkspaceTerminalOpenResponse>;

export const workspaceTerminalListResponseSchema = z
  .object({
    placement_id: z.string(),
    terminals: z.array(workspaceTerminalSummarySchema),
    generated_at: z.string(),
  })
  .strict() satisfies z.ZodType<WorkspaceTerminalListResponse>;

export const workspaceTerminalStreamFrameSchema = z.discriminatedUnion("kind", [
  z
    .object({
      kind: z.literal("output"),
      terminal_id: z.string(),
      seq: z.number().int().nonnegative(),
      data: z.string(),
      sent_at: z.string(),
    })
    .strict(),
  z
    .object({
      kind: z.literal("status"),
      terminal_id: z.string(),
      state: workspaceTerminalStateSchema,
      exit_code: nullableNumber,
      message: nullableString,
      sent_at: z.string(),
    })
    .strict(),
  z
    .object({
      kind: z.literal("pong"),
      sent_at: z.string(),
    })
    .strict(),
  z
    .object({
      kind: z.literal("error"),
      terminal_id: z.string(),
      message: z.string(),
      sent_at: z.string(),
    })
    .strict(),
]) satisfies z.ZodType<WorkspaceTerminalStreamFrame>;

export const eventEnvelopeSchema = z
  .object({
    event_id: z.string(),
    command_id: nullableString,
    correlation_id: nullableString.optional(),
    actor_ref: z.unknown(),
    scope_ref: z.unknown(),
    node_id: nullableString,
    runtime_session_id: nullableString,
    session_thread_id: nullableString,
    turn_id: nullableString,
    seq: z.number().int().nonnegative(),
    session_projection_seq: z
      .number()
      .int()
      .nonnegative()
      .nullable()
      .optional(),
    kind: eventKindSchema,
    happened_at: z.string(),
    source_refs: z.array(protocolRefSchema),
    evidence_refs: z.array(protocolRefSchema),
    cause_refs: z.array(protocolRefSchema),
    result_refs: z.array(protocolRefSchema),
    payload: eventPayloadSchema,
  })
  .strict()
  .superRefine((event, context) => {
    if (payloadTypeByEventKind[event.kind] !== event.payload.type) {
      context.addIssue({
        code: "custom",
        path: ["payload", "type"],
        message: `payload type ${event.payload.type} does not match event kind ${event.kind}`,
      });
    }
  }) satisfies z.ZodType<EventEnvelope>;

export function parseTerminalStreamFrame(
  value: unknown,
): WorkspaceTerminalStreamFrame | null {
  const parsed = typeof value === "string" ? parseJson(value) : value;
  if (parsed === null) {
    return null;
  }
  const result = workspaceTerminalStreamFrameSchema.safeParse(parsed);
  return result.success ? result.data : null;
}

export function parseProtocolPayload<T>(
  schema: ProtocolSchema<T>,
  value: unknown,
): T | null {
  const result = schema.safeParse(value);
  return result.success ? result.data : null;
}

export function formatProtocolIssues(issues: ProtocolValidationIssue[]) {
  return issues
    .slice(0, 5)
    .map((issue) => {
      const path =
        issue.path.length > 0 ? issue.path.map(String).join(".") : "<root>";
      return `${path}: ${issue.message}`;
    })
    .join("; ");
}

function parseJson(value: string): unknown | null {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return null;
  }
}
