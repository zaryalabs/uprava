import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  File,
  FileText,
  Folder,
  GitCompare,
  History,
  Play,
  RefreshCw,
  Save,
  ShieldAlert,
  SquareTerminal,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  CommandState,
  WorkspaceCommandHistoryItem,
  WorkspaceCommandRunResponse,
  WorkspaceEntry,
  WorkspaceEntryStatus,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { Textarea } from "../../shared/ui/textarea";

type WorkspaceInspectorProps = {
  placementId: string;
  workspacePath: string;
};

type RunCommandInput = {
  commandLine: string;
  intent: "command" | "check";
  label: string | null;
};

export function WorkspaceInspector({
  placementId,
  workspacePath,
}: WorkspaceInspectorProps) {
  const queryClient = useQueryClient();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [editorContent, setEditorContent] = useState("");
  const [savedContent, setSavedContent] = useState<string | null>(null);
  const [commandText, setCommandText] = useState("make l");
  const [lastCommandResult, setLastCommandResult] =
    useState<WorkspaceCommandRunResponse | null>(null);

  const tree = useQuery({
    queryKey: queryKeys.workspaceTree(placementId, "."),
    queryFn: () => coreApi.workspaceTree(placementId),
    enabled: Boolean(placementId),
  });
  const selectedFile = useQuery({
    queryKey: queryKeys.workspaceFile(placementId, selectedPath ?? ""),
    queryFn: () => coreApi.workspaceFile(placementId, selectedPath ?? "."),
    enabled: Boolean(placementId && selectedPath),
  });
  const history = useQuery({
    queryKey: queryKeys.workspaceCommandHistory(placementId),
    queryFn: () => coreApi.workspaceCommandHistory(placementId),
    enabled: Boolean(placementId),
  });
  const diff = useQuery({
    queryKey: queryKeys.workspaceDiff(placementId),
    queryFn: () => coreApi.workspaceDiff(placementId),
    enabled: false,
  });

  const firstFilePath = useMemo(
    () => (tree.data ? firstInspectablePath(tree.data.root) : null),
    [tree.data],
  );

  const saveMutation = useMutation({
    mutationFn: (request: {
      path: string;
      content: string;
      expected: string;
    }) =>
      coreApi.writeWorkspaceFile(placementId, {
        path: request.path,
        content: request.content,
        expected_content: request.expected,
      }),
    onSuccess: (_response, request) => {
      setEditorContent(request.content);
      setSavedContent(request.content);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceFile(placementId, request.path),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceTree(placementId, "."),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceCommandHistory(placementId),
      });
    },
  });

  const commandMutation = useMutation({
    mutationFn: (input: RunCommandInput) => {
      const parsed = parseCommandLine(input.commandLine);
      if (!parsed) {
        throw new Error("Command is required");
      }
      return coreApi.runWorkspaceCommand(placementId, {
        command: parsed.command,
        args: parsed.args,
        intent: input.intent,
        label: input.label,
        timeout_seconds: 120,
      });
    },
    onSuccess: (result) => {
      setLastCommandResult(result);
      setCommandText(formatCommandLine(result.command, result.args));
    },
    onSettled: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceCommandHistory(placementId),
      });
    },
  });

  useEffect(() => {
    setSelectedPath(null);
    setLastCommandResult(null);
  }, [placementId]);

  useEffect(() => {
    if (!selectedPath && firstFilePath) {
      setSelectedPath(firstFilePath);
    }
  }, [firstFilePath, selectedPath]);

  useEffect(() => {
    if (!selectedFile.data || selectedFile.data.path !== selectedPath) {
      return;
    }
    const content = selectedFile.data.content;
    setEditorContent(content ?? "");
    setSavedContent(content);
  }, [selectedFile.data, selectedPath]);

  const isDirty = savedContent !== null && editorContent !== savedContent;

  const refetchWorkspace = () => {
    void tree.refetch();
    if (selectedPath) {
      void selectedFile.refetch();
    }
    void history.refetch();
  };

  const saveSelectedFile = () => {
    if (!selectedPath || savedContent === null) {
      return;
    }
    saveMutation.mutate({
      path: selectedPath,
      content: editorContent,
      expected: savedContent,
    });
  };

  const refreshDiff = async () => {
    await diff.refetch();
    void queryClient.invalidateQueries({
      queryKey: queryKeys.workspaceCommandHistory(placementId),
    });
  };

  return (
    <section className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
            Workspace Inspector
          </h2>
          <div className="truncate font-mono text-xs text-[#536257]">
            {workspacePath}
          </div>
        </div>
        <Button
          variant="secondary"
          disabled={tree.isFetching || history.isFetching}
          onClick={refetchWorkspace}
        >
          <RefreshCw size={15} />
          Refresh
        </Button>
      </div>

      <div className="grid min-h-[620px] grid-cols-[minmax(220px,320px)_minmax(0,1fr)] gap-3 max-xl:grid-cols-1">
        <section className="min-h-0 rounded-md border border-[#d9ded4] bg-white">
          <div className="border-b border-[#e0e5db] px-3 py-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
            Files
          </div>
          <div className="max-h-[720px] overflow-auto py-2">
            {tree.isError ? (
              <div className="px-3">
                <ErrorNotice error={tree.error} title="File tree unavailable" />
              </div>
            ) : null}
            {tree.isLoading ? (
              <div className="px-3 py-2 text-sm text-[#536257]">Loading</div>
            ) : null}
            {tree.data ? (
              <WorkspaceTreeNode
                entry={tree.data.root}
                depth={0}
                selectedPath={selectedPath}
                onSelect={setSelectedPath}
              />
            ) : null}
          </div>
        </section>

        <div className="min-w-0 space-y-3">
          <section className="min-h-0 rounded-md border border-[#d9ded4] bg-white">
            <WorkspaceFileViewer
              selectedPath={selectedPath}
              entry={selectedFile.data?.metadata ?? null}
              content={selectedFile.data?.content ?? null}
              editorContent={editorContent}
              isDirty={isDirty}
              isLoading={selectedFile.isLoading}
              error={selectedFile.error}
              saveError={saveMutation.error}
              isSaving={saveMutation.isPending}
              onEditorChange={setEditorContent}
              onSave={saveSelectedFile}
            />
          </section>

          <div className="grid grid-cols-2 gap-3 max-2xl:grid-cols-1">
            <WorkspaceCommandPanel
              commandText={commandText}
              isRunning={commandMutation.isPending}
              error={commandMutation.error}
              result={lastCommandResult}
              onCommandTextChange={setCommandText}
              onRun={(input) => commandMutation.mutate(input)}
            />
            <WorkspaceDiffPanel
              isLoading={diff.isFetching}
              error={diff.error}
              diff={diff.data ?? null}
              onRefresh={() => void refreshDiff()}
            />
          </div>

          <WorkspaceHistoryPanel
            isLoading={history.isLoading}
            error={history.error}
            commands={history.data?.commands ?? []}
          />
        </div>
      </div>
    </section>
  );
}

function WorkspaceTreeNode({
  entry,
  depth,
  selectedPath,
  onSelect,
}: {
  entry: WorkspaceEntry;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
}) {
  const isSelected = selectedPath === entry.path;
  const isInspectable = entry.kind !== "directory";
  const Icon = entry.kind === "directory" ? Folder : File;

  return (
    <div>
      <button
        type="button"
        className={`flex min-h-8 w-full items-center gap-2 px-3 py-1 text-left text-sm hover:bg-[#f2f5ef] ${
          isSelected ? "bg-[#e4ece1] text-[#1d4f3a]" : "text-[#253129]"
        }`}
        style={{ paddingLeft: 12 + depth * 14 }}
        aria-current={isSelected ? "true" : undefined}
        onClick={() => {
          if (isInspectable) {
            onSelect(entry.path);
          }
        }}
      >
        <Icon size={14} className="shrink-0" />
        <span className="min-w-0 flex-1 truncate">{entry.name}</span>
        {entry.status !== "directory" && entry.status !== "readable" ? (
          <Badge tone={statusTone(entry.status)}>
            {workspaceStatusLabel(entry.status)}
          </Badge>
        ) : null}
      </button>
      {entry.children.map((child) => (
        <WorkspaceTreeNode
          key={child.path}
          entry={child}
          depth={depth + 1}
          selectedPath={selectedPath}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function WorkspaceFileViewer({
  selectedPath,
  entry,
  content,
  editorContent,
  isDirty,
  isLoading,
  error,
  saveError,
  isSaving,
  onEditorChange,
  onSave,
}: {
  selectedPath: string | null;
  entry: WorkspaceEntry | null;
  content: string | null;
  editorContent: string;
  isDirty: boolean;
  isLoading: boolean;
  error: unknown;
  saveError: unknown;
  isSaving: boolean;
  onEditorChange: (content: string) => void;
  onSave: () => void;
}) {
  if (!selectedPath) {
    return (
      <div className="flex min-h-[520px] items-center justify-center text-sm text-[#536257]">
        No text file selected
      </div>
    );
  }
  if (error) {
    return (
      <div className="p-3">
        <ErrorNotice error={error} title="File unavailable" />
      </div>
    );
  }
  if (isLoading || !entry) {
    return <div className="p-3 text-sm text-[#536257]">Loading file</div>;
  }

  const canEdit = content !== null && entry.status === "readable";

  return (
    <div className="flex max-h-[720px] min-h-[520px] flex-col">
      <div className="flex flex-wrap items-start justify-between gap-2 border-b border-[#e0e5db] px-3 py-2">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <FileText size={15} className="shrink-0" />
            <span className="truncate font-mono text-sm">{entry.path}</span>
          </div>
          <div className="mt-1 flex flex-wrap gap-2 text-xs text-[#536257]">
            <span>{formatBytes(entry.byte_len)}</span>
            {entry.modified_at ? (
              <span>{new Date(entry.modified_at).toLocaleString()}</span>
            ) : null}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone={statusTone(entry.status)}>
            {workspaceStatusLabel(entry.status)}
          </Badge>
          {isDirty ? <Badge tone="warn">Modified</Badge> : null}
          <Button
            variant="primary"
            disabled={!canEdit || !isDirty || isSaving}
            onClick={onSave}
          >
            <Save size={15} />
            {isSaving ? "Saving" : "Save"}
          </Button>
        </div>
      </div>

      {saveError ? (
        <div className="border-b border-[#f0d1cd] p-3">
          <ErrorNotice error={saveError} title="Save failed" />
        </div>
      ) : null}

      {canEdit ? (
        <Textarea
          className="min-h-0 flex-1 resize-none rounded-none border-0 font-mono text-xs leading-5 shadow-none focus:outline-none"
          value={editorContent}
          spellCheck={false}
          onChange={(event) => onEditorChange(event.target.value)}
        />
      ) : (
        <div className="flex min-h-0 flex-1 items-center justify-center gap-2 p-6 text-sm text-[#536257]">
          <ShieldAlert size={17} />
          <span>{workspaceStatusLabel(entry.status)}</span>
        </div>
      )}
    </div>
  );
}

function WorkspaceCommandPanel({
  commandText,
  isRunning,
  error,
  result,
  onCommandTextChange,
  onRun,
}: {
  commandText: string;
  isRunning: boolean;
  error: unknown;
  result: WorkspaceCommandRunResponse | null;
  onCommandTextChange: (value: string) => void;
  onRun: (input: RunCommandInput) => void;
}) {
  return (
    <section className="rounded-md border border-[#d9ded4] bg-white">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e0e5db] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
          <SquareTerminal size={15} />
          Command
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            disabled={isRunning}
            onClick={() =>
              onRun({
                commandLine: "make l",
                intent: "check",
                label: "Local check",
              })
            }
          >
            <CheckCircle2 size={15} />
            make l
          </Button>
          <Button
            variant="secondary"
            disabled={isRunning}
            onClick={() =>
              onRun({
                commandLine: "make c",
                intent: "check",
                label: "Full check",
              })
            }
          >
            <CheckCircle2 size={15} />
            make c
          </Button>
        </div>
      </div>
      <div className="space-y-3 p-3">
        <div className="flex gap-2 max-sm:flex-col">
          <input
            className="h-9 min-w-0 flex-1 rounded-md border border-[#bfc8bc] bg-white px-3 font-mono text-sm text-[#17211c] shadow-sm"
            value={commandText}
            onChange={(event) => onCommandTextChange(event.target.value)}
          />
          <Button
            variant="primary"
            disabled={isRunning}
            onClick={() =>
              onRun({
                commandLine: commandText,
                intent: "command",
                label: null,
              })
            }
          >
            <Play size={15} />
            {isRunning ? "Running" : "Run"}
          </Button>
        </div>
        {error ? <ErrorNotice error={error} title="Command failed" /> : null}
        {result ? <CommandResult result={result} /> : null}
      </div>
    </section>
  );
}

function WorkspaceDiffPanel({
  isLoading,
  error,
  diff,
  onRefresh,
}: {
  isLoading: boolean;
  error: unknown;
  diff: {
    summary: string;
    diff: string;
    summary_truncated: boolean;
    diff_truncated: boolean;
    generated_at: string;
  } | null;
  onRefresh: () => void;
}) {
  return (
    <section className="rounded-md border border-[#d9ded4] bg-white">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e0e5db] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
          <GitCompare size={15} />
          Diff
        </div>
        <Button variant="secondary" disabled={isLoading} onClick={onRefresh}>
          <RefreshCw size={15} />
          {isLoading ? "Loading" : "Refresh"}
        </Button>
      </div>
      <div className="space-y-3 p-3">
        {error ? <ErrorNotice error={error} title="Diff unavailable" /> : null}
        {diff ? (
          <>
            <div className="flex flex-wrap items-center gap-2 text-xs text-[#536257]">
              <span>{new Date(diff.generated_at).toLocaleString()}</span>
              {diff.summary_truncated || diff.diff_truncated ? (
                <Badge tone="warn">Truncated</Badge>
              ) : null}
            </div>
            <pre className="max-h-28 overflow-auto whitespace-pre-wrap rounded-md bg-[#f6f8f3] p-3 font-mono text-xs leading-5 text-[#17211c]">
              {diff.summary}
            </pre>
            <pre className="max-h-80 overflow-auto whitespace-pre-wrap rounded-md bg-[#111812] p-3 font-mono text-xs leading-5 text-[#dce8dd]">
              {diff.diff || "No diff"}
            </pre>
          </>
        ) : (
          <div className="text-sm text-[#536257]">No diff loaded</div>
        )}
      </div>
    </section>
  );
}

function WorkspaceHistoryPanel({
  isLoading,
  error,
  commands,
}: {
  isLoading: boolean;
  error: unknown;
  commands: WorkspaceCommandHistoryItem[];
}) {
  return (
    <section className="rounded-md border border-[#d9ded4] bg-white">
      <div className="flex items-center gap-2 border-b border-[#e0e5db] px-3 py-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
        <History size={15} />
        History
      </div>
      <div className="space-y-2 p-3">
        {error ? (
          <ErrorNotice error={error} title="History unavailable" />
        ) : null}
        {isLoading ? (
          <div className="text-sm text-[#536257]">Loading history</div>
        ) : null}
        {!isLoading && commands.length === 0 ? (
          <div className="text-sm text-[#536257]">No commands recorded</div>
        ) : null}
        {commands.slice(0, 8).map((item) => (
          <HistoryItem key={item.command_id} item={item} />
        ))}
      </div>
    </section>
  );
}

function HistoryItem({ item }: { item: WorkspaceCommandHistoryItem }) {
  const result = isCommandResult(item.result_payload)
    ? item.result_payload
    : null;
  return (
    <div className="rounded-md border border-[#e0e5db] bg-[#fbfcf8] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate font-mono text-sm text-[#17211c]">
            {historyItemTitle(item)}
          </div>
          <div className="mt-1 text-xs text-[#536257]">
            {formatDateTime(item.created_at)}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone={commandStateTone(item.state)}>
            {commandStateLabel(item.state)}
          </Badge>
          {result ? (
            <Badge tone={result.success ? "good" : "bad"}>
              {result.success ? "Success" : "Failed"}
            </Badge>
          ) : null}
        </div>
      </div>
      {result ? (
        <div className="mt-2 truncate text-xs text-[#536257]">
          exit {result.exit_code ?? "n/a"} ·{" "}
          {formatDuration(result.duration_ms)}
        </div>
      ) : null}
    </div>
  );
}

function CommandResult({ result }: { result: WorkspaceCommandRunResponse }) {
  return (
    <div className="space-y-2 rounded-md border border-[#d9ded4] bg-[#fbfcf8] p-3">
      <div className="flex flex-wrap items-center gap-2">
        {result.success ? (
          <CheckCircle2 size={16} className="text-[#1f6559]" />
        ) : (
          <XCircle size={16} className="text-[#88332f]" />
        )}
        <span className="font-mono text-sm">
          {formatCommandLine(result.command, result.args)}
        </span>
        <Badge tone={result.success ? "good" : "bad"}>
          exit {result.exit_code ?? "n/a"}
        </Badge>
        <Badge tone="neutral">{formatDuration(result.duration_ms)}</Badge>
      </div>
      {result.stdout ? (
        <OutputBlock
          title="stdout"
          content={result.stdout}
          truncated={result.stdout_truncated}
        />
      ) : null}
      {result.stderr ? (
        <OutputBlock
          title="stderr"
          content={result.stderr}
          truncated={result.stderr_truncated}
        />
      ) : null}
      {!result.stdout && !result.stderr ? (
        <div className="text-sm text-[#536257]">No output</div>
      ) : null}
    </div>
  );
}

function OutputBlock({
  title,
  content,
  truncated,
}: {
  title: string;
  content: string;
  truncated: boolean;
}) {
  return (
    <div>
      <div className="mb-1 flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
        <span>{title}</span>
        {truncated ? <Badge tone="warn">Truncated</Badge> : null}
      </div>
      <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-[#111812] p-3 font-mono text-xs leading-5 text-[#dce8dd]">
        {content}
      </pre>
    </div>
  );
}

function firstInspectablePath(entry: WorkspaceEntry): string | null {
  if (entry.kind !== "directory") {
    return entry.path;
  }
  for (const child of entry.children) {
    const path = firstInspectablePath(child);
    if (path) {
      return path;
    }
  }
  return null;
}

export function workspaceStatusLabel(status: WorkspaceEntryStatus) {
  switch (status) {
    case "readable":
      return "Readable";
    case "directory":
      return "Directory";
    case "large":
      return "Large";
    case "binary":
      return "Binary";
    case "ignored":
      return "Ignored";
    case "generated":
      return "Generated";
    case "permission_denied":
      return "Permission denied";
    case "outside_workspace":
      return "Outside workspace";
    case "missing":
      return "Missing";
    case "not_file":
      return "Not a file";
    case "not_directory":
      return "Not a directory";
    case "symlink":
      return "Symlink";
    case "error":
      return "Error";
  }
}

function statusTone(status: WorkspaceEntryStatus) {
  if (status === "readable") {
    return "good";
  }
  if (status === "directory") {
    return "neutral";
  }
  if (
    status === "permission_denied" ||
    status === "outside_workspace" ||
    status === "missing" ||
    status === "error"
  ) {
    return "bad";
  }
  return "warn";
}

function commandStateTone(state: CommandState) {
  if (state === "completed") {
    return "good";
  }
  if (state === "failed" || state === "blocked" || state === "expired") {
    return "bad";
  }
  if (state === "dispatched" || state === "acknowledged") {
    return "info";
  }
  return "neutral";
}

function commandStateLabel(state: CommandState) {
  return state
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatBytes(bytes: number | null) {
  if (bytes === null) {
    return "Size unknown";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function parseCommandLine(value: string) {
  const parts =
    value
      .match(/"[^"]*"|'[^']*'|\S+/g)
      ?.map((part) => part.replace(/^["']|["']$/g, "")) ?? [];
  const [command, ...args] = parts;
  if (!command) {
    return null;
  }
  return { command, args };
}

function formatCommandLine(command: string, args: string[]) {
  return [command, ...args].join(" ");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function isCommandResult(value: unknown): value is WorkspaceCommandRunResponse {
  return (
    isRecord(value) &&
    typeof value.command === "string" &&
    Array.isArray(value.args) &&
    typeof value.success === "boolean" &&
    typeof value.duration_ms === "number"
  );
}

function historyItemTitle(item: WorkspaceCommandHistoryItem) {
  const payload = isRecord(item.payload) ? item.payload : {};
  if (item.kind === "RunWorkspaceCommand") {
    const command =
      typeof payload.command === "string" ? payload.command : "command";
    const args = Array.isArray(payload.args)
      ? payload.args.filter((arg): arg is string => typeof arg === "string")
      : [];
    return formatCommandLine(command, args);
  }
  if (item.kind === "WriteWorkspaceFile") {
    const path = typeof payload.path === "string" ? payload.path : "file";
    return `Save ${path}`;
  }
  if (item.kind === "ReadWorkspaceDiff") {
    return "Diff snapshot";
  }
  return item.kind;
}

function formatDateTime(value: string) {
  return new Date(value).toLocaleString();
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) {
    return `${durationMs} ms`;
  }
  return `${(durationMs / 1000).toFixed(1)} s`;
}
