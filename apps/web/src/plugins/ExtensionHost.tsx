import { useQuery } from "@tanstack/react-query";
import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

import { coreApi } from "../shared/api/http-client";
import { queryKeys } from "../shared/api/query-keys";
import type { ThemeContributionV1 } from "../shared/protocol/types";
import {
  cacheEffectiveTheme,
  CORE_LIGHT_THEME_ID,
  readSelectedTheme,
  rememberSelectedTheme,
} from "./appearance-preference";
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

  return (
    <ThemeHostContext.Provider value={value}>
      {children}
    </ThemeHostContext.Provider>
  );
}

export function useThemeHost() {
  return useContext(ThemeHostContext);
}
