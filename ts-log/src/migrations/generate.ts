/**
 * `makeGenerator(codec, exclusion)` — the Drizzle-shaped repo-local generation workflow
 * (chapter 33, C11): `generateMigrations` / `checkMigrations`. Generation
 * evaluates the ordinary typed schema value, compares its NATIVE canonical
 * identity/snapshot against the recorded chain, applies declared typed intent
 * through the pure diff, ingests bounded declarative seed rows, and asks the
 * native codec to validate + render + digest the new plan and manifest. Files
 * are written with atomic per-file commits and the manifest last, so an
 * interruption leaves either the old recorded chain or the new one. `check`
 * repeats the same computation in memory and writes NOTHING.
 *
 * The user never writes a migration file: ambiguous rename/drop/backfill/
 * type/data intent is either declared as typed metadata or generation
 * refuses with the complete finite requirement list.
 *
 * The codec and repository-exclusion seams are the injected boundaries: the
 * production binding uses the native codec and kernel lock; authored language
 * tests may script the codec. Acceptance of exclusion/compile uses the native
 * stamped lock (no TS ownership table) and the full compiled chain. Core
 * schema lowering and the core row-cell codec are imported literally.
 */
import * as path from "node:path"
import { cellBytes, cellOf, lower } from "@bjornpagen/bumbledb"
import type { AnyRelation, ExecutionPolicy, NativeRuntime, RelationData, SchemaSpec } from "@bjornpagen/bumbledb"
import type { LogError } from "#errors.ts"
import { Effect } from "effect"
import type { Scope } from "effect"
import { compiledMappingsJson, planJson, renderContract, renderIndex, renderSnapshots } from "#migrations/canonical.ts"
import { bytesHex, f64Bits } from "#migrations/canonical.ts"
import type { JsonValue } from "#migrations/canonical.ts"
import type { ChainPayload, CompiledChainInput, MigrationCodec } from "#migrations/codec.ts"
import { decodePlanData } from "#migrations/decode.ts"
import { diffSchemas } from "#migrations/diff.ts"
import type { DiffResult } from "#migrations/diff.ts"
import { budget, drift, intentRequired, unsupported } from "#migrations/fail.ts"
import {
	ensureDirectory,
	joinPendingIo,
	readBounded,
	removeFile,
	writeDerived,
	writeImmutable,
	writeManifest
} from "#migrations/fsops.ts"
import type { MigrationIntentEntry } from "#migrations/intent.ts"
import type { RepositoryExclusion } from "#migrations/lock.ts"
import {
	baseSnapshotPath,
	contractPath,
	indexPath,
	latestSnapshot,
	manifestPath,
	planId,
	planPath,
	readRepository,
	snapshotPath,
	snapshotsSidecarPath
} from "#migrations/repo.ts"
import { parseTheory } from "#migrations/theory.ts"
import type {
	CheckOptions,
	CheckReport,
	GenerateOptions,
	GenerationReport,
	ManifestEntry,
	MigrationPlan,
	PlanOperation,
	PlanValue,
	RuntimeContract,
	TheorySnapshot
} from "#migrations/types.ts"
import type { SchemaRelations } from "@bjornpagen/bumbledb"

const EMPTY_SPEC: SchemaSpec = { relations: [], statements: [] }
const MAX_SEQUENCE = 9999
const MAX_LABEL = 59 // full id `NNNN-<label>` stays within the native 64 cap
const SEED_CHUNK = 512
const MAX_DERIVED_BYTES = 16 * 1024 * 1024

function validLabel(label: string): boolean {
	if (label.length === 0 || label.length > MAX_LABEL) {
		return false
	}
	for (const ch of label) {
		if (!((ch >= "a" && ch <= "z") || (ch >= "0" && ch <= "9") || ch === "-")) {
			return false
		}
	}
	return true
}

function deriveLabel(tokens: readonly string[]): string {
	const joined = tokens
		.filter((token) => token.length > 0)
		.slice(0, 3)
		.join("-")
	const label = joined.length === 0 ? "changes" : joined.slice(0, MAX_LABEL)
	return label.endsWith("-") ? label.slice(0, -1) : label
}

// ---------------------------------------------------------------------------
// Bounded declarative seed ingestion. The caller-owned iterables are read
// exactly once, in chunks that let the event loop turn, with row/byte budgets
// charged against the supplied policy. Values lower through the core's one
// row-cell codec (`cellOf`) — no duplicate field roster.
// ---------------------------------------------------------------------------

function isOrdinaryRelation(member: unknown): member is AnyRelation {
	return (
		typeof member === "object" &&
		member !== null &&
		"data" in member &&
		typeof member.data === "object" &&
		member.data !== null &&
		"fields" in member.data
	)
}

function planValueOfCell(relation: RelationData, ordinal: number, cell: unknown): PlanValue {
	const declared = relation.fields[ordinal]
	if (declared === undefined) {
		throw new Error(`relation ${relation.name}: cell ${ordinal} has no declared field`)
	}
	const field = declared.field
	switch (field.kind) {
		case "bool":
			return { bool: cell === true }
		case "u64":
			return { u64: String(cell) }
		case "i64":
			return { i64: String(cell) }
		case "f64":
			return { $f64: f64Bits(typeof cell === "number" ? cell : Number.NaN) }
		case "id128": {
			// The core row-cell codec lowers id128 to its canonical
			// 32-lowercase-hex string (chapter 35) — already the plan wire
			// spelling.
			if (typeof cell !== "string" || cell.length !== 32) {
				throw new Error(`relation ${relation.name}.${declared.name}: id128 cell did not lower to canonical hex`)
			}
			return { id128: cell }
		}
		case "str":
			return { string: String(cell) }
		case "bytes": {
			if (!(cell instanceof Uint8Array)) {
				throw new Error(`relation ${relation.name}.${declared.name}: bytes cell did not lower to bytes`)
			}
			return { fixedBytes: bytesHex(cell) }
		}
		case "interval": {
			if (typeof cell !== "object" || cell === null || !("start" in cell) || !("end" in cell)) {
				throw new Error(`relation ${relation.name}.${declared.name}: interval cell did not lower to endpoints`)
			}
			if (field.element === "f64") {
				return { intervalF64: [f64Bits(Number(cell.start)), f64Bits(Number(cell.end))] }
			}
			if (field.element === "i64") {
				return { intervalI64: [String(cell.start), String(cell.end)] }
			}
			return { intervalU64: [String(cell.start), String(cell.end)] }
		}
	}
}

interface SeedBudget {
	rows: bigint
	bytes: bigint
}

const lowerSeeds = Effect.fn("bumbledb-log.migrations.lowerSeeds")(function* (
	relations: Readonly<Record<string, unknown>>,
	intents: readonly MigrationIntentEntry[],
	seedRelations: readonly string[],
	work: ExecutionPolicy
) {
	const operation = "migrations.generate"
	const seedOps: PlanOperation[] = []
	const used: SeedBudget = { rows: 0n, bytes: 0n }
	for (const relationName of seedRelations) {
		const member = relations[relationName]
		if (!isOrdinaryRelation(member)) {
			return yield* Effect.fail(unsupported(operation, `seed target ${relationName} is not an ordinary relation value`))
		}
		const data = member.data
		const rows: Array<readonly PlanValue[]> = []
		for (const intent of intents) {
			if (intent.kind !== "seed" || intent.relation !== relationName) {
				continue
			}
			const iterator = intent.rows[Symbol.iterator]()
			let done = false
			while (!done) {
				// One bounded chunk per Effect step so the event loop can turn.
				const chunk = yield* Effect.try({
					try: () => {
						const lowered: Array<readonly PlanValue[]> = []
						for (let index = 0; index < SEED_CHUNK; index += 1) {
							const next = iterator.next()
							if (next.done === true) {
								done = true
								break
							}
							const fact = next.value
							const cells = data.fields.map((declared, ordinal) => {
								const raw = fact[declared.name]
								const cell = cellOf(`seed ${relationName}.${declared.name}`, declared.field, raw)
								used.bytes += cellBytes(cell)
								return planValueOfCell(data, ordinal, cell)
							})
							used.rows += 1n
							lowered.push(cells)
						}
						return lowered
					},
					catch: (cause) =>
						unsupported(
							operation,
							`seed ${relationName}: ${cause instanceof Error ? cause.message : "row refused by the core cell codec"}`
						)
				})
				if (used.rows > work.rows) {
					return yield* Effect.fail(budget(operation, "seed.rows", used.rows, used.rows, work.rows))
				}
				if (used.bytes > work.inputBytes) {
					return yield* Effect.fail(budget(operation, "seed.inputBytes", used.bytes, used.bytes, work.inputBytes))
				}
				rows.push(...chunk)
				if (!done) {
					yield* Effect.yieldNow
				}
			}
		}
		seedOps.push({ kind: "seed", target: relationName, rows })
	}
	return seedOps
})

// ---------------------------------------------------------------------------
// The generator factory. Everything below is closed over one codec value; the
// production instance is bound once in `#migrations/workflow.ts`, so the CLI
// and the direct API run literally the same Effects (TS-MIG-10).
// ---------------------------------------------------------------------------

interface Analysis {
	readonly manifestTree: JsonValue | null
	readonly entries: readonly ManifestEntry[]
	readonly planTrees: readonly JsonValue[]
	readonly snapshotTrees: readonly JsonValue[]
	readonly planTexts: readonly string[]
	readonly snapshotTexts: readonly string[]
	readonly emptySnapshot: string
	readonly baseSchemaId: string
	readonly headPrefixDigest: string
	readonly currentSchemaId: string
	readonly currentSnapshot: string
	readonly prevSchemaId: string
	readonly diff: DiffResult
	readonly changed: boolean
	readonly hasSeeds: boolean
	readonly seedIntents: readonly MigrationIntentEntry[]
	readonly staleDrafts: readonly string[]
	/** Empty-base first, then every recorded target — never an empty list. */
	readonly snapshotChain: readonly JsonValue[]
}

function parseTree(operation: string, label: string, text: string): Effect.Effect<JsonValue, LogError> {
	return Effect.try({
		try: () => JSON.parse(text) as JsonValue,
		catch: () => drift(operation, `${label} is not JSON`)
	})
}

function mappingsFromTrees(trees: readonly JsonValue[]): string {
	const plans: MigrationPlan[] = []
	for (const tree of trees) {
		const decoded = decodePlanData(tree)
		if (decoded.ok) {
			plans.push(decoded.value)
		}
	}
	return compiledMappingsJson(plans)
}

function compiledChain(
	emptySnapshot: string,
	snapshotTexts: readonly string[],
	planTexts: readonly string[],
	append: { readonly snapshot: string; readonly plan: MigrationPlan } | null
): CompiledChainInput {
	const baseSnapshot = snapshotTexts[0] ?? emptySnapshot
	const recordedTargets = snapshotTexts.length > 0 ? snapshotTexts.slice(1) : []
	const intermediateSnapshots =
		append === null ? recordedTargets : [...recordedTargets, append.snapshot]
	const appendTree = append === null ? null : planJson(append.plan)
	const mappingTrees: JsonValue[] = []
	for (const text of planTexts) {
		try {
			mappingTrees.push(JSON.parse(text) as JsonValue)
		} catch {
			// decodePlanData skips malformed trees; native still sees orderedPlans.
		}
	}
	if (appendTree !== null) {
		mappingTrees.push(appendTree)
	}
	const orderedPlans =
		appendTree === null ? [...planTexts] : [...planTexts, JSON.stringify(appendTree)]
	return {
		baseSnapshot,
		intermediateSnapshots,
		orderedPlans,
		compiledMappings: mappingsFromTrees(mappingTrees)
	}
}

export function makeGenerator(codec: MigrationCodec, exclusion: RepositoryExclusion) {
	const analyze = Effect.fn("bumbledb-log.migrations.analyze")(function* <Rels extends SchemaRelations>(
		options: CheckOptions<Rels>
	) {
	const operation = "migrations.analyze"
	if (options.intent !== undefined && options.intent.schema !== options.schema) {
		return yield* Effect.fail(
			unsupported(operation, "the migration intent was declared for a different schema value than the one being generated")
		)
	}
	const repoState = yield* readRepository(options.repository)
	// Recorded texts become parsed trees for the bridge; identity is judged
	// from canonical frames natively, never from this formatting.
	const manifestTree =
		repoState.manifestText === null ? null : yield* parseTree(operation, "manifest.json", repoState.manifestText)
	const snapshotTrees: JsonValue[] = []
	for (const [index, text] of repoState.snapshotTexts.entries()) {
		snapshotTrees.push(yield* parseTree(operation, `snapshot ${index}`, text))
	}
	const planTrees: JsonValue[] = []
	for (const [index, text] of repoState.planTexts.entries()) {
		planTrees.push(yield* parseTree(operation, `plan ${index}`, text))
	}
	const currentSpec = lower(options.schema)
	const identity = yield* codec.schemaIdentity(currentSpec, options.work)
	// Empty-base snapshot is always compiled — empty source is not a shortcut.
	const emptyIdentity = yield* codec.schemaIdentity(EMPTY_SPEC, options.work)
	const baseSchemaId = repoState.manifest === null ? emptyIdentity.schemaId : repoState.manifest.baseSchemaId
	const emptySnapshotTree = yield* parseTree(operation, "empty-base snapshot", emptyIdentity.snapshot)
	const snapshotChain = snapshotTrees.length > 0 ? snapshotTrees : [emptySnapshotTree]
	const compiled = compiledChain(emptyIdentity.snapshot, repoState.snapshotTexts, repoState.planTexts, null)
	const chain = yield* codec.verifyChain(
		{
			manifest: manifestTree,
			baseSchemaId: manifestTree === null ? baseSchemaId : null,
			snapshots: snapshotChain,
			plans: planTrees,
			append: null,
			planSet: null,
			compiled
		},
		options.work
	)
	const previousText = latestSnapshot(repoState)
	const prevSource = previousText === null ? emptyIdentity.snapshot : previousText
	const parsedPrev = parseTheory(prevSource)
	if (!parsedPrev.ok) {
		return yield* Effect.fail(
			drift(
				operation,
				previousText === null
					? `empty-base snapshot is not the canonical theory grammar: ${parsedPrev.detail}`
					: `recorded snapshot is not the canonical theory grammar: ${parsedPrev.detail}`
			)
		)
	}
	const prevTheory = parsedPrev.snapshot
	const currentParsed = parseTheory(identity.snapshot)
	if (!currentParsed.ok) {
		return yield* Effect.fail(drift(operation, `native snapshot is not the canonical theory grammar: ${currentParsed.detail}`))
	}
	const intents = options.intent === undefined ? [] : options.intent.entries
	const diff = diffSchemas(prevTheory, currentParsed.snapshot, intents)
	const entries = repoState.manifest === null ? [] : repoState.manifest.entries
	const prevSchemaId = entries.length === 0 ? baseSchemaId : (entries[entries.length - 1]?.toSchemaId ?? baseSchemaId)
	const analysis: Analysis = {
		manifestTree,
		entries,
		planTrees,
		snapshotTrees,
		planTexts: repoState.planTexts,
		snapshotTexts: repoState.snapshotTexts,
		emptySnapshot: emptyIdentity.snapshot,
		baseSchemaId,
		headPrefixDigest: chain.headPrefixDigest,
		currentSchemaId: identity.schemaId,
		currentSnapshot: identity.snapshot,
		prevSchemaId,
		diff,
		changed: prevSchemaId !== identity.schemaId || !diff.identity,
		hasSeeds: diff.seedRelations.length > 0,
		seedIntents: intents,
		staleDrafts: repoState.staleDrafts,
		snapshotChain
	}
	return analysis
})

function contractOf(analysis: Analysis, appended: ChainPayload["appended"]): RuntimeContract {
	if (appended !== null) {
		return {
			contractVersion: 1,
			schemaId: appended.entry.toSchemaId,
			appliedPrefixDigest: appended.entry.prefixDigest,
			steps: (analysis.entries.length + 1).toString(10)
		}
	}
	return {
		contractVersion: 1,
		schemaId: analysis.prevSchemaId,
		appliedPrefixDigest: analysis.headPrefixDigest,
		steps: analysis.entries.length.toString(10)
	}
}

	// -------------------------------------------------------------------------
	// generateMigrations
	// -------------------------------------------------------------------------

	function exclusive<A, E>(
		operation: string,
		directory: string,
		work: ExecutionPolicy,
		body: Effect.Effect<A, E, NativeRuntime | Scope.Scope>
	): Effect.Effect<A, E | LogError, NativeRuntime> {
		return Effect.uninterruptibleMask((restore) =>
			Effect.scoped(
				Effect.gen(function* () {
					// L16 `internalAcquireRepositoryLock` registers
					// `RepositoryLock.release` (`logRepositoryLockRelease`)
					// before its interruptible mint. After the lease exists,
					// cancel/defect/success must join host I/O first; Scope
					// then runs that release. Do not call `lock.release`
					// here — L16 does not consume the slot on explicit
					// release (request: clear `slot.owner` after
					// `joinLockRelease` so `joinPendingIo.andThen(lock.release)`
					// is the one close).
					yield* restore(exclusion.acquire(operation, directory, work))
					return yield* restore(body).pipe(Effect.ensuring(joinPendingIo))
				})
			)
		)
	}

	const generateMigrations = Effect.fn("bumbledb-log.generateMigrations")(function* <
		Rels extends SchemaRelations
	>(options: GenerateOptions<Rels>) {
	const operation = "migrations.generate"
	const directory = options.repository.directory
	return yield* exclusive(
		operation,
		directory,
		options.work,
		Effect.gen(function* () {
	yield* ensureDirectory(operation, directory)
	if (options.label !== undefined && !validLabel(options.label)) {
		return yield* Effect.fail(
			unsupported(operation, `label must be 1..${MAX_LABEL} characters of [a-z0-9-]`)
		)
	}
	const analysis = yield* analyze(options)
	if (analysis.diff.requirements.length > 0) {
		return yield* Effect.fail(intentRequired(operation, analysis.diff.requirements))
	}
	if (!analysis.changed && !analysis.hasSeeds) {
		// Nothing to record. Remove interrupted-generation leftovers and repair
		// derived files only when they drifted from the recorded chain (never
		// touch recorded history).
		const removed: string[] = []
		for (const draft of analysis.staleDrafts) {
			yield* removeFile(operation, path.join(directory, draft))
			removed.push(draft)
		}
		const contract = contractOf(analysis, null)
		const files: string[] = []
		if (analysis.entries.length > 0) {
			const wantIndex = renderIndex(analysis.entries)
			const haveIndex = yield* readBounded(operation, indexPath(directory), MAX_DERIVED_BYTES)
			if (haveIndex !== wantIndex) {
				yield* writeDerived(operation, indexPath(directory), wantIndex)
				files.push("index.ts")
			}
			const wantSnapshots = renderSnapshots(analysis.snapshotTexts)
			const haveSnapshots = yield* readBounded(operation, snapshotsSidecarPath(directory), MAX_DERIVED_BYTES)
			if (haveSnapshots !== wantSnapshots) {
				yield* writeDerived(operation, snapshotsSidecarPath(directory), wantSnapshots)
				files.push("snapshots.json")
			}
			const wantContract = renderContract(contract)
			const haveContract = yield* readBounded(operation, contractPath(options.repository), MAX_DERIVED_BYTES)
			if (haveContract !== wantContract) {
				yield* writeDerived(operation, contractPath(options.repository), wantContract)
				files.push(path.basename(contractPath(options.repository)))
			}
		}
		const report: GenerationReport = { status: "unchanged", planId: null, contract, files, removed }
		return report
	}
	const sequence = analysis.entries.length
	if (sequence > MAX_SEQUENCE) {
		return yield* Effect.fail(unsupported(operation, `the manifest already records ${MAX_SEQUENCE + 1} plans`))
	}
	const label = options.label ?? deriveLabel(analysis.diff.labelTokens)
	const id = planId(sequence, label)
	// Seeds are ingested exactly once, bounded, at generation time.
	const seedOps = yield* lowerSeeds(
		options.schema.relations,
		analysis.seedIntents,
		analysis.diff.seedRelations,
		options.work
	)
	const operations: PlanOperation[] = [
		...analysis.diff.operations,
		...seedOps,
		{ kind: "validate-schema", schemaId: analysis.currentSchemaId }
	]
	const plan: MigrationPlan = {
		planVersion: 1,
		sequence: sequence.toString(10),
		id,
		fromSchemaId: analysis.prevSchemaId,
		toSchemaId: analysis.currentSchemaId,
		operations,
		destructive: analysis.diff.destructive
	}
	// Native validation + canonical rendering + digest + manifest append.
	const compiled = compiledChain(analysis.emptySnapshot, analysis.snapshotTexts, analysis.planTexts, {
		snapshot: analysis.currentSnapshot,
		plan
	})
	const currentTree = yield* parseTree(operation, "current snapshot", analysis.currentSnapshot)
	const chain = yield* codec.verifyChain(
		{
			manifest: analysis.manifestTree,
			baseSchemaId: analysis.manifestTree === null ? analysis.baseSchemaId : null,
			snapshots: [...analysis.snapshotChain, currentTree],
			plans: analysis.planTrees,
			append: planJson(plan),
			planSet: null,
			compiled
		},
		options.work
	)
	if (chain.appended === null) {
		return yield* Effect.fail(drift(operation, "the native chain pass did not append the validated plan"))
	}
	const contract = contractOf(analysis, chain.appended)
	// Write order is the interruption-safety contract: snapshot and plan are
	// inert until the manifest (the commit point) records them; index and
	// contract are derived and rewritten deterministically. Leftovers of a
	// previously interrupted generation under a different derived label are
	// removed first — they were never recorded, and leaving them would read
	// as drift once the manifest advances past their sequence.
	yield* ensureDirectory(operation, path.join(directory, "meta"))
	const written = new Set([
		`${id}.plan.json`,
		`meta/${sequence.toString(10).padStart(4, "0")}.schema.json`,
		"meta/base.schema.json"
	])
	const removed: string[] = []
	for (const draft of analysis.staleDrafts) {
		if (written.has(draft)) {
			continue
		}
		yield* removeFile(operation, path.join(directory, draft))
		removed.push(draft)
	}
	const uniqueSnapshots =
		analysis.snapshotTexts.length > 0
			? [...analysis.snapshotTexts, analysis.currentSnapshot]
			: [analysis.emptySnapshot, analysis.currentSnapshot]
	yield* writeImmutable(operation, baseSnapshotPath(directory), analysis.emptySnapshot)
	yield* writeImmutable(operation, snapshotPath(directory, sequence), analysis.currentSnapshot)
	yield* writeImmutable(operation, planPath(directory, id), chain.appended.planText)
	yield* writeManifest(operation, manifestPath(directory), chain.appended.manifestText)
	yield* writeDerived(operation, indexPath(directory), renderIndex([...analysis.entries, chain.appended.entry]))
	yield* writeDerived(operation, snapshotsSidecarPath(directory), renderSnapshots(uniqueSnapshots))
	yield* writeDerived(operation, contractPath(options.repository), renderContract(contract))
	const report: GenerationReport = {
		status: "generated",
		planId: id,
		contract,
		files: [
			"meta/base.schema.json",
			`meta/${sequence.toString(10).padStart(4, "0")}.schema.json`,
			`${id}.plan.json`,
			"manifest.json",
			"index.ts",
			"snapshots.json",
			path.basename(contractPath(options.repository))
		],
		removed
	}
	return report
		})
	)
	})

	// -------------------------------------------------------------------------
	// checkMigrations — the same computation, writing nothing.
	// -------------------------------------------------------------------------

	const checkMigrations = Effect.fn("bumbledb-log.checkMigrations")(function* <Rels extends SchemaRelations>(
		options: CheckOptions<Rels>
	) {
	const operation = "migrations.check"
	const directory = options.repository.directory
	return yield* exclusive(
		operation,
		directory,
		options.work,
		Effect.gen(function* () {
	const analysis = yield* analyze(options)
	if (analysis.diff.requirements.length > 0) {
		return yield* Effect.fail(intentRequired(operation, analysis.diff.requirements))
	}
	const contract = contractOf(analysis, null)
	if (analysis.changed || analysis.hasSeeds) {
		const report: CheckReport = {
			status: "generation-required",
			detail:
				analysis.changed && analysis.hasSeeds
					? "the schema and declared seed data have changes with no recorded plan"
					: analysis.prevSchemaId !== analysis.currentSchemaId
						? "the schema differs from the latest recorded snapshot"
						: analysis.changed
							? "declared field conversions have no recorded plan"
							: "declared seed data has no recorded plan",
			contract
		}
		return report
	}
	if (analysis.staleDrafts.length > 0) {
		const report: CheckReport = {
			status: "generation-required",
			detail: `interrupted generation leftovers exist (${analysis.staleDrafts.join(", ")}); rerun generate`,
			contract
		}
		return report
	}
	// Recorded chain verified natively in analyze; now hold the derived files
	// and the latest snapshot to their recorded meaning, byte for byte.
	if (analysis.entries.length > 0) {
		const wantIndex = renderIndex(analysis.entries)
		const haveIndex = yield* readBounded(operation, indexPath(directory), MAX_DERIVED_BYTES)
		if (haveIndex !== wantIndex) {
			return yield* Effect.fail(drift(operation, "index.ts does not match the recorded manifest"))
		}
		const wantSnapshots = renderSnapshots(analysis.snapshotTexts)
		const haveSnapshots = yield* readBounded(operation, snapshotsSidecarPath(directory), MAX_DERIVED_BYTES)
		if (haveSnapshots !== wantSnapshots) {
			return yield* Effect.fail(drift(operation, "snapshots.json does not match the recorded snapshot chain"))
		}
		const wantContract = renderContract(contract)
		const haveContract = yield* readBounded(operation, contractPath(options.repository), MAX_DERIVED_BYTES)
		if (haveContract !== wantContract) {
			return yield* Effect.fail(drift(operation, "runtime-contract.json does not match the recorded chain head"))
		}
	}
	const report: CheckReport = { status: "clean", detail: "recorded chain verified; schema unchanged", contract }
	return report
		})
	)
	})

	return { generateMigrations, checkMigrations }
}
