import { describe, expect, it } from "vitest";

import { sessionSummaryFromProjection } from "./AgentProjectionPanel";
import type { AgentProjection } from "../../shared/protocol/types";

describe("sessionSummaryFromProjection", () => {
  it("builds command context from safe projection fields", () => {
    const session = sessionSummaryFromProjection(projection());

    expect(session.session_thread_id).toBe("session-1");
    expect(session.project_placement_id).toBe("placement-1");
    expect(session.runtime_session_id).toBe("runtime-1");
    expect(session.runtime.state).toBe("blocked");
    expect(session.message_count).toBe(1);
  });
});

function projection(): AgentProjection {
  return {
    session_thread_id: "session-1",
    project_placement: {
      project_placement_id: "placement-1",
      project_id: null,
      node_id: "node-1",
      display_name: "cortex",
      workspace_path: "/workspace",
      state: "validated",
      resource_badges: [],
      last_validated_at: null,
    },
    runtime_summary: {
      runtime_session_id: "runtime-1",
      provider: "codex",
      state: "blocked",
      resume_supported: true,
      degraded_reason: null,
      last_runtime_step_at: null,
    },
    current_turn: "turn-1",
    pending_approvals: [],
    active_warnings: [],
    recent_turn_summaries: [],
    recent_message_refs: [{ kind: "message", message_id: "message-1" }],
    artifact_tree_summary: "summary",
    available_block_types: [],
    available_commands: [],
    visible_refs: [],
    source_cause_summary: "sources",
    resume_context: "resume",
    generated_at: "2026-06-17T00:00:00Z",
  };
}
