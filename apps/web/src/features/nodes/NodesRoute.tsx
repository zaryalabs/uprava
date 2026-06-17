import { Link } from "react-router-dom";
import { Plus } from "lucide-react";

import { useInventory } from "../inventory/api";
import { HeartbeatAge } from "./HeartbeatAge";
import { NodeEnrollmentPanel } from "./NodeEnrollmentPanel";
import { NodeStatusBadge } from "./NodeStatusBadge";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";

export function NodesRoute() {
  const inventory = useInventory();

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">Nodes</h1>
          <p className="text-sm text-[#536257]">
            Registered runtime environments and current heartbeat state.
          </p>
        </div>
      </div>
      <NodeEnrollmentPanel />
      <div className="grid gap-3">
        {inventory.data?.nodes.map((node) => (
          <article
            key={node.node_id}
            className="rounded-md border border-[#d9ded4] bg-white p-4 shadow-sm"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <Link
                  to={`/nodes/${node.node_id}`}
                  className="text-lg font-semibold hover:underline"
                >
                  {node.display_name}
                </Link>
                <div className="mt-1 text-sm text-[#536257]">
                  heartbeat{" "}
                  <HeartbeatAge seconds={node.heartbeat_age_seconds} />
                </div>
              </div>
              <NodeStatusBadge presence={node.presence} />
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              <Badge tone={node.active_runtime_count > 0 ? "info" : "neutral"}>
                {node.active_runtime_count} active sessions
              </Badge>
              {node.capabilities.map((capability) => (
                <Badge key={capability.key}>{capability.key}</Badge>
              ))}
            </div>
            <div className="mt-4">
              <Link to={`/nodes/${node.node_id}/placements/new`}>
                <Button>
                  <Plus size={15} />
                  Workspace
                </Button>
              </Link>
            </div>
          </article>
        ))}
      </div>
      {inventory.data?.nodes.length === 0 ? (
        <div className="rounded-md border border-[#cad2c7] bg-white p-5 text-sm text-[#536257]">
          Start a Node daemon and heartbeat will populate this list.
        </div>
      ) : null}
    </section>
  );
}
