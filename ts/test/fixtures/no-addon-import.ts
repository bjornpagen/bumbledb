/**
 * Child program for the no-addon import discriminator. The preload has
 * already made `@bjornpagen/bumbledb-*` unresolvable.
 */
import assert from "node:assert/strict"
import { loadNativeBinding, nativeBindingIsLoaded } from "#native.ts"
import { bool, id128, str } from "#fields.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { Scalar } from "#scalar.ts"

assert.equal(nativeBindingIsLoaded(), false, "package import must not load the addon")

const Note = relation("Note", { id: id128, text: str, pinned: bool })
const Notes = schema("Notes", { Note }, [key(Note, ["id"])])
assert.equal(Notes.relations.Note.name, "Note")
const pinned = Scalar.bool(false)
assert.equal(pinned.kind, "literal")
assert.equal(pinned.result, "bool")
const units = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
assert.equal(units.result, "unresolved")
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
