import type {
  ContributionTargetResolution,
  EffectiveContribution,
  EffectivePluginSnapshot,
} from "../shared/protocol/types";

export function resolveVisualRendererChain(
  snapshot: EffectivePluginSnapshot | undefined,
  sourceKind: string,
  surface: string,
): EffectiveContribution[] {
  const resolution = snapshot?.resolutions.find(
    (candidate) =>
      candidate.extension_point === "visual.renderer" &&
      candidate.mode === "exclusive" &&
      candidate.target.kind === "visual_renderer" &&
      candidate.target.source_kind === sourceKind &&
      candidate.target.surface === surface &&
      candidate.target.render_scope === "content_enhancement",
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
        candidate.contract_version === 1,
    ) ?? []
  );
}
