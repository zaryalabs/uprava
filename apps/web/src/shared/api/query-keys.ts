export const queryKeys = {
  health: ["health"] as const,
  version: ["version"] as const,
  inventory: ["inventory"] as const,
  jobs: ["jobs"] as const,
  job: (jobId: string) => ["job", jobId] as const,
  jobRun: (jobRunId: string) => ["job-run", jobRunId] as const,
  providerQuota: (provider: string) => ["provider-quota", provider] as const,
  nodeEnrollments: ["node-enrollments"] as const,
  node: (nodeId: string) => ["node", nodeId] as const,
  placement: (placementId: string) => ["placement", placementId] as const,
  workspaceTree: (placementId: string, path: string) =>
    ["placement", placementId, "workspace-tree", path] as const,
  workspaceFile: (placementId: string, path: string) =>
    ["placement", placementId, "workspace-file", path] as const,
  workspaceCommandHistory: (placementId: string) =>
    ["placement", placementId, "workspace-command-history"] as const,
  workspaceDiff: (placementId: string) =>
    ["placement", placementId, "workspace-diff"] as const,
  workspaceTerminals: (placementId: string) =>
    ["placement", placementId, "workspace-terminals"] as const,
  session: (sessionThreadId: string) => ["session", sessionThreadId] as const,
  sessionEvidenceProjection: (sessionThreadId: string) =>
    ["session", sessionThreadId, "evidence-projection"] as const,
  agentProjection: (sessionThreadId: string) =>
    ["session", sessionThreadId, "agent-projection"] as const,
};
