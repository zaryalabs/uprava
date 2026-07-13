import { Navigate, useLocation } from "react-router-dom";

import { ErrorNotice } from "../../shared/ui/error-notice";
import { LoadingState, PageHeader } from "../../shared/ui/system";
import { routeWithSearch, lastNodeId } from "../workspaces/routes";
import { useInventory } from "../inventory/api";
import { NodeEnrollmentPanel } from "./NodeEnrollmentPanel";

export function NodePairRoute() {
  return (
    <section>
      <PageHeader
        title="Pair Node"
        description="Approve a trusted local or controlled-development Node enrollment request."
        meta="NODES / ENROLLMENT"
      />
      <NodeEnrollmentPanel />
    </section>
  );
}

export function NodesCompatibilityRoute() {
  const inventory = useInventory();
  const location = useLocation();

  if (inventory.isError && !inventory.data) {
    return (
      <ErrorNotice error={inventory.error} title="Inventory load failed" />
    );
  }
  if (!inventory.data) return <LoadingState stage="Resolving Node" />;
  const preferredId = lastNodeId();
  const node =
    inventory.data.nodes.find(
      (candidate) => candidate.node_id === preferredId,
    ) ?? inventory.data.nodes[0];
  const pathname = node
    ? `/nodes/${encodeURIComponent(node.node_id)}`
    : "/nodes/pair";
  return <Navigate replace to={routeWithSearch(pathname, location.search)} />;
}
