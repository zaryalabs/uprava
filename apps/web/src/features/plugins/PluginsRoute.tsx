import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowDown,
  ArrowUp,
  Check,
  Moon,
  Package,
  RotateCcw,
  Sun,
} from "lucide-react";

import { useThemeHost } from "../../plugins/ExtensionHost";
import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  ContributionRef,
  ContributionTargetResolution,
  PluginInstallationSummary,
  UpdateContributionTargetPreferencesRequest,
} from "../../shared/protocol/types";
import { Badge, type BadgeTone } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

export function PluginsRoute() {
  const queryClient = useQueryClient();
  const plugins = useQuery({
    queryKey: queryKeys.plugins,
    queryFn: coreApi.plugins,
  });
  const contributions = useQuery({
    queryKey: queryKeys.pluginContributions,
    queryFn: coreApi.pluginContributions,
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
  const preferences = useMutation({
    mutationFn: ({
      targetId,
      request,
    }: {
      targetId: string;
      request: UpdateContributionTargetPreferencesRequest;
    }) => coreApi.updatePluginContributionTarget(targetId, request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.pluginContributions,
      });
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
      {preferences.isError ? (
        <ErrorNotice
          error={preferences.error}
          title="Contribution preferences could not be changed"
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

      <section
        aria-labelledby="contribution-resolution-heading"
        className="space-y-4"
      >
        <div>
          <h2
            id="contribution-resolution-heading"
            className="text-lg font-semibold"
          >
            Contribution resolution
          </h2>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            Exclusive targets use the first available contribution. Changes are
            stored by Core and survive reloads.
          </p>
        </div>
        {contributions.isError ? (
          <ErrorNotice
            error={contributions.error}
            title="Contribution resolution is unavailable"
          />
        ) : null}
        {contributions.data?.resolutions.map((resolution) => (
          <ContributionResolutionCard
            key={resolution.target_id}
            resolution={resolution}
            busy={
              preferences.isPending &&
              preferences.variables?.targetId === resolution.target_id
            }
            onChange={(request) =>
              preferences.mutate({ targetId: resolution.target_id, request })
            }
          />
        ))}
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

function ContributionResolutionCard({
  resolution,
  busy,
  onChange,
}: {
  resolution: ContributionTargetResolution;
  busy: boolean;
  onChange: (request: UpdateContributionTargetPreferencesRequest) => void;
}) {
  const ordered = resolution.contributions.map(contributionReference);
  const disabled = resolution.contributions
    .filter((contribution) => contribution.effective_state === "disabled")
    .map(contributionReference);
  const targetLabel = contributionTargetLabel(resolution);
  const winnerIndex = resolution.contributions.findIndex(
    (contribution) => contribution.effective_state === "available",
  );
  const update = (
    nextOrder: ContributionRef[],
    nextDisabled: ContributionRef[],
  ) =>
    onChange({
      expected_revision: resolution.revision,
      ordered_contributions: nextOrder,
      disabled_contributions: nextDisabled,
    });

  return (
    <article className="border border-[var(--color-border)] bg-[var(--color-bg-raised)] p-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-medium">{resolution.extension_point}</h3>
            <Badge>{resolution.mode}</Badge>
            {resolution.conflict ? <Badge tone="warn">Conflict</Badge> : null}
          </div>
          <p className="mt-1 font-mono text-xs text-[var(--color-muted)]">
            {targetLabel}
          </p>
        </div>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() => update([], disabled)}
        >
          <RotateCcw size={15} aria-hidden="true" /> Reset order
        </Button>
      </div>
      <ol className="mt-4 space-y-2">
        {resolution.contributions.map((contribution, index) => {
          const enabled = contribution.effective_state === "available";
          const reference = contributionReference(contribution);
          return (
            <li
              key={`${contribution.plugin_id}:${contribution.contribution_id}`}
              className="flex flex-wrap items-center gap-2 border border-[var(--color-border)] p-3"
            >
              <span className="w-6 text-center text-xs text-[var(--color-muted)]">
                {index + 1}
              </span>
              <span className="min-w-0 flex-1">
                <a
                  href={`#plugin-${contribution.plugin_id}`}
                  className="block font-medium underline-offset-4 hover:underline"
                >
                  {contribution.plugin_id}
                </a>
                <span className="block truncate font-mono text-xs text-[var(--color-muted)]">
                  {contribution.contribution_id}
                </span>
              </span>
              {index === winnerIndex ? <Badge tone="good">Winner</Badge> : null}
              <Button
                variant="ghost"
                aria-label={`Move ${contribution.plugin_id} up`}
                disabled={busy || index === 0}
                onClick={() =>
                  update(moveReference(ordered, index, -1), disabled)
                }
              >
                <ArrowUp size={15} aria-hidden="true" />
              </Button>
              <Button
                variant="ghost"
                aria-label={`Move ${contribution.plugin_id} down`}
                disabled={busy || index === ordered.length - 1}
                onClick={() =>
                  update(moveReference(ordered, index, 1), disabled)
                }
              >
                <ArrowDown size={15} aria-hidden="true" />
              </Button>
              <Button
                variant={enabled ? "secondary" : "primary"}
                disabled={busy}
                onClick={() =>
                  update(
                    ordered,
                    enabled
                      ? [...disabled, reference]
                      : disabled.filter(
                          (candidate) => !sameReference(candidate, reference),
                        ),
                  )
                }
              >
                {enabled ? "Disable contribution" : "Enable contribution"}
              </Button>
            </li>
          );
        })}
      </ol>
    </article>
  );
}

function contributionReference(
  contribution: ContributionTargetResolution["contributions"][number],
): ContributionRef {
  return {
    plugin_id: contribution.plugin_id,
    contribution_id: contribution.contribution_id,
  };
}

function sameReference(left: ContributionRef, right: ContributionRef) {
  return (
    left.plugin_id === right.plugin_id &&
    left.contribution_id === right.contribution_id
  );
}

function moveReference(
  references: ContributionRef[],
  index: number,
  offset: -1 | 1,
) {
  const next = [...references];
  const target = index + offset;
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

function contributionTargetLabel(resolution: ContributionTargetResolution) {
  const target = resolution.target;
  if (target.kind === "ui_theme") return target.theme_id;
  if (target.kind === "artifact_type") return target.artifact_type;
  if (target.kind === "generated_ui_runtime") return target.runtime_id;
  if (target.kind === "generated_ui_sdk") return target.sdk_id;
  if (target.kind === "generated_ui_action_bridge") return target.bridge_id;
  const selector = target.selector ? ` · ${target.selector}` : "";
  return `${target.source_kind} · ${target.surface} · ${target.render_scope}${selector}`;
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
  const contributions = plugin.package.contributions
    .map(contributionLabel)
    .join(", ");
  return (
    <article
      id={`plugin-${plugin.package.plugin_id}`}
      className="border border-[var(--color-border)] bg-[var(--color-bg-raised)] p-5"
    >
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
          <dd className="mt-1 font-mono">{contributions || "none"}</dd>
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

function contributionLabel(
  contribution: PluginInstallationSummary["package"]["contributions"][number],
) {
  switch (contribution.kind) {
    case "ui_theme":
      return `ui.theme:${contribution.contribution.theme_id}`;
    case "visual_renderer":
      return `visual.renderer:${contribution.contribution.renderer_id}`;
    case "agent_tool":
      return `agent.tool:${contribution.tool_id}`;
    case "artifact_type":
      return `artifact.type:${contribution.contribution.artifact_type_id}`;
    case "generated_ui_runtime":
      return `generated_ui.runtime:${contribution.contribution.runtime_id}`;
    case "generated_ui_sdk":
      return `generated_ui.sdk:${contribution.contribution.sdk_id}`;
    case "generated_ui_action_bridge":
      return `generated_ui.action_bridge:${contribution.contribution.bridge_id}`;
  }
}

function effectiveStateTone(state: string): BadgeTone {
  if (state === "active") return "good";
  if (state === "incompatible" || state === "error") return "bad";
  if (state === "degraded") return "warn";
  return "neutral";
}
