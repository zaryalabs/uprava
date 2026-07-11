import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type WorkspaceDraft = {
  baseContent: string;
  localContent: string;
  dirty: boolean;
  conflict: boolean;
  remoteContent: string | null;
};

type DraftStore = Record<string, WorkspaceDraft>;

type WorkspaceDraftContextValue = {
  drafts: DraftStore;
  update(
    key: string,
    updater: (draft: WorkspaceDraft | undefined) => WorkspaceDraft,
  ): void;
};

const WorkspaceDraftContext = createContext<WorkspaceDraftContextValue | null>(
  null,
);

export function WorkspaceDraftProvider({ children }: { children: ReactNode }) {
  const [drafts, setDrafts] = useState<DraftStore>({});
  const update = useCallback(
    (
      key: string,
      updater: (draft: WorkspaceDraft | undefined) => WorkspaceDraft,
    ) => {
      setDrafts((current) => ({ ...current, [key]: updater(current[key]) }));
    },
    [],
  );

  const hasDirtyDraft = Object.values(drafts).some((draft) => draft.dirty);
  useEffect(() => {
    if (!hasDirtyDraft) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [hasDirtyDraft]);

  const value = useMemo(() => ({ drafts, update }), [drafts, update]);
  return (
    <WorkspaceDraftContext.Provider value={value}>
      {children}
    </WorkspaceDraftContext.Provider>
  );
}

export function useWorkspaceDraft(
  placementId: string,
  path: string | null,
  remoteContent: string | null | undefined,
) {
  const context = useContext(WorkspaceDraftContext);
  if (!context) {
    throw new Error("useWorkspaceDraft requires WorkspaceDraftProvider");
  }
  const key = path ? `${placementId}\u0000${path}` : null;
  const draft = key ? context.drafts[key] : undefined;

  useEffect(() => {
    if (!key || remoteContent == null) return;
    context.update(key, (current) => receiveRemote(current, remoteContent));
  }, [context.update, key, remoteContent]);

  const mutate = useCallback(
    (updater: (current: WorkspaceDraft) => WorkspaceDraft) => {
      if (!key) return;
      context.update(key, (current) => updater(current ?? emptyDraft()));
    },
    [context.update, key],
  );

  return {
    draft,
    edit: (content: string) =>
      mutate((current) => ({
        ...current,
        localContent: content,
        dirty: content !== current.baseContent,
      })),
    markSaved: (content: string) =>
      mutate(() => ({
        baseContent: content,
        localContent: content,
        dirty: false,
        conflict: false,
        remoteContent: null,
      })),
    discard: () =>
      mutate((current) => ({
        ...current,
        localContent: current.baseContent,
        dirty: false,
        conflict: false,
        remoteContent: null,
      })),
    reload: () =>
      mutate((current) => {
        const content = current.remoteContent ?? current.baseContent;
        return {
          baseContent: content,
          localContent: content,
          dirty: false,
          conflict: false,
          remoteContent: null,
        };
      }),
  };
}

export function receiveRemote(
  current: WorkspaceDraft | undefined,
  remoteContent: string,
): WorkspaceDraft {
  if (!current || !current.dirty) {
    return {
      baseContent: remoteContent,
      localContent: remoteContent,
      dirty: false,
      conflict: false,
      remoteContent: null,
    };
  }
  if (remoteContent === current.baseContent) return current;
  return { ...current, conflict: true, remoteContent };
}

function emptyDraft(): WorkspaceDraft {
  return {
    baseContent: "",
    localContent: "",
    dirty: false,
    conflict: false,
    remoteContent: null,
  };
}
