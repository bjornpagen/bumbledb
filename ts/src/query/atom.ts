/**
 * Atoms and conditions, STRUCTURAL edition — the body vocabulary of a
 * rule, mirroring the engine IR variant for variant
 * (`bumbledb/crates/bumbledb/src/ir.rs`, the bijection target;
 * `docs/architecture/20-query-ir.md` normative). A `match` binding record
 * binds fields to vars, params, ∈-set params, or bare structural literals
 * — a closed-reference field's literal is its handle NAME, and a plain
 * ARRAY of names there is membership, folded into the program (closed-only
 * by owner ruling; see {@link BindingInput})
 * (unmentioned fields ARE the wildcard — no wildcard value exists);
 * `not(Rel, {...})` is negation-as-position (anti-join); `eq`/`ne` and the
 * order roster, `pointIn` (the one spelling of `ir::CmpOp::PointIn`,
 * always lowered interval-left), `allen` (the 13-bit mask pair
 * comparison), and `and`/`or` (the input condition-tree grammar) complete
 * the roster. Nothing beyond the IR exists here — and the walls the engine
 * enforces at prepare are TYPES first: a var joins only domain-equal
 * fields (`JoinOk`, checked against the rule environment AND against the
 * binding record's own same-named siblings — two first occurrences of one
 * name inside one record are a join too), an
 * interval-typed var under a non-`pointIn` comparison is unwritable, and a
 * negated atom's variables must be positively bound (env membership IS the
 * safety rule). Every condition value carries its operands raw — the
 * runtime representation is the type-inference carrier, no phantoms.
 *
 * This module also owns the plain runtime DATA a built rule is made of
 * (`RuleData`/`RecData` and friends): frozen values the lowering walks —
 * pure data, so lowering stays a pure, stable function of the query value.
 */

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import type { AnyField, ClosedIdField, ClosedRoster, Infer, IntervalValue } from "#fields.ts"
import type { ClassLookup, ClassRecordOf, SchemaClasses } from "#law.ts"
import type {
	ClassedField,
	Duration,
	EnvShape,
	JoinOk,
	MaskParam,
	Param,
	ParamValueAt,
	SetParam,
	ShapeOf,
	Var
} from "#query/scope.ts"
import { isTerm } from "#query/scope.ts"
import type { AnyRelation, FieldsShape, RelationFields } from "#relation.ts"

/**
 * What a query atom matches over: an ordinary relation or a CLOSED
 * vocabulary (ψ query atoms — the engine folds a resolvable closed atom
 * into a plan-constant member set at prepare, or joins the L1-resident
 * virtual image when the shape does not fold; the SDK never pre-folds and
 * never knows which — transparency is the contract).
 */
type MatchOwner = AnyRelation | AnyClosed

/**
 * The matchable field block of an atom owner: a relation's declared
 * fields; a closed relation's SEALED shape — the synthetic `id` (the
 * value's OWN roster-carrying descriptor, at its precise type: the handle
 * union rides into ψ id bindings and joins exactly as it does on a
 * referencing column) first, then the declared payload columns read
 * through the typed `columns` carrier (the one source of payload typing —
 * no parallel column table exists). The runtime twin is `sealedFieldsOf` in
 * `#query/lower.ts`; the id-first ordinal shift the two tiers share is
 * pinned by the lowering golden.
 */
type MatchFields<R extends MatchOwner> = R extends AnyClosed
	? { readonly id: R["id"] } & R["columns"]
	: R extends AnyRelation
		? RelationFields<R>
		: never

/**
 * One atom-binding position as runtime data. `literalSet` is a membership
 * ARRAY at a closed-reference field, folded into the program: `name` is
 * the content-addressed registry key its dense `ParamId` is minted under
 * (the lowering rides the existing param-set term; the SDK itself supplies
 * the translated member set at every execute — never the host).
 */
type BindingTermData =
	| { readonly kind: "var"; readonly name: string }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "literalSet"; readonly name: string; readonly members: readonly string[] }
	| { readonly kind: "literal"; readonly value: unknown }

/** One resolved binding: the field's name, its descriptor, its law-computed class, and the term. */
interface BindingEntry {
	readonly field: string
	readonly data: AnyField
	readonly class: string | undefined
	readonly term: BindingTermData
}

/** One EDB atom as runtime data (either polarity — polarity is the rule item's; a closed owner is a ψ atom). */
interface AtomData {
	readonly relation: MatchOwner
	readonly bindings: readonly BindingEntry[]
}

/** One comparison operator name (mirrors `ir::CmpOp`). */
type CmpKind = "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"

/** One comparison side as runtime data. */
type CmpTermData =
	| { readonly kind: "var"; readonly name: string }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "measure"; readonly name: string }
	| { readonly kind: "literal"; readonly value: unknown }

/** The `allen` mask position as runtime data. */
type MaskData = { readonly kind: "literal"; readonly mask: number } | { readonly kind: "param"; readonly name: string }

/** One comparison condition as runtime data (`mask` present exactly for `allen`). */
interface CmpData {
	readonly kind: "cmp"
	readonly op: CmpKind
	readonly mask: MaskData | undefined
	readonly lhs: CmpTermData
	readonly rhs: CmpTermData
}

/** One condition-tree node as runtime data (`ir::ConditionTree`). */
interface TreeData {
	readonly kind: "tree"
	readonly op: "and" | "or"
	readonly children: readonly CondData[]
}

/** Any condition node as runtime data. */
type CondData = CmpData | TreeData

/** One aggregate's runtime description (select vocabulary, over var NAMES). */
type AggData =
	| { readonly op: "count" }
	| { readonly op: "countDistinct"; readonly over: string }
	| { readonly op: "fold"; readonly fold: "sum" | "min" | "max"; readonly over: string | { readonly duration: string } }
	| { readonly op: "arg"; readonly direction: "argMax" | "argMin"; readonly over: string; readonly key: string }
	| { readonly op: "pack"; readonly over: string }

/** One classified select entry as runtime data. */
type SelectEntryData =
	| { readonly kind: "var"; readonly over: string }
	| { readonly kind: "measure"; readonly over: string }
	| { readonly kind: "aggregate"; readonly agg: AggData }

/**
 * One answer column: its name (the row object key), its entry, and — when
 * the column's value is a closed reference (a projected var or an
 * Arg-carried payload bound at a closed-referencing field) — the roster the
 * decode lifts row ids back to handle NAMES through (the read half of the
 * marshal bijection; `undefined` on every bare column). The slice is
 * SDK-side marshaling data only: the wire `ProgramIr` never carries it.
 */
interface SelectColumn {
	readonly name: string
	readonly entry: SelectEntryData
	readonly closed: ClosedRoster | undefined
}

/** One body item of a rule, in written order. */
type RuleItem =
	| { readonly kind: "atom"; readonly atom: AtomData }
	| { readonly kind: "negated"; readonly atom: AtomData }
	| { readonly kind: "idb"; readonly rec: RecData; readonly vars: readonly string[] }
	| { readonly kind: "cond"; readonly cond: CondData }

/**
 * One use of a parameter inside a rule, in written order: the census the
 * query-level registry folds (first use mints the dense `ParamId`, first
 * FIELD-ANCHORED use types the wire). `members` is present exactly on a
 * membership-array use (a literal set folded into the program): the handle
 * names the SDK itself translates and supplies at execute — the entry
 * never appears in the host's params object.
 */
interface ParamUse {
	readonly name: string
	readonly shape: "value" | "set" | "mask"
	readonly anchor: AnyField | "measure" | undefined
	readonly op: "binding" | CmpKind
	readonly members: readonly string[] | undefined
}

/** One complete rule as runtime data. */
interface RuleData {
	readonly items: readonly RuleItem[]
	readonly select: readonly SelectColumn[]
	/** Variable name → the classed slot its FIRST positive binding carries (the runtime env — descriptor + class). */
	readonly varFields: Readonly<Record<string, ClassedField>>
	readonly paramUses: readonly ParamUse[]
}

/**
 * One recursive predicate's runtime description — identity keys the dense
 * `PredId` at lowering. `rules` is appended by `rec.rule(...)` and sealed
 * (frozen) when the program's output is declared.
 */
interface RecData {
	readonly name: string
	readonly rules: RuleData[]
}

/**
 * What a binding position of field `F` accepts: a bare structural literal
 * of the field's value type, a var/param/∈-set-param term — and, when the
 * field is interval-typed, a bare point literal (the IR's membership
 * typing rule: an element-typed term at an interval field is point
 * membership; an interval-typed term is value equality). A
 * CLOSED-reference field additionally takes a plain ARRAY of handle names
 * read as membership — `kind: ["Practice", "Review"]` (the drizzle law:
 * set membership is an array, never an operator). Arrays are CLOSED-ONLY
 * in this packet by owner ruling: ordinary u64/str membership already has
 * its spelling through `r.inSet` params; widening literal arrays to every
 * literal-capable kind is a separate future taste call — deliberately not
 * done here.
 */
type BindingInput<F extends AnyField> =
	| Infer<F>
	| (F extends ClosedIdField ? readonly Infer<F>[] : never)
	| (F extends { readonly kind: "interval" } ? bigint : never)
	| Var<string>
	| Param<string>
	| SetParam<string>

/**
 * The `match`/`not` bindings record: per field, a term or literal of that
 * field's structural type; unmentioned fields are wildcards (absence IS
 * the wildcard — the IR has no wildcard variant to spell).
 */
type MatchShape<F extends FieldsShape> = {
	readonly [K in keyof F]?: BindingInput<F[K]>
}

/**
 * One field position of a bindings record as a classed slot: the declared
 * descriptor plus the slot's law-computed class, read off the relation's
 * class record (`CR` — the schema class map's entry for the atom's
 * relation). The one shape every join judgment compares.
 */
type SlotAt<F extends FieldsShape, CR, K> = {
	readonly field: F[K & keyof F]
	readonly class: ClassLookup<CR, K>
}

/**
 * The var binding's judgment against the incoming rule environment: a name
 * already bound must land on a class-equal slot (bare pairs only with bare).
 */
type EnvJoinOk<Env extends EnvShape, F extends FieldsShape, CR, K, N extends string> = N extends keyof Env
	? JoinOk<Env[N], SlotAt<F, CR, K>>
	: true

/**
 * The var binding's judgment against its OWN record's siblings: two
 * bindings of one var name inside a single bindings record are the same
 * join the environment check judges across atoms, so every same-named
 * sibling must be class-equal too. Without this arm two FIRST occurrences
 * of one name (a record the environment has not seen yet) would meet no
 * check at all — the intra-atom join would silently cross classes.
 */
type SiblingJoinOk<F extends FieldsShape, CR, B, K extends keyof B, N extends string> = false extends {
	[K2 in Exclude<keyof B & keyof F, K>]: B[K2] extends Var<N> ? JoinOk<SlotAt<F, CR, K2>, SlotAt<F, CR, K>> : true
}[Exclude<keyof B & keyof F, K>]
	? false
	: true

/**
 * The per-property join judgment of a bindings record: a var binding must
 * be class-equal to the rule environment's binding of the name AND to
 * every same-named sibling of its own record (a cross-class reuse maps
 * the property to `never` — the compile error the old value brand carried,
 * now law-born off the schema type's class map).
 */
type BindingOk<Env extends EnvShape, F extends FieldsShape, CR, B, K extends keyof B> =
	B[K] extends Var<infer N extends string>
		? [EnvJoinOk<Env, F, CR, K, N>, SiblingJoinOk<F, CR, B, K, N>] extends [true, true]
			? true
			: false
		: true

/** The validated bindings record (intersect with the inferred `B` — errors land on the offending property). */
type CheckBindings<Env extends EnvShape, F extends FieldsShape, CR, B> = {
	readonly [K in keyof B]: K extends keyof F ? (BindingOk<Env, F, CR, B, K> extends true ? B[K] : never) : never
}

/** The environment a bindings record contributes: var name → the bound slot (descriptor + class). */
type BindEnv<F extends FieldsShape, CR, B> = {
	readonly [K in keyof B & keyof F as B[K] extends Var<infer N extends string> ? N : never]: SlotAt<F, CR, K>
}

/** The params-object fragments a bindings record contributes (one union member per param use). */
type BindParams<F extends FieldsShape, B> = {
	[K in keyof B & keyof F]: B[K] extends Param<infer P extends string>
		? { readonly [Q in P]: ParamValueAt<F[K]> }
		: B[K] extends SetParam<infer P extends string>
			? { readonly [Q in P]: readonly ParamValueAt<F[K]>[] }
			: never
}[keyof B & keyof F]

/**
 * One comparison VALUE: op plus its operands, raw — the runtime
 * representation carries the operand types, so `.where`'s environment
 * check and the params inference both read the value itself (no phantom).
 * `mask` is populated exactly for `allen`.
 */
interface Cmp<Op extends CmpKind, L, R, M = undefined> {
	readonly cond: "cmp"
	readonly op: Op
	readonly lhs: L
	readonly rhs: R
	readonly mask: M
}

/** One condition-tree VALUE (`and`/`or` over comparisons and nested trees). */
interface Tree<Ch extends readonly AnyTreeChild[]> {
	readonly cond: "tree"
	readonly op: "and" | "or"
	readonly children: Ch
}

/**
 * One negated-atom VALUE — negation is a position in the rule (anti-join
 * over sets, no null trick): a binding satisfies it iff NO fact matches.
 * Its variables must be positively bound in the rule — environment
 * membership at `.where` IS the safety rule, a compile error before it is
 * the engine's refusal.
 */
interface NotAtom<R extends MatchOwner, B> {
	readonly cond: "not"
	readonly relation: R
	readonly bindings: B
}

/** Any comparison value. */
type AnyCmp = Cmp<CmpKind, unknown, unknown, unknown>

/** Any condition-tree child (trees hold comparisons and trees — never atoms). */
type AnyTreeChild = AnyCmp | Tree<readonly AnyTreeChild[]>

/** Any negated-atom value. */
type AnyNotAtom = NotAtom<MatchOwner, unknown>

/** Any `.where` input: a comparison, a condition tree, or a negated atom. */
type AnyCond = AnyCmp | Tree<readonly AnyTreeChild[]> | AnyNotAtom

/** What `eq`'s right side accepts (`ParamSet` is `Eq`-only — the IR's rule). */
type EqRight = Var<string> | Param<string> | SetParam<string> | bigint | string | boolean | Uint8Array | IntervalValue

/** What `ne`'s right side accepts. */
type NeRight = Var<string> | Param<string> | bigint | string | boolean | Uint8Array | IntervalValue

/** One side of an order comparison: orderable terms only (the IR's comparison rules). */
type OrderSide = Var<string> | Param<string> | Duration<string> | bigint

/** The point side of `pointIn`. */
type PointSide = Var<string> | Param<string> | bigint

/** The interval side of `pointIn`/`allen`. */
type IntervalSide = Var<string> | Param<string> | IntervalValue

/** Builds one comparison value. */
function comparison<Op extends CmpKind, L, R, M>(op: Op, lhs: L, rhs: R, mask: M): Cmp<Op, L, R, M> {
	return Object.freeze({ cond: "cmp", op, lhs, rhs, mask })
}

/**
 * Rejects a comparison with no term side: it is constant-valued, the
 * engine's own validation refuses it, and the lowering has no anchored
 * position to type the literals by — fail here with the same verdict.
 */
function assertTermSide(op: string, lhs: unknown, rhs: unknown): void {
	if (!isTerm(lhs) && !isTerm(rhs)) {
		throw errors.new(
			`${op}: a comparison without a variable or parameter side is constant-valued — write the query you mean`
		)
	}
}

/**
 * The equality comparison (`ir::CmpOp::Eq`) — a bound variable against a
 * variable (var-to-var unification, domain-equal by the environment
 * check), a param (typed by the variable), an ∈-set param (`Eq`-only, the
 * IR's set rule), or a bare literal of the variable's own value type.
 * Prefer direct placement in `match` where punning applies.
 */
function eq<L extends Var<string>, const R extends EqRight>(left: L, right: R): Cmp<"eq", L, R> {
	return comparison("eq", left, right, undefined)
}

/** Typed disequality (`ir::CmpOp::Ne`). "Not in set" has no operator — write a negated atom. */
function ne<L extends Var<string>, const R extends NeRight>(left: L, right: R): Cmp<"ne", L, R> {
	return comparison("ne", left, right, undefined)
}

/** The shared order-comparison constructor. */
function order<Op extends "lt" | "le" | "gt" | "ge", const L extends OrderSide, const R extends OrderSide>(
	op: Op,
	left: L,
	right: R
): Cmp<Op, L, R> {
	assertTermSide(op, left, right)
	return comparison(op, left, right, undefined)
}

/** Strict less-than (`ir::CmpOp::Lt`) — orderable sides only, never intervals/bytes/strings/bools. */
function lt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"lt", L, R> {
	return order("lt", left, right)
}

/** Less-or-equal (`ir::CmpOp::Le`). */
function le<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"le", L, R> {
	return order("le", left, right)
}

/** Strict greater-than (`ir::CmpOp::Gt`). */
function gt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"gt", L, R> {
	return order("gt", left, right)
}

/** Greater-or-equal (`ir::CmpOp::Ge`). */
function ge<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"ge", L, R> {
	return order("ge", left, right)
}

/**
 * Point membership as a predicate (`ir::CmpOp::PointIn`) — THE one
 * spelling: `pointIn(t, w)` holds iff `w.start ≤ t < w.end`. The IR
 * orders the operands interval-left, point-right; the value stores them
 * that way whatever the surface argument order (a literal `span(...)`
 * interval operand is legal and tags by the point sibling's element
 * domain — the bug-hunt fix, now also a type-level guarantee).
 * Interval ⊇ interval is NOT this operator; that predicate is
 * `allen(a, ALLEN.covers, b)` — the name `covers` belongs to the Allen
 * roster alone (the canonical-utterance law: one meaning, one spelling).
 */
function pointIn<const P extends PointSide, const I extends IntervalSide>(point: P, interval: I): Cmp<"pointIn", I, P> {
	assertTermSide("pointIn", point, interval)
	return comparison("pointIn", interval, point, undefined)
}

/**
 * The 13-bit mask range: bits above the low 13 are unrepresentable in the
 * engine's `AllenMask` (`bumbledb/crates/bumbledb/src/allen.rs`:
 * `AllenMask::new` refuses them) — the check here is the same boundary,
 * moved to construction where the message can name the constants.
 */
const ALLEN_ALL_BITS = (1 << 13) - 1

/**
 * The Allen coordinate system's named constants — the 13 basics in the
 * engine's palindromic bit order (bit i = basic i:
 * `bumbledb/crates/bumbledb/src/allen.rs`, a specified representation)
 * plus the workload composites, values identical to the engine's. Compose
 * with `|`: `ALLEN.before | ALLEN.meets`.
 */
const ALLEN = Object.freeze({
	before: 1 << 0,
	meets: 1 << 1,
	overlaps: 1 << 2,
	starts: 1 << 3,
	during: 1 << 4,
	finishes: 1 << 5,
	equals: 1 << 6,
	finishedBy: 1 << 7,
	contains: 1 << 8,
	startedBy: 1 << 9,
	overlappedBy: 1 << 10,
	metBy: 1 << 11,
	after: 1 << 12,
	/** The point-sets share a point (9 bits; under half-open intervals *meets* shares none). */
	intersects: (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10),
	/** Point-set ⊇: equals ∪ contains ∪ started-by ∪ finished-by. */
	covers: (1 << 6) | (1 << 8) | (1 << 9) | (1 << 7),
	/** Point-set ⊆ — `covers`' converse: equals ∪ during ∪ starts ∪ finishes. */
	coveredBy: (1 << 6) | (1 << 4) | (1 << 3) | (1 << 5),
	/** The point-sets share no point: before ∪ meets ∪ met-by ∪ after. */
	disjoint: (1 << 0) | (1 << 1) | (1 << 11) | (1 << 12)
})

/**
 * THE interval-pair comparison (`ir::CmpOp::Allen`): two interval terms of
 * one element type, satisfied iff the pair's classification is in the
 * 13-bit mask — a literal built from the `ALLEN` constants, or a mask
 * parameter (`r.maskParam`). Vacuous masks (empty/full) are the engine's
 * two distinct typed rejections; nothing is pre-judged here beyond the
 * representable bit range.
 */
function allen<const A extends IntervalSide, const M extends number | MaskParam<string>, const B extends IntervalSide>(
	left: A,
	mask: M,
	right: B
): Cmp<"allen", A, B, M> {
	assertTermSide("allen", left, right)
	if (typeof mask === "number" && (!Number.isInteger(mask) || mask < 0 || mask > ALLEN_ALL_BITS)) {
		throw errors.new(
			`allen mask ${mask} is not a 13-bit mask — build masks from the ALLEN constants (bumbledb allen.rs: bits above the low 13 are unrepresentable)`
		)
	}
	return comparison("allen", left, right, mask)
}

/**
 * Conjunction node of the input condition grammar (`ConditionTree::And`).
 * The rule's condition list is already a conjunction — `and` exists for
 * nesting under `or`, and the empty combination keeps the IR's algebraic
 * reading (`And([])` is true).
 */
function and<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "and", children: Object.freeze(children) })
}

/**
 * Disjunction node of the input condition grammar (`ConditionTree::Or`) —
 * the one place the surface admits a nested OR; validation distributes it
 * to DNF rules engine-side (OR is data or it is nothing). `Or([])` keeps
 * its algebraic reading (false: the rule denotes nothing).
 */
function or<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "or", children: Object.freeze(children) })
}

/**
 * Negation — anti-join over sets: `not(Rel, { field: r.var("x"), ... })`
 * rejects every binding some matching fact extends. A negated atom binds
 * nothing, only rejects: every variable it names must be positively bound
 * in the rule, which `.where`'s environment check makes a COMPILE error
 * (the engine's safety refusal stands behind it). A CLOSED owner is legal
 * here too — the engine folds a resolvable negated closed atom to the
 * COMPLEMENT of its member set (domain-witness guarded), and the SDK's
 * negation rules apply to it unchanged.
 */
function not<R extends MatchOwner, const B extends MatchShape<MatchFields<R>>>(
	relation: R,
	bindings: B
): NotAtom<R, B> {
	const value: NotAtom<R, B> = { cond: "not", relation, bindings }
	return Object.freeze(value)
}

/**
 * Whether a var name is bound in the environment at an orderable (u64/i64)
 * field. A CLOSED reference is excluded even though its kind is `u64`: a
 * vocabulary's declaration-id order is an accident, not semantics
 * (`docs/architecture/10-data-model.md` § orderability — order on it is
 * refused exactly as the enum's ordinal order was), so every
 * order-comparison and fold position refuses closed-bound terms — this
 * judgment is the one gate they all read, and the construction-time
 * validations in `#query/lower.ts` are its runtime twin.
 */
type OrderVarOk<Env extends EnvShape, N extends string> = N extends keyof Env
	? Env[N]["field"] extends { readonly closed: ClosedRoster }
		? false
		: Env[N]["field"]["kind"] extends "u64" | "i64"
			? true
			: false
	: false

/** Whether a var name is bound at an interval field. */
type IntervalVarOk<Env extends EnvShape, N extends string> = N extends keyof Env
	? Env[N]["field"]["kind"] extends "interval"
		? true
		: false
	: false

/** One order-comparison side's judgment against the environment. */
type OrderSideOk<Env extends EnvShape, T> =
	T extends Var<infer N extends string>
		? OrderVarOk<Env, N>
		: T extends Duration<infer N extends string>
			? IntervalVarOk<Env, N>
			: true

/** One point side's judgment. */
type PointSideOk<Env extends EnvShape, T> = T extends Var<infer N extends string> ? OrderVarOk<Env, N> : true

/** One interval side's judgment. */
type IntervalSideOk<Env extends EnvShape, T> = T extends Var<infer N extends string> ? IntervalVarOk<Env, N> : true

/** The `eq`/`ne` judgment: left var bound; right joins it (class-equal var, param, or an exact-type literal). */
type EqOk<Env extends EnvShape, L, R> =
	L extends Var<infer N extends string>
		? N extends keyof Env
			? R extends Var<infer M extends string>
				? M extends keyof Env
					? JoinOk<Env[N], Env[M]>
					: false
				: R extends Param<string> | SetParam<string>
					? true
					: [R] extends [Infer<Env[N]["field"]>]
						? true
						: false
			: false
		: false

/** One negated-atom binding's judgment: a var must be positively bound (safety) AND class-equal. */
type NotBindingOk<Env extends EnvShape, S extends ClassedField, T> =
	T extends Var<infer N extends string> ? (N extends keyof Env ? JoinOk<Env[N], S> : false) : true

/** The whole negated atom's judgment (`CR` — the negated relation's class record off the schema class map). */
type NotOk<Env extends EnvShape, F extends FieldsShape, CR, B> = false extends {
	[K in keyof B]: NotBindingOk<Env, SlotAt<F, CR, K>, B[K]>
}[keyof B]
	? false
	: true

/**
 * One condition's judgment against the rule environment — the type-level
 * twin of the engine's comparison roster: class-equal joins (off the
 * schema type's class map), orderable order sides (an interval var under a
 * non-`pointIn` op is exactly here refused), kind-correct
 * `pointIn`/`allen` sides, and negated-atom safety (the negated
 * relation's class record is resolved through `Classes` by its name). The
 * leading `[AnyTreeChild] extends [C]` arm is the recursion's base case:
 * at an UNRESOLVED constraint (the whole condition union — or a tree's
 * child union, which is the union itself) the judgment is vacuously true —
 * without it the constraint instantiation recurses into itself.
 */
type CondOkBool<Env extends EnvShape, Classes extends SchemaClasses, C> = [AnyTreeChild] extends [C]
	? true
	: C extends Cmp<infer Op, infer L, infer R, unknown>
		? Op extends "eq" | "ne"
			? EqOk<Env, L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? [OrderSideOk<Env, L>, OrderSideOk<Env, R>] extends [true, true]
					? true
					: false
				: Op extends "pointIn"
					? [IntervalSideOk<Env, L>, PointSideOk<Env, R>] extends [true, true]
						? true
						: false
					: Op extends "allen"
						? [IntervalSideOk<Env, L>, IntervalSideOk<Env, R>] extends [true, true]
							? true
							: false
						: false
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? false extends CondOkBool<Env, Classes, Ch[number]>
				? false
				: true
			: C extends NotAtom<infer R extends MatchOwner, infer B>
				? NotOk<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
				: false

/** The validated `.where` argument (intersect with the inferred condition type). */
type CheckCond<Env extends EnvShape, Classes extends SchemaClasses, C> =
	CondOkBool<Env, Classes, C> extends true ? C : never

/** The `eq`/`ne` params contribution: the param typed by the left variable's field. */
type EqParams<Env extends EnvShape, L, R> =
	L extends Var<infer N extends string>
		? R extends Param<infer P extends string>
			? { readonly [Q in P]: Infer<Env[N & keyof Env]["field"]> }
			: R extends SetParam<infer P extends string>
				? { readonly [Q in P]: readonly Infer<Env[N & keyof Env]["field"]>[] }
				: never
		: never

/** An order side's params contribution (order params are always `bigint`). */
type OrderSideParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: bigint } : never

/** An interval side's params contribution. */
type IntervalSideParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: IntervalValue } : never

/** The mask position's params contribution. */
type MaskParams<M> = M extends MaskParam<infer P extends string> ? { readonly [Q in P]: number } : never

/**
 * One condition's params-object fragments (a union; the rule builder folds
 * them into the inferred `Params` record) — every param typed by its use.
 * The leading arm is the same base case as {@link CondOkBool}'s: the
 * unresolved constraint contributes nothing.
 */
type CondParams<Env extends EnvShape, C> = [AnyTreeChild] extends [C]
	? never
	: C extends Cmp<infer Op, infer L, infer R, infer M>
		? Op extends "eq" | "ne"
			? EqParams<Env, L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? OrderSideParams<L> | OrderSideParams<R>
				: Op extends "pointIn"
					? IntervalSideParams<L> | OrderSideParams<R>
					: Op extends "allen"
						? IntervalSideParams<L> | IntervalSideParams<R> | MaskParams<M>
						: never
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? CondParams<Env, Ch[number]>
			: C extends NotAtom<infer R extends MatchOwner, infer B>
				? BindParams<MatchFields<R>, B>
				: never

/** The flattened params record one bindings record contributes. */
type BindParamsShape<F extends FieldsShape, B> = ShapeOf<BindParams<F, B>>

/** The flattened params record one condition contributes. */
type CondParamsShape<Env extends EnvShape, C> = ShapeOf<CondParams<Env, C>>

export type {
	AggData,
	AnyCmp,
	AnyCond,
	AnyNotAtom,
	AnyTreeChild,
	AtomData,
	BindEnv,
	BindingEntry,
	BindingInput,
	BindingTermData,
	BindParams,
	BindParamsShape,
	CheckBindings,
	CheckCond,
	Cmp,
	CmpData,
	CmpKind,
	CmpTermData,
	CondData,
	CondOkBool,
	CondParams,
	CondParamsShape,
	IntervalSide,
	IntervalVarOk,
	MaskData,
	MatchFields,
	MatchOwner,
	MatchShape,
	NotAtom,
	OrderSide,
	OrderVarOk,
	ParamUse,
	PointSide,
	RecData,
	RuleData,
	RuleItem,
	SelectColumn,
	SelectEntryData,
	Tree,
	TreeData
}
export { ALLEN, ALLEN_ALL_BITS, allen, and, comparison, eq, ge, gt, le, lt, ne, not, or, pointIn }
