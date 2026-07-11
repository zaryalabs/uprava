import { describe, expect, it } from "vitest";

import { buildInspectorDetail } from "./InspectorStack";

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
});
