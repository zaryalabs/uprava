import { describe, expect, it } from "vitest";

import { parseChecks, taskRuntimeAvailable } from "./TasksRoute";

describe("Task Run form helpers", () => {
  it("turns unique command lines into bounded argv checks", () => {
    expect(parseChecks("make c\nmake c\nnpm test", 900)).toEqual([
      {
        label: "make c",
        command: "make",
        args: ["c"],
        timeout_seconds: 600,
      },
      {
        label: "npm test",
        command: "npm",
        args: ["test"],
        timeout_seconds: 600,
      },
    ]);
  });

  it("requires a complete Codex OpenSandbox capability", () => {
    const capability = (value: Record<string, unknown>) => [
      {
        key: "task_runtime.opensandbox.docker",
        value: { name: "task_runtime", value },
      },
    ];

    expect(
      taskRuntimeAvailable(
        capability({
          available: true,
          provider: "codex",
          runtime_image: "uprava/codex-runtime:test",
        }),
      ),
    ).toBe(true);
    expect(taskRuntimeAvailable(capability({ available: true }))).toBe(false);
  });
});
