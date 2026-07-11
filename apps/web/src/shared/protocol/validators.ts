import { z } from "zod";

import {
  COMMAND_KIND_VALUES,
  COMMAND_STATE_VALUES,
  EVENT_KIND_VALUES,
  MESSAGE_ROLE_VALUES,
  PLACEMENT_STATE_VALUES,
  WARNING_SEVERITY_VALUES,
  WORKSPACE_COMMAND_INTENT_VALUES,
  WORKSPACE_TERMINAL_STATE_VALUES,
} from "./literals";
import type {
  CommandAcceptedResponse,
  CommandKind,
  CommandState,
  EventEnvelope,
  EventKind,
  EventPayload,
  MessageRole,
  WorkspaceCommandHistoryItem,
  WorkspaceCommandHistoryResponse,
  WorkspaceCommandIntent,
  WorkspaceCommandRunResponse,
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
      type: z.literal("extension"),
      name: z.string(),
      value: z.unknown(),
    })
    .strict(),
]) satisfies z.ZodType<EventPayload>;

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
