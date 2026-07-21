import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { MarkdownRenderer } from "./MarkdownRenderer";

const sourceRef = { kind: "message", message_id: "message-1" } as const;

describe("MarkdownRenderer", () => {
  it("renders GitHub-flavored Markdown structures", () => {
    render(
      <MarkdownRenderer
        content={[
          "## Result",
          "",
          "- first",
          "- second",
          "",
          "| Name | State |",
          "| --- | --- |",
          "| Core | Ready |",
        ].join("\n")}
        sourceRef={sourceRef}
        state="complete"
      />,
    );

    expect(screen.getByRole("heading", { name: "Result" })).toBeVisible();
    expect(screen.getByRole("list")).toBeVisible();
    expect(screen.getByRole("table")).toBeVisible();
  });

  it("keeps incomplete streaming Markdown readable", () => {
    render(
      <MarkdownRenderer
        content="**partial response"
        sourceRef={sourceRef}
        state="streaming"
      />,
    );

    expect(screen.getByText("partial response")).toBeVisible();
  });

  it("drops raw HTML and remote images", () => {
    render(
      <MarkdownRenderer
        content={
          '<script>alert("xss")</script>\n\n![track](https://evil.test/pixel.png)'
        }
        sourceRef={sourceRef}
        state="complete"
      />,
    );

    expect(document.querySelector("script")).not.toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("allows web links and removes unsafe link targets", () => {
    render(
      <MarkdownRenderer
        content="[Docs](https://example.com) [Unsafe](javascript:alert(1))"
        sourceRef={sourceRef}
        state="complete"
      />,
    );

    expect(screen.getByRole("link", { name: "Docs" })).toHaveAttribute(
      "href",
      "https://example.com/",
    );
    expect(screen.getByText(/Unsafe/).closest("a")).toBeNull();
  });
});
