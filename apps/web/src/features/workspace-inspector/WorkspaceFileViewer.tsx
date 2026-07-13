import { FileText, Save, ShieldAlert } from "lucide-react";
import { lazy, Suspense } from "react";

import type {
  WorkspaceEntry,
  WorkspaceEntryStatus,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

const MonacoFileEditor = lazy(() =>
  import("./MonacoViews").then((module) => ({
    default: module.MonacoFileEditor,
  })),
);

export function WorkspaceFileViewer({
  placementId,
  selectedPath,
  entry,
  content,
  editorContent,
  isDirty,
  hasConflict,
  isLoading,
  error,
  saveError,
  isSaving,
  onEditorChange,
  onSave,
  onDiscard,
  onReload,
}: {
  placementId: string;
  selectedPath: string | null;
  entry: WorkspaceEntry | null;
  content: string | null;
  editorContent: string;
  isDirty: boolean;
  hasConflict: boolean;
  isLoading: boolean;
  error: unknown;
  saveError: unknown;
  isSaving: boolean;
  onEditorChange: (content: string) => void;
  onSave: () => void;
  onDiscard: () => void;
  onReload: () => void;
}) {
  if (!selectedPath) {
    return (
      <div className="flex h-full min-h-48 items-center justify-center text-sm text-[var(--color-muted)]">
        No text file selected
      </div>
    );
  }
  if (error) {
    return (
      <div className="h-full overflow-auto p-3">
        <ErrorNotice error={error} title="File unavailable" />
      </div>
    );
  }
  if (isLoading || !entry) {
    return (
      <div className="h-full p-3 text-sm text-[var(--color-muted)]">
        Loading file
      </div>
    );
  }

  const canEdit = content !== null && entry.status === "readable";
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex flex-wrap items-start justify-between gap-2 border-b border-[var(--color-muted)] px-3 py-2">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <FileText size={15} className="shrink-0" />
            <span className="truncate font-mono text-sm">{entry.path}</span>
          </div>
          <div className="mt-1 flex flex-wrap gap-2 text-xs text-[var(--color-muted)]">
            <span>{formatBytes(entry.byte_len)}</span>
            {entry.modified_at ? (
              <span>{new Date(entry.modified_at).toLocaleString()}</span>
            ) : null}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone={statusTone(entry.status)}>
            {statusLabel(entry.status)}
          </Badge>
          {isDirty ? <Badge tone="warn">Modified</Badge> : null}
          {hasConflict ? <Badge tone="bad">Conflict</Badge> : null}
          {isDirty ? (
            <Button variant="secondary" disabled={isSaving} onClick={onDiscard}>
              Discard
            </Button>
          ) : null}
          {hasConflict ? (
            <Button variant="secondary" disabled={isSaving} onClick={onReload}>
              Reload remote
            </Button>
          ) : null}
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
        <div className="border-b border-[var(--color-risk)] p-3">
          <ErrorNotice error={saveError} title="Save failed" />
        </div>
      ) : null}
      {hasConflict ? (
        <div className="border-b border-[var(--color-muted)] bg-[var(--color-bg-muted)] px-3 py-2 text-sm text-[var(--color-muted)]">
          The file changed on the node. Your draft is preserved; save to test
          against its original base, or reload the remote version.
        </div>
      ) : null}
      {canEdit ? (
        <Suspense fallback={<Fallback />}>
          <MonacoFileEditor
            placementId={placementId}
            path={entry.path}
            value={editorContent}
            readOnly={false}
            onChange={onEditorChange}
          />
        </Suspense>
      ) : (
        <div className="flex min-h-0 flex-1 items-center justify-center gap-2 p-6 text-sm text-[var(--color-muted)]">
          <ShieldAlert size={17} />
          <span>{statusLabel(entry.status)}</span>
        </div>
      )}
    </div>
  );
}

function Fallback() {
  return (
    <div className="flex min-h-24 flex-1 items-center justify-center">
      Loading editor
    </div>
  );
}

function statusLabel(status: WorkspaceEntryStatus) {
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusTone(status: WorkspaceEntryStatus) {
  if (status === "readable" || status === "directory") return "good";
  if (
    ["permission_denied", "outside_workspace", "missing", "error"].includes(
      status,
    )
  ) {
    return "bad";
  }
  return "warn";
}

function formatBytes(bytes: number | null) {
  if (bytes === null) return "Size unknown";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}
