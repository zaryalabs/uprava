import type {
  EventEnvelope,
  Message,
  SessionDetail,
  UpravaRef,
} from "../../shared/protocol/types";
import type { UiBlock } from "../../workbench/blocks/types";

export type TimelineBlockItem = {
  block: UiBlock;
  approvalId?: string;
};

type OrderedTimelineBlockItem = {
  item: TimelineBlockItem;
  timestamp: string;
  seq: number;
  sourceIndex: number;
};

export function buildSessionTimelineBlocks(
  detail: SessionDetail,
): TimelineBlockItem[] {
  const eventById = new Map(
    detail.events.map((event) => [event.event_id, event]),
  );
  const activityGroups = turnActivityGroups(detail.events);
  const groupedActivityEventIds = new Set(
    activityGroups.flatMap((group) =>
      group.events.map((event) => event.event_id),
    ),
  );
  const groupedActivityTurnIds = new Set(
    activityGroups.map((group) => group.turnId),
  );
  const pendingApprovals = pendingApprovalIds(detail.events);
  const messageSourceEventIds = new Set(
    detail.messages
      .map((message) => message.source_event_id)
      .filter((eventId): eventId is string => typeof eventId === "string"),
  );
  const messageBlocks = detail.messages.map((message, sourceIndex) => {
    const sourceEvent = message.source_event_id
      ? eventById.get(message.source_event_id)
      : undefined;
    return orderedTimelineBlockItem(
      { block: blockFromMessage(message, detail.events) },
      sourceEvent?.happened_at ?? message.created_at,
      sourceEvent?.seq ?? 0,
      sourceIndex,
    );
  });
  const activityBlocks = activityGroups.map((group, sourceIndex) =>
    orderedTimelineBlockItem(
      { block: blockFromTurnActivity(group, detail.events) },
      group.events[0]?.happened_at ?? group.startedAt ?? "",
      group.events[0]?.seq ?? 0,
      detail.messages.length + sourceIndex,
    ),
  );
  const eventBlocks = detail.events
    .filter(
      (event) =>
        !messageSourceEventIds.has(event.event_id) &&
        !groupedActivityEventIds.has(event.event_id) &&
        !isGroupedTurnBoundaryEvent(event, groupedActivityTurnIds),
    )
    .map((event, sourceIndex) =>
      orderedTimelineBlockItem(
        blockFromEvent(event, { pendingApprovals }),
        event.happened_at,
        event.seq,
        detail.messages.length + activityBlocks.length + sourceIndex,
      ),
    );

  return [...messageBlocks, ...activityBlocks, ...eventBlocks]
    .sort(compareTimelineBlockItems)
    .map(({ item }) => item);
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

export function blockFromEvent(
  event: EventEnvelope,
  options: { pendingApprovals?: Set<string> } = {},
): TimelineBlockItem {
  const approvalId = approvalIdFromEvent(event);
  if (approvalId) {
    const pending = options.pendingApprovals?.has(approvalId) ?? true;
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
          state: pending ? "pending" : "resolved",
        },
        actions: pending ? ["approval.resolve"] : [],
        fallbackText: approvalPromptFromEvent(event),
      }),
      approvalId: pending ? approvalId : undefined,
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

type TurnActivityGroup = {
  turnId: string;
  startedAt?: string;
  completedAt?: string;
  events: EventEnvelope[];
};

function turnActivityGroups(events: EventEnvelope[]): TurnActivityGroup[] {
  const byTurn = new Map<string, EventEnvelope[]>();
  for (const event of events) {
    if (event.kind !== "provider.activity" || !event.turn_id) continue;
    const group = byTurn.get(event.turn_id) ?? [];
    group.push(event);
    byTurn.set(event.turn_id, group);
  }

  return Array.from(byTurn.entries())
    .map(([turnId, groupEvents]) => {
      const sortedEvents = [...groupEvents].sort(compareEvents);
      return {
        turnId,
        startedAt: eventTime(events, turnId, "turn.started"),
        completedAt: eventTime(events, turnId, "turn.completed"),
        events: sortedEvents,
      };
    })
    .sort((left, right) => compareEvents(left.events[0]!, right.events[0]!));
}

function blockFromTurnActivity(
  group: TurnActivityGroup,
  allEvents: EventEnvelope[],
): UiBlock {
  const rows = group.events.map(turnActivityRow);
  const counts = turnActivityCounts(rows);
  const durationMs =
    group.startedAt && group.completedAt
      ? new Date(group.completedAt).getTime() -
        new Date(group.startedAt).getTime()
      : null;

  return baseBlock({
    blockId: `turn-activity:${group.turnId}`,
    type: "core.turn-activity",
    primaryRef: { kind: "turn", turn_id: group.turnId },
    sourceRefs: group.events.map(eventRef),
    causeRefs: allEvents
      .filter(
        (event) =>
          event.turn_id === group.turnId &&
          (event.kind === "turn.started" || event.kind === "turn.completed"),
      )
      .map(eventRef),
    data: {
      turnId: group.turnId,
      startedAt: group.startedAt ?? null,
      completedAt: group.completedAt ?? null,
      completed: Boolean(group.completedAt),
      durationMs: durationMs !== null && durationMs >= 0 ? durationMs : null,
      eventCount: rows.length,
      commandCount: counts.commandCount,
      fileChangeCount: counts.fileChangeCount,
      reasoningCount: counts.reasoningCount,
      warningErrorCount: counts.warningErrorCount,
      rows,
    },
    fallbackText: `${rows.length} provider activity events`,
  });
}

function isGroupedTurnBoundaryEvent(
  event: EventEnvelope,
  groupedActivityTurnIds: Set<string>,
) {
  return Boolean(
    event.turn_id &&
    groupedActivityTurnIds.has(event.turn_id) &&
    (event.kind === "turn.started" || event.kind === "turn.completed"),
  );
}

function turnActivityRow(event: EventEnvelope) {
  const payload =
    event.payload.type === "provider_activity" ? event.payload : null;
  const providerEventType = stringValue(
    payload?.provider_event_type,
    "provider.activity",
  );
  const providerItemType = optionalString(payload?.provider_item_type);
  const providerItemId = optionalString(payload?.provider_item_id);
  const phase = optionalString(payload?.phase);
  const status = optionalString(payload?.status);
  const summary = stringValue(payload?.summary, providerEventType);

  return {
    eventId: event.event_id,
    seq: event.seq,
    happenedAt: event.happened_at,
    providerEventType,
    providerItemType,
    providerItemId,
    phase,
    status,
    summary,
    rawEvent: payload?.raw_event,
    rawEventPreview: optionalString(payload?.raw_event_preview),
    rawEventTruncated: payload?.raw_event_truncated === true,
  };
}

function turnActivityCounts(rows: ReturnType<typeof turnActivityRow>[]) {
  return rows.reduce(
    (counts, row) => {
      const searchable = [
        row.providerEventType,
        row.providerItemType,
        row.phase,
        row.status,
        row.summary,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      if (
        searchable.includes("command") ||
        searchable.includes("tool") ||
        searchable.includes("exec")
      ) {
        counts.commandCount += 1;
      }
      if (
        searchable.includes("file") ||
        searchable.includes("patch") ||
        searchable.includes("diff")
      ) {
        counts.fileChangeCount += 1;
      }
      if (searchable.includes("reason")) counts.reasoningCount += 1;
      if (
        searchable.includes("warn") ||
        searchable.includes("error") ||
        searchable.includes("failed") ||
        searchable.includes("parse_error") ||
        searchable.includes("stderr")
      ) {
        counts.warningErrorCount += 1;
      }
      return counts;
    },
    {
      commandCount: 0,
      fileChangeCount: 0,
      reasoningCount: 0,
      warningErrorCount: 0,
    },
  );
}

export function approvalIdFromEvent(event: EventEnvelope) {
  if (event.kind !== "approval.requested") return null;
  return approvalIdFromPayload(event.payload);
}

function approvalIdFromResolutionEvent(event: EventEnvelope) {
  if (event.kind !== "approval.resolved") return null;
  return approvalIdFromPayload(event.payload);
}

function approvalIdFromPayload(payload: unknown) {
  if (!isRecord(payload)) return null;
  const approvalId = payload.approval_id;
  return typeof approvalId === "string" ? approvalId : null;
}

function pendingApprovalIds(events: EventEnvelope[]) {
  const pending = new Set<string>();
  for (const event of [...events].sort(compareEvents)) {
    const requestedId = approvalIdFromEvent(event);
    if (requestedId) {
      pending.add(requestedId);
      continue;
    }
    const resolvedId = approvalIdFromResolutionEvent(event);
    if (resolvedId) {
      pending.delete(resolvedId);
    }
  }
  return pending;
}

function approvalPromptFromEvent(event: EventEnvelope) {
  return event.payload.type === "approval_requested" && event.payload.prompt
    ? event.payload.prompt
    : "Approval requested";
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

function primaryRefForEvent(event: EventEnvelope): UpravaRef {
  return eventRef(event);
}

function eventRef(event: EventEnvelope): UpravaRef {
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
  primaryRef: UpravaRef;
  data: unknown;
  parentRef?: UpravaRef | null;
  sourceRefs?: UpravaRef[];
  evidenceRefs?: UpravaRef[];
  causeRefs?: UpravaRef[];
  relatedRefs?: UpravaRef[];
  traceRefs?: UpravaRef[];
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

function optionalString(value: unknown) {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function stringValue(value: unknown, fallback: string) {
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

function eventTime(events: EventEnvelope[], turnId: string, kind: string) {
  return events.find((event) => event.turn_id === turnId && event.kind === kind)
    ?.happened_at;
}

function compareEvents(left: EventEnvelope, right: EventEnvelope) {
  const timestampComparison = left.happened_at.localeCompare(right.happened_at);
  if (timestampComparison !== 0) return timestampComparison;
  return left.seq - right.seq;
}

function orderedTimelineBlockItem(
  item: TimelineBlockItem,
  timestamp: string,
  seq: number,
  sourceIndex: number,
): OrderedTimelineBlockItem {
  return { item, timestamp, seq, sourceIndex };
}

function compareTimelineBlockItems(
  left: OrderedTimelineBlockItem,
  right: OrderedTimelineBlockItem,
) {
  const timestampComparison = left.timestamp.localeCompare(right.timestamp);
  if (timestampComparison !== 0) return timestampComparison;
  const seqComparison = left.seq - right.seq;
  if (seqComparison !== 0) return seqComparison;
  return left.sourceIndex - right.sourceIndex;
}
