import { afterEach, describe, expect, it } from "vitest";

import {
  applyCachedThemeMarker,
  cacheEffectiveTheme,
  CORE_LIGHT_THEME_ID,
  DARK_THEME_ID,
  readSelectedTheme,
  rememberSelectedTheme,
} from "./appearance-preference";

describe("appearance preference", () => {
  afterEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.style.colorScheme = "";
  });

  it("falls back when preference data is malformed", () => {
    window.localStorage.setItem("uprava.appearance.v1", "not-json");

    expect(readSelectedTheme()).toBe(CORE_LIGHT_THEME_ID);
  });

  it("stores only the versioned selected theme id", () => {
    rememberSelectedTheme(DARK_THEME_ID);

    expect(readSelectedTheme()).toBe(DARK_THEME_ID);
    expect(
      JSON.parse(localStorage.getItem("uprava.appearance.v1") ?? "{}"),
    ).toEqual({
      version: 1,
      selected_theme_id: DARK_THEME_ID,
    });
  });

  it("applies only an allowlisted cached bundled marker", () => {
    cacheEffectiveTheme(DARK_THEME_ID);
    applyCachedThemeMarker();

    expect(document.documentElement.dataset.theme).toBe(DARK_THEME_ID);
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });

  it("rejects arbitrary cached theme ids", () => {
    window.localStorage.setItem(
      "uprava.theme-cache.v1",
      JSON.stringify({
        version: 1,
        effective_theme_id: "attacker.theme",
        palette_version: 1,
      }),
    );
    applyCachedThemeMarker();

    expect(document.documentElement.dataset.theme).toBe(CORE_LIGHT_THEME_ID);
  });
});
