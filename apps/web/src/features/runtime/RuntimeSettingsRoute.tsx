import { useHealth, useVersion } from "../inventory/api";
import { Badge } from "../../shared/ui/badge";

export function RuntimeSettingsRoute() {
  const health = useHealth();
  const version = useVersion();
  const webRelease = import.meta.env.VITE_UPRAVA_RELEASE_ID ?? "dev";
  const releaseMatches =
    !version.data ||
    webRelease === "dev" ||
    version.data.release_id === webRelease;
  return (
    <section className="space-y-4">
      <h1 className="text-2xl font-semibold">Runtime Settings</h1>
      <div className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-4">
        <div className="flex flex-wrap gap-2">
          <Badge tone="info">{health.data?.profile ?? "unknown"}</Badge>
          <Badge tone={health.data?.status === "ok" ? "good" : "warn"}>
            {health.data?.status ?? "pending"}
          </Badge>
          <Badge tone={releaseMatches ? "good" : "bad"}>
            {releaseMatches ? "Release aligned" : "Release mismatch"}
          </Badge>
        </div>
        <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
              Core
            </dt>
            <dd className="mt-1 text-[var(--color-ink)]">
              {version.data?.name ?? "pending"} {version.data?.version ?? ""}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
              Release
            </dt>
            <dd className="mt-1 break-all font-mono text-[var(--color-ink)]">
              {version.data?.release_id ?? webRelease}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
              API
            </dt>
            <dd className="mt-1 text-[var(--color-ink)]">
              {version.data?.api_version ?? "pending"}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
              Schema
            </dt>
            <dd className="mt-1 text-[var(--color-ink)]">
              {version.data?.schema_version ?? "pending"}
            </dd>
          </div>
          <div>
            <dt className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
              Profile
            </dt>
            <dd className="mt-1 text-[var(--color-ink)]">
              {version.data?.profile ?? health.data?.profile ?? "unknown"}
            </dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
