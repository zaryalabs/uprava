import { lazy, Suspense } from "react";
import { Route, Routes } from "react-router-dom";

import { AppShell } from "./shell/AppShell";
import { AuthGate } from "../features/auth/AuthGate";
import { WorkspaceDraftProvider } from "../features/workspace-inspector/WorkspaceDrafts";

const DashboardRoute = lazy(() =>
  import("../features/dashboard/DashboardRoute").then((module) => ({
    default: module.DashboardRoute,
  })),
);
const NodeDetailRoute = lazy(() =>
  import("../features/nodes/NodeDetailRoute").then((module) => ({
    default: module.NodeDetailRoute,
  })),
);
const NodesRoute = lazy(() =>
  import("../features/nodes/NodesRoute").then((module) => ({
    default: module.NodesRoute,
  })),
);
const PlacementNewRoute = lazy(() =>
  import("../features/placements/PlacementNewRoute").then((module) => ({
    default: module.PlacementNewRoute,
  })),
);
const PlacementRoute = lazy(() =>
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
const SessionRoute = lazy(() =>
  import("../features/sessions/SessionRoute").then((module) => ({
    default: module.SessionRoute,
  })),
);

export function App() {
  return (
    <AuthGate>
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
              path="/nodes"
              element={
                <LazyRoute>
                  <NodesRoute />
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
              path="/projects/:projectId"
              element={
                <LazyRoute>
                  <ProjectRoute />
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
              path="/workspaces/:placementId"
              element={
                <LazyRoute>
                  <PlacementRoute />
                </LazyRoute>
              }
            />
            <Route
              path="/sessions/:sessionThreadId"
              element={
                <LazyRoute>
                  <SessionRoute />
                </LazyRoute>
              }
            />
            <Route
              path="/settings/runtime"
              element={
                <LazyRoute>
                  <RuntimeSettingsRoute />
                </LazyRoute>
              }
            />
          </Route>
        </Routes>
      </WorkspaceDraftProvider>
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
