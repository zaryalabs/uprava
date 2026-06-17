import { useQuery } from "@tanstack/react-query";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { ArtifactTreeNode } from "../../shared/protocol/types";
import { runWorkbenchCommand } from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { refTitle } from "../../workbench/references/refs";
import { useOpenReference } from "../../workbench/references/use-inspector-stack";

export function ArtifactTree({ sessionThreadId }: { sessionThreadId: string }) {
  const tree = useQuery({
    queryKey: queryKeys.artifactTree(sessionThreadId),
    queryFn: () => coreApi.artifactTree(sessionThreadId),
  });

  return (
    <section className="rounded-md border border-[#d9ded4] bg-white p-3">
      <h2 className="text-sm font-semibold">Artifact Tree</h2>
      {tree.data ? <TreeNode node={tree.data.root} /> : null}
    </section>
  );
}

function TreeNode({ node }: { node: ArtifactTreeNode }) {
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
          className="min-w-0 flex-1 truncate rounded-md px-1 py-1 text-left text-[#27362f] hover:bg-[#edf1e9]"
          aria-label={`Inspect ${node.label}`}
          title={`Inspect ${refTitle(node.primary_ref)}`}
          onClick={inspectNode}
        >
          {node.label}
        </button>
        <ReferenceActions reference={node.primary_ref} showInspect={false} />
      </div>
      {node.children.length > 0 ? (
        <div className="ml-3 border-l border-[#d9ded4] pl-2">
          {node.children.slice(0, 8).map((child) => (
            <TreeNode key={child.artifact_id} node={child} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
