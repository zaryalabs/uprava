import { describe, expect, it } from "vitest";

import { canRunCommand, runWorkbenchCommand } from "./registry";
import type {
  ProjectPlacementSummary,
  RuntimeSummary,
  SessionSummary,
} from "../../shared/protocol/types";

describe("workbench command registry", () => {
  it("enables node enrollment and revoke commands from explicit context", () => {
    expect(
      canRunCommand("node.createEnrollment", {
        nodeDisplayName: "Local Node",
      }),
    ).toBe(true);
    expect(
      canRunCommand("node.createEnrollment", {
        nodeDisplayName: "   ",
      }),
    ).toBe(false);
    expect(canRunCommand("node.revoke", { nodeId: "node-1" })).toBe(true);
    expect(canRunCommand("node.rotateCredential", { nodeId: "node-1" })).toBe(
      true,
    );
    expect(canRunCommand("node.delete", { nodeId: "node-1" })).toBe(true);
    expect(
      canRunCommand("node.approveEnrollment", { enrollmentId: "enroll-1" }),
    ).toBe(true);
  });

  it("uses authoritative projected runtime actions", () => {
    expect(
      canRunCommand("runtime.interrupt", {
        session: sessionWithState("active"),
        runtime: runtimeWithState("running"),
        availableCommands: ["runtime.interrupt"],
      }),
    ).toBe(true);
    expect(
      canRunCommand("runtime.interrupt", {
        session: sessionWithState("active"),
        runtime: runtimeWithState("running"),
        availableCommands: [],
      }),
    ).toBe(false);
  });

  it("enables warning acknowledgement with session and warning kind", () => {
    expect(
      canRunCommand("warning.acknowledge", {
        session: sessionWithState("active"),
        warningKind: "dirty_workspace",
        availableCommands: ["warning.acknowledge"],
      }),
    ).toBe(true);
    expect(
      canRunCommand("warning.acknowledge", {
        session: sessionWithState("active"),
        availableCommands: ["warning.acknowledge"],
      }),
    ).toBe(false);
  });

  it("uses authoritative projected stop action", () => {
    expect(
      canRunCommand("runtime.stop", {
        session: sessionWithState("active"),
        runtime: runtimeWithState("running"),
        availableCommands: ["runtime.stop"],
      }),
    ).toBe(true);
    expect(
      canRunCommand("runtime.stop", {
        session: sessionWithState("active"),
        runtime: runtimeWithState("running"),
        availableCommands: [],
      }),
    ).toBe(false);
  });

  it("requires attached session state and non-empty content for send turn", () => {
    expect(
      canRunCommand("session.sendTurn", {
        session: sessionWithState("active"),
        turnContent: "hello",
        availableCommands: ["session.sendTurn"],
      }),
    ).toBe(true);
    expect(
      canRunCommand("session.sendTurn", {
        session: sessionWithState("detached"),
        turnContent: "hello",
        availableCommands: [],
      }),
    ).toBe(false);
    expect(
      canRunCommand("session.sendTurn", {
        session: sessionWithState("active"),
        turnContent: "   ",
        availableCommands: ["session.sendTurn"],
      }),
    ).toBe(false);
    expect(
      canRunCommand("session.sendTurn", {
        session: sessionWithRuntime("active", runtimeWithState("expired")),
        turnContent: "hello",
        availableCommands: [],
      }),
    ).toBe(false);
  });

  it("requires blocked attached runtime state for approval resolution", () => {
    expect(
      canRunCommand("approval.resolve", {
        session: sessionWithRuntime("active", runtimeWithState("blocked")),
        approvalId: "approval-1",
        approved: true,
        availableCommands: ["approval.resolve"],
      }),
    ).toBe(true);
    expect(
      canRunCommand("approval.resolve", {
        session: sessionWithRuntime("active", runtimeWithState("ready")),
        approvalId: "approval-1",
        approved: true,
        availableCommands: [],
      }),
    ).toBe(false);
    expect(
      canRunCommand("approval.resolve", {
        session: sessionWithRuntime("detached", runtimeWithState("blocked")),
        approvalId: "approval-1",
        approved: true,
        availableCommands: [],
      }),
    ).toBe(false);
  });

  it("blocks session start when placement has a hard resource badge", () => {
    expect(
      canRunCommand("session.start", {
        placement: placementWithHardBlock(false),
      }),
    ).toBe(true);
    expect(
      canRunCommand("session.start", {
        placement: placementWithHardBlock(true),
      }),
    ).toBe(false);
  });

  it("enables workspace deletion when placement context is present", () => {
    expect(
      canRunCommand("placement.delete", {
        placement: placementWithHardBlock(false),
      }),
    ).toBe(true);
    expect(canRunCommand("placement.delete", {})).toBe(false);
  });

  it("opens and copies references through injected command context", async () => {
    const ref = { kind: "session" as const, session_thread_id: "session-1" };
    const opened: import("../../shared/protocol/types").UpravaRef[] = [];
    let copied = "";

    expect(
      canRunCommand("reference.openInInspector", {
        reference: ref,
        openReference: (nextRef) => {
          opened.push(nextRef);
        },
      }),
    ).toBe(true);
    expect(canRunCommand("reference.copy", { reference: ref })).toBe(true);

    await runWorkbenchCommand("reference.openInInspector", {
      reference: ref,
      openReference: (nextRef) => {
        opened.push(nextRef);
      },
    });
    await runWorkbenchCommand("reference.copy", {
      reference: ref,
      copyText: (text) => {
        copied = text;
      },
    });

    expect(opened).toEqual([ref]);
    expect(copied).toContain("session-1");
  });
});

function runtimeWithState(state: RuntimeSummary["state"]): RuntimeSummary {
  return {
    runtime_session_id: "runtime-1",
    provider: "codex",
    state,
    resume_supported: true,
    degraded_reason: null,
    last_runtime_step_at: null,
  };
}

function sessionWithState(state: SessionSummary["state"]): SessionSummary {
  return sessionWithRuntime(state, runtimeWithState("ready"));
}

function sessionWithRuntime(
  state: SessionSummary["state"],
  runtime: RuntimeSummary,
): SessionSummary {
  return {
    session_thread_id: "session-1",
    project_placement_id: "placement-1",
    runtime_session_id: runtime.runtime_session_id,
    title: "Session",
    state,
    runtime,
    message_count: 0,
    updated_at: "2026-06-17T00:00:00Z",
  };
}

function placementWithHardBlock(hardBlock: boolean): ProjectPlacementSummary {
  return {
    project_placement_id: "placement-1",
    project_id: null,
    node_id: "node-1",
    display_name: "uprava",
    workspace_path: "/workspace",
    state: "validated",
    resource_badges: hardBlock
      ? [
          {
            kind: "missing_workspace",
            severity: "hard_block",
            label: "Missing",
          },
        ]
      : [],
    last_validated_at: null,
  };
}
