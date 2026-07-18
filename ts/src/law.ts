/**
 * The law-typing engine (owner ruling 2026-07-18, "option 2, zero debate"):
 * THE LAWS TYPE THE COLUMNS. Domains are declared nowhere — `schema()`
 * computes every field's domain FROM the statement list, at BOTH the type
 * level (this module's type machinery, reading the statements tuple type)
 * and at runtime (a plain union-find over the same pairs), and the two
 * tiers are the same computation by construction.
 *
 * The three class laws (ratified; implemented exactly):
 *
 *   1. GENERATORS — a `fresh` field is a generator and names its class by
 *      its declaration coordinate (`"Account.id"`); a closed relation's
 *      synthetic id is a generator named `"Kind.id"`.
 *   2. GENERATOR-LESS classes are named by their least member coordinate
 *      in relation-declaration × field-declaration order (readable off the
 *      relation record and each member's frozen field list at the VALUE
 *      tier — deterministic, pinned forever; the wire reads only this
 *      tier). At the TYPE tier the same class is carried as its
 *      member-coordinate SET (see {@link ClassOfCoord}): TypeScript's
 *      union member order is not observably deterministic, so a type-level
 *      least-member pick would drift between compilations — the set is the
 *      canonical deterministic spelling, the runtime name is always a
 *      member of it, and the join judgment is identical at both tiers.
 *   3. BARE — a field in no law has NO class and pairs only with bare in
 *      queries (the deliberate sum-domain pointer stays legal).
 *
 * THE WALL: a class containing more than one generator is a contradiction
 * (two mints cannot share a carrier) — a schema-level COMPILE error (the
 * named, self-locating {@link ClassWall}: which generator coordinates
 * collided, through which paired slots) with a construction-time runtime
 * twin (`computeClasses` throws with the same content, naming the exact
 * statement).
 *
 * Every paired face of the statement tuple unions its positionwise field
 * slots: containment (ψ-selected targets included — a selection changes
 * pairing not at all), the `==` bijection, and window source/target pairs.
 * `key()` statements pair nothing (an FD constrains one relation's own
 * rows; it identifies no carriers).
 *
 * The type tier reads pairs off the statement types' exact face data, so
 * spell the statement list INLINE in `schema()` (the `const` type
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

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import type { FaceData } from "#face.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import type { SchemaRelation, SchemaRelations } from "#schema.ts"
import { renderStatement, type Statement } from "#statements.ts"

// ————————————————————————————————————————————————————————————————————————
// The class-map shapes.
// ————————————————————————————————————————————————————————————————————————

/** One relation's computed classes: field name → class name, `undefined` = bare. */
type RelationClasses = { readonly [field: string]: string | undefined }

/**
 * The class map a schema carries — relation name → field name → the
 * computed class name (`undefined` = bare). THE domain authority: queries
 * and the wire lowering read domains from here and nowhere else. The wide
 * shape is the default every `Schema`-generic surface accepts; a concrete
 * schema's `classes` property carries the EXACT computed map
 * ({@link ClassesOf}) at the type level and the frozen runtime twin
 * (`computeClasses`) at the value level — one property, two tiers, one
 * computation.
 */
type SchemaClasses = { readonly [relation: string]: RelationClasses }

/** Looks one relation's class record up in a schema's class map (absent relation = no classes). */
type ClassRecordOf<Classes extends SchemaClasses, N extends string> = N extends keyof Classes
	? Classes[N]
	: Record<never, never>

/** Looks one field's class up in a relation's class record (absent field = bare). */
type ClassLookup<CR, K> = K extends keyof CR ? CR[K] & (string | undefined) : undefined

// ————————————————————————————————————————————————————————————————————————
// Coordinates.
// ————————————————————————————————————————————————————————————————————————

/** A closed member's declared payload-column record (`never` on ordinary relations). */
type MemberColumns<M> = M extends AnyClosed ? M["columns"] : never

/** The field names of one schema member: a relation's declared fields; a closed relation's sealed `id` + columns. */
type MemberFieldNames<M extends SchemaRelation> = M extends AnyClosed
	? "id" | (keyof MemberColumns<M> & string)
	: M extends AnyRelation
		? keyof RelationFields<M> & string
		: never

/** The fresh-marked field names of one field block. */
type FreshFieldNames<Fields> = {
	[F in keyof Fields & string]: Fields[F] extends { readonly fresh: true } ? F : never
}[keyof Fields & string]

/** One member's generator coordinates: fresh fields; a closed relation's synthetic id. */
type MemberGenerators<N extends string, M extends SchemaRelation> = M extends AnyClosed
	? `${N}.id`
	: M extends AnyRelation
		? `${N}.${FreshFieldNames<RelationFields<M>>}`
		: never

/** Every generator coordinate of a relation record (a union). */
type GeneratorsOf<Rels extends SchemaRelations> = {
	[N in keyof Rels & string]: MemberGenerators<N, Rels[N]>
}[keyof Rels & string]

// ————————————————————————————————————————————————————————————————————————
// Pairs: the positionwise slot pairs of every paired face.
// ————————————————————————————————————————————————————————————————————————

/** One paired-slot pair of coordinates. */
type Pair = readonly [string, string]

/** A list of slot pairs. */
type PairList = readonly Pair[]

/** Zips two faces' projections into coordinate pairs, positionwise. */
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

/**
 * One statement's slot pairs: containments (bidirectional included — pair
 * unions are symmetric) and windows pair their two faces positionwise;
 * `key()` pairs nothing. A widened face (owner name or projection no
 * longer literal) contributes nothing — the runtime map stays complete.
 */
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

/** Every slot pair of a statements tuple, in written order. */
type PairsOf<Stmts extends readonly Statement[], Acc extends PairList = []> = Stmts extends readonly [
	infer H extends Statement,
	...infer T extends readonly Statement[]
]
	? PairsOf<T, readonly [...Acc, ...StatementPairs<H>]>
	: Acc

// ————————————————————————————————————————————————————————————————————————
// Union-find over the pairs: connected components as coordinate unions.
// ————————————————————————————————————————————————————————————————————————

/** The component (a union of coordinates) containing `X`, or `never` when `X` is in none. */
type CompOf<Comps extends readonly string[], X extends string> = Comps extends readonly [
	infer H extends string,
	...infer T extends readonly string[]
]
	? [X] extends [H]
		? H
		: CompOf<T, X>
	: never

/** Rebuilds the component list without the component `C` (components are disjoint; identity is mutual extension). */
type WithoutComp<
	Comps extends readonly string[],
	C extends string,
	Acc extends readonly string[] = []
> = Comps extends readonly [infer H extends string, ...infer T extends readonly string[]]
	? [H, C] extends [C, H]
		? WithoutComp<T, C, Acc>
		: WithoutComp<T, C, readonly [...Acc, H]>
	: Acc

/** Unions one pair into the component list: create, extend, keep, or merge. */
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

/** Folds every pair into connected components (tail-recursive — the whole walk is one loop). */
type BuildComps<Pairs extends PairList, Comps extends readonly string[] = readonly []> = Pairs extends readonly [
	infer P extends Pair,
	...infer T extends PairList
]
	? BuildComps<T, AddPair<Comps, P[0], P[1]>>
	: Comps

// ————————————————————————————————————————————————————————————————————————
// The wall and the names.
// ————————————————————————————————————————————————————————————————————————

/** Whether a union holds two or more members (`All` captures the whole union across distribution). */
type IsMulti<U, All = U> = [U] extends [never] ? false : U extends unknown ? ([All] extends [U] ? false : true) : never

/**
 * The named, self-locating compile verdict of the one-generator wall: the
 * generator coordinates that collided and the paired slots (rendered
 * `A.x ~ B.y`, statement order) whose chain unified them. Intersected into
 * `schema()`'s statements parameter, so the error lands ON the statement
 * list with this key naming the law. The runtime twin throws from
 * `computeClasses` with the same content, naming the exact statement.
 */
interface ClassWall<Generators extends string, Chain extends readonly string[]> {
	readonly "schema class wall — the statements unify two generators into one class (two mints cannot share a carrier)": {
		readonly generators: Generators
		readonly through: Chain
	}
}

/** The paired slots lying inside component `C`, rendered — the wall's self-locating chain. */
type ChainOf<Pairs extends PairList, C extends string, Acc extends readonly string[] = []> = Pairs extends readonly [
	infer P extends Pair,
	...infer T extends PairList
]
	? [P[0]] extends [C]
		? ChainOf<T, C, readonly [...Acc, `${P[0]} ~ ${P[1]}`]>
		: ChainOf<T, C, Acc>
	: Acc

/** Scans the components for a two-generator class: `unknown` (lawful) or the {@link ClassWall}. */
type WallScan<Comps extends readonly string[], Gens extends string, Pairs extends PairList> = Comps extends readonly [
	infer H extends string,
	...infer T extends readonly string[]
]
	? true extends IsMulti<Extract<H, Gens>>
		? ClassWall<Extract<H, Gens>, ChainOf<Pairs, H>>
		: WallScan<T, Gens, Pairs>
	: unknown

/**
 * The one-generator-per-class law as a constraint: resolves to `unknown`
 * (a no-op intersection into the statements parameter) when the statement
 * list is lawful, and to the named {@link ClassWall} otherwise.
 */
type LawfulStatements<Rels extends SchemaRelations, Stmts extends readonly Statement[]> = WallScan<
	BuildComps<PairsOf<Stmts>>,
	GeneratorsOf<Rels>,
	PairsOf<Stmts>
>

/**
 * One coordinate's class per the three laws, at the TYPE tier: its
 * component's single generator (the exact literal — a generator names its
 * class); a component-less generator is its own class; a component-less
 * non-generator is bare (`undefined`); and a GENERATOR-LESS component is
 * carried as its member-coordinate SET (the union of the component's
 * coordinates — a canonical, deterministic type). The set REPRESENTS the
 * runtime's least-member class name faithfully: the runtime name is by
 * construction a member, two slots share a class exactly when their sets
 * are identical (so `JoinOk` judges identically at both tiers), and bare
 * never equals a set. The least-member PICK itself is deliberately not
 * made at the type tier: TypeScript's union member order is not observably
 * deterministic (the same key union tuples differently across checking
 * contexts — measured, not conjectured), so any type-level "least in
 * declaration order" would drift between compilations; the ratified
 * declaration-order name lives at the VALUE tier (`computeClasses`), which
 * is the only tier the wire reads.
 */
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

/** The computed class map over precomputed components/generators. */
type ComputedClasses<Rels extends SchemaRelations, Comps extends readonly string[], Gens extends string> = {
	readonly [N in keyof Rels & string]: {
		readonly [F in MemberFieldNames<Rels[N]>]: ClassOfCoord<Comps, Gens, `${N}.${F}`>
	}
}

/**
 * THE type-level class map of a schema: relation name → field name → the
 * law-computed class (`undefined` = bare) — what `schema()` returns as the
 * `classes` property's type, and what query joins compare. Generator
 * classes are exact name literals; generator-less classes are their
 * member-coordinate sets (see {@link ClassOfCoord} — the runtime map's
 * least-member name is always a member, so the property type is honest).
 */
type ClassesOf<Rels extends SchemaRelations, Stmts extends readonly Statement[]> = ComputedClasses<
	Rels,
	BuildComps<PairsOf<Stmts>>,
	GeneratorsOf<Rels>
>

// ————————————————————————————————————————————————————————————————————————
// The runtime twin: the same computation as a plain union-find.
// ————————————————————————————————————————————————————————————————————————

/** One relation's declared coordinates and generator flags, in declaration order. */
interface MemberCoords {
	readonly relation: string
	readonly fields: ReadonlyArray<{ readonly name: string; readonly generator: boolean }>
}

/** Reads every member's coordinates off the relation record, declaration order throughout. */
function memberCoords(relations: SchemaRelations): MemberCoords[] {
	const out: MemberCoords[] = []
	for (const [relationName, member] of Object.entries(relations)) {
		if ("handles" in member.data) {
			const fields = [
				{ name: "id", generator: true },
				...member.data.columns.map(function columnCoord(column) {
					return { name: column.name, generator: false }
				})
			]
			out.push({ relation: relationName, fields })
			continue
		}
		const fields = member.data.fields.map(function fieldCoord(declared) {
			return { name: declared.name, generator: "fresh" in declared.field && declared.field.fresh === true }
		})
		out.push({ relation: relationName, fields })
	}
	return out
}

/** A plain union-find over coordinate strings, with per-root generator rosters. */
interface UnionFind {
	find(coord: string): string
	union(a: string, b: string): string
	generatorsOf(root: string): readonly string[]
	markGenerator(coord: string): void
}

/** Builds the union-find. */
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

/** The paired faces of one statement, or undefined for a key (an FD pairs nothing). */
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
				throw errors.new(
					`schema ${name}: the statements unify two generators into one class — ${gens.join(" and ")} (two mints cannot share a carrier) — ${renderStatement(statement)}`
				)
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
 * The trusted seam of the class-map mint (the `refsComplete` pattern): the
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

export type { ClassesOf, ClassLookup, ClassRecordOf, ClassWall, LawfulStatements, RelationClasses, SchemaClasses }
export { classesComplete, computeClasses }
