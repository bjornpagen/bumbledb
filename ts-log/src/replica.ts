/**
 * The replica: a local store that is a materialized view of the
 * braids' prefixes, plus the loop that keeps it current. Disposable by
 * construction — the sidecar is a floor cache with one wholeness
 * check; recovery IS the catch-up loop (L10). Generation is a total
 * function of the chain: Settled sums the vector, Pending is that sum
 * plus one. A replica that finds no manifest refuses; only a writer
 * births a store.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import type { Db, MemberRelation, Schema, SchemaRelations, WriteOutcome } from "@bjornpagen/bumbledb"
import { factOf, internalBlake3, Db as SdkDb } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { parse } from "#braids.ts"
import type { Digest32 } from "#bytes.ts"
import { bytesEqual, digest32, hex32, saturatingAddU64 } from "#bytes.ts"
import type { Chain, ChainEntry, Pending } from "#chain.ts"
import { chainGeneration, chainSum, readSidecar, writeSidecar } from "#chain.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import type { Braid, Descriptor } from "#descriptor.ts"
import { descriptorOf } from "#descriptor.ts"
import { ErrRefused, ErrReplayDiverged, refuse, refuseManifestMissing, wrapStore } from "#errors.ts"
import type { Generation } from "#keys.ts"
import {
	CKPT_SCRATCH_LEASE,
	checkpointMdbKey,
	ckptDocKey,
	generation,
	LEASE_NAMESPACE,
	logKey,
	manifestKey,
	parseCkptScratch,
	TEMP_NAMESPACE
} from "#keys.ts"
import type { CheckpointFacts } from "#manifest.ts"
import { auditCatalog, parseCheckpoint, parseManifest } from "#manifest.ts"
import type { Etag, ObjectStore } from "#store.ts"
import { Vector } from "#vector.ts"

const ZERO_HASH = digest32(new Uint8Array(32))

/** The gc-safety heartbeat cadence: every N-th pass re-reads the manifest. */
const HEARTBEAT_PASSES = 16

/** The re-poll cadence of waitFor, its one consumer — the
 *  machine-constants table's `wait_for_poll_ms` fact. */
const WAIT_FOR_POLL_MS = 10

type ReplicaState =
	| { readonly tag: "bootstrapped" }
	| { readonly tag: "checkpoint-seeded"; readonly catalog: Digest32 }
	| { readonly tag: "sidecar-resumed"; readonly floor: ReadonlyMap<Braid, Generation> }

type RefreshOutcome =
	| { readonly tag: "advanced"; readonly vector: ReadonlyMap<Braid, Generation> }
	| { readonly tag: "wedged"; readonly braid: Braid; readonly cause: string }
	| { readonly tag: "reseed"; readonly cause: string }
	| { readonly tag: "refused"; readonly detail: string }

type Step =
	| { readonly tag: "applied" }
	| { readonly tag: "tip" }
	| { readonly tag: "wedged"; readonly braid: Braid; readonly cause: string }
	| { readonly tag: "reseed"; readonly cause: string }

/** The full waitFor sum: a wedged braid the target needs is an
 *  outcome, never an infinite poll. */
type Waited =
	| { readonly tag: "reached"; readonly vector: ReadonlyMap<Braid, Generation> }
	| { readonly tag: "wedged"; readonly braid: Braid; readonly cause: string }
	| { readonly tag: "refused"; readonly detail: string }

type ApplyPhase = "open" | "steady"

type SlotApply = { readonly tag: "applied"; readonly generation: bigint } | { readonly tag: "discard" }

interface OpenReplicaOptions<Rels extends SchemaRelations> {
	readonly store: ObjectStore
	readonly prefix: string
	readonly dir: string
	readonly theory: Schema<Rels>
}

interface Replica<Rels extends SchemaRelations> extends AsyncDisposable {
	readonly db: Db<Rels>
	readonly vector: ReadonlyMap<Braid, Generation>
	refresh(braid?: Braid): Promise<ReadonlyMap<Braid, Generation>>
	waitFor(vector: ReadonlyMap<Braid, Generation>): Promise<Waited>
}

interface Core<Rels extends SchemaRelations> {
	readonly store: ObjectStore
	readonly prefix: string
	readonly dir: string
	readonly theory: Schema<Rels>
	readonly descriptor: Descriptor
	db: Db<Rels>
	/** Settled|Pending. Generation is chainGeneration(this.chain) — a
	 *  total function of the sum. There is no pending Option beside the
	 *  vector and no 0|1 addend a reader can skip (§30). */
	chain: Chain
	manifestEtag: Etag | null
	checkpoint: CheckpointFacts | null
	checkpointDigest: Digest32 | null
	passes: number
	closed: boolean
	storeName: string
	gate: Promise<unknown>
	wedged: Map<Braid, string>
	provenance: ReplicaState
}

const cores = new WeakMap<object, Core<SchemaRelations>>()

function coreOf<Rels extends SchemaRelations>(replica: Replica<Rels>): Core<Rels> {
	const core = cores.get(replica)
	if (core === undefined) {
		throw errors.new("not a replica of this driver")
	}
	return core as unknown as Core<Rels>
}

/** One asynchronous door per replica: refreshes, commits, and disposal serialize. */
function withGate<Rels extends SchemaRelations, R>(core: Core<Rels>, body: () => Promise<R>): Promise<R> {
	const run = core.gate.then(body, body)
	core.gate = run.then(
		function absorb() {
			return undefined
		},
		function absorbFailure() {
			return undefined
		}
	)
	return run
}

function blake3Digest(bytes: Uint8Array): Digest32 {
	return digest32(new Uint8Array(internalBlake3(bytes)))
}

function sidecarPath<Rels extends SchemaRelations>(core: Core<Rels>): string {
	return path.join(core.dir, "chain")
}

let storeSequence = 0

/** LMDB registers environments per canonical path for the life of the
 *  process and the engine has no close verb, so a store path is never
 *  reused: every bootstrap gets a fresh name and discards leave the old
 *  environment to GC. */
function freshStoreName(): string {
	storeSequence += 1
	return `store-${process.pid.toString(36)}-${Date.now().toString(36)}-${storeSequence}`
}

function storePath<Rels extends SchemaRelations>(core: Core<Rels>): string {
	return path.join(core.dir, core.storeName)
}

/** A wire braid id is a braid of this theory or a typed refuse. */
function braidOf(theory: Schema<SchemaRelations> | Schema<never>, raw: Braid): Braid {
	const id = Number.parseInt(raw.slice(1), 16)
	const parsed = parse(theory, id)
	if (parsed === undefined) {
		refuse({ kind: "UnknownBraid" }, `unknown braid ${raw}`)
	}
	return parsed
}

function zeroChain(descriptor: Descriptor): Map<Braid, ChainEntry> {
	const chain = new Map<Braid, ChainEntry>()
	for (const id of descriptor.braidMembers.keys()) {
		chain.set(id, { g: generation(0n), prev: ZERO_HASH, ts: 0n })
	}
	return chain
}

function entriesOf<Rels extends SchemaRelations>(core: Core<Rels>): Map<Braid, ChainEntry> {
	return core.chain.entries as Map<Braid, ChainEntry>
}

function chainEntry<Rels extends SchemaRelations>(core: Core<Rels>, braid: Braid): ChainEntry {
	const id = braidOf(core.theory, braid)
	const entry = entriesOf(core).get(id)
	if (entry === undefined) {
		throw errors.new(`braid ${id} is not derived from this theory`)
	}
	return entry
}

function generationOf<Rels extends SchemaRelations>(core: Core<Rels>): bigint {
	return core.db.read(function readGeneration(instance) {
		return instance.generation
	})
}

/** The opened store's catalog digest — the engine handle's computed claim. */
function catalogDigestOf<Rels extends SchemaRelations>(core: Core<Rels>): Digest32 {
	return digest32(core.db.catalogDigest())
}

/** Pending-arm payload: recorded ops and the batch header's timestamp
 *  ride on the batch, not a third Option beside the vector. Wire
 *  Pending is {braid,gen,bytes}; ops/ts are the in-memory fields of
 *  that arm (§30) — null on a batch adopted as bytes the seat has not
 *  decoded. */
type HeldBatch = Pending & { readonly ops: readonly Op[] | null; readonly ts: bigint | null }

function holdPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	batch: Pending,
	ops: readonly Op[] | null,
	ts: bigint | null
): void {
	const held: HeldBatch = { braid: batch.braid, slot: batch.slot, bytes: batch.bytes, ops, ts }
	core.chain = { tag: "pending", entries: entriesOf(core), batch: held }
}

function pendingOf<Rels extends SchemaRelations>(core: Core<Rels>): HeldBatch | null {
	if (core.chain.tag !== "pending") {
		return null
	}
	return core.chain.batch as HeldBatch
}

function settleHeld<Rels extends SchemaRelations>(core: Core<Rels>): void {
	core.chain = { tag: "settled", entries: entriesOf(core) }
}

/** The local vector equals the published checkpoint vector and the
 *  chain is Settled — the seed/open floor the catalog claim is audited
 *  against. */
function atCheckpointFloor<Rels extends SchemaRelations>(core: Core<Rels>): boolean {
	if (core.checkpoint === null || core.chain.tag === "pending") {
		return false
	}
	for (const [braid, head] of core.checkpoint.braids) {
		if (chainEntry(core, braid).g !== head.g) {
			return false
		}
	}
	return true
}

async function persistSidecar<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	await writeSidecar(core.descriptor.codec, sidecarPath(core), core.chain)
}

/** Writes the checkpoint store file and fsyncs the file and its parent
 *  directory — the mdb is durable before the sidecar. */
async function writeCheckpointMdb(dir: string, bytes: Uint8Array): Promise<void> {
	const data = path.join(dir, "data.mdb")
	const handle = await fs.open(data, "w")
	const written = await errors.try(
		(async function writeAll() {
			await handle.writeFile(bytes)
			await handle.sync()
		})()
	)
	await handle.close()
	if (written.error) {
		throw errors.wrap(written.error, `write checkpoint store ${data}`)
	}
	const parent = await fs.open(dir, "r")
	const synced = await errors.try(parent.sync())
	await parent.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync checkpoint store directory ${dir}`)
	}
}

async function settle<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	settleHeld(core)
	await persistSidecar(core)
}

async function clearPending<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	await settle(core)
}

function disposed<Rels extends SchemaRelations>(core: Core<Rels>): void {
	if (core.closed) {
		throw errors.new("replica is disposed")
	}
}

function adoptChain<Rels extends SchemaRelations>(core: Core<Rels>, chain: Chain): void {
	const entries = new Map<Braid, ChainEntry>()
	for (const [raw, entry] of chain.entries) {
		entries.set(braidOf(core.theory, raw), entry)
	}
	for (const id of core.descriptor.braidMembers.keys()) {
		if (!entries.has(id)) {
			entries.set(id, { g: generation(0n), prev: ZERO_HASH, ts: 0n })
		}
	}
	if (chain.tag === "pending") {
		const held: HeldBatch = {
			braid: braidOf(core.theory, chain.batch.braid),
			slot: chain.batch.slot,
			bytes: chain.batch.bytes,
			ops: null,
			ts: null
		}
		core.chain = { tag: "pending", entries, batch: held }
	} else {
		core.chain = { tag: "settled", entries }
	}
}

/** One `db.write` applying ops in listed order, rows in listed order.
 *  The positional row → named fact lift is the engine's `factOf` —
 *  closed-ref handles included. */
function applyOps<Rels extends SchemaRelations>(core: Core<Rels>, ops: readonly Op[]): WriteOutcome<Rels, number> {
	return core.db.write(function applyBatch(tx) {
		for (const op of ops) {
			const member = core.theory.relations[op.relation]
			if (member === undefined) {
				throw errors.new(`batch op cites unknown relation ${op.relation}`)
			}
			const relation = member as MemberRelation<Rels>
			const facts = op.rows.map(function liftRow(row) {
				return factOf(relation, row)
			})
			if (op.op === "insert") {
				tx.insert(relation, facts)
			} else {
				tx.delete(relation, facts)
			}
		}
		return 0
	})
}

/**
 * Decode, verify the chain, `db.write` the batch, then refuse a
 * first-applied no-op against the identity. The sidecar advances
 * only after that check; a rejected replay is phase-scoped:
 * discard before the store has proven itself whole,
 * `ErrReplayDiverged` after.
 */
async function applySlot<Rels extends SchemaRelations>(
	core: Core<Rels>,
	braid: Braid,
	slot: Generation,
	bytes: Uint8Array,
	phase: ApplyPhase
): Promise<SlotApply> {
	const decoded = decodeBatch(core.descriptor, bytes)
	const entry = chainEntry(core, braid)
	verifyChain(decoded.header, braid, slot, { g: entry.g, prev: entry.prev, ts: entry.ts })
	const outcome = applyOps(core, decoded.ops)
	if (outcome.tag === "rejected") {
		if (phase === "open") {
			return { tag: "discard" }
		}
		throw errors.wrap(ErrReplayDiverged, `braid ${braid} slot ${slot} writer ${decoded.header.writer}`)
	}
	const identity = chainSum(core.chain) - entry.g + slot
	if (outcome.value.generation < identity) {
		refuse(
			{ kind: "NoOpSlot", braid, slot, writer: decoded.header.writer },
			`braid ${braid} slot ${slot}: a first-applied slot changed nothing — publish-law violation by writer ${decoded.header.writer}`
		)
	}
	entriesOf(core).set(braid, { g: slot, prev: blake3Digest(bytes), ts: decoded.header.timestamp })
	await persistSidecar(core)
	return { tag: "applied", generation: outcome.value.generation }
}

/**
 * The published checkpoint vector is the one floor: a slot at or
 * below it is retired, and a create must not touch the store.
 */
function belowFloor<Rels extends SchemaRelations>(core: Core<Rels>, braid: Braid, slot: Generation): boolean {
	const id = braidOf(core.theory, braid)
	if (core.checkpoint === null) {
		return false
	}
	const floor = core.checkpoint.braids.get(id)
	if (floor === undefined) {
		return false
	}
	return slot <= floor.g
}

/**
 * The gc floor rule: below the current checkpoint's vector a 404 is a
 * collected hole, at or above it the honest tip.
 */
function holeAt<Rels extends SchemaRelations>(core: Core<Rels>, braid: Braid, next: Generation): boolean {
	return belowFloor(core, braid, next)
}

/**
 * Classification of a pending batch against occupant, store
 * generation, and the floor. BelowFloor is published (`Clear`) — the
 * slot is already in the trusted history, not a candidate to re-judge.
 */
type PendingFold =
	| { readonly tag: "ours" }
	| { readonly tag: "theirs-unapplied" }
	| { readonly tag: "theirs-applied" }
	| { readonly tag: "absent-unapplied" }
	| { readonly tag: "absent-applied" }
	| { readonly tag: "below-floor" }
	| { readonly tag: "phantom" }

function foldPending(
	sum: bigint,
	generation: bigint,
	occupant: Uint8Array | null,
	pendingBytes: Uint8Array,
	covered: boolean
): PendingFold {
	if (covered) {
		return { tag: "below-floor" }
	}
	if (occupant !== null && bytesEqual(occupant, pendingBytes)) {
		return { tag: "ours" }
	}
	if (occupant !== null) {
		return generation === sum ? { tag: "theirs-unapplied" } : { tag: "theirs-applied" }
	}
	if (generation === sum) {
		return { tag: "absent-unapplied" }
	}
	if (generation === saturatingAddU64(sum, 1n)) {
		return { tag: "absent-applied" }
	}
	return { tag: "phantom" }
}

function isCorruption(error: unknown): boolean {
	if (!(error instanceof Error)) {
		return false
	}
	return errors.is(error, ErrReplayDiverged) || errors.is(error, ErrRefused)
}

/**
 * One braid, one slot. Catch-up, refresh, waitFor, and open all call
 * this; a hot braid cannot drain past one step per round.
 */
async function stepBraid<Rels extends SchemaRelations>(
	core: Core<Rels>,
	braid: Braid,
	phase: ApplyPhase
): Promise<Step> {
	disposed(core)
	const id = braidOf(core.theory, braid)
	const wedged = core.wedged.get(id)
	if (wedged !== undefined) {
		return { tag: "wedged", braid: id, cause: wedged }
	}
	const next = generation(chainEntry(core, id).g + 1n)
	const fetched = await core.store.get(logKey(core.prefix, id, next))
	if (fetched === null) {
		if (holeAt(core, id, next)) {
			return { tag: "reseed", cause: "gap-below-floor" }
		}
		return { tag: "tip" }
	}
	const chain = core.chain
	if (chain.tag === "pending" && id === chain.batch.braid && next === chain.batch.slot) {
		if (!bytesEqual(fetched.bytes, chain.batch.bytes)) {
			return { tag: "reseed", cause: "lost-pending-fork" }
		}
		await settle(core)
	}
	const applied = await errors.try(applySlot(core, id, next, fetched.bytes, phase))
	if (applied.error) {
		if (phase === "steady" && isCorruption(applied.error)) {
			core.wedged.set(id, applied.error.message)
			return { tag: "wedged", braid: id, cause: applied.error.message }
		}
		throw applied.error
	}
	if (applied.data.tag === "discard") {
		return { tag: "reseed", cause: "rejected-in-open" }
	}
	return { tag: "applied" }
}

/**
 * One pass: heartbeat, one slot per braid, wholeness, disposed. The
 * same function refresh, waitFor, catch-up, and open execute.
 */
async function runPass<Rels extends SchemaRelations>(
	core: Core<Rels>,
	braids: readonly Braid[],
	phase: ApplyPhase
): Promise<RefreshOutcome> {
	disposed(core)
	core.passes += 1
	if (core.passes % HEARTBEAT_PASSES === 0) {
		await refreshManifest(core)
	}
	const remaining = new Set(braids)
	while (remaining.size > 0) {
		disposed(core)
		let progressed = false
		for (const braid of braids) {
			if (!remaining.has(braid)) {
				continue
			}
			const step = await stepBraid(core, braid, phase)
			if (step.tag === "applied") {
				progressed = true
			} else if (step.tag === "tip" || step.tag === "wedged") {
				remaining.delete(braid)
			} else {
				return { tag: "reseed", cause: step.cause }
			}
		}
		if (!progressed) {
			break
		}
	}
	if (!wholenessHolds(core)) {
		return { tag: "reseed", cause: "wholeness" }
	}
	for (const [braid, cause] of core.wedged) {
		return { tag: "wedged", braid, cause }
	}
	return { tag: "advanced", vector: vectorOf(core) }
}

function allBraids<Rels extends SchemaRelations>(core: Core<Rels>): Braid[] {
	return [...core.descriptor.braidMembers.keys()]
}

function wholenessHolds<Rels extends SchemaRelations>(core: Core<Rels>): boolean {
	return generationOf(core) === chainGeneration(core.chain)
}

async function adoptManifest<Rels extends SchemaRelations>(
	core: Core<Rels>,
	bytes: Uint8Array,
	etag: Etag
): Promise<void> {
	const manifest = parseManifest(bytes)
	if (!bytesEqual(manifest.fingerprint, digest32(core.descriptor.fingerprintBytes))) {
		refuse(
			{
				kind: "FingerprintMismatch",
				carried: hex32(manifest.fingerprint),
				expected: core.descriptor.fingerprint
			},
			"the store's manifest names a different theory"
		)
	}
	if (manifest.checkpoint === null) {
		core.checkpoint = null
		core.checkpointDigest = null
	} else if (core.checkpointDigest === null || !bytesEqual(manifest.checkpoint, core.checkpointDigest)) {
		const facts = await core.store.get(ckptDocKey(core.prefix, manifest.checkpoint))
		if (facts === null) {
			throw errors.new(`manifest points at absent checkpoint ${hex32(manifest.checkpoint)}`)
		}
		// The codec-backed parseCheckpoint judges the braid set against the
		// sealed handle — an unknown or drifted braid refuses at parse.
		core.checkpoint = parseCheckpoint(core.descriptor.codec, facts.bytes)
		core.checkpointDigest = manifest.checkpoint
	}
	// The pointer is adopted only after the checkpoint it names is in
	// hand. A failed fetch leaves the old etag and the old floor (40/67).
	core.manifestEtag = etag
}

/** A replica never births a manifest. Absence is ManifestMissing. */
async function refreshManifest<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	if (core.manifestEtag !== null) {
		const poll = await core.store.getIfChanged(manifestKey(core.prefix), core.manifestEtag)
		if (poll.tag === "unchanged") {
			return
		}
		await adoptManifest(core, poll.fetched.bytes, poll.fetched.etag)
		return
	}
	const fetched = await core.store.get(manifestKey(core.prefix))
	if (fetched === null) {
		refuseManifestMissing("the store has no manifest")
	}
	await adoptManifest(core, fetched.bytes, fetched.etag)
}

/** Bootstraps a fresh local store: from the current checkpoint when one
 *  exists, else `Db.create` at the zero vector. Always holds Settled —
 *  a prior Pending arm cannot survive beside the new vector. */
async function initializeStore<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	core.storeName = freshStoreName()
	const target = storePath(core)
	await fs.rm(target, { recursive: true, force: true })
	await fs.mkdir(core.dir, { recursive: true })
	if (core.checkpoint !== null && core.checkpointDigest !== null) {
		const mdb = await core.store.get(checkpointMdbKey(core.prefix, core.checkpointDigest))
		if (mdb === null) {
			throw errors.new(`checkpoint ${hex32(core.checkpointDigest)} names an absent .mdb`)
		}
		await fs.mkdir(target, { recursive: true })
		await writeCheckpointMdb(target, mdb.bytes)
		core.db = await SdkDb.open(target, core.theory)
		core.chain = {
			tag: "settled",
			entries: new Map(
				[...core.checkpoint.braids.entries()].map(function seed([raw, head]) {
					const braid = braidOf(core.theory, raw)
					return [braid, { g: head.g, prev: head.hash, ts: head.ts }] as const
				})
			)
		}
		const opened = generationOf(core)
		if (opened !== chainGeneration(core.chain)) {
			throw errors.new(
				`checkpoint store opened at generation ${opened}, chain generation is ${chainGeneration(core.chain)}`
			)
		}
		auditCatalog(core.checkpoint, catalogDigestOf(core))
		core.provenance = { tag: "checkpoint-seeded", catalog: core.checkpointDigest }
		await persistSidecar(core)
		return
	}
	const created = await SdkDb.create(target, core.theory)
	if (created.tag === "rejected") {
		throw errors.new("the theory's ground axioms were rejected at bootstrap")
	}
	core.db = created.value
	core.chain = { tag: "settled", entries: zeroChain(core.descriptor) }
	core.provenance = { tag: "bootstrapped" }
	await persistSidecar(core)
}

/** The scream tracks the set of repair signatures; a recurrence alarms. */
function screamOf(context: string): { attempt(signature: string): void } {
	const seen = new Set<string>()
	let attempts = 0
	return {
		attempt(signature: string): void {
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

/** The disposable law: the directory is cache, never truth. The local
 *  LMDB path rotates because the engine has no close verb — the old
 *  environment is left for GC while the fresh pull takes a new path.
 *  initializeStore writes Settled, so the open-identity is
 *  chainGeneration(core.chain) — there is no addend a reader can skip. */
async function discardAndReopen<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const scream = screamOf("replica discard-and-re-pull")
	for (;;) {
		const old = storePath(core)
		await fs.rm(old, { recursive: true, force: true })
		core.wedged.clear()
		await initializeStore(core)
		const outcome = await runPass(core, allBraids(core), "open")
		if (outcome.tag === "reseed") {
			scream.attempt(outcome.cause)
			continue
		}
		if (outcome.tag === "refused") {
			throw errors.wrap(ErrRefused, outcome.detail)
		}
		if (generationOf(core) === chainGeneration(core.chain)) {
			return
		}
		scream.attempt("wholeness")
	}
}

async function newestStoreDir(dir: string): Promise<string | null> {
	const listed = await errors.try(fs.readdir(dir))
	if (listed.error) {
		return null
	}
	let newest: string | null = null
	let newestAt = -1
	for (const name of listed.data) {
		if (!name.startsWith("store-")) {
			continue
		}
		const stat = await errors.try(fs.stat(path.join(dir, name)))
		if (stat.error === undefined && stat.data.mtimeMs > newestAt) {
			newestAt = stat.data.mtimeMs
			newest = name
		}
	}
	return newest
}

/**
 * Open reclaims the reserved `~tmp`/`~lease` namespace. The known
 * document `{dir}/~lease/ckpt-scratch` names a crash-stranded
 * candidate; when that digest is not the live head the ckpt pair is
 * deleted with the lease.
 */
async function sweepReservedKeys<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const lease = path.join(core.dir, LEASE_NAMESPACE, CKPT_SCRATCH_LEASE)
	const read = await errors.try(fs.readFile(lease))
	if (read.error === undefined) {
		const digest = parseCkptScratch(read.data)
		if (digest !== null && (core.checkpointDigest === null || !bytesEqual(digest, core.checkpointDigest))) {
			await core.store.delete(ckptDocKey(core.prefix, digest))
			await core.store.delete(checkpointMdbKey(core.prefix, digest))
		}
	}
	await fs.rm(path.join(core.dir, TEMP_NAMESPACE), { recursive: true, force: true })
	await fs.rm(path.join(core.dir, LEASE_NAMESPACE), { recursive: true, force: true })
}

/** The disposable law says cache directories do not hoard corpses: every
 *  rotated `store-*` LMDB dir except the adopted one is dead — left by a
 *  crashed process or a prior rotation — and is swept at open. */
async function sweepRotations<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const listed = await errors.try(fs.readdir(core.dir))
	if (listed.error) {
		return
	}
	for (const name of listed.data) {
		if (!name.startsWith("store-") || name === core.storeName) {
			continue
		}
		await fs.rm(path.join(core.dir, name), { recursive: true, force: true })
	}
}

/**
 * Pending recovery: apply the resurrected batch. A batch rejected here
 * was never acked; a born-no-op settles; anything else stays Pending
 * so generation(chain) accounts for the unpublished apply.
 */
async function resolvePendingAtOpen<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const chain = core.chain
	if (chain.tag === "settled") {
		return
	}
	// A slot the floor already covers is published (`Clear`), not re-judged (46).
	if (
		foldPending(
			chainSum(core.chain),
			generationOf(core),
			null,
			chain.batch.bytes,
			belowFloor(core, chain.batch.braid, chain.batch.slot)
		).tag === "below-floor"
	) {
		await settle(core)
		return
	}
	const decoded = errors.trySync(function decodePending() {
		return decodeBatch(core.descriptor, chain.batch.bytes)
	})
	if (decoded.error) {
		await settle(core)
		return
	}
	const outcome = applyOps(core, decoded.data.ops)
	if (outcome.tag === "rejected") {
		await settle(core)
		return
	}
	// Store matches chainGeneration of the held Pending (sum+1) when the
	// apply is in the db — just now, or already. A born-no-op leaves the
	// store at the Settled generation, so the identity fails and we settle.
	if (generationOf(core) !== chainGeneration(core.chain)) {
		await settle(core)
		return
	}
	holdPending(core, chain.batch, decoded.data.ops, decoded.data.header.timestamp)
}

/**
 * Pending resurrection through a cold re-pull: the store directory was
 * unopenable (or discarded), so the sidecar's pending is resolved
 * against a freshly caught-up store — a byte-equal published slot
 * absorbs it (L10's idempotent replay), and everything else takes the
 * one loss path's re-judgment at the tip. A slot the floor already
 * covers is published (`Clear`), not re-judged (46).
 */
async function resolveColdPending<Rels extends SchemaRelations>(core: Core<Rels>, pending: Pending): Promise<void> {
	const braid = braidOf(core.theory, pending.braid)
	if (
		foldPending(chainSum(core.chain), generationOf(core), null, pending.bytes, belowFloor(core, braid, pending.slot))
			.tag === "below-floor"
	) {
		await settle(core)
		return
	}
	const decoded = errors.trySync(function decodePending() {
		return decodeBatch(core.descriptor, pending.bytes)
	})
	if (decoded.error) {
		return
	}
	const slot = decoded.data.header.braidGen
	if (entriesOf(core).has(braid) && chainEntry(core, braid).g >= slot) {
		const published = await core.store.get(logKey(core.prefix, braid, slot))
		if (published !== null && bytesEqual(published.bytes, pending.bytes)) {
			return
		}
	}
	const before = generationOf(core)
	const outcome = applyOps(core, decoded.data.ops)
	if (outcome.tag === "accepted" && outcome.value.generation > before) {
		holdPending(core, { ...pending, braid }, decoded.data.ops, decoded.data.header.timestamp)
		await readdressPending(core, decoded.data.ops, decoded.data.header.writer)
	}
}

async function openCore<Rels extends SchemaRelations>(options: OpenReplicaOptions<Rels>): Promise<Core<Rels>> {
	const descriptor = descriptorOf(options.theory)
	const core: Core<Rels> = {
		store: options.store,
		prefix: options.prefix,
		dir: path.resolve(options.dir),
		theory: options.theory,
		descriptor,
		db: undefined as unknown as Db<Rels>,
		chain: { tag: "settled", entries: zeroChain(descriptor) },
		manifestEtag: null,
		checkpoint: null,
		checkpointDigest: null,
		passes: 0,
		closed: false,
		storeName: freshStoreName(),
		gate: Promise.resolve(),
		wedged: new Map(),
		provenance: { tag: "bootstrapped" }
	}
	await fs.mkdir(core.dir, { recursive: true })
	const fetched = await core.store.get(manifestKey(core.prefix))
	if (fetched === null) {
		refuseManifestMissing("the store has no manifest")
	}
	await adoptManifest(core, fetched.bytes, fetched.etag)

	const existing = await newestStoreDir(core.dir)
	const sidecar = await readSidecar(descriptor.codec, sidecarPath(core))
	let opened = false
	let coldPending: Pending | null = null
	if (sidecar.tag === "fault") {
		throw wrapStore(sidecar.io, `read sidecar ${sidecarPath(core)}`)
	}
	// The codec-backed readSidecar judges every braid against the sealed
	// handle — a foreign braid is a corrupt sidecar, and a corrupt
	// sidecar is discarded cache (the disposable law), never adopted.
	if (existing !== null && sidecar.tag === "read") {
		core.storeName = existing
		const openedDb = await errors.try(SdkDb.open(storePath(core), options.theory))
		if (openedDb.error === undefined) {
			core.db = openedDb.data
			adoptChain(core, sidecar.chain)
			core.provenance = { tag: "sidecar-resumed", floor: vectorOf(core) }
			opened = true
		}
	}
	if (!opened) {
		if (sidecar.tag === "read" && sidecar.chain.tag === "pending") {
			coldPending = sidecar.chain.batch
		}
		await initializeStore(core)
	}

	if (opened) {
		await resolvePendingAtOpen(core)
	}

	const outcome = await runPass(core, allBraids(core), "open")
	if (outcome.tag === "reseed" || outcome.tag === "refused" || !wholenessHolds(core)) {
		coldPending = core.chain.tag === "pending" ? core.chain.batch : coldPending
		await discardAndReopen(core)
	}

	if (coldPending !== null) {
		await resolveColdPending(core, coldPending)
	}
	if (atCheckpointFloor(core)) {
		auditCatalog(core.checkpoint, catalogDigestOf(core))
	}
	await sweepRotations(core)
	await sweepReservedKeys(core)
	return core
}

/**
 * The steady-state discard route: a contested pending slot surrenders
 * the directory. The pending batch rides in memory — the sidecar is
 * not settled ahead of the re-judgment — and takes the one loss path
 * at the fresh tip.
 */
async function repairDiscard<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const coldPending = core.chain.tag === "pending" ? core.chain.batch : null
	await discardAndReopen(core)
	if (coldPending !== null) {
		await resolveColdPending(core, coldPending)
	}
}

/**
 * Re-addresses an applied-but-unpublished batch at the braid's current
 * tip: fresh slot, `prev` citing the new predecessor, timestamp
 * re-clamped — the ops are exactly the recorded ops the re-judgment
 * just accepted. Publication itself retries on the next commit (60).
 */
async function readdressPending<Rels extends SchemaRelations>(
	core: Core<Rels>,
	ops: readonly Op[],
	writerId: bigint
): Promise<void> {
	const first = ops[0]
	if (first === undefined) {
		await settle(core)
		return
	}
	const relation = core.descriptor.relationByName.get(first.relation)
	const raw = relation === undefined ? undefined : core.descriptor.braidOfRelation.get(relation.id)
	if (raw === undefined) {
		await settle(core)
		return
	}
	const braid = braidOf(core.theory, raw)
	const entry = chainEntry(core, braid)
	const timestamp = maxBigint(BigInt(Date.now()), entry.ts)
	const bytes = encodeBatch(
		core.descriptor,
		{
			braid,
			braidGen: generation(entry.g + 1n),
			prev: entry.prev,
			writer: writerId,
			timestamp
		},
		ops
	)
	holdPending(core, { braid, slot: generation(entry.g + 1n), bytes }, ops, timestamp)
	await persistSidecar(core)
}

function maxBigint(a: bigint, b: bigint): bigint {
	return a > b ? a : b
}

function vectorOf<Rels extends SchemaRelations>(core: Core<Rels>): ReadonlyMap<Braid, Generation> {
	const vector = new Map<Braid, Generation>()
	for (const [id, entry] of core.chain.entries) {
		vector.set(id, entry.g)
	}
	return vector
}

async function refreshPass<Rels extends SchemaRelations>(core: Core<Rels>, braid?: Braid): Promise<RefreshOutcome> {
	const braids = braid === undefined ? allBraids(core) : [braidOf(core.theory, braid)]
	const outcome = await runPass(core, braids, "steady")
	if (outcome.tag === "reseed") {
		await repairDiscard(core)
		return { tag: "reseed", cause: outcome.cause }
	}
	return outcome
}

/** waitFor is refresh with a verdict: the same pass, then the full
 *  Waited sum. A braid the target needs that is wedged below it returns
 *  Wedged — no refresh will ever reach the target — and a heartbeat
 *  refusal returns Refused. */
async function waitForVector<Rels extends SchemaRelations>(
	core: Core<Rels>,
	target: ReadonlyMap<Braid, Generation>
): Promise<Waited> {
	for (;;) {
		disposed(core)
		const have = vectorOf(core)
		for (const [braid, wanted] of target) {
			const cause = core.wedged.get(braid)
			const at = have.get(braid)
			if (cause !== undefined && at !== undefined && at < wanted) {
				return { tag: "wedged", braid, cause }
			}
		}
		if (Vector.from(have).dominates(Vector.from(target))) {
			return { tag: "reached", vector: have }
		}
		const outcome = await withGate(core, async function waitPass() {
			disposed(core)
			return refreshPass(core)
		})
		if (outcome.tag === "refused") {
			return { tag: "refused", detail: outcome.detail }
		}
		await new Promise(function later(resolve) {
			setTimeout(resolve, WAIT_FOR_POLL_MS)
		})
	}
}

async function openReplica<Rels extends SchemaRelations>(options: OpenReplicaOptions<Rels>): Promise<Replica<Rels>> {
	const core = await openCore(options)
	const replica: Replica<Rels> = {
		get db() {
			disposed(core)
			return core.db
		},
		get vector() {
			return vectorOf(core)
		},
		async refresh(braid?: Braid) {
			return withGate(core, async function refreshBody() {
				disposed(core)
				await refreshPass(core, braid)
				return vectorOf(core)
			})
		},
		async waitFor(vector) {
			const target = new Map<Braid, Generation>()
			for (const [braid, wanted] of vector) {
				target.set(braidOf(core.theory, braid), wanted)
			}
			return waitForVector(core, target)
		},
		async [Symbol.asyncDispose]() {
			await withGate(core, async function disposeBody() {
				core.closed = true
				await persistSidecar(core)
			})
		}
	}
	cores.set(replica, core as unknown as Core<SchemaRelations>)
	return replica
}

export type { Core, OpenReplicaOptions, RefreshOutcome, Replica, Waited }
export {
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
	withGate,
	ZERO_HASH
}
