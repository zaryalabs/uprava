import { useQuery } from "@tanstack/react-query";
import { File, FileText, Folder, RefreshCw, ShieldAlert } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  WorkspaceEntry,
  WorkspaceEntryStatus,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

type WorkspaceInspectorProps = {
  placementId: string;
  workspacePath: string;
};

export function WorkspaceInspector({
  placementId,
  workspacePath,
}: WorkspaceInspectorProps) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
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
  const firstFilePath = useMemo(
    () => (tree.data ? firstInspectablePath(tree.data.root) : null),
    [tree.data],
  );

  useEffect(() => {
    setSelectedPath(null);
  }, [placementId]);

  useEffect(() => {
    if (!selectedPath && firstFilePath) {
      setSelectedPath(firstFilePath);
    }
  }, [firstFilePath, selectedPath]);

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
          disabled={tree.isFetching}
          onClick={() => void tree.refetch()}
        >
          <RefreshCw size={15} />
          Refresh
        </Button>
      </div>

      <div className="grid min-h-[520px] grid-cols-[minmax(220px,320px)_minmax(0,1fr)] gap-3 max-xl:grid-cols-1">
        <section className="min-h-0 rounded-md border border-[#d9ded4] bg-white">
          <div className="border-b border-[#e0e5db] px-3 py-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
            Files
          </div>
          <div className="max-h-[600px] overflow-auto py-2">
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

        <section className="min-h-0 rounded-md border border-[#d9ded4] bg-white">
          <WorkspaceFileViewer
            selectedPath={selectedPath}
            entry={selectedFile.data?.metadata ?? null}
            content={selectedFile.data?.content ?? null}
            isLoading={selectedFile.isLoading}
            error={selectedFile.error}
          />
        </section>
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
  isLoading,
  error,
}: {
  selectedPath: string | null;
  entry: WorkspaceEntry | null;
  content: string | null;
  isLoading: boolean;
  error: unknown;
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

  return (
    <div className="flex max-h-[650px] min-h-[520px] flex-col">
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
        <Badge tone={statusTone(entry.status)}>
          {workspaceStatusLabel(entry.status)}
        </Badge>
      </div>

      {content !== null ? (
        <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words p-3 font-mono text-xs leading-5 text-[#17211c]">
          {content}
        </pre>
      ) : (
        <div className="flex min-h-0 flex-1 items-center justify-center gap-2 p-6 text-sm text-[#536257]">
          <ShieldAlert size={17} />
          <span>{workspaceStatusLabel(entry.status)}</span>
        </div>
      )}
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
