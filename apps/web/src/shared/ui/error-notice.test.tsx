import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CortexApiError } from "../api/http-client";
import { ErrorNotice } from "./error-notice";

describe("ErrorNotice", () => {
  it("renders api error envelope diagnostics", () => {
    render(
      <ErrorNotice
        title="Workspace validation failed"
        error={
          new CortexApiError({
            error_code: "placement.invalid",
            message: "Workspace is not writable",
            retryable: false,
            correlation_id: "corr-1",
          })
        }
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Workspace validation failed",
    );
    expect(screen.getByText("Workspace is not writable")).toBeVisible();
    expect(screen.getByText("placement.invalid")).toBeVisible();
    expect(screen.getByText("corr-1")).toBeVisible();
  });
});
