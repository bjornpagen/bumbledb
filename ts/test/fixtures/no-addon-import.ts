/**
 * Child program for the no-addon import discriminator. The preload has
 * already made `@bjornpagen/bumbledb-*` unresolvable.
 */
import assert from "node:assert/strict"
import { alternatives } from "#alternatives.ts"
import { closed, closedId } from "#closed.ts"
import { bool, i64, interval, str, u64, uuid } from "#fields.ts"
import { loadNativeBinding, nativeBindingIsLoaded } from "#native.ts"
import { Compute } from "#query/compute.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"

assert.equal(nativeBindingIsLoaded(), false, "package import must not load the addon")

const Note = relation("Note", { id: uuid, text: str, pinned: bool })
const Notes = schema("Notes", { Note }, [key(Note, ["id"])])
assert.equal(Notes.relations.Note.name, "Note")
const pinned = Compute.bool(false)
assert.equal(pinned.kind, "literal")
assert.equal(pinned.result, "bool")
const units = Compute.add(Compute.u64(2n), Compute.u64(1n))
assert.equal(units.result, "u64")
assert.equal(nativeBindingIsLoaded(), false, "pure constructors must not touch the addon")

assert.throws(
	() => loadNativeBinding(process.platform, process.arch),
	(error: unknown) => {
		assert.ok(error instanceof Error)
		assert.match(error.message, /native unavailable|no native binary/)
		return true
	}
)
assert.equal(nativeBindingIsLoaded(), false, "the detector must not leave a cached binding")

// All structural extensions remain pure even with the addon unavailable.
const Observed = relation("Observed", { span: interval(i64), amount: u64 })
const Observations = schema("Observations", { Observed }, [])
const derived = query(Observations).rule((r) => {
	const row = v(Observed)
	return r.match(Observed, row).find({
		span: r.difference(row.span, row.span),
		amount: Compute.mulDiv(row.amount, Compute.u64(2n), Compute.u64(3n), "nearestTiesToEven")
	})
})
assert.equal(v(derived).span.field.kind, "interval")
const Choice = closed("Choice", ["One"])
const Parent = relation("Parent", { id: u64, choice: closedId(Choice) })
const Child = relation("Child", { parent: u64 })
assert.equal(alternatives(key(Parent, ["id"]), "choice", Choice, { One: key(Child, ["parent"]) }).length, 2)
assert.equal(nativeBindingIsLoaded(), false)
