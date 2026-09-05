import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { within } from "#capacity.ts"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { str, u64 } from "#fields.ts"
import type { ClassWall, LawfulStatements } from "#law.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { capacity, contained, key, mirrors, renderStatement } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

describe("the three class laws", function laws() {
	test("generators name their classes; a 3-hop chain lands the whole chain in the generator's class; bare stays bare", function chainGolden() {
		const Vocab = closed("Vocab", ["Alpha", "Beta"])
		const A = relation("A", { x: Vocab.id, note: str })
		const B = relation("B", { y: Vocab.id })
		const Chain = schema("Chain", { Vocab, A, B }, [
			key(B, ["y"]),
			contained(on(A, "x"), on(B, "y")),
			contained(on(B, "y"), on(Vocab, "id"))
		])

		const probeAx: Equal<(typeof Chain)["classes"]["A"]["x"], "Vocab.id"> = true
		const probeBy: Equal<(typeof Chain)["classes"]["B"]["y"], "Vocab.id"> = true
		const probeVocab: Equal<(typeof Chain)["classes"]["Vocab"]["id"], "Vocab.id"> = true
		const probeBare: Equal<(typeof Chain)["classes"]["A"]["note"], undefined> = true
		assert.ok(probeAx && probeBy && probeVocab && probeBare)
		assert.deepStrictEqual(Chain.classes, {
			Vocab: { id: "Vocab.id" },
			A: { x: "Vocab.id", note: undefined },
			B: { y: "Vocab.id" }
		})
		assert.ok(Object.isFrozen(Chain.classes) && Object.isFrozen(Chain.classes.A))
	})

	test("a generator-less class is named by its least member in relation-declaration × field-declaration order", function leastMember() {
		const First = relation("First", { a: u64 })
		const Second = relation("Second", { b: u64 })

		const Pairing = schema("Pairing", { Second, First }, [
			key(First, ["a"]),
			key(Second, ["b"]),
			mirrors(on(First, "a"), on(Second, "b"))
		])
		const probeFirst: Equal<(typeof Pairing)["classes"]["First"]["a"], "First.a" | "Second.b"> = true
		const probeSecond: Equal<(typeof Pairing)["classes"]["Second"]["b"], "First.a" | "Second.b"> = true
		const probeSameClass: Equal<(typeof Pairing)["classes"]["First"]["a"], (typeof Pairing)["classes"]["Second"]["b"]> =
			true
		assert.ok(probeFirst && probeSecond && probeSameClass)

		const runtimeName: (typeof Pairing)["classes"]["First"]["a"] = Pairing.classes.First.a
		assert.equal(runtimeName, "Second.b", "the ratified least-member name — Second declared first")
		assert.deepStrictEqual(Pairing.classes, {
			Second: { b: "Second.b" },
			First: { a: "Second.b" }
		})
	})

	test("a ψ-selected closed face pairs exactly like the bare face — the selection changes classes not at all", function psiPairing() {
		const Grade = closed(
			"Grade",
			["Failed", "DirectPass"],
			{ mastered: str },
			{ Failed: { mastered: "no" }, DirectPass: { mastered: "yes" } }
		)
		const Certificate = relation("Certificate", { id: u64, grade: Grade.id })
		const Mastery = schema("Mastery", { Grade, Certificate }, [
			contained(on(Certificate, "grade"), on(Grade.where({ mastered: "yes" }), "id"))
		])
		const probeGrade: Equal<(typeof Mastery)["classes"]["Certificate"]["grade"], "Grade.id"> = true
		// An id in no law is BARE now — the fresh generator authority is deleted.
		const probeId: Equal<(typeof Mastery)["classes"]["Certificate"]["id"], undefined> = true
		const probeColumn: Equal<(typeof Mastery)["classes"]["Grade"]["mastered"], undefined> = true
		assert.ok(probeGrade && probeId && probeColumn)
		assert.equal(Mastery.classes.Certificate?.grade, "Grade.id")
		assert.equal(Mastery.classes.Grade?.mastered, undefined, "a σ-selected column is not a paired slot")
	})

	test("the selected-mirrors shape (Calendar): the mirrors law types the source column with the target's class", function selectedMirrors() {
		const Booking = relation("Booking", { id: u64, room: u64 })
		const CalendarEntry = relation("CalendarEntry", { booking: u64, label: str })
		const Calendar = schema("Calendar", { Booking, CalendarEntry }, [
			key(CalendarEntry, ["booking"]),
			mirrors(on(CalendarEntry.where({ label: "hold" }), "booking"), on(Booking, "id"))
		])

		const probeSource: Equal<(typeof Calendar)["classes"]["CalendarEntry"]["booking"], "Booking.id"> = true
		assert.ok(probeSource)
		assert.deepStrictEqual(Calendar.classes, {
			Booking: { id: "Booking.id", room: undefined },
			CalendarEntry: { booking: "Booking.id", label: undefined }
		})
	})
})

describe("the generator authority after the fresh deletion — closed ids are the ONLY generators", function wall() {
	const Left = relation("Left", { id: u64, peer: u64 })
	const Right = relation("Right", { id: u64 })
	const Vocab = closed("Vocab", ["Alpha", "Beta"])

	test("unifying two ordinary id columns is LAWFUL now — no mint, no wall; least member names the class", function idsUnify() {
		// With fresh deleted, an ordinary id is an application-owned value
		// column like any other: pairing two of them is one class, not a
		// two-mints collision (E-NO-RESERVE: the database issues no identity).
		const direct = [key(Right, ["id"]), contained(on(Left, "id"), on(Right, "id"))] as const
		type Verdict = LawfulStatements<{ Left: typeof Left; Right: typeof Right }, typeof direct>
		const probeLawful: Equal<Verdict, unknown> = true
		assert.ok(probeLawful)
		type NoWall = Verdict extends ClassWall<infer _G, infer _Chain> ? true : false
		const probeNoWall: Equal<NoWall, false> = true
		assert.ok(probeNoWall)

		const Unified = schema("Unified", { Left, Right }, [
			key(Right, ["id"]),
			contained(on(Left, "id"), on(Right, "id"))
		])
		assert.equal(Unified.classes.Left?.id, "Left.id", "least member in declaration order names the class")
		assert.equal(Unified.classes.Right?.id, "Left.id")
	})

	test("mirrors and capacity pairings over ordinary ids are lawful classes too", function pairingsUnify() {
		const Mirrored = schema("Mirrored", { Left, Right }, [
			key(Left, ["id"]),
			key(Right, ["id"]),
			mirrors(on(Left, "id"), on(Right, "id"))
		])
		assert.equal(Mirrored.classes.Left?.id, Mirrored.classes.Right?.id)
		const Capped = schema("Capped", { Left, Right }, [
			key(Left, ["id"]),
			capacity(on(Left, "id"), { from: on(Right, "id"), within: within(0n, 3n) })
		])
		assert.equal(Capped.classes.Left?.id, Capped.classes.Right?.id)
	})

	test("a closed relation's id stays a generator — a bare column never pairs it (the roster wall, at construction)", function closedWall() {
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — a bare u64 never pairs the closed [id]: the roster rides the face shape
			schema("Broken", { Vocab, Left }, [contained(on(Left, "id"), on(Vocab, "id"))])
		}, /Left\.id is a bare column but Vocab\.id is a Vocab reference — closedness rides the descriptor/)
	})

	test("a transitive chain over ordinary ids is one lawful class, not a wall", function transitiveChain() {
		const Bridge = relation("Bridge", { ref: u64 })
		const Chained = schema("Chained", { Left, Right, Bridge }, [
			key(Left, ["id"]),
			key(Right, ["id"]),
			contained(on(Bridge, "ref"), on(Left, "id")),
			contained(on(Bridge, "ref"), on(Right, "id"))
		])
		assert.equal(Chained.classes.Bridge?.ref, "Left.id", "the merged class is named by its least member")
		assert.equal(Chained.classes.Right?.id, "Left.id")
	})
})

describe("the runtime/type agreement and the wire", function agreement() {
	function buildFixture() {
		const Vocab = closed("Vocab", ["Alpha", "Beta"])
		const Holder = relation("Holder", { id: u64, name: str })
		const Account = relation("Account", { id: u64, holder: u64, kind: Vocab.id, note: str })
		const Terms = relation("Terms", { account: u64 })
		return schema("Agreement", { Vocab, Holder, Account, Terms }, [
			key(Holder, ["id"]),
			key(Account, ["id"]),
			key(Terms, ["account"]),
			contained(on(Account, "holder"), on(Holder, "id")),
			contained(on(Account, "kind"), on(Vocab, "id")),
			mirrors(on(Account, "id"), on(Terms, "account")),
			capacity(on(Holder, "id"), { from: on(Account, "holder"), within: within(0n, 3n) })
		])
	}

	const GOLDEN = {
		Vocab: { id: "Vocab.id" },
		Holder: { id: "Holder.id", name: undefined },
		Account: { id: "Account.id", holder: "Holder.id", kind: "Vocab.id", note: undefined },
		Terms: { account: "Account.id" }
	} as const

	test("the runtime class map and the type-level class map are the same map", function diff() {
		const fixture = buildFixture()
		const probe: Equal<(typeof fixture)["classes"], typeof GOLDEN> = true
		assert.ok(probe)
		assert.deepStrictEqual(fixture.classes, GOLDEN)
	})

	test("the manifest golden: statements in == statements out — nothing synthesized, order preserved, count equal", function manifestGolden() {
		const fixture = buildFixture()
		const written = [
			"Holder(id) -> Holder",
			"Account(id) -> Account",
			"Terms(account) -> Terms",
			"Account(holder) <= Holder(id)",
			"Account(kind) <= Vocab(id)",
			"Account(id) == Terms(account)",
			"Holder(id) <={0..3} Account(holder)"
		]
		assert.deepStrictEqual(fixture.statements.map(renderStatement), written)
		const spec = lower(fixture)
		assert.equal(
			spec.statements.length,
			fixture.statements.length,
			"the lowered list is the declared list — count equal"
		)
		assert.equal(spec.statements.length, written.length)
	})

	test("the wire: spec newtype = class name for classed slots, omitted for bare (fingerprint-neutral by the engine's drop)", function wire() {
		const spec = lower(buildFixture())
		const account = spec.relations.find(function byName(candidate) {
			return candidate.name === "Account"
		})
		assert.ok(account)
		assert.deepStrictEqual(
			account.fields.map(function newtypeOf(field) {
				return [field.name, field.newtype]
			}),
			[
				["id", "Account.id"],
				["holder", "Holder.id"],
				["kind", "Vocab.id"],
				["note", undefined]
			]
		)
		const vocab = spec.relations.find(function byName(candidate) {
			return candidate.name === "Vocab"
		})
		assert.ok(vocab)
		assert.equal(vocab.closed?.newtype, "Vocab.id", "a closed relation's handle newtype IS its id's generator class")
	})

	test("integer-index names are refused at construction — the declaration-order law's enumeration hazard", function integerNames() {
		assert.throws(function integerField() {
			relation("R", { "0": u64 })
		}, /integer index — JavaScript object keys re-order integer indices/)
	})

	test("dotted names are refused at construction — the coordinate encoding stays injective at both tiers", function dottedNames() {
		assert.throws(function dottedRelation() {
			relation("A.B", { x: u64 })
		}, /contains a dot — the law classes key on the `relation\.field` coordinate/)
		assert.throws(function dottedField() {
			relation("A", { "B.x": u64 })
		}, /contains a dot/)
		assert.throws(function dottedClosed() {
			closed("A.B", ["Yes", "No"])
		}, /contains a dot/)
	})

	test("a plain __proto__ declaration entry is refused — the Annex B setter would silently drop the key", function protoEntries() {
		const litHandles = { __proto__: { pages: 1n }, Warn: { pages: 2n } }
		assert.throws(function protoLiteralHandle() {
			closed("Sev", ["__proto__", "Warn"], { pages: u64 }, litHandles)
		}, /prototype was replaced/)

		const Sev = closed(
			"Sev",
			["__proto__", "Warn"],
			{ pages: u64 },
			{ ["__proto__"]: { pages: 1n }, Warn: { pages: 2n } }
		)
		assert.deepStrictEqual([...Sev.data.handles], ["__proto__", "Warn"])
		assert.ok(Object.hasOwn(Sev.axioms, "__proto__"))
		assert.deepStrictEqual(Object.getOwnPropertyDescriptor(Sev.axioms, "__proto__")?.value, { pages: 1n })
	})
})
