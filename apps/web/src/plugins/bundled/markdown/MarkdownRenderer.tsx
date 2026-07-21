import { Children, memo, type ReactNode, useMemo } from "react";
import {
  Streamdown,
  type Components,
  type ControlsConfig,
  type UrlTransform,
} from "streamdown";

import type { ContentRendererProps } from "../../content-renderers";
import { PluginInlineFragmentRenderer } from "../../ExtensionHost";
import { colorLiteralKind } from "../../source-matchers";

const SAFE_URL = /^(?:https?:|mailto:)/i;
const DISALLOWED_ELEMENTS = ["img"] as const;
const CONTROLS: ControlsConfig = {
  table: false,
  code: { copy: true, download: false },
  mermaid: false,
};
const COLOR_TOKEN_PATTERN =
  /(#[\da-f]{3,8}\b|rgba?\(\s*\d{1,3}(?:\s*,\s*\d{1,3}){2}(?:\s*,\s*(?:0|1|0?\.\d+))?\s*\)|hsla?\(\s*-?\d+(?:\.\d+)?(?:deg)?\s*,\s*\d+(?:\.\d+)?%\s*,\s*\d+(?:\.\d+)?%(?:\s*,\s*(?:0|1|0?\.\d+))?\s*\))/gi;
const safeUrlTransform: UrlTransform = (url, key) => {
  if (key === "src" || !SAFE_URL.test(url)) {
    return null;
  }
  return url;
};

export const MarkdownRenderer = memo(function MarkdownRenderer({
  content,
  state,
  sourceRef,
}: ContentRendererProps) {
  const streaming = state === "streaming";
  const components = useMemo(
    () => createComponents(sourceRef, content),
    [content, sourceRef],
  );
  return (
    <Streamdown
      animated={false}
      className="uprava-markdown text-sm"
      components={components}
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

function createComponents(
  sourceRef: ContentRendererProps["sourceRef"],
  content: string,
): Components {
  return {
    a({ node: _node, ...props }) {
      return <a {...props} rel="noopener noreferrer" target="_blank" />;
    },
    code({ node, className, children, ...props }) {
      const languageId = languageFromClassName(className);
      const source = String(children).replace(/\n$/, "");
      const fallback = (
        <code {...props} className={className}>
          {children}
        </code>
      );
      if (!languageId) return fallback;
      return (
        <PluginInlineFragmentRenderer
          sourceKind="markdown.code_fence"
          selector={languageId}
          source={source}
          languageId={languageId}
          sourceRef={rangeRef(sourceRef, node)}
          surfaceId="session.timeline"
          fallback={fallback}
        />
      );
    },
    p({ node, children, ...props }) {
      return (
        <p {...props}>
          {enhanceColorTokens(children, sourceRef, content, node)}
        </p>
      );
    },
    li({ node, children, ...props }) {
      return (
        <li {...props}>
          {enhanceColorTokens(children, sourceRef, content, node)}
        </li>
      );
    },
  };
}

function enhanceColorTokens(
  children: ReactNode,
  sourceRef: ContentRendererProps["sourceRef"],
  content: string,
  node: PositionedNode | undefined,
): ReactNode {
  let searchOffset = node?.position?.start?.offset ?? 0;
  const endOffset = node?.position?.end?.offset ?? content.length;
  return Children.map(children, (child) => {
    if (typeof child !== "string") return child;
    return child.split(COLOR_TOKEN_PATTERN).map((part, index) => {
      const matchOffset = content.indexOf(part, searchOffset);
      const boundedOffset =
        matchOffset >= searchOffset && matchOffset + part.length <= endOffset
          ? matchOffset
          : null;
      if (boundedOffset !== null) searchOffset = boundedOffset + part.length;
      const kind = colorLiteralKind(part);
      if (!kind) return part;
      const fallback = <span className="font-mono">{part}</span>;
      return (
        <PluginInlineFragmentRenderer
          key={`${index}:${part}`}
          sourceKind="markdown.color_literal"
          selector={kind}
          source={part}
          languageId={kind}
          sourceRef={
            boundedOffset === null
              ? rangeRef(sourceRef, node)
              : literalRangeRef(sourceRef, boundedOffset, part.length)
          }
          surfaceId="session.timeline"
          fallback={fallback}
        />
      );
    });
  });
}

function languageFromClassName(className: string | undefined) {
  return /(?:^|\s)language-([\w-]+)/.exec(className ?? "")?.[1]?.toLowerCase();
}

function rangeRef(
  sourceRef: ContentRendererProps["sourceRef"],
  node: PositionedNode | undefined,
): ContentRendererProps["sourceRef"] {
  if (sourceRef.kind !== "message") return sourceRef;
  const startOffset = node?.position?.start?.offset;
  const endOffset = node?.position?.end?.offset;
  if (startOffset === undefined || endOffset === undefined) return sourceRef;
  return {
    kind: "message_range",
    message_id: sourceRef.message_id,
    range: { start_offset: startOffset, end_offset: endOffset },
  };
}

function literalRangeRef(
  sourceRef: ContentRendererProps["sourceRef"],
  startOffset: number,
  length: number,
): ContentRendererProps["sourceRef"] {
  if (sourceRef.kind !== "message") return sourceRef;
  return {
    kind: "message_range",
    message_id: sourceRef.message_id,
    range: { start_offset: startOffset, end_offset: startOffset + length },
  };
}

type PositionedNode = {
  position?: { start?: { offset?: number }; end?: { offset?: number } };
};
