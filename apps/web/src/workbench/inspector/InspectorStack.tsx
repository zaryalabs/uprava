import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronRight, X } from "lucide-react";
import { useParams, useSearchParams } from "react-router-dom";

import { useInventory } from "../../features/inventory/api";
import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  ArtifactTree,
  ArtifactTreeNode,
  CortexRef,
  EventEnvelope,
  InventorySnapshot,
  Message,
  ResourceBadge,
  SessionDetail,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { runWorkbenchCommand } from "../commands/registry";
import { ReferenceActions } from "../references/ReferenceActions";
import {
  decodeInspectorStack,
  INSPECT_QUERY_PARAM,
  popInspectorRef,
  refKindLabel,
  refPrimaryValue,
  refTitle,
  replaceInspectorStack,
} from "../references/refs";
import { useOpenReference } from "../references/use-inspector-stack";

type InspectorStatus = "resolved" | "not_available" | "not_implemented";

type InspectorRow = {
  label: string;
  value: string | number | boolean | null | undefined;
};

type InspectorRefLink = {
  label: string;
  ref: CortexRef;
};

type InspectorDetail = {
  title: string;
  status: InspectorStatus;
  rows: InspectorRow[];
  refs: InspectorRefLink[];
  payload?: unknown;
};

type InspectorContext = {
  inventory?: InventorySnapshot;
  session?: SessionDetail;
  artifactTree?: ArtifactTree;
};

const reservedFutureKinds = new Set([
  "file",
  "file_range",
  "terminal",
  "terminal_command",
  "terminal_output_range",
  "diff_hunk",
  "check_result",
  "tool_call",
  "external_entity",
]);

export function InspectorStack() {
  const [searchParams, setSearchParams] = useSearchParams();
  const openReference = useOpenReference();
  const { sessionThreadId } = useParams();
  const stack = decodeInspectorStack(searchParams.get(INSPECT_QUERY_PARAM));
  const selected = stack.at(-1) ?? null;
  const inventory = useInventory();
  const session = useQuery({
    queryKey: queryKeys.session(sessionThreadId ?? ""),
    queryFn: () => coreApi.session(sessionThreadId ?? ""),
    enabled: Boolean(selected && sessionThreadId),
  });
  const artifactTree = useQuery({
    queryKey: queryKeys.artifactTree(sessionThreadId ?? ""),
    queryFn: () => coreApi.artifactTree(sessionThreadId ?? ""),
    enabled: Boolean(
      selected && sessionThreadId && selected.kind === "artifact",
    ),
  });

  useEffect(() => {
    if (stack.length === 0) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      setSearchParams((current) => popInspectorRef(current));
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [setSearchParams, stack.length]);

  const detail = selected
    ? buildInspectorDetail(selected, {
        inventory: inventory.data,
        session: session.data,
        artifactTree: artifactTree.data,
      })
    : null;

  const closeTop = () => setSearchParams((current) => popInspectorRef(current));
  const selectStackIndex = (index: number) => {
    setSearchParams((current) =>
      replaceInspectorStack(current, stack.slice(0, index + 1)),
    );
  };

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
          Inspector
        </h2>
        {selected ? (
          <Button
            type="button"
            variant="ghost"
            className="h-7 w-7 px-0"
            aria-label="Close inspector"
            title="Close inspector"
            onClick={closeTop}
          >
            <X size={14} />
          </Button>
        ) : null}
      </div>

      {stack.length > 0 ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1 text-xs">
          {stack.map((ref, index) => (
            <button
              key={`${index}:${refTitle(ref)}`}
              type="button"
              className={`inline-flex max-w-full items-center gap-1 rounded-md border px-2 py-1 text-left ${
                index === stack.length - 1
                  ? "border-[#9ccdbd] bg-[#e4f4ef] text-[#1f6559]"
                  : "border-[#d9ded4] bg-white text-[#536257]"
              }`}
              onClick={() => selectStackIndex(index)}
            >
              {index > 0 ? <ChevronRight size={12} /> : null}
              <span className="truncate">{refKindLabel(ref)}</span>
            </button>
          ))}
        </div>
      ) : null}

      {selected && detail ? (
        <article className="rounded-md border border-[#d9ded4] bg-white p-3">
          <div className="mb-3 flex items-start justify-between gap-2">
            <div className="min-w-0">
              <Badge tone={statusTone(detail.status)}>{detail.status}</Badge>
              <h3 className="mt-2 truncate text-sm font-semibold">
                {detail.title}
              </h3>
            </div>
            <ReferenceActions reference={selected} showInspect={false} />
          </div>
          <dl className="space-y-2">
            {detail.rows.map((row) => (
              <div key={row.label} className="min-w-0">
                <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
                  {row.label}
                </dt>
                <dd className="break-words font-mono text-xs text-[#27362f]">
                  {formatValue(row.value)}
                </dd>
              </div>
            ))}
          </dl>
          {detail.refs.length > 0 ? (
            <div className="mt-4 space-y-2">
              <div className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
                References
              </div>
              <div className="flex flex-wrap gap-1.5">
                {detail.refs.map((item, index) => (
                  <button
                    key={`${index}:${refTitle(item.ref)}`}
                    type="button"
                    className="max-w-full truncate rounded-md border border-[#d9ded4] bg-[#f8faf5] px-2 py-1 text-left text-xs hover:bg-[#edf1e9]"
                    title={refTitle(item.ref)}
                    onClick={() => {
                      void runWorkbenchCommand("reference.openInInspector", {
                        reference: item.ref,
                        openReference,
                      });
                    }}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {detail.payload !== undefined ? (
            <pre className="mt-4 max-h-64 overflow-auto rounded-md border border-[#e0e5dc] bg-[#f8faf5] p-2 text-xs text-[#27362f]">
              {safeJson(detail.payload)}
            </pre>
          ) : null}
        </article>
      ) : (
        <div className="rounded-md border border-[#d9ded4] bg-white p-3 text-sm text-[#536257]">
          No reference selected.
        </div>
      )}
    </section>
  );
}

function buildInspectorDetail(
  ref: CortexRef,
  context: InspectorContext,
): InspectorDetail {
  switch (ref.kind) {
    case "node":
      return nodeDetail(ref as Extract<CortexRef, { kind: "node" }>, context);
    case "placement":
      return placementDetail(
        ref as Extract<CortexRef, { kind: "placement" }>,
        context,
      );
    case "session":
      return sessionDetail(
        ref as Extract<CortexRef, { kind: "session" }>,
        context,
      );
    case "runtime":
      return runtimeDetail(
        ref as Extract<CortexRef, { kind: "runtime" }>,
        context,
      );
    case "turn":
      return turnDetail(ref as Extract<CortexRef, { kind: "turn" }>, context);
    case "message":
      return messageDetail(
        ref as Extract<CortexRef, { kind: "message" }>,
        context,
      );
    case "artifact":
      return artifactDetail(
        ref as Extract<CortexRef, { kind: "artifact" }>,
        context,
      );
    case "event":
      return eventDetail(ref as Extract<CortexRef, { kind: "event" }>, context);
    case "command":
      return commandDetail(
        ref as Extract<CortexRef, { kind: "command" }>,
        context,
      );
    case "approval":
      return approvalDetail(
        ref as Extract<CortexRef, { kind: "approval" }>,
        context,
      );
    case "warning":
      return warningDetail(
        ref as Extract<CortexRef, { kind: "warning" }>,
        context,
      );
    case "unknown":
      return {
        title: refTitle(ref),
        status: "not_available",
        rows: [{ label: "type", value: refPrimaryValue(ref) }],
        refs: [],
        payload:
          "locator" in ref && typeof ref.locator === "object"
            ? ref.locator
            : ref,
      };
    default:
      if (reservedFutureKinds.has(ref.kind)) {
        return {
          title: refTitle(ref),
          status: "not_implemented",
          rows: [
            { label: "kind", value: ref.kind },
            { label: "state", value: "reserved for a future workbench slice" },
          ],
          refs: [],
          payload: ref,
        };
      }
      return notAvailable(ref, "No V01 inspector resolver for this ref kind");
  }
}

function nodeDetail(
  ref: Extract<CortexRef, { kind: "node" }>,
  context: InspectorContext,
): InspectorDetail {
  const node = context.inventory?.nodes.find(
    (candidate) => candidate.node_id === ref.node_id,
  );
  if (!node) return notAvailable(ref, "Node snapshot is not loaded");
  const placements =
    context.inventory?.placements.filter(
      (placement) => placement.node_id === node.node_id,
    ) ?? [];
  return {
    title: node.display_name,
    status: "resolved",
    rows: [
      { label: "node id", value: node.node_id },
      { label: "presence", value: node.presence },
      { label: "heartbeat age", value: node.heartbeat_age_seconds },
      { label: "sleep hint", value: node.sleep_hint },
      { label: "active runtimes", value: node.active_runtime_count },
      { label: "diagnostics", value: node.diagnostics },
    ],
    refs: placements.map((placement) => ({
      label: placement.display_name,
      ref: { kind: "placement", placement_id: placement.project_placement_id },
    })),
    payload: node.capabilities,
  };
}

function placementDetail(
  ref: Extract<CortexRef, { kind: "placement" }>,
  context: InspectorContext,
): InspectorDetail {
  const placement =
    context.session?.placement.project_placement_id === ref.placement_id
      ? context.session.placement
      : context.inventory?.placements.find(
          (candidate) => candidate.project_placement_id === ref.placement_id,
        );
  if (!placement) return notAvailable(ref, "Placement snapshot is not loaded");
  const sessions =
    context.inventory?.sessions.filter(
      (session) =>
        session.project_placement_id === placement.project_placement_id,
    ) ?? [];
  return {
    title: placement.display_name,
    status: "resolved",
    rows: [
      { label: "placement id", value: placement.project_placement_id },
      { label: "workspace path", value: placement.workspace_path },
      { label: "state", value: placement.state },
      { label: "node id", value: placement.node_id },
      { label: "last validated", value: placement.last_validated_at },
    ],
    refs: [
      { label: "node", ref: { kind: "node", node_id: placement.node_id } },
      ...sessions.map((session) => ({
        label: session.title,
        ref: {
          kind: "session" as const,
          session_thread_id: session.session_thread_id,
        },
      })),
    ],
    payload: placement.resource_badges,
  };
}

function sessionDetail(
  ref: Extract<CortexRef, { kind: "session" }>,
  context: InspectorContext,
): InspectorDetail {
  const session =
    context.session?.session.session_thread_id === ref.session_thread_id
      ? context.session.session
      : context.inventory?.sessions.find(
          (candidate) => candidate.session_thread_id === ref.session_thread_id,
        );
  if (!session) return notAvailable(ref, "Session snapshot is not loaded");
  const recentEvents = context.session?.events.slice(-5) ?? [];
  return {
    title: session.title,
    status: "resolved",
    rows: [
      { label: "session id", value: session.session_thread_id },
      { label: "state", value: session.state },
      { label: "runtime state", value: session.runtime.state },
      { label: "provider", value: session.runtime.provider },
      { label: "messages", value: session.message_count },
      { label: "updated", value: session.updated_at },
    ],
    refs: [
      {
        label: "placement",
        ref: {
          kind: "placement",
          placement_id: session.project_placement_id,
        },
      },
      {
        label: "runtime",
        ref: {
          kind: "runtime",
          runtime_session_id: session.runtime_session_id,
        },
      },
      ...recentEvents.map((event) => ({
        label: `event ${event.seq}`,
        ref: eventRef(event),
      })),
    ],
    payload: context.session
      ? {
          messages: context.session.messages.length,
          events: context.session.events.length,
        }
      : undefined,
  };
}

function runtimeDetail(
  ref: Extract<CortexRef, { kind: "runtime" }>,
  context: InspectorContext,
): InspectorDetail {
  const session =
    context.session?.session.runtime_session_id === ref.runtime_session_id
      ? context.session.session
      : context.inventory?.sessions.find(
          (candidate) =>
            candidate.runtime_session_id === ref.runtime_session_id,
        );
  const runtime = session?.runtime;
  if (!runtime) return notAvailable(ref, "Runtime snapshot is not loaded");
  return {
    title: runtime.runtime_session_id,
    status: "resolved",
    rows: [
      { label: "provider", value: runtime.provider },
      { label: "state", value: runtime.state },
      { label: "resume supported", value: runtime.resume_supported },
      { label: "degraded reason", value: runtime.degraded_reason },
      { label: "last step", value: runtime.last_runtime_step_at },
    ],
    refs: session
      ? [
          {
            label: "session",
            ref: {
              kind: "session",
              session_thread_id: session.session_thread_id,
            },
          },
        ]
      : [],
  };
}

function turnDetail(
  ref: Extract<CortexRef, { kind: "turn" }>,
  context: InspectorContext,
): InspectorDetail {
  const messages =
    context.session?.messages.filter(
      (message) => message.turn_id === ref.turn_id,
    ) ?? [];
  const events =
    context.session?.events.filter((event) => event.turn_id === ref.turn_id) ??
    [];
  if (messages.length === 0 && events.length === 0) {
    return notAvailable(ref, "Turn is not present in the current session");
  }
  return {
    title: ref.turn_id,
    status: "resolved",
    rows: [
      { label: "messages", value: messages.length },
      { label: "events", value: events.length },
    ],
    refs: [
      ...messages.map((message) => ({
        label: message.role,
        ref: { kind: "message" as const, message_id: message.message_id },
      })),
      ...events.map((event) => ({
        label: `event ${event.seq}`,
        ref: eventRef(event),
      })),
    ],
  };
}

function messageDetail(
  ref: Extract<CortexRef, { kind: "message" }>,
  context: InspectorContext,
): InspectorDetail {
  const message = context.session?.messages.find(
    (candidate) => candidate.message_id === ref.message_id,
  );
  if (!message) return notAvailable(ref, "Message is not loaded locally");
  return {
    title: `${message.role} message`,
    status: "resolved",
    rows: [
      { label: "message id", value: message.message_id },
      { label: "role", value: message.role },
      { label: "turn", value: message.turn_id },
      { label: "created", value: message.created_at },
      { label: "completed", value: message.completed_at },
      { label: "source event", value: message.source_event_id },
    ],
    refs: messageRefs(message),
    payload: { content: message.content },
  };
}

function artifactDetail(
  ref: Extract<CortexRef, { kind: "artifact" }>,
  context: InspectorContext,
): InspectorDetail {
  const node = context.artifactTree
    ? findArtifactNode(context.artifactTree.root, ref.artifact_id)
    : null;
  if (!node) return notAvailable(ref, "Artifact is not in the current tree");
  return {
    title: node.label,
    status: "resolved",
    rows: [
      { label: "artifact id", value: node.artifact_id },
      { label: "children", value: node.children.length },
      { label: "primary ref", value: refTitle(node.primary_ref) },
    ],
    refs: [
      { label: "primary", ref: node.primary_ref },
      ...refLinks("source", node.source_refs),
      ...refLinks("evidence", node.evidence_refs),
      ...refLinks("cause", node.cause_refs),
    ],
  };
}

function eventDetail(
  ref: Extract<CortexRef, { kind: "event" }>,
  context: InspectorContext,
): InspectorDetail {
  const event = context.session?.events.find(
    (candidate) => candidate.event_id === ref.event_id,
  );
  if (!event) return notAvailable(ref, "Event is not loaded locally");
  return {
    title: event.kind,
    status: "resolved",
    rows: [
      { label: "event id", value: event.event_id },
      { label: "seq", value: event.seq },
      { label: "happened at", value: event.happened_at },
      { label: "command", value: event.command_id },
      { label: "actor", value: safeInlineJson(event.actor_ref) },
      { label: "node", value: event.node_id },
      { label: "runtime", value: event.runtime_session_id },
      { label: "session", value: event.session_thread_id },
      { label: "turn", value: event.turn_id },
    ],
    refs: [
      ...(event.command_id
        ? [
            {
              label: "command",
              ref: {
                kind: "command" as const,
                command_id: event.command_id,
              },
            },
          ]
        : []),
      ...eventIdentityRefs(event),
      ...refLinks("source", event.source_refs),
      ...refLinks("evidence", event.evidence_refs),
      ...refLinks("cause", event.cause_refs),
      ...refLinks("result", event.result_refs),
    ],
    payload: event.payload,
  };
}

function commandDetail(
  ref: Extract<CortexRef, { kind: "command" }>,
  context: InspectorContext,
): InspectorDetail {
  const events =
    context.session?.events.filter(
      (event) => event.command_id === ref.command_id,
    ) ?? [];
  if (events.length === 0) {
    return notAvailable(ref, "Command details are not loaded locally");
  }
  return {
    title: ref.command_id,
    status: "resolved",
    rows: [
      { label: "command id", value: ref.command_id },
      { label: "status", value: "observed in event log" },
      { label: "events", value: events.length },
      { label: "first kind", value: events[0]?.kind },
      { label: "first timestamp", value: events[0]?.happened_at },
      { label: "last timestamp", value: events.at(-1)?.happened_at },
    ],
    refs: events.map((event) => ({
      label: `event ${event.seq}`,
      ref: eventRef(event),
    })),
    payload: events.map((event) => ({
      kind: event.kind,
      seq: event.seq,
      payload: event.payload,
    })),
  };
}

function approvalDetail(
  ref: Extract<CortexRef, { kind: "approval" }>,
  context: InspectorContext,
): InspectorDetail {
  const events =
    context.session?.events.filter(
      (event) => approvalIdFromPayload(event.payload) === ref.approval_id,
    ) ?? [];
  const request = events.find((event) => event.kind === "approval.requested");
  const resolution = events.find((event) => event.kind === "approval.resolved");
  if (!request && !resolution) {
    return notAvailable(ref, "Approval is not loaded locally");
  }
  return {
    title: ref.approval_id,
    status: "resolved",
    rows: [
      { label: "approval id", value: ref.approval_id },
      { label: "state", value: resolution ? "resolved" : "pending" },
      { label: "prompt", value: payloadString(request?.payload, "prompt") },
      {
        label: "approved",
        value: payloadDisplayValue(resolution?.payload, "approved"),
      },
    ],
    refs: events.map((event) => ({
      label: event.kind,
      ref: eventRef(event),
    })),
    payload: {
      request: request?.payload,
      resolution: resolution?.payload,
    },
  };
}

function warningDetail(
  ref: Extract<CortexRef, { kind: "warning" }>,
  context: InspectorContext,
): InspectorDetail {
  const activeBadge = findWarningBadge(ref.warning_kind, context);
  const events =
    context.session?.events.filter((event) =>
      eventMatchesWarning(event, ref.warning_kind),
    ) ?? [];
  if (!activeBadge && events.length === 0) {
    return notAvailable(ref, "Warning is not loaded locally");
  }
  const acknowledged = events.some(
    (event) => event.kind === "coordination.warning_acknowledged",
  );
  return {
    title: ref.warning_kind,
    status: "resolved",
    rows: [
      { label: "warning kind", value: ref.warning_kind },
      { label: "severity", value: activeBadge?.severity },
      { label: "label", value: activeBadge?.label },
      { label: "acknowledged", value: acknowledged },
      { label: "command", value: ref.command_id },
    ],
    refs: events.map((event) => ({
      label: event.kind,
      ref: eventRef(event),
    })),
    payload: { activeBadge, events: events.map((event) => event.payload) },
  };
}

function notAvailable(ref: CortexRef, reason: string): InspectorDetail {
  return {
    title: refTitle(ref),
    status: "not_available",
    rows: [
      { label: "kind", value: ref.kind },
      { label: "id", value: refPrimaryValue(ref) },
      { label: "reason", value: reason },
    ],
    refs: [],
    payload: ref,
  };
}

function eventRef(event: EventEnvelope): CortexRef {
  return {
    kind: "event",
    event_id: event.event_id,
    scope_ref: event.scope_ref,
    seq: event.seq,
  };
}

function eventIdentityRefs(event: EventEnvelope): InspectorRefLink[] {
  const refs: InspectorRefLink[] = [];
  if (event.node_id) {
    refs.push({ label: "node", ref: { kind: "node", node_id: event.node_id } });
  }
  if (event.runtime_session_id) {
    refs.push({
      label: "runtime",
      ref: {
        kind: "runtime",
        runtime_session_id: event.runtime_session_id,
      },
    });
  }
  if (event.session_thread_id) {
    refs.push({
      label: "session",
      ref: { kind: "session", session_thread_id: event.session_thread_id },
    });
  }
  if (event.turn_id) {
    refs.push({ label: "turn", ref: { kind: "turn", turn_id: event.turn_id } });
  }
  return refs;
}

function messageRefs(message: Message): InspectorRefLink[] {
  const refs: InspectorRefLink[] = [];
  if (message.turn_id) {
    refs.push({
      label: "turn",
      ref: { kind: "turn", turn_id: message.turn_id },
    });
  }
  if (message.source_event_id) {
    refs.push({
      label: "source event",
      ref: {
        kind: "event",
        event_id: message.source_event_id,
        scope_ref: {
          kind: "session",
          session_thread_id: message.session_thread_id,
        },
        seq: 0,
      },
    });
  }
  return refs;
}

function refLinks(label: string, refs: CortexRef[]): InspectorRefLink[] {
  return refs.map((ref, index) => ({
    label: `${label} ${index + 1}`,
    ref,
  }));
}

function findArtifactNode(
  node: ArtifactTreeNode,
  artifactId: string,
): ArtifactTreeNode | null {
  if (node.artifact_id === artifactId) return node;
  for (const child of node.children) {
    const found = findArtifactNode(child, artifactId);
    if (found) return found;
  }
  return null;
}

function findWarningBadge(
  warningKind: string,
  context: InspectorContext,
): ResourceBadge | undefined {
  const sessionBadge = context.session?.placement.resource_badges.find(
    (badge) => badge.kind === warningKind,
  );
  if (sessionBadge) return sessionBadge;
  return context.inventory?.placements
    .flatMap((placement) => placement.resource_badges)
    .find((badge) => badge.kind === warningKind);
}

function eventMatchesWarning(event: EventEnvelope, warningKind: string) {
  return (
    payloadString(event.payload, "warning_kind") === warningKind ||
    payloadString(event.payload, "kind") === warningKind ||
    payloadString(event.payload, "resource_kind") === warningKind
  );
}

function approvalIdFromPayload(payload: unknown) {
  return payloadString(payload, "approval_id");
}

function payloadString(payload: unknown, key: string) {
  const value = payloadValue(payload, key);
  return typeof value === "string" ? value : undefined;
}

function payloadDisplayValue(
  payload: unknown,
  key: string,
): InspectorRow["value"] {
  const value = payloadValue(payload, key);
  if (
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean" ||
    value === null ||
    value === undefined
  ) {
    return value;
  }
  return safeInlineJson(value);
}

function payloadValue(payload: unknown, key: string) {
  if (typeof payload !== "object" || payload === null || !(key in payload)) {
    return undefined;
  }
  return (payload as Record<string, unknown>)[key];
}

function formatValue(value: InspectorRow["value"]) {
  if (value === null || value === undefined || value === "") {
    return "unavailable";
  }
  return String(value);
}

function safeJson(value: unknown) {
  return JSON.stringify(value, null, 2) ?? "null";
}

function safeInlineJson(value: unknown) {
  try {
    return JSON.stringify(value);
  } catch {
    return "unavailable";
  }
}

function statusTone(status: InspectorStatus) {
  if (status === "resolved") return "good";
  if (status === "not_implemented") return "info";
  return "warn";
}
