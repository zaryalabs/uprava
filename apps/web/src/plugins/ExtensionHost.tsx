import { useQuery } from "@tanstack/react-query";
import {
  Component,
  createContext,
  lazy,
  type PropsWithChildren,
  type ErrorInfo,
  type ReactNode,
  Suspense,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

import { coreApi } from "../shared/api/http-client";
import { queryKeys } from "../shared/api/query-keys";
import { logClientEvent } from "../shared/logging/client-logger";
import type {
  EffectivePluginSnapshot,
  ThemeContributionV1,
} from "../shared/protocol/types";
import {
  cacheEffectiveTheme,
  CORE_LIGHT_THEME_ID,
  readSelectedTheme,
  rememberSelectedTheme,
} from "./appearance-preference";
import {
  type ContentRendererProps,
  type LazyContentRenderer,
} from "./content-renderers";
import {
  applyTheme,
  CORE_LIGHT_THEME,
  normalizeTheme,
  THEME_CONTRACT_VERSION,
} from "./themes";
import { resolveVisualRendererChain } from "./contribution-resolution";

type ThemeHostValue = {
  themes: ThemeContributionV1[];
  selectedThemeId: string;
  effectiveTheme: ThemeContributionV1;
  selectTheme: (themeId: string) => void;
  isLoading: boolean;
  isError: boolean;
};

const DEFAULT_THEME_HOST: ThemeHostValue = {
  themes: [CORE_LIGHT_THEME],
  selectedThemeId: CORE_LIGHT_THEME_ID,
  effectiveTheme: CORE_LIGHT_THEME,
  selectTheme: () => undefined,
  isLoading: false,
  isError: false,
};

const ThemeHostContext = createContext<ThemeHostValue>(DEFAULT_THEME_HOST);

type ContentRendererHostValue = {
  snapshot: EffectivePluginSnapshot | undefined;
};

const DEFAULT_CONTENT_RENDERER_HOST: ContentRendererHostValue = {
  snapshot: undefined,
};

const ContentRendererHostContext = createContext<ContentRendererHostValue>(
  DEFAULT_CONTENT_RENDERER_HOST,
);

const bundledContentRenderers = new Map<string, LazyContentRenderer>([
  [
    "uprava.markdown.v1",
    lazy(() =>
      import("./bundled/markdown/MarkdownRenderer").then((module) => ({
        default: module.MarkdownRenderer,
      })),
    ),
  ],
  [
    "uprava.plain-text.v1",
    lazy(() =>
      import("./bundled/plain-text/PlainTextRenderer").then((module) => ({
        default: module.PlainTextRenderer,
      })),
    ),
  ],
]);

export function ExtensionHostProvider({ children }: PropsWithChildren) {
  const contributions = useQuery({
    queryKey: queryKeys.pluginContributions,
    queryFn: coreApi.pluginContributions,
  });
  const [selectedThemeId, setSelectedThemeId] = useState(readSelectedTheme);
  const themes = useMemo(() => {
    const resolved = [CORE_LIGHT_THEME];
    for (const item of contributions.data?.contributions ?? []) {
      const declared = item.contribution;
      if (
        item.effective_state !== "available" ||
        declared.kind !== "ui_theme" ||
        item.contract_version !== THEME_CONTRACT_VERSION
      ) {
        continue;
      }
      const theme = normalizeTheme(declared.contribution);
      if (
        theme &&
        !resolved.some((current) => current.theme_id === theme.theme_id)
      ) {
        resolved.push(theme);
      }
    }
    return resolved;
  }, [contributions.data?.contributions]);
  const effectiveTheme =
    themes.find((theme) => theme.theme_id === selectedThemeId) ??
    CORE_LIGHT_THEME;

  useEffect(() => {
    applyTheme(effectiveTheme);
    cacheEffectiveTheme(effectiveTheme.theme_id);
  }, [effectiveTheme]);

  const selectTheme = useCallback(
    (themeId: string) => {
      if (!themes.some((theme) => theme.theme_id === themeId)) return;
      rememberSelectedTheme(themeId);
      setSelectedThemeId(themeId);
    },
    [themes],
  );

  const value = useMemo<ThemeHostValue>(
    () => ({
      themes,
      selectedThemeId,
      effectiveTheme,
      selectTheme,
      isLoading: contributions.isLoading,
      isError: contributions.isError,
    }),
    [
      contributions.isError,
      contributions.isLoading,
      effectiveTheme,
      selectTheme,
      selectedThemeId,
      themes,
    ],
  );
  const rendererValue = useMemo<ContentRendererHostValue>(
    () => ({ snapshot: contributions.data }),
    [contributions.data],
  );

  return (
    <ThemeHostContext.Provider value={value}>
      <ContentRendererHostContext.Provider value={rendererValue}>
        {children}
      </ContentRendererHostContext.Provider>
    </ThemeHostContext.Provider>
  );
}

export function useThemeHost() {
  return useContext(ThemeHostContext);
}

export function PluginContentRenderer({
  sourceKind,
  surfaceId,
  content,
  state,
  sourceRef,
  fallback,
}: ContentRendererProps & {
  sourceKind: string;
  surfaceId: string;
  fallback: ReactNode;
}) {
  const host = useContext(ContentRendererHostContext);
  const candidates = resolveVisualRendererChain(
    host.snapshot,
    sourceKind,
    surfaceId,
  );
  const chainKey = candidates
    .map((candidate) => `${candidate.plugin_id}:${candidate.contribution_id}`)
    .join("|");
  return (
    <ExclusiveRendererChain
      key={chainKey}
      candidates={candidates}
      content={content}
      state={state}
      sourceRef={sourceRef}
      fallback={fallback}
    />
  );
}

function ExclusiveRendererChain({
  candidates,
  content,
  state,
  sourceRef,
  fallback,
}: ContentRendererProps & {
  candidates: ReturnType<typeof resolveVisualRendererChain>;
  fallback: ReactNode;
}) {
  const [failedCount, setFailedCount] = useState(0);
  const available = candidates.flatMap((candidate) => {
    if (candidate.contribution.kind !== "visual_renderer") return [];
    const Renderer = bundledContentRenderers.get(
      candidate.contribution.contribution.implementation_id,
    );
    return Renderer
      ? [
          {
            Renderer,
            rendererId: candidate.contribution.contribution.renderer_id,
          },
        ]
      : [];
  });
  const selected = available[failedCount];
  if (!selected) return fallback;

  const rendererId = selected.rendererId;
  return (
    <RendererErrorBoundary
      key={rendererId}
      fallback={fallback}
      rendererId={rendererId}
      onFailure={() => setFailedCount((count) => count + 1)}
    >
      <Suspense fallback={fallback}>
        <selected.Renderer
          content={content}
          state={state}
          sourceRef={sourceRef}
        />
      </Suspense>
    </RendererErrorBoundary>
  );
}

class RendererErrorBoundary extends Component<
  PropsWithChildren<{
    fallback: ReactNode;
    rendererId: string;
    onFailure: () => void;
  }>,
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    logClientEvent("error", "web.plugin_renderer", error.message, {
      renderer_id: this.props.rendererId,
      component_stack: info.componentStack,
    });
    this.props.onFailure();
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}
