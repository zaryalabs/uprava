import type {
  AcknowledgeWarningRequest,
  ApiError,
  ApproveNodeEnrollmentResponse,
  ClientCreateNodeEnrollmentRequest,
  CommandAcceptedResponse,
  CommandState,
  CreatePlacementRequest,
  CreateJobRequest,
  CreateSessionRequest,
  CreateDeductionRequest,
  DeductionAcceptedResponse,
  DeductionRecord,
  EventLogPage,
  HealthResponse,
  NodeCredentialRotationResponse,
  NodeDeletionResponse,
  NodeEnrollmentRequestedResponse,
  NodeEnrollmentSummary,
  NodeRevocationResponse,
  PlacementDeletionResponse,
  ResolveApprovalRequest,
  SendTurnRequest,
  CreateScheduledMessageRequest,
  ScheduledSessionMessage,
  JobDetail,
  JobRunSummary,
  JobSummary,
  ProviderQuotaStatus,
  PersistDeductionResponse,
  ReferenceResolution,
  UpdateJobRequest,
  SessionDetail,
  SessionTraceProjection,
  UpravaRef,
  VersionResponse,
  WebAuthLoginRequest,
  WebAuthResponse,
  WebAuthSetupRequest,
  WebAuthStatusResponse,
  WarningAcknowledgementResponse,
  WorkspaceCommandHistoryResponse,
  WorkspaceCommandHistoryItem,
  WorkspaceCommandRunRequest,
  WorkspaceCommandRunResponse,
  WorkspaceDiffResponse,
  WorkspaceFileContentResponse,
  WorkspaceFileWriteRequest,
  WorkspaceFileWriteResponse,
  WorkspaceTerminalListResponse,
  WorkspaceTerminalOpenRequest,
  WorkspaceTerminalOpenResponse,
  WorkspaceTreeResponse,
} from "../protocol/types";
import {
  commandAcceptedResponseSchema,
  formatProtocolIssues,
  parseProtocolPayload,
  type ProtocolSchema,
  workspaceCommandHistoryItemSchema,
  workspaceCommandHistoryResponseSchema,
  workspaceCommandRunResponseSchema,
  workspaceTerminalListResponseSchema,
  workspaceTerminalOpenResponseSchema,
} from "../protocol/validators";
import { apiBase, apiWsBase } from "./config";
import { logClientEvent } from "../logging/client-logger";
import { readCookie } from "../auth/cookies";

export class UpravaApiError extends Error {
  constructor(
    readonly envelope: ApiError,
    readonly status?: number,
  ) {
    super(envelope.message);
  }
}

export function shouldRetryQuery(failureCount: number, error: unknown) {
  if (failureCount >= 2) return false;
  if (error instanceof UpravaApiError) {
    return (
      error.status !== undefined &&
      error.status >= 500 &&
      error.envelope.retryable
    );
  }
  return true;
}

export async function apiGet<T>(
  path: string,
  schema?: ProtocolSchema<T>,
): Promise<T> {
  return apiRequest<T>(path, { method: "GET" }, schema);
}

export async function apiPost<T>(
  path: string,
  body?: unknown,
  schema?: ProtocolSchema<T>,
): Promise<T> {
  return apiRequest<T>(
    path,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
    schema,
  );
}

export async function apiPatch<T>(
  path: string,
  body?: unknown,
  schema?: ProtocolSchema<T>,
): Promise<T> {
  return apiRequest<T>(
    path,
    {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
    schema,
  );
}

export async function apiDelete<T>(
  path: string,
  schema?: ProtocolSchema<T>,
): Promise<T> {
  return apiRequest<T>(path, { method: "DELETE" }, schema);
}

const workspaceCommandTerminalStates = new Set<CommandState>([
  "completed",
  "failed",
  "blocked",
  "expired",
]);

async function apiRequest<T>(
  path: string,
  init: RequestInit,
  schema?: ProtocolSchema<T>,
): Promise<T> {
  const headers = new Headers(init.headers);
  const method = init.method?.toUpperCase() ?? "GET";
  const csrf = readCookie("uprava_csrf");
  if (csrf && !["GET", "HEAD", "OPTIONS"].includes(method)) {
    headers.set("x-uprava-csrf", csrf);
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
    throw new UpravaApiError(envelope, response.status);
  }
  const payload = (await response.json()) as unknown;
  if (!schema) {
    return payload as T;
  }
  const parsed = schema.safeParse(payload);
  if (parsed.success) {
    return parsed.data;
  }
  const detail = formatProtocolIssues(parsed.error.issues);
  logClientEvent("error", "web.protocol", "Core response validation failed", {
    path,
    status: response.status,
    detail,
  });
  throw new UpravaApiError({
    error_code: "web.protocol_validation_failed",
    message: `Core response did not match protocol v2 contract: ${detail}`,
    retryable: false,
    correlation_id: "client",
  });
}

async function sleep(delayMs: number) {
  await new Promise((resolve) => window.setTimeout(resolve, delayMs));
}

function commandResourceError(
  code: string,
  message: string,
  retryable = false,
) {
  return new UpravaApiError({
    error_code: code,
    message,
    retryable,
    correlation_id: "client",
  });
}

async function pollWorkspaceCommandRunResponse(
  placementId: string,
  commandId: string,
  timeoutSeconds: number | null,
) {
  const timeoutMs = Math.max((timeoutSeconds ?? 120) * 1000 + 10_000, 15_000);
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const resource = await apiGet<WorkspaceCommandHistoryItem>(
      `/placements/${encodeURIComponent(placementId)}/workspace/commands/async/${encodeURIComponent(commandId)}`,
      workspaceCommandHistoryItemSchema,
    );
    if (!workspaceCommandTerminalStates.has(resource.state)) {
      await sleep(250);
      continue;
    }
    if (resource.state !== "completed") {
      throw commandResourceError(
        "workspace.command_failed",
        `Workspace command finished with state ${resource.state}`,
      );
    }
    const result = parseProtocolPayload(
      workspaceCommandRunResponseSchema,
      resource.result_payload,
    );
    if (result) {
      return result;
    }
    throw commandResourceError(
      "workspace.command_result_invalid",
      "Workspace command completed without a valid result payload",
    );
  }

  throw commandResourceError(
    "workspace.command_poll_timeout",
    "Timed out waiting for the workspace command result",
    true,
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
  jobs: () => apiGet<JobSummary[]>("/jobs"),
  createJob: (request: CreateJobRequest) =>
    apiPost<JobDetail>("/jobs", request),
  job: (jobId: string) =>
    apiGet<JobDetail>(`/jobs/${encodeURIComponent(jobId)}`),
  updateJob: (jobId: string, request: UpdateJobRequest) =>
    apiPatch<JobDetail>(`/jobs/${encodeURIComponent(jobId)}`, request),
  enableJob: (jobId: string) =>
    apiPost<JobDetail>(`/jobs/${encodeURIComponent(jobId)}/enable`),
  disableJob: (jobId: string) =>
    apiPost<JobDetail>(`/jobs/${encodeURIComponent(jobId)}/disable`),
  runJob: (jobId: string, force = false) =>
    apiPost<JobRunSummary>(`/jobs/${encodeURIComponent(jobId)}/runs`, {
      force,
    }),
  jobRun: (jobRunId: string) =>
    apiGet<JobRunSummary>(`/job-runs/${encodeURIComponent(jobRunId)}`),
  cancelJobRun: (jobRunId: string) =>
    apiPost<JobRunSummary>(`/job-runs/${encodeURIComponent(jobRunId)}/cancel`),
  providerQuota: (provider: string) =>
    apiGet<ProviderQuotaStatus>(
      `/provider-quota/${encodeURIComponent(provider)}`,
    ),
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
      undefined,
      commandAcceptedResponseSchema,
    ),
  workspaceTree: (placementId: string, path = ".") =>
    apiGet<WorkspaceTreeResponse>(
      `/placements/${encodeURIComponent(
        placementId,
      )}/workspace/tree?path=${encodeURIComponent(path)}`,
    ),
  workspaceFile: (placementId: string, path: string) =>
    apiGet<WorkspaceFileContentResponse>(
      `/placements/${encodeURIComponent(
        placementId,
      )}/workspace/file?path=${encodeURIComponent(path)}`,
    ),
  writeWorkspaceFile: (
    placementId: string,
    request: WorkspaceFileWriteRequest,
  ) =>
    apiPost<WorkspaceFileWriteResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/file`,
      request,
    ),
  runWorkspaceCommand: (
    placementId: string,
    request: WorkspaceCommandRunRequest,
  ) =>
    apiPost<CommandAcceptedResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/commands/async`,
      request,
      commandAcceptedResponseSchema,
    ).then((accepted) =>
      pollWorkspaceCommandRunResponse(
        placementId,
        accepted.command_id,
        request.timeout_seconds,
      ),
    ),
  runWorkspaceCommandAsync: (
    placementId: string,
    request: WorkspaceCommandRunRequest,
  ) =>
    apiPost<CommandAcceptedResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/commands/async`,
      request,
      commandAcceptedResponseSchema,
    ),
  workspaceCommandResource: (placementId: string, commandId: string) =>
    apiGet<WorkspaceCommandHistoryItem>(
      `/placements/${encodeURIComponent(placementId)}/workspace/commands/async/${encodeURIComponent(commandId)}`,
      workspaceCommandHistoryItemSchema,
    ),
  cancelWorkspaceCommand: (placementId: string, commandId: string) =>
    apiDelete<WorkspaceCommandHistoryItem>(
      `/placements/${encodeURIComponent(placementId)}/workspace/commands/async/${encodeURIComponent(commandId)}`,
      workspaceCommandHistoryItemSchema,
    ),
  workspaceCommandHistory: (placementId: string, limit = 20) =>
    apiGet<WorkspaceCommandHistoryResponse>(
      `/placements/${encodeURIComponent(
        placementId,
      )}/workspace/commands?limit=${encodeURIComponent(String(limit))}`,
      workspaceCommandHistoryResponseSchema,
    ),
  workspaceDiff: (placementId: string) =>
    apiGet<WorkspaceDiffResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/diff`,
    ),
  workspaceTerminals: (placementId: string) =>
    apiGet<WorkspaceTerminalListResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/terminals`,
      workspaceTerminalListResponseSchema,
    ),
  openWorkspaceTerminal: (
    placementId: string,
    request: WorkspaceTerminalOpenRequest,
  ) =>
    apiPost<WorkspaceTerminalOpenResponse>(
      `/placements/${encodeURIComponent(placementId)}/workspace/terminals`,
      request,
      workspaceTerminalOpenResponseSchema,
    ),
  workspaceTerminalStreamUrl: (placementId: string, terminalId: string) =>
    `${apiWsBase}/placements/${encodeURIComponent(
      placementId,
    )}/workspace/terminals/${encodeURIComponent(terminalId)}/stream`,
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
  sessionEvidenceProjection: (sessionThreadId: string) =>
    apiGet<import("../protocol/types").SessionEvidenceProjection>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/evidence-projection`,
    ),
  sessionTrace: (sessionThreadId: string) =>
    apiGet<SessionTraceProjection>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/trace`,
    ),
  events: (
    filters: {
      sessionThreadId?: string;
      placementId?: string;
      kind?: string;
      cursor?: string;
      limit?: number;
    } = {},
  ) => {
    const params = new URLSearchParams();
    if (filters.sessionThreadId) {
      params.set("session_thread_id", filters.sessionThreadId);
    }
    if (filters.placementId) params.set("placement_id", filters.placementId);
    if (filters.kind) params.set("kind", filters.kind);
    if (filters.cursor) params.set("cursor", filters.cursor);
    if (filters.limit !== undefined) params.set("limit", String(filters.limit));
    const query = params.size > 0 ? `?${params.toString()}` : "";
    return apiGet<EventLogPage>(`/events${query}`);
  },
  event: (eventId: string) =>
    apiGet<import("../protocol/types").EventEnvelope>(
      `/events/${encodeURIComponent(eventId)}`,
    ),
  resolveReference: (reference: UpravaRef) =>
    apiPost<ReferenceResolution>("/references/resolve", { reference }),
  createDeduction: (sessionThreadId: string, request: CreateDeductionRequest) =>
    apiPost<DeductionAcceptedResponse>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/deductions`,
      request,
    ),
  deduction: (deductionId: string) =>
    apiGet<DeductionRecord>(`/deductions/${encodeURIComponent(deductionId)}`),
  cancelDeduction: (deductionId: string) =>
    apiPost<DeductionAcceptedResponse>(
      `/deductions/${encodeURIComponent(deductionId)}/cancel`,
    ),
  persistDeduction: (deductionId: string) =>
    apiPost<PersistDeductionResponse>(
      `/deductions/${encodeURIComponent(deductionId)}/persist`,
    ),
  agentProjection: (sessionThreadId: string) =>
    apiGet<import("../protocol/types").AgentProjection>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/agent-projection`,
    ),
  sendTurn: (sessionThreadId: string, request: SendTurnRequest) =>
    apiPost<CommandAcceptedResponse>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/turns`,
      request,
      commandAcceptedResponseSchema,
    ),
  createScheduledMessage: (
    sessionThreadId: string,
    request: CreateScheduledMessageRequest,
  ) =>
    apiPost<ScheduledSessionMessage>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/scheduled-messages`,
      request,
    ),
  updateScheduledMessage: (
    sessionThreadId: string,
    scheduledMessageId: string,
    request: import("../protocol/types").UpdateScheduledMessageRequest,
  ) =>
    apiPatch<ScheduledSessionMessage>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/scheduled-messages/${encodeURIComponent(scheduledMessageId)}`,
      request,
    ),
  cancelScheduledMessage: (
    sessionThreadId: string,
    scheduledMessageId: string,
  ) =>
    apiDelete<ScheduledSessionMessage>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/scheduled-messages/${encodeURIComponent(scheduledMessageId)}`,
    ),
  sendScheduledMessageNow: (
    sessionThreadId: string,
    scheduledMessageId: string,
  ) =>
    apiPost<ScheduledSessionMessage>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/scheduled-messages/${encodeURIComponent(scheduledMessageId)}/send-now`,
    ),
  retryScheduledMessage: (
    sessionThreadId: string,
    scheduledMessageId: string,
  ) =>
    apiPost<ScheduledSessionMessage>(
      `/sessions/${encodeURIComponent(sessionThreadId)}/scheduled-messages/${encodeURIComponent(scheduledMessageId)}/retry`,
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
      commandAcceptedResponseSchema,
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
      undefined,
      commandAcceptedResponseSchema,
    ),
  stopRuntime: (runtimeSessionId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/runtime-sessions/${encodeURIComponent(runtimeSessionId)}/stop`,
      undefined,
      commandAcceptedResponseSchema,
    ),
  resumeRuntime: (runtimeSessionId: string) =>
    apiPost<CommandAcceptedResponse>(
      `/runtime-sessions/${encodeURIComponent(runtimeSessionId)}/resume`,
      undefined,
      commandAcceptedResponseSchema,
    ),
};
