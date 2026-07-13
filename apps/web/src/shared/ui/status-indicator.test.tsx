import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StatusIndicator, statusPresentation } from "./status-indicator";

describe("statusPresentation", () => {
  it("maps protocol values without changing their dimensions", () => {
    expect(statusPresentation("presence", "reachable")).toMatchObject({
      label: "Reachable",
      tone: "good",
    });
    expect(statusPresentation("lifecycle", "blocked")).toMatchObject({
      label: "Blocked",
      tone: "neutral",
    });
    expect(statusPresentation("attention", "blocked")).toMatchObject({
      label: "Blocked",
      tone: "warn",
    });
    expect(statusPresentation("workspace", "read_only")).toMatchObject({
      label: "Read only",
      tone: "warn",
    });
  });

  it("renders lifecycle and attention as independent accessible states", () => {
    render(
      <div>
        <StatusIndicator showDimension dimension="lifecycle" value="active" />
        <StatusIndicator showDimension dimension="attention" value="blocked" />
      </div>,
    );

    expect(screen.getByText("Lifecycle: Active")).toBeVisible();
    expect(screen.getByText("Attention: Blocked")).toBeVisible();
  });
});
