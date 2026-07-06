import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { queryKeys } from "../../shared/api/query-keys";
import type { EventEnvelope, SessionDetail } from "../../shared/protocol/types";
import { applySessionStreamEventToCache } from "./session-stream-cache";

describe("applySessionStreamEventToCache", () => {
  it("applies contiguous events and invalidates derived snapshots", async () => {
    const queryClient = queryClientWithSnapshots(detailWithSeq(1));

    const result = await applySessionStreamEventToCache(
      queryClient,
      "session-1",
      eventWithSeq(2, "provider.message.completed", {
        content: "Assistant reply",
      }),
    );

    const detail = queryClient.getQueryData<SessionDetail>(
      queryKeys.session("session-1"),
    );
    expect(result).toEqual({ kind: "applied" });
    expect(detail?.events.map((event) => event.seq)).toEqual([1, 2]);
    expect(detail?.messages.at(-1)).toMatchObject({
      role: "assistant",
      content: "Assistant reply",
    });
    expect(isInvalidated(queryClient, queryKeys.session("session-1"))).toBe(
      false,
    );
    expect(
      isInvalidated(queryClient, queryKeys.artifactTree("session-1")),
    ).toBe(true);
    expect(
      isInvalidated(queryClient, queryKeys.agentProjection("session-1")),
    ).toBe(true);
    expect(isInvalidated(queryClient, queryKeys.inventory)).toBe(true);
  });

  it("keeps cached session data and invalidates all snapshots on sequence gap", async () => {
    const queryClient = queryClientWithSnapshots(detailWithSeq(1));

    const result = await applySessionStreamEventToCache(
      queryClient,
      "session-1",
      eventWithSeq(3),
    );

    const detail = queryClient.getQueryData<SessionDetail>(
      queryKeys.session("session-1"),
    );
    expect(result).toEqual({
      kind: "reloaded",
      reason: "sequence-gap",
      expectedSeq: 2,
      receivedSeq: 3,
    });
    expect(detail?.events.map((event) => event.seq)).toEqual([1]);
    expect(isInvalidated(queryClient, queryKeys.session("session-1"))).toBe(
      true,
    );
    expect(
      isInvalidated(queryClient, queryKeys.artifactTree("session-1")),
    ).toBe(true);
    expect(
      isInvalidated(queryClient, queryKeys.agentProjection("session-1")),
    ).toBe(true);
    expect(isInvalidated(queryClient, queryKeys.inventory)).toBe(true);
  });
});

function queryClientWithSnapshots(detail: SessionDetail) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  queryClient.setQueryData(queryKeys.session("session-1"), detail);
  queryClient.setQueryData(queryKeys.artifactTree("session-1"), {
    root: { children: [] },
  });
  queryClient.setQueryData(queryKeys.agentProjection("session-1"), {
    available_commands: [],
  });
  queryClient.setQueryData(queryKeys.inventory, { nodes: [] });
  return queryClient;
}

function isInvalidated(queryClient: QueryClient, queryKey: readonly unknown[]) {
  return queryClient.getQueryState(queryKey)?.isInvalidated ?? false;
}

function detailWithSeq(seq: number): SessionDetail {
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
    events: [eventWithSeq(seq)],
  };
}

function eventWithSeq(
  seq: number,
  kind = "runtime.ready",
  payload: unknown = {},
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
    kind,
    happened_at: "2026-06-17T00:00:00Z",
    source_refs: [],
    evidence_refs: [],
    cause_refs: [],
    result_refs: [],
    payload,
  };
}
