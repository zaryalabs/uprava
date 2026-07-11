import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { History, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  CommandState,
  WorkspaceCommandHistoryItem,
  WorkspaceCommandRunResponse,
  WorkspaceEntryStatus,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { useWorkspaceDraft } from "./WorkspaceDrafts";
import {
  WorkspaceCommandPanel,
  type RunCommandInput,
} from "./WorkspaceCommandPanel";
import { WorkspaceDiffPanel } from "./WorkspaceDiffPanel";
import { WorkspaceFileViewer } from "./WorkspaceFileViewer";
import { WorkspaceFileTree } from "./WorkspaceFileTree";
import { WorkspaceTerminalPanel } from "./WorkspaceTerminalPanel";

type WorkspaceInspectorProps = {
  placementId: string;
  workspacePath: string;
};

export function WorkspaceInspector({
  placementId,
  workspacePath,
}: WorkspaceInspectorProps) {
  const queryClient = useQueryClient();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [treeRefreshVersion, setTreeRefreshVersion] = useState(0);
  const [commandText, setCommandText] = useState("make l");
  const [lastCommandResult, setLastCommandResult] =
    useState<WorkspaceCommandRunResponse | null>(null);

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
  const selectedRemoteContent =
    selectedFile.data?.path === selectedPath ? selectedFile.data.content : null;
  const fileDraft = useWorkspaceDraft(
    placementId,
    selectedPath,
    selectedRemoteContent,
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
      fileDraft.markSaved(request.content);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceFile(placementId, request.path),
      });
      void queryClient.invalidateQueries({
        queryKey: ["placement", placementId, "workspace-tree"],
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceCommandHistory(placementId),
      });
    },
    onError: (_error, request) => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceFile(placementId, request.path),
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

  const editorContent = fileDraft.draft?.localContent ?? "";
  const savedContent = fileDraft.draft?.baseContent ?? null;
  const isDirty = fileDraft.draft?.dirty ?? false;

  const refetchWorkspace = () => {
    void queryClient.invalidateQueries({
      queryKey: ["placement", placementId, "workspace-tree"],
    });
    setTreeRefreshVersion((version) => version + 1);
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
          <h2 className="text-sm font-semibold uppercase tracking-normal text-[var(--color-muted)]">
            Workspace Inspector
          </h2>
          <div className="truncate font-mono text-xs text-[var(--color-muted)]">
            {workspacePath}
          </div>
        </div>
        <Button
          variant="secondary"
          disabled={history.isFetching}
          onClick={refetchWorkspace}
        >
          <RefreshCw size={15} />
          Refresh
        </Button>
      </div>

      <div className="grid min-h-[620px] grid-cols-[minmax(220px,320px)_minmax(0,1fr)] gap-3 max-xl:grid-cols-1">
        <section className="min-h-0 border border-[var(--color-muted)] bg-[var(--color-bg)]">
          <div className="border-b border-[var(--color-muted)] px-3 py-2 text-xs font-medium uppercase tracking-normal text-[var(--color-muted)]">
            Files
          </div>
          <div className="max-h-[720px] overflow-auto py-2">
            <WorkspaceFileTree
              key={placementId}
              placementId={placementId}
              selectedPath={selectedPath}
              refreshVersion={treeRefreshVersion}
              onSelect={setSelectedPath}
            />
          </div>
        </section>

        <div className="min-w-0 space-y-3">
          <section className="min-h-0 border border-[var(--color-muted)] bg-[var(--color-bg)]">
            <WorkspaceFileViewer
              placementId={placementId}
              selectedPath={selectedPath}
              entry={selectedFile.data?.metadata ?? null}
              content={selectedFile.data?.content ?? null}
              editorContent={editorContent}
              isDirty={isDirty}
              hasConflict={fileDraft.draft?.conflict ?? false}
              isLoading={selectedFile.isLoading}
              error={selectedFile.error}
              saveError={saveMutation.error}
              isSaving={saveMutation.isPending}
              onEditorChange={fileDraft.edit}
              onSave={saveSelectedFile}
              onDiscard={fileDraft.discard}
              onReload={fileDraft.reload}
            />
          </section>

          <WorkspaceTerminalPanel placementId={placementId} />

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
    <section className="border border-[var(--color-muted)] bg-[var(--color-bg)]">
      <div className="flex items-center gap-2 border-b border-[var(--color-muted)] px-3 py-2 text-xs font-medium uppercase tracking-normal text-[var(--color-muted)]">
        <History size={15} />
        History
      </div>
      <div className="space-y-2 p-3">
        {error ? (
          <ErrorNotice error={error} title="History unavailable" />
        ) : null}
        {isLoading ? (
          <div className="text-sm text-[var(--color-muted)]">
            Loading history
          </div>
        ) : null}
        {!isLoading && commands.length === 0 ? (
          <div className="text-sm text-[var(--color-muted)]">
            No commands recorded
          </div>
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
    <div className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate font-mono text-sm text-[var(--color-ink)]">
            {historyItemTitle(item)}
          </div>
          <div className="mt-1 text-xs text-[var(--color-muted)]">
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
        <div className="mt-2 truncate text-xs text-[var(--color-muted)]">
          exit {result.exit_code ?? "n/a"} ·{" "}
          {formatDuration(result.duration_ms)}
        </div>
      ) : null}
    </div>
  );
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
