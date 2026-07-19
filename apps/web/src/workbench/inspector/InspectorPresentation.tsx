import { Minus } from "lucide-react";

import type { UpravaRef } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { runWorkbenchCommand } from "../commands/registry";
import { ReferenceActions } from "../references/ReferenceActions";
import { refKindLabel, refTitle } from "../references/refs";
import type { useOpenReference } from "../references/use-inspector-stack";

export type InspectorStatus = "resolved" | "not_available" | "not_implemented";

export type InspectorRow = {
  label: string;
  value: string | number | boolean | null | undefined;
};

export type InspectorRefLink = {
  label: string;
  ref: UpravaRef;
  aspect?: "source" | "evidence" | "cause" | "result" | "raw" | "related";
};

export type InspectorDetail = {
  title: string;
  status: InspectorStatus;
  rows: InspectorRow[];
  refs: InspectorRefLink[];
  payload?: unknown;
};

export function InspectorPresentation({
  stack,
  selected,
  detail,
  openReference,
  closeTop,
  selectStackIndex,
}: {
  stack: UpravaRef[];
  selected: UpravaRef | null;
  detail: InspectorDetail | null;
  openReference: ReturnType<typeof useOpenReference>;
  closeTop: () => void;
  selectStackIndex: (index: number) => void;
}) {
  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-xs font-bold text-[var(--color-muted)]">
          Context Inspector
        </h2>
        {selected ? (
          <Button
            type="button"
            variant="ghost"
            className="h-7 w-7 px-0"
            aria-label={
              stack.length === 1
                ? "Close Inspector"
                : "Return to Previous Inspector Layer"
            }
            title={
              stack.length === 1
                ? "Close Inspector"
                : "Return to Previous Layer"
            }
            onClick={closeTop}
          >
            <Minus size={14} aria-hidden="true" />
          </Button>
        ) : null}
      </div>

      {stack.length > 0 ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1 text-xs">
          {stack.slice(-3).map((reference, visibleIndex) => {
            const index = Math.max(0, stack.length - 3) + visibleIndex;
            return (
              <button
                key={`${index}:${refTitle(reference)}`}
                type="button"
                className={`inline-flex max-w-full items-center gap-1 border px-2 py-1 text-left ${
                  index === stack.length - 1
                    ? "border-[var(--color-ink)] text-[var(--color-ink)]"
                    : "border-transparent text-[var(--color-muted)] hover:border-[var(--color-muted)]"
                }`}
                onClick={() => selectStackIndex(index)}
              >
                {index > 0 ? <span aria-hidden="true">/</span> : null}
                <span className="truncate">{refKindLabel(reference)}</span>
              </button>
            );
          })}
        </div>
      ) : null}

      {selected && detail ? (
        <article className="border-t border-[var(--color-border)] pt-4">
          <div className="mb-3 flex items-start justify-between gap-2">
            <div className="min-w-0">
              <Badge tone={statusTone(detail.status)}>
                {statusLabel(detail.status)}
              </Badge>
              <h3 className="mt-2 break-words text-base font-bold">
                {detail.title}
              </h3>
            </div>
            <ReferenceActions reference={selected} showInspect={false} />
          </div>
          <dl className="mt-4 grid gap-3">
            {detail.rows.map((row) => (
              <div key={row.label} className="min-w-0">
                <dt className="text-xs text-[var(--color-muted)]">
                  {row.label}
                </dt>
                <dd className="break-words font-mono text-xs text-[var(--color-ink)]">
                  {formatValue(row.value)}
                </dd>
              </div>
            ))}
          </dl>
          {detail.refs.length > 0 ? (
            <div className="mt-4 space-y-2">
              <div className="text-xs font-bold text-[var(--color-muted)]">
                Causality
              </div>
              {groupRefLinks(detail.refs).map(([aspect, refs]) => (
                <div
                  key={aspect}
                  className="border-l border-[var(--color-border)] pl-2"
                >
                  <div className="mb-1 text-[10px] font-bold uppercase tracking-wide text-[var(--color-muted)]">
                    {aspect}
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {refs.map((item, index) => (
                      <button
                        key={`${index}:${refTitle(item.ref)}`}
                        type="button"
                        className="max-w-full truncate border border-[var(--color-muted)] bg-[var(--color-bg)] px-2 py-1 text-left text-xs hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]"
                        title={refTitle(item.ref)}
                        onClick={() => {
                          void runWorkbenchCommand(
                            "reference.openInInspector",
                            {
                              reference: item.ref,
                              openReference,
                            },
                          );
                        }}
                      >
                        {item.label}
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : null}
          {detail.payload !== undefined ? (
            <details className="mt-4 border-t border-[var(--color-border)] pt-3">
              <summary className="cursor-pointer text-xs font-bold text-[var(--color-muted)]">
                Raw payload
              </summary>
              <pre className="mt-2 max-h-64 overflow-auto border-l border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-2 text-xs text-[var(--color-ink)]">
                {safeJson(detail.payload)}
              </pre>
            </details>
          ) : null}
        </article>
      ) : (
        <div className="py-4 text-sm text-[var(--color-muted)]">
          Select a source, cause, or evidence reference to inspect it here.
        </div>
      )}
    </section>
  );
}

function groupRefLinks(refs: InspectorRefLink[]) {
  const order: NonNullable<InspectorRefLink["aspect"]>[] = [
    "source",
    "evidence",
    "cause",
    "result",
    "raw",
    "related",
  ];
  return order
    .map(
      (aspect) =>
        [
          aspect,
          refs.filter((item) => (item.aspect ?? "related") === aspect),
        ] as const,
    )
    .filter(([, items]) => items.length > 0);
}

function formatValue(value: InspectorRow["value"]) {
  if (value === null || value === undefined || value === "") {
    return "unavailable";
  }
  return String(value);
}

function safeJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return "[unserializable]";
  }
}

function statusTone(status: InspectorStatus) {
  if (status === "resolved") return "good" as const;
  if (status === "not_implemented") return "info" as const;
  return "warn" as const;
}

function statusLabel(status: InspectorStatus) {
  if (status === "resolved") return "Reference available";
  if (status === "not_implemented") return "Preview unavailable";
  return "Reference unavailable";
}
