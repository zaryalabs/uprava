import type { NodePresence } from "../../shared/protocol/types";
import { StatusIndicator } from "../../shared/ui/status-indicator";

type Props = {
  presence: NodePresence;
  compact?: boolean;
};

export function NodeStatusBadge({ presence, compact = false }: Props) {
  return (
    <StatusIndicator compact={compact} dimension="presence" value={presence} />
  );
}
