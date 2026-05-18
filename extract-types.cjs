// Runs after `cargo build && neon dist && tsc`. Reads the .d.ts text that
// neon's `typescript` feature auto-attaches to the addon under
// Symbol.for("neon:types"), then regenerates src/generated.d.cts (used by
// the next tsc compile) and overwrites lib/load.d.cts with the same types
// as direct top-level exports so consumers see them natively on the addon.
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

const cleaned = addon[Symbol.for("neon:types")];

// src/generated.d.cts: module augmentation so tsc (compiling src/) sees the
// types on ./load.cjs. Wrapped in `declare module` and made a module via
// `export {}`.
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
