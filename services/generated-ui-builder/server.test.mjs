import assert from "node:assert/strict";
import test from "node:test";

import { buildArtifact } from "./server.mjs";

const BASE_REQUEST = {
  runtime_id: "uprava.generated-react",
  runtime_version: "1.0.0",
  sdk_version: "1.0.0",
  allowed_imports: ["react", "react/jsx-runtime", "@uprava/ui-sdk"],
  max_bundle_bytes: 1024 * 1024,
};

test("buildArtifact bundles a Generated React entrypoint", async () => {
  const result = await buildArtifact({
    ...BASE_REQUEST,
    source: `
      import React from "react";
      import { Card, Heading } from "@uprava/ui-sdk";
      export default function App() {
        return <Card><Heading>Safe artifact</Heading></Card>;
      }
    `,
  });

  assert.match(result.bundle, /Safe artifact/);
  assert.equal(result.dependency_lock.runtime_id, "uprava.generated-react");
});

test("buildArtifact rejects imports outside the allowlist", async () => {
  await assert.rejects(
    () => buildArtifact({
      ...BASE_REQUEST,
      source: `import value from "untrusted-package"; export default () => value;`,
    }),
    /not allowed/,
  );
});

test("buildArtifact rejects dynamic execution primitives", async () => {
  await assert.rejects(
    () => buildArtifact({
      ...BASE_REQUEST,
      source: `export default function App() { return eval("1 + 1"); }`,
    }),
    /unsupported eval/,
  );
});

test("buildArtifact rejects a Core allowlist mismatch", async () => {
  await assert.rejects(
    () =>
      buildArtifact({
        ...BASE_REQUEST,
        allowed_imports: ["react", "react/jsx-runtime"],
        source: `export default function App() { return null; }`,
      }),
    /allowlists do not match/,
  );
});
