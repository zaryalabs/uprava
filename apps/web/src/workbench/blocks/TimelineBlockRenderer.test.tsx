import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Button } from "../../shared/ui/button";
import { TimelineBlockRenderer } from "./TimelineBlockRenderer";
import type { UiBlock } from "./types";

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

  it("renders running turn activity expanded and supports manual collapse", () => {
    render(
      <TimelineBlockRenderer block={turnActivityBlock({ completed: false })} />,
    );

    expect(screen.getByText("Turn Activity")).toBeVisible();
    expect(screen.getByText("make c")).toBeVisible();
    expect(screen.getByText("item.completed")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Collapse activity" }));

    expect(screen.queryByText("make c")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Expand activity" }),
    ).toBeVisible();
  });

  it("renders completed turn activity collapsed by default", () => {
    render(
      <TimelineBlockRenderer block={turnActivityBlock({ completed: true })} />,
    );

    expect(screen.getByText("Turn Activity")).toBeVisible();
    expect(screen.queryByText("make c")).not.toBeInTheDocument();
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

function turnActivityBlock({ completed }: { completed: boolean }): UiBlock {
  return {
    ...baseBlock,
    block_id: "turn-activity:turn-1",
    type: "core.turn-activity",
    primary_ref: { kind: "turn", turn_id: "turn-1" },
    data: {
      turnId: "turn-1",
      completed,
      durationMs: completed ? 1500 : null,
      commandCount: 1,
      fileChangeCount: 0,
      reasoningCount: 0,
      warningErrorCount: 0,
      rows: [
        {
          eventId: "event-activity-1",
          seq: 3,
          happenedAt: "2026-06-17T00:00:02Z",
          providerEventType: "item.completed",
          providerItemType: "command_execution",
          status: "completed",
          phase: "completed",
          summary: "make c",
          rawEvent: { type: "item.completed" },
        },
      ],
    },
    fallback_text: "1 provider activity event",
  };
}
