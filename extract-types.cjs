const addon = require("./index.node");
const fs = require("fs");
const path = require("path");

const raw = addon.generateTypescriptDeclarations();

// Neon emits top-level `interface` and `type` without `export`. Prepend `export`
// so consumers can import them. Also strip the `generateTypescriptDeclarations`
// function — it's an internal helper, not part of the public API.
const withExports = raw.replace(/^(interface |type )/gm, "export $1");
const cleaned = withExports.replace(
  /^export declare function generateTypescriptDeclarations\(\):.*$\n?/gm,
  ""
);

for (const dir of ["src", "lib"]) {
  const dirPath = path.join(__dirname, dir);
  fs.mkdirSync(dirPath, { recursive: true });
  const outPath = path.join(dirPath, "generated.d.cts");
  fs.writeFileSync(outPath, cleaned);
  console.log(`Wrote ${outPath}`);
}
