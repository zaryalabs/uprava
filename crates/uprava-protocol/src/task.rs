use super::*;

/// One declarative validation command executed inside the task sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCheckSpec {
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_task_check_timeout_seconds")]
    pub timeout_seconds: u64,
}

const fn default_task_check_timeout_seconds() -> u64 {
    300
}

/// Bounded resources requested from the task runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResourceLimits {
    #[serde(default = "default_task_cpu")]
    pub cpu: String,
    #[serde(default = "default_task_memory")]
    pub memory: String,
}

impl Default for TaskResourceLimits {
    fn default() -> Self {
        Self {
            cpu: default_task_cpu(),
            memory: default_task_memory(),
        }
    }
}

fn default_task_cpu() -> String {
    "2".to_owned()
}

fn default_task_memory() -> String {
    "4Gi".to_owned()
}

/// User-controlled input accepted when creating an isolated task run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTaskRunRequest {
    pub project_placement_id: ProjectPlacementId,
    pub prompt: String,
    /// Immutable Git commit. When omitted, Core snapshots the placement HEAD.
    pub base_revision: Option<String>,
    #[serde(default)]
    pub checks: Vec<TaskCheckSpec>,
    /// Explicit relative paths to retain as hashed evidence (globs are not accepted).
    #[serde(default)]
    pub artifact_paths: Vec<String>,
    #[serde(default = "default_task_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_task_ttl_seconds")]
    pub ttl_seconds: u64,
    #[serde(default)]
    pub resource_limits: TaskResourceLimits,
    pub runtime_image: Option<String>,
}

const fn default_task_timeout_seconds() -> u64 {
    3_600
}

const fn default_task_ttl_seconds() -> u64 {
    7_200
}

/// Immutable execution contract stored by Core and delivered to Node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunSpec {
    pub task_run_id: TaskRunId,
    pub project_placement_id: ProjectPlacementId,
    pub provider: String,
    pub prompt: String,
    pub base_revision: String,
    pub branch: String,
    pub checks: Vec<TaskCheckSpec>,
    pub artifact_paths: Vec<String>,
    pub timeout_seconds: u64,
    pub ttl_seconds: u64,
    pub resource_limits: TaskResourceLimits,
    pub runtime_image: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCheckResult {
    pub label: String,
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskArtifactEvidence {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunResultPackage {
    pub task_run_id: TaskRunId,
    pub state: TaskRunState,
    pub cleanup_state: TaskCleanupState,
    pub summary: String,
    pub base_revision: String,
    pub final_revision: Option<String>,
    pub branch: String,
    pub worktree_path: String,
    pub runtime_image: String,
    pub diff: String,
    pub diff_truncated: bool,
    pub checks: Vec<TaskCheckResult>,
    pub artifacts: Vec<TaskArtifactEvidence>,
    #[serde(default)]
    pub unresolved_risks: Vec<String>,
    pub terminal_reason: Option<ScheduledMessageFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunSummary {
    pub task_run_id: TaskRunId,
    pub project_placement_id: ProjectPlacementId,
    pub placement_name: String,
    pub node_id: NodeId,
    pub provider: String,
    pub state: TaskRunState,
    pub cleanup_state: TaskCleanupState,
    pub base_revision: String,
    pub branch: String,
    pub runtime_image: String,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub terminal_reason: Option<ScheduledMessageFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunDetail {
    pub task: TaskRunSummary,
    pub prompt: String,
    pub checks: Vec<TaskCheckSpec>,
    pub artifact_paths: Vec<String>,
    pub timeout_seconds: u64,
    pub ttl_seconds: u64,
    pub resource_limits: TaskResourceLimits,
    pub worktree_path: Option<String>,
    pub result: Option<TaskRunResultPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunListResponse {
    pub items: Vec<TaskRunSummary>,
}
