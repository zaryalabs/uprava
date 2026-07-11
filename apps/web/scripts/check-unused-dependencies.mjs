import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const files = [];
for (const relative of ["src", "e2e", "scripts"]) collect(join(root, relative));
for (const relative of [
  "vite.config.ts",
  "playwright.config.ts",
  "package.json",
]) {
  files.push(join(root, relative));
}
const source = files.map((file) => readFileSync(file, "utf8")).join("\n");
const unused = Object.keys(manifest.dependencies).filter(
  (dependency) => !source.includes(dependency),
);
if (unused.length > 0) {
  throw new Error(`Unused direct Web dependencies: ${unused.join(", ")}`);
}
console.log(
  `Direct Web dependency check passed (${Object.keys(manifest.dependencies).length} dependencies)`,
);

function collect(directory) {
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) collect(path);
    else if (/\.(?:css|js|mjs|ts|tsx)$/.test(entry)) files.push(path);
  }
}
