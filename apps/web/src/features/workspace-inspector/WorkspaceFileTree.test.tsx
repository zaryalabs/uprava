import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { coreApi } from "../../shared/api/http-client";
import type {
  WorkspaceEntry,
  WorkspaceTreeResponse,
} from "../../shared/protocol/types";
import { WorkspaceFileTree } from "./WorkspaceFileTree";

describe("WorkspaceFileTree", () => {
  afterEach(() => vi.restoreAllMocks());

  it("loads dot-directories lazily and reports truncated directories", async () => {
    const workspaceTree = vi
      .spyOn(coreApi, "workspaceTree")
      .mockImplementation(async (_placementId, path = ".") =>
        path === ".github"
          ? directoryResponse(".github", [file(".github/workflows.yml")])
          : directoryResponse(
              ".",
              [directory(".github"), file(".env")],
              true,
              120,
            ),
      );
    const onSelect = vi.fn();
    renderTree(onSelect);

    expect(
      await screen.findByRole("treeitem", { name: ".github" }),
    ).toBeVisible();
    expect(screen.getByRole("treeitem", { name: ".env" })).toBeVisible();
    expect(
      screen.getByText("Only the first 100 of 120 entries are shown"),
    ).toBeVisible();
    expect(workspaceTree).toHaveBeenCalledWith("placement-1", ".");
    expect(workspaceTree).not.toHaveBeenCalledWith("placement-1", ".github");

    fireEvent.click(screen.getByRole("treeitem", { name: ".github" }));
    expect(
      await screen.findByRole("treeitem", { name: "workflows.yml" }),
    ).toBeVisible();
    expect(workspaceTree).toHaveBeenCalledWith("placement-1", ".github");

    fireEvent.click(screen.getByRole("treeitem", { name: "workflows.yml" }));
    expect(onSelect).toHaveBeenCalledWith(".github/workflows.yml");

    fireEvent.click(screen.getByRole("treeitem", { name: ".github" }));
    fireEvent.click(screen.getByRole("treeitem", { name: ".github" }));
    await waitFor(() =>
      expect(
        workspaceTree.mock.calls.filter(([, path]) => path === ".github"),
      ).toHaveLength(1),
    );
  });
});

function renderTree(onSelect: (path: string) => void) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Infinity } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <WorkspaceFileTree
        placementId="placement-1"
        selectedPath={null}
        refreshVersion={0}
        onSelect={onSelect}
      />
    </QueryClientProvider>,
  );
}

function directoryResponse(
  path: string,
  children: WorkspaceEntry[],
  truncated = false,
  totalEntries: number | null = children.length,
): WorkspaceTreeResponse {
  return {
    placement_id: "placement-1",
    root: { ...directory(path), children },
    truncated,
    total_entries: totalEntries,
    generated_at: "2026-07-11T00:00:00Z",
  };
}

function directory(path: string): WorkspaceEntry {
  return {
    name: path === "." ? "." : (path.split("/").at(-1) ?? path),
    path,
    kind: "directory",
    status: "directory",
    classification: "normal",
    expandable: true,
    byte_len: null,
    modified_at: null,
    children: [],
  };
}

function file(path: string): WorkspaceEntry {
  return {
    name: path.split("/").at(-1) ?? path,
    path,
    kind: "file",
    status: "readable",
    classification: "normal",
    expandable: false,
    byte_len: 1,
    modified_at: null,
    children: [],
  };
}
