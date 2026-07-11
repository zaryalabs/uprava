import { afterEach, describe, expect, it, vi } from "vitest";

import { logClientEvent } from "./client-logger";

describe("client logging authentication", () => {
  afterEach(() => {
    document.cookie = "uprava_csrf=; Path=/; Max-Age=0";
    vi.unstubAllGlobals();
  });

  it("does not send logs before an authenticated CSRF session exists", () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);

    logClientEvent("warn", "web.api", "Request failed");

    expect(fetch).not.toHaveBeenCalled();
  });

  it("sends the CSRF cookie value as the request header", () => {
    document.cookie = "uprava_csrf=csrf-token; Path=/";
    const fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetch);

    logClientEvent("warn", "web.api", "Request failed");

    expect(fetch).toHaveBeenCalledOnce();
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining("/client/logs"),
      expect.objectContaining({
        headers: {
          "content-type": "application/json",
          "x-uprava-csrf": "csrf-token",
        },
      }),
    );
  });
});
