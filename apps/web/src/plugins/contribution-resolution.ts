import type {
  ContributionTargetResolution,
  EffectiveContribution,
  EffectivePluginSnapshot,
} from "../shared/protocol/types";

export function resolveVisualRendererChain(
  snapshot: EffectivePluginSnapshot | undefined,
  sourceKind: string,
  surface: string,
  renderScope: Extract<
    import("../shared/protocol/types").VisualRenderScope,
    "content_enhancement" | "inline_fragment" | "block" | "artifact_viewer"
  > = "content_enhancement",
  selector?: string,
): EffectiveContribution[] {
  const resolution = snapshot?.resolutions.find(
    (candidate) =>
      candidate.extension_point === "visual.renderer" &&
      candidate.mode === "exclusive" &&
      candidate.target.kind === "visual_renderer" &&
      candidate.target.source_kind === sourceKind &&
      candidate.target.surface === surface &&
      candidate.target.render_scope === renderScope &&
      (candidate.target.selector ?? undefined) === selector,
  );
  return availableContributions(resolution);
}

function availableContributions(
  resolution: ContributionTargetResolution | undefined,
) {
  return (
    resolution?.contributions.filter(
      (candidate) =>
        candidate.effective_state === "available" &&
        candidate.contribution.kind === "visual_renderer" &&
        (candidate.contract_version === 1 || candidate.contract_version === 2),
    ) ?? []
  );
}

export function resolveInlineRendererChain(
  snapshot: EffectivePluginSnapshot | undefined,
  sourceKind: string,
  surface: string,
  selector: string,
) {
  return resolveVisualRendererChain(
    snapshot,
    sourceKind,
    surface,
    "inline_fragment",
    selector,
  );
}

export function resolveBlockRendererChain(
  snapshot: EffectivePluginSnapshot | undefined,
  blockType: string,
  surface: string,
) {
  return resolveVisualRendererChain(
    snapshot,
    `block.${blockType}`,
    surface,
    "block",
  );
}

export function resolveArtifactViewerChain(
  snapshot: EffectivePluginSnapshot | undefined,
  artifactType: string,
) {
  return resolveVisualRendererChain(
    snapshot,
    `artifact.${artifactType}`,
    "artifact.viewer",
    "artifact_viewer",
  );
}
