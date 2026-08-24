/**
 * The writer (60): a replica plus the right to create log objects. One
 * commit path and one loss path: a lost slot's byte-equal occupant is
 * an ambiguous PUT absorbed; anything else discards the local
 * directory, re-opens through the replica to the current tip, and
 * re-judges the recorded ops once — the verdict IS a serial execution,
 * performed. Each loop iteration races once at the then-tip, so a
 * historical loss is structurally uncountable, and bounded live-tip
 * losses surface as ErrContention carrying the terminal re-judgment's
 * own violation or the racing tip.
 */

import * as crypto from "node:crypto"
import type { Fact, FreshKeys, MemberRelation, SchemaRelations, Violation } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { bytesEqual, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import type { RelationInfo } from "#descriptor.ts"
import { ErrSpanningCommit, throwContention } from "#errors.ts"
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
import type { Value } from "#value.ts"
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
interface Batch<Rels extends SchemaRelations> {
	insert<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	delete<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	reserve<Rel extends MemberRelation<Rels>>(
		relation: Rel,
		field: FreshKeys<Rel> & string,
		count: bigint
	): readonly bigint[]
}

interface Writer<Rels extends SchemaRelations> {
	commit<R>(body: (batch: Batch<Rels>) => R): Promise<Commit<Rels, R>>
	commitSplit<R>(body: (batch: Batch<Rels>) => R): Promise<CommitSplit<Rels, R>>
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

/** Runs the recording body, refilling the id-lease pool on a drained
 *  draw and re-running — recording is pure, so a re-run before any
 *  judgment invokes nothing twice that the store could observe. */
async function recordWithLeases<Rels extends SchemaRelations, R>(
	core: Core<Rels>,
	state: WriterState,
	body: (batch: Batch<Rels>) => R
): Promise<{ value: R; ops: Op[] }> {
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

function braidsTouched<Rels extends SchemaRelations>(core: Core<Rels>, ops: readonly Op[]): Map<string, Op[]> {
	const partitioned = new Map<string, Op[]>()
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

/** The terminal contention scream: the re-judgment's own rejection names
 *  the hot statement and carries the offending facts' raw values; an
 *  accepted-but-outraced terminal loss carries the racing tip. */
function screamContention<Rels extends SchemaRelations>(
	braid: string,
	rejudged:
		| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }
		| { readonly tag: "outraced"; readonly tip: bigint }
): never {
	if (rejudged.tag === "rejected") {
		const violation = rejudged.violations[0]
		throwContention(
			{
				braid,
				cause: {
					kind: "hot-key",
					statement: violation === undefined ? "" : violation.canonical,
					determinants: (violation?.facts ?? []).map(function rawOf(offending) {
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

		losses += 1
		core.pendingApplied = false
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
	braid: string,
	ops: readonly Op[]
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
				const [braid, ops] = [...partitioned.entries()][0] as [string, Op[]]
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

export type { Batch, BraidOutcome, Commit, CommitSplit, Durability, Writer }
export { openWriter }
