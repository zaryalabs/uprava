import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  DEFAULT_WORKSPACE_PATH,
  PlacementNewRoute,
  workspacePathSuggestions,
} from "./PlacementNewRoute";
import type { InventorySnapshot } from "../../shared/protocol/types";

describe("PlacementNewRoute", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("prefills the compose workspace path and offers node-local path suggestions", async () => {
    vi.stubGlobal("fetch", vi.fn(mockFetch));

    renderRoute();

    const workspaceInput = screen.getByLabelText(
      "Workspace path",
    ) as HTMLInputElement;

    expect(workspaceInput).toHaveValue(DEFAULT_WORKSPACE_PATH);
    expect(workspaceInput).toHaveAttribute(
      "list",
      "workspace-path-suggestions",
    );
    expect(
      screen.getByText(
        `Node-local path; the compose node exposes ${DEFAULT_WORKSPACE_PATH}.`,
      ),
    ).toBeVisible();

    const knownPath = await screen.findByRole("button", {
      name: "Use /Users/dev/uprava",
    });

    fireEvent.click(knownPath);

    expect(workspaceInput).toHaveValue("/Users/dev/uprava");
    expect(
      screen.getByRole("button", { name: "Use /workspace/uprava" }),
    ).toBeVisible();
  });

  it("deduplicates and slugifies path suggestions", () => {
    expect(
      workspacePathSuggestions("My App", [
        { workspace_path: " /workspace " },
        { workspace_path: "/srv/my-app" },
      ]),
    ).toEqual([
      "/workspace",
      "/srv/my-app",
      "/workspace/my-app",
      "~/Projects/my-app",
      "~/work/my-app",
      "/tmp/my-app",
    ]);
  });
});

function renderRoute() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/nodes/node-1/placements/new"]}>
        <Routes>
          <Route
            path="/nodes/:nodeId/placements/new"
            element={<PlacementNewRoute />}
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

async function mockFetch(input: RequestInfo | URL) {
  const url = new URL(input.toString());
  if (url.pathname !== "/api/v1/inventory") {
    throw new Error(`Unhandled mocked Core path: ${url.pathname}`);
  }
  return new Response(JSON.stringify(inventory), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

const inventory: InventorySnapshot = {
  nodes: [
    {
      node_id: "node-1",
      display_name: "Local Node",
      presence: "reachable",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 4,
      active_runtime_count: 0,
      capabilities: [],
      diagnostics: "ok",
    },
  ],
  placements: [
    {
      project_placement_id: "placement-1",
      project_id: "project-1",
      node_id: "node-1",
      display_name: "Uprava",
      workspace_path: "/Users/dev/uprava",
      state: "validated",
      resource_badges: [],
      last_validated_at: "2026-06-17T00:00:00Z",
    },
    {
      project_placement_id: "placement-2",
      project_id: "project-2",
      node_id: "node-2",
      display_name: "Other",
      workspace_path: "/Users/dev/other",
      state: "validated",
      resource_badges: [],
      last_validated_at: "2026-06-17T00:00:00Z",
    },
  ],
  sessions: [],
  generated_at: "2026-06-17T00:00:00Z",
};
