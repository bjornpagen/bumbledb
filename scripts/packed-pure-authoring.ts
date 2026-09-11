/**
 * D27 / D22 addon-unavailable authoring. Native platform packages must
 * be absent. This file must not import or invoke NativeRuntime.layer.
 */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import { Compute, v, uuid, key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"

const Units = relation("Units", { id: uuid, units: u64, name: str })
const incrementUnits = Compute.add(v(Units).units, Compute.u64(1n))
assert.equal(incrementUnits.kind, "add")
assert.equal(incrementUnits.result, "u64")
const nested = Compute.toF64(Compute.add(v(Units).units, Compute.u64(1n)))
assert.equal(nested.kind, "cast")
if (nested.kind === "cast") assert.equal(nested.cast, "toF64")
assert.equal(nested.result, "f64")

let refused = false
try {
	// @ts-expect-error Deliberately exercise the runtime refusal for untyped callers.
	Compute.add(Compute.i64(1n), Compute.u64(1n))
} catch {
	refused = true
}
assert.ok(refused, "D27: known I64/U64 mixing refuses before native load")

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

console.log("packed-pure-authoring: OK — constructs query arithmetic with addon unavailable")
