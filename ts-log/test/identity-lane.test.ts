/**
 * The FFI identity lane (one-core doc 40 §4): every refusal row of every
 * family the bridge crosses — batch decode, batch encode, manifest,
 * checkpoint, sidecar — is forced through the real bridge from Node and
 * asserted at the ts-log seat: the `ErrRefused` sentinel carries the
 * core's own identity kind, spelled exactly as the generated table
 * (`conformance/v3/identities.json`) spells it. One row, one test,
 * keyed off the table itself, so a new core variant lands red here
 * until its hostile input is written. Rows unconstructible from TS by
 * design are asserted as unconstructible under the ruling that pins
 * them, never skipped. Hostile bytes reuse the conformance corpus where
 * a family carries the fixture; the rest are minimal constructions.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import type { FactValue, LiteralSpec, SchemaSpec, StatementSpec, ValueSpec, ValueTypeSpec } from "@bjornpagen/bumbledb"
import { internalLogEncodeBatch } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { digest32, fromHex, toHex } from "#bytes.ts"
import { parseSidecar } from "#chain.ts"
import type { EncodeHeader } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import type { Braid, Descriptor } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import { ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import type { CheckpointFacts, CheckpointHead } from "#manifest.ts"
import { parseCheckpoint, parseManifest, renderCheckpoint } from "#manifest.ts"
import { assembleFromSpec } from "#test/assemble.ts"

const corpusRoot = path.resolve(import.meta.dirname, "../../crates/bumbledb-log/conformance/v3")

/** The generated identity table: one array per boundary enum, refusal
 *  kinds spelled by each core enum's `identity()`, outcome arms spelled
 *  as the tags the tagged hosts narrow. */
interface IdentityTable {
	readonly comment: string
	readonly batchDecode: readonly string[]
	readonly batchEncode: readonly string[]
	readonly manifest: readonly string[]
	readonly checkpoint: readonly string[]
	readonly sidecar: readonly string[]
	readonly counter: readonly string[]
	readonly admission: readonly string[]
	readonly waited: readonly string[]
	readonly refreshOutcome: readonly string[]
}

const table = JSON.parse(fs.readFileSync(path.join(corpusRoot, "identities.json"), "utf8")) as IdentityTable

/** The refusal families the bridge crosses; each has a cover lane below. */
const BRIDGED_FAMILIES = ["batchDecode", "batchEncode", "manifest", "checkpoint", "sidecar"] as const

/** Families the bridge never crosses: `counter` is the id-lease refusal
 *  family each driver mints host-side (the counter body is the canonical
 *  decimal, spoken natively by both drivers, never bridged);
 *  `admission`/`waited`/`refreshOutcome` are outcome-arm tags the tagged
 *  hosts narrow — outcomes, not refusal crossings. errors.test.ts locks
 *  those rosters arm for arm; this lane owns the bridged crossings. */
const HOST_FAMILIES = ["counter", "admission", "waited", "refreshOutcome"] as const

// ---------------------------------------------------------------------------
// Corpus schema assembly: the shared schemas.json roster, walked into
// engine SchemaSpecs and sealed through the shadow sealer — the corpus
// pins codecs and braid maps on shapes the engine seal refuses.
// ---------------------------------------------------------------------------

type CorpusValue = Record<string, unknown>

interface CorpusField {
	readonly name: string
	readonly type: unknown
	readonly generation?: string
}

interface CorpusRelation {
	readonly name: string
	readonly fields: readonly CorpusField[]
	readonly extension?: ReadonlyArray<{ readonly handle: string; readonly values: readonly CorpusValue[] }>
}

interface CorpusSide {
	readonly relation: number
	readonly projection: readonly number[]
	readonly selection?: ReadonlyArray<readonly [number, readonly CorpusValue[]]>
}

interface CorpusSchema {
	readonly relations: readonly CorpusRelation[]
	readonly statements: readonly Record<string, unknown>[]
}

const schemasRaw = JSON.parse(fs.readFileSync(path.join(corpusRoot, "schemas.json"), "utf8")) as {
	schemas: Record<string, CorpusSchema>
}
const assembled = new Map<string, Descriptor>()

function typeOf(raw: unknown): ValueTypeSpec {
	if (raw === "bool" || raw === "u64" || raw === "i64" || raw === "string") {
		return { kind: raw }
	}
	const record = raw as Record<string, unknown>
	if (typeof record.fixedBytes === "number") {
		return { kind: "fixedBytes", len: record.fixedBytes }
	}
	if (record.interval === "u64" || record.interval === "i64") {
		return { kind: "interval", element: record.interval, width: undefined }
	}
	const fixed = record.fixedInterval as { element: "u64" | "i64"; width: string } | undefined
	if (fixed !== undefined) {
		return { kind: "interval", element: fixed.element, width: BigInt(fixed.width) }
	}
	throw errors.new(`corpus type unreadable: ${JSON.stringify(raw)}`)
}

function valueSpecOf(raw: CorpusValue): ValueSpec {
	if (typeof raw.bool === "boolean") {
		return { kind: "bool", value: raw.bool }
	}
	if (typeof raw.u64 === "string") {
		return { kind: "u64", value: BigInt(raw.u64) }
	}
	if (typeof raw.i64 === "string") {
		return { kind: "i64", value: BigInt(raw.i64) }
	}
	if (typeof raw.string === "string") {
		return { kind: "string", value: raw.string }
	}
	if (typeof raw.fixedBytes === "string") {
		return { kind: "fixedBytes", value: fromHex(raw.fixedBytes) }
	}
	const u = raw.intervalU64 as readonly [string, string] | undefined
	if (u !== undefined) {
		return { kind: "intervalU64", start: BigInt(u[0]), end: BigInt(u[1]) }
	}
	const i = raw.intervalI64 as readonly [string, string] | undefined
	if (i !== undefined) {
		return { kind: "intervalI64", start: BigInt(i[0]), end: BigInt(i[1]) }
	}
	throw errors.new(`corpus value unreadable: ${JSON.stringify(raw)}`)
}

function specOf(corpus: CorpusSchema): SchemaSpec {
	function sealedName(relation: CorpusRelation, ordinal: number): string {
		const sealed = relation.extension === undefined ? relation.fields : [{ name: "id" }, ...relation.fields]
		const field = sealed[ordinal]
		if (field === undefined) {
			throw errors.new(`corpus relation ${relation.name} has no sealed field ${ordinal}`)
		}
		return field.name
	}
	function relationNamed(id: number): CorpusRelation {
		const relation = corpus.relations[id]
		if (relation === undefined) {
			throw errors.new(`corpus cites unknown relation ${id}`)
		}
		return relation
	}
	function sideOf(raw: CorpusSide) {
		const relation = relationNamed(raw.relation)
		return {
			relation: relation.name,
			projection: raw.projection.map(function nameOf(ordinal) {
				return sealedName(relation, ordinal)
			}),
			selection: (raw.selection ?? []).map(function bindingOf(binding) {
				const literals = binding[1].map(function literalOf(value): LiteralSpec {
					return { kind: "value", value: valueSpecOf(value) }
				})
				const first = literals[0]
				if (literals.length === 1 && first !== undefined) {
					return [sealedName(relation, binding[0]), { kind: "one", literal: first }] as const
				}
				return [sealedName(relation, binding[0]), { kind: "many", literals }] as const
			})
		}
	}
	const statements: StatementSpec[] = corpus.statements.map(function statementOf(raw): StatementSpec {
		const fd = raw.functionality as { relation: number; projection: readonly number[] } | undefined
		if (fd !== undefined) {
			const relation = relationNamed(fd.relation)
			return {
				kind: "fd",
				relation: relation.name,
				projection: fd.projection.map(function nameOf(ordinal) {
					return sealedName(relation, ordinal)
				})
			}
		}
		const containment = raw.containment as { source: CorpusSide; target: CorpusSide } | undefined
		if (containment !== undefined) {
			return {
				kind: "containment",
				source: sideOf(containment.source),
				target: sideOf(containment.target),
				bidirectional: false
			}
		}
		const capacity = raw.capacity as
			| {
					target: CorpusSide
					weight: unknown
					lo: string
					hi?: Record<string, unknown>
					source: CorpusSide
			  }
			| undefined
		if (capacity !== undefined) {
			const sourceRelation = relationNamed(capacity.source.relation)
			const targetRelation = relationNamed(capacity.target.relation)
			let weight: { kind: "unit" } | { kind: "field"; field: string } | { kind: "durationField"; field: string } = {
				kind: "unit"
			}
			if (typeof capacity.weight === "object" && capacity.weight !== null) {
				const record = capacity.weight as Record<string, number>
				if (typeof record.field === "number") {
					weight = { kind: "field", field: sealedName(sourceRelation, record.field) }
				}
				if (typeof record.durationOf === "number") {
					weight = { kind: "durationField", field: sealedName(sourceRelation, record.durationOf) }
				}
			}
			const lo = { kind: "lit", value: BigInt(capacity.lo) } as const
			let window: Extract<StatementSpec, { kind: "capacity" }>["window"]
			if (capacity.hi === undefined) {
				window = { kind: "floor", lo }
			} else if (typeof capacity.hi.lit === "string") {
				window = { kind: "range", lo, hi: { kind: "lit", value: BigInt(capacity.hi.lit) } }
			} else if (typeof capacity.hi.targetField === "number") {
				window = {
					kind: "range",
					lo,
					hi: { kind: "field", field: sealedName(targetRelation, capacity.hi.targetField) }
				}
			} else if (typeof capacity.hi.targetDuration === "number") {
				window = {
					kind: "range",
					lo,
					hi: { kind: "durationField", field: sealedName(targetRelation, capacity.hi.targetDuration) }
				}
			} else {
				throw errors.new(`corpus capacity hi unreadable: ${JSON.stringify(capacity.hi)}`)
			}
			return { kind: "capacity", target: sideOf(capacity.target), weight, window, source: sideOf(capacity.source) }
		}
		throw errors.new(`corpus statement unreadable: ${JSON.stringify(raw)}`)
	})
	return {
		relations: corpus.relations.map(function relationSpecOf(relation) {
			return {
				name: relation.name,
				fields: relation.fields.map(function fieldSpecOf(field) {
					return {
						name: field.name,
						valueType: typeOf(field.type),
						newtype: undefined,
						fresh: field.generation === "fresh"
					}
				}),
				closed:
					relation.extension === undefined
						? undefined
						: {
								newtype: `${relation.name}.id`,
								rows: relation.extension.map(function rowOf(row) {
									return {
										handle: row.handle,
										values: row.values.map(function literalOf(value): LiteralSpec {
											return { kind: "value", value: valueSpecOf(value) }
										})
									}
								})
							}
			}
		}),
		statements
	}
}

function schemaNamed(name: string): Descriptor {
	const hit = assembled.get(name)
	if (hit !== undefined) {
		return hit
	}
	const corpus = schemasRaw.schemas[name]
	if (corpus === undefined) {
		throw errors.new(`lane cites schema ${name}`)
	}
	const descriptor = assembleFromSpec(specOf(corpus))
	assembled.set(name, descriptor)
	return descriptor
}

// ---------------------------------------------------------------------------
// The lane itself.
// ---------------------------------------------------------------------------

/** One table row's forcing function: runs the hostile input and asserts
 *  the crossed identity is the row. */
type Cover = (row: string) => void

interface RefusalSidecar {
	readonly expect: string
	readonly refusal?: string
	readonly schema?: string
}

/** Reads one conformance refusal golden and pins the fixture's own named
 *  refusal to the table row, so a re-purposed fixture fails the lane
 *  instead of silently testing the wrong offense. */
function refusalFixture(rel: string, row: string): { readonly bytes: Uint8Array; readonly schema: string | undefined } {
	const sidecar = JSON.parse(fs.readFileSync(path.join(corpusRoot, `${rel}.json`), "utf8")) as RefusalSidecar
	assert.equal(sidecar.refusal, row, `${rel}: the fixture's named refusal is the table row`)
	return {
		bytes: new Uint8Array(fs.readFileSync(path.join(corpusRoot, `${rel}.bin`))),
		schema: sidecar.schema
	}
}

/** Forces the seat call and returns the crossed identity kind off the
 *  `ErrRefused` sentinel's cause — the one shape a bridge refusal wears
 *  on this side. */
function seatKind(force: () => unknown): string {
	const ran = errors.trySync(force)
	assert.ok(ran.error, "the hostile input refuses")
	assert.ok(errors.is(ran.error, ErrRefused), `the seat's sentinel is ErrRefused, got: ${ran.error.message}`)
	const cause = refusalOf(ran.error)
	assert.ok(cause !== undefined, "the sentinel carries its cause")
	return cause.kind
}

function firstBraid(descriptor: Descriptor): Braid {
	const first = descriptor.braidMembers.keys().next()
	assert.ok(!first.done, "the theory decomposes into at least one braid")
	return first.value
}

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

/** A valid encode header for the given braid. The header carries no
 *  fingerprint — the sealed handle is the wire's fingerprint authority. */
function encodeHeaderOf(id: Braid): EncodeHeader {
	return {
		braid: id,
		braidGen: generation(1n),
		prev: new Uint8Array(32),
		writer: 1n,
		timestamp: 1n
	}
}

function fixtureDecodeCover(rel: string): Cover {
	return function force(row) {
		const { bytes, schema } = refusalFixture(rel, row)
		assert.ok(schema !== undefined, `${rel}: the batch fixture names its schema`)
		const descriptor = schemaNamed(schema)
		// The corpus pins each schema's own fingerprint in the wire's
		// bytes 8..40, and the shadow sealer mints the lane's codec under
		// the zero fingerprint, so the lane re-seats that region to the
		// handle's own before decode — theory identity, never the row's
		// offense. The FingerprintMismatch row keeps its bytes: that
		// region IS its offense, foreign under any handle.
		const hostile = new Uint8Array(bytes)
		if (row !== "FingerprintMismatch" && hostile.length >= 40) {
			hostile.set(descriptor.fingerprintBytes, 8)
		}
		const kind = seatKind(function decodeIt() {
			return decodeBatch(descriptor, hostile)
		})
		assert.equal(kind, row, `${rel}: the crossed kind is the table row`)
	}
}

function fixtureManifestCover(rel: string): Cover {
	return function force(row) {
		const { bytes } = refusalFixture(rel, row)
		const kind = seatKind(function parseIt() {
			return parseManifest(bytes)
		})
		assert.equal(kind, row, `${rel}: the crossed kind is the table row`)
	}
}

function fixtureCheckpointCover(rel: string): Cover {
	return function force(row) {
		const { bytes, schema } = refusalFixture(rel, row)
		assert.ok(schema !== undefined, `${rel}: the checkpoint fixture names its schema`)
		const kind = seatKind(function parseIt() {
			return parseCheckpoint(schemaNamed(schema).codec, bytes)
		})
		assert.equal(kind, row, `${rel}: the crossed kind is the table row`)
	}
}

function fixtureSidecarCover(rel: string): Cover {
	return function force(row) {
		const { bytes, schema } = refusalFixture(rel, row)
		assert.ok(schema !== undefined, `${rel}: the sidecar fixture names its schema`)
		const kind = seatKind(function parseIt() {
			return parseSidecar(schemaNamed(schema).codec, bytes)
		})
		assert.equal(kind, row, `${rel}: the crossed kind is the table row`)
	}
}

const batchDecodeCovers: Readonly<Record<string, Cover>> = {
	Truncated: fixtureDecodeCover("batch/r_truncated_row"),
	BadMagic: fixtureDecodeCover("batch/r_bad_magic"),
	Version: fixtureDecodeCover("batch/r_version_1"),
	Flags: fixtureDecodeCover("batch/r_flags_nonzero"),
	FingerprintMismatch: fixtureDecodeCover("batch/r_fingerprint_mismatch"),
	UnknownBraid: fixtureDecodeCover("batch/r_unknown_braid"),
	UnknownOpKind: fixtureDecodeCover("batch/r_op_kind_unknown"),
	UnknownRelation: fixtureDecodeCover("batch/r_unknown_relation"),
	ClosedRelation: fixtureDecodeCover("batch/r_closed_relation"),
	OpRelationOutsideBraid: fixtureDecodeCover("batch/r_relation_outside_braid"),
	TagMismatch: fixtureDecodeCover("batch/r_tag_mismatch"),
	BoolByte: fixtureDecodeCover("batch/r_bool_byte_2"),
	InvalidUtf8: fixtureDecodeCover("batch/r_string_bad_utf8"),
	EmptyInterval: fixtureDecodeCover("batch/r_interval_empty"),
	IntervalOverflow: fixtureDecodeCover("batch/r_fixed_interval_overflow"),
	TrailingBytes: fixtureDecodeCover("batch/r_trailing_byte")
}

const batchEncodeCovers: Readonly<Record<string, Cover>> = {
	/** UNCONSTRUCTIBLE from TS by design — the S2 ruling "the handle is
	 *  the fingerprint authority": the batch-header bridge crossing
	 *  carries no fingerprint slot in either direction, and encode fills
	 *  the wire's fingerprint from the sealed handle, so no input from
	 *  this side reaches the core with a foreign fingerprint. The proof
	 *  is the wire itself: the fingerprint region of a freshly encoded
	 *  batch is the handle's own, whatever the caller crossed. */
	FingerprintMismatch: function unconstructible() {
		const descriptor = schemaNamed("kitchen")
		const bytes = encodeBatch(descriptor, encodeHeaderOf(firstBraid(descriptor)), [])
		assert.equal(
			toHex(bytes.subarray(8, 40)),
			descriptor.fingerprint,
			"the wire fingerprint is the sealed handle's own"
		)
	},
	UnknownBraid: function force(row) {
		const descriptor = schemaNamed("kitchen")
		const kind = seatKind(function encodeIt() {
			// A braid id the decomposition does not mint; the bridge mints
			// the refusal as the twin of the core's encode refusal.
			return encodeBatch(descriptor, encodeHeaderOf(braidHex(0x0ffffff0)), [])
		})
		assert.equal(kind, row, "the foreign header braid crosses as the table row")
	},
	UnknownRelation: function force(row) {
		// The seat's vocabulary gate resolves relation NAMES to sealed
		// ids, so the offense is spelled at the bridge itself: a relation
		// id the descriptor does not mint.
		const descriptor = schemaNamed("kitchen")
		const raw = internalLogEncodeBatch(
			descriptor.codec,
			{
				braid: braidIdOf(firstBraid(descriptor)),
				braidGen: 1n,
				prev: new Uint8Array(32),
				writer: 1n,
				timestamp: 1n
			},
			[{ kind: "insert", relation: 200, rows: [] }]
		)
		assert.ok(!raw.ok, "the bridge refuses the unminted relation id")
		assert.equal(raw.kind, row, "the bridge spells the table row")
		// The seat never mints this row: an unknown name refuses at the
		// vocabulary gate, host-side, before any bridge crossing.
		const ran = errors.trySync(function encodeIt() {
			return encodeBatch(descriptor, encodeHeaderOf(firstBraid(descriptor)), [
				{ op: "insert", relation: "Ghost", rows: [] }
			])
		})
		assert.ok(ran.error, "the seat's vocabulary gate refuses the unknown name")
		assert.equal(errors.is(ran.error, ErrRefused), false, "a host gate, not a bridge refusal")
	},
	ClosedRelation: function force(row) {
		const descriptor = schemaNamed("multi")
		const closed = descriptor.relations.find(function closedOf(relation) {
			return relation.closed
		})
		assert.ok(closed !== undefined, "multi carries a closed relation")
		const kind = seatKind(function encodeIt() {
			return encodeBatch(descriptor, encodeHeaderOf(firstBraid(descriptor)), [
				{ op: "insert", relation: closed.name, rows: [] }
			])
		})
		assert.equal(kind, row, "an op citing the closed roster crosses as the table row")
	},
	OpRelationOutsideBraid: function force(row) {
		const descriptor = schemaNamed("booking")
		const braids = [...descriptor.braidMembers.entries()]
		const home = braids[0]
		const away = braids[1]
		assert.ok(home !== undefined && away !== undefined, "booking decomposes into at least two braids")
		const foreignId = away[1][0]
		assert.ok(foreignId !== undefined, "the away braid has a member")
		const foreign = descriptor.relations.find(function memberOf(relation) {
			return relation.id === foreignId
		})
		assert.ok(foreign !== undefined, "the away member is a descriptor relation")
		const kind = seatKind(function encodeIt() {
			return encodeBatch(descriptor, encodeHeaderOf(home[0]), [{ op: "insert", relation: foreign.name, rows: [] }])
		})
		assert.equal(kind, row, "an op outside the header braid crosses as the table row")
	},
	Arity: function force(row) {
		// A SHORT row: the seat's own Arity gate judges only a cell past
		// the layout, so the one-cell row against Sample's nine-field
		// layout crosses whole and the refusal is the core's own.
		const descriptor = schemaNamed("kitchen")
		const kind = seatKind(function encodeIt() {
			return encodeBatch(descriptor, encodeHeaderOf(firstBraid(descriptor)), [
				{ op: "insert", relation: "Sample", rows: [[true]] }
			])
		})
		assert.equal(kind, row, "the short row crosses as the table row")
	},
	Value: function force(row) {
		// A two-byte cell against Sample's three-byte `code` layout: the
		// seat tags the JS shape and the core is the one judge of width.
		const descriptor = schemaNamed("kitchen")
		const cells: readonly FactValue[] = [
			true,
			1n,
			-1n,
			"x",
			new Uint8Array(2),
			{ start: 1n, end: 2n },
			{ start: -1n, end: 1n },
			{ start: 0n, end: 5n },
			{ start: -2n, end: 3n }
		]
		const kind = seatKind(function encodeIt() {
			return encodeBatch(descriptor, encodeHeaderOf(firstBraid(descriptor)), [
				{ op: "insert", relation: "Sample", rows: [cells] }
			])
		})
		assert.equal(kind, row, "the short fixedBytes cell crosses as the table row")
	},
	/** UNCONSTRUCTIBLE from TS: the wire op count is u32 and the
	 *  ECMAScript array length ceiling is 2^32 - 1 — u32::MAX exactly —
	 *  so an op roster past the cap has no array spelling on this side.
	 *  The proof is the host's own ceiling. */
	TooManyOps: function unconstructible() {
		assert.throws(
			function overflowIt() {
				return new Array(4294967296)
			},
			RangeError,
			"an op roster past u32::MAX has no ECMAScript array spelling"
		)
	},
	/** UNCONSTRUCTIBLE from TS: the wire row count per op is u32 and the
	 *  ECMAScript array length ceiling is 2^32 - 1 — u32::MAX exactly —
	 *  so a row roster past the cap has no array spelling on this side. */
	TooManyRows: function unconstructible() {
		assert.throws(
			function overflowIt() {
				return new Array(4294967296)
			},
			RangeError,
			"a row roster past u32::MAX has no ECMAScript array spelling"
		)
	}
}

const manifestCovers: Readonly<Record<string, Cover>> = {
	Malformed: fixtureManifestCover("documents/manifest/r_truncated"),
	Version: fixtureManifestCover("documents/manifest/r_version_2")
}

const checkpointCovers: Readonly<Record<string, Cover>> = {
	Malformed: fixtureCheckpointCover("documents/checkpoint/r_truncated"),
	Version: fixtureCheckpointCover("documents/checkpoint/r_version_2"),
	Overflow: fixtureCheckpointCover("documents/checkpoint/r_overflow"),
	UnknownBraid: fixtureCheckpointCover("documents/checkpoint/r_unknown_braid"),
	BraidSet: function force(row) {
		// Constructed, not fixture-backed: both braid sets are pure
		// functions of the same schema, so the corpus carries no
		// braid-set drift golden. A checkpoint carrying a strict subset
		// of the derived set — every id minted, ascending — is the
		// minimal hostile document.
		const descriptor = schemaNamed("booking")
		const braids = [...descriptor.braidMembers.keys()]
		assert.ok(braids.length >= 2, "booking decomposes into at least two braids")
		const subset = new Map<Braid, CheckpointHead>()
		for (const id of braids.slice(0, -1)) {
			subset.set(id, { g: generation(1n), hash: digest32(new Uint8Array(32)), ts: 1n })
		}
		const facts: CheckpointFacts = {
			braids: subset,
			catalog: digest32(new Uint8Array(32)),
			writer: 1n,
			prev: null,
			sum: 1n
		}
		const bytes = renderCheckpoint(descriptor.codec, facts)
		const kind = seatKind(function parseIt() {
			return parseCheckpoint(descriptor.codec, bytes)
		})
		assert.equal(kind, row, "the drifted braid set crosses as the table row")
	}
}

const sidecarCovers: Readonly<Record<string, Cover>> = {
	Malformed: fixtureSidecarCover("documents/sidecar/r_truncated"),
	Version: fixtureSidecarCover("documents/sidecar/r_version_2"),
	UnknownBraid: fixtureSidecarCover("documents/sidecar/r_unknown_braid"),
	Overflow: fixtureSidecarCover("documents/sidecar/r_overflow")
}

/** One lane per bridged family: the roster test holds covers and table
 *  rows to the same set (a new core identity is red until its hostile
 *  input lands; a cover whose row left the core is a ghost and dies),
 *  then one test per row runs its forcing. */
function familyLane(family: string, rows: readonly string[], covers: Readonly<Record<string, Cover>>): void {
	describe(`identity lane: ${family}`, function suite() {
		test(`${family}: one cover per table row, no ghosts`, function roster() {
			assert.deepEqual(
				[...Object.keys(covers)].sort(),
				[...rows].sort(),
				`${family}: the cover set is the table's row set`
			)
		})
		for (const row of rows) {
			test(`${family}/${row}`, function lane() {
				const cover = covers[row]
				assert.ok(cover !== undefined, `${family}/${row}: a new core identity is red until its hostile input lands`)
				cover(row)
			})
		}
	})
}

describe("identity lane: the family roster", function suite() {
	test("every table family is placed: bridged or host-side", function placement() {
		const families = Object.keys(table)
			.filter(function rowFamilies(key) {
				return key !== "comment"
			})
			.sort()
		assert.deepEqual(
			families,
			[...BRIDGED_FAMILIES, ...HOST_FAMILIES].sort(),
			"a new identity family is red here until it is placed"
		)
	})
})

familyLane("batchDecode", table.batchDecode, batchDecodeCovers)
familyLane("batchEncode", table.batchEncode, batchEncodeCovers)
familyLane("manifest", table.manifest, manifestCovers)
familyLane("checkpoint", table.checkpoint, checkpointCovers)
familyLane("sidecar", table.sidecar, sidecarCovers)
