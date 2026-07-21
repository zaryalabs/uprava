import { describe, expect, it } from "vitest";

import { colorLiteralKind } from "../../source-matchers";

describe("colorLiteralKind", () => {
  it.each([
    ["#fff", "hex"],
    ["#11aa22cc", "hex"],
    ["rgb(10, 20, 30)", "rgb"],
    ["rgba(10, 20, 30, 0.5)", "rgb"],
    ["hsl(120, 50%, 25%)", "hsl"],
  ])("recognizes strict literal %s", (source, kind) => {
    expect(colorLiteralKind(source)).toBe(kind);
  });

  it.each(["red", "var(--color)", "#12", "rgb(1, 2)", "url(javascript:x)"])(
    "rejects non-literal color %s",
    (source) => {
      expect(colorLiteralKind(source)).toBeNull();
    },
  );
});
