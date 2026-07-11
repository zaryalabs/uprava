import { afterEach, describe, expect, it, vi } from "vitest";

import { coreApi } from "./http-client";

describe("coreApi workspace commands", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("runs workspace commands through async acceptance and resource polling", async () => {
    const requests: Array<{ method: string; pathname: string }> = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = new URL(input.toString());
        const method = init?.method?.toUpperCase() ?? "GET";
        requests.push({ method, pathname: url.pathname });

        if (
          method === "POST" &&
          url.pathname ===
            "/api/v1/placements/placement-1/workspace/commands/async"
        ) {
          return jsonResponse({
            command_id: "command-1",
            session: null,
          });
        }

        if (
          method === "GET" &&
          url.pathname ===
            "/api/v1/placements/placement-1/workspace/commands/async/command-1"
        ) {
          return jsonResponse({
            command_id: "command-1",
            kind: "RunWorkspaceCommand",
            state: "completed",
            created_at: "2026-06-17T00:00:00Z",
            completed_at: "2026-06-17T00:00:01Z",
            payload: {},
            result_payload: {
              placement_id: "placement-1",
              terminal_command_id: "terminal-command-1",
              command: "rustc",
              args: ["--version"],
              intent: "command",
              label: null,
              exit_code: 0,
              success: true,
              stdout: "rustc 1.0.0\n",
              stderr: "",
              stdout_truncated: false,
              stderr_truncated: false,
              duration_ms: 10,
              started_at: "2026-06-17T00:00:00Z",
              completed_at: "2026-06-17T00:00:01Z",
            },
          });
        }

        throw new Error(
          `Unhandled mocked Core request: ${method} ${url.pathname}`,
        );
      }),
    );

    const result = await coreApi.runWorkspaceCommand("placement-1", {
      command: "rustc",
      args: ["--version"],
      intent: "command",
      label: null,
      timeout_seconds: 30,
    });

    expect(result.stdout).toBe("rustc 1.0.0\n");
    expect(requests).toEqual([
      {
        method: "POST",
        pathname: "/api/v1/placements/placement-1/workspace/commands/async",
      },
      {
        method: "GET",
        pathname:
          "/api/v1/placements/placement-1/workspace/commands/async/command-1",
      },
    ]);
  });

  it("rejects malformed async command resources at the protocol boundary", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = new URL(input.toString());
        const method = init?.method?.toUpperCase() ?? "GET";

        if (url.pathname === "/api/v1/client/logs") {
          return new Response(null, { status: 204 });
        }

        if (
          method === "POST" &&
          url.pathname ===
            "/api/v1/placements/placement-1/workspace/commands/async"
        ) {
          return jsonResponse({
            command_id: "command-1",
            session: null,
          });
        }

        if (
          method === "GET" &&
          url.pathname ===
            "/api/v1/placements/placement-1/workspace/commands/async/command-1"
        ) {
          return jsonResponse({
            command_id: "command-1",
            kind: "RunWorkspaceCommand",
            state: "done",
            created_at: "2026-06-17T00:00:00Z",
            completed_at: "2026-06-17T00:00:01Z",
            payload: {},
            result_payload: {},
          });
        }

        throw new Error(
          `Unhandled mocked Core request: ${method} ${url.pathname}`,
        );
      }),
    );

    await expect(
      coreApi.runWorkspaceCommand("placement-1", {
        command: "rustc",
        args: ["--version"],
        intent: "command",
        label: null,
        timeout_seconds: 30,
      }),
    ).rejects.toMatchObject({
      envelope: { error_code: "web.protocol_validation_failed" },
    });
  });
});

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}
