import { describe, expect, it } from "vitest";

import { applySessionEvent } from "./apply-session-event";
import type {
  EventEnvelope,
  EventKind,
  EventPayload,
  SessionDetail,
} from "../../shared/protocol/types";
import { eventPayloadTypeForKind } from "../../shared/protocol/validators";

describe("applySessionEvent", () => {
  it("appends the next event when seq is contiguous", () => {
    const result = applySessionEvent(detailWithSeq(1), eventWithSeq(2));

    expect(result.kind).toBe("applied");
  });

  it("adds assistant message for completed provider event", () => {
    const result = applySessionEvent(
      detailWithSeq(1),
      eventWithSeq(2, "provider.message.completed", {
        content: "Assistant reply",
      }),
    );

    expect(result.kind).toBe("applied");
    if (result.kind === "applied") {
      expect(result.detail.messages.at(-1)).toMatchObject({
        role: "assistant",
        content: "Assistant reply",
        source_event_id: "event-2",
      });
      expect(result.detail.session.message_count).toBe(1);
    }
  });

  it("marks runtime and session degraded for runtime error", () => {
    const result = applySessionEvent(
      detailWithSeq(1),
      eventWithSeq(2, "runtime.error", { message: "Provider failed" }),
    );

    expect(result.kind).toBe("applied");
    if (result.kind === "applied") {
      expect(result.detail.session.state).toBe("degraded");
      expect(result.detail.session.runtime.state).toBe("error");
      expect(result.detail.messages.at(-1)?.role).toBe("runtime");
    }
  });

  it("updates placement resource badges from resource snapshot events", () => {
    const result = applySessionEvent(
      detailWithSeq(1),
      eventWithSeq(2, "resource.snapshot.updated", {
        resource_badges: [
          {
            kind: "dirty_workspace",
            severity: "warning",
            label: "Dirty workspace",
          },
        ],
      }),
    );

    expect(result.kind).toBe("applied");
    if (result.kind === "applied") {
      expect(result.detail.placement.resource_badges).toEqual([
        {
          kind: "dirty_workspace",
          severity: "warning",
          label: "Dirty workspace",
        },
      ]);
    }
  });

  it("reports a gap when seq skips ahead", () => {
    const result = applySessionEvent(detailWithSeq(1), eventWithSeq(3));

    expect(result).toEqual({ kind: "gap", expectedSeq: 2, receivedSeq: 3 });
  });

  it("uses session projection seq when raw event seqs come from different scopes", () => {
    const result = applySessionEvent(
      detailWithSeq(5, 1),
      eventWithSeq(1, "coordination.warning_acknowledged", {}, 2),
    );

    expect(result.kind).toBe("applied");
  });
});

function detailWithSeq(
  seq: number,
  sessionProjectionSeq?: number,
): SessionDetail {
  return {
    session: {
      session_thread_id: "session-1",
      project_placement_id: "placement-1",
      runtime_session_id: "runtime-1",
      title: "Session",
      state: "active",
      runtime: {
        runtime_session_id: "runtime-1",
        provider: "codex",
        state: "ready",
        resume_supported: true,
        degraded_reason: null,
        last_runtime_step_at: null,
      },
      message_count: 0,
      updated_at: "2026-06-17T00:00:00Z",
    },
    placement: {
      project_placement_id: "placement-1",
      project_id: null,
      node_id: "node-1",
      display_name: "uprava",
      workspace_path: "/workspace",
      state: "validated",
      resource_badges: [],
      last_validated_at: null,
    },
    messages: [],
    events: [eventWithSeq(seq, "runtime.ready", {}, sessionProjectionSeq)],
  };
}

function eventWithSeq(
  seq: number,
  kind: EventKind = "runtime.ready",
  payload: unknown = {},
  sessionProjectionSeq?: number,
): EventEnvelope {
  return {
    event_id: `event-${seq}`,
    command_id: null,
    actor_ref: { kind: "system" },
    scope_ref: { kind: "runtime", runtime_session_id: "runtime-1" },
    node_id: "node-1",
    runtime_session_id: "runtime-1",
    session_thread_id: "session-1",
    turn_id: null,
    seq,
    session_projection_seq: sessionProjectionSeq,
    kind,
    happened_at: "2026-06-17T00:00:00Z",
    source_refs: [],
    evidence_refs: [],
    cause_refs: [],
    result_refs: [],
    payload: typedPayload(kind, payload),
  };
}

function typedPayload(kind: EventKind, payload: unknown): EventPayload {
  const fields =
    typeof payload === "object" && payload !== null && !Array.isArray(payload)
      ? payload
      : {};
  return { type: eventPayloadTypeForKind(kind), ...fields } as EventPayload;
}
