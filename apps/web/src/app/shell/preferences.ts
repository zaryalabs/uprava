const APP_SHELL_PREFERENCES_KEY = "uprava.app-shell.v1";
const APP_SHELL_PREFERENCES_VERSION = 1;

type AppShellPreferences = {
  version: typeof APP_SHELL_PREFERENCES_VERSION;
  sidebarOpen: boolean;
};

const defaultPreferences = (): AppShellPreferences => ({
  version: APP_SHELL_PREFERENCES_VERSION,
  sidebarOpen: true,
});

export function preferredSidebarOpen() {
  if (typeof window === "undefined") return true;
  try {
    const value = JSON.parse(
      window.localStorage.getItem(APP_SHELL_PREFERENCES_KEY) ?? "null",
    ) as unknown;
    if (
      !isRecord(value) ||
      value.version !== APP_SHELL_PREFERENCES_VERSION ||
      typeof value.sidebarOpen !== "boolean"
    ) {
      return true;
    }
    return value.sidebarOpen;
  } catch {
    return true;
  }
}

export function rememberSidebarOpen(sidebarOpen: boolean) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      APP_SHELL_PREFERENCES_KEY,
      JSON.stringify({
        ...defaultPreferences(),
        sidebarOpen,
      }),
    );
  } catch {
    // Navigation remains usable when storage is unavailable or full.
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
