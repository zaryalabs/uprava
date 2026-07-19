import type { ThemeContributionV1 } from "../shared/protocol/types";
import { CORE_LIGHT_THEME_ID } from "./appearance-preference";

export const THEME_CONTRACT_VERSION = 1;

export const CORE_LIGHT_THEME: ThemeContributionV1 = {
  theme_id: CORE_LIGHT_THEME_ID,
  label: "Light",
  kind: "light",
  color_scheme: "light",
  semantic_tokens: {
    "surface.background": "#ffffff",
    "surface.muted": "#f3f3ef",
    "surface.raised": "#ffffff",
    "content.primary": "#0b0b0b",
    "content.muted": "#5f5f58",
    "content.inverse": "#ffffff",
    "border.default": "#e5e5df",
    "border.strong": "#c7c7bf",
    "status.risk": "#c7282f",
    "status.notice": "#6b4bb2",
    focus: "#0b0b0b",
    selection: "#d9e4dc",
    "editor.background": "#ffffff",
    "editor.foreground": "#0b0b0b",
    "terminal.background": "#111812",
    "terminal.foreground": "#dce8dd",
  },
  monaco: {
    base: "vs",
    colors: {
      "editor.background": "#ffffff",
      "editor.foreground": "#0b0b0b",
      "editorLineNumber.foreground": "#77776f",
      "editor.selectionBackground": "#d9e4dc",
      "editor.inactiveSelectionBackground": "#e8eee9",
    },
  },
  terminal: {
    colors: {
      background: "#111812",
      foreground: "#dce8dd",
      cursor: "#f4f7f2",
      selectionBackground: "#355343",
      black: "#171a16",
      red: "#ff777d",
      green: "#8ecf88",
      yellow: "#e1c56b",
      blue: "#8eb8ff",
      magenta: "#c7a0ff",
      cyan: "#79c8c3",
      white: "#dce8dd",
    },
  },
};

const TOKEN_TO_CSS_VARIABLE: Record<string, string> = {
  "surface.background": "--color-bg",
  "surface.muted": "--color-bg-muted",
  "surface.raised": "--color-bg-raised",
  "content.primary": "--color-ink",
  "content.muted": "--color-muted",
  "content.inverse": "--color-inverse",
  "border.default": "--color-border",
  "border.strong": "--color-border-strong",
  "status.risk": "--color-risk",
  "status.notice": "--color-notice",
  focus: "--color-focus",
  selection: "--color-selection",
  "editor.background": "--color-editor-bg",
  "editor.foreground": "--color-editor-ink",
  "terminal.background": "--color-terminal-bg",
  "terminal.foreground": "--color-terminal-ink",
};

const REQUIRED_TOKENS = Object.keys(TOKEN_TO_CSS_VARIABLE);
const HEX_COLOR = /^#[\da-f]{3}(?:[\da-f]{3}|[\da-f]{5})?$/i;
const NAMESPACED_ID = /^[a-z0-9][a-z0-9._-]*\.[a-z0-9._-]+$/;
const MAX_THEME_ID_CHARS = 128;
const MAX_THEME_LABEL_CHARS = 2_000;
const MAX_THEME_COLORS = 128;

export function normalizeTheme(
  theme: ThemeContributionV1,
): ThemeContributionV1 | null {
  if (
    theme.theme_id.length > MAX_THEME_ID_CHARS ||
    !NAMESPACED_ID.test(theme.theme_id) ||
    theme.label.length === 0 ||
    theme.label.length > MAX_THEME_LABEL_CHARS ||
    !["vs", "vs-dark", "hc-black"].includes(theme.monaco.base) ||
    Object.keys(theme.semantic_tokens).length > MAX_THEME_COLORS ||
    Object.keys(theme.monaco.colors).length > MAX_THEME_COLORS ||
    Object.keys(theme.terminal.colors).length > MAX_THEME_COLORS ||
    REQUIRED_TOKENS.some((token) => !theme.semantic_tokens[token])
  ) {
    return null;
  }
  const colors = [
    ...Object.values(theme.semantic_tokens),
    ...Object.values(theme.monaco.colors),
    ...Object.values(theme.terminal.colors),
  ];
  return colors.every((color) => HEX_COLOR.test(color)) ? theme : null;
}

export function applyTheme(theme: ThemeContributionV1) {
  const root = document.documentElement;
  root.dataset.theme = theme.theme_id;
  root.style.colorScheme = theme.color_scheme;
  for (const [token, cssVariable] of Object.entries(TOKEN_TO_CSS_VARIABLE)) {
    root.style.setProperty(cssVariable, theme.semantic_tokens[token]);
  }
}
