import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Badge } from "./badge";
import { DisclosureControl, LoadingState } from "./system";

describe("Zarya UI primitives", () => {
  it.each([
    ["neutral", "neutral"],
    ["good", "normal"],
    ["warn", "review"],
    ["bad", "risk"],
    ["info", "notice"],
  ] as const)("renders the %s state with a text label", (tone, label) => {
    render(<Badge tone={tone}>{label}</Badge>);
    expect(screen.getByText(label)).toBeVisible();
  });

  it("exposes disclosure depth and invokes its action", () => {
    const onClick = vi.fn();
    render(
      <DisclosureControl expanded={false} label="evidence" onClick={onClick} />,
    );
    const control = screen.getByRole("button", { name: "Expand evidence" });
    expect(control).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(control);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("announces the current loading stage", () => {
    render(<LoadingState stage="Loading workspace" />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading workspace…");
  });
});
