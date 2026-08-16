/**
 * Type-level pins for the Violation discriminant. Implied auto-keys are
 * the ONE arm whose `statement` is the value `undefined`; every declared
 * form carries `Statement` — not `statement?: Statement` (omit-optional,
 * which `exactOptionalPropertyTypes` refuses when the runtime writes
 * `undefined`). Identity-strength `Equal` probes lock the arms; real
 * `@ts-expect-error` fail-probes lock the refusals.
 */

import assert from "node:assert/strict"
import { test } from "node:test"

import type {
	CapacityViolation,
	ContainmentViolation,
	DeclaredKeyViolation,
	ImpliedKeyViolation,
	MirrorViolation,
	Violation
} from "#db.ts"
import { contained, key, on, relation, str, u64 } from "#index.ts"
import type { Statement } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", { id: u64.fresh, holder: u64 })
const Terms = relation("Terms", { account: u64, rate: u64 })
const termsKey = key(Terms, ["account"])
const holderOf = contained(on(Account, "holder"), on(Holder, "id"))
type Rels = {
	Holder: typeof Holder
	Account: typeof Account
	Terms: typeof Terms
}

type Functionality = Extract<Violation<Rels>, { kind: "functionality" }>
type Containment = Extract<Violation<Rels>, { kind: "containment" }>
type Capacity = Extract<Violation<Rels>, { kind: "capacity" }>

type Cases = [
	Expect<Equal<Violation<Rels>["kind"], "functionality" | "containment" | "capacity">>,
	Expect<Equal<ImpliedKeyViolation<Rels>["statement"], undefined>>,
	Expect<Equal<DeclaredKeyViolation<Rels>["statement"], Statement>>,
	Expect<Equal<ContainmentViolation<Rels>["statement"], Statement>>,
	Expect<Equal<MirrorViolation<Rels>["statement"], Statement>>,
	Expect<Equal<CapacityViolation<Rels>["statement"], Statement>>,
	Expect<Equal<Functionality, ImpliedKeyViolation<Rels> | DeclaredKeyViolation<Rels>>>,
	Expect<Equal<Containment, ContainmentViolation<Rels> | MirrorViolation<Rels>>>,
	Expect<Equal<Capacity, CapacityViolation<Rels>>>,
	Expect<Equal<Extract<Violation<Rels>, { statement: undefined }>, ImpliedKeyViolation<Rels>>>,
	Expect<Equal<"orientation" extends keyof ContainmentViolation<Rels> ? true : false, false>>,
	Expect<Equal<MirrorViolation<Rels>["orientation"], "written" | "mirrored">>
]

function impliedIsUndefined(violation: ImpliedKeyViolation<Rels>): undefined {
	return violation.statement
}

function declaredIsStatement(
	violation: DeclaredKeyViolation<Rels> | ContainmentViolation<Rels> | CapacityViolation<Rels>
): Statement {
	return violation.statement
}

function containmentRejectsUndefined(): Violation<Rels> {
	// @ts-expect-error — declared containments always carry the SDK statement
	return {
		kind: "containment",
		statement: undefined,
		canonical: "",
		direction: "sourceUnsatisfied",
		facts: []
	}
}

function capacityRejectsUndefined(): Violation<Rels> {
	// @ts-expect-error — declared capacity always carries the SDK statement
	return {
		kind: "capacity",
		statement: undefined,
		canonical: "",
		measure: 1n,
		facts: []
	}
}

function impliedRejectsAStatement(): ImpliedKeyViolation<Rels> {
	return {
		kind: "functionality",
		// @ts-expect-error — implied auto-keys have no SDK spelling
		statement: termsKey,
		canonical: "",
		facts: []
	}
}

test("the Violation discriminant loads and the implied arm is undefined", function probe() {
	const implied: ImpliedKeyViolation<Rels> = {
		kind: "functionality",
		statement: undefined,
		canonical: "Holder(id) -> Holder",
		facts: []
	}
	assert.equal(impliedIsUndefined(implied), undefined)
	assert.equal(declaredIsStatement({ kind: "functionality", statement: termsKey, canonical: "", facts: [] }), termsKey)
	assert.equal(
		declaredIsStatement({
			kind: "containment",
			statement: holderOf,
			canonical: "",
			direction: "sourceUnsatisfied",
			facts: []
		}),
		holderOf
	)
})

export type { Cases }
export { capacityRejectsUndefined, containmentRejectsUndefined, impliedRejectsAStatement }
