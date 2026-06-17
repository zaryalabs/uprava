import type { NodePresence } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";

type Props = {
  presence: NodePresence;
  compact?: boolean;
};

export function NodeStatusBadge({ presence, compact = false }: Props) {
  const tone =
    presence === "reachable"
      ? "good"
      : presence === "stale"
        ? "warn"
        : presence === "offline"
          ? "bad"
          : "neutral";
  const label = compact
    ? presence[0]?.toUpperCase()
    : presence.replace("_", " ");
  return <Badge tone={tone}>{label}</Badge>;
}
