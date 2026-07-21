import { memo } from "react";
import {
  Streamdown,
  type Components,
  type ControlsConfig,
  type UrlTransform,
} from "streamdown";

import type { ContentRendererProps } from "../../content-renderers";

const SAFE_URL = /^(?:https?:|mailto:)/i;
const DISALLOWED_ELEMENTS = ["img"] as const;
const CONTROLS: ControlsConfig = {
  table: false,
  code: { copy: true, download: false },
  mermaid: false,
};
const COMPONENTS: Components = {
  a({ node: _node, ...props }) {
    return <a {...props} rel="noopener noreferrer" target="_blank" />;
  },
};
const safeUrlTransform: UrlTransform = (url, key) => {
  if (key === "src" || !SAFE_URL.test(url)) {
    return null;
  }
  return url;
};

export const MarkdownRenderer = memo(function MarkdownRenderer({
  content,
  state,
}: ContentRendererProps) {
  const streaming = state === "streaming";
  return (
    <Streamdown
      animated={false}
      className="uprava-markdown text-sm"
      components={COMPONENTS}
      controls={CONTROLS}
      disallowedElements={DISALLOWED_ELEMENTS}
      isAnimating={streaming}
      lineNumbers={false}
      mode={streaming ? "streaming" : "static"}
      skipHtml
      urlTransform={safeUrlTransform}
    >
      {content}
    </Streamdown>
  );
});
