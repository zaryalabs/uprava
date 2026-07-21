import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { ArtifactDetail } from "../../../shared/protocol/types";
import { TraceArtifactViewer } from "./TraceArtifactViewer";

describe("TraceArtifactViewer", () => {
  it("renders nested causality narrative results", () => {
    render(
      <TraceArtifactViewer
        detail={detail({
          result: {
            conclusion: "The check failed after the edit.",
            steps: [{ step_id: "step-1", summary: "Workspace changed" }],
          },
        })}
        fallback={<div>Raw fallback</div>}
      />,
    );

    expect(screen.getByText("The check failed after the edit.")).toBeVisible();
    expect(screen.getByText("Workspace changed")).toBeVisible();
    expect(screen.queryByText("Raw fallback")).not.toBeInTheDocument();
  });

  it("keeps the host fallback for malformed payloads", () => {
    render(
      <TraceArtifactViewer
        detail={detail("raw trace")}
        fallback={<div>Raw fallback</div>}
      />,
    );

    expect(screen.getByText("Raw fallback")).toBeVisible();
  });
});

function detail(payload: unknown): ArtifactDetail {
  return {
    artifact: {
      artifact_id: "artifact-1",
      artifact_type: "uprava.causality-narrative",
      title: "Causality narrative",
      scope_ref: { kind: "session", session_thread_id: "session-1" },
      owner_plugin_id: "uprava.trace-artifacts",
      current_version: 1,
      state: "active",
      created_by: { kind: "system" },
      created_at: "2026-07-21T00:00:00Z",
      updated_at: "2026-07-21T00:00:00Z",
    },
    version: {
      artifact_id: "artifact-1",
      version: 1,
      schema_version: 1,
      payload,
      fallback_text: "Raw fallback",
      source_version: null,
      source_refs: [],
      evidence_refs: [],
      cause_refs: [],
      trace_refs: [],
      provenance: {},
      created_at: "2026-07-21T00:00:00Z",
    },
  };
}
