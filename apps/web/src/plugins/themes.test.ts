import { afterEach, describe, expect, it } from "vitest";

import { applyTheme, CORE_LIGHT_THEME, normalizeTheme } from "./themes";

describe("theme contracts", () => {
  afterEach(() => {
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("style");
  });

  it("accepts the complete Core Light theme", () => {
    expect(normalizeTheme(CORE_LIGHT_THEME)).toEqual(CORE_LIGHT_THEME);
  });

  it("isolates themes with unknown color expressions", () => {
    const unsafe = {
      ...CORE_LIGHT_THEME,
      theme_id: "vendor.unsafe",
      semantic_tokens: {
        ...CORE_LIGHT_THEME.semantic_tokens,
        "surface.background": "url(https://attacker.test/pixel)",
      },
    };

    expect(normalizeTheme(unsafe)).toBeNull();
  });

  it("isolates oversized adapter palettes", () => {
    const oversized = {
      ...CORE_LIGHT_THEME,
      theme_id: "vendor.oversized",
      monaco: {
        ...CORE_LIGHT_THEME.monaco,
        colors: Object.fromEntries(
          Array.from({ length: 129 }, (_, index) => [`color.${index}`, "#fff"]),
        ),
      },
    };

    expect(normalizeTheme(oversized)).toBeNull();
  });

  it("applies allowlisted tokens to the controlled root", () => {
    applyTheme(CORE_LIGHT_THEME);

    expect(document.documentElement.dataset.theme).toBe("core.light");
    expect(document.documentElement.style.getPropertyValue("--color-bg")).toBe(
      "#ffffff",
    );
    expect(document.documentElement.style.colorScheme).toBe("light");
  });
});
