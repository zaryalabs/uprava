import type { ContentRendererProps } from "../../content-renderers";

export function PlainTextRenderer({ content }: ContentRendererProps) {
  return <p className="whitespace-pre-wrap break-words">{content}</p>;
}
