import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Moon, Package, Sun } from "lucide-react";

import { useThemeHost } from "../../plugins/ExtensionHost";
import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { PluginInstallationSummary } from "../../shared/protocol/types";
import { Badge, type BadgeTone } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

export function PluginsRoute() {
  const queryClient = useQueryClient();
  const plugins = useQuery({
    queryKey: queryKeys.plugins,
    queryFn: coreApi.plugins,
  });
  const lifecycle = useMutation({
    mutationFn: ({
      pluginId,
      enable,
    }: {
      pluginId: string;
      enable: boolean;
    }) =>
      enable ? coreApi.enablePlugin(pluginId) : coreApi.disablePlugin(pluginId),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.plugins }),
        queryClient.invalidateQueries({
          queryKey: queryKeys.pluginContributions,
        }),
      ]);
    },
  });
  const themeHost = useThemeHost();

  return (
    <section className="mx-auto max-w-5xl space-y-8">
      <header className="space-y-2 border-b border-[var(--color-border)] pb-5">
        <p className="text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
          Extension Host
        </p>
        <h1 className="text-2xl font-semibold">Plugins &amp; Appearance</h1>
        <p className="max-w-3xl text-sm text-[var(--color-muted)]">
          Core owns installed and effective plugin state. Appearance is a local
          preference resolved only against currently active contributions.
        </p>
      </header>

      {plugins.isError ? (
        <ErrorNotice error={plugins.error} title="Plugins are unavailable" />
      ) : null}
      {lifecycle.isError ? (
        <ErrorNotice
          error={lifecycle.error}
          title="Plugin state could not be changed"
        />
      ) : null}

      <section
        aria-labelledby="installed-plugins-heading"
        className="space-y-4"
      >
        <div className="flex items-center gap-2">
          <Package size={18} aria-hidden="true" />
          <h2 id="installed-plugins-heading" className="text-lg font-semibold">
            Installed Plugins
          </h2>
        </div>
        {plugins.isLoading ? (
          <p className="text-sm text-[var(--color-muted)]">Loading plugins…</p>
        ) : plugins.data?.items.length ? (
          <div className="grid gap-4">
            {plugins.data.items.map((plugin) => (
              <PluginCard
                key={plugin.package.plugin_id}
                plugin={plugin}
                busy={
                  lifecycle.isPending &&
                  lifecycle.variables?.pluginId === plugin.package.plugin_id
                }
                onToggle={(enable) =>
                  lifecycle.mutate({
                    pluginId: plugin.package.plugin_id,
                    enable,
                  })
                }
              />
            ))}
          </div>
        ) : (
          <p className="border border-dashed border-[var(--color-border-strong)] p-5 text-sm text-[var(--color-muted)]">
            No plugin packages are installed.
          </p>
        )}
      </section>

      <section aria-labelledby="appearance-heading" className="space-y-4">
        <div>
          <h2 id="appearance-heading" className="text-lg font-semibold">
            Appearance
          </h2>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            A disabled or unavailable selection always falls back to Core Light.
          </p>
        </div>
        {themeHost.isError ? (
          <p role="status" className="text-sm text-[var(--color-risk)]">
            Plugin themes could not be loaded. Core Light remains active.
          </p>
        ) : null}
        <fieldset className="grid gap-3 sm:grid-cols-2">
          <legend className="sr-only">Theme</legend>
          {themeHost.themes.map((theme) => {
            const active = themeHost.effectiveTheme.theme_id === theme.theme_id;
            return (
              <label
                key={theme.theme_id}
                className={`flex cursor-pointer items-center gap-3 border p-4 ${
                  active
                    ? "border-[var(--color-ink)] bg-[var(--color-bg-raised)]"
                    : "border-[var(--color-border)] hover:border-[var(--color-border-strong)]"
                }`}
              >
                <input
                  type="radio"
                  name="uprava-theme"
                  value={theme.theme_id}
                  checked={active}
                  onChange={() => themeHost.selectTheme(theme.theme_id)}
                />
                {theme.kind === "dark" ? (
                  <Moon size={18} aria-hidden="true" />
                ) : (
                  <Sun size={18} aria-hidden="true" />
                )}
                <span className="min-w-0 flex-1">
                  <span className="block font-medium">{theme.label}</span>
                  <span className="block truncate font-mono text-xs text-[var(--color-muted)]">
                    {theme.theme_id}
                  </span>
                </span>
                {active ? <Check size={17} aria-hidden="true" /> : null}
              </label>
            );
          })}
        </fieldset>
      </section>
    </section>
  );
}

function PluginCard({
  plugin,
  busy,
  onToggle,
}: {
  plugin: PluginInstallationSummary;
  busy: boolean;
  onToggle: (enable: boolean) => void;
}) {
  const active = plugin.effective_state === "active";
  const themeContribution = plugin.package.contributions.find(
    (contribution) => contribution.kind === "ui_theme",
  );
  return (
    <article className="border border-[var(--color-border)] bg-[var(--color-bg-raised)] p-5">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0 space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-semibold">{plugin.package.display_name}</h3>
            <Badge tone={effectiveStateTone(plugin.effective_state)}>
              {plugin.effective_state}
            </Badge>
            <Badge>{plugin.package.trust_level.replaceAll("_", " ")}</Badge>
          </div>
          <p className="text-sm text-[var(--color-muted)]">
            {plugin.package.description}
          </p>
          <p className="font-mono text-xs text-[var(--color-muted)]">
            {plugin.package.plugin_id}@{plugin.package.version} ·{" "}
            {plugin.package.publisher} · {plugin.package.install_source}
          </p>
        </div>
        <Button
          variant={active ? "secondary" : "primary"}
          disabled={busy || plugin.compatibility.state === "incompatible"}
          onClick={() => onToggle(!active)}
        >
          {busy ? "Updating…" : active ? "Disable" : "Enable"}
        </Button>
      </div>

      <dl className="mt-5 grid gap-3 border-t border-[var(--color-border)] pt-4 text-sm sm:grid-cols-3">
        <div>
          <dt className="text-xs text-[var(--color-muted)]">Contribution</dt>
          <dd className="mt-1 font-mono">
            {themeContribution?.kind === "ui_theme"
              ? `ui.theme:${themeContribution.contribution.theme_id}`
              : "none"}
          </dd>
        </div>
        <div>
          <dt className="text-xs text-[var(--color-muted)]">Permission</dt>
          <dd className="mt-1 font-mono">
            {plugin.package.requested_permissions.join(", ") || "none"}
          </dd>
        </div>
        <div>
          <dt className="text-xs text-[var(--color-muted)]">Compatibility</dt>
          <dd className="mt-1">{plugin.compatibility.state}</dd>
        </div>
      </dl>
      {plugin.compatibility.diagnostics.length ? (
        <ul className="mt-3 list-disc pl-5 text-sm text-[var(--color-risk)]">
          {plugin.compatibility.diagnostics.map((diagnostic) => (
            <li key={diagnostic}>{diagnostic}</li>
          ))}
        </ul>
      ) : null}
    </article>
  );
}

function effectiveStateTone(state: string): BadgeTone {
  if (state === "active") return "good";
  if (state === "incompatible" || state === "error") return "bad";
  if (state === "degraded") return "warn";
  return "neutral";
}
