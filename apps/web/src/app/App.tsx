import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "./shell/AppShell";
import { NodeDetailRoute } from "../features/nodes/NodeDetailRoute";
import { NodesRoute } from "../features/nodes/NodesRoute";
import { PlacementNewRoute } from "../features/placements/PlacementNewRoute";
import { PlacementRoute } from "../features/placements/PlacementRoute";
import { RuntimeSettingsRoute } from "../features/runtime/RuntimeSettingsRoute";
import { SessionRoute } from "../features/sessions/SessionRoute";

export function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/nodes" replace />} />
        <Route path="/nodes" element={<NodesRoute />} />
        <Route path="/nodes/:nodeId" element={<NodeDetailRoute />} />
        <Route
          path="/nodes/:nodeId/placements/new"
          element={<PlacementNewRoute />}
        />
        <Route path="/placements/:placementId" element={<PlacementRoute />} />
        <Route path="/sessions/:sessionThreadId" element={<SessionRoute />} />
        <Route path="/settings/runtime" element={<RuntimeSettingsRoute />} />
      </Route>
    </Routes>
  );
}
