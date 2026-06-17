import { ShieldAlert } from "lucide-react";

import { useHealth } from "../inventory/api";

export function TrustedProfileBanner() {
  const health = useHealth();
  return (
    <div className="flex min-h-10 items-center gap-2 border-b border-[#d9ded4] bg-[#fff5ce] px-4 text-sm text-[#715b13]">
      <ShieldAlert size={16} />
      <span>
        {health.data?.profile ?? "local_trusted"} profile · trusted local or
        controlled development use only
      </span>
    </div>
  );
}
