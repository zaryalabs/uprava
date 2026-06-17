import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { UiBlock } from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { TimelineBlockRenderer } from "./TimelineBlockRenderer";

describe("TimelineBlockRenderer", () => {
  it("renders unknown block fallback text and keeps supplied actions visible", () => {
    render(
      <TimelineBlockRenderer
        block={{
          ...baseBlock,
          type: "plugin.unregistered",
          fallback_text: "Safe fallback payload",
        }}
        actions={<Button>Inspect</Button>}
      />,
    );

    expect(screen.getByText("Unknown")).toBeVisible();
    expect(screen.getByText("Safe fallback payload")).toBeVisible();
    expect(screen.getByRole("button", { name: "Inspect" })).toBeVisible();
  });

  it("falls back when a known block has an unsupported schema version", () => {
    render(
      <TimelineBlockRenderer
        block={{
          ...baseBlock,
          type: "core.assistant-message",
          schema_version: 99,
          fallback_text: "Unsupported schema fallback",
        }}
      />,
    );

    expect(screen.getByText("Unknown")).toBeVisible();
    expect(screen.getByText("Unsupported schema fallback")).toBeVisible();
  });
});

const baseBlock: UiBlock = {
  block_id: "block-unknown",
  type: "core.unknown",
  schema_version: 1,
  surface_id: "session.timeline",
  primary_ref: { kind: "block", block_id: "block-unknown" },
  parent_ref: null,
  children: [],
  source_refs: [],
  evidence_refs: [],
  cause_refs: [],
  related_refs: [],
  trace_refs: [],
  data: { raw: true },
  actions: [],
  fallback_text: "Fallback",
};
