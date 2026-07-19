import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import type { WorkspaceReviewProjection } from "../../shared/protocol/types";
import { WorkspaceReviewPanel } from "./WorkspaceReviewPanel";

describe("WorkspaceReviewPanel", () => {
  it("shows risky git state, changed files and durable check results", () => {
    const onSelectPath = vi.fn();
    const onScopeChange = vi.fn();
    renderReview({ onSelectPath, onScopeChange });

    expect(screen.getByText("feature/review")).toBeVisible();
    expect(screen.getByText("rebase in progress")).toBeVisible();
    expect(screen.getByText("1 conflicts")).toBeVisible();
    expect(screen.getByText("src/review.ts")).toBeVisible();
    expect(screen.getByText("Quick check")).toBeVisible();
    expect(screen.getByText("completed")).toBeVisible();

    fireEvent.click(screen.getByText("src/review.ts"));
    expect(onSelectPath).toHaveBeenCalledWith("src/review.ts");
    fireEvent.click(screen.getByRole("button", { name: "staged" }));
    expect(onScopeChange).toHaveBeenCalledWith("staged");
  });
});

function renderReview({
  onSelectPath,
  onScopeChange,
}: {
  onSelectPath: (path: string) => void;
  onScopeChange: (scope: "all" | "staged" | "unstaged") => void;
}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <WorkspaceReviewPanel
          placementId="placement-1"
          isLoading={false}
          error={null}
          review={reviewFixture()}
          scope="all"
          selectedPath={null}
          onScopeChange={onScopeChange}
          onSelectPath={onSelectPath}
          onOpenSource={vi.fn()}
          onRefresh={vi.fn()}
        />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

function reviewFixture(): WorkspaceReviewProjection {
  const changedFile = {
    path: "src/review.ts",
    previous_path: null,
    index_status: "modified" as const,
    worktree_status: "unmerged" as const,
    conflicted: true,
    binary: false,
  };
  const gitSnapshot = {
    state: "ready" as const,
    repo_id: "sha256:review",
    head_state: "branch" as const,
    branch: "feature/review",
    commit: "0123456789abcdef",
    upstream: "origin/feature/review",
    ahead: 1,
    behind: 0,
    worktree_kind: "linked" as const,
    operation: "rebase" as const,
    changed_files: [changedFile],
    staged_count: 1,
    unstaged_count: 1,
    untracked_count: 0,
    conflicted_count: 1,
    truncated: false,
    generated_at: "2026-07-19T00:00:00Z",
  };
  return {
    placement_id: "placement-1",
    git_snapshot: gitSnapshot,
    diff: {
      placement_id: "placement-1",
      diff_id: "diff-1",
      git_snapshot: gitSnapshot,
      summary: "1 changed",
      diff: "@@ -1 +1 @@",
      scope: "all",
      path: null,
      changed_files: [changedFile],
      hunks: [],
      original: null,
      modified: null,
      binary: false,
      summary_truncated: false,
      diff_truncated: false,
      generated_at: "2026-07-19T00:00:00Z",
    },
    checks: [
      {
        command_id: "command-1",
        state: "completed",
        command: "make",
        args: ["l"],
        label: "Quick check",
        success: false,
        exit_code: 1,
        stdout: "",
        stderr: "failed\n",
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 12,
        created_at: "2026-07-19T00:00:00Z",
        completed_at: "2026-07-19T00:00:01Z",
      },
    ],
    generated_at: "2026-07-19T00:00:01Z",
  };
}
