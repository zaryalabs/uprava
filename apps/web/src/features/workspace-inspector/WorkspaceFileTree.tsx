import {
  asyncDataLoaderFeature,
  hotkeysCoreFeature,
  selectionFeature,
} from "@headless-tree/core";
import { useTree } from "@headless-tree/react";
import { useQueryClient } from "@tanstack/react-query";
import {
  ChevronDown,
  ChevronRight,
  File,
  Folder,
  FolderOpen,
  Link,
  LoaderCircle,
  RotateCcw,
} from "lucide-react";
import { useEffect, useMemo, useRef } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  WorkspaceEntry,
  WorkspaceTreeResponse,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";

type TreeItemData =
  | { type: "entry"; entry: WorkspaceEntry }
  | { type: "loading" }
  | { type: "truncated"; parentPath: string; total: number | null }
  | { type: "error"; parentPath: string; message: string };

const ROOT_ID = ".";

export function WorkspaceFileTree({
  placementId,
  selectedPath,
  refreshVersion,
  onSelect,
}: {
  placementId: string;
  selectedPath: string | null;
  refreshVersion: number;
  onSelect: (path: string) => void;
}) {
  const queryClient = useQueryClient();
  const appliedRefreshVersion = useRef(0);
  const dataLoader = useMemo(
    () => ({
      getItem: async (itemId: string): Promise<TreeItemData> => {
        const response = await fetchDirectory(queryClient, placementId, itemId);
        return { type: "entry", entry: response.root };
      },
      getChildrenWithData: async (itemId: string) => {
        try {
          const response = await fetchDirectory(
            queryClient,
            placementId,
            itemId,
          );
          const children: { id: string; data: TreeItemData }[] =
            response.root.children.map((entry) => ({
              id: entry.path,
              data: { type: "entry", entry },
            }));
          if (response.truncated) {
            children.push({
              id: truncationId(itemId),
              data: {
                type: "truncated",
                parentPath: itemId,
                total: response.total_entries,
              },
            });
          }
          return children;
        } catch (error) {
          return [
            {
              id: errorId(itemId),
              data: {
                type: "error" as const,
                parentPath: itemId,
                message:
                  error instanceof Error
                    ? error.message
                    : "Directory unavailable",
              },
            },
          ];
        }
      },
    }),
    [placementId, queryClient],
  );

  const tree = useTree<TreeItemData>({
    rootItemId: ROOT_ID,
    initialState: { expandedItems: [ROOT_ID] },
    dataLoader,
    createLoadingItemData: () => ({ type: "loading" }),
    getItemName: (item) => itemName(item.getItemData()),
    isItemFolder: (item) => {
      const data = item.getItemData();
      return data.type === "entry" && data.entry.expandable;
    },
    onPrimaryAction: (item) => {
      const data = item.getItemData();
      if (data.type === "entry" && data.entry.kind !== "directory") {
        onSelect(data.entry.path);
      }
      if (data.type === "error") {
        void item.getParent()?.invalidateChildrenIds();
      }
    },
    indent: 14,
    features: [asyncDataLoaderFeature, selectionFeature, hotkeysCoreFeature],
  });

  useEffect(() => {
    if (
      refreshVersion === 0 ||
      appliedRefreshVersion.current === refreshVersion
    )
      return;
    appliedRefreshVersion.current = refreshVersion;
    for (const item of tree.getItems()) {
      if (item.isFolder()) void item.invalidateChildrenIds(true);
    }
  }, [refreshVersion, tree]);

  return (
    <div {...tree.getContainerProps("Workspace files")} className="py-1">
      {tree.getItems().map((item) => {
        const data = item.getItemData();
        const level = item.getItemMeta().level;
        const isSelected =
          data.type === "entry" && data.entry.path === selectedPath;
        return (
          <button
            {...item.getProps()}
            key={item.getKey()}
            type="button"
            className={`flex min-h-8 w-full items-center gap-1.5 py-1 pr-2 text-left text-sm outline-none hover:bg-[var(--color-bg-muted)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--color-ink)] ${
              isSelected
                ? "bg-[var(--color-bg-muted)] text-[var(--color-ink)]"
                : "text-[var(--color-ink)]"
            } ${classificationClass(data)}`}
            style={{ paddingLeft: 8 + level * 14 }}
          >
            <TreeItemIcon data={data} expanded={item.isExpanded()} />
            <span className="min-w-0 flex-1 truncate">{itemName(data)}</span>
            {data.type === "entry" && data.entry.classification !== "normal" ? (
              <Badge tone="neutral">{data.entry.classification}</Badge>
            ) : null}
            {data.type === "entry" &&
            data.entry.status !== "directory" &&
            data.entry.status !== "readable" ? (
              <Badge tone={entryStatusTone(data.entry.status)}>
                {data.entry.status.replaceAll("_", " ")}
              </Badge>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

async function fetchDirectory(
  queryClient: ReturnType<typeof useQueryClient>,
  placementId: string,
  path: string,
) {
  return queryClient.fetchQuery<WorkspaceTreeResponse>({
    queryKey: queryKeys.workspaceTree(placementId, path),
    queryFn: () => coreApi.workspaceTree(placementId, path),
  });
}

function TreeItemIcon({
  data,
  expanded,
}: {
  data: TreeItemData;
  expanded: boolean;
}) {
  if (data.type === "loading") {
    return <LoaderCircle size={14} className="shrink-0 animate-spin" />;
  }
  if (data.type === "error") {
    return (
      <RotateCcw size={14} className="shrink-0 text-[var(--color-risk)]" />
    );
  }
  if (data.type === "truncated") {
    return <span className="w-3.5 shrink-0 text-center">…</span>;
  }
  const entry = data.entry;
  if (entry.kind === "directory") {
    return (
      <>
        {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        {expanded ? <FolderOpen size={14} /> : <Folder size={14} />}
      </>
    );
  }
  if (entry.kind === "symlink") return <Link size={14} className="shrink-0" />;
  return <File size={14} className="shrink-0" />;
}

function itemName(data: TreeItemData) {
  if (data.type === "loading") return "Loading…";
  if (data.type === "error") return `${data.message} — retry`;
  if (data.type === "truncated") {
    return data.total === null
      ? "Only the first 100 entries are shown"
      : `Only the first 100 of ${data.total} entries are shown`;
  }
  return data.entry.name;
}

function classificationClass(data: TreeItemData) {
  return data.type === "entry" && data.entry.classification !== "normal"
    ? "text-[var(--color-muted)]"
    : "";
}

function entryStatusTone(status: WorkspaceEntry["status"]) {
  return status === "permission_denied" ||
    status === "missing" ||
    status === "error"
    ? "bad"
    : "warn";
}

function truncationId(path: string) {
  return `${path}\u0000truncated`;
}

function errorId(path: string) {
  return `${path}\u0000error`;
}
