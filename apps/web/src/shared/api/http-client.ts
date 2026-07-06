import type {
  AcknowledgeWarningRequest,
  ApiError,
  ApproveNodeEnrollmentResponse,
  ClientCreateNodeEnrollmentRequest,
  CommandAcceptedResponse,
  CreatePlacementRequest,
  CreateSessionRequest,
  HealthResponse,
  NodeCredentialRotationResponse,
  NodeDeletionResponse,
  NodeEnrollmentRequestedResponse,
  NodeEnrollmentSummary,
  NodeRevocationResponse,
  PlacementDeletionResponse,
  ResolveApprovalRequest,
  SendTurnRequest,
  SessionDetail,
  VersionResponse,
  WebAuthLoginRequest,
  WebAuthResponse,
  WebAuthSetupRequest,
  WebAuthStatusResponse,
  WarningAcknowledgementResponse,
} from "../protocol/types";
import { apiBase } from "./config";
import { logClientEvent } from "../logging/client-logger";

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

export async function apiDelete<T>(path: string): Promise<T> {
  return apiRequest<T>(path, { method: "DELETE" });
}

async function apiRequest<T>(path: string, init: RequestInit): Promise<T> {
  const headers = new Headers(init.headers);
  const method = init.method?.toUpperCase() ?? "GET";
  const csrf = readCookie("cortex_csrf");
  if (csrf && !["GET", "HEAD", "OPTIONS"].includes(method)) {
    headers.set("x-cortex-csrf", csrf);
  }
  const response = await fetch(`${apiBase}${path}`, {
    ...init,
    headers,
    credentials: "include",
  });
  if (!response.ok) {
    const fallback: ApiError = {
      error_code: "network.http",
      message: `HTTP ${response.status}`,
      retryable: response.status >= 500,
      correlation_id: "unavailable",
    };
    const envelope = await response.json().catch(() => fallback);
    logClientEvent("warn", "web.api", envelope.message, {
      path,
      status: response.status,
      error_code: envelope.error_code,
      correlation_id: envelope.correlation_id,
    });
    throw new CortexApiError(envelope);
  }
  return response.json() as Promise<T>;
}

function readCookie(name: string): string | null {
  return (
    document.cookie
      .split(";")
      .map((part) => part.trim())
      .find((part) => part.startsWith(`${name}=`))
      ?.slice(name.length + 1) ?? null
  );
}

export const coreApi = {
  health: () => apiGet<HealthResponse>("/health"),
  version: () => apiGet<VersionResponse>("/version"),
  authStatus: () => apiGet<WebAuthStatusResponse>("/auth/status"),
  authSetup: (request: WebAuthSetupRequest) =>
    apiPost<WebAuthResponse>("/auth/setup", request),
  authLogin: (request: WebAuthLoginRequest) =>
    apiPost<WebAuthResponse>("/auth/login", request),
  authLogout: () => apiPost<WebAuthResponse>("/auth/logout"),
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
  rotateNodeCredential: (nodeId: string) =>
    apiPost<NodeCredentialRotationResponse>(
      `/nodes/${encodeURIComponent(nodeId)}/rotate-credential`,
    ),
  deleteNode: (nodeId: string) =>
    apiDelete<NodeDeletionResponse>(`/nodes/${encodeURIComponent(nodeId)}`),
  placement: (placementId: string) =>
    apiGet<import("../protocol/types").ProjectPlacementSummary>(
      `/placements/${encodeURIComponent(placementId)}`,
    ),
  deletePlacement: (placementId: string) =>
    apiDelete<PlacementDeletionResponse>(
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
