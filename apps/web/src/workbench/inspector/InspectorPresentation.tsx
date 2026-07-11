import { ChevronRight, X } from "lucide-react";

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
    <section className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
          Inspector
        </h2>
        {selected ? (
          <Button
            type="button"
            variant="ghost"
            className="h-7 w-7 px-0"
            aria-label="Close inspector"
            title="Close inspector"
            onClick={closeTop}
          >
            <X size={14} />
          </Button>
        ) : null}
      </div>

      {stack.length > 0 ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1 text-xs">
          {stack.map((reference, index) => (
            <button
              key={`${index}:${refTitle(reference)}`}
              type="button"
              className={`inline-flex max-w-full items-center gap-1 rounded-md border px-2 py-1 text-left ${
                index === stack.length - 1
                  ? "border-[#9ccdbd] bg-[#e4f4ef] text-[#1f6559]"
                  : "border-[#d9ded4] bg-white text-[#536257]"
              }`}
              onClick={() => selectStackIndex(index)}
            >
              {index > 0 ? <ChevronRight size={12} /> : null}
              <span className="truncate">{refKindLabel(reference)}</span>
            </button>
          ))}
        </div>
      ) : null}

      {selected && detail ? (
        <article className="rounded-md border border-[#d9ded4] bg-white p-3">
          <div className="mb-3 flex items-start justify-between gap-2">
            <div className="min-w-0">
              <Badge tone={statusTone(detail.status)}>{detail.status}</Badge>
              <h3 className="mt-2 truncate text-sm font-semibold">
                {detail.title}
              </h3>
            </div>
            <ReferenceActions reference={selected} showInspect={false} />
          </div>
          <dl className="space-y-2">
            {detail.rows.map((row) => (
              <div key={row.label} className="min-w-0">
                <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
                  {row.label}
                </dt>
                <dd className="break-words font-mono text-xs text-[#27362f]">
                  {formatValue(row.value)}
                </dd>
              </div>
            ))}
          </dl>
          {detail.refs.length > 0 ? (
            <div className="mt-4 space-y-2">
              <div className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
                References
              </div>
              <div className="flex flex-wrap gap-1.5">
                {detail.refs.map((item, index) => (
                  <button
                    key={`${index}:${refTitle(item.ref)}`}
                    type="button"
                    className="max-w-full truncate rounded-md border border-[#d9ded4] bg-[#f8faf5] px-2 py-1 text-left text-xs hover:bg-[#edf1e9]"
                    title={refTitle(item.ref)}
                    onClick={() => {
                      void runWorkbenchCommand("reference.openInInspector", {
                        reference: item.ref,
                        openReference,
                      });
                    }}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {detail.payload !== undefined ? (
            <pre className="mt-4 max-h-64 overflow-auto rounded-md border border-[#e0e5dc] bg-[#f8faf5] p-2 text-xs text-[#27362f]">
              {safeJson(detail.payload)}
            </pre>
          ) : null}
        </article>
      ) : (
        <div className="rounded-md border border-[#d9ded4] bg-white p-3 text-sm text-[#536257]">
          No reference selected.
        </div>
      )}
    </section>
  );
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
