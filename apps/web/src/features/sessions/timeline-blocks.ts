import type {
  CortexRef,
  EventEnvelope,
  Message,
  SessionDetail,
  UiBlock,
} from "../../shared/protocol/types";

export type TimelineBlockItem = {
  block: UiBlock;
  approvalId?: string;
};

export function buildSessionTimelineBlocks(
  detail: SessionDetail,
): TimelineBlockItem[] {
  const messageSourceEventIds = new Set(
    detail.messages
      .map((message) => message.source_event_id)
      .filter((eventId): eventId is string => typeof eventId === "string"),
  );
  const messageBlocks = detail.messages.map((message) => ({
    block: blockFromMessage(message, detail.events),
  }));
  const eventBlocks = detail.events
    .filter((event) => !messageSourceEventIds.has(event.event_id))
    .map((event) => blockFromEvent(event));

  return [...messageBlocks, ...eventBlocks];
}

export function blockFromMessage(
  message: Message,
  events: EventEnvelope[],
): UiBlock {
  const sourceEvent = message.source_event_id
    ? events.find((event) => event.event_id === message.source_event_id)
    : undefined;
  const sourceRefs = sourceEvent ? [eventRef(sourceEvent)] : [];
  const type =
    message.role === "assistant"
      ? "core.assistant-message"
      : message.role === "user"
        ? "core.user-message"
        : "core.runtime-event";

  return baseBlock({
    blockId: `message:${message.message_id}`,
    type,
    primaryRef: { kind: "message", message_id: message.message_id },
    sourceRefs,
    data: {
      role: message.role,
      content: message.content,
      createdAt: message.created_at,
      completedAt: message.completed_at,
    },
    fallbackText: message.content,
  });
}

export function blockFromEvent(event: EventEnvelope): TimelineBlockItem {
  const approvalId = approvalIdFromEvent(event);
  if (approvalId) {
    return {
      block: baseBlock({
        blockId: `event:${event.event_id}`,
        type: "core.approval-request",
        primaryRef: { kind: "approval", approval_id: approvalId },
        sourceRefs: [eventRef(event)],
        data: {
          approvalId,
          prompt: approvalPromptFromEvent(event),
          eventKind: event.kind,
          seq: event.seq,
          happenedAt: event.happened_at,
        },
        actions: ["approval.resolve"],
        fallbackText: approvalPromptFromEvent(event),
      }),
      approvalId,
    };
  }

  const type = blockTypeForEvent(event);
  const summary = payloadSummary(event.payload);

  return {
    block: baseBlock({
      blockId: `event:${event.event_id}`,
      type,
      primaryRef: primaryRefForEvent(event),
      sourceRefs: [eventRef(event), ...event.source_refs],
      evidenceRefs: event.evidence_refs,
      causeRefs: event.cause_refs,
      data: {
        eventKind: event.kind,
        seq: event.seq,
        happenedAt: event.happened_at,
        commandId: event.command_id,
        summary,
      },
      fallbackText: `${event.kind} seq ${event.seq}`,
    }),
  };
}

export function approvalIdFromEvent(event: EventEnvelope) {
  if (event.kind !== "approval.requested") return null;
  if (!isRecord(event.payload)) return null;
  const approvalId = event.payload.approval_id;
  return typeof approvalId === "string" ? approvalId : null;
}

function approvalPromptFromEvent(event: EventEnvelope) {
  if (!isRecord(event.payload)) return "Approval requested";
  const prompt = event.payload.prompt;
  return typeof prompt === "string" ? prompt : "Approval requested";
}

function blockTypeForEvent(event: EventEnvelope) {
  if (event.kind === "coordination.warning_acknowledged") {
    return "core.warning";
  }
  if (event.kind === "workspace.validated") {
    return "core.workspace-validation";
  }
  if (event.kind === "resource.snapshot.updated") {
    return "core.resource-snapshot";
  }
  if (event.kind.startsWith("provider.output.")) {
    return "core.provider-output-stream";
  }
  if (event.kind === "runtime.error") {
    return "core.error";
  }
  return "core.runtime-event";
}

function primaryRefForEvent(event: EventEnvelope): CortexRef {
  return eventRef(event);
}

function eventRef(event: EventEnvelope): CortexRef {
  return {
    kind: "event",
    event_id: event.event_id,
    scope_ref: event.scope_ref,
    seq: event.seq,
  };
}

function baseBlock({
  blockId,
  type,
  primaryRef,
  data,
  parentRef = null,
  sourceRefs = [],
  evidenceRefs = [],
  causeRefs = [],
  relatedRefs = [],
  traceRefs = [],
  actions = [],
  fallbackText = null,
}: {
  blockId: string;
  type: string;
  primaryRef: CortexRef;
  data: unknown;
  parentRef?: CortexRef | null;
  sourceRefs?: CortexRef[];
  evidenceRefs?: CortexRef[];
  causeRefs?: CortexRef[];
  relatedRefs?: CortexRef[];
  traceRefs?: CortexRef[];
  actions?: string[];
  fallbackText?: string | null;
}): UiBlock {
  return {
    block_id: blockId,
    type,
    schema_version: 1,
    surface_id: "session.timeline",
    primary_ref: primaryRef,
    parent_ref: parentRef,
    children: [],
    source_refs: sourceRefs,
    evidence_refs: evidenceRefs,
    cause_refs: causeRefs,
    related_refs: relatedRefs,
    trace_refs: traceRefs,
    data,
    actions,
    fallback_text: fallbackText,
  };
}

function payloadSummary(payload: unknown) {
  if (!isRecord(payload)) return "No structured payload";
  const message = payload.message;
  if (typeof message === "string" && message.length > 0) return message;
  const content = payload.content;
  if (typeof content === "string" && content.length > 0) return content;
  const code = payload.code;
  if (typeof code === "string" && code.length > 0) return code;
  const keys = Object.keys(payload);
  return keys.length > 0 ? keys.slice(0, 4).join(", ") : "No payload fields";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
