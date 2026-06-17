export const queryKeys = {
  health: ["health"] as const,
  version: ["version"] as const,
  inventory: ["inventory"] as const,
  nodeEnrollments: ["node-enrollments"] as const,
  node: (nodeId: string) => ["node", nodeId] as const,
  placement: (placementId: string) => ["placement", placementId] as const,
  session: (sessionThreadId: string) => ["session", sessionThreadId] as const,
  artifactTree: (sessionThreadId: string) =>
    ["session", sessionThreadId, "artifact-tree"] as const,
  agentProjection: (sessionThreadId: string) =>
    ["session", sessionThreadId, "agent-projection"] as const,
};
