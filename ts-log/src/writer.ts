/**
 * The writer (60): a replica plus the right to create log objects. Role
 * is a field on the handle; this handle births the store. One commit
 * path and one loss path: a lost slot's byte-equal occupant is an
 * ambiguous PUT absorbed; anything else discards the local directory,
 * re-opens through the replica to the current tip, and re-judges the
 * recorded ops once — the verdict IS a serial execution, performed.
 * Each loop iteration races once at the then-tip, so a historical loss
 * is structurally uncountable, and bounded live-tip losses surface as
 * ErrContention carrying the terminal re-judgment's own violation or
 * the racing tip.
 */

import * as crypto from "node:crypto"
import {
	type Fact,
	type FreshKeys,
	internalBlake3,
	type MemberRelation,
	type SchemaRelations,
	type Violation
} from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import type { Digest32 } from "#bytes.ts"
import { bytesEqual, checkedAddU64, digest32, digest32FromHex, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import type { Braid, RelationInfo } from "#descriptor.ts"
import { ErrSpanningCommit, refuseExhausted, refuseOverWidth, throwContention } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation, idsKey, logKey, manifestKey } from "#keys.ts"
import { renderManifest } from "#manifest.ts"
import type { Core, Replica } from "#replica.ts"
import {
	applyOps,
	chainEntry,
	clearPending,
	coreOf,
	discardAndReopen,
	generationOf,
	maxBigint,
	persistSidecar,
	readdressPending,
	withGate
} from "#replica.ts"
import type { Value } from "#value.ts"
import { checkAgainst } from "#value.ts"

/** 10 owns the width: one CAS amortizes counter traffic 4096× below slot traffic. */
const LEASE_WIDTH = 4096n

/** The live-loss bound (60): consecutive losses at the live tip, history never counts. */
const LOSS_BOUND = 16

/** Writer id in the fixed-layout header: magic + version + flags + fingerprint + braid + braid_gen + prev. */
const WRITER_AT = 4 + 2 + 2 + 32 + 4 + 8 + 32

const BATCH_MAGIC = utf8Encoder.encode("BDBL")

type Durability = "published" | "local-pending"

type Commit<Rels extends SchemaRelations, R> =
	| {
			readonly tag: "accepted"
			readonly value: R
			readonly braid: Braid
			readonly generation: Generation
			readonly durability: Durability
	  }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }

type BraidOutcome<Rels extends SchemaRelations> =
	| {
			readonly tag: "accepted"
			readonly braid: Braid
			readonly generation: Generation
			readonly durability: Durability
	  }
	| { readonly tag: "rejected"; readonly braid: Braid; readonly violations: readonly Violation<Rels>[] }

interface CommitSplit<Rels extends SchemaRelations, R> {
	readonly value: R
	readonly outcomes: readonly BraidOutcome<Rels>[]
}

/**
 * The recorder: typed inserts and deletes as raw-valued ops, `reserve`
 * drawing on the id lease (10) — reservations never appear in the log;
 * the resulting inserts carry concrete values. Pure and synchronous.
 */
interface Batch<Rels extends SchemaRelations> {
	insert<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	delete<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	reserve<Rel extends MemberRelation<Rels>>(
		relation: Rel,
		field: FreshKeys<Rel> & string,
		count: bigint
	): readonly bigint[]
}

/** Ownership of a slot, read from the winner's header. */
interface Deposition {
	readonly braid: Braid
	readonly slot: Generation
	readonly resident: bigint
	readonly usurper: bigint
}

interface Writer<Rels extends SchemaRelations> {
	readonly role: "writer"
	deposition(): Deposition | null
	commit<R>(body: (batch: Batch<Rels>) => R | Promise<R>): Promise<Commit<Rels, R>>
	commitSplit<R>(body: (batch: Batch<Rels>) => R | Promise<R>): Promise<CommitSplit<Rels, R>>
}

interface LeaseRange {
	next: bigint
	readonly end: bigint
}

/** The scream of an unbounded repair loop: the set of recent signatures,
 *  a warning every eighth attempt, and an alarm the moment any
 *  signature recurs. */
interface Scream {
	attempt(signature: string): void
}

function screamOf(context: string): Scream {
	const seen = new Set<string>()
	let attempts = 0
	return {
		attempt(signature) {
			attempts += 1
			if (seen.has(signature)) {
				console.error(`bumbledb-log alarm: ${context} repair signature recurs: ${signature}`)
			} else {
				seen.add(signature)
			}
			if (attempts % 8 === 0) {
				console.error(`bumbledb-log warning: ${context} repair attempt ${attempts}: ${signature}`)
			}
		}
	}
}

interface WriterState {
	readonly writerId: bigint
	readonly pools: Map<string, LeaseRange[]>
	readonly scream: Scream
	deposition: Deposition | null
}

function poolKey(relation: number, field: number): string {
	return `${relation}:${field}`
}

function remainingOf(state: WriterState, relation: number, field: number): bigint {
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	let remaining = 0n
	for (const range of pool) {
		remaining += range.end - range.next
	}
	return remaining
}

function drawIds(state: WriterState, relation: number, field: number, count: bigint): bigint[] {
	if (count < 0n) {
		throw errors.new(`id-lease count is unsigned: ${count}`)
	}
	if (count > LEASE_WIDTH) {
		refuseOverWidth({ requested: count }, `id-lease draw ${count} exceeds the lease width ${LEASE_WIDTH}`)
	}
	if (count === 0n) {
		return []
	}
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	const range = pool[0]
	if (range === undefined || range.end - range.next < count) {
		refuseExhausted(
			{ relation, field },
			`id-lease relation ${relation} field ${field} cannot draw ${count} from the cached block`
		)
	}
	if (checkedAddU64(range.next, count) === undefined) {
		refuseExhausted({ relation, field }, `id-lease relation ${relation} field ${field} would leave u64`)
	}
	const ids: bigint[] = []
	let remaining = count
	while (remaining > 0n && range.next < range.end) {
		ids.push(range.next)
		range.next += 1n
		remaining -= 1n
	}
	if (range.next >= range.end) {
		pool.shift()
	}
	state.pools.set(poolKey(relation, field), pool)
	return ids
}

/** `ids/{relation}/{field}`: birth claims [0, width); every later lease CAS-increments. */
async function acquireLease<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	relation: number,
	field: number
): Promise<void> {
	const key = idsKey(core.prefix, relation, field)
	for (;;) {
		const fetched = await core.store.get(key)
		if (fetched === null) {
			const created = await core.store.putCreate(key, utf8Encoder.encode(String(LEASE_WIDTH)))
			if (created.tag === "created") {
				pushRange(state, relation, field, 0n, LEASE_WIDTH)
				return
			}
			continue
		}
		const body = utf8StrictDecoder.decode(fetched.bytes)
		if (!/^\d+$/.test(body)) {
			throw errors.new(`id-lease counter ${key} is not a canonical decimal: ${body}`)
		}
		const next = BigInt(body)
		const end = checkedAddU64(next, LEASE_WIDTH)
		if (end === undefined) {
			refuseExhausted({ relation, field }, `id-lease relation ${relation} field ${field} would leave u64`)
		}
		const swapped = await core.store.putSwap(key, utf8Encoder.encode(String(end)), fetched.etag)
		if (swapped.tag === "swapped") {
			pushRange(state, relation, field, next, end)
			return
		}
	}
}

function pushRange(state: WriterState, relation: number, field: number, next: bigint, end: bigint): void {
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	pool.push({ next, end })
	state.pools.set(poolKey(relation, field), pool)
}

async function ensureFreshLeases<Rels extends SchemaRelations>(core: Core<Rels>, state: WriterState): Promise<void> {
	for (const info of core.descriptor.relations) {
		if (info.closed) {
			continue
		}
		for (const [ordinal, field] of info.fields.entries()) {
			if (field.fresh && remainingOf(state, info.id, ordinal) === 0n) {
				await acquireLease(core, state, info.id, ordinal)
			}
		}
	}
}

interface Recording {
	readonly ops: Op[]
}

function lowerFact<Rels extends SchemaRelations>(
	core: Core<Rels>,
	info: RelationInfo,
	fact: Record<string, unknown>
): Value[] {
	return info.fields.map(function lowerCell(field) {
		const raw = fact[field.name]
		if (raw === undefined) {
			throw errors.new(`relation ${info.name}: fact is missing field ${field.name}`)
		}
		let value: Value
		if (field.closedRef !== undefined) {
			if (typeof raw !== "string") {
				throw errors.new(`relation ${info.name} field ${field.name}: expected a ${field.closedRef} handle name`)
			}
			const roster = core.descriptor.relationByName.get(field.closedRef)
			const id = roster?.handles.indexOf(raw) ?? -1
			if (id === -1) {
				throw errors.new(`relation ${info.name} field ${field.name}: "${raw}" is not in the ${field.closedRef} roster`)
			}
			value = BigInt(id)
		} else if (typeof raw === "object" && raw !== null && !(raw instanceof Uint8Array)) {
			const interval = raw as { start?: unknown; end?: unknown }
			if (typeof interval.start !== "bigint" || typeof interval.end !== "bigint") {
				throw errors.new(`relation ${info.name} field ${field.name}: expected an interval of bigints`)
			}
			value = { start: interval.start, end: interval.end }
		} else {
			value = raw as Value
		}
		checkAgainst(`relation ${info.name} field ${field.name}`, field.type, value)
		return value
	})
}

function recorderOf<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState
): { batch: Batch<Rels>; recording: Recording } {
	const recording: Recording = { ops: [] }
	function infoOf(name: string): RelationInfo {
		const info = core.descriptor.relationByName.get(name)
		if (info === undefined) {
			throw errors.new(`relation ${name} is not a member of this theory`)
		}
		if (info.closed) {
			throw errors.new(`relation ${name} is closed — sealed rows never change`)
		}
		return info
	}
	function record(op: "insert" | "delete", relationName: string, facts: Iterable<unknown>): void {
		const info = infoOf(relationName)
		const rows: Value[][] = []
		for (const fact of facts) {
			if (typeof fact !== "object" || fact === null) {
				throw errors.new(`relation ${relationName}: a fact is not an object`)
			}
			rows.push(lowerFact(core, info, fact as Record<string, unknown>))
		}
		recording.ops.push({ op, relation: relationName, rows })
	}
	const batch: Batch<Rels> = {
		insert(relation, facts) {
			record("insert", relation.name, facts)
		},
		delete(relation, facts) {
			record("delete", relation.name, facts)
		},
		reserve(relation, field, count) {
			const info = infoOf(relation.name)
			const ordinal = info.fields.findIndex(function byName(candidate) {
				return candidate.name === field
			})
			const declared = info.fields[ordinal]
			if (declared === undefined || !declared.fresh) {
				throw errors.new(`relation ${relation.name}: field ${field} is not a fresh cell`)
			}
			return drawIds(state, info.id, ordinal, count)
		}
	}
	return { batch, recording }
}

/** Runs the recording body exactly once. The body is awaited to
 *  completion before the batch is sealed; reserve draws from the
 *  cached block (OverWidth | Exhausted | Drawn). */
async function recordWithLeases<Rels extends SchemaRelations, R>(
	core: Core<Rels>,
	state: WriterState,
	body: (batch: Batch<Rels>) => R | Promise<R>
): Promise<{ value: R; ops: Op[] }> {
	await ensureFreshLeases(core, state)
	const { batch, recording } = recorderOf(core, state)
	const value = await body(batch)
	return { value, ops: recording.ops }
}

function braidsTouched<Rels extends SchemaRelations>(core: Core<Rels>, ops: readonly Op[]): Map<Braid, Op[]> {
	const partitioned = new Map<Braid, Op[]>()
	for (const op of ops) {
		const info = core.descriptor.relationByName.get(op.relation)
		const braid = info === undefined ? undefined : core.descriptor.braidOfRelation.get(info.id)
		if (braid === undefined) {
			throw errors.new(`relation ${op.relation} belongs to no braid`)
		}
		const bucket = partitioned.get(braid)
		if (bucket === undefined) {
			partitioned.set(braid, [op])
		} else {
			bucket.push(op)
		}
	}
	return partitioned
}

function u64leAt(bytes: Uint8Array, at: number): bigint | undefined {
	if (bytes.length < at + 8) {
		return undefined
	}
	const view = new DataView(bytes.buffer, bytes.byteOffset + at, 8)
	return view.getBigUint64(0, true)
}

/** The usurper is a fact in the header. A body that refuses to decode
 *  does not hide the slot's owner. */
function headerWriter(bytes: Uint8Array): bigint | undefined {
	if (bytes.length < BATCH_MAGIC.length || !bytesEqual(bytes.subarray(0, BATCH_MAGIC.length), BATCH_MAGIC)) {
		return undefined
	}
	return u64leAt(bytes, WRITER_AT)
}

function headerTimestamp(bytes: Uint8Array): bigint | undefined {
	return u64leAt(bytes, WRITER_AT + 8)
}

/** Header prev is 32 bytes. A hex string is parsed; a 32-byte buffer is branded. */
function digestPrev(prev: Digest32 | Uint8Array | string): Digest32 {
	if (typeof prev === "string") {
		return digest32FromHex(prev)
	}
	return digest32(prev)
}

function noteDeposition(state: WriterState, braid: Braid, slot: Generation, winnerBytes: Uint8Array): void {
	if (state.deposition !== null) {
		return
	}
	const usurper = headerWriter(winnerBytes)
	if (usurper === undefined) {
		return
	}
	state.deposition = { braid, slot, resident: state.writerId, usurper }
}

/** The terminal contention scream: the re-judgment's own rejection names
 *  the hot statement and carries the offending facts' raw values; an
 *  accepted-but-outraced terminal loss carries the racing tip. A
 *  rejection without a violation is refused — the empty statement is
 *  unrepresentable. */
function screamContention<Rels extends SchemaRelations>(
	braid: Braid,
	rejudged:
		| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }
		| { readonly tag: "outraced"; readonly tip: Generation }
): never {
	if (rejudged.tag === "rejected") {
		const violation = rejudged.violations[0]
		if (violation === undefined) {
			throw errors.new("a rejection carries at least one violation")
		}
		throwContention(
			{
				braid,
				cause: {
					kind: "hot-key",
					statement: violation.canonical,
					determinants: violation.facts.map(function rawOf(offending) {
						return offending.fact
					})
				}
			},
			`braid ${braid}: ${LOSS_BOUND} consecutive losses at the live tip and the terminal re-judgment rejected`
		)
	}
	throwContention(
		{ braid, cause: { kind: "slot-race", tip: rejudged.tip } },
		`braid ${braid}: ${LOSS_BOUND} consecutive losses at the live tip outraced accepted re-judgments`
	)
}

/**
 * Publishes the applied pending batch: slot CAS, then the one loss path
 * on Exists. A byte-equal occupant is our own ambiguous PUT, absorbed.
 * Anything else carries the pending through a directory discard —
 * re-persisted into the fresh sidecar before any re-judgment, so a
 * crash mid-loss resolves it at the next open — re-opens to the
 * current tip, and re-judges the recorded ops in one db.write: publish
 * on accepted-and-state-changing, Accepted at the current generation
 * on a net no-op (the publish law), or the serial Rejected. Each
 * iteration races once at the then-tip; the bound counts iterations.
 */
async function publishPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	ops: readonly Op[]
): Promise<Commit<Rels, undefined>> {
	let losses = 0
	for (;;) {
		const pending = core.pending
		if (pending === null) {
			throw errors.new("publish reached with no pending batch")
		}
		const braid = pending.braid
		const created = await core.store.putCreate(logKey(core.prefix, braid, pending.gen), pending.bytes)
		let winnerBytes: Uint8Array | null = null
		if (created.tag !== "created") {
			const fetched = await core.store.get(logKey(core.prefix, braid, pending.gen))
			if (fetched === null) {
				state.scream.attempt("slot vanished after create")
				continue
			}
			if (!bytesEqual(fetched.bytes, pending.bytes)) {
				winnerBytes = fetched.bytes
			}
		}
		if (winnerBytes === null) {
			const timestamp = headerTimestamp(pending.bytes)
			if (timestamp === undefined) {
				throw errors.new("pending batch header is shorter than the fixed layout")
			}
			core.chain.set(braid, {
				g: pending.gen,
				prev: digest32(new Uint8Array(internalBlake3(pending.bytes))),
				ts: timestamp
			})
			await clearPending(core)
			return { tag: "accepted", value: undefined, braid, generation: pending.gen, durability: "published" }
		}

		losses += 1
		state.scream.attempt("slot occupant is not ours")
		noteDeposition(state, braid, pending.gen, winnerBytes)
		await discardAndReopen(core)
		const before = generationOf(core)
		const rejudged = applyOps(core, ops)
		if (rejudged.tag === "rejected") {
			await clearPending(core)
			if (losses >= LOSS_BOUND) {
				screamContention(braid, { tag: "rejected", violations: rejudged.violations })
			}
			return { tag: "rejected", violations: rejudged.violations }
		}
		if (rejudged.value.generation === before) {
			await clearPending(core)
			return {
				tag: "accepted",
				value: undefined,
				braid,
				generation: chainEntry(core, braid).g,
				durability: "published"
			}
		}
		const tip = chainEntry(core, braid)
		core.chain.set(braid, { g: tip.g, prev: digestPrev(tip.prev), ts: tip.ts })
		await readdressPending(core, ops, state.writerId)
		if (losses >= LOSS_BOUND) {
			screamContention(braid, { tag: "outraced", tip: chainEntry(core, braid).g })
		}
	}
}

/** The commit discipline (60): encode, fsync the pending, judge locally,
 *  publish only what advanced the generation. */
async function disciplineCommit<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	braid: Braid,
	ops: readonly Op[]
): Promise<Commit<Rels, undefined>> {
	const entry = chainEntry(core, braid)
	const timestamp = maxBigint(BigInt(Date.now()), entry.ts)
	const bytes = encodeBatch(
		core.descriptor,
		{
			fingerprint: digest32FromHex(core.descriptor.fingerprint),
			braid,
			braidGen: generation(entry.g + 1n),
			prev: digestPrev(entry.prev),
			writer: state.writerId,
			timestamp
		},
		ops
	)
	core.pending = { braid, gen: generation(entry.g + 1n), bytes }
	core.pendingOps = ops
	await persistSidecar(core)

	const before = generationOf(core)
	const outcome = applyOps(core, ops)
	if (outcome.tag === "rejected") {
		await clearPending(core)
		return { tag: "rejected", violations: outcome.violations }
	}
	if (outcome.value.generation === before) {
		await clearPending(core)
		return { tag: "accepted", value: undefined, braid, generation: entry.g, durability: "published" }
	}
	return publishPending(core, state, ops)
}

/** An inherited pending is resolved-and-published by open. */
async function settleInheritedPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState
): Promise<void> {
	if (core.pending === null) {
		return
	}
	let ops = core.pendingOps
	if (ops === null) {
		const decoded = errors.trySync(function decodePending() {
			return decodeBatch(core.descriptor, (core.pending as { bytes: Uint8Array }).bytes)
		})
		if (decoded.error) {
			await clearPending(core)
			return
		}
		ops = decoded.data.ops
	}
	const published = await publishPending(core, state, ops)
	if (published.tag === "rejected") {
		return
	}
}

/** Only the writer births a store: create-only PUT of a genesis manifest. */
async function birthStore<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const key = manifestKey(core.prefix)
	for (;;) {
		const fetched = await core.store.get(key)
		if (fetched !== null) {
			if (core.manifestEtag === null) {
				core.manifestEtag = fetched.etag
			}
			return
		}
		const bytes = renderManifest({ fingerprint: core.descriptor.fingerprint, checkpoint: null })
		const created = await core.store.putCreate(key, bytes)
		if (created.tag === "created") {
			core.manifestEtag = created.etag
			core.checkpoint = null
			core.checkpointDigest = null
			return
		}
	}
}

function openWriter<Rels extends SchemaRelations>(replica: Replica<Rels>): Writer<Rels> {
	const core = coreOf(replica)
	const state: WriterState = {
		writerId: crypto.randomBytes(8).readBigUInt64LE(),
		pools: new Map(),
		scream: screamOf("writer discard-and-re-pull"),
		deposition: null
	}
	const opened = withGate(core, async function openTransition() {
		await birthStore(core)
		await ensureFreshLeases(core, state)
		await settleInheritedPending(core, state)
	})
	return {
		role: "writer",
		deposition() {
			return state.deposition
		},
		async commit(body) {
			return withGate(core, async function commitBody() {
				await opened
				const recorded = await recordWithLeases(core, state, body)
				if (recorded.ops.length === 0) {
					throw errors.new("commit recorded no ops — an empty transaction names no braid")
				}
				const partitioned = braidsTouched(core, recorded.ops)
				if (partitioned.size > 1) {
					throw errors.wrap(ErrSpanningCommit, `the recorded ops span braids ${[...partitioned.keys()].join(", ")}`)
				}
				const [braid, ops] = [...partitioned.entries()][0] as [Braid, Op[]]
				const outcome = await disciplineCommit(core, state, braid, ops)
				if (outcome.tag === "rejected") {
					return outcome
				}
				return { ...outcome, value: recorded.value }
			})
		},

		async commitSplit(body) {
			return withGate(core, async function splitBody() {
				await opened
				const recorded = await recordWithLeases(core, state, body)
				if (recorded.ops.length === 0) {
					throw errors.new("commitSplit recorded no ops — an empty transaction names no braid")
				}
				const partitioned = braidsTouched(core, recorded.ops)
				const outcomes: BraidOutcome<Rels>[] = []
				for (const [braid, ops] of partitioned) {
					const outcome = await disciplineCommit(core, state, braid, ops)
					if (outcome.tag === "rejected") {
						outcomes.push({ tag: "rejected", braid, violations: outcome.violations })
					} else {
						outcomes.push({
							tag: "accepted",
							braid: outcome.braid,
							generation: outcome.generation,
							durability: outcome.durability
						})
					}
				}
				return { value: recorded.value, outcomes }
			})
		}
	}
}

export type { Batch, BraidOutcome, Commit, CommitSplit, Deposition, Durability, Writer }
export { openWriter }
