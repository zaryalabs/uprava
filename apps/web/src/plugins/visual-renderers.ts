import type { ComponentType, LazyExoticComponent, ReactNode } from "react";

import type { ArtifactDetail, UpravaRef } from "../shared/protocol/types";
import type { UiBlock } from "../workbench/blocks/types";

export type InlineRendererProps = {
  source: string;
  languageId: string;
  sourceRef: UpravaRef;
  surfaceId: string;
  fallback: ReactNode;
};

export type ArtifactRendererProps = {
  detail: ArtifactDetail;
  fallback: ReactNode;
};

export type PluginBlockRendererProps = {
  block: UiBlock;
  actions?: ReactNode;
};

export type LazyInlineRenderer = LazyExoticComponent<
  ComponentType<InlineRendererProps>
>;
export type LazyArtifactRenderer = LazyExoticComponent<
  ComponentType<ArtifactRendererProps>
>;
export type LazyBlockRenderer = LazyExoticComponent<
  ComponentType<PluginBlockRendererProps>
>;
