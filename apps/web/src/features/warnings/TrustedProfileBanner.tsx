import { ShieldAlert } from "lucide-react";

import { useHealth } from "../inventory/api";

export function TrustedProfileBanner() {
  const health = useHealth();
  return (
    <div className="flex min-h-10 items-center gap-2 border-b border-[#bfd8ce] bg-[#e3f4ed] px-4 text-sm text-[#1d5b49]">
      <ShieldAlert size={16} />
      <span>
        {health.data?.profile ?? "controlled_dev"} profile · local auth and CSRF
        enabled
      </span>
    </div>
  );
}
