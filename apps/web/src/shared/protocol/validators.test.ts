import { describe, expect, it } from "vitest";

import {
  eventEnvelopeSchema,
  parseTerminalStreamFrame,
  workspaceCommandHistoryItemSchema,
} from "./validators";

describe("protocol validators", () => {
  it("parses valid terminal stream frames", () => {
    expect(
      parseTerminalStreamFrame(
        JSON.stringify({
          kind: "status",
          terminal_id: "terminal-1",
          state: "running",
          exit_code: null,
          message: null,
          sent_at: "2026-07-10T00:00:00Z",
        }),
      ),
    ).toEqual({
      kind: "status",
      terminal_id: "terminal-1",
      state: "running",
      exit_code: null,
      message: null,
      sent_at: "2026-07-10T00:00:00Z",
    });
  });

  it("rejects malformed terminal stream frames", () => {
    expect(
      parseTerminalStreamFrame(
        JSON.stringify({
          kind: "status",
          terminal_id: "terminal-1",
          state: "unknown",
          exit_code: null,
          message: null,
          sent_at: "2026-07-10T00:00:00Z",
        }),
      ),
    ).toBeNull();
  });

  it("rejects command history enum drift", () => {
    const parsed = workspaceCommandHistoryItemSchema.safeParse({
      command_id: "command-1",
      kind: "RunWorkspaceShell",
      state: "completed",
      created_at: "2026-07-10T00:00:00Z",
      completed_at: "2026-07-10T00:00:01Z",
      payload: {},
      result_payload: {},
    });

    expect(parsed.success).toBe(false);
  });

  it("rejects malformed SSE envelopes", () => {
    const parsed = eventEnvelopeSchema.safeParse({
      event_id: "event-1",
      command_id: null,
      actor_ref: {},
      scope_ref: {},
      node_id: null,
      runtime_session_id: null,
      session_thread_id: "session-1",
      turn_id: null,
      seq: "1",
      kind: "runtime.ready",
      happened_at: "2026-07-10T00:00:00Z",
      source_refs: [],
      evidence_refs: [],
      cause_refs: [],
      result_refs: [],
      payload: {},
    });

    expect(parsed.success).toBe(false);
  });
});
