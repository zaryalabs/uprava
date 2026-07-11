import { describe, expect, it } from "vitest";

import fixtures from "./fixtures.json";
import {
  commandAcceptedResponseSchema,
  eventEnvelopeSchema,
  workspaceCommandHistoryResponseSchema,
  workspaceCommandRunResponseSchema,
  workspaceTerminalListResponseSchema,
  workspaceTerminalOpenResponseSchema,
  workspaceTerminalOutputFrameSchema,
  workspaceTerminalStreamFrameSchema,
} from "./validators";

describe("Rust-generated protocol fixtures", () => {
  it.each([
    ["command_accepted", commandAcceptedResponseSchema],
    ["workspace_command_run", workspaceCommandRunResponseSchema],
    ["workspace_command_history", workspaceCommandHistoryResponseSchema],
    ["workspace_terminal_output", workspaceTerminalOutputFrameSchema],
    ["workspace_terminal_open", workspaceTerminalOpenResponseSchema],
    ["workspace_terminal_list", workspaceTerminalListResponseSchema],
    ["workspace_terminal_stream", workspaceTerminalStreamFrameSchema],
    ["event_envelope", eventEnvelopeSchema],
  ] as const)("validates %s", (name, schema) => {
    expect(schema.safeParse(fixtures[name]).success).toBe(true);
  });
});
