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
          { key: "provider.codex", value: { available: false } },
        ]),
      ),
    ).toEqual([{ id: "codex", label: "Codex", available: false }]);

    expect(
      providerChoiceOptions(
        nodeWithCapabilities([
          { key: "provider.codex", value: { available: true } },
        ]),
      ),
    ).toEqual([{ id: "codex", label: "Codex", available: true }]);
  });
});

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
