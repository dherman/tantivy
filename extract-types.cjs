const addon = require("./index.node");
const fs = require("fs");
const declarations = addon.generateTypescriptDeclarations();
fs.writeFileSync("generated.d.ts", declarations);
console.log(declarations);
