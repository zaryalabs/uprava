import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Box, Play, Square } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  CreateTaskRunRequest,
  TaskRunState,
  TaskRunSummary,
} from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { EmptyState, LoadingState } from "../../shared/ui/system";
import { useWorkspaceContext } from "../workspaces/WorkspaceLayout";
import {
  routeWithSearch,
  workspaceTaskRoute,
  workspaceTasksRoute,
} from "../workspaces/routes";

const terminalTaskStates = new Set<TaskRunState>([
  "succeeded",
  "failed",
  "cancelled",
  "timed_out",
]);

export function TasksRoute() {
  const { placement, node } = useWorkspaceContext();
  const { taskRunId } = useParams();
  const location = useLocation();
  const placementId = placement.project_placement_id;
  const tasks = useQuery({
    queryKey: queryKeys.taskRuns(placementId),
    queryFn: () => coreApi.taskRuns(placementId),
    refetchInterval: (query) =>
      query.state.data?.items.some(
        (item) => !terminalTaskStates.has(item.state),
      )
        ? 1_500
        : false,
  });
  const runtimeAvailable = taskRuntimeAvailable(node.capabilities);

  return (
    <section className="space-y-4" aria-labelledby="workspace-tasks-title">
      <header>
        <div className="zarya-caption">ISOLATED RUN MODE</div>
        <h2 id="workspace-tasks-title" className="mt-1 text-xl font-bold">
          Task Runs
        </h2>
        <p className="mt-1 max-w-3xl text-sm text-[var(--color-muted)]">
          One prompt, one linked Git worktree and one bounded OpenSandbox. The
          worktree remains available for review after the sandbox is removed.
        </p>
      </header>

      {!runtimeAvailable ? (
        <div className="border border-amber-500/60 bg-amber-500/10 p-3 text-sm">
          This Node does not advertise an available OpenSandbox runtime. Start
          it and set <code>UPRAVA_OPENSANDBOX_URL</code> before creating a run.
        </div>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-[18rem_minmax(0,1fr)]">
        <aside className="border border-[var(--color-border)] p-3">
          <div className="flex items-center justify-between gap-3 border-b border-[var(--color-border)] pb-3">
            <div className="zarya-label">RUNS</div>
            <Link
              className="text-xs underline"
              to={routeWithSearch(
                workspaceTasksRoute(placementId),
                location.search,
              )}
            >
              New
            </Link>
          </div>
          {tasks.isError ? (
            <ErrorNotice error={tasks.error} title="Task Runs load failed" />
          ) : null}
          {tasks.isPending ? <LoadingState stage="Loading Task Runs" /> : null}
          <ul className="mt-2 space-y-1">
            {tasks.data?.items.map((task) => (
              <li key={task.task_run_id}>
                <TaskLink
                  task={task}
                  selected={task.task_run_id === taskRunId}
                  to={routeWithSearch(
                    workspaceTaskRoute(placementId, task.task_run_id),
                    location.search,
                  )}
                />
              </li>
            ))}
          </ul>
          {tasks.isSuccess && tasks.data.items.length === 0 ? (
            <p className="py-5 text-xs text-[var(--color-muted)]">
              No Task Runs yet.
            </p>
          ) : null}
        </aside>

        {taskRunId ? (
          <TaskDetail taskRunId={taskRunId} placementId={placementId} />
        ) : (
          <TaskCreateForm runtimeAvailable={runtimeAvailable} />
        )}
      </div>
    </section>
  );
}

function TaskCreateForm({ runtimeAvailable }: { runtimeAvailable: boolean }) {
  const { placement } = useWorkspaceContext();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const [prompt, setPrompt] = useState("");
  const [baseRevision, setBaseRevision] = useState(
    placement.git_snapshot?.commit ?? "",
  );
  const [checks, setChecks] = useState("make c");
  const [artifactPaths, setArtifactPaths] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState(3_600);
  const [ttlSeconds, setTtlSeconds] = useState(7_200);
  const [cpu, setCpu] = useState("2");
  const [memory, setMemory] = useState("4Gi");
  const placementId = placement.project_placement_id;
  const create = useMutation({
    mutationFn: () => {
      const request: CreateTaskRunRequest = {
        project_placement_id: placementId,
        prompt,
        base_revision: baseRevision.trim() || null,
        checks: parseChecks(checks, timeoutSeconds),
        artifact_paths: parseLines(artifactPaths),
        timeout_seconds: timeoutSeconds,
        ttl_seconds: ttlSeconds,
        resource_limits: { cpu, memory },
        runtime_image: null,
      };
      return coreApi.createTaskRun(request);
    },
    onSuccess: async (detail) => {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.taskRuns(placementId),
      });
      navigate(
        routeWithSearch(
          workspaceTaskRoute(placementId, detail.task.task_run_id),
          location.search,
        ),
      );
    },
  });

  return (
    <form
      className="space-y-4 border border-[var(--color-border-strong)] p-4"
      onSubmit={(event) => {
        event.preventDefault();
        create.mutate();
      }}
    >
      <div className="flex items-center gap-2 font-bold">
        <Box size={17} aria-hidden="true" /> New isolated Task Run
      </div>
      <TaskField label="Prompt / completion contract">
        <textarea
          className="min-h-40 w-full resize-y border border-[var(--color-muted)] bg-[var(--color-bg)] p-3 text-sm"
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          required
        />
      </TaskField>
      <TaskField label="Immutable base commit">
        <input
          className="w-full border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 font-mono text-xs"
          value={baseRevision}
          onChange={(event) => setBaseRevision(event.target.value)}
          placeholder="40-character Git commit"
        />
      </TaskField>
      <div className="grid gap-3 md:grid-cols-2">
        <TaskField label="Checks, one whitespace-separated argv per line">
          <textarea
            className="min-h-24 w-full resize-y border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 font-mono text-xs"
            value={checks}
            onChange={(event) => setChecks(event.target.value)}
            placeholder="make c"
          />
        </TaskField>
        <TaskField label="Evidence files, one relative path per line">
          <textarea
            className="min-h-24 w-full resize-y border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 font-mono text-xs"
            value={artifactPaths}
            onChange={(event) => setArtifactPaths(event.target.value)}
            placeholder="coverage/report.json"
          />
        </TaskField>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <NumberField
          label="Hard timeout, sec"
          min={60}
          value={timeoutSeconds}
          onChange={setTimeoutSeconds}
        />
        <NumberField
          label="Sandbox TTL, sec"
          min={60}
          value={ttlSeconds}
          onChange={setTtlSeconds}
        />
        <TaskField label="CPU limit">
          <select
            className="w-full border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 text-sm"
            value={cpu}
            onChange={(event) => setCpu(event.target.value)}
          >
            {["0.5", "1", "2", "4", "8"].map((value) => (
              <option key={value} value={value}>
                {value} CPU
              </option>
            ))}
          </select>
        </TaskField>
        <TaskField label="Memory limit">
          <select
            className="w-full border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 text-sm"
            value={memory}
            onChange={(event) => setMemory(event.target.value)}
          >
            {["512Mi", "1Gi", "2Gi", "4Gi", "8Gi", "16Gi"].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </TaskField>
      </div>
      <p className="text-xs text-[var(--color-muted)]">
        The immutable runtime image advertised by this Node will be used. Codex
        credential-profile mounting is intentionally deferred in this iteration;
        authenticate the runtime image manually for the final smoke test.
      </p>
      {create.isError ? (
        <ErrorNotice error={create.error} title="Task Run creation failed" />
      ) : null}
      <Button
        type="submit"
        variant="primary"
        disabled={
          !runtimeAvailable ||
          !prompt.trim() ||
          !baseRevision.trim() ||
          ttlSeconds < timeoutSeconds ||
          create.isPending
        }
      >
        <Play size={15} aria-hidden="true" /> Start Task Run
      </Button>
    </form>
  );
}

function TaskDetail({
  taskRunId,
  placementId,
}: {
  taskRunId: string;
  placementId: string;
}) {
  const queryClient = useQueryClient();
  const detail = useQuery({
    queryKey: queryKeys.taskRun(taskRunId),
    queryFn: () => coreApi.taskRun(taskRunId),
    refetchInterval: (query) =>
      query.state.data && !terminalTaskStates.has(query.state.data.task.state)
        ? 1_000
        : false,
  });
  const cancel = useMutation({
    mutationFn: () => coreApi.cancelTaskRun(taskRunId),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: queryKeys.taskRun(taskRunId),
        }),
        queryClient.invalidateQueries({
          queryKey: queryKeys.taskRuns(placementId),
        }),
      ]);
    },
  });
  if (detail.isPending) return <LoadingState stage="Loading Task Run" />;
  if (detail.isError) {
    return <ErrorNotice error={detail.error} title="Task Run load failed" />;
  }
  const task = detail.data.task;
  const cancellable =
    !terminalTaskStates.has(task.state) && task.state !== "cancelling";

  return (
    <article className="min-w-0 space-y-4 border border-[var(--color-border-strong)] p-4">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="zarya-caption">TASK RUN</div>
          <h3 className="mt-1 font-mono text-sm">{task.task_run_id}</h3>
          <div className="mt-2 flex flex-wrap gap-2 text-xs">
            <TaskState state={task.state} />
            <span className="border border-[var(--color-border)] px-2 py-1">
              cleanup: {task.cleanup_state}
            </span>
          </div>
        </div>
        {cancellable ? (
          <Button
            variant="secondary"
            disabled={cancel.isPending}
            onClick={() => cancel.mutate()}
          >
            <Square size={14} aria-hidden="true" /> Cancel
          </Button>
        ) : null}
      </header>
      {cancel.isError ? (
        <ErrorNotice error={cancel.error} title="Cancellation failed" />
      ) : null}
      <dl className="grid gap-2 text-xs sm:grid-cols-2">
        <Meta label="Base" value={task.base_revision} mono />
        <Meta label="Branch" value={task.branch} mono />
        <Meta label="Image" value={task.runtime_image} mono />
        <Meta
          label="Worktree"
          value={detail.data.worktree_path ?? "pending"}
          mono
        />
      </dl>
      <section>
        <div className="zarya-label">PROMPT</div>
        <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap border border-[var(--color-border)] p-3 text-xs">
          {detail.data.prompt}
        </pre>
      </section>
      {detail.data.result ? (
        <>
          <section>
            <div className="zarya-label">SUMMARY</div>
            <p className="mt-2 whitespace-pre-wrap text-sm">
              {detail.data.result.summary}
            </p>
          </section>
          {detail.data.result.terminal_reason ? (
            <section className="border border-red-500/60 bg-red-500/10 p-3 text-sm">
              <div className="zarya-label">TERMINAL REASON</div>
              <p className="mt-2">
                {detail.data.result.terminal_reason.message}
              </p>
              <code className="mt-2 block text-xs">
                {detail.data.result.terminal_reason.code}
              </code>
            </section>
          ) : null}
          <section>
            <div className="zarya-label">CHECKS</div>
            {detail.data.result.checks.length === 0 ? (
              <p className="mt-2 text-xs text-[var(--color-muted)]">
                No checks recorded.
              </p>
            ) : (
              <ul className="mt-2 space-y-2">
                {detail.data.result.checks.map((check, index) => (
                  <li
                    key={`${check.label}-${index}`}
                    className="border border-[var(--color-border)] p-3 text-xs"
                  >
                    <div className="flex justify-between gap-3 font-bold">
                      <span>{check.label}</span>
                      <span>{check.success ? "passed" : "failed"}</span>
                    </div>
                    <code className="mt-1 block break-all">
                      {check.command}
                    </code>
                    {check.stderr ? (
                      <pre className="mt-2 max-h-32 overflow-auto whitespace-pre-wrap text-red-500">
                        {check.stderr}
                      </pre>
                    ) : null}
                    {check.stdout ? (
                      <pre className="mt-2 max-h-32 overflow-auto whitespace-pre-wrap">
                        {check.stdout}
                      </pre>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section>
            <div className="zarya-label">DIFF EVIDENCE</div>
            <pre className="mt-2 max-h-[32rem] overflow-auto whitespace-pre-wrap border border-[var(--color-border)] p-3 font-mono text-xs">
              {detail.data.result.diff || "No tracked diff."}
            </pre>
            {detail.data.result.diff_truncated ? (
              <p className="mt-1 text-xs text-amber-500">
                Diff was truncated at the evidence limit.
              </p>
            ) : null}
          </section>
          <section>
            <div className="zarya-label">HASHED ARTIFACTS</div>
            <ul className="mt-2 space-y-1 font-mono text-xs">
              {detail.data.result.artifacts.map((artifact) => (
                <li
                  key={artifact.path}
                  className="break-all border border-[var(--color-border)] p-2"
                >
                  {artifact.path} · {artifact.size_bytes} B · sha256:
                  {artifact.sha256}
                </li>
              ))}
            </ul>
          </section>
          {detail.data.result.unresolved_risks.length > 0 ? (
            <section className="border border-amber-500/60 bg-amber-500/10 p-3">
              <div className="zarya-label">UNRESOLVED RISKS</div>
              <ul className="mt-2 list-disc space-y-1 pl-5 text-xs">
                {detail.data.result.unresolved_risks.map((risk) => (
                  <li key={risk}>{risk}</li>
                ))}
              </ul>
            </section>
          ) : null}
        </>
      ) : terminalTaskStates.has(task.state) ? (
        <section className="border border-red-500/60 bg-red-500/10 p-3 text-sm">
          <div className="zarya-label">TERMINAL RESULT</div>
          <p className="mt-2">
            {task.terminal_reason?.message ??
              "The Task Run ended before an evidence package was produced."}
          </p>
          {task.terminal_reason ? (
            <code className="mt-2 block text-xs">
              {task.terminal_reason.code}
            </code>
          ) : null}
        </section>
      ) : (
        <EmptyState
          title="Execution in progress"
          detail="State and evidence refresh automatically while the Node owns this run."
        />
      )}
    </article>
  );
}

function TaskLink({
  task,
  selected,
  to,
}: {
  task: TaskRunSummary;
  selected: boolean;
  to: string;
}) {
  return (
    <Link
      to={to}
      className={`block border p-2 text-xs ${
        selected
          ? "border-[var(--color-ink)] bg-[var(--color-subtle)]"
          : "border-transparent hover:border-[var(--color-border)]"
      }`}
    >
      <div className="flex items-center justify-between gap-2">
        <TaskState state={task.state} />
        <time className="text-[var(--color-muted)]">
          {new Date(task.queued_at).toLocaleString()}
        </time>
      </div>
      <div className="mt-2 line-clamp-2">{task.summary ?? task.branch}</div>
    </Link>
  );
}

function TaskState({ state }: { state: TaskRunState }) {
  const terminal = terminalTaskStates.has(state);
  return (
    <span
      className={`border px-2 py-1 font-bold ${
        state === "succeeded"
          ? "border-emerald-500 text-emerald-600"
          : terminal
            ? "border-red-500 text-red-500"
            : "border-blue-500 text-blue-500"
      }`}
    >
      {state.replaceAll("_", " ")}
    </span>
  );
}

function TaskField({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <label className="grid gap-1 text-xs font-bold text-[var(--color-muted)]">
      {label}
      {children}
    </label>
  );
}

function NumberField({
  label,
  min,
  value,
  onChange,
}: {
  label: string;
  min: number;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <TaskField label={label}>
      <input
        className="w-full border border-[var(--color-muted)] bg-[var(--color-bg)] p-2 text-sm"
        type="number"
        min={min}
        max={86_400}
        value={value}
        onChange={(event) => onChange(event.target.valueAsNumber)}
      />
    </TaskField>
  );
}

function Meta({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="min-w-0">
      <dt className="font-bold text-[var(--color-muted)]">{label}</dt>
      <dd className={`break-all ${mono ? "font-mono" : ""}`}>{value}</dd>
    </div>
  );
}

export function parseChecks(value: string, timeoutSeconds: number) {
  return parseLines(value).map((line) => {
    const [command = "", ...args] = line.split(/\s+/u);
    return {
      label: line,
      command,
      args,
      timeout_seconds: Math.min(timeoutSeconds, 600),
    };
  });
}

function parseLines(value: string) {
  return [
    ...new Set(
      value
        .split(/\r?\n/u)
        .map((line) => line.trim())
        .filter(Boolean),
    ),
  ];
}

export function taskRuntimeAvailable(
  capabilities: {
    key: string;
    value: unknown;
  }[],
) {
  const capability = capabilities.find(
    (candidate) => candidate.key === "task_runtime.opensandbox.docker",
  );
  if (!capability || !isRecord(capability.value)) return false;
  const extensionValue = capability.value.value;
  return (
    isRecord(extensionValue) &&
    extensionValue.available === true &&
    extensionValue.provider === "codex" &&
    typeof extensionValue.runtime_image === "string" &&
    extensionValue.runtime_image.trim().length > 0
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
