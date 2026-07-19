import { useQuery } from "@tanstack/react-query";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { SessionEvidenceProjectionNode } from "../../shared/protocol/types";
import { runWorkbenchCommand } from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { refTitle } from "../../workbench/references/refs";
import { useOpenReference } from "../../workbench/references/use-inspector-stack";

export function EvidenceProjection({
  sessionThreadId,
}: {
  sessionThreadId: string;
}) {
  const projection = useQuery({
    queryKey: queryKeys.sessionEvidenceProjection(sessionThreadId),
    queryFn: () => coreApi.sessionEvidenceProjection(sessionThreadId),
  });

  return (
    <section>
      <div className="zarya-label">Evidence</div>
      <h2 className="mt-1 text-base font-bold">Evidence Projection</h2>
      {projection.data ? <TreeNode node={projection.data.root} /> : null}
    </section>
  );
}

function TreeNode({ node }: { node: SessionEvidenceProjectionNode }) {
  const openReference = useOpenReference();
  const inspectNode = () => {
    void runWorkbenchCommand("reference.openInInspector", {
      reference: node.primary_ref,
      openReference,
    });
  };

  return (
    <div className="mt-2 text-sm">
      <div className="flex min-w-0 items-center justify-between gap-1">
        <button
          type="button"
          className="min-w-0 flex-1 truncate border-l border-transparent px-1 py-1 text-left text-[var(--color-ink)] hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]"
          aria-label={`Inspect ${node.label}`}
          title={`Inspect ${refTitle(node.primary_ref)}`}
          onClick={inspectNode}
        >
          {node.label}
        </button>
        <ReferenceActions reference={node.primary_ref} showInspect={false} />
      </div>
      {node.children.length > 0 ? (
        <div className="ml-3 border-l border-[var(--color-border)] pl-2">
          {node.children.slice(0, 8).map((child) => (
            <TreeNode key={child.evidence_id} node={child} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
