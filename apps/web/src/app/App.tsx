import { lazy, Suspense } from "react";
import { Route, Routes } from "react-router-dom";

import { AuthGate } from "../features/auth/AuthGate";
import { WorkspaceDraftProvider } from "../features/workspace-inspector/WorkspaceDrafts";
import { ExtensionHostProvider } from "../plugins/ExtensionHost";
import { AppShell } from "./shell/AppShell";

const DashboardRoute = lazy(() =>
  import("../features/dashboard/DashboardRoute").then((module) => ({
    default: module.DashboardRoute,
  })),
);
const JobCompatibilityRoute = lazy(() =>
  import("../features/jobs/WorkspaceJobRoutes").then((module) => ({
    default: module.JobCompatibilityRoute,
  })),
);
const JobRunCompatibilityRoute = lazy(() =>
  import("../features/jobs/WorkspaceJobRoutes").then((module) => ({
    default: module.JobRunCompatibilityRoute,
  })),
);
const JobsCompatibilityRoute = lazy(() =>
  import("../features/jobs/WorkspaceJobRoutes").then((module) => ({
    default: module.JobsCompatibilityRoute,
  })),
);
const JobsRoute = lazy(() =>
  import("../features/jobs/JobsRoute").then((module) => ({
    default: module.JobsRoute,
  })),
);
const WorkspaceJobsIndexRoute = lazy(() =>
  import("../features/jobs/JobsRoute").then((module) => ({
    default: module.WorkspaceJobsIndexRoute,
  })),
);
const JobCreateRoute = lazy(() =>
  import("../features/jobs/JobCreateRoute").then((module) => ({
    default: module.JobCreateRoute,
  })),
);
const WorkspaceJobDetailRoute = lazy(() =>
  import("../features/jobs/JobDetailRoute").then((module) => ({
    default: module.JobDetailRoute,
  })),
);
const WorkspaceJobRunRoute = lazy(() =>
  import("../features/jobs/JobRunRoute").then((module) => ({
    default: module.JobRunRoute,
  })),
);
const NodeCompatibilityRoute = lazy(() =>
  import("../features/nodes/NodeCompatibilityRoutes").then((module) => ({
    default: module.NodesCompatibilityRoute,
  })),
);
const NodePairRoute = lazy(() =>
  import("../features/nodes/NodeCompatibilityRoutes").then((module) => ({
    default: module.NodePairRoute,
  })),
);
const NodeDetailRoute = lazy(() =>
  import("../features/nodes/NodeDetailRoute").then((module) => ({
    default: module.NodeDetailRoute,
  })),
);
const PlacementNewRoute = lazy(() =>
  import("../features/placements/PlacementNewRoute").then((module) => ({
    default: module.PlacementNewRoute,
  })),
);
const WorkspaceWorkbenchRoute = lazy(() =>
  import("../features/placements/PlacementRoute").then((module) => ({
    default: module.PlacementRoute,
  })),
);
const ProjectRoute = lazy(() =>
  import("../features/projects/ProjectRoute").then((module) => ({
    default: module.ProjectRoute,
  })),
);
const RuntimeSettingsRoute = lazy(() =>
  import("../features/runtime/RuntimeSettingsRoute").then((module) => ({
    default: module.RuntimeSettingsRoute,
  })),
);
const ToolingRoute = lazy(() =>
  import("../features/tooling/ToolingRoute").then((module) => ({
    default: module.ToolingRoute,
  })),
);
const PluginsRoute = lazy(() =>
  import("../features/plugins/PluginsRoute").then((module) => ({
    default: module.PluginsRoute,
  })),
);
const SessionCompatibilityRoute = lazy(() =>
  import("../features/sessions/WorkspaceAgentRoute").then((module) => ({
    default: module.SessionCompatibilityRoute,
  })),
);
const WorkspaceAgentRoute = lazy(() =>
  import("../features/sessions/WorkspaceAgentRoute").then((module) => ({
    default: module.WorkspaceAgentRoute,
  })),
);
const WorkspaceSessionRoute = lazy(() =>
  import("../features/sessions/WorkspaceAgentRoute").then((module) => ({
    default: module.WorkspaceSessionRoute,
  })),
);
const WorkspaceLayout = lazy(() =>
  import("../features/workspaces/WorkspaceLayout").then((module) => ({
    default: module.WorkspaceLayout,
  })),
);
const WorkspaceResolverRoute = lazy(() =>
  import("../features/workspaces/WorkspaceLayout").then((module) => ({
    default: module.WorkspaceResolverRoute,
  })),
);

export function App() {
  return (
    <AuthGate>
      <ExtensionHostProvider>
        <WorkspaceDraftProvider>
          <Routes>
            <Route element={<AppShell />}>
              <Route
                index
                element={
                  <LazyRoute>
                    <DashboardRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/dashboard"
                element={
                  <LazyRoute>
                    <DashboardRoute />
                  </LazyRoute>
                }
              />

              <Route
                path="/jobs"
                element={
                  <LazyRoute>
                    <JobsCompatibilityRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/jobs/:jobId"
                element={
                  <LazyRoute>
                    <JobCompatibilityRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/job-runs/:jobRunId"
                element={
                  <LazyRoute>
                    <JobRunCompatibilityRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/sessions/:sessionThreadId"
                element={
                  <LazyRoute>
                    <SessionCompatibilityRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/nodes"
                element={
                  <LazyRoute>
                    <NodeCompatibilityRoute />
                  </LazyRoute>
                }
              />

              <Route
                path="/nodes/pair"
                element={
                  <LazyRoute>
                    <NodePairRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/nodes/:nodeId"
                element={
                  <LazyRoute>
                    <NodeDetailRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/nodes/:nodeId/placements/new"
                element={
                  <LazyRoute>
                    <PlacementNewRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/projects/:projectId"
                element={
                  <LazyRoute>
                    <ProjectRoute />
                  </LazyRoute>
                }
              />

              <Route
                path="/workspaces/:placementId"
                element={
                  <LazyRoute>
                    <WorkspaceLayout />
                  </LazyRoute>
                }
              >
                <Route
                  index
                  element={
                    <LazyRoute>
                      <WorkspaceResolverRoute />
                    </LazyRoute>
                  }
                />
                <Route
                  path="agent"
                  element={
                    <LazyRoute>
                      <WorkspaceAgentRoute />
                    </LazyRoute>
                  }
                />
                <Route
                  path="agent/:sessionThreadId"
                  element={
                    <LazyRoute>
                      <WorkspaceSessionRoute />
                    </LazyRoute>
                  }
                />
                <Route
                  path="workbench"
                  element={
                    <LazyRoute>
                      <WorkspaceWorkbenchRoute />
                    </LazyRoute>
                  }
                />
                <Route
                  path="jobs"
                  element={
                    <LazyRoute>
                      <JobsRoute />
                    </LazyRoute>
                  }
                >
                  <Route
                    index
                    element={
                      <LazyRoute>
                        <WorkspaceJobsIndexRoute />
                      </LazyRoute>
                    }
                  />
                  <Route
                    path="new"
                    element={
                      <LazyRoute>
                        <JobCreateRoute />
                      </LazyRoute>
                    }
                  />
                  <Route
                    path=":jobId"
                    element={
                      <LazyRoute>
                        <WorkspaceJobDetailRoute />
                      </LazyRoute>
                    }
                  />
                  <Route
                    path=":jobId/runs/:jobRunId"
                    element={
                      <LazyRoute>
                        <WorkspaceJobRunRoute />
                      </LazyRoute>
                    }
                  />
                </Route>
              </Route>

              <Route
                path="/settings/runtime"
                element={
                  <LazyRoute>
                    <RuntimeSettingsRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/settings/tooling"
                element={
                  <LazyRoute>
                    <ToolingRoute />
                  </LazyRoute>
                }
              />
              <Route
                path="/settings/plugins"
                element={
                  <LazyRoute>
                    <PluginsRoute />
                  </LazyRoute>
                }
              />
            </Route>
          </Routes>
        </WorkspaceDraftProvider>
      </ExtensionHostProvider>
    </AuthGate>
  );
}

function LazyRoute({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<RouteLoading />}>{children}</Suspense>;
}

function RouteLoading() {
  return (
    <div className="flex min-h-40 items-center justify-center text-sm text-muted-foreground">
      Loading…
    </div>
  );
}
