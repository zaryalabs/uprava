import { describe, expect, it } from "vitest";

import { sessionAttention } from "./session-attention";

describe("sessionAttention", () => {
  it("marks a running runtime without recent events as stalled", () => {
    expect(
      sessionAttention(
        "active",
        "running",
        null,
        "2026-07-19T12:00:00Z",
        Date.parse("2026-07-19T12:00:31Z"),
      ),
    ).toBe("stalled");
  });

  it("keeps a recently active running runtime clear", () => {
    expect(
      sessionAttention(
        "active",
        "running",
        null,
        "2026-07-19T12:00:00Z",
        Date.parse("2026-07-19T12:00:10Z"),
      ),
    ).toBe("clear");
  });
});
