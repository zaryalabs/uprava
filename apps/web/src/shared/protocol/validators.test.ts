import { describe, expect, it } from "vitest";

import {
  eventEnvelopeSchema,
  parseTerminalStreamFrame,
  workspaceCommandHistoryItemSchema,
  workspaceReviewProjectionSchema,
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

  it("parses typed git review snapshots and check results", () => {
    const gitSnapshot = {
      state: "ready",
      repo_id: "sha256:repo",
      head_state: "branch",
      branch: "feature/review",
      commit: "0123456789abcdef",
      upstream: "origin/feature/review",
      ahead: 1,
      behind: 0,
      worktree_kind: "linked",
      operation: null,
      changed_files: [
        {
          path: "src/review.ts",
          previous_path: null,
          index_status: "modified",
          worktree_status: null,
          conflicted: false,
          binary: false,
        },
      ],
      staged_count: 1,
      unstaged_count: 0,
      untracked_count: 0,
      conflicted_count: 0,
      truncated: false,
      generated_at: "2026-07-19T00:00:00Z",
    } as const;
    const diff = {
      placement_id: "placement-1",
      diff_id: "diff-1",
      git_snapshot: gitSnapshot,
      summary: "1 changed",
      diff: "@@ -1 +1 @@",
      scope: "staged",
      path: "src/review.ts",
      changed_files: gitSnapshot.changed_files,
      hunks: [
        {
          hunk_id: "diff-1:hunk-1",
          header: "@@ -1 +1 @@",
          patch: "@@ -1 +1 @@\n-before\n+after\n",
        },
      ],
      original: "before\n",
      modified: "after\n",
      binary: false,
      summary_truncated: false,
      diff_truncated: false,
      generated_at: "2026-07-19T00:00:00Z",
    } as const;

    const parsed = workspaceReviewProjectionSchema.safeParse({
      placement_id: "placement-1",
      git_snapshot: gitSnapshot,
      diff,
      checks: [
        {
          command_id: "command-1",
          state: "completed",
          command: "make",
          args: ["l"],
          label: "Quick check",
          success: true,
          exit_code: 0,
          stdout: "ok\n",
          stderr: "",
          stdout_truncated: false,
          stderr_truncated: false,
          duration_ms: 10,
          created_at: "2026-07-19T00:00:00Z",
          completed_at: "2026-07-19T00:00:01Z",
        },
      ],
      generated_at: "2026-07-19T00:00:01Z",
    });

    expect(parsed.success).toBe(true);
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

  it("rejects event payloads that do not match the envelope kind", () => {
    const parsed = eventEnvelopeSchema.safeParse({
      event_id: "event-1",
      command_id: null,
      actor_ref: { kind: "system" },
      scope_ref: { kind: "runtime", runtime_session_id: "runtime-1" },
      node_id: "node-1",
      runtime_session_id: "runtime-1",
      session_thread_id: "session-1",
      turn_id: null,
      seq: 1,
      kind: "runtime.ready",
      happened_at: "2026-07-10T00:00:00Z",
      source_refs: [],
      evidence_refs: [],
      cause_refs: [],
      result_refs: [],
      payload: { type: "turn_started" },
    });

    expect(parsed.success).toBe(false);
  });
});
