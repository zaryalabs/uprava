import { useParams } from "react-router-dom";

import { SessionSurface } from "./SessionSurface";

export function SessionRoute() {
  const { sessionThreadId = "" } = useParams();
  return <SessionSurface sessionThreadId={sessionThreadId} />;
}
