import { useState } from "react";

import { colorLiteralKind } from "../../source-matchers";
import type { InlineRendererProps } from "../../visual-renderers";

export function ColorTokenRenderer({ source, fallback }: InlineRendererProps) {
  const [copied, setCopied] = useState(false);
  if (!isStrictColor(source)) return fallback;

  const copy = async () => {
    if (!navigator.clipboard) return;
    await navigator.clipboard.writeText(source);
    setCopied(true);
  };

  return (
    <button
      type="button"
      className="mx-0.5 inline-flex items-center gap-1 border border-[var(--color-border)] bg-[var(--color-bg-raised)] px-1 py-0.5 font-mono text-xs text-[var(--color-ink)] hover:border-[var(--color-ink)]"
      title={`Copy color ${source}`}
      aria-label={`Color ${source}. ${copied ? "Copied" : "Copy value"}`}
      onClick={() => void copy()}
    >
      <span
        className="h-3 w-3 border border-[var(--color-border-strong)]"
        style={{ backgroundColor: source }}
        aria-hidden="true"
      />
      {source}
    </button>
  );
}

function isStrictColor(source: string) {
  return (
    colorLiteralKind(source) !== null &&
    typeof CSS !== "undefined" &&
    CSS.supports("color", source)
  );
}
