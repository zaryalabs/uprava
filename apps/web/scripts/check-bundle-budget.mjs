import { gzipSync } from "node:zlib";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";

const dist = new URL("../dist/", import.meta.url);
const manifest = JSON.parse(
  await readFile(new URL(".vite/manifest.json", dist), "utf8"),
);
const entry = Object.values(manifest).find((chunk) => chunk.isEntry);

if (!entry) {
  throw new Error("Vite manifest has no application entry");
}

const initialFiles = collectInitialFiles(entry, manifest);
const forbidden = /(?:monaco|xterm|editor\.worker|languageFeatures)/i;
const forbiddenFiles = [...initialFiles].filter((file) => forbidden.test(file));
if (forbiddenFiles.length > 0) {
  throw new Error(
    `Heavy editor/terminal runtime leaked into initial graph: ${forbiddenFiles.join(", ")}`,
  );
}

const dynamicEntries = (entry.dynamicImports ?? [])
  .map((name) => manifest[name])
  .filter(Boolean);
let markdownRenderer;
for (const chunk of dynamicEntries) {
  const contents = await readFile(new URL(chunk.file, dist), "utf8");
  if (contents.includes("uprava-markdown")) {
    markdownRenderer = chunk;
    break;
  }
}
if (!markdownRenderer?.isDynamicEntry) {
  throw new Error("Markdown renderer was not emitted as an on-demand chunk");
}
if (initialFiles.has(markdownRenderer.file)) {
  throw new Error(
    "Markdown renderer leaked into the initial application graph",
  );
}

let gzipBytes = 0;
for (const file of initialFiles) {
  const contents = await readFile(new URL(file, dist));
  gzipBytes += gzipSync(contents).byteLength;
}

const maxGzipBytes = 350 * 1024;
if (gzipBytes > maxGzipBytes) {
  throw new Error(
    `Initial graph is ${formatKiB(gzipBytes)} gzip; budget is ${formatKiB(maxGzipBytes)}`,
  );
}

const assets = await readdir(new URL("assets/", dist));
const placementChunk = Object.values(manifest).find(
  (chunk) => chunk.name === "PlacementRoute",
);
if (
  !placementChunk ||
  !assets.includes(placementChunk.file.replace("assets/", ""))
) {
  throw new Error("Workspace route was not emitted as a lazy chunk");
}

const dynamicNames = (placementChunk.dynamicImports ?? [])
  .map((name) => manifest[name]?.name)
  .filter(Boolean);
for (const required of ["MonacoViews", "XtermTerminal"]) {
  if (!dynamicNames.includes(required)) {
    throw new Error(`${required} is not a workspace on-demand chunk`);
  }
}
const placementStaticFiles = collectInitialFiles(placementChunk, manifest);
const leakedWorkspaceFiles = [...placementStaticFiles].filter((file) =>
  forbidden.test(file),
);
if (leakedWorkspaceFiles.length > 0) {
  throw new Error(
    `Heavy runtime is statically loaded by workspace route: ${leakedWorkspaceFiles.join(", ")}`,
  );
}

console.log(
  `Bundle budget passed: ${formatKiB(gzipBytes)} gzip across ${initialFiles.size} initial files`,
);

function collectInitialFiles(root, chunks) {
  const files = new Set();
  const pending = [root];
  const visited = new Set();
  while (pending.length > 0) {
    const chunk = pending.pop();
    if (!chunk || visited.has(chunk.file)) continue;
    visited.add(chunk.file);
    files.add(chunk.file);
    for (const css of chunk.css ?? []) files.add(css);
    for (const asset of chunk.assets ?? []) files.add(asset);
    for (const importedName of chunk.imports ?? []) {
      pending.push(chunks[importedName]);
    }
  }
  return files;
}

function formatKiB(bytes) {
  return `${(bytes / 1024).toFixed(1)} KiB`;
}
