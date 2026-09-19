/**
 * Child program for the no-addon import discriminator. The preload has
 * already made `@bjornpagen/bumbledb-*` unresolvable.
 */
import assert from "node:assert/strict"
import { Result } from "effect"
import { alternatives } from "#alternatives.ts"
import { closed, closedId } from "#closed.ts"
import { Event } from "#event.ts"
import { EventDescriptor } from "#event-descriptor.ts"
import { ExactRational } from "#exact.ts"
import { bool, event, i64, interval, str, u64, uuid } from "#fields.ts"
import { FiniteFunction } from "#finite-function.ts"
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

// Pure Event transport owns an envelope without claiming graph/law admission.
const envelope = Uint8Array.from([66, 69, 86, 84, 1])
const value = Result.getOrThrow(Event.fromBytes(envelope))
envelope.fill(0)
assert.deepEqual(Event.toBytes(value), Uint8Array.from([66, 69, 86, 84, 1]))
const Region = relation("Region", { value: event })
const Regions = schema("Regions", { Region }, [])
assert.equal(
	v(
		query(Regions).rule((r) => {
			const row = v(Region)
			return r.match(Region, row).find({ value: r.pack(row.value) })
		})
	).value.field.kind,
	"event"
)
assert.equal(nativeBindingIsLoaded(), false, "Event authoring must not load the addon")
const descriptor = Result.getOrThrow(EventDescriptor.fromBytes(Buffer.from("BEDC\x01")))
EventDescriptor.describe(descriptor)
EventDescriptor.inspect(descriptor)
EventDescriptor.admit({ kind: "map", map: { source: value, target: value, readouts: [] } })
assert.equal(nativeBindingIsLoaded(), false, "descriptor Effects must be lazy without the addon")
const rational = Result.getOrThrow(ExactRational.fromBytes(Buffer.from("BERA\x01")))
const fn = Result.getOrThrow(FiniteFunction.fromBytes(Buffer.from("BESC\x01\x00")))
ExactRational.decimal("0.1")
ExactRational.add(rational, rational)
ExactRational.toString(rational)
FiniteFunction.constant(value, rational)
FiniteFunction.describe(fn)
FiniteFunction.isZero(fn)
FiniteFunction.expectation(fn, value)
Event.mass(value)
Event.probability(value, value)
assert.equal(nativeBindingIsLoaded(), false, "source Effects must be lazy without the addon")

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
