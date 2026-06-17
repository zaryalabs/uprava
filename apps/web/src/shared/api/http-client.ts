import type {
  AcknowledgeWarningRequest,
  ApiError,
  ApproveNodeEnrollmentResponse,
  ClientCreateNodeEnrollmentRequest,
  CommandAcceptedResponse,
  CreatePlacementRequest,
  CreateSessionRequest,
  HealthResponse,
  NodeEnrollmentRequestedResponse,
  NodeEnrollmentSummary,
  NodeRevocationResponse,
  ResolveApprovalRequest,
  SendTurnRequest,
  SessionDetail,
  VersionResponse,
  WarningAcknowledgementResponse,
} from "../protocol/types";

export const apiBase =
  import.meta.env.VITE_CORTEX_API_BASE?.toString() ??
  "http://127.0.0.1:8080/api/v1";

export class CortexApiError extends Error {
  constructor(readonly envelope: ApiError) {
    super(envelope.message);
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  return apiRequest<T>(path, { method: "GET" });
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return apiRequest<T>(path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

async function apiRequest<T>(path: string, init: RequestInit): Promise<T> {
  const response = await fetch(`${apiBase}${path}`, init);
  if (!response.ok) {
    const fallback: ApiError = {
      error_code: "network.http",
      message: `HTTP ${response.status}`,
      retryable: response.status >= 500,
      correlation_id: "unavailable",
    };
    throw new CortexApiError(await response.json().catch(() => fallback));
  }
  return response.json() as Promise<T>;
}

export const coreApi = {
  health: () => apiGet<HealthResponse>("/health"),
  version: () => apiGet<VersionResponse>("/version"),
  inventory: () =>
    apiGet<import("../protocol/types").InventorySnapshot>("/inventory"),
  node: (nodeId: string) =>
    apiGet<import("../protocol/types").NodeSummary>(
      `/nodes/${encodeURIComponent(nodeId)}`,
    ),
  nodeEnrollments: () => apiGet<NodeEnrollmentSummary[]>("/node-enrollments"),
  createNodeEnrollment: (request: ClientCreateNodeEnrollmentRequest) =>
    apiPost<NodeEnrollmentRequestedResponse>("/node-enrollments", request),
  approveNodeEnrollment: (enrollmentId: string) =>
    apiPost<ApproveNodeEnrollmentResponse>(
      `/node-enrollments/${encodeURIComponent(enrollmentId)}/approve`,
    ),
  revokeNode: (nodeId: string) =>
    apiPost<NodeRevocationResponse>(
      `/nodes/${encodeURIComponent(nodeId)}/revoke`,
    ),
  placement: (placementId: string) =>
    apiGet<import("../protocol/types").ProjectPlacementSummary>(
      `/placements/${encodeURIComponent(placementId)}`,
    ),
  refreshResourceSnapshot: (placementId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/placements/${encodeURIComponent(
        placementId,
      )}/resource-snapshot/refresh`,
    ),
  validatePlacement: (request: CreatePlacementRequest) =>
    apiPost<import("../protocol/types").ProjectPlacementSummary>(
      "/project-placements/validate",
      request,
    ),
  createSession: (request: CreateSessionRequest) =>
    apiPost<import("../protocol/types").SessionDetail>("/sessions", request),
  session: (sessionThreadId: string) =>
    apiGet<import("../protocol/types").SessionDetail>(
      `/sessions/${encodeURIComponent(sessionThreadId)}`,
    ),
  attachSession: (sessionThreadId: string) =>
    apiPost<SessionDetail>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/attach`,
    ),
  detachSession: (sessionThreadId: string) =>
    apiPost<SessionDetail>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/detach`,
    ),
  artifactTree: (sessionThreadId: string) =>
    apiGet<import("../protocol/types").ArtifactTree>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/artifact-tree`,
    ),
  agentProjection: (sessionThreadId: string) =>
    apiGet<import("../protocol/types").AgentProjection>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/agent-projection`,
    ),
  sendTurn: (sessionThreadId: string, request: SendTurnRequest) =>
    apiPost<CommandAcceptedResponse>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/turns`,
      request,
    ),
  resolveApproval: (
    sessionThreadId: string,
    approvalId: string,
    request: ResolveApprovalRequest,
  ) =>
    apiPost<CommandAcceptedResponse>(
      `/sessions/${encodeURIComponent(
        sessionThreadId,
      )}/approvals/${encodeURIComponent(approvalId)}/resolve`,
      request,
    ),
  acknowledgeWarning: (
    sessionThreadId: string,
    warningKind: string,
    request: AcknowledgeWarningRequest,
  ) =>
    apiPost<WarningAcknowledgementResponse>(
      `/sessions/${encodeURIComponent(
        sessionThreadId,
      )}/warnings/${encodeURIComponent(warningKind)}/acknowledge`,
      request,
    ),
  interruptRuntime: (runtimeSessionId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/runtime-sessions/${encodeURIComponent(runtimeSessionId)}/interrupt`,
    ),
  stopRuntime: (runtimeSessionId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/runtime-sessions/${encodeURIComponent(runtimeSessionId)}/stop`,
    ),
  resumeRuntime: (runtimeSessionId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/runtime-sessions/${encodeURIComponent(runtimeSessionId)}/resume`,
    ),
};
