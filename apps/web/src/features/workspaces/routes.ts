export const WORKSPACE_SURFACES = ["agent", "workbench", "jobs"] as const;

export type WorkspaceSurface = (typeof WORKSPACE_SURFACES)[number];

const ROUTE_PREFERENCES_KEY = "uprava.workspace-routes.v1";
const ROUTE_PREFERENCES_VERSION = 1;
const MAX_WORKSPACE_PREFERENCES = 50;

type RoutePreferences = {
  version: typeof ROUTE_PREFERENCES_VERSION;
  lastWorkspaceId?: string;
  lastNodeId?: string;
  workspaceSurfaces: Record<string, WorkspaceSurface>;
};

const emptyPreferences = (): RoutePreferences => ({
  version: ROUTE_PREFERENCES_VERSION,
  workspaceSurfaces: {},
});

export function workspaceRoute(placementId: string) {
  return `/workspaces/${encodeURIComponent(placementId)}`;
}

export function workspaceSurfaceRoute(
  placementId: string,
  surface: WorkspaceSurface,
) {
  return `${workspaceRoute(placementId)}/${surface}`;
}

export function workspaceAgentRoute(placementId: string) {
  return workspaceSurfaceRoute(placementId, "agent");
}

export function workspaceAgentSessionRoute(
  placementId: string,
  sessionThreadId: string,
) {
  return `${workspaceAgentRoute(placementId)}/${encodeURIComponent(sessionThreadId)}`;
}

export function workspaceWorkbenchRoute(placementId: string) {
  return workspaceSurfaceRoute(placementId, "workbench");
}

export function workspaceJobsRoute(placementId: string) {
  return workspaceSurfaceRoute(placementId, "jobs");
}

export function workspaceJobNewRoute(placementId: string) {
  return `${workspaceJobsRoute(placementId)}/new`;
}

export function workspaceJobRoute(placementId: string, jobId: string) {
  return `${workspaceJobsRoute(placementId)}/${encodeURIComponent(jobId)}`;
}

export function workspaceJobRunRoute(
  placementId: string,
  jobId: string,
  jobRunId: string,
) {
  return `${workspaceJobRoute(placementId, jobId)}/runs/${encodeURIComponent(jobRunId)}`;
}

export function routeWithSearch(
  pathname: string,
  search: string | URLSearchParams,
) {
  const query =
    typeof search === "string" ? search.replace(/^\?/u, "") : search.toString();
  return query ? `${pathname}?${query}` : pathname;
}

export function workspaceSurfaceFromPathname(
  placementId: string,
  pathname: string,
): WorkspaceSurface | null {
  const prefix = `${workspaceRoute(placementId)}/`;
  if (!pathname.startsWith(prefix)) return null;
  const surface = pathname.slice(prefix.length).split("/", 1)[0];
  return isWorkspaceSurface(surface) ? surface : null;
}

export function preferredWorkspaceSurface(placementId: string) {
  return readRoutePreferences().workspaceSurfaces[placementId] ?? "agent";
}

export function preferredWorkspaceRoute(placementId: string) {
  return workspaceSurfaceRoute(
    placementId,
    preferredWorkspaceSurface(placementId),
  );
}

export function lastWorkspaceId() {
  return readRoutePreferences().lastWorkspaceId ?? null;
}

export function lastNodeId() {
  return readRoutePreferences().lastNodeId ?? null;
}

export function rememberWorkspaceRoute(
  placementId: string,
  nodeId: string,
  surface: WorkspaceSurface | null,
) {
  const current = readRoutePreferences();
  const workspaceSurfaces = { ...current.workspaceSurfaces };
  if (surface) {
    delete workspaceSurfaces[placementId];
    workspaceSurfaces[placementId] = surface;
  }
  writeRoutePreferences({
    ...current,
    lastWorkspaceId: placementId,
    lastNodeId: nodeId,
    workspaceSurfaces: Object.fromEntries(
      Object.entries(workspaceSurfaces).slice(-MAX_WORKSPACE_PREFERENCES),
    ),
  });
}

export function rememberNodeRoute(nodeId: string) {
  const current = readRoutePreferences();
  writeRoutePreferences({ ...current, lastNodeId: nodeId });
}

function readRoutePreferences(): RoutePreferences {
  if (typeof window === "undefined") return emptyPreferences();
  try {
    const value = JSON.parse(
      window.localStorage.getItem(ROUTE_PREFERENCES_KEY) ?? "null",
    ) as unknown;
    if (!isRecord(value) || value.version !== ROUTE_PREFERENCES_VERSION) {
      return emptyPreferences();
    }
    const workspaceSurfaces = isRecord(value.workspaceSurfaces)
      ? Object.fromEntries(
          Object.entries(value.workspaceSurfaces)
            .filter((entry): entry is [string, WorkspaceSurface] =>
              isWorkspaceSurface(entry[1]),
            )
            .slice(-MAX_WORKSPACE_PREFERENCES),
        )
      : {};
    return {
      version: ROUTE_PREFERENCES_VERSION,
      lastWorkspaceId: boundedString(value.lastWorkspaceId),
      lastNodeId: boundedString(value.lastNodeId),
      workspaceSurfaces,
    };
  } catch {
    return emptyPreferences();
  }
}

function writeRoutePreferences(preferences: RoutePreferences) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      ROUTE_PREFERENCES_KEY,
      JSON.stringify(preferences),
    );
  } catch {
    // Routing still works when storage is unavailable or full.
  }
}

function isWorkspaceSurface(value: unknown): value is WorkspaceSurface {
  return (
    typeof value === "string" &&
    WORKSPACE_SURFACES.includes(value as WorkspaceSurface)
  );
}

function boundedString(value: unknown) {
  return typeof value === "string" && value.length > 0 && value.length <= 512
    ? value
    : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
