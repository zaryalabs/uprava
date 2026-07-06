import type {
  EventEnvelope,
  Message,
  MessageRole,
  PlacementState,
  ResourceBadge,
  RuntimeSessionState,
  SessionDetail,
  SessionThreadState,
} from "../../shared/protocol/types";

export type ProjectionApplyResult =
  | { kind: "applied"; detail: SessionDetail }
  | { kind: "gap"; expectedSeq: number; receivedSeq: number };

export function applySessionEvent(
  detail: SessionDetail,
  event: EventEnvelope,
): ProjectionApplyResult {
  const currentSeq = detail.events.reduce(
    (max, candidate) => Math.max(max, sessionEventCursor(candidate)),
    0,
  );
  const receivedSeq = sessionEventCursor(event);
  if (receivedSeq !== currentSeq + 1) {
    return {
      kind: "gap",
      expectedSeq: currentSeq + 1,
      receivedSeq,
    };
  }
  return {
    kind: "applied",
    detail: applyContiguousEvent(detail, event),
  };
}

export function sessionEventCursor(event: EventEnvelope): number {
  return event.session_projection_seq ?? event.seq;
}

function applyContiguousEvent(
  detail: SessionDetail,
  event: EventEnvelope,
): SessionDetail {
  const messages = appendEventMessage(detail.messages, event);
  const runtimeState = runtimeStateForEvent(event.kind);
  const sessionState = sessionStateForEvent(event.kind, detail.session.state);
  const placementUpdate = placementUpdateFromEvent(event);

  return {
    ...detail,
    placement: placementUpdate
      ? {
          ...detail.placement,
          ...placementUpdate,
        }
      : detail.placement,
    session: {
      ...detail.session,
      state: sessionState ?? detail.session.state,
      message_count: messages.length,
      updated_at: event.happened_at,
      runtime: {
        ...detail.session.runtime,
        state: runtimeState ?? detail.session.runtime.state,
        last_runtime_step_at: event.happened_at,
      },
    },
    messages,
    events: [...detail.events, event],
  };
}

function appendEventMessage(
  messages: Message[],
  event: EventEnvelope,
): Message[] {
  if (messages.some((message) => message.source_event_id === event.event_id)) {
    return messages;
  }
  const message = messageFromEvent(event);
  return message ? [...messages, message] : messages;
}

function messageFromEvent(event: EventEnvelope): Message | null {
  const role = messageRoleForEvent(event.kind);
  if (!role || !event.session_thread_id) return null;

  return {
    message_id: `event-message:${event.event_id}`,
    session_thread_id: event.session_thread_id,
    turn_id: event.turn_id,
    role,
    content: messageContentForEvent(event),
    created_at: event.happened_at,
    completed_at: event.happened_at,
    source_event_id: event.event_id,
  };
}

function messageRoleForEvent(kind: string): MessageRole | null {
  if (kind === "provider.message.completed") return "assistant";
  if (kind === "approval.requested" || kind === "approval.resolved") {
    return "approval";
  }
  if (kind === "runtime.error") return "runtime";
  return null;
}

function messageContentForEvent(event: EventEnvelope) {
  const payload = eventPayload(event);
  if (event.kind === "approval.requested") {
    return stringField(payload, "prompt", "Approval requested");
  }
  if (event.kind === "approval.resolved") {
    return stringField(payload, "message", "Approval resolved");
  }
  if (event.kind === "runtime.error") {
    return stringField(payload, "message", "Runtime error");
  }
  return stringField(payload, "content", "Provider completed a message");
}

function runtimeStateForEvent(kind: string): RuntimeSessionState | null {
  switch (kind) {
    case "runtime.starting":
      return "starting";
    case "runtime.ready":
      return "ready";
    case "runtime.running":
    case "turn.started":
      return "running";
    case "runtime.blocked":
    case "approval.requested":
      return "blocked";
    case "runtime.expired":
      return "expired";
    case "runtime.resuming":
      return "resuming";
    case "runtime.stopped":
      return "stopped";
    case "runtime.error":
      return "error";
    case "turn.interrupted":
      return "interrupted";
    default:
      return null;
  }
}

function sessionStateForEvent(
  kind: string,
  current: SessionThreadState,
): SessionThreadState | null {
  if (kind === "runtime.stopped") return "stopped";
  if (kind === "runtime.error") return "degraded";
  if (current === "detached") return "detached";
  if (
    kind === "runtime.ready" ||
    kind === "runtime.running" ||
    kind === "approval.requested" ||
    kind === "approval.resolved" ||
    kind === "turn.started" ||
    kind === "turn.completed" ||
    kind === "turn.interrupted"
  ) {
    return "active";
  }
  return null;
}

function placementUpdateFromEvent(
  event: EventEnvelope,
): Partial<SessionDetail["placement"]> | null {
  if (
    event.kind !== "workspace.validated" &&
    event.kind !== "resource.snapshot.updated"
  ) {
    return null;
  }

  const payload = eventPayload(event);
  const update: Partial<SessionDetail["placement"]> = {
    last_validated_at: event.happened_at,
  };
  const displayName = stringField(payload, "display_name", undefined);
  const workspacePath = stringField(payload, "workspace_path", undefined);
  const state = placementStateField(payload, "state");
  const resourceBadges = resourceBadgesField(payload, "resource_badges");

  if (displayName) update.display_name = displayName;
  if (workspacePath) update.workspace_path = workspacePath;
  if (state) update.state = state;
  if (resourceBadges) update.resource_badges = resourceBadges;

  return update;
}

function eventPayload(event: EventEnvelope): Record<string, unknown> {
  return typeof event.payload === "object" && event.payload !== null
    ? (event.payload as Record<string, unknown>)
    : {};
}

function stringField(
  payload: Record<string, unknown>,
  field: string,
  fallback: string,
): string;
function stringField(
  payload: Record<string, unknown>,
  field: string,
  fallback: undefined,
): string | undefined;
function stringField(
  payload: Record<string, unknown>,
  field: string,
  fallback: string | undefined,
) {
  const value = payload[field];
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

function placementStateField(
  payload: Record<string, unknown>,
  field: string,
): PlacementState | undefined {
  const value = payload[field];
  if (
    value === "pending" ||
    value === "validated" ||
    value === "missing" ||
    value === "read_only" ||
    value === "error"
  ) {
    return value;
  }
  return undefined;
}

function resourceBadgesField(
  payload: Record<string, unknown>,
  field: string,
): ResourceBadge[] | undefined {
  const value = payload[field];
  if (!Array.isArray(value)) return undefined;
  return value.filter(isResourceBadge);
}

function isResourceBadge(value: unknown): value is ResourceBadge {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.kind === "string" &&
    typeof candidate.label === "string" &&
    (candidate.severity === "info" ||
      candidate.severity === "warning" ||
      candidate.severity === "hard_block")
  );
}
