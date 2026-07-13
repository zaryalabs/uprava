import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Button } from "../../shared/ui/button";
import { useWorkspaceDraft } from "./WorkspaceDrafts";
import { WorkspaceDiffPanel } from "./WorkspaceDiffPanel";
import { WorkspaceFileViewer } from "./WorkspaceFileViewer";
import { WorkspaceFileTree } from "./WorkspaceFileTree";
import { WorkspaceTerminalPanel } from "./WorkspaceTerminalPanel";

type EditorMode = "source" | "diff";

export function WorkspaceWorkbench({
  placementId,
  workspacePath,
  actions,
}: {
  placementId: string;
  workspacePath: string;
  actions?: ReactNode;
}) {
  const queryClient = useQueryClient();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [treeRefreshVersion, setTreeRefreshVersion] = useState(0);
  const [editorMode, setEditorMode] = useState<EditorMode>("source");

  const selectedFile = useQuery({
    queryKey: queryKeys.workspaceFile(placementId, selectedPath ?? ""),
    queryFn: () => coreApi.workspaceFile(placementId, selectedPath ?? "."),
    enabled: Boolean(placementId && selectedPath),
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
    },
    onError: (_error, request) => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceFile(placementId, request.path),
      });
    },
  });

  useEffect(() => {
    setSelectedPath(null);
    setEditorMode("source");
  }, [placementId]);

  const editorContent = fileDraft.draft?.localContent ?? "";
  const savedContent = fileDraft.draft?.baseContent ?? null;
  const isDirty = fileDraft.draft?.dirty ?? false;

  const refreshTree = () => {
    void queryClient.invalidateQueries({
      queryKey: ["placement", placementId, "workspace-tree"],
    });
    setTreeRefreshVersion((version) => version + 1);
    if (selectedPath) void selectedFile.refetch();
  };

  const selectFile = (path: string) => {
    setSelectedPath(path);
    setEditorMode("source");
  };

  const saveSelectedFile = () => {
    if (!selectedPath || savedContent === null) return;
    saveMutation.mutate({
      path: selectedPath,
      content: editorContent,
      expected: savedContent,
    });
  };

  const openDiff = () => {
    setEditorMode("diff");
    if (!diff.data && !diff.isFetching) void diff.refetch();
  };

  return (
    <section className="min-w-0" aria-labelledby="workspace-workbench-title">
      <div className="mb-3 flex min-w-0 flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <h2
            id="workspace-workbench-title"
            className="text-sm font-semibold uppercase tracking-normal text-[var(--color-muted)]"
          >
            Workbench
          </h2>
          <div className="truncate font-mono text-xs text-[var(--color-muted)]">
            {workspacePath}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {actions}
          <Button
            variant="secondary"
            aria-label="Refresh workspace files"
            onClick={refreshTree}
          >
            <RefreshCw size={15} />
            Refresh files
          </Button>
        </div>
      </div>

      <div className="uprava-workbench-grid">
        <aside className="uprava-workbench-files" aria-label="File tree pane">
          <div className="uprava-pane-heading">Files</div>
          <div className="min-h-0 flex-1 overflow-auto py-1">
            <WorkspaceFileTree
              key={placementId}
              placementId={placementId}
              selectedPath={selectedPath}
              refreshVersion={treeRefreshVersion}
              onSelect={selectFile}
            />
          </div>
        </aside>

        <section
          className="uprava-workbench-editor"
          aria-label="Editor and diff pane"
        >
          <div
            className="uprava-pane-tabs"
            role="tablist"
            aria-label="Editor mode"
          >
            <button
              type="button"
              role="tab"
              id="workbench-source-tab"
              aria-controls="workbench-source-panel"
              aria-selected={editorMode === "source"}
              className="uprava-pane-tab"
              onClick={() => setEditorMode("source")}
            >
              Source
              {isDirty ? <span aria-label="Modified">●</span> : null}
            </button>
            <button
              type="button"
              role="tab"
              id="workbench-diff-tab"
              aria-controls="workbench-diff-panel"
              aria-selected={editorMode === "diff"}
              className="uprava-pane-tab"
              onClick={openDiff}
            >
              Diff
            </button>
          </div>
          <div className="min-h-0 flex-1 overflow-hidden">
            {editorMode === "source" ? (
              <div
                id="workbench-source-panel"
                role="tabpanel"
                aria-labelledby="workbench-source-tab"
                className="h-full min-h-0"
              >
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
              </div>
            ) : (
              <div
                id="workbench-diff-panel"
                role="tabpanel"
                aria-labelledby="workbench-diff-tab"
                className="h-full min-h-0"
              >
                <WorkspaceDiffPanel
                  isLoading={diff.isFetching}
                  error={diff.error}
                  diff={diff.data ?? null}
                  onRefresh={() => void diff.refetch()}
                />
              </div>
            )}
          </div>
        </section>

        <WorkspaceTerminalPanel placementId={placementId} />
      </div>
    </section>
  );
}
