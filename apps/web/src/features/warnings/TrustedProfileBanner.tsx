import { ShieldAlert } from "lucide-react";

import { useHealth } from "../inventory/api";

export function TrustedProfileBanner() {
  const health = useHealth();
  return (
    <div className="flex min-h-10 items-center gap-2 border-b border-[var(--color-muted)] bg-[var(--color-bg-muted)] px-4 text-sm text-[var(--color-muted)]">
      <ShieldAlert size={16} />
      <span>
        {health.data?.profile ?? "controlled_dev"} profile · local auth and CSRF
        enabled
      </span>
    </div>
  );
}
