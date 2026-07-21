import type { ComponentType, LazyExoticComponent } from "react";

import type { UpravaRef } from "../shared/protocol/types";

export const CONTENT_RENDERER_CONTRACT_VERSION = 1;

export type ContentRendererProps = {
  content: string;
  state: "streaming" | "complete";
  sourceRef: UpravaRef;
};

export type LazyContentRenderer = LazyExoticComponent<
  ComponentType<ContentRendererProps>
>;
