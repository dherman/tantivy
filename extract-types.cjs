// Runs after `cargo build && neon dist && tsc`. Locates the platform-specific
// addon that `neon dist` just produced, then regenerates src/generated.d.cts
// (used by the next tsc compile) and overwrites lib/load.d.cts with the types
// as direct exports so consumers see them natively on the addon module.
const fs = require("fs");
const path = require("path");

// `neon dist` stages the binary at ./platforms/<platform>/index.node when
// NEON_BUILD_PLATFORM is set (the CI path), and at ./index.node otherwise
// (the local debug path).
const platform = process.env.NEON_BUILD_PLATFORM;
const addonPath = platform
  ? path.join(__dirname, "platforms", platform, "index.node")
  : path.join(__dirname, "index.node");
const addon = require(addonPath);

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

// src/generated.d.cts: a module augmentation so the next tsc (compiling src/)
// sees the types on ./load.cjs. Checked into git so fresh clones can run tsc
// without a prior build.
const indented = cleaned
  .split("\n")
  .map((line) => (line.trim() ? "  " + line : line))
  .join("\n");
const srcContent = `declare module "./load.cjs" {\n${indented}\n}\n\nexport {};\n`;

const srcPath = path.join(__dirname, "src", "generated.d.cts");
fs.writeFileSync(srcPath, srcContent);
console.log(`Wrote ${srcPath}`);

// lib/load.d.cts: overwrite the `export {};` tsc emitted with the same types
// as direct top-level exports so consumers see them natively.
const libLoad = path.join(__dirname, "lib", "load.d.cts");
fs.writeFileSync(libLoad, cleaned);
console.log(`Wrote ${libLoad}`);

// Clean up any stale generated.d.cts from a previous pipeline version.
const staleGenerated = path.join(__dirname, "lib", "generated.d.cts");
if (fs.existsSync(staleGenerated)) {
  fs.unlinkSync(staleGenerated);
}
