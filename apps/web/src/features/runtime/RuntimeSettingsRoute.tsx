import { useHealth, useVersion } from "../inventory/api";
import { Badge } from "../../shared/ui/badge";

export function RuntimeSettingsRoute() {
  const health = useHealth();
  const version = useVersion();
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-semibold">Runtime Settings</h1>
      <div className="rounded-md border border-[#d9ded4] bg-white p-4">
        <div className="flex flex-wrap gap-2">
          <Badge tone="info">{health.data?.profile ?? "unknown"}</Badge>
          <Badge tone={health.data?.status === "ok" ? "good" : "warn"}>
            {health.data?.status ?? "pending"}
          </Badge>
        </div>
        <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
              Core
            </dt>
            <dd className="mt-1 text-[#1f2a24]">
              {version.data?.name ?? "pending"} {version.data?.version ?? ""}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
              API
            </dt>
            <dd className="mt-1 text-[#1f2a24]">
              {version.data?.api_version ?? "pending"}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
              Schema
            </dt>
            <dd className="mt-1 text-[#1f2a24]">
              {version.data?.schema_version ?? "pending"}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[#667268]">
              Profile
            </dt>
            <dd className="mt-1 text-[#1f2a24]">
              {version.data?.profile ?? health.data?.profile ?? "unknown"}
            </dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
