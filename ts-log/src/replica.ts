/**
 * The replica (50): a local store that is a materialized view of the
 * braids' prefixes, plus the loop that keeps it current. Disposable by
 * construction — the only local protocol state is the chain sidecar,
 * a floor cache with one wholeness check; recovery IS the catch-up
 * loop (L10), never a procedure. The store's engine generation is the
 * vector sum; `generation == Σ chain + |applied pending|` is the one
 * instrument, and anything else discards the directory and re-pulls.
 */

import * as fs from "node:fs/promises"
import * as path from "node:path"
import type { Db, Fact, MemberRelation, Schema, SchemaRelations, WriteOutcome } from "@bjornpagen/bumbledb"
import { internalBlake3, Db as SdkDb } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { bytesEqual, toHex } from "#bytes.ts"
import type { ChainEntry, Pending } from "#chain.ts"
import { readSidecar, writeSidecar } from "#chain.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import type { Braid, Descriptor, RelationInfo } from "#descriptor.ts"
import { descriptorOf } from "#descriptor.ts"
import { ErrGapDetected, ErrReplayDiverged, refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { checkpointJsonKey, checkpointMdbKey, generation, logKey, manifestKey } from "#keys.ts"
import type { CheckpointFacts } from "#manifest.ts"
import { parseCheckpoint, parseManifest, renderManifest } from "#manifest.ts"
import type { Etag, ObjectStore } from "#store.ts"
import type { Value } from "#value.ts"

const ZERO_HASH = "0".repeat(64)

/** The gc-safety heartbeat cadence (50): every N-th refresh pass re-reads the manifest. */
const HEARTBEAT_PASSES = 16

/** The re-poll cadence between waitFor's catch-up passes; the
 *  read-your-writes waiter in `waitFor` is this number's one consumer. */
const WAIT_FOR_POLL_MS = 20

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
	waitFor(vector: ReadonlyMap<Braid, Generation>): Promise<void>
}

interface Core<Rels extends SchemaRelations> {
	readonly store: ObjectStore
	readonly prefix: string
	readonly dir: string
	readonly theory: Schema<Rels>
	readonly descriptor: Descriptor
	db: Db<Rels>
	chain: Map<Braid, ChainEntry>
	pending: Pending | null
	pendingOps: readonly Op[] | null
	pendingApplied: boolean
	manifestEtag: Etag | null
	checkpoint: CheckpointFacts | null
	checkpointDigest: string | null
	passes: number
	closed: boolean
	storeName: string
	gate: Promise<unknown>
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

function blake3Hex(bytes: Uint8Array): string {
	return toHex(new Uint8Array(internalBlake3(bytes)))
}

function sidecarPath<Rels extends SchemaRelations>(core: Core<Rels>): string {
	return path.join(core.dir, "chain.json")
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

function zeroChain(descriptor: Descriptor): Map<Braid, ChainEntry> {
	const chain = new Map<Braid, ChainEntry>()
	for (const id of descriptor.braidMembers.keys()) {
		chain.set(id, { g: generation(0n), prev: ZERO_HASH, ts: 0n })
	}
	return chain
}

function chainEntry<Rels extends SchemaRelations>(core: Core<Rels>, braid: Braid): ChainEntry {
	const entry = core.chain.get(braid)
	if (entry === undefined) {
		throw errors.new(`braid ${braid} is not derived from this theory`)
	}
	return entry
}

function generationOf<Rels extends SchemaRelations>(core: Core<Rels>): bigint {
	return core.db.read(function readGeneration(instance) {
		return instance.generation
	})
}

function chainSum<Rels extends SchemaRelations>(core: Core<Rels>): bigint {
	let sum = 0n
	for (const entry of core.chain.values()) {
		sum += entry.g
	}
	return sum
}

function pendingTerm<Rels extends SchemaRelations>(core: Core<Rels>): bigint {
	return core.pendingApplied ? 1n : 0n
}

async function persistSidecar<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	await writeSidecar(sidecarPath(core), { chain: core.chain, pending: core.pending })
}

async function clearPending<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	core.pending = null
	core.pendingOps = null
	core.pendingApplied = false
	await persistSidecar(core)
}

/** Positional decoded row → the SDK's named fact, handles lifted for closed refs. */
function factOf<Rels extends SchemaRelations>(
	core: Core<Rels>,
	relation: RelationInfo,
	row: readonly Value[]
): Record<string, unknown> {
	const fact: Record<string, unknown> = {}
	relation.fields.forEach(function liftCell(field, ordinal) {
		const value = row[ordinal]
		if (value === undefined) {
			throw errors.new(`relation ${relation.name}: decoded row cell ${ordinal} absent`)
		}
		if (field.closedRef !== undefined && typeof value === "bigint") {
			const roster = core.descriptor.relationByName.get(field.closedRef)
			const handle = roster?.handles[Number(value)]
			if (handle === undefined) {
				throw errors.new(
					`relation ${relation.name}.${field.name}: id ${value} is outside the ${field.closedRef} roster`
				)
			}
			fact[field.name] = handle
			return
		}
		fact[field.name] = value
	})
	return fact
}

/** One `db.write` applying ops in listed order, rows in listed order. */
function applyOps<Rels extends SchemaRelations>(core: Core<Rels>, ops: readonly Op[]): WriteOutcome<Rels, number> {
	return core.db.write(function applyBatch(tx) {
		for (const op of ops) {
			const info = core.descriptor.relationByName.get(op.relation)
			const member = core.theory.relations[op.relation]
			if (info === undefined || member === undefined) {
				throw errors.new(`batch op cites unknown relation ${op.relation}`)
			}
			const relation = member as MemberRelation<Rels>
			const facts = op.rows.map(function liftRow(row) {
				return factOf(core, info, row)
			}) as unknown as Iterable<Fact<MemberRelation<Rels>>>
			if (op.op === "insert") {
				tx.insert(relation, facts)
			} else {
				tx.delete(relation, facts)
			}
		}
		return 0
	})
}

type ApplyPhase = "open" | "steady"

type SlotApply = { readonly tag: "applied"; readonly generation: bigint } | { readonly tag: "discard" }

/**
 * The two-step apply discipline: `db.write` the batch, then advance the
 * sidecar. Chain and publish-law refusals live here; a rejected replay
 * is phase-scoped (50): discard before the store has proven itself
 * whole, `ErrReplayDiverged` after.
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
	core.chain.set(braid, { g: slot, prev: blake3Hex(bytes), ts: decoded.header.timestamp })
	await persistSidecar(core)
	const expected = chainSum(core) + pendingTerm(core)
	if (outcome.value.generation < expected) {
		refuse(
			{ kind: "NoOpSlot", braid, slot, writer: decoded.header.writer },
			`braid ${braid} slot ${slot}: a first-applied slot changed nothing — publish-law violation by writer ${decoded.header.writer}`
		)
	}
	return { tag: "applied", generation: outcome.value.generation }
}

/**
 * The gc floor rule (10): below the current checkpoint's vector a 404 is
 * a collected hole, at or above it the honest tip.
 */
function holeAt<Rels extends SchemaRelations>(core: Core<Rels>, braid: Braid, next: Generation): boolean {
	if (core.checkpoint === null) {
		return false
	}
	const floor = core.checkpoint.braids.get(braid)
	if (floor === undefined) {
		return false
	}
	return next <= floor.g
}

async function catchUpBraid<Rels extends SchemaRelations>(
	core: Core<Rels>,
	braid: Braid,
	phase: ApplyPhase
): Promise<"tip" | "discard"> {
	for (;;) {
		const next = generation(chainEntry(core, braid).g + 1n)
		const fetched = await core.store.get(logKey(core.prefix, braid, next))
		if (fetched === null) {
			if (holeAt(core, braid, next)) {
				throw errors.wrap(ErrGapDetected, `braid ${braid} slot ${next} is below the checkpoint vector`)
			}
			return "tip"
		}
		if (core.pendingApplied && core.pending !== null && braid === core.pending.braid && next === core.pending.gen) {
			// An applied pending contests exactly its own slot: byte-equal
			// means our slot was already published and the pending ends
			// here; any other occupant is a lost race, and the store —
			// carrying the pending's effects on a stale base — is
			// discarded so the one loss path re-judges at the tip (L10).
			if (!bytesEqual(fetched.bytes, core.pending.bytes)) {
				return "discard"
			}
			core.pending = null
			core.pendingOps = null
			core.pendingApplied = false
			await persistSidecar(core)
		}
		const applied = await applySlot(core, braid, next, fetched.bytes, phase)
		if (applied.tag === "discard") {
			return "discard"
		}
	}
}

async function catchUpAll<Rels extends SchemaRelations>(
	core: Core<Rels>,
	phase: ApplyPhase
): Promise<"tip" | "discard"> {
	for (const braid of core.descriptor.braidMembers.keys()) {
		const outcome = await catchUpBraid(core, braid, phase)
		if (outcome === "discard") {
			return "discard"
		}
	}
	return "tip"
}

function wholenessHolds<Rels extends SchemaRelations>(core: Core<Rels>): boolean {
	return generationOf(core) === chainSum(core) + pendingTerm(core)
}

async function adoptManifest<Rels extends SchemaRelations>(
	core: Core<Rels>,
	bytes: Uint8Array,
	etag: Etag
): Promise<void> {
	const manifest = parseManifest(bytes)
	if (manifest.fingerprint !== core.descriptor.fingerprint) {
		refuse(
			{ kind: "FingerprintMismatch", carried: manifest.fingerprint, expected: core.descriptor.fingerprint },
			"the store's manifest names a different theory"
		)
	}
	core.manifestEtag = etag
	if (manifest.checkpoint === null) {
		core.checkpoint = null
		core.checkpointDigest = null
		return
	}
	if (manifest.checkpoint === core.checkpointDigest) {
		return
	}
	const facts = await core.store.get(checkpointJsonKey(core.prefix, manifest.checkpoint))
	if (facts === null) {
		throw errors.new(`manifest points at absent checkpoint ${manifest.checkpoint}`)
	}
	const checkpoint = parseCheckpoint(facts.bytes)
	const carried = [...checkpoint.braids.keys()].sort()
	const derived = [...core.descriptor.braidMembers.keys()].sort()
	if (carried.join(",") !== derived.join(",")) {
		refuse({ kind: "CheckpointBraids", carried, derived }, "checkpoint braid set drifted from the derived braids")
	}
	core.checkpoint = checkpoint
	core.checkpointDigest = manifest.checkpoint
}

async function refreshManifest<Rels extends SchemaRelations>(core: Core<Rels>, force: boolean): Promise<void> {
	if (!force && core.manifestEtag !== null) {
		const poll = await core.store.getIfChanged(manifestKey(core.prefix), core.manifestEtag)
		if (poll.tag === "unchanged") {
			return
		}
		await adoptManifest(core, poll.fetched.bytes, poll.fetched.etag)
		return
	}
	const fetched = await core.store.get(manifestKey(core.prefix))
	if (fetched !== null) {
		await adoptManifest(core, fetched.bytes, fetched.etag)
		return
	}
	const birth = renderManifest({ fingerprint: core.descriptor.fingerprint, checkpoint: null })
	const created = await core.store.putCreate(manifestKey(core.prefix), birth)
	if (created.tag === "created") {
		core.manifestEtag = created.etag
		core.checkpoint = null
		core.checkpointDigest = null
		return
	}
	const reread = await core.store.get(manifestKey(core.prefix))
	if (reread === null) {
		throw errors.new("manifest vanished between create refusal and re-read")
	}
	await adoptManifest(core, reread.bytes, reread.etag)
}

/** Bootstraps a fresh local store: from the current checkpoint when one
 *  exists, else `Db.create` at the zero vector. */
async function initializeStore<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	core.storeName = freshStoreName()
	const target = storePath(core)
	await fs.rm(target, { recursive: true, force: true })
	await fs.mkdir(core.dir, { recursive: true })
	if (core.checkpoint !== null && core.checkpointDigest !== null) {
		for (let attempt = 0; ; attempt++) {
			const mdb = await core.store.get(checkpointMdbKey(core.prefix, core.checkpointDigest))
			if (mdb === null) {
				throw errors.new(`checkpoint ${core.checkpointDigest} names an absent .mdb`)
			}
			const digest = blake3Hex(mdb.bytes)
			if (digest !== core.checkpointDigest) {
				if (attempt === 0) {
					continue
				}
				refuse(
					{ kind: "CheckpointDigest", expected: core.checkpointDigest, computed: digest },
					"checkpoint bytes do not hash to their own name"
				)
			}
			await fs.mkdir(target, { recursive: true })
			await fs.writeFile(path.join(target, "data.mdb"), mdb.bytes)
			core.db = await SdkDb.open(target, core.theory)
			core.chain = new Map(
				[...core.checkpoint.braids.entries()].map(function seed([braid, head]) {
					return [braid, { g: head.g, prev: head.hash, ts: head.ts }] as const
				})
			)
			let sum = 0n
			for (const head of core.checkpoint.braids.values()) {
				sum += head.g
			}
			const generation = generationOf(core)
			if (generation !== sum) {
				throw errors.new(`checkpoint store opened at generation ${generation}, its vector sums to ${sum}`)
			}
			await persistSidecar(core)
			return
		}
	}
	const created = await SdkDb.create(target, core.theory)
	if (created.tag === "rejected") {
		throw errors.new("the theory's ground axioms were rejected at bootstrap")
	}
	core.db = created.value
	core.chain = zeroChain(core.descriptor)
	await persistSidecar(core)
}

/** The disposable law: the directory is cache, never truth. The local
 *  LMDB path rotates because the engine has no close verb — the old
 *  environment is left for GC while the fresh pull takes a new path. */
async function discardAndReopen<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	for (let attempt = 0; attempt < 3; attempt++) {
		const old = storePath(core)
		await fs.rm(old, { recursive: true, force: true })
		await initializeStore(core)
		const outcome = await catchUpAll(core, "open")
		if (outcome === "discard") {
			continue
		}
		if (generationOf(core) === chainSum(core)) {
			return
		}
	}
	throw errors.new("the store failed to reach wholeness after repeated re-pulls")
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
 * Pending recovery, first half (60): apply the resurrected batch; the
 * verdict plus the wholeness instrument force all three arms. A batch
 * rejected here was never acked (fsync preceded its first judgment); a
 * born-no-op publishes nothing; anything else is a real unpublished
 * commit that catch-up and the tip attempt will place.
 */
async function resolvePendingAtOpen<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	if (core.pending === null) {
		return
	}
	const decoded = errors.trySync(function decodePending() {
		return decodeBatch(core.descriptor, (core.pending as Pending).bytes)
	})
	if (decoded.error) {
		await clearPending(core)
		return
	}
	const before = generationOf(core)
	const outcome = applyOps(core, decoded.data.ops)
	if (outcome.tag === "rejected") {
		await clearPending(core)
		return
	}
	if (outcome.value.generation === before && before === chainSum(core)) {
		await clearPending(core)
		return
	}
	core.pendingOps = decoded.data.ops
	core.pendingApplied = true
}

/**
 * Pending resurrection through a cold re-pull: the store directory was
 * unopenable (or discarded), so the sidecar's pending is resolved
 * against a freshly caught-up store — a byte-equal published slot
 * absorbs it (L10's idempotent replay), and everything else takes the
 * one loss path's re-judgment at the tip.
 */
async function resolveColdPending<Rels extends SchemaRelations>(core: Core<Rels>, pending: Pending): Promise<void> {
	const decoded = errors.trySync(function decodePending() {
		return decodeBatch(core.descriptor, pending.bytes)
	})
	if (decoded.error) {
		return
	}
	const braid = decoded.data.header.braid
	const slot = decoded.data.header.braidGen
	if (core.chain.has(braid) && chainEntry(core, braid).g >= slot) {
		const published = await core.store.get(logKey(core.prefix, braid, slot))
		if (published !== null && bytesEqual(published.bytes, pending.bytes)) {
			return
		}
	}
	const before = generationOf(core)
	const outcome = applyOps(core, decoded.data.ops)
	if (outcome.tag === "accepted" && outcome.value.generation > before) {
		core.pendingOps = decoded.data.ops
		core.pendingApplied = true
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
		chain: zeroChain(descriptor),
		pending: null,
		pendingOps: null,
		pendingApplied: false,
		manifestEtag: null,
		checkpoint: null,
		checkpointDigest: null,
		passes: 0,
		closed: false,
		storeName: freshStoreName(),
		gate: Promise.resolve()
	}
	await fs.mkdir(core.dir, { recursive: true })
	await refreshManifest(core, true)

	const existing = await newestStoreDir(core.dir)
	const sidecar = await readSidecar(sidecarPath(core))
	let opened = false
	let coldPending: Pending | null = null
	if (existing !== null) {
		core.storeName = existing
		const openedDb = await errors.try(SdkDb.open(storePath(core), options.theory))
		if (openedDb.error === undefined && sidecar !== null) {
			core.db = openedDb.data
			core.chain = new Map(sidecar.chain)
			for (const braid of descriptor.braidMembers.keys()) {
				if (!core.chain.has(braid)) {
					core.chain.set(braid, { g: generation(0n), prev: ZERO_HASH, ts: 0n })
				}
			}
			core.pending = sidecar.pending
			opened = true
		}
	}
	if (!opened) {
		coldPending = sidecar?.pending ?? null
		await initializeStore(core)
	}

	if (opened) {
		await resolvePendingAtOpen(core)
	}

	const outcome = await catchUpAll(core, "open")
	if (outcome === "discard" || !wholenessHolds(core)) {
		coldPending = core.pending ?? coldPending
		await clearPending(core)
		await discardAndReopen(core)
	}

	if (coldPending !== null) {
		await resolveColdPending(core, coldPending)
	}
	await sweepRotations(core)
	return core
}

/**
 * The steady-state discard route: a catch-up that met a contested
 * pending slot (or a rejected replay at open phase) surrenders the
 * directory; the sidecar's pending rides through as a cold pending and
 * takes the one loss path's re-judgment at the fresh tip.
 */
async function repairDiscard<Rels extends SchemaRelations>(core: Core<Rels>): Promise<void> {
	const coldPending = core.pending
	await clearPending(core)
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
		await clearPending(core)
		return
	}
	const relation = core.descriptor.relationByName.get(first.relation)
	const braid = relation === undefined ? undefined : core.descriptor.braidOfRelation.get(relation.id)
	if (braid === undefined) {
		await clearPending(core)
		return
	}
	const entry = chainEntry(core, braid)
	const timestamp = maxBigint(BigInt(Date.now()), entry.ts)
	const bytes = encodeBatch(
		core.descriptor,
		{
			fingerprint: core.descriptor.fingerprint,
			braid,
			braidGen: generation(entry.g + 1n),
			prev: entry.prev,
			writer: writerId,
			timestamp
		},
		ops
	)
	core.pending = { braid, gen: generation(entry.g + 1n), bytes }
	core.pendingOps = ops
	core.pendingApplied = true
	await persistSidecar(core)
}

function maxBigint(a: bigint, b: bigint): bigint {
	return a > b ? a : b
}

function vectorOf<Rels extends SchemaRelations>(core: Core<Rels>): ReadonlyMap<Braid, Generation> {
	const vector = new Map<Braid, Generation>()
	for (const [id, entry] of core.chain) {
		vector.set(id, entry.g)
	}
	return vector
}

function dominates(have: ReadonlyMap<Braid, Generation>, want: ReadonlyMap<Braid, Generation>): boolean {
	for (const [braid, generation] of want) {
		if ((have.get(braid) ?? -1n) < generation) {
			return false
		}
	}
	return true
}

async function refreshPass<Rels extends SchemaRelations>(core: Core<Rels>, braid?: Braid): Promise<void> {
	core.passes += 1
	if (core.passes % HEARTBEAT_PASSES === 0) {
		await refreshManifest(core, false)
	}
	if (braid !== undefined) {
		chainEntry(core, braid)
		if ((await catchUpBraid(core, braid, "steady")) === "discard") {
			await repairDiscard(core)
		}
		return
	}
	if ((await catchUpAll(core, "steady")) === "discard") {
		await repairDiscard(core)
	}
}

async function openReplica<Rels extends SchemaRelations>(options: OpenReplicaOptions<Rels>): Promise<Replica<Rels>> {
	const core = await openCore(options)
	const replica: Replica<Rels> = {
		get db() {
			if (core.closed) {
				throw errors.new("replica is disposed")
			}
			return core.db
		},
		get vector() {
			return vectorOf(core)
		},
		async refresh(braid?: Braid) {
			return withGate(core, async function refreshBody() {
				if (core.closed) {
					throw errors.new("replica is disposed")
				}
				await refreshPass(core, braid)
				return vectorOf(core)
			})
		},
		async waitFor(vector) {
			for (const braid of vector.keys()) {
				chainEntry(core, braid)
			}
			for (;;) {
				if (dominates(vectorOf(core), vector)) {
					return
				}
				await withGate(core, async function waitPass() {
					for (const braid of vector.keys()) {
						if ((await catchUpBraid(core, braid, "steady")) === "discard") {
							await repairDiscard(core)
						}
					}
				})
				if (dominates(vectorOf(core), vector)) {
					return
				}
				await new Promise(function later(resolve) {
					setTimeout(resolve, WAIT_FOR_POLL_MS)
				})
			}
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

export type { ApplyPhase, Core, OpenReplicaOptions, Replica, SlotApply }
export {
	applyOps,
	applySlot,
	blake3Hex,
	catchUpBraid,
	chainEntry,
	chainSum,
	clearPending,
	coreOf,
	discardAndReopen,
	generationOf,
	maxBigint,
	openReplica,
	pendingTerm,
	persistSidecar,
	readdressPending,
	vectorOf,
	wholenessHolds,
	withGate,
	ZERO_HASH
}
