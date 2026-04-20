const addon = require("./index.node");
const fs = require("fs");
const path = require("path");

const raw = addon.generateTypescriptDeclarations();

// Strip the `generateTypescriptDeclarations` function — it's an internal
// helper, not part of the public API.
let cleaned = raw.replace(
  /^export declare function generateTypescriptDeclarations\(\):.*$\n?/gm,
  ""
);

// Strip class declarations — they're hand-written in src/index.cts so we can
// override return types neon can't yet infer (BigInt, SearchResult[], etc.).
cleaned = cleaned.replace(
  /^export declare class \w+ \{[\s\S]*?^\}\n?/gm,
  ""
);

// Neon emits top-level `interface` and `type` without `export`. Prepend
// `export` so consumers can import them.
cleaned = cleaned.replace(/^(interface |type )/gm, "export $1");

// src/generated.d.cts: a module augmentation so tsc (compiling src/) sees the
// types on ./load.cjs. Nested inside `declare module`, then made a module via
// `export {}`.
const indented = cleaned
  .split("\n")
  .map((line) => (line.trim() ? "  " + line : line))
  .join("\n");
const srcContent = `declare module "./load.cjs" {\n${indented}\n}\n\nexport {};\n`;

const srcPath = path.join(__dirname, "src", "generated.d.cts");
fs.writeFileSync(srcPath, srcContent);
console.log(`Wrote ${srcPath}`);

// lib/load.d.cts: after tsc emits `export {};`, overwrite with the same types
// as direct top-level exports so consumers see them natively on the addon
// module. Skip if tsc hasn't run yet (this script runs once before tsc to
// prime src/, and again after tsc to update lib/).
const libLoad = path.join(__dirname, "lib", "load.d.cts");
if (fs.existsSync(libLoad)) {
  fs.writeFileSync(libLoad, cleaned);
  console.log(`Wrote ${libLoad}`);

  // Clean up any stale generated.d.cts from a previous pipeline version.
  const staleGenerated = path.join(__dirname, "lib", "generated.d.cts");
  if (fs.existsSync(staleGenerated)) {
    fs.unlinkSync(staleGenerated);
  }
}
