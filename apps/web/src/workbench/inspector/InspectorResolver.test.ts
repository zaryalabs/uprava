import { describe, expect, it } from "vitest";

import {
  buildInspectorDetail,
  buildResolvedInspectorDetail,
} from "./InspectorStack";

describe("inspector reference resolution", () => {
  it("resolves reserved file references without rendering or API setup", () => {
    const detail = buildInspectorDetail(
      { kind: "file", placement_id: "placement-1", path: "src/main.rs" },
      {},
    );

    expect(detail.status).toBe("not_implemented");
    expect(detail.rows).toContainEqual({ label: "kind", value: "file" });
  });

  it("reports missing inventory entities deterministically", () => {
    const detail = buildInspectorDetail(
      { kind: "node", node_id: "node-1" },
      {},
    );

    expect(detail.status).toBe("not_available");
    expect(detail.rows).toContainEqual({
      label: "reason",
      value: "Node snapshot is not loaded",
    });
  });

  it("keeps server-resolved causality aspects separate", () => {
    const detail = buildResolvedInspectorDetail({
      reference: { kind: "event", event_id: "event-1", scope_ref: {}, seq: 1 },
      status: "resolved",
      title: "provider.activity",
      summary: "Read the source file",
      source_refs: [
        { kind: "file", placement_id: "placement-1", path: "src/main.rs" },
      ],
      evidence_refs: [
        { kind: "terminal_command", terminal_command_id: "command-1" },
      ],
      cause_refs: [{ kind: "command", command_id: "command-1" }],
      result_refs: [
        {
          kind: "workspace_diff",
          placement_id: "placement-1",
          diff_id: "diff-1",
        },
      ],
      raw_refs: [{ kind: "event", event_id: "event-1", scope_ref: {}, seq: 1 }],
      raw_payload: { type: "provider_activity" },
      raw_truncated: false,
      unavailable_reason: null,
    });

    expect(detail.status).toBe("resolved");
    expect(detail.refs.map((item) => item.aspect)).toEqual([
      "source",
      "evidence",
      "cause",
      "result",
      "raw",
    ]);
  });
});
