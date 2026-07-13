import { describe, expect, it } from "vitest";

import type { SessionSummary } from "../../shared/protocol/types";
import { orderWorkspaceSessions } from "./WorkspaceAgentRoute";

describe("orderWorkspaceSessions", () => {
  it("sorts sessions by latest update and keeps ties deterministic", () => {
    const sessions = [
      session("session-b", "2026-06-17T00:00:00Z"),
      session("session-c", "2026-06-18T00:00:00Z"),
      session("session-a", "2026-06-17T00:00:00Z"),
    ];

    expect(
      orderWorkspaceSessions(sessions).map(
        (candidate) => candidate.session_thread_id,
      ),
    ).toEqual(["session-c", "session-a", "session-b"]);
    expect(sessions.map((candidate) => candidate.session_thread_id)).toEqual([
      "session-b",
      "session-c",
      "session-a",
    ]);
  });
});

function session(sessionThreadId: string, updatedAt: string): SessionSummary {
  return {
    session_thread_id: sessionThreadId,
    project_placement_id: "placement-1",
    runtime_session_id: `runtime-${sessionThreadId}`,
    title: sessionThreadId,
    state: "active",
    runtime: {
      runtime_session_id: `runtime-${sessionThreadId}`,
      provider: "codex",
      state: "ready",
      resume_supported: true,
      degraded_reason: null,
      last_runtime_step_at: updatedAt,
    },
    message_count: 0,
    updated_at: updatedAt,
  };
}
