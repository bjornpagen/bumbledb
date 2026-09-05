import { AuthoringError } from "#errors.ts"
/**
 * The law-typing engine (owner ruling 2026-07-18, "option 2, zero debate"):
 * THE LAWS TYPE THE COLUMNS. Domains are declared nowhere — `schema`
 * computes every field's domain FROM the statement list, at BOTH the type
 * level (this module's type machinery, reading the statements tuple type)
 * and at runtime (a plain union-find over the same pairs), and the two
 * tiers are the same computation by construction.
 *
 * The three class laws (ratified; implemented exactly):
 *
 * 1. GENERATORS — a `fresh` field is a generator and names its class by
 * its declaration coordinate (`"Account.id"`); a closed relation's
 * synthetic id is a generator named `"Kind.id"`.
 * 2. GENERATOR-LESS classes are named by their least member coordinate
 * in relation-declaration × field-declaration order (readable off the
 * relation record and each member's frozen field list at the VALUE
 * tier — deterministic, pinned forever; the wire reads only this
 * tier). At the TYPE tier the same class is carried as its
 * member-coordinate SET (see {@link ClassOfCoord}): TypeScript's
 * union member order is not observably deterministic, so a type-level
 * least-member pick would drift between compilations — the set is the
 * canonical deterministic spelling, the runtime name is always a
 * member of it, and the join judgment is identical at both tiers.
 * 3. BARE — a field in no law has NO class and pairs only with bare in
 * queries (the deliberate sum-domain pointer stays legal).
 *
 * THE WALL: a class containing more than one generator is a contradiction
 * (two mints cannot share a carrier) — a schema-level COMPILE error (the
 * named, self-locating {@link ClassWall}: which generator coordinates
 * collided, through which paired slots) with a construction-time runtime
 * twin (`computeClasses` throws with the same content, naming the exact
 * statement). The TARGET-KEY WALL ({@link TargetKeyWall},
 * 60-containment-parity) rides the same constraint seam: a containment/
 * mirrors/capacity target projection that resolves no key of its relation
 * is the same kind of schema-level compile error, with `schema`'s
 * `verifyTargetKeys` as its authoritative runtime twin.
 *
 * Every paired face of the statement tuple unions its positionwise field
 * slots: containment (ψ-selected targets included — a selection changes
 * pairing not at all), the `==` bijection, and capacity source/target pairs.
 * `key` statements pair nothing (an FD constrains one relation's own
 * rows; it identifies no carriers).
 *
 * The type tier reads pairs off the statement types' exact face data, so
 * spell the statement list INLINE in `schema` (the `const` type
 * parameter keeps the tuple precise). A widened `Statement[]` list
 * degrades the TYPE tier to generators-only (no pair is readable off a
 * widened type) — the runtime map stays complete and authoritative, and
 * the wire lowering reads only the runtime map. Every loop below is
 * tail-recursive with an accumulator, so the machinery rides TypeScript's
 * tail-recursion elimination at primer scale (~40 relations, ~200 slots,
 * ~123 statements); should a schema ever exceed the compiler's limits, tsc
 * fails LOUDLY with its own instantiation-depth error — the map is never
 * silently widened.
 */

import type { AnyClosed } from "#closed.ts"
import { isClosedMember, sealedFieldsOf } from "#closed.ts"
import type { FaceData } from "#face.ts"
import type { Same } from "#judgment.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import type { SchemaRelation, SchemaRelations } from "#schema.ts"
import { renderStatement, type Statement } from "#statements.ts"

type RelationClasses = { readonly [field: string]: string | undefined }

type SchemaClasses = { readonly [relation: string]: RelationClasses }

type ClassRecordOf<Classes extends SchemaClasses, N extends string> = N extends keyof Classes
	? Classes[N]
	: Record<never, never>

type ClassLookup<CR, K> = K extends keyof CR ? CR[K] & (string | undefined) : undefined

type MemberColumns<M> = M extends AnyClosed ? M["columns"] : never

type MemberFieldNames<M extends SchemaRelation> = M extends AnyClosed
	? "id" | (keyof MemberColumns<M> & string)
	: M extends AnyRelation
		? keyof RelationFields<M> & string
		: never

type FreshFieldNames<Fields> = {
	[F in keyof Fields & string]: Fields[F] extends { readonly fresh: true } ? F : never
}[keyof Fields & string]

type MemberGenerators<N extends string, M extends SchemaRelation> = M extends AnyClosed
	? `${N}.id`
	: M extends AnyRelation
		? `${N}.${FreshFieldNames<RelationFields<M>>}`
		: never

type GeneratorsOf<Rels extends SchemaRelations> = {
	[N in keyof Rels & string]: MemberGenerators<N, Rels[N]>
}[keyof Rels & string]

type Pair = readonly [string, string]

type PairList = readonly Pair[]

type ZipCoords<
	SN extends string,
	SP extends readonly string[],
	TN extends string,
	TP extends readonly string[],
	Acc extends PairList = []
> = SP extends readonly [infer SH extends string, ...infer ST extends readonly string[]]
	? TP extends readonly [infer TH extends string, ...infer TT extends readonly string[]]
		? ZipCoords<SN, ST, TN, TT, readonly [...Acc, readonly [`${SN}.${SH}`, `${TN}.${TH}`]]>
		: Acc
	: Acc

type StatementPairs<St extends Statement> = St["data"] extends {
	readonly source: infer S extends FaceData
	readonly target: infer T extends FaceData
}
	? string extends S["owner"]["name"]
		? []
		: string extends T["owner"]["name"]
			? []
			: ZipCoords<S["owner"]["name"], S["projection"], T["owner"]["name"], T["projection"]>
	: []

type PairsOf<Stmts extends readonly Statement[], Acc extends PairList = []> = Stmts extends readonly [
	infer H extends Statement,
	...infer T extends readonly Statement[]
]
	? PairsOf<T, readonly [...Acc, ...StatementPairs<H>]>
	: Acc

type CompOf<Comps extends readonly string[], X extends string> = Comps extends readonly [
	infer H extends string,
	...infer T extends readonly string[]
]
	? [X] extends [H]
		? H
		: CompOf<T, X>
	: never

type WithoutComp<
	Comps extends readonly string[],
	C extends string,
	Acc extends readonly string[] = []
> = Comps extends readonly [infer H extends string, ...infer T extends readonly string[]]
	? [H, C] extends [C, H]
		? WithoutComp<T, C, Acc>
		: WithoutComp<T, C, readonly [...Acc, H]>
	: Acc

type AddPair<Comps extends readonly string[], A extends string, B extends string> = [
	CompOf<Comps, A>,
	CompOf<Comps, B>
] extends [infer CA extends string, infer CB extends string]
	? [CA] extends [never]
		? [CB] extends [never]
			? readonly [...Comps, A | B]
			: readonly [...WithoutComp<Comps, CB>, CB | A]
		: [CB] extends [never]
			? readonly [...WithoutComp<Comps, CA>, CA | B]
			: [CA, CB] extends [CB, CA]
				? Comps
				: readonly [...WithoutComp<WithoutComp<Comps, CA>, CB>, CA | CB]
	: Comps

type BuildComps<Pairs extends PairList, Comps extends readonly string[] = readonly []> = Pairs extends readonly [
	infer P extends Pair,
	...infer T extends PairList
]
	? BuildComps<T, AddPair<Comps, P[0], P[1]>>
	: Comps

type IsMulti<U, All = U> = [U] extends [never] ? false : U extends unknown ? ([All] extends [U] ? false : true) : never

interface ClassWall<Generators extends string, Chain extends readonly string[]> {
	readonly "schema class wall — the statements unify two generators into one class (two mints cannot share a carrier)": {
		readonly generators: Generators
		readonly through: Chain
	}
}

type ChainOf<Pairs extends PairList, C extends string, Acc extends readonly string[] = []> = Pairs extends readonly [
	infer P extends Pair,
	...infer T extends PairList
]
	? [P[0]] extends [C]
		? ChainOf<T, C, readonly [...Acc, `${P[0]} ~ ${P[1]}`]>
		: ChainOf<T, C, Acc>
	: Acc

type WallScan<Comps extends readonly string[], Gens extends string, Pairs extends PairList> = Comps extends readonly [
	infer H extends string,
	...infer T extends readonly string[]
]
	? true extends IsMulti<Extract<H, Gens>>
		? ClassWall<Extract<H, Gens>, ChainOf<Pairs, H>>
		: WallScan<T, Gens, Pairs>
	: unknown

type SetEq<A extends string, B extends string> = Same<A, B>

type KeyEntry = readonly [string, string]

/**
 * Every declared `key` of a statements tuple as {@link KeyEntry} rows —
 * the declared half of the target-key roster (the implied half is read off
 * each target face's own relation value: fresh marks, closed ids). A
 * statement-tuple union or any undecidable key element degrades the WHOLE
 * wall to silent before this roster is ever consulted
 * ({@link DecidableRoster}) — the skip arm here is the same judgment
 * stated locally, kept as belt, as is {@link DeclaredKeyMatch}'s
 * unjudgeable-projection arm.
 */
type DeclaredKeysOf<Stmts extends readonly Statement[], Acc extends readonly KeyEntry[] = []> = Stmts extends readonly [
	infer H extends Statement,
	...infer T extends readonly Statement[]
]
	? H["data"] extends {
			readonly kind: "key"
			readonly owner: infer O extends AnyRelation
			readonly projection: infer P extends readonly string[]
		}
		? string extends O["name"]
			? DeclaredKeysOf<T, Acc>
			: DeclaredKeysOf<T, readonly [...Acc, readonly [O["name"], P[number]]]>
		: DeclaredKeysOf<T, Acc>
	: Acc

type LiteralProjection<P extends readonly string[]> = [true] extends [IsMulti<P>]
	? false
	: [number] extends [P["length"]]
		? false
		: [P] extends [readonly [infer H extends string, ...infer T extends readonly string[]]]
			? [string] extends [H]
				? false
				: [true] extends [IsMulti<H>]
					? false
					: LiteralProjection<T>
			: true

type DecidableKeyData<D> = [D] extends [
	{
		readonly kind: "key"
		readonly owner: infer O extends AnyRelation
		readonly projection: infer P extends readonly string[]
	}
]
	? [true] extends [IsMulti<D>]
		? false
		: [string] extends [O["name"]]
			? false
			: [true] extends [IsMulti<O["name"]>]
				? false
				: LiteralProjection<P>
	: false

/**
 * THE total decidability detector of the type-tier target-key judgment —
 * a strict WHITELIST of provably-judgeable shapes, never a blocklist of
 * known-bad spellings (representation over control flow: the detector is
 * total by construction, so a spelling nobody has met yet degrades
 * instead of firing). It answers `true` ONLY when ALL hold, each check
 * tuple-bracketed against distribution:
 *
 * - SINGULAR — `Stmts` itself is a single non-union type (a ternary
 * between two individually-lawful `as const` statement lists infers a
 * UNION of tuples; a naked `Stmts` would distribute the scan
 * ({@link TargetKeyScan}) and the roster ({@link DeclaredKeysOf})
 * INDEPENDENTLY, cross-judging one arm's faces against the other
 * arm's keys). The union test is the house device ({@link IsMulti}).
 * - FIXED-LENGTH — `Stmts` is a literal tuple with no rest tail:
 * `[number] extends [Stmts["length"]]` detects a rest/array length. A
 * rest-tail tuple (`readonly [typeof stmt,...Statement[]]`) peels its
 * literal head and hides EVERY key in the tail from a head-peeling
 * roster read — the scan would judge the head against a roster blind
 * to keys the value tier can see.
 * - CONCRETE ELEMENTS — every element is a single non-union statement
 * (per-element {@link IsMulti}), and every element whose
 * `data["kind"]` type INCLUDES `"key"` is a single concrete `KeyData`
 * with a literal owner name and a projection that is itself a single
 * non-union fixed-length tuple of string literals
 * ({@link DecidableKeyData}, {@link LiteralProjection}). A bare
 * `Statement` element, a `Statement` union, a `KeyStatement` union, a
 * widened owner, a widened or union projection each make the roster
 * UNKNOWABLE.
 *
 * Any failure is the tier's one forbidden verdict in waiting — a false
 * wall — so anything outside the whitelist degrades the WHOLE
 * {@link TargetKeyWall} to silent (the degradation law: best effort
 * degrades to silent, never to a wrong judgment), exactly as a widened
 * FACE already silences its own judgment; the value tier stays
 * authoritative. ONE recorded limit, pre-existing and outside this
 * detector's reach: a generic type parameter CONSTRAINED to a union of
 * tuples hangs tsc in the deferred-scan machinery before any verdict is
 * reached — a degenerate spelling no shipped surface writes, recorded
 * here as the tier's known boundary, not fixed.
 */
type DecidableRoster<Stmts extends readonly Statement[]> = [true] extends [IsMulti<Stmts>]
	? false
	: [number] extends [Stmts["length"]]
		? false
		: [Stmts] extends [readonly [infer H extends Statement, ...infer T extends readonly Statement[]]]
			? [true] extends [IsMulti<H>]
				? false
				: "key" extends H["data"]["kind"]
					? [DecidableKeyData<H["data"]>] extends [true]
						? DecidableRoster<T>
						: false
					: DecidableRoster<T>
			: true

interface TargetKeyWall<Target extends string, Projection extends string> {
	readonly "schema target-key wall — a containment/mirrors/capacity target projection matches no key of the target relation (declare the key() it resolves, or project an existing one)": {
		readonly target: Target
		readonly projection: Projection
	}
}

type DeclaredKeyMatch<N extends string, PU extends string, Keys extends readonly KeyEntry[]> = Keys extends readonly [
	infer H extends KeyEntry,
	...infer T extends readonly KeyEntry[]
]
	? SetEq<H[0], N> extends true
		? string extends H[1]
			? true
			: SetEq<PU, H[1]> extends true
				? true
				: DeclaredKeyMatch<N, PU, T>
		: DeclaredKeyMatch<N, PU, T>
	: TargetKeyWall<N, PU>

type FreshKeyMatch<PU extends string, O extends AnyRelation> = [PU] extends [FreshFieldNames<RelationFields<O>>]
	? true extends IsMulti<PU>
		? false
		: true
	: false

type JudgeTargetFace<F extends FaceData, Keys extends readonly KeyEntry[]> = string extends F["owner"]["name"]
	? true
	: string extends F["projection"][number]
		? true
		: F["owner"] extends AnyClosed
			? SetEq<F["projection"][number], "id"> extends true
				? true
				: TargetKeyWall<F["owner"]["name"], F["projection"][number]>
			: F["owner"] extends AnyRelation
				? FreshKeyMatch<F["projection"][number], F["owner"]> extends true
					? true
					: DeclaredKeyMatch<F["owner"]["name"], F["projection"][number], Keys>
				: true

type JudgeStatement<St extends Statement, Keys extends readonly KeyEntry[]> = St["data"] extends {
	readonly source: FaceData
	readonly target: infer Tg extends FaceData
}
	? JudgeTargetFace<Tg, Keys>
	: true

type TargetKeyScan<Stmts extends readonly Statement[], Keys extends readonly KeyEntry[]> = Stmts extends readonly [
	infer H extends Statement,
	...infer T extends readonly Statement[]
]
	? [JudgeStatement<H, Keys>] extends [true]
		? TargetKeyScan<T, Keys>
		: JudgeStatement<H, Keys>
	: unknown

type LawfulStatements<Rels extends SchemaRelations, Stmts extends readonly Statement[]> = WallScan<
	BuildComps<PairsOf<Stmts>>,
	GeneratorsOf<Rels>,
	PairsOf<Stmts>
> &
	([DecidableRoster<Stmts>] extends [true] ? TargetKeyScan<Stmts, DeclaredKeysOf<Stmts>> : unknown)

type ClassOfCoord<Comps extends readonly string[], Gens extends string, C extends string> = [CompOf<Comps, C>] extends [
	infer M extends string
]
	? [M] extends [never]
		? [C] extends [Gens]
			? C
			: undefined
		: [Extract<M, Gens>] extends [infer G extends string]
			? [G] extends [never]
				? M
				: G
			: never
	: never

type ComputedClasses<Rels extends SchemaRelations, Comps extends readonly string[], Gens extends string> = {
	readonly [N in keyof Rels & string]: {
		readonly [F in MemberFieldNames<Rels[N]>]: ClassOfCoord<Comps, Gens, `${N}.${F}`>
	}
}

type ClassesOf<Rels extends SchemaRelations, Stmts extends readonly Statement[]> = ComputedClasses<
	Rels,
	BuildComps<PairsOf<Stmts>>,
	GeneratorsOf<Rels>
>

interface MemberCoords {
	readonly relation: string
	readonly fields: ReadonlyArray<{ readonly name: string; readonly generator: boolean }>
}

function memberCoords(relations: SchemaRelations): MemberCoords[] {
	const out: MemberCoords[] = []
	for (const [relationName, member] of Object.entries(relations)) {
		const closed = isClosedMember(member)
		const fields = sealedFieldsOf(member).map(function fieldCoord(declared) {
			return {
				name: declared.name,
				generator: closed ? declared.name === "id" : "fresh" in declared.field && declared.field.fresh === true
			}
		})
		out.push({ relation: relationName, fields })
	}
	return out
}

interface UnionFind {
	find(coord: string): string
	union(a: string, b: string): string
	generatorsOf(root: string): readonly string[]
	markGenerator(coord: string): void
}

function makeUnionFind(): UnionFind {
	const parent = new Map<string, string>()
	const generators = new Map<string, string[]>()
	function find(coord: string): string {
		const at = parent.get(coord)
		if (at === undefined) {
			parent.set(coord, coord)
			return coord
		}
		if (at === coord) {
			return coord
		}
		const root = find(at)
		parent.set(coord, root)
		return root
	}
	return {
		find,
		union(a, b) {
			const rootA = find(a)
			const rootB = find(b)
			if (rootA === rootB) {
				return rootA
			}
			parent.set(rootB, rootA)
			const merged = [...(generators.get(rootA) ?? []), ...(generators.get(rootB) ?? [])]
			generators.delete(rootB)
			if (merged.length > 0) {
				generators.set(rootA, merged)
			}
			return rootA
		},
		generatorsOf(root) {
			return generators.get(root) ?? []
		},
		markGenerator(coord) {
			const root = find(coord)
			generators.set(root, [...(generators.get(root) ?? []), coord])
		}
	}
}

function statementFaces(statement: Statement): readonly [FaceData, FaceData] | undefined {
	const data = statement.data
	if (data.kind === "key") {
		return undefined
	}
	return [data.source, data.target]
}

/**
 * Computes the class map — the runtime twin of {@link ClassesOf}, the SAME
 * computation as a plain union-find: every paired face's positionwise slot
 * pairs union their coordinates; a fresh field or closed id is a generator
 * naming its class; a generator-less class is named by its least member in
 * relation-declaration × field-declaration order; a slot in no law is bare
 * (`undefined`). The one-generator wall throws HERE, naming the two
 * coordinates and the statement that unified them — the same content the
 * compile-tier {@link ClassWall} carries. The returned map is frozen, own
 * properties throughout (arbitrary field names ride own-property
 * definition, never the object protocol).
 */
function computeClasses(name: string, relations: SchemaRelations, statements: readonly Statement[]): SchemaClasses {
	const members = memberCoords(relations)
	const uf = makeUnionFind()
	const paired = new Set<string>()
	const generatorSet = new Set<string>()
	for (const member of members) {
		for (const field of member.fields) {
			const coord = `${member.relation}.${field.name}`
			uf.find(coord)
			if (field.generator) {
				generatorSet.add(coord)
				uf.markGenerator(coord)
			}
		}
	}
	for (const statement of statements) {
		const faces = statementFaces(statement)
		if (faces === undefined) {
			continue
		}
		const [source, target] = faces
		source.projection.forEach(function unionSlot(fieldName, position) {
			const targetField = target.projection[position]
			if (targetField === undefined) {
				return
			}
			const coordA = `${source.owner.name}.${fieldName}`
			const coordB = `${target.owner.name}.${targetField}`
			paired.add(coordA)
			paired.add(coordB)
			const root = uf.union(coordA, coordB)
			const gens = uf.generatorsOf(root)
			if (gens.length > 1) {
				throw new AuthoringError({
					message: `schema ${name}: the statements unify two generators into one class — ${gens.join(" and ")} (two mints cannot share a carrier) — ${renderStatement(statement)}`
				})
			}
		})
	}
	const names = new Map<string, string>()
	for (const member of members) {
		for (const field of member.fields) {
			const coord = `${member.relation}.${field.name}`
			const root = uf.find(coord)
			if (!names.has(root)) {
				const gens = uf.generatorsOf(root)
				names.set(root, gens[0] ?? coord)
			}
		}
	}
	const classes: Record<string, RelationClasses> = {}
	for (const member of members) {
		const record: Record<string, string | undefined> = {}
		for (const field of member.fields) {
			const coord = `${member.relation}.${field.name}`
			const classed = paired.has(coord) || generatorSet.has(coord)
			Object.defineProperty(record, field.name, {
				value: classed ? names.get(uf.find(coord)) : undefined,
				enumerable: true
			})
		}
		Object.freeze(record)
		Object.defineProperty(classes, member.relation, { value: record, enumerable: true })
	}
	return Object.freeze(classes)
}

/**
 * The trusted admission seam of the class-map mint (the pattern's home is
 * `isTypedScope` in query/lower.ts): the
 * checkable facts — one own record per declared relation, one own entry
 * per declared field (the closed sealed shape's `id` included), everything
 * frozen — are verified before the runtime map is admitted at the computed
 * {@link ClassesOf} type. The NAME agreement of the two tiers is pinned by
 * the generated fixture probes (the runtime/type diff check).
 */
function classesComplete<Classes extends SchemaClasses>(
	classes: SchemaClasses,
	relations: SchemaRelations
): classes is Classes {
	if (!Object.isFrozen(classes)) {
		return false
	}
	return memberCoords(relations).every(function relationMinted(member) {
		const record = classes[member.relation]
		if (record === undefined || !Object.isFrozen(record)) {
			return false
		}
		return member.fields.every(function fieldMinted(field) {
			return Object.hasOwn(record, field.name)
		})
	})
}

export type {
	ClassesOf,
	ClassLookup,
	ClassRecordOf,
	ClassWall,
	LawfulStatements,
	RelationClasses,
	SchemaClasses,
	TargetKeyWall
}
export { classesComplete, computeClasses }
