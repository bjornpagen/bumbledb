/**
 * The writer (60): a replica plus the right to create log objects. One
 * commit path; the loser algebra (15) between it and the store. There
 * is no Contended arm — contention is absorbed: a subsumed loss reports
 * the winner's generation, a disjoint loss republishes silently, a
 * conflicting loss re-judges the recorded ops (never the host closure),
 * and bounded live-tip losses surface as ErrContention carrying the hot
 * key's raw determinants or the racing tip.
 */

import * as crypto from "node:crypto"
import type { Fact, FreshKeys, MemberRelation, SchemaRelations, Violation } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { bytesEqual, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import type { RelationInfo } from "#descriptor.ts"
import { ErrReplayDiverged, ErrSpanningCommit, throwContention } from "#errors.ts"
import type { BatchOp } from "#footprint.ts"
import { computeFootprint } from "#footprint.ts"
import type { SharedKey } from "#intersect.ts"
import { intersectionOf } from "#intersect.ts"
import { idsKey, logKey } from "#keys.ts"
import type { Core, Replica } from "#replica.ts"
import {
	applyOps,
	blake3Hex,
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
import type { LogValue } from "#value.ts"
import { checkAgainst } from "#value.ts"

/** 10 owns the width: one CAS amortizes counter traffic 4096× below slot traffic. */
const LEASE_WIDTH = 4096n

/** The live-loss bound (60): consecutive losses at the live tip, history never counts. */
const LOSS_BOUND = 16

type Durability = "published" | "local-pending"

type Commit<Rels extends SchemaRelations, R> =
	| {
			readonly tag: "accepted"
			readonly value: R
			readonly braid: string
			readonly generation: bigint
			readonly durability: Durability
	  }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }

type BraidOutcome<Rels extends SchemaRelations> =
	| { readonly tag: "accepted"; readonly braid: string; readonly generation: bigint; readonly durability: Durability }
	| { readonly tag: "rejected"; readonly braid: string; readonly violations: readonly Violation<Rels>[] }

interface CommitSplit<Rels extends SchemaRelations, R> {
	readonly value: R
	readonly outcomes: readonly BraidOutcome<Rels>[]
}

/**
 * The recorder: typed inserts and deletes as raw-valued ops, `reserve`
 * drawing on the id lease (10) — reservations never appear in the log;
 * the resulting inserts carry concrete values. Pure and synchronous.
 */
interface LogBatch<Rels extends SchemaRelations> {
	insert<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	delete<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	reserve<Rel extends MemberRelation<Rels>>(
		relation: Rel,
		field: FreshKeys<Rel> & string,
		count: bigint
	): readonly bigint[]
}

interface Writer<Rels extends SchemaRelations> {
	commit<R>(body: (batch: LogBatch<Rels>) => R): Promise<Commit<Rels, R>>
	commitSplit<R>(body: (batch: LogBatch<Rels>) => R): Promise<CommitSplit<Rels, R>>
}

interface LeaseRange {
	next: bigint
	readonly end: bigint
}

interface WriterState {
	readonly writerId: bigint
	readonly pools: Map<string, LeaseRange[]>
}

const ErrLeaseDrained = errors.new("bumbledb-log lease pool drained mid-recording")
const leaseDemand = new WeakMap<Error, { relation: number; field: number }>()

function poolKey(relation: number, field: number): string {
	return `${relation}:${field}`
}

function drawIds(state: WriterState, relation: number, field: number, count: bigint): bigint[] {
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	const ids: bigint[] = []
	let remaining = count
	while (remaining > 0n) {
		const range = pool[0]
		if (range === undefined) {
			const fault = errors.wrap(ErrLeaseDrained, `relation ${relation} field ${field} needs ${remaining} more ids`)
			leaseDemand.set(fault, { relation, field })
			throw fault
		}
		while (remaining > 0n && range.next < range.end) {
			ids.push(range.next)
			range.next += 1n
			remaining -= 1n
		}
		if (range.next >= range.end) {
			pool.shift()
		}
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
		const swapped = await core.store.putSwap(key, utf8Encoder.encode(String(next + LEASE_WIDTH)), fetched.etag)
		if (swapped.tag === "swapped") {
			pushRange(state, relation, field, next, next + LEASE_WIDTH)
			return
		}
	}
}

function pushRange(state: WriterState, relation: number, field: number, next: bigint, end: bigint): void {
	const pool = state.pools.get(poolKey(relation, field)) ?? []
	pool.push({ next, end })
	state.pools.set(poolKey(relation, field), pool)
}

interface Recording {
	readonly ops: BatchOp[]
}

function lowerFact<Rels extends SchemaRelations>(
	core: Core<Rels>,
	info: RelationInfo,
	fact: Record<string, unknown>
): LogValue[] {
	return info.fields.map(function lowerCell(field) {
		const raw = fact[field.name]
		if (raw === undefined) {
			throw errors.new(`relation ${info.name}: fact is missing field ${field.name}`)
		}
		let value: LogValue
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
			value = raw as LogValue
		}
		checkAgainst(`relation ${info.name} field ${field.name}`, field.type, value)
		return value
	})
}

function recorderOf<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState
): { batch: LogBatch<Rels>; recording: Recording } {
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
		const rows: LogValue[][] = []
		for (const fact of facts) {
			if (typeof fact !== "object" || fact === null) {
				throw errors.new(`relation ${relationName}: a fact is not an object`)
			}
			rows.push(lowerFact(core, info, fact as Record<string, unknown>))
		}
		recording.ops.push({ op, relation: relationName, rows })
	}
	const batch: LogBatch<Rels> = {
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

/** Runs the recording body, refilling the id-lease pool on a drained
 *  draw and re-running — recording is pure, so a re-run before any
 *  judgment re-invokes nothing the loser algebra promised not to. */
async function recordWithLeases<Rels extends SchemaRelations, R>(
	core: Core<Rels>,
	state: WriterState,
	body: (batch: LogBatch<Rels>) => R
): Promise<{ value: R; ops: BatchOp[] }> {
	for (let attempt = 0; attempt < 16; attempt++) {
		const { batch, recording } = recorderOf(core, state)
		const ran = errors.trySync(function runBody() {
			return body(batch)
		})
		if (ran.error === undefined) {
			return { value: ran.data, ops: recording.ops }
		}
		const demand = leaseDemand.get(ran.error) ?? leaseDemand.get(errors.cause(ran.error))
		if (demand === undefined) {
			throw ran.error
		}
		await acquireLease(core, state, demand.relation, demand.field)
	}
	throw errors.new("the id-lease pool could not satisfy the recording after 16 refills")
}

function braidsTouched<Rels extends SchemaRelations>(
	core: Core<Rels>,
	ops: readonly BatchOp[]
): Map<string, BatchOp[]> {
	const partitioned = new Map<string, BatchOp[]>()
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

type LastLoss = { readonly kind: "slot-race" } | { readonly kind: "conflict"; readonly shared: readonly SharedKey[] }

/**
 * Publishes the applied pending batch: slot CAS, then the loser algebra
 * on Exists — subsume (drop), republish (fully key-disjoint), or
 * re-judge (anything else, the quantitative W arm included: with no
 * base measure at hand the arithmetic shortcut is skipped and the
 * always-sound re-judgment runs).
 */
async function publishPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	ops: readonly BatchOp[]
): Promise<Commit<Rels, undefined>> {
	let losses = 0
	let lastLoss: LastLoss = { kind: "slot-race" }
	for (;;) {
		const pending = core.pending
		if (pending === null) {
			throw errors.new("publish reached with no pending batch")
		}
		const braid = pending.braid
		if (losses >= LOSS_BOUND) {
			if (lastLoss.kind === "conflict") {
				const shared = lastLoss.shared[0]
				const provenance =
					shared === undefined
						? undefined
						: computeFootprint(core.descriptor, ops).provenance.get(
								`${shared.class}:${shared.statement ?? ""}:${shared.keyHex}`
							)
				throwContention(
					{
						braid,
						cause: {
							kind: "hot-key",
							statement: shared?.statement ?? provenance?.statement ?? -1,
							determinants: provenance?.values ?? []
						}
					},
					`braid ${braid}: ${LOSS_BOUND} consecutive conflicting losses at the live tip`
				)
			}
			throwContention(
				{ braid, cause: { kind: "slot-race", tip: chainEntry(core, braid).g } },
				`braid ${braid}: ${LOSS_BOUND} consecutive disjoint losses at the live tip`
			)
		}
		const created = await core.store.putCreate(logKey(core.prefix, braid, pending.gen), pending.bytes)
		let winnerBytes: Uint8Array | null = null
		if (created.tag === "exists") {
			const fetched = await core.store.get(logKey(core.prefix, braid, pending.gen))
			if (fetched === null) {
				continue
			}
			if (!bytesEqual(fetched.bytes, pending.bytes)) {
				winnerBytes = fetched.bytes
			}
		}
		if (winnerBytes === null) {
			const header = decodeBatch(core.descriptor, pending.bytes).header
			core.chain.set(braid, { g: pending.gen, prev: blake3Hex(pending.bytes), ts: header.timestamp })
			await clearPending(core)
			return { tag: "accepted", value: undefined, braid, generation: pending.gen, durability: "published" }
		}

		const winner = decodeBatch(core.descriptor, winnerBytes)
		const meet = intersectionOf(core.descriptor, ops, winner.ops)
		losses += 1

		if (meet.tag === "subsumed") {
			const before = generationOf(core)
			const applied = applyOps(core, winner.ops)
			if (applied.tag !== "accepted") {
				throw errors.wrap(ErrReplayDiverged, `braid ${braid} slot ${pending.gen}: a subsuming winner rejected locally`)
			}
			core.chain.set(braid, { g: pending.gen, prev: blake3Hex(winnerBytes), ts: winner.header.timestamp })
			const identical = applied.value.generation === before
			await clearPending(core)
			if (!identical) {
				await discardAndReopen(core)
			}
			return { tag: "accepted", value: undefined, braid, generation: pending.gen, durability: "published" }
		}

		if (meet.tag === "disjoint") {
			lastLoss = { kind: "slot-race" }
			const before = generationOf(core)
			const applied = applyOps(core, winner.ops)
			if (applied.tag !== "accepted" || applied.value.generation === before) {
				throw errors.wrap(
					ErrReplayDiverged,
					`braid ${braid} slot ${pending.gen}: a fully disjoint winner failed its provably state-changing apply`
				)
			}
			core.chain.set(braid, { g: pending.gen, prev: blake3Hex(winnerBytes), ts: winner.header.timestamp })
			await readdressPending(core, ops, state.writerId)
			continue
		}

		lastLoss = { kind: "conflict", shared: meet.tag === "conflict" ? meet.shared : [] }
		await clearPending(core)
		await discardAndReopen(core)
		const before = generationOf(core)
		const rejudged = applyOps(core, ops)
		if (rejudged.tag === "rejected") {
			return { tag: "rejected", violations: rejudged.violations }
		}
		if (rejudged.value.generation === before) {
			return {
				tag: "accepted",
				value: undefined,
				braid,
				generation: chainEntry(core, braid).g,
				durability: "published"
			}
		}
		await readdressPending(core, ops, state.writerId)
	}
}

/** The commit discipline (60): encode, fsync the pending, judge locally,
 *  publish only what advanced the generation. */
async function disciplineCommit<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState,
	braid: string,
	ops: readonly BatchOp[]
): Promise<Commit<Rels, undefined>> {
	const entry = chainEntry(core, braid)
	const timestamp = maxBigint(BigInt(Date.now()), entry.ts)
	const bytes = encodeBatch(
		core.descriptor,
		{
			fingerprint: core.descriptor.fingerprint,
			braid,
			braidGen: entry.g + 1n,
			prev: entry.prev,
			writer: state.writerId,
			timestamp
		},
		ops
	)
	core.pending = { braid, gen: entry.g + 1n, bytes }
	core.pendingOps = ops
	core.pendingApplied = false
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
	core.pendingApplied = true
	return publishPending(core, state, ops)
}

/** A recovered pending owed from a previous life publishes before any new commit. */
async function settleInheritedPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	state: WriterState
): Promise<void> {
	if (core.pending === null || !core.pendingApplied || core.pendingOps === null) {
		return
	}
	await publishPending(core, state, core.pendingOps)
}

function openWriter<Rels extends SchemaRelations>(replica: Replica<Rels>): Writer<Rels> {
	const core = coreOf(replica)
	const state: WriterState = {
		writerId: crypto.randomBytes(8).readBigUInt64LE(),
		pools: new Map()
	}
	return {
		async commit(body) {
			return withGate(core, async function commitBody() {
				await settleInheritedPending(core, state)
				const recorded = await recordWithLeases(core, state, body)
				if (recorded.ops.length === 0) {
					throw errors.new("commit recorded no ops — an empty transaction names no braid")
				}
				const partitioned = braidsTouched(core, recorded.ops)
				if (partitioned.size > 1) {
					throw errors.wrap(ErrSpanningCommit, `the recorded ops span braids ${[...partitioned.keys()].join(", ")}`)
				}
				const [braid, ops] = [...partitioned.entries()][0] as [string, BatchOp[]]
				const outcome = await disciplineCommit(core, state, braid, ops)
				if (outcome.tag === "rejected") {
					return outcome
				}
				return { ...outcome, value: recorded.value }
			})
		},

		async commitSplit(body) {
			return withGate(core, async function splitBody() {
				await settleInheritedPending(core, state)
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

export type { BraidOutcome, Commit, CommitSplit, Durability, LogBatch, Writer }
export { openWriter }
