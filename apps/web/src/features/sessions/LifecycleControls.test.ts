import { describe, expect, it } from "vitest";

import type {
  RuntimeSummary,
  SessionSummary,
} from "../../shared/protocol/types";
import { lifecycleControlStates } from "./LifecycleControls";

describe("lifecycleControlStates", () => {
  it("enables lifecycle controls from session and runtime state", () => {
    expect(enabledLabels(session("active", runtime("running")))).toEqual([
      "Detach",
      "Interrupt",
      "Stop",
    ]);
    expect(enabledLabels(session("detached", runtime("ready")))).toEqual([
      "Attach",
      "Stop",
    ]);
    expect(enabledLabels(session("active", runtime("error")))).toEqual([
      "Detach",
      "Stop",
      "Resume",
    ]);
    expect(enabledLabels(session("active", runtime("expired")))).toEqual([
      "Detach",
      "Resume",
    ]);
    expect(enabledLabels(session("stopped", runtime("stopped")))).toEqual([
      "Resume",
    ]);
  });

  it("disables every lifecycle control while a lifecycle mutation is pending", () => {
    const controls = lifecycleControlStates(
      {
        session: session("active", runtime("running")),
        runtime: runtime("running"),
      },
      true,
    );

    expect(controls.every((control) => !control.enabled)).toBe(true);
  });
});

function enabledLabels(sessionSummary: SessionSummary) {
  return lifecycleControlStates({
    session: sessionSummary,
    runtime: sessionSummary.runtime,
  })
    .filter((control) => control.enabled)
    .map((control) => control.label);
}

function runtime(state: RuntimeSummary["state"]): RuntimeSummary {
  return {
    runtime_session_id: "runtime-1",
    provider: "codex",
    state,
    resume_supported: true,
    degraded_reason: null,
    last_runtime_step_at: null,
  };
}

function session(
  state: SessionSummary["state"],
  runtimeSummary: RuntimeSummary,
): SessionSummary {
  return {
    session_thread_id: "session-1",
    project_placement_id: "placement-1",
    runtime_session_id: runtimeSummary.runtime_session_id,
    title: "Session",
    state,
    runtime: runtimeSummary,
    message_count: 0,
    updated_at: "2026-06-17T00:00:00Z",
  };
}
