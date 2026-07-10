import { Route, Routes } from "react-router-dom";

import { AppShell } from "./shell/AppShell";
import { DashboardRoute } from "../features/dashboard/DashboardRoute";
import { NodeDetailRoute } from "../features/nodes/NodeDetailRoute";
import { NodesRoute } from "../features/nodes/NodesRoute";
import { PlacementNewRoute } from "../features/placements/PlacementNewRoute";
import { PlacementRoute } from "../features/placements/PlacementRoute";
import { ProjectRoute } from "../features/projects/ProjectRoute";
import { RuntimeSettingsRoute } from "../features/runtime/RuntimeSettingsRoute";
import { SessionRoute } from "../features/sessions/SessionRoute";
import { AuthGate } from "../features/auth/AuthGate";

export function App() {
  return (
    <AuthGate>
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<DashboardRoute />} />
          <Route path="/dashboard" element={<DashboardRoute />} />
          <Route path="/nodes" element={<NodesRoute />} />
          <Route path="/nodes/:nodeId" element={<NodeDetailRoute />} />
          <Route path="/projects/:projectId" element={<ProjectRoute />} />
          <Route
            path="/nodes/:nodeId/placements/new"
            element={<PlacementNewRoute />}
          />
          <Route path="/workspaces/:placementId" element={<PlacementRoute />} />
          <Route path="/sessions/:sessionThreadId" element={<SessionRoute />} />
          <Route path="/settings/runtime" element={<RuntimeSettingsRoute />} />
        </Route>
      </Routes>
    </AuthGate>
  );
}
