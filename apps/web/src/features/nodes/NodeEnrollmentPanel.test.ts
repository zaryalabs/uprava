import { describe, expect, it } from "vitest";

import { isEnrollmentApprovable } from "./NodeEnrollmentPanel";
import type { NodeEnrollmentSummary } from "../../shared/protocol/types";

describe("isEnrollmentApprovable", () => {
  it("allows only unapproved pending enrollment requests", () => {
    expect(isEnrollmentApprovable(enrollment())).toBe(true);
    expect(
      isEnrollmentApprovable(
        enrollment({
          approved_at: "2026-06-17T00:00:00Z",
        }),
      ),
    ).toBe(false);
    expect(
      isEnrollmentApprovable(
        enrollment({
          status: "registered",
          claimed_node_id: "node-1",
        }),
      ),
    ).toBe(false);
  });
});

function enrollment(
  overrides: Partial<NodeEnrollmentSummary> = {},
): NodeEnrollmentSummary {
  return {
    enrollment_id: "enrollment-1",
    display_name: "Local Node",
    status: "pending_user_approval",
    claimed_node_id: null,
    expires_at: "2026-06-17T00:10:00Z",
    created_at: "2026-06-17T00:00:00Z",
    approved_at: null,
    ...overrides,
  };
}
