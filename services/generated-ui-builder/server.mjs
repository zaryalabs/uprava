import { createServer } from "node:http";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

import { SDK_SOURCE } from "./sdk-source.mjs";

const serviceDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(serviceDirectory, "../..");
const webRequire = createRequire(resolve(repositoryRoot, "apps/web/package.json"));
const { build } = webRequire("esbuild");

const MAX_REQUEST_BYTES = 256 * 1024;
const SERVER_ALLOWED_IMPORTS = new Set([
  "react",
  "react/jsx-runtime",
  "@uprava/ui-sdk",
]);
const FORBIDDEN_SOURCE_PATTERNS = [
  ["dynamic import", /\bimport\s*\(/],
  ["eval", /\beval\s*\(/],
  ["Function constructor", /\bnew\s+Function\b/],
  ["WebSocket", /\bWebSocket\s*\(/],
  ["Worker", /\b(?:Shared)?Worker\s*\(/],
];

const ENTRY_SOURCE = String.raw`
import React from "react";
import { createRoot } from "react-dom/client";
import App from "uprava:artifact";
import { __initializeUpravaUi } from "@uprava/ui-sdk";

let initialized = false;
let bridgePort = null;
function reportRuntimeError(value) {
  const message = value instanceof Error ? value.message : String(value || "Generated UI runtime failed");
  bridgePort?.postMessage({ protocol: 1, type: "ui.error", message: message.slice(0, 2000) });
}
window.addEventListener("error", (event) => reportRuntimeError(event.error || event.message));
window.addEventListener("unhandledrejection", (event) => reportRuntimeError(event.reason));
window.addEventListener("message", (event) => {
  const message = event.data;
  if (initialized || !message || message.type !== "uprava.ui.init" || message.protocol !== 1) return;
  const [port] = event.ports;
  if (!port) return;
  initialized = true;
  bridgePort = port;
  __initializeUpravaUi(port, message.payload);
  const root = document.getElementById("uprava-generated-root");
  if (!root) throw new Error("Generated UI root is missing");
  createRoot(root).render(React.createElement(App));
  port.postMessage({ protocol: 1, type: "ui.ready" });
}, { once: true });
`;

export async function buildArtifact(request) {
  validateBuildRequest(request);
  for (const [label, pattern] of FORBIDDEN_SOURCE_PATTERNS) {
    if (pattern.test(request.source)) throw new Error(`Source uses unsupported ${label}`);
  }
  const diagnostics = [];
  const artifactPlugin = {
    name: "uprava-generated-ui-boundary",
    setup(buildApi) {
      buildApi.onResolve({ filter: /^uprava:artifact$/ }, () => ({
        path: "uprava:artifact",
        namespace: "uprava-artifact",
      }));
      buildApi.onLoad({ filter: /.*/, namespace: "uprava-artifact" }, () => ({
        contents: request.source,
        loader: "tsx",
        resolveDir: resolve(repositoryRoot, "apps/web"),
      }));
      buildApi.onResolve({ filter: /^@uprava\/ui-sdk$/ }, () => ({
        path: "@uprava/ui-sdk",
        namespace: "uprava-sdk",
      }));
      buildApi.onLoad({ filter: /.*/, namespace: "uprava-sdk" }, () => ({
        contents: SDK_SOURCE,
        loader: "tsx",
        resolveDir: resolve(repositoryRoot, "apps/web"),
      }));
      buildApi.onResolve({ filter: /.*/, namespace: "uprava-artifact" }, (args) => {
        if (args.kind === "dynamic-import") {
          return { errors: [{ text: "Dynamic imports are not allowed" }] };
        }
        if (!SERVER_ALLOWED_IMPORTS.has(args.path)) {
          return { errors: [{ text: `Import ${JSON.stringify(args.path)} is not allowed` }] };
        }
        return null;
      });
    },
  };
  let output;
  try {
    output = await build({
      absWorkingDir: resolve(repositoryRoot, "apps/web"),
      bundle: true,
      format: "iife",
      platform: "browser",
      target: ["es2022"],
      jsx: "automatic",
      stdin: {
        contents: ENTRY_SOURCE,
        loader: "tsx",
        sourcefile: "uprava-generated-entry.tsx",
        resolveDir: resolve(repositoryRoot, "apps/web"),
      },
      plugins: [artifactPlugin],
      write: false,
      logLevel: "silent",
      legalComments: "none",
      minify: true,
      define: { "process.env.NODE_ENV": '"production"' },
      sourcemap: false,
      metafile: false,
    });
  } catch (error) {
    const messages = Array.isArray(error.errors) ? error.errors : [];
    const detail = messages.map((message) => message.text).join("; ") || error.message;
    throw new Error(detail);
  }
  const bundle = output.outputFiles[0]?.text;
  if (!bundle) throw new Error("Builder produced no JavaScript bundle");
  if (Buffer.byteLength(bundle) > request.max_bundle_bytes) {
    throw new Error("Builder output exceeds max_bundle_bytes");
  }
  return {
    bundle,
    dependency_lock: {
      runtime_id: request.runtime_id,
      runtime_version: request.runtime_version,
      sdk_version: request.sdk_version,
      react: webRequire("react/package.json").version,
      esbuild: webRequire("esbuild/package.json").version,
      imports: [...SERVER_ALLOWED_IMPORTS].sort(),
    },
    diagnostics,
  };
}

function validateBuildRequest(request) {
  if (!request || typeof request !== "object") throw new Error("Build request must be an object");
  for (const field of ["source", "runtime_id", "runtime_version", "sdk_version"]) {
    if (typeof request[field] !== "string" || request[field].length === 0) {
      throw new Error(`${field} must be a non-empty string`);
    }
  }
  if (!Array.isArray(request.allowed_imports)) throw new Error("allowed_imports must be an array");
  const requestedImports = new Set(request.allowed_imports);
  if (
    requestedImports.size !== SERVER_ALLOWED_IMPORTS.size ||
    [...SERVER_ALLOWED_IMPORTS].some((name) => !requestedImports.has(name))
  ) {
    throw new Error("Core and builder import allowlists do not match");
  }
  if (!Number.isSafeInteger(request.max_bundle_bytes) || request.max_bundle_bytes <= 0 || request.max_bundle_bytes > 4 * 1024 * 1024) {
    throw new Error("max_bundle_bytes is invalid");
  }
  if (Buffer.byteLength(request.source) > 128 * 1024) throw new Error("Source exceeds the builder limit");
}

function jsonResponse(response, status, value) {
  const body = JSON.stringify(value);
  response.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(body),
    "cache-control": "no-store",
  });
  response.end(body);
}

async function readJson(request) {
  let size = 0;
  const chunks = [];
  for await (const chunk of request) {
    size += chunk.length;
    if (size > MAX_REQUEST_BYTES) throw new Error("Request body exceeds the builder limit");
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

export function createBuilderServer() {
  return createServer(async (request, response) => {
    if (request.method === "GET" && request.url === "/health") {
      return jsonResponse(response, 200, { status: "ok" });
    }
    if (request.method !== "POST" || request.url !== "/build") {
      return jsonResponse(response, 404, { error: "not_found" });
    }
    if (!String(request.headers["content-type"] || "").startsWith("application/json")) {
      return jsonResponse(response, 415, { error: "content_type_required" });
    }
    try {
      const result = await buildArtifact(await readJson(request));
      return jsonResponse(response, 200, result);
    } catch (error) {
      return jsonResponse(response, 400, { error: "build_failed", message: String(error.message || error) });
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const port = Number.parseInt(process.env.UPRAVA_GENERATED_UI_BUILDER_PORT || "18082", 10);
  createBuilderServer().listen(port, "0.0.0.0");
}
