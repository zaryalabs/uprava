import { afterEach, describe, expect, it } from "vitest";

import { preferredSidebarOpen, rememberSidebarOpen } from "./preferences";

describe("app shell preferences", () => {
  afterEach(() => window.localStorage.clear());

  it("defaults to an open sidebar and remembers an explicit choice", () => {
    expect(preferredSidebarOpen()).toBe(true);

    rememberSidebarOpen(false);

    expect(preferredSidebarOpen()).toBe(false);
  });

  it("ignores invalid or unknown preference payloads", () => {
    window.localStorage.setItem("uprava.app-shell.v1", "not-json");
    expect(preferredSidebarOpen()).toBe(true);

    window.localStorage.setItem(
      "uprava.app-shell.v1",
      JSON.stringify({ version: 2, sidebarOpen: false }),
    );
    expect(preferredSidebarOpen()).toBe(true);
  });
});
