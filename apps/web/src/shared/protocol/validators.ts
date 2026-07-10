import { z } from "zod";

import {
  COMMAND_KIND_VALUES,
  COMMAND_STATE_VALUES,
  MESSAGE_ROLE_VALUES,
  WORKSPACE_COMMAND_INTENT_VALUES,
  WORKSPACE_TERMINAL_STATE_VALUES,
} from "./literals";
import type {
  CommandAcceptedResponse,
  CommandKind,
  CommandState,
  EventEnvelope,
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
    kind: z.string(),
    happened_at: z.string(),
    source_refs: z.array(protocolRefSchema),
    evidence_refs: z.array(protocolRefSchema),
    cause_refs: z.array(protocolRefSchema),
    result_refs: z.array(protocolRefSchema),
    payload: z.unknown(),
  })
  .strict() satisfies z.ZodType<EventEnvelope>;

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
