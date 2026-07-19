import { readFileSync, readdirSync } from "node:fs";
import { extname, join, relative } from "node:path";

const root = new URL("../src/", import.meta.url);
const allowed = new Set(["styles.css"]);
const forbidden = [
  { label: "raw color", pattern: /#[\da-f]{3,8}/i },
  { label: "radius utility", pattern: /\brounded(?:-[\w[\].:/-]+)?\b/ },
  { label: "shadow utility", pattern: /\bshadow(?:-[\w[\].:/-]+)?\b/ },
];
const failures = [];

function walk(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      walk(path);
      continue;
    }
    if (![".css", ".tsx"].includes(extname(path))) continue;
    const name = relative(root.pathname, path);
    if (allowed.has(name)) continue;
    const lines = readFileSync(path, "utf8").split("\n");
    lines.forEach((line, index) => {
      for (const rule of forbidden) {
        if (rule.pattern.test(line)) {
          failures.push(`${name}:${index + 1} ${rule.label}`);
        }
      }
    });
  }
}

walk(root.pathname);
if (failures.length > 0) {
  console.error(`Zarya design-system check failed:\n${failures.join("\n")}`);
  process.exit(1);
}
console.log("Zarya design-system check passed");
