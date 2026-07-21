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
  ThemeContributionV1,
  VisualRendererContributionV1,
} from "../shared/protocol/types";
import {
  cacheEffectiveTheme,
  CORE_LIGHT_THEME_ID,
  readSelectedTheme,
  rememberSelectedTheme,
} from "./appearance-preference";
import {
  CONTENT_RENDERER_CONTRACT_VERSION,
  type ContentRendererProps,
  type LazyContentRenderer,
} from "./content-renderers";
import {
  applyTheme,
  CORE_LIGHT_THEME,
  normalizeTheme,
  THEME_CONTRACT_VERSION,
} from "./themes";

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
  renderers: VisualRendererContributionV1[];
};

const DEFAULT_CONTENT_RENDERER_HOST: ContentRendererHostValue = {
  renderers: [],
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
      if (
        item.kind !== "ui_theme" ||
        item.contract_version !== THEME_CONTRACT_VERSION
      ) {
        continue;
      }
      const theme = normalizeTheme(item.contribution);
      if (
        theme &&
        !resolved.some((current) => current.theme_id === theme.theme_id)
      ) {
        resolved.push(theme);
      }
    }
    return resolved;
  }, [contributions.data?.contributions]);
  const renderers = useMemo(
    () =>
      (contributions.data?.contributions ?? []).flatMap((item) =>
        item.kind === "visual_renderer" &&
        item.contract_version === CONTENT_RENDERER_CONTRACT_VERSION
          ? [item.contribution]
          : [],
      ),
    [contributions.data?.contributions],
  );
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
    () => ({ renderers }),
    [renderers],
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
  const registration = host.renderers.find(
    (renderer) =>
      renderer.renderer_kind === "content" &&
      renderer.render_scopes.includes("content_enhancement") &&
      renderer.accepted_source_kinds.includes(sourceKind) &&
      renderer.allowed_surfaces.includes(surfaceId),
  );
  if (!registration) {
    return fallback;
  }
  const Renderer = bundledContentRenderers.get(registration.implementation_id);
  if (!Renderer) {
    return fallback;
  }
  return (
    <RendererErrorBoundary
      key={registration.renderer_id}
      fallback={fallback}
      rendererId={registration.renderer_id}
    >
      <Suspense fallback={fallback}>
        <Renderer content={content} state={state} sourceRef={sourceRef} />
      </Suspense>
    </RendererErrorBoundary>
  );
}

class RendererErrorBoundary extends Component<
  PropsWithChildren<{
    fallback: ReactNode;
    rendererId: string;
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
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}
