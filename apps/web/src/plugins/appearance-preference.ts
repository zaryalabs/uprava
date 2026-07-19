export const CORE_LIGHT_THEME_ID = "core.light";
export const DARK_THEME_ID = "uprava.dark";
export const APPEARANCE_PREFERENCE_KEY = "uprava.appearance.v1";
export const EFFECTIVE_THEME_CACHE_KEY = "uprava.theme-cache.v1";

type AppearancePreferenceV1 = {
  version: 1;
  selected_theme_id: string;
};

type EffectiveThemeCacheV1 = {
  version: 1;
  effective_theme_id: typeof CORE_LIGHT_THEME_ID | typeof DARK_THEME_ID;
  palette_version: 1;
};

export function readSelectedTheme(): string {
  try {
    const value = JSON.parse(
      window.localStorage.getItem(APPEARANCE_PREFERENCE_KEY) ?? "null",
    ) as Partial<AppearancePreferenceV1> | null;
    return value?.version === 1 && typeof value.selected_theme_id === "string"
      ? value.selected_theme_id
      : CORE_LIGHT_THEME_ID;
  } catch {
    return CORE_LIGHT_THEME_ID;
  }
}

export function rememberSelectedTheme(themeId: string) {
  try {
    const value: AppearancePreferenceV1 = {
      version: 1,
      selected_theme_id: themeId,
    };
    window.localStorage.setItem(
      APPEARANCE_PREFERENCE_KEY,
      JSON.stringify(value),
    );
  } catch {
    // Storage is optional; the in-memory preference remains usable.
  }
}

export function cacheEffectiveTheme(themeId: string) {
  const effectiveThemeId =
    themeId === DARK_THEME_ID ? DARK_THEME_ID : CORE_LIGHT_THEME_ID;
  try {
    const value: EffectiveThemeCacheV1 = {
      version: 1,
      effective_theme_id: effectiveThemeId,
      palette_version: 1,
    };
    window.localStorage.setItem(
      EFFECTIVE_THEME_CACHE_KEY,
      JSON.stringify(value),
    );
  } catch {
    // The safe light fallback does not depend on local storage.
  }
}

export function applyCachedThemeMarker() {
  try {
    const value = JSON.parse(
      window.localStorage.getItem(EFFECTIVE_THEME_CACHE_KEY) ?? "null",
    ) as Partial<EffectiveThemeCacheV1> | null;
    const themeId =
      value?.version === 1 &&
      value.palette_version === 1 &&
      value.effective_theme_id === DARK_THEME_ID
        ? DARK_THEME_ID
        : CORE_LIGHT_THEME_ID;
    document.documentElement.dataset.theme = themeId;
    document.documentElement.style.colorScheme =
      themeId === DARK_THEME_ID ? "dark" : "light";
  } catch {
    document.documentElement.dataset.theme = CORE_LIGHT_THEME_ID;
    document.documentElement.style.colorScheme = "light";
  }
}
