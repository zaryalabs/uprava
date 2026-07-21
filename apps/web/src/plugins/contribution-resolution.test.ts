import { describe, expect, it } from "vitest";

import fixtures from "../shared/protocol/fixtures.json";
import type {
  ContributionTarget,
  EffectivePluginSnapshot,
  VisualRendererContributionV1,
} from "../shared/protocol/types";
import { effectivePluginSnapshotSchema } from "../shared/protocol/validators";
import {
  resolveArtifactViewerChain,
  resolveBlockRendererChain,
  resolveInlineRendererChain,
  resolveVisualRendererChain,
} from "./contribution-resolution";

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

  it("matches inline renderers by declarative source selector", () => {
    const value = withVisualTarget(
      {
        kind: "visual_renderer",
        source_kind: "markdown.code_fence",
        surface: "session.timeline",
        render_scope: "inline_fragment",
        selector: "mermaid",
      },
      { renderer_kind: "inline_fragment" },
    );

    expect(
      resolveInlineRendererChain(
        value,
        "markdown.code_fence",
        "session.timeline",
        "mermaid",
      ),
    ).toHaveLength(1);
    expect(
      resolveInlineRendererChain(
        value,
        "markdown.code_fence",
        "session.timeline",
        "plantuml",
      ),
    ).toEqual([]);
  });

  it("resolves plugin block and artifact viewers through bounded targets", () => {
    const block = withVisualTarget(
      {
        kind: "visual_renderer",
        source_kind: "block.core.turn-activity",
        surface: "session.timeline",
        render_scope: "block",
        selector: null,
      },
      { renderer_kind: "block" },
    );
    const artifact = withVisualTarget(
      {
        kind: "visual_renderer",
        source_kind: "artifact.uprava.trace-timeline",
        surface: "artifact.viewer",
        render_scope: "artifact_viewer",
        selector: null,
      },
      { renderer_kind: "artifact_viewer" },
    );

    expect(
      resolveBlockRendererChain(
        block,
        "core.turn-activity",
        "session.timeline",
      ),
    ).toHaveLength(1);
    expect(
      resolveArtifactViewerChain(artifact, "uprava.trace-timeline"),
    ).toHaveLength(1);
  });
});

function withVisualTarget(
  target: ContributionTarget,
  contributionPatch: Partial<VisualRendererContributionV1>,
): EffectivePluginSnapshot {
  const value = snapshot();
  const template = value.resolutions.find(
    (resolution) => resolution.target.kind === "visual_renderer",
  );
  if (!template) throw new Error("visual renderer fixture is missing");
  const candidate = template.contributions[0];
  if (candidate.contribution.kind !== "visual_renderer") {
    throw new Error("visual renderer contribution fixture is missing");
  }
  const contribution = {
    ...candidate,
    contract_version: 2,
    target,
    contribution: {
      ...candidate.contribution,
      contract_version: 2,
      contribution: {
        ...candidate.contribution.contribution,
        ...contributionPatch,
      },
    },
  };
  return {
    ...value,
    resolutions: [
      ...value.resolutions,
      {
        ...template,
        target_id: `test:${JSON.stringify(target)}`,
        target,
        contributions: [contribution],
        conflict: false,
      },
    ],
  };
}
