import { describe, expect, it } from "vitest";

import type { NodeSummary } from "../../shared/protocol/types";
import { providerChoiceOptions } from "./PlacementRoute";

describe("providerChoiceOptions", () => {
  it("enables Codex only when the node advertises an available provider capability", () => {
    expect(providerChoiceOptions(undefined)).toEqual([
      { id: "codex", label: "Codex", available: false },
    ]);

    expect(
      providerChoiceOptions(
        nodeWithCapabilities([
          { key: "provider.codex", value: providerCapability(false) },
        ]),
      ),
    ).toEqual([{ id: "codex", label: "Codex", available: false }]);

    expect(
      providerChoiceOptions(
        nodeWithCapabilities([
          { key: "provider.codex", value: providerCapability(true) },
        ]),
      ),
    ).toEqual([{ id: "codex", label: "Codex", available: true }]);
  });
});

function providerCapability(available: boolean) {
  return {
    kind: "provider" as const,
    available,
    configured: true,
    mode: "exec",
    timeout_seconds: 120,
    unavailable_reason: available ? null : "binary_not_found",
  };
}

function nodeWithCapabilities(
  capabilities: NodeSummary["capabilities"],
): NodeSummary {
  return {
    node_id: "node-1",
    display_name: "Node",
    presence: "reachable",
    sleep_hint: "awake",
    heartbeat_age_seconds: 1,
    active_runtime_count: 0,
    capabilities,
    diagnostics: "",
  };
}
