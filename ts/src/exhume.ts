/**
 * The exhume surface — the SDK's ONE schema-independent read path
 * (docs/course-serialization/prd-02-sdk-exhume-surface.md; engine
 * 70-api.md § exhume): a read-only, theory-less open returning the store's
 * SELF-DESCRIBED relation shapes and raw facts by relation name. The
 * sighting it exists for: a run store whose creating schema has since
 * evolved — the record outlives the schema, and exhume is how the record
 * is read back for rebirth (exhume the old store, create the successor
 * under the new theory, copy by NAME, re-derive).
 *
 * DELIBERATELY SCHEMA-FREE: no schema type appears anywhere on this
 * surface. The caller's schema is the wrong theory for an exhumed store BY
 * DEFINITION (a store the current theory could open would never need
 * exhuming), so every value crosses TYPED at its bare structural form
 * ({@link FactValue} — bigint/string/boolean/bytes/interval, never
 * `unknown`) and every fact is keyed by field NAME. The SDK never
 * reconstructs a `Schema` value from the descriptor — that inverse mapping
 * is deliberately out of scope; the rebirth tool keys by name.
 *
 * ZERO CLOSABLES: no value here carries a close, dispose, or release
 * spelling. The engine-side handle (and the store's exclusive advisory
 * lock) is reclaimed when the `Exhumed` value is garbage-collected —
 * reclamation only, never correctness: the store is never written through
 * this surface, so there is nothing to flush.
 */

import * as errors from "@superbuilders/errors"
import type { FactValue, Manifest } from "#native.ts"
import { native } from "#native.ts"
import type { ValueTypeSpec } from "#spec.ts"

/**
 * The typed `descriptorMissing` refusal: the store predates self-describing
 * stores and has not been adopted. The remedy is in the message — one
 * fingerprint-matching `Db.open` under the creating schema back-fills the
 * descriptor (engine 50-storage.md § the `_meta` block) and the store is
 * self-describing forever. Match with `errors.is`.
 */
const ErrExhumeNoDescriptor = errors.new(
	"bumbledb exhume: the store carries no schema descriptor (not yet adopted) — open it once under its creating schema (one fingerprint-matching Db.open back-fills the descriptor; engine 50-storage.md)"
)

/**
 * The typed `formatMismatch` refusal: the store's on-disk format version is
 * not this engine's (no migration path exists, as everywhere). Match with
 * `errors.is`.
 */
const ErrExhumeFormatMismatch = errors.new(
	"bumbledb exhume: storage format version mismatch — the store was written by a different engine format"
)

/**
 * The typed `corruption` refusal: the persisted descriptor fails its
 * integrity gates (the stored bytes hash to something other than the stored
 * fingerprint, or the bytes do not decode and re-encode faithfully). Match
 * with `errors.is`.
 */
const ErrExhumeCorruption = errors.new("bumbledb exhume: the persisted schema descriptor fails its integrity gates")

/**
 * One exhumed fact as a plain name-keyed record of bare structural values
 * — deliberately schema-free (module doc): `bigint` for u64/i64, `string`
 * for str, `boolean` for bool, `Uint8Array` for bytes<N>, a `{ start, end }`
 * bigint pair for intervals. Closed-relation rows open with the synthetic
 * `id` field, exactly as the descriptor's sealed field list declares.
 */
type ExhumedFact = Readonly<Record<string, FactValue>>

/**
 * One field of an exhumed relation, exactly as the store's creator declared
 * it: the name and the structural type tag (byte width on `fixedBytes`,
 * element and optional width on `interval`).
 */
interface ExhumedField {
	readonly name: string
	readonly valueType: ValueTypeSpec
}

/**
 * One ground axiom of an exhumed closed relation: the handle (the row's
 * identity, never a column), the declaration-order row id, and the declared
 * payload columns by name.
 */
interface ExhumedAxiom {
	readonly handle: string
	readonly id: bigint
	readonly values: ExhumedFact
}

/**
 * One relation of an exhumed store: the name, the SEALED field list in
 * declaration order (a closed relation opens with the synthetic (`id`,
 * u64) handle field), and — exactly on closed relations — the roster of
 * ground axioms. Scan rows come back in this field order, so pairing a
 * row's positions against `fields` is the name-keyed reading `scan`
 * performs.
 */
interface ExhumedRelation {
	readonly name: string
	readonly fields: readonly ExhumedField[]
	readonly roster: readonly ExhumedAxiom[] | undefined
}

/**
 * The exhumed store's schema as declared, decoded from the store's own
 * persisted descriptor: relations in engine-id order (declaration order
 * mints every id), each with its ordered field descriptions and closed
 * roster — enough for a caller to key facts by name and re-insert them
 * into a differently-fingerprinted successor store. No schema type appears
 * here (module doc: the surface is schema-free; rows are typed bare).
 */
interface ExhumedDescriptor {
	readonly relations: readonly ExhumedRelation[]
}

/**
 * One exhumed store: the self-described relation shapes and the raw facts
 * by relation name. Read-only by construction — no write verb, no prepare
 * verb, no close (zero closables; the engine handle is GC-reclaimed).
 */
interface Exhumed {
	/** The store's own persisted schema, as its creator declared it. */
	readonly descriptor: ExhumedDescriptor
	/**
	 * Full-relation export in row-id order, each row decoded per the
	 * STORED descriptor to a name-keyed record of natural values (str
	 * resolved through the engine's `_dict` before crossing; a closed
	 * relation scans its sealed roster). Each call reads one consistent
	 * snapshot. An unknown relation name is a typed error — the
	 * descriptor is the caller's roster.
	 */
	scan(relation: string): readonly ExhumedFact[]
}

/**
 * The bridge guard (db.ts's twin over this module's calls): runs one
 * native call and wraps anything it throws, so marshal-shape and
 * handle-lifecycle refusals cross as genuine typed failures, never bare
 * foreign errors.
 */
function bridged<T>(context: string, run: () => T): T {
	const result = errors.trySync(run)
	if (result.error) {
		throw errors.wrap(result.error, context)
	}
	return result.data
}

/**
 * Shapes the bridge's manifest rendering of the stored descriptor into the
 * SDK's {@link ExhumedDescriptor}: same relations in the same engine-id
 * order, extension rows re-keyed by column name. Pure reshaping — no
 * re-validation of engine-decoded values happens here (the bridge stays
 * dumb and so does this).
 */
function descriptorOf(manifest: Manifest): ExhumedDescriptor {
	const relations = manifest.relations.map(function relationOf(relation): ExhumedRelation {
		const fields = relation.fields.map(function fieldOf(field): ExhumedField {
			return Object.freeze({ name: field.name, valueType: field.valueType })
		})
		let roster: readonly ExhumedAxiom[] | undefined
		if (relation.extension !== undefined) {
			roster = Object.freeze(
				relation.extension.map(function axiomOf(row): ExhumedAxiom {
					const values: Record<string, FactValue> = {}
					for (const cell of row.values) {
						values[cell.name] = cell.value
					}
					return Object.freeze({ handle: row.handle, id: row.id, values: Object.freeze(values) })
				})
			)
		}
		return Object.freeze({ name: relation.name, fields: Object.freeze(fields), roster })
	})
	return Object.freeze({ relations: Object.freeze(relations) })
}

/**
 * Opens one exhumed store at an ALREADY-CANONICAL path (`Db.exhume` is the
 * public spelling — the path law lives in db.ts beside `open`/`create`).
 * The bridge's three domain refusals become the typed error constants
 * ({@link ErrExhumeNoDescriptor}, {@link ErrExhumeFormatMismatch},
 * {@link ErrExhumeCorruption}), each carrying the engine's message in its
 * wrap. The returned value is NOT cached: exhume is a forensic read, and
 * caching would pin the store's exclusive lock for the process's life —
 * GC reclamation is the whole lifecycle.
 */
function exhumeStore(canonical: string): Exhumed {
	const outcome = bridged(`exhume bumbledb store at ${canonical}`, function callExhume() {
		return native.dbExhume(canonical)
	})
	if (!outcome.ok) {
		switch (outcome.kind) {
			case "descriptorMissing":
				throw errors.wrap(ErrExhumeNoDescriptor, `exhume ${canonical}: ${outcome.message}`)
			case "formatMismatch":
				throw errors.wrap(ErrExhumeFormatMismatch, `exhume ${canonical}: ${outcome.message}`)
			case "corruption":
				throw errors.wrap(ErrExhumeCorruption, `exhume ${canonical}: ${outcome.message}`)
		}
	}
	const handle = outcome.exhume
	const manifest = bridged("read bumbledb exhumed descriptor", function callDescriptor() {
		return native.exhumeDescriptor(handle)
	})
	const descriptor = descriptorOf(manifest)
	const fieldNames = new Map<string, readonly string[]>()
	for (const relation of descriptor.relations) {
		fieldNames.set(
			relation.name,
			relation.fields.map(function nameOf(field) {
				return field.name
			})
		)
	}
	function scan(relation: string): readonly ExhumedFact[] {
		const names = fieldNames.get(relation)
		if (names === undefined) {
			throw errors.new(`bumbledb exhume: the store's descriptor declares no relation ${relation}`)
		}
		const rows = bridged(`scan bumbledb exhumed relation ${relation}`, function callScan() {
			return native.exhumeScan(handle, relation)
		})
		return Object.freeze(
			rows.map(function factOf(row): ExhumedFact {
				const fact: Record<string, FactValue> = {}
				names.forEach(function pair(name, index) {
					const cell = row[index]
					if (cell === undefined) {
						throw errors.new(
							`bumbledb exhume drift: relation ${relation} row has no value at position ${index} (${name})`
						)
					}
					fact[name] = cell
				})
				return Object.freeze(fact)
			})
		)
	}
	return Object.freeze({ descriptor, scan })
}

export type { Exhumed, ExhumedAxiom, ExhumedDescriptor, ExhumedFact, ExhumedField, ExhumedRelation }
export { ErrExhumeCorruption, ErrExhumeFormatMismatch, ErrExhumeNoDescriptor, exhumeStore }
