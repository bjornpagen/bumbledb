/** Generate/check the error type roster from the selected exact native addon. */
import assert from "node:assert/strict"
import { readFile, writeFile } from "node:fs/promises"
import { runtimeNative } from "#runtime-native.ts"

const codes = runtimeNative.runtimeErrorCodes()
assert.ok(codes.length > 0 && codes.length < 256)
assert.equal(new Set(codes).size, codes.length)
for (const code of codes) assert.match(code, /^[A-Z][A-Za-z0-9]+$/)
const file = new URL("../src/runtime-codes.ts", import.meta.url)
const source = `// Generated from the exact native runtimeErrorCodes export; do not edit.\nexport const runtimeErrorCodes = [\n${codes.map((code) => `\t${JSON.stringify(code)}`).join(",\n")}\n] as const\n`
if (process.argv.includes("--write")) await writeFile(file, source)
else assert.equal(await readFile(file, "utf8"), source, "native runtime error roster is stale")
