import { describe, expect, it } from "vitest";

import fixtures from "../shared/protocol/fixtures.json";
import { effectivePluginSnapshotSchema } from "../shared/protocol/validators";
import { resolveVisualRendererChain } from "./contribution-resolution";

function snapshot() {
  return effectivePluginSnapshotSchema.parse(
    fixtures.plugin_contract.effective_snapshot,
  );
}

describe("resolveVisualRendererChain", () => {
  it("returns the persisted effective order for an exclusive target", () => {
    const chain = resolveVisualRendererChain(
      snapshot(),
      "chat.assistant_message",
      "session.timeline",
    );

    expect(chain.map((candidate) => candidate.plugin_id)).toEqual([
      "uprava.markdown",
      "uprava.plain-text",
    ]);
  });

  it("excludes disabled contributions without changing the saved order", () => {
    const value = snapshot();
    value.resolutions[0].contributions[0].effective_state = "disabled";

    const chain = resolveVisualRendererChain(
      value,
      "chat.assistant_message",
      "session.timeline",
    );

    expect(chain.map((candidate) => candidate.plugin_id)).toEqual([
      "uprava.plain-text",
    ]);
  });

  it("returns no candidates for another bounded target", () => {
    const chain = resolveVisualRendererChain(
      snapshot(),
      "chat.user_message",
      "session.timeline",
    );

    expect(chain).toEqual([]);
  });
});
