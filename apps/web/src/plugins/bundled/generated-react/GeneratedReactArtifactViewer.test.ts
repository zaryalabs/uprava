import { afterEach, describe, expect, it, vi } from "vitest";

import { buildSandboxDocument } from "./GeneratedReactArtifactViewer";

afterEach(() => vi.unstubAllGlobals());

describe("Generated React sandbox document", () => {
  it("uses a script-only opaque iframe CSP and escapes script termination", () => {
    vi.stubGlobal("crypto", { randomUUID: () => "nonce-value" });
    const document = buildSandboxDocument(
      `console.log("</script><script>escape()</script>")`,
      {},
    );

    expect(document).toContain("default-src 'none'");
    expect(document).toContain("connect-src 'none'");
    expect(document).toContain("form-action 'none'");
    expect(document).not.toContain(`</script><script>escape()`);
    expect(document).toContain("<\\/script>");
  });
});
