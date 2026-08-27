/**
 * The writer (60): a replica plus the right to create log objects. Role
 * is a field on the handle. `openWriter(options)` births the store;
 * `openWriter(replica)` wraps a born replica and settles an inherited
 * pending without drawing id leases. A replica never births —
 * ManifestMissing is its refusal. One commit path and one loss path: a
 * lost slot's byte-equal occupant is an ambiguous PUT absorbed; anything
 * else discards the local directory, re-opens through the replica to the
 * current tip, and re-judges the recorded ops once — the verdict IS a
 * serial execution, performed. Each loop iteration races once at the
 * then-tip, so a historical loss is structurally uncountable, and bounded
 * live-tip losses surface as ErrContention carrying the terminal
 * re-judgment's own violation or the racing tip.
 */

import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import {
	type Admission,
	type Fact,
	type FactValue,
	type FreshKeys,
	type FreshRange,
	internalBlake3,
	type LogCodecHandle,
	type MemberRelation,
	type MutationReport,
	rowOf,
	type SchemaRelations,
	type Violation
} from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import type { Digest32 } from "#bytes.ts"
import { bytesEqual, checkedAddU64, digest32, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"
import { chainSum } from "#chain.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import type { Braid, RelationInfo, Theory } from "#descriptor.ts"
import { descriptorOf } from "#descriptor.ts"
import {
	ErrRefillNeeded,
	ErrSpanningCommit,
	refillNeededOf,
	refuse,
	refuseExhausted,
	refuseOverWidth,
	refuseSlotRetired,
	throwContention,
	throwRefillNeeded
} from "#errors.ts"
import type { Generation, StoreKey } from "#keys.ts"
import {
	CKPT_SCRATCH_LEASE,
	checkpointMdbKey,
	ckptDocKey,
	encodeCkptScratch,
	generation,
	idsKey,
	LEASE_NAMESPACE,
	logKey,
	manifestKey
} from "#keys.ts"
import type { CheckpointFacts } from "#manifest.ts"
import { checkpointVector, parseCheckpoint, parseManifest, renderCheckpoint, renderManifest } from "#manifest.ts"
import type { Core, OpenReplicaOptions, Replica } from "#replica.ts"
import {
	applyOps,
	belowFloor,
	chainEntry,
	clearPending,
	coreOf,
	discardAndReopen,
	entriesOf,
	foldPending,
	generationOf,
	holdPending,
	maxBigint,
	openReplica,
	pendingOf,
	persistSidecar,
	readdressPending,
	withGate
} from "#replica.ts"
import type { ObjectStore } from "#store.ts"

/** 10 owns the width: one CAS amortizes counter traffic 4096× below slot traffic. */
const LEASE_WIDTH = 4096n

/** The counter body's one grammar (`conformance/v3/counter/`): a
 *  canonical u64 decimal — no sign, no leading zero, no trailing bytes. */
const ID_LEASE_COUNTER = regex("^(?:0|[1-9][0-9]*)$")
const U64_MAX = 0xffffffffffffffffn

/** The live-loss bound (60): consecutive losses at the live tip, history never counts. */
const LOSS_BOUND = 16

/** Writer id in the fixed-layout header: magic + version + flags + fingerprint + braid + braid_gen + prev. */
const WRITER_AT = 4 + 2 + 2 + 32 + 4 + 8 + 32

const BATCH_MAGIC = utf8Encoder.encode("BDBL")

type Durability = "published" | "local-pending"

/** Where an accepted commit landed on its braid: the slot (the braid
 *  position — never the store-wide sum) and how durable the batch is. */
interface Landing {
	readonly slot: Generation
	readonly durability: Durability
}

/** The accepted payload of `commit`: the body's value plus the landing. */
interface CommitReceipt<R> extends Landing {
	readonly value: R
	readonly braid: Braid
}

/** The empty commit is not a commit: a body that records no ops names no
 *  braid, so the refusal is its own outcome — never a slot, never a
 *  thrown surprise. The body's value rides out, the way an abandoned
 *  write's payload does in the engine. */
interface EmptyCommit<R> {
	readonly tag: "empty"
	readonly value: R
}

/** The engine's Admission sum carrying the log's receipt in its accepted
 *  arm; the rejected arm is the engine's violations, shell and payload. */
type Commit<Rels extends SchemaRelations, R> = Admission<Rels, CommitReceipt<R>> | EmptyCommit<R>

/** One braid's verdict inside a split commit: the engine's Admission
 *  carrying the landing, beside the braid it judged. */
interface BraidOutcome<Rels extends SchemaRelations> {
	readonly braid: Braid
	readonly admission: Admission<Rels, Landing>
}

type CommitSplit<Rels extends SchemaRelations, R> =
	| { readonly tag: "split"; readonly value: R; readonly outcomes: readonly BraidOutcome<Rels>[] }
	| EmptyCommit<R>

/**
 * The recorder: the engine `WriteTx`'s write surface, verbatim — insert,
 * delete, and reserve carry the engine's own signatures, so a write-only
 * body typechecks against both dialects. `contains`/`get` are absent by
 * law: the journaled dialect is a pure recorder, and change is judged at
 * commit, so a report's `changed` is 0n at record time. Reservations
 * never appear in the log; the resulting inserts carry concrete values.
 */
interface Batch<Rels extends SchemaRelations> {
	insert<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): MutationReport
	delete<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): MutationReport
	reserve<Rel extends MemberRelation<Rels>>(relation: Rel, field: FreshKeys<Rel> & string, count: bigint): FreshRange
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
	readonly replica: Replica<Rels>
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

/** The zero draw is the absence of a range (the engine's ruling). */
const EMPTY_RANGE: FreshRange = Object.freeze({
	empty: true,
	count: 0n,
	at(_index: bigint) {
		return undefined
	},
	*[Symbol.iterator](): IterableIterator<bigint> {}
})

/** A drawn range as the engine's FreshRange value: contiguous, frozen. */
function drawnRange(start: bigint, endExclusive: bigint): FreshRange {
	const count = endExclusive - start
	return Object.freeze({
		empty: false,
		start,
		endExclusive,
		count,
		at(index: bigint) {
			if (index < 0n || index >= count) {
				return undefined
			}
			return start + index
		},
		*[Symbol.iterator](): IterableIterator<bigint> {
			for (let id = start; id < endExclusive; id++) {
				yield id
			}
		}
	})
}

/** Untouched full-width blocks pooled for the key: the record loop's
 *  guarantee unit — k full blocks serve any k draws, each at most one
 *  width, whatever tails the draws abandon. */
function fullBlocksOf(state: WriterState, relation: number, field: number): number {
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	let full = 0
	for (const range of pool) {
		if (range.end - range.next >= LEASE_WIDTH) {
			full += 1
		}
	}
	return full
}

/** `Lease.draw(count)` = OverWidth | Exhausted | Drawn, the log-Rust
 *  algebra: Exhausted names the u64 ceiling ONLY; a draw the pooled
 *  tail cannot serve abandons the tail (unique, never dense) and falls
 *  to the next block, and an empty pool signals a refill. */
function drawIds(
	state: WriterState,
	draws: Map<string, number>,
	relation: number,
	field: number,
	count: bigint
): FreshRange {
	if (count < 0n) {
		throw errors.new(`id-lease count is unsigned: ${count}`)
	}
	if (count > LEASE_WIDTH) {
		refuseOverWidth({ requested: count }, `id-lease draw ${count} exceeds the lease width ${LEASE_WIDTH}`)
	}
	if (count === 0n) {
		return EMPTY_RANGE
	}
	const key = poolKey(relation, field)
	const drawn = (draws.get(key) ?? 0) + 1
	draws.set(key, drawn)
	const pool = state.pools.get(key) ?? []
	while (pool.length > 0) {
		const range = pool[0]
		if (range === undefined) {
			break
		}
		const end = checkedAddU64(range.next, count)
		if (end === undefined) {
			refuseExhausted({ relation, field }, `id-lease relation ${relation} field ${field} would leave u64`)
		}
		if (end <= range.end) {
			const start = range.next
			range.next = end
			if (range.next >= range.end) {
				pool.shift()
			}
			return drawnRange(start, end)
		}
		pool.shift()
	}
	throwRefillNeeded(
		{ relation, field, requested: count },
		`id-lease relation ${relation} field ${field} pool cannot serve ${count}`
	)
}

/** The counter parse: canonical decimal within u64 or a typed Counter
 *  refusal — leading zeros, signs, trailing bytes, non-UTF-8, and
 *  values past u64 all refuse under the one pinned identity. */
function parseCounter(key: StoreKey, bytes: Uint8Array): bigint {
	const decoded = errors.trySync(function decodeCounter() {
		return utf8StrictDecoder.decode(bytes)
	})
	if (decoded.error) {
		refuse({ kind: "Counter", key }, `id-lease counter ${key} body is not UTF-8`)
	}
	if (!ID_LEASE_COUNTER.test(decoded.data)) {
		refuse({ kind: "Counter", key }, `id-lease counter ${key} body is not a canonical decimal`)
	}
	const value = BigInt(decoded.data)
	if (value > U64_MAX) {
		refuse({ kind: "Counter", key }, `id-lease counter ${key} value leaves u64`)
	}
	return value
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
		const next = parseCounter(key, fetched.bytes)
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

function recorderOf<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	draws: Map<string, number>
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
	// The cell judge is the engine's `rowOf` — one marshal per language.
	// The recorder applies nothing, so `changed` is 0n: change is judged
	// at commit, when the recording meets the store.
	function record<Rel extends MemberRelation<Rels>>(
		op: "insert" | "delete",
		relation: Rel,
		facts: Iterable<Fact<Rel>>
	): MutationReport {
		const info = infoOf(relation.name)
		const rows: FactValue[][] = []
		for (const fact of facts) {
			rows.push(rowOf(relation.data, fact))
		}
		recording.ops.push({ op, relation: info.name, rows })
		return Object.freeze({ submitted: BigInt(rows.length), changed: 0n })
	}
	const batch: Batch<Rels> = {
		insert(relation, facts) {
			return record("insert", relation, facts)
		},
		delete(relation, facts) {
			return record("delete", relation, facts)
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
			return drawIds(state, draws, info.id, ordinal, count)
		}
	}
	return { batch, recording }
}

/** Runs the recording body against the cached pool. A draw the pool
 *  cannot serve discards the attempt's recording, leases fresh blocks —
 *  one full block per draw the attempt made on the starved key — and
 *  runs the body again: the refill is a path, not an exhaustion
 *  (Exhausted names the u64 ceiling only). Ids drawn by a discarded
 *  attempt are abandoned — unique, never dense. Draw counts rise
 *  strictly between refills of one key, so a body with finitely many
 *  draws settles; a recurring signature screams. */
async function recordWithLeases<Rels extends SchemaRelations, R>(
	core: Core<Rels>,
	state: WriterState,
	body: (batch: Batch<Rels>) => R | Promise<R>
): Promise<{ value: R; ops: Op[] }> {
	await ensureFreshLeases(core, state)
	for (;;) {
		const draws = new Map<string, number>()
		const { batch, recording } = recorderOf(core, state, draws)
		const ran = await errors.try(
			(async function runBody() {
				return body(batch)
			})()
		)
		if (!ran.error) {
			return { value: ran.data, ops: recording.ops }
		}
		if (!errors.is(ran.error, ErrRefillNeeded)) {
			throw ran.error
		}
		const need = refillNeededOf(ran.error)
		if (need === undefined) {
			throw ran.error
		}
		// Draws this attempt made on the starved key, the missed one included.
		const drawn = draws.get(poolKey(need.relation, need.field)) ?? 0
		state.scream.attempt(`id-lease refill relation ${need.relation} field ${need.field} draws ${drawn}`)
		while (fullBlocksOf(state, need.relation, need.field) < drawn) {
			await acquireLease(core, state, need.relation, need.field)
		}
	}
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
 *  does not hide the slot's owner. The Rust writer machine sniffs the
 *  same fixed offset (`writer/loss.rs`); the batch grammar itself has
 *  one reader — the codec seat. */
function headerWriter(bytes: Uint8Array): bigint | undefined {
	if (bytes.length < BATCH_MAGIC.length || !bytesEqual(bytes.subarray(0, BATCH_MAGIC.length), BATCH_MAGIC)) {
		return undefined
	}
	return u64leAt(bytes, WRITER_AT)
}

/** Header prev is 32 branded bytes. */
function digestPrev(prev: Digest32 | Uint8Array): Digest32 {
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
 * Publishes the applied pending batch: refuse a below-floor create,
 * then slot CAS, then the one loss path on Exists. A byte-equal
 * occupant is our own ambiguous PUT, absorbed. Anything else carries
 * the pending through a directory discard — re-persisted into the
 * fresh sidecar before any re-judgment — re-opens to the current tip,
 * and re-judges the recorded ops in one db.write. An occupant that
 * then vanishes is retired, not a loop that forges the swept slot.
 */
async function publishPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	ops: readonly Op[]
): Promise<Admission<Rels, Landing>> {
	let losses = 0
	for (;;) {
		const pending = pendingOf(core)
		if (pending === null) {
			throw errors.new("publish reached with no pending batch")
		}
		const braid = pending.braid
		// The floor is a write precondition: a slot at or below it is
		// retired. A create must not touch the store (70/116/127).
		if (belowFloor(core, braid, pending.slot)) {
			refuseSlotRetired({ braid, slot: pending.slot }, "the slot is retired")
		}
		const created = await core.store.putCreate(logKey(core.prefix, braid, pending.slot), pending.bytes)
		let winnerBytes: Uint8Array | null = null
		if (created.tag !== "created") {
			const fetched = await core.store.get(logKey(core.prefix, braid, pending.slot))
			if (fetched === null) {
				// Exists then null: the occupant was swept. Refuse
				// rather than loop back into putCreate.
				refuseSlotRetired({ braid, slot: pending.slot }, "the slot is retired")
			}
			if (!bytesEqual(fetched.bytes, pending.bytes)) {
				winnerBytes = fetched.bytes
			}
		}
		if (winnerBytes === null) {
			if (pending.ts === null) {
				throw errors.new("publish reached with an undecoded pending timestamp")
			}
			entriesOf(core).set(braid, {
				g: pending.slot,
				prev: digest32(new Uint8Array(internalBlake3(pending.bytes))),
				ts: pending.ts
			})
			await clearPending(core)
			return { tag: "accepted", value: { slot: pending.slot, durability: "published" } }
		}

		losses += 1
		state.scream.attempt("slot occupant is not ours")
		noteDeposition(state, braid, pending.slot, winnerBytes)
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
			return { tag: "accepted", value: { slot: chainEntry(core, braid).g, durability: "published" } }
		}
		const tip = chainEntry(core, braid)
		entriesOf(core).set(braid, { g: tip.g, prev: digestPrev(tip.prev), ts: tip.ts })
		await readdressPending(core, ops, state.writerId)
		if (losses >= LOSS_BOUND) {
			screamContention(braid, { tag: "outraced", tip: chainEntry(core, braid).g })
		}
	}
}

/** The commit discipline (60): encode, fsync as Pending before any
 *  apply, judge locally, publish only what advanced the generation.
 *  Settled is written only after the verdict. */
async function disciplineCommit<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	braid: Braid,
	ops: readonly Op[]
): Promise<Admission<Rels, Landing>> {
	const entry = chainEntry(core, braid)
	const timestamp = maxBigint(BigInt(Date.now()), entry.ts)
	const bytes = encodeBatch(
		core.descriptor,
		{
			braid,
			braidGen: generation(entry.g + 1n),
			prev: digestPrev(entry.prev),
			writer: state.writerId,
			timestamp
		},
		ops
	)
	// Pending → durable, before any apply.
	holdPending(core, { braid, slot: generation(entry.g + 1n), bytes }, ops, timestamp)
	await persistSidecar(core)

	const before = generationOf(core)
	const outcome = applyOps(core, ops)
	if (outcome.tag === "rejected") {
		await clearPending(core)
		return { tag: "rejected", violations: outcome.violations }
	}
	if (outcome.value.generation === before) {
		await clearPending(core)
		return { tag: "accepted", value: { slot: entry.g, durability: "published" } }
	}
	return publishPending(core, state, ops)
}

/** An inherited pending is resolved-and-published by open. A slot the
 *  floor already covers is published (`Clear`), not re-judged (46). */
async function settleInheritedPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState
): Promise<void> {
	const pending = pendingOf(core)
	if (pending === null) {
		return
	}
	if (
		foldPending(
			chainSum(core.chain),
			generationOf(core),
			null,
			pending.bytes,
			belowFloor(core, pending.braid, pending.slot)
		).tag === "below-floor"
	) {
		await clearPending(core)
		return
	}
	let ops = pending.ops
	if (ops === null) {
		const decoded = errors.trySync(function decodePending() {
			return decodeBatch(core.descriptor, pending.bytes)
		})
		if (decoded.error) {
			await clearPending(core)
			return
		}
		ops = decoded.data.ops
		// The decoded arm re-holds so publish reads the header facts once.
		holdPending(
			core,
			{ braid: pending.braid, slot: pending.slot, bytes: pending.bytes },
			ops,
			decoded.data.header.timestamp
		)
	}
	const published = await publishPending(core, state, ops)
	if (published.tag === "rejected") {
		return
	}
}

type PublishRefusal = "manifest-missing" | "manifest" | "checkpoint-doc-missing" | "checkpoint"

type Published =
	| { readonly tag: "replaced" }
	| { readonly tag: "kept"; readonly incumbent: Digest32 }
	| { readonly tag: "refused"; readonly reason: PublishRefusal }

async function putCreateOnce(store: ObjectStore, key: ReturnType<typeof ckptDocKey>, bytes: Uint8Array): Promise<void> {
	for (;;) {
		const created = await store.putCreate(key, bytes)
		if (created.tag === "created" || created.tag === "exists") {
			return
		}
		const fetched = await store.get(key)
		if (fetched !== null) {
			return
		}
	}
}

/** The loser deletes its own `ckpt/{digest}` and `.mdb`. */
async function deleteOrphan(store: ObjectStore, prefix: string, digest: Digest32): Promise<void> {
	await store.delete(ckptDocKey(prefix, digest))
	await store.delete(checkpointMdbKey(prefix, digest))
}

function scratchPath(dir: string): string {
	return path.join(dir, LEASE_NAMESPACE, CKPT_SCRATCH_LEASE)
}

async function claimScratch(dir: string, digest: Digest32): Promise<void> {
	const target = scratchPath(dir)
	await fs.mkdir(path.dirname(target), { recursive: true })
	const handle = await fs.open(target, "w")
	const written = await errors.try(
		(async function writeLease() {
			await handle.writeFile(encodeCkptScratch(digest))
			await handle.sync()
		})()
	)
	await handle.close()
	if (written.error) {
		await fs.rm(target, { force: true })
		throw written.error
	}
}

async function releaseScratch(dir: string): Promise<void> {
	await fs.rm(scratchPath(dir), { force: true })
}

async function casPublish(
	store: ObjectStore,
	prefix: string,
	codec: LogCodecHandle,
	candidate: CheckpointFacts,
	digest: Digest32,
	bytes: Uint8Array
): Promise<Published> {
	await putCreateOnce(store, ckptDocKey(prefix, digest), bytes)
	for (;;) {
		const fetched = await store.get(manifestKey(prefix))
		if (fetched === null) {
			return { tag: "refused", reason: "manifest-missing" }
		}
		const parsed = errors.trySync(function parse() {
			return parseManifest(fetched.bytes)
		})
		if (parsed.error) {
			return { tag: "refused", reason: "manifest" }
		}
		const incumbent = parsed.data.checkpoint
		if (incumbent !== null && bytesEqual(incumbent, digest)) {
			return { tag: "replaced" }
		}
		if (incumbent !== null) {
			const doc = await store.get(ckptDocKey(prefix, incumbent))
			if (doc === null) {
				return { tag: "refused", reason: "checkpoint-doc-missing" }
			}
			const incumbentDoc = errors.trySync(function parseIncumbent() {
				return parseCheckpoint(codec, doc.bytes)
			})
			if (incumbentDoc.error) {
				return { tag: "refused", reason: "checkpoint" }
			}
			if (checkpointVector(candidate).order(checkpointVector(incumbentDoc.data)) !== "after") {
				return { tag: "kept", incumbent }
			}
		}
		const next = renderManifest({ fingerprint: parsed.data.fingerprint, checkpoint: digest })
		const swapped = await store.putSwap(manifestKey(prefix), next, fetched.etag)
		if (swapped.tag === "swapped") {
			return { tag: "replaced" }
		}
	}
}

/**
 * Publishes `candidate` under the checkpoint order. The digest is the
 * blake3 of the full bytes, `prev` included. A scratch lease
 * `{dir}/~lease/ckpt-scratch` names the candidate for the publish
 * window; a crash leaves that lease and a successor sweep reclaims
 * it. `Kept` and every refused publish delete the candidate's `ckpt`
 * pair.
 */
async function publishCheckpoint(
	store: ObjectStore,
	prefix: string,
	dir: string,
	theory: Theory,
	candidate: CheckpointFacts,
	mdb: Uint8Array
): Promise<Published> {
	const codec = descriptorOf(theory).codec
	const bytes = renderCheckpoint(codec, candidate)
	const digest = digest32(new Uint8Array(internalBlake3(bytes)))
	await claimScratch(dir, digest)
	const ran = await errors.try(
		(async function publish() {
			await putCreateOnce(store, checkpointMdbKey(prefix, digest), mdb)
			return await casPublish(store, prefix, codec, candidate, digest, bytes)
		})()
	)
	if (ran.error) {
		throw ran.error
	}
	if (ran.data.tag === "kept" || ran.data.tag === "refused") {
		await deleteOrphan(store, prefix, digest)
	}
	await releaseScratch(dir)
	return ran.data
}

/** Only the writer births a store: create-only PUT of a genesis manifest. */
async function birthStore(store: ObjectStore, prefix: string, fingerprint: Digest32): Promise<void> {
	const key = manifestKey(prefix)
	for (;;) {
		const fetched = await store.get(key)
		if (fetched !== null) {
			return
		}
		const bytes = renderManifest({ fingerprint, checkpoint: null })
		const created = await store.putCreate(key, bytes)
		if (created.tag === "created") {
			return
		}
	}
}

function isReplica<Rels extends SchemaRelations>(
	source: Replica<Rels> | OpenReplicaOptions<Rels>
): source is Replica<Rels> {
	return typeof (source as Replica<Rels>).refresh === "function"
}

/** Wrap a born replica: settle an inherited pending; do not birth; do not draw id leases. */
async function writerOn<Rels extends SchemaRelations>(replica: Replica<Rels>): Promise<Writer<Rels>> {
	const core = coreOf(replica)
	const state: WriterState = {
		writerId: crypto.randomBytes(8).readBigUInt64LE(),
		pools: new Map(),
		scream: screamOf("writer discard-and-re-pull"),
		deposition: null
	}
	await withGate(core, async function openTransition() {
		await settleInheritedPending(core, state)
	})
	return {
		role: "writer",
		replica,
		deposition() {
			return state.deposition
		},
		async commit(body) {
			return withGate(core, async function commitBody() {
				const recorded = await recordWithLeases(core, state, body)
				if (recorded.ops.length === 0) {
					return { tag: "empty" as const, value: recorded.value }
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
				return {
					tag: "accepted" as const,
					value: { value: recorded.value, braid, slot: outcome.value.slot, durability: outcome.value.durability }
				}
			})
		},

		async commitSplit(body) {
			return withGate(core, async function splitBody() {
				const recorded = await recordWithLeases(core, state, body)
				if (recorded.ops.length === 0) {
					return { tag: "empty" as const, value: recorded.value }
				}
				const partitioned = braidsTouched(core, recorded.ops)
				const outcomes: BraidOutcome<Rels>[] = []
				for (const [braid, ops] of partitioned) {
					const admission = await disciplineCommit(core, state, braid, ops)
					outcomes.push({ braid, admission })
				}
				return { tag: "split" as const, value: recorded.value, outcomes }
			})
		}
	}
}

async function openWriter<Rels extends SchemaRelations>(replica: Replica<Rels>): Promise<Writer<Rels>>
async function openWriter<Rels extends SchemaRelations>(options: OpenReplicaOptions<Rels>): Promise<Writer<Rels>>
async function openWriter<Rels extends SchemaRelations>(
	source: Replica<Rels> | OpenReplicaOptions<Rels>
): Promise<Writer<Rels>> {
	if (isReplica(source)) {
		return writerOn(source)
	}
	await birthStore(source.store, source.prefix, digest32(descriptorOf(source.theory).fingerprintBytes))
	return writerOn(await openReplica(source))
}

export type {
	Batch,
	BraidOutcome,
	Commit,
	CommitReceipt,
	CommitSplit,
	Deposition,
	Durability,
	EmptyCommit,
	Landing,
	Writer
}
export { openWriter, publishCheckpoint }
