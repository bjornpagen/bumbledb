/**
 * D27 / D22 addon-unavailable authoring. Native platform packages must
 * be absent. This file must not import or invoke NativeRuntime.layer.
 */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import { Scalar, id128, key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"

const incrementUnits = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
assert.equal(incrementUnits.kind, "add")
assert.equal(incrementUnits.result, "unresolved")
const nested = Scalar.toF64(Scalar.add(Scalar.field("units"), Scalar.u64(1n)))
assert.equal(nested.kind, "toF64")
assert.equal(nested.result, "unresolved")

let refused = false
try {
	Scalar.add(Scalar.i64(1n), Scalar.u64(1n))
} catch {
	refused = true
}
assert.ok(refused, "D27: known I64/U64 mixing refuses before native load")

const Units = relation("Units", { id: id128, units: u64, name: str })
const theory = schema("UnitsTheory", { Units }, [key(Units, ["id"])])
assert.equal(theory.name, "UnitsTheory")

const req = createRequire(import.meta.url)
for (const plat of ["darwin-arm64", "linux-arm64", "linux-x64"]) {
	try {
		req.resolve(`@bjornpagen/bumbledb-${plat}`)
		throw new Error(`D27: native package @bjornpagen/bumbledb-${plat} must be unavailable`)
	} catch (err) {
		if (err instanceof Error && err.message.startsWith("D27:")) throw err
	}
}

console.log("packed-pure-authoring: OK — D27 constructs unresolved field arithmetic with addon unavailable")
