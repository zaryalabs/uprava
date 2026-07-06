import { ShieldAlert } from "lucide-react";

import { useHealth } from "../inventory/api";

export function TrustedProfileBanner() {
  const health = useHealth();
  const hardened = health.data?.security?.mode === "hardened";
  return (
    <div
      className={`flex min-h-10 items-center gap-2 border-b px-4 text-sm ${
        hardened
          ? "border-[#bfd8ce] bg-[#e3f4ed] text-[#1d5b49]"
          : "border-[#d9ded4] bg-[#fff5ce] text-[#715b13]"
      }`}
    >
      <ShieldAlert size={16} />
      <span>
        {health.data?.profile ?? "local_trusted"} profile ·{" "}
        {hardened
          ? "local auth and CSRF enabled"
          : "trusted local or controlled development use only"}
      </span>
    </div>
  );
}
