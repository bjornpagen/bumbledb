/**
 * The one thin Effect layer over the internal native history machine.
 * Everything here is language adaptation: capability bookkeeping, wire
 * conversion, certainty preservation and scope ownership. No protocol
 * transition, CAS, catch-up, retry loop, checkpoint, GC step, lock, or
 * cache-eviction decision is implemented in JavaScript — those live in the
 * one Rust machine reached through the C09 executor. The factory takes the
 * wire and the core integration seam so authored tests can drive the layer
 * deterministically; production binds the real addon in `#production.ts`.
 */
import type {
	AnySchema,
	DbError,
	ExecutionPolicy,
	ExecutionSession,
	NativeRuntime,
	RuntimeHandle,
	SchemaId
} from "@bjornpagen/bumbledb"
import type { Capability, ChangeSet, CompleteResult, QueryReader } from "@bjornpagen/bumbledb/internal/log"
import { policyWire } from "@bjornpagen/bumbledb/internal/log"
import { Effect, Result } from "effect"
import type { Scope } from "effect"
import type { CancelVerb } from "#bridge.ts"
import { certaintyOperation, closeReportOf, drainClose, logOperation, scopedResource } from "#bridge.ts"
import type { LogError } from "#errors.ts"
import { closedHandle, invalidInput, logFailure } from "#errors.ts"
import type {
	CommandDigest,
	CommandRef,
	DatabaseId,
	DatabaseIdentity,
	DecisionDigest,
	DecisionStamp,
	Freshness,
	IncarnationId,
	OperationId,
	OperationRef,
	PlanSetDigest,
	ReceiptEpoch,
	RequestId,
	RootId,
	StateStamp
} from "#identity.ts"
import type {
	ActivationRefWire,
	AdminRequestWire,
	AdminResultWire,
	AdminValueWire,
	BindingWire,
	CacheHandle,
	CacheInspectionWire,
	CacheMakeWire,
	CommandRefWire,
	CommandWire,
	ConsistencyWire,
	CoreSnapshotHandle,
	DestinationWire,
	ErrorWire,
	FreshnessWire,
	HealthWire,
	HistoryCapability,
	HistoryHandleWire,
	HistoryInspectionWire,
	HistoryRequestWire,
	HistoryResultWire,
	LogNative,
	LogSnapshotHandle,
	MigrateValueWire,
	MigrationRefWire,
	MigrationStatusWire,
	OutcomeWire,
	PlansWire,
	PreconditionWire,
	ProvenanceWire,
	PublicationPhaseWire,
	ReceiptWire,
	ResultWire,
	SealRequestWire,
	SourceAccessWire,
	StampWire,
	StateWire,
	SubmitWire
} from "#native.ts"
import type {
	CreationOptions,
	HistoryBinding,
	HostedBinding,
	HostedCreateOptions,
	HostedCredentials,
	HostedOpenOptions,
	LocalBinding,
	LocalCreateOptions,
	LocalOpenOptions,
	ReadOptions,
	SubmitOptions,
	TenantCacheOptions
} from "#options.ts"
import type {
	AbortReport,
	ActivationRef,
	ActivationReport,
	AdminOutcome,
	BackupReport,
	BackupVerification,
	CacheInspection,
	CheckpointReport,
	CommandResult,
	CommandScalar,
	ErasureReport,
	GcReport,
	GeneratedMigrations,
	HistoryInspection,
	InitializeValue,
	LocalMaterializationHealth,
	MigrateValue,
	MigrationRef,
	MigrationStatus,
	PublicationPhase,
	ReceiptRetirementReport,
	ReceiptRotationReport,
	ResolveOutcome,
	RestorePointReport,
	RestoreReport,
	RootReleaseReport,
	SourceAccessReport,
	SubmitOutcome,
	TerminalOutcome,
	TerminalReceipt
} from "#outcome.ts"
import type { Command, CommandInput, History, HistoryBorrow, PublishedSnapshot, TenantCache } from "#surface.ts"

// ── The core integration seam (C10; production binds P07's internals) ──────

export interface PublishedReadCapability<S extends AnySchema> {
	/** Self-contained (closure-captured) methods; never `this`-dependent. */
	readonly get: QueryReader<S>["get"]
	/** Core QueryReader.execute — yields the CompleteResult owner. */
	readonly execute: QueryReader<S>["execute"]
	readonly session: (work: ExecutionPolicy) => Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
}

/** Published execute result: the core CompleteResult, not a copied page. */
export type PublishedCompleteResult<A> = CompleteResult<A>

/** The exact shape the core's landed `internalChanges` accessor returns. */
export interface CoreChangesView {
	readonly handle: unknown
	readonly schemaId: SchemaId
	readonly closed: boolean
}

export interface CoreIntegration {
	/**
	 * Wrap a published core snapshot Capability in the exact core QueryReader
	 * (`internalPublishedReader` in production).
	 */
	reader<S extends AnySchema>(core: CoreSnapshotHandle | Capability, schema: S): PublishedReadCapability<S>
	/**
	 * The core's private ChangeSet registry accessor (`internalChanges`):
	 * `undefined` for a foreign dynamic object, `closed` mirrors the native
	 * capability state, and `handle` is the retained native change — the
	 * exact accepted bytes, never a reconstruction.
	 */
	changes(value: ChangeSet<AnySchema> | object): CoreChangesView | undefined
	/** The core schema lowering (`lower`) the native boundary admits. */
	schemaSpec(schema: AnySchema): unknown
	/** The acquired shared runtime capability (core `runtimeHandle`). */
	runtime(): Effect.Effect<RuntimeHandle, DbError, NativeRuntime>
}

export type LogWire = LogNative

// ── Trusted-native decode: branding data from the typed addon boundary ─────
// The binding declaration is the trust boundary (as in the core's
// runtime-native.ts); user input never flows through these.

function identityOf(wire: HistoryHandleWire["meta"]["identity"]): DatabaseIdentity {
	return {
		databaseId: wire.databaseId as DatabaseId,
		incarnationId: wire.incarnationId as IncarnationId,
		schemaId: wire.schemaId as DatabaseIdentity["schemaId"]
	}
}

function stampOf(wire: StampWire): DecisionStamp {
	return { seq: wire.seq, hash: wire.hash as DecisionDigest }
}

function stateOf(wire: StateWire): StateStamp {
	return { incarnation: wire.incarnation as IncarnationId, dataRevision: wire.dataRevision }
}

function freshnessOf(wire: FreshnessWire): Freshness {
	if (wire.kind === "at-least") {
		return { kind: "at-least", requested: stampOf(wire.requested) }
	}
	return { kind: wire.kind }
}

function refOf(wire: CommandRefWire): CommandRef {
	return {
		identity: identityOf(wire.identity),
		id: {
			receiptEpoch: wire.receiptEpoch as ReceiptEpoch,
			requestId: wire.requestId as RequestId
		},
		digest: wire.digest as CommandDigest
	}
}

function resultOf(wire: ResultWire): CommandResult {
	const out: Record<string, CommandScalar> = {}
	for (const [key, value] of Object.entries(wire)) {
		out[key] = value
	}
	return out
}

function outcomeOf(wire: OutcomeWire): TerminalOutcome {
	switch (wire.kind) {
		case "committed":
			return {
				kind: "committed",
				changed: { added: wire.added, removed: wire.removed },
				result: resultOf(wire.result)
			}
		case "no-change":
			return { kind: "no-change", result: resultOf(wire.result) }
		case "precondition-failed":
			return {
				kind: "precondition-failed",
				expected: stateOf(wire.expected),
				observed: stateOf(wire.observed)
			}
		case "invariant-rejected":
			return { kind: "invariant-rejected", violations: wire.violations }
	}
}

function receiptOf(wire: ReceiptWire): TerminalReceipt {
	return {
		command: refOf(wire.command),
		decisionAt: stampOf(wire.decisionAt),
		stateAt: stateOf(wire.stateAt),
		outcome: outcomeOf(wire.outcome)
	}
}

function errorOf(operation: string, wire: ErrorWire): LogError {
	return logFailure(operation, wire)
}

function healthOf(operation: string, wire: HealthWire): LocalMaterializationHealth {
	if (wire.kind === "ready") {
		return { kind: "ready", at: stampOf(wire.at) }
	}
	return { kind: "unavailable", error: errorOf(operation, wire.error) }
}

function phaseOf(operation: string, wire: PublicationPhaseWire): PublicationPhase {
	switch (wire) {
		case "prepared":
		case "dispatchedUnresolved":
		case "confirmed":
		case "provedNonpublication":
			return wire
		default:
			throw invalidInput(operation)
	}
}

function inspectionOf(wire: HistoryInspectionWire): HistoryInspection {
	return {
		identity: identityOf(wire.identity),
		accessMode: wire.accessMode,
		headRevision: wire.headRevision,
		decision: stampOf(wire.decision),
		state: stateOf(wire.state),
		receipts: {
			openEpoch: wire.openEpoch as ReceiptEpoch,
			retiredThrough: wire.retiredThrough
		},
		tail: { count: wire.tailCount, bytes: wire.tailBytes },
		unknownCommands: { count: wire.unknownCount, oldestMillis: wire.unknownOldestMillis },
		roots: { count: wire.rootCount, capacity: wire.rootCapacity },
		gc: wire.gc,
		lastMaintenanceError: wire.lastMaintenanceError,
		accounted: { diskBytes: wire.diskBytes, workingBytes: wire.workingBytes },
		operations: { queued: wire.queued, active: wire.active }
	}
}

function cacheInspectionOf(wire: CacheInspectionWire): CacheInspection {
	return {
		openCount: wire.openCount,
		opening: wire.opening,
		budget: { bytes: wire.budgetBytes, maxOpen: wire.maxOpen },
		evictions: wire.evictions,
		slots: wire.slots.map((slot) => ({
			binding: slot.binding,
			state: slot.state,
			borrows: slot.borrows,
			diskBytes: slot.diskBytes
		}))
	}
}

// ── Checked outbound wire conversion (caller input; refuses, never casts) ──

function checkedString(operation: string, value: string, maxLength: number): string {
	if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
		throw invalidInput(operation)
	}
	return value
}

function identityWire(operation: string, identity: DatabaseIdentity) {
	return {
		databaseId: checkedString(operation, identity.databaseId, 32),
		incarnationId: checkedString(operation, identity.incarnationId, 32),
		schemaId: checkedString(operation, String(identity.schemaId), 64)
	}
}

function credentialsWire(operation: string, credentials: HostedCredentials | undefined) {
	if (credentials === undefined || credentials.kind === "provider-chain") {
		return { kind: "provider-chain" } as const
	}
	return {
		kind: "static",
		accessKeyId: checkedString(operation, credentials.accessKeyId, 256),
		secretAccessKey: checkedString(operation, credentials.secretAccessKey, 256),
		sessionToken: credentials.sessionToken === undefined ? null : checkedString(operation, credentials.sessionToken, 4096)
	} as const
}

function bindingWire(operation: string, binding: HistoryBinding): BindingWire {
	if (binding.kind === "local") {
		return {
			kind: "local",
			directory: checkedString(operation, binding.directory, 4096),
			identity: identityWire(operation, binding.identity)
		}
	}
	if (binding.kind !== "hosted") {
		throw invalidInput(operation)
	}
	return {
		kind: "hosted",
		directory: checkedString(operation, binding.directory, 4096),
		bucket: checkedString(operation, binding.origin.bucket, 255),
		prefix: checkedString(operation, binding.origin.prefix, 1024),
		region: binding.origin.region === undefined ? null : checkedString(operation, binding.origin.region, 64),
		identity: identityWire(operation, binding.identity),
		credentials: credentialsWire(operation, binding.credentials)
	}
}

function bindingOf(wire: BindingWire): HistoryBinding {
	if (wire.kind === "local") {
		return { kind: "local", directory: wire.directory, identity: identityOf(wire.identity) }
	}
	return {
		kind: "hosted",
		directory: wire.directory,
		origin: {
			bucket: wire.bucket,
			prefix: wire.prefix,
			...(wire.region === null ? {} : { region: wire.region })
		},
		identity: identityOf(wire.identity),
		credentials: { kind: "provider-chain" }
	}
}

function consistencyWire(operation: string, options: ReadOptions): ConsistencyWire {
	const consistency = options.consistency
	if (consistency.kind === "cached" || consistency.kind === "latest") {
		return { kind: consistency.kind }
	}
	if (consistency.kind !== "at-least") {
		throw invalidInput(operation)
	}
	// Checked outbound: the requested stamp is caller input. The hash is the
	// exact 64-hex decision digest the native ancestry witness validates —
	// AtLeast is never a bare sequence floor, so a malformed stamp refuses
	// HERE, before dispatch.
	if (typeof consistency.at.seq !== "bigint" || consistency.at.seq < 0n) {
		throw invalidInput(operation)
	}
	return { kind: "at-least", seq: consistency.at.seq, hash: checkedString(operation, consistency.at.hash, 64) }
}

function refWire(operation: string, ref: CommandRef): CommandRefWire {
	return {
		identity: identityWire(operation, ref.identity),
		receiptEpoch: ref.id.receiptEpoch,
		requestId: checkedString(operation, ref.id.requestId, 32),
		digest: checkedString(operation, ref.digest, 64)
	}
}

function scalarOk(value: unknown): value is CommandScalar {
	const kind = typeof value
	if (kind === "bigint" || kind === "number" || kind === "string" || kind === "boolean") {
		return true
	}
	return value instanceof Uint8Array && value.buffer instanceof ArrayBuffer
}

function resultWire(operation: string, result: CommandResult): ResultWire {
	const out: Record<string, CommandScalar> = {}
	const entries = Object.entries(result)
	if (entries.length > 64) {
		throw invalidInput(operation)
	}
	for (const [key, value] of entries) {
		checkedString(operation, key, 128)
		if (!scalarOk(value)) {
			throw invalidInput(operation)
		}
		// The canonical result cell splits bigints at the sign: nonnegative is
		// U64 (tag 1), negative is I64 (tag 2); anything outside those widths
		// refuses HERE, before dispatch, never as a mid-marshal native throw.
		if (typeof value === "bigint" && (value >= 1n << 64n || value < -(1n << 63n))) {
			throw invalidInput(operation)
		}
		out[key] = value
	}
	return out
}

function preconditionWire(
	operation: string,
	precondition: import("#surface.ts").Precondition
): PreconditionWire {
	if (precondition.kind === "blind") {
		return { kind: "blind" }
	}
	if (precondition.kind !== "exact-state") {
		throw invalidInput(operation)
	}
	return {
		kind: "exact-state",
		incarnation: checkedString(operation, precondition.at.incarnation, 32),
		dataRevision: precondition.at.dataRevision
	}
}

function checkedCount(operation: string, value: number, max: number): number {
	if (!Number.isSafeInteger(value) || value <= 0 || value > max) {
		throw invalidInput(operation)
	}
	return value
}

function checkedNonNegative(operation: string, value: number, max: number): number {
	if (!Number.isSafeInteger(value) || value < 0 || value > max) {
		throw invalidInput(operation)
	}
	return value
}

function creationWire(operation: string, creation: CreationOptions) {
	if (!(creation.artifact instanceof Uint8Array) || !(creation.artifact.buffer instanceof ArrayBuffer)) {
		throw invalidInput(operation)
	}
	return {
		operationId: checkedString(operation, creation.operationId, 32),
		artifact: creation.artifact
	}
}

function snapshotsField(operation: string, plans: MigrationPlansInput): { readonly snapshots: readonly string[] } {
	// Base schema snapshot first, then each entry's target: entries + 1 rows.
	// Empty source still supplies the empty-schema render — never optional.
	if (!Array.isArray(plans.snapshots) || plans.snapshots.length !== plans.manifest.entries.length + 1) {
		throw invalidInput(operation)
	}
	return { snapshots: plans.snapshots.map((snapshot) => checkedString(operation, snapshot, 4 << 20)) }
}

function plansWire(operation: string, plans: MigrationPlansInput): PlansWire {
	if (plans.manifest.entries.length !== plans.plans.length || plans.manifest.entries.length > 4096) {
		throw invalidInput(operation)
	}
	return {
		...snapshotsField(operation, plans),
		manifestVersion: checkedNonNegative(operation, plans.manifest.manifestVersion, 0xffff),
		planVersion: checkedNonNegative(operation, plans.manifest.planVersion, 0xffff),
		baseSchemaId: checkedString(operation, plans.manifest.baseSchemaId, 64),
		basePrefixDigest: checkedString(operation, plans.manifest.basePrefixDigest, 64),
		entries: plans.manifest.entries.map((entry) => ({
			sequence: checkedString(operation, entry.sequence, 20),
			id: checkedString(operation, entry.id, 64),
			fromSchemaId: checkedString(operation, entry.fromSchemaId, 64),
			toSchemaId: checkedString(operation, entry.toSchemaId, 64),
			planDigest: checkedString(operation, entry.planDigest, 64),
			prefixDigest: checkedString(operation, entry.prefixDigest, 64)
		})),
		// Inert canonical transport of the generated plan data; the native
		// migration codec is the one canonicalization/digest authority.
		plans: plans.plans.map((plan) => JSON.stringify(plan))
	}
}

// ── The machine ────────────────────────────────────────────────────────────

interface CommandEntry {
	readonly handle: CommandWire["command"]
	readonly state: { closed: boolean }
}

export interface LogMachine {
	readonly LocalHistory: {
		open<S extends AnySchema>(binding: LocalBinding, schema: S, options: LocalOpenOptions): OpenEffect<S>
		create<S extends AnySchema>(binding: LocalBinding, schema: S, options: LocalCreateOptions): OpenEffect<S>
	}
	readonly HostedHistory: {
		open<S extends AnySchema>(binding: HostedBinding, schema: S, options: HostedOpenOptions): OpenEffect<S>
		create<S extends AnySchema>(binding: HostedBinding, schema: S, options: HostedCreateOptions): OpenEffect<S>
	}
	readonly Command: {
		seal<S extends AnySchema>(input: CommandInput<S>, work: ExecutionPolicy): Effect.Effect<Command<S>, LogError, Scope.Scope>
		encode<S extends AnySchema>(command: Command<S>, work: ExecutionPolicy): Effect.Effect<Uint8Array, LogError>
		decode<S extends AnySchema>(
			bytes: Uint8Array,
			schema: S,
			work: ExecutionPolicy
		): Effect.Effect<Command<S>, LogError, NativeRuntimeService | Scope.Scope>
	}
	readonly TenantCache: {
		make<S extends AnySchema>(
			schema: S,
			options: TenantCacheOptions
		): Effect.Effect<TenantCache<S>, LogError, NativeRuntimeService | Scope.Scope>
	}
	readonly admin: AdminOperations
	readonly migrations: MigrationOperations
}

type NativeRuntimeService = NativeRuntime

type OpenEffect<S extends AnySchema> = Effect.Effect<History<S>, LogError, NativeRuntimeService | Scope.Scope>

/**
 * Optional tenant-open input for admin/migration verbs: the core schema
 * whose lowered `SchemaSpec` the native side needs whenever it must open
 * the local materialization and the tenant is not already open in the
 * runtime's registry. Absent stays a typed native refusal (not-started);
 * this layer never invents a descriptor.
 */
export interface TenantOpenOptions {
	readonly schema?: AnySchema
}

/**
 * Optional target-location inputs for `activateMigration`/`abortMigration`:
 * the SOURCE binding (locates the stable `<dir>/targets` namespace) and the
 * TARGET's core schema. Absent stays a typed native refusal.
 */
export interface MigrationTargetOptions extends TenantOpenOptions {
	readonly binding?: HistoryBinding
}

/**
 * The generated migrations including the mandatory snapshot chain
 * (empty-base first, then each entry's target). Digests alone cannot
 * reconstruct descriptors; empty source is not a compile shortcut.
 */
export type MigrationPlansInput = GeneratedMigrations

export interface AdminIdentityOptions extends ExecutionPolicy, TenantOpenOptions {
	readonly operationId: OperationId
}

export interface AdminOperations {
	checkpoint(
		binding: HistoryBinding,
		options: AdminIdentityOptions
	): Effect.Effect<AdminOutcome<CheckpointReport>, never, NativeRuntimeService>
	pinRestorePoint(
		binding: HistoryBinding,
		options: AdminIdentityOptions & { readonly label: string }
	): Effect.Effect<AdminOutcome<RestorePointReport>, never, NativeRuntimeService>
	releaseRestorePoint(
		binding: HistoryBinding,
		options: AdminIdentityOptions & { readonly root: RootId }
	): Effect.Effect<AdminOutcome<RootReleaseReport>, never, NativeRuntimeService>
	rotateReceiptEpoch(
		binding: HistoryBinding,
		options: AdminIdentityOptions
	): Effect.Effect<AdminOutcome<ReceiptRotationReport>, never, NativeRuntimeService>
	retireReceipts(
		binding: HistoryBinding,
		options: AdminIdentityOptions & { readonly through: bigint }
	): Effect.Effect<AdminOutcome<ReceiptRetirementReport>, never, NativeRuntimeService>
	collectGarbage(
		binding: HistoryBinding,
		options: AdminIdentityOptions
	): Effect.Effect<AdminOutcome<GcReport>, never, NativeRuntimeService>
	backup(
		binding: HistoryBinding,
		options: AdminIdentityOptions & { readonly destination: BackupDestination }
	): Effect.Effect<AdminOutcome<BackupReport>, never, NativeRuntimeService>
	verifyBackup(
		destination: BackupDestination,
		options: ExecutionPolicy & { readonly backup?: OperationId }
	): Effect.Effect<BackupVerification, LogError, NativeRuntimeService>
	restore(
		source: BackupDestination,
		target: HistoryBinding,
		options: AdminIdentityOptions & { readonly backup?: OperationId }
	): Effect.Effect<AdminOutcome<RestoreReport>, never, NativeRuntimeService>
	erase(
		binding: HistoryBinding,
		options: AdminIdentityOptions & { readonly retainRoots: readonly RootId[] }
	): Effect.Effect<AdminOutcome<ErasureReport>, never, NativeRuntimeService>
}

export type BackupDestination =
	| { readonly kind: "filesystem"; readonly directory: string }
	| {
			readonly kind: "s3"
			readonly bucket: string
			readonly prefix: string
			readonly region?: string
			readonly credentials?: HostedCredentials
	  }

export interface MigrationOperations {
	migrationStatus(
		binding: HistoryBinding,
		plans: MigrationPlansInput,
		work: ExecutionPolicy & TenantOpenOptions
	): Effect.Effect<MigrationStatus, LogError, NativeRuntimeService>
	initialize(
		binding: HistoryBinding,
		plans: MigrationPlansInput,
		options: AdminIdentityOptions
	): Effect.Effect<AdminOutcome<InitializeValue>, never, NativeRuntimeService>
	migrate(
		binding: HistoryBinding,
		plans: MigrationPlansInput,
		options: AdminIdentityOptions & { readonly to?: string }
	): Effect.Effect<AdminOutcome<MigrateValue>, never, NativeRuntimeService>
	activateMigration(
		ref: ActivationRef,
		options: ExecutionPolicy & MigrationTargetOptions
	): Effect.Effect<AdminOutcome<ActivationReport>, never, NativeRuntimeService>
	abortMigration(
		ref: MigrationRef,
		options: ExecutionPolicy & MigrationTargetOptions
	): Effect.Effect<AdminOutcome<AbortReport>, never, NativeRuntimeService>
}

export function makeLogMachine(wire: LogWire, core: CoreIntegration): LogMachine {
	const cancel: CancelVerb = (operation, callback) => wire.runtimeCancel(operation, callback)
	const commandEntries = new WeakMap<object, CommandEntry>()

	function makeCommand<S extends AnySchema>(cw: CommandWire): Command<S> {
		const state = { closed: false }
		const command: Command<S> = {
			ref: refOf(cw.ref),
			close: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("Command.close", (callback) => wire.logCommandClose(cw.command, callback))
				})
		}
		commandEntries.set(command, { handle: cw.command, state })
		return command
	}

	function makeSnapshot<S extends AnySchema>(
		schema: S,
		snapshot: LogSnapshotHandle,
		coreHandle: CoreSnapshotHandle,
		provenance: ProvenanceWire
	): PublishedSnapshot<S> {
		const capability = core.reader(coreHandle, schema)
		return {
			identity: identityOf(provenance.identity),
			decisionStamp: stampOf(provenance.decision),
			stateStamp: stateOf(provenance.state),
			freshness: freshnessOf(provenance.freshness),
			get: capability.get,
			execute: capability.execute,
			session: capability.session,
			close: () => drainClose("PublishedSnapshot.close", (callback) => wire.logSnapshotClose(snapshot, callback))
		}
	}

	interface FacadeMembers<S extends AnySchema> {
		readonly identity: DatabaseIdentity
		readonly receiptEpoch: ReceiptEpoch
		snapshot(options: ReadOptions): Effect.Effect<PublishedSnapshot<S>, LogError, Scope.Scope>
		submit(command: Command<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome>
		resolve(ref: CommandRef, work: ExecutionPolicy): Effect.Effect<ResolveOutcome, LogError>
		inspect(work: ExecutionPolicy): Effect.Effect<HistoryInspection, LogError>
	}

	function decodeSubmit(operation: string, ref: CommandRef, result: HistoryResultWire): SubmitOutcome {
		if (result.verb !== "submit") {
			throw invalidInput(operation)
		}
		const outcome: SubmitWire = result.outcome
		switch (outcome.kind) {
			case "decided":
				return {
					kind: "decided",
					receipt: receiptOf(outcome.receipt),
					localHealth: healthOf(operation, outcome.localHealth),
					phase: phaseOf(operation, outcome.publicationPhase)
				}
			case "not-submitted":
				return {
					kind: "not-submitted",
					command: ref,
					error: errorOf(operation, outcome.error),
					phase: phaseOf(operation, outcome.publicationPhase)
				}
			case "outcome-unknown":
				return {
					kind: "outcome-unknown",
					command: ref,
					error: errorOf(operation, outcome.error),
					phase: phaseOf(operation, outcome.publicationPhase)
				}
		}
	}

	function decodeResolve(operation: string, result: HistoryResultWire): ResolveOutcome {
		if (result.verb !== "resolve") {
			throw invalidInput(operation)
		}
		switch (result.outcome.kind) {
			case "found":
				return { kind: "found", receipt: receiptOf(result.outcome.receipt) }
			case "not-recorded-at":
				return { kind: "not-recorded-at", decisionAt: stampOf(result.outcome.decisionAt) }
			case "command-epoch-closed":
				return { kind: "command-epoch-closed" }
			case "receipt-expired-unknown":
				return { kind: "receipt-expired-unknown" }
		}
	}

	function makeFacadeMembers<S extends AnySchema>(
		schema: S,
		capability: HistoryCapability,
		meta: HistoryHandleWire["meta"],
		state: { closed: boolean }
	): FacadeMembers<S> {
		const identity = identityOf(meta.identity)
		const receiptEpoch = meta.receiptEpoch as ReceiptEpoch

		function call<A>(
			operation: string,
			work: ExecutionPolicy,
			request: HistoryRequestWire,
			accept: (result: HistoryResultWire) => A
		): Effect.Effect<A, LogError> {
			return Effect.suspend(() => {
				if (state.closed) {
					return Effect.fail(closedHandle(operation))
				}
				return logOperation(
					operation,
					cancel,
					(callback) => wire.logHistoryCall(capability, policyWire(work, operation), request, callback),
					wire.logHistoryResult,
					accept
				)
			})
		}

		return {
			identity,
			receiptEpoch,
			snapshot(options: ReadOptions) {
				const operation = "History.snapshot"
				const acquire = Effect.suspend(() => {
					let request: HistoryRequestWire
					try {
						request = { verb: "snapshot", consistency: consistencyWire(operation, options) }
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return call(operation, options, request, (result) => {
						if (result.verb !== "snapshot") {
							throw invalidInput(operation)
						}
						return makeSnapshot(schema, result.snapshot, result.core, result.provenance)
					})
				})
				return scopedResource(operation, acquire, (snapshot) => snapshot.close())
			},
			submit(command: Command<S>, options: SubmitOptions) {
				const operation = "History.submit"
				return Effect.suspend(() => {
					const entry = commandEntries.get(command)
					if (entry === undefined) {
						// A forged capability with no authentic ref is misuse: a defect,
						// never a fabricated certainty arm or a forged receipt.
						return Effect.die(invalidInput(operation))
					}
					const ref = command.ref
					// Identity is this sealed ref. Interrupt after dispatch is
					// outcome-unknown under `ref`; retry resolve/resubmit here,
					// never Command.seal of a newly minted id.
					if (state.closed || entry.state.closed) {
						return Effect.succeed<SubmitOutcome>({
							kind: "not-submitted",
							command: ref,
							error: closedHandle(operation),
							phase: "prepared"
						})
					}
					return certaintyOperation(
						operation,
						cancel,
						(callback) =>
							wire.logHistoryCall(
								capability,
								policyWire(options, operation),
								{
									verb: "submit",
									command: entry.handle,
									attempts: checkedCount(operation, options.attempts, 0xffff),
									backoffBaseMillis: checkedNonNegative(operation, options.backoff.baseMillis, 0xffffffff),
									backoffCapMillis: checkedNonNegative(operation, options.backoff.capMillis, 0xffffffff)
								},
								callback
							),
						wire.logHistoryResult,
						(result) => decodeSubmit(operation, ref, result),
						(error): SubmitOutcome => ({
							kind: "not-submitted",
							command: ref,
							error,
							phase: "prepared"
						}),
						(error): SubmitOutcome => ({
							kind: "outcome-unknown",
							command: ref,
							error,
							phase: "dispatchedUnresolved"
						})
					)
				})
			},
			resolve(ref: CommandRef, work: ExecutionPolicy) {
				const operation = "History.resolve"
				return Effect.suspend(() => {
					let wireRef: CommandRefWire
					try {
						wireRef = refWire(operation, ref)
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return call(operation, work, { verb: "resolve", ref: wireRef }, (result) => decodeResolve(operation, result))
				})
			},
			inspect(work: ExecutionPolicy) {
				const operation = "History.inspect"
				return call(operation, work, { verb: "inspect" }, (result) => {
					if (result.verb !== "inspect") {
						throw invalidInput(operation)
					}
					return inspectionOf(result.inspection)
				})
			}
		}
	}

	function makeHistory<S extends AnySchema>(schema: S, handle: HistoryHandleWire): History<S> {
		const state = { closed: false }
		const members = makeFacadeMembers(schema, handle.history, handle.meta, state)
		return {
			...members,
			close: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("History.close", (callback) => wire.logHistoryClose(handle.history, callback))
				})
		}
	}

	function makeBorrow<S extends AnySchema>(schema: S, handle: HistoryHandleWire): HistoryBorrow<S> {
		const state = { closed: false }
		const members = makeFacadeMembers(schema, handle.history, handle.meta, state)
		return {
			...members,
			release: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("HistoryBorrow.release", (callback) => wire.logBorrowRelease(handle.history, callback))
				})
		}
	}

	function openHistory<S extends AnySchema>(
		operation: string,
		kind: "local" | "hosted",
		mode: "open" | "create",
		binding: HistoryBinding,
		schema: S,
		options: (LocalOpenOptions | HostedOpenOptions) & { readonly creation?: CreationOptions }
	): OpenEffect<S> {
		return Effect.gen(function* () {
			const runtime = yield* core.runtime()
			const acquire = Effect.suspend(() => {
				if (binding.kind !== kind) {
					return Effect.fail(invalidInput(operation))
				}
				if (mode === "create" && options.creation === undefined) {
					return Effect.fail(invalidInput(operation))
				}
				return logOperation(
					operation,
					cancel,
					(callback) =>
						wire.logHistoryOpen(
							runtime,
							policyWire(options, operation),
							{
								mode,
								binding: bindingWire(operation, binding),
								schema: core.schemaSpec(schema),
								discardMismatchedCache:
									"discardMismatchedCache" in options ? options.discardMismatchedCache === true : false,
								creation: options.creation === undefined ? null : creationWire(operation, options.creation)
							},
							callback
						),
					wire.logHistoryTake,
					(handle) => makeHistory(schema, handle)
				)
			})
			return yield* scopedResource(operation, acquire, (history) => history.close())
		})
	}

	const LocalHistory: LogMachine["LocalHistory"] = {
		open: (binding, schema, options) => openHistory("LocalHistory.open", "local", "open", binding, schema, options),
		create: (binding, schema, options) =>
			openHistory("LocalHistory.create", "local", "create", binding, schema, options)
	}

	const HostedHistory: LogMachine["HostedHistory"] = {
		open: (binding, schema, options) => openHistory("HostedHistory.open", "hosted", "open", binding, schema, options),
		create: (binding, schema, options) =>
			openHistory("HostedHistory.create", "hosted", "create", binding, schema, options)
	}

	const CommandNamespace: LogMachine["Command"] = {
		seal<S extends AnySchema>(input: CommandInput<S>, work: ExecutionPolicy) {
			const operation = "Command.seal"
			const acquire = Effect.suspend(() => {
				// The exact core registry accessor: a foreign dynamic object
				// refuses before any native dispatch (the Db.apply pattern);
				// a spent/closed ChangeSet refuses as ClosedHandle; a scope/
				// change schema mismatch refuses before native work.
				const internal = core.changes(input.changes)
				if (internal === undefined) {
					return Effect.fail<LogError>(invalidInput(operation))
				}
				if (internal.closed) {
					return Effect.fail<LogError>(closedHandle(operation))
				}
				let request: SealRequestWire
				try {
					if (String(internal.schemaId) !== String(input.scope.schemaId)) {
						throw invalidInput(operation)
					}
					request = {
						scope: identityWire(operation, input.scope),
						receiptEpoch: input.id.receiptEpoch,
						requestId: checkedString(operation, input.id.requestId, 32),
						precondition: preconditionWire(operation, input.precondition),
						result: resultWire(operation, input.result)
					}
				} catch (cause) {
					return Effect.fail(logFailure(operation, cause))
				}
				return logOperation(
					operation,
					cancel,
					(callback) => wire.logCommandSeal(internal.handle, policyWire(work, operation), request, callback),
					wire.logCommandTake,
					(cw) => makeCommand<S>(cw)
				)
			})
			return scopedResource(operation, acquire, (command) => command.close())
		},
		encode<S extends AnySchema>(command: Command<S>, work: ExecutionPolicy) {
			const operation = "Command.encode"
			return Effect.suspend(() => {
				const entry = commandEntries.get(command)
				if (entry === undefined) {
					return Effect.die(invalidInput(operation))
				}
				if (entry.state.closed) {
					return Effect.fail(closedHandle(operation))
				}
				return logOperation(
					operation,
					cancel,
					(callback) => wire.logCommandEncode(entry.handle, policyWire(work, operation), callback),
					wire.logBytesTake,
					(bytes) => bytes
				)
			})
		},
		decode<S extends AnySchema>(bytes: Uint8Array, schema: S, work: ExecutionPolicy) {
			const operation = "Command.decode"
			return Effect.gen(function* () {
				const runtime = yield* core.runtime()
				const acquire = Effect.suspend(() => {
					if (!(bytes instanceof Uint8Array) || !(bytes.buffer instanceof ArrayBuffer)) {
						return Effect.fail(logFailure(operation, invalidInput(operation)))
					}
					return logOperation(
						operation,
						cancel,
						(callback) =>
							wire.logCommandDecode(runtime, policyWire(work, operation), bytes, core.schemaSpec(schema), callback),
						wire.logCommandTake,
						(cw) => makeCommand<S>(cw)
					)
				})
				return yield* scopedResource(operation, acquire, (command) => command.close())
			})
		}
	}

	const TenantCacheNamespace: LogMachine["TenantCache"] = {
		make<S extends AnySchema>(schema: S, options: TenantCacheOptions) {
			const operation = "TenantCache.make"
			return Effect.gen(function* () {
				const runtime = yield* core.runtime()
				const acquire = Effect.suspend(() => {
					let request: CacheMakeWire
					try {
						if (typeof options.budgetBytes !== "bigint" || options.budgetBytes < 0n) {
							throw invalidInput(operation)
						}
						request = {
							maxOpen: checkedCount(operation, options.maxOpen, 0xffffffff),
							budgetBytes: options.budgetBytes,
							expected:
								options.expected === undefined
									? null
									: {
											schemaId: checkedString(operation, options.expected.schemaId, 64),
											appliedPrefixDigest: checkedString(operation, options.expected.appliedPrefixDigest, 64)
										},
							schema: core.schemaSpec(schema)
						}
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logCacheMake(runtime, policyWire(options.maintenance, operation), request, callback),
						wire.logCacheTake,
						(cache) => makeCache(schema, cache, options)
					)
				})
				return yield* scopedResource(operation, acquire, (cache) => cache.close())
			})
		}
	}

	function makeCache<S extends AnySchema>(schema: S, cache: CacheHandle, options: TenantCacheOptions): TenantCache<S> {
		const state = { closed: false }
		return {
			acquire(binding: HistoryBinding, work: ExecutionPolicy) {
				const operation = "TenantCache.acquire"
				const acquire = Effect.suspend(() => {
					if (state.closed) {
						return Effect.fail(closedHandle(operation))
					}
					let wireBinding: BindingWire
					try {
						wireBinding = bindingWire(operation, binding)
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logCacheAcquire(cache, policyWire(work, operation), { binding: wireBinding }, callback),
						wire.logBorrowTake,
						(handle) => makeBorrow(schema, handle)
					)
				})
				return scopedResource(operation, acquire, (borrow) => borrow.release())
			},
			inspect(work: ExecutionPolicy) {
				const operation = "TenantCache.inspect"
				return Effect.suspend(() => {
					if (state.closed) {
						return Effect.fail(closedHandle(operation))
					}
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logCacheInspect(cache, policyWire(work, operation), callback),
						wire.logCacheInspectTake,
						cacheInspectionOf
					)
				})
			},
			evict(binding: HistoryBinding) {
				const operation = "TenantCache.evict"
				return Effect.suspend(() => {
					if (state.closed) {
						return Effect.fail(closedHandle(operation))
					}
					let wireBinding: BindingWire
					try {
						wireBinding = bindingWire(operation, binding)
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return logOperation(
						operation,
						cancel,
						(callback) =>
							wire.logCacheEvict(cache, policyWire(options.maintenance, operation), { binding: wireBinding }, callback),
						wire.logCacheEvictTake,
						(report) => closeReportOf(operation, report)
					)
				})
			},
			close: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("TenantCache.close", (callback) => wire.logCacheClose(cache, callback))
				})
		}
	}

	// ── Admin/migration certainty wrappers ─────────────────────────────────

	function decodeAdmin<Value>(
		operation: string,
		ref: OperationRef,
		result: AdminResultWire,
		decode: (value: AdminValueWire) => Value
	): AdminOutcome<Value> {
		switch (result.certainty) {
			case "completed":
				return {
					kind: "completed",
					ref,
					value: decode(result.value),
					phase: phaseOf(operation, result.publicationPhase)
				}
			case "not-started":
				return {
					kind: "not-started",
					ref,
					error: errorOf(operation, result.error),
					phase: phaseOf(operation, result.publicationPhase)
				}
			case "outcome-unknown":
				return {
					kind: "outcome-unknown",
					ref,
					error: errorOf(operation, result.error),
					phase: phaseOf(operation, result.publicationPhase)
				}
			case "report":
				// A read-only certainty from a mutating verb is a wire defect.
				throw invalidInput(operation)
		}
	}

	function adminMutation<Value>(
		operation: string,
		ref: OperationRef,
		work: ExecutionPolicy,
		request: () => AdminRequestWire,
		decode: (value: AdminValueWire) => Value
	): Effect.Effect<AdminOutcome<Value>, never, NativeRuntimeService> {
		return Effect.gen(function* () {
			const runtime = yield* Effect.result(core.runtime())
			if (Result.isFailure(runtime)) {
				return {
					kind: "not-started",
					ref,
					error: runtime.failure,
					phase: "prepared"
				} satisfies AdminOutcome<Value>
			}
			// `ref` is the caller-supplied operationId, fixed before dispatch.
			// Interrupt after the native lease is outcome-unknown under this
			// same ref; retry status/the same operationId, never a new mint.
			return yield* certaintyOperation(
				operation,
				cancel,
				(callback) => wire.logAdmin(runtime.success, policyWire(work, operation), request(), callback),
				wire.logAdminTake,
				(result) => decodeAdmin(operation, ref, result, decode),
				(error): AdminOutcome<Value> => ({ kind: "not-started", ref, error, phase: "prepared" }),
				(error): AdminOutcome<Value> => ({
					kind: "outcome-unknown",
					ref,
					error,
					phase: "dispatchedUnresolved"
				})
			)
		})
	}

	function adminQuery<Value>(
		operation: string,
		work: ExecutionPolicy,
		request: () => AdminRequestWire,
		decode: (value: AdminValueWire) => Value
	): Effect.Effect<Value, LogError, NativeRuntimeService> {
		return Effect.gen(function* () {
			const runtime = yield* core.runtime()
			return yield* logOperation(
				operation,
				cancel,
				(callback) => wire.logAdmin(runtime, policyWire(work, operation), request(), callback),
				wire.logAdminTake,
				(result) => {
					if (result.certainty !== "report") {
						throw invalidInput(operation)
					}
					return decode(result.value)
				}
			)
		})
	}

	function operationRef(binding: HistoryBinding, operationId: OperationId): OperationRef {
		return { identity: binding.identity, operation: operationId }
	}

	/** The lowered core SchemaSpec, when the caller supplied a schema. */
	function schemaField(schema: AnySchema | undefined): { readonly schema?: unknown } {
		return schema === undefined ? {} : { schema: core.schemaSpec(schema) }
	}

	/** The backup operation id, when supplied (32-hex, checked outbound). */
	function backupField(operation: string, backup: OperationId | undefined): { readonly backup?: string } {
		return backup === undefined ? {} : { backup: checkedString(operation, backup, 32) }
	}

	/** Optional source binding + target schema for activate/abort. */
	function targetFields(
		operation: string,
		options: MigrationTargetOptions
	): { readonly binding?: BindingWire; readonly schema?: unknown } {
		return {
			...(options.binding === undefined ? {} : { binding: bindingWire(operation, options.binding) }),
			...schemaField(options.schema)
		}
	}

	function destinationWire(operation: string, destination: BackupDestination): DestinationWire {
		if (destination.kind === "filesystem") {
			return { kind: "filesystem", directory: checkedString(operation, destination.directory, 4096) }
		}
		if (destination.kind !== "s3") {
			throw invalidInput(operation)
		}
		return {
			kind: "s3",
			bucket: checkedString(operation, destination.bucket, 255),
			prefix: checkedString(operation, destination.prefix, 1024),
			region: destination.region === undefined ? null : checkedString(operation, destination.region, 64),
			credentials: credentialsWire(operation, destination.credentials)
		}
	}

	function expectVerb<Verb extends AdminValueWire["verb"]>(
		operation: string,
		verb: Verb
	): (value: AdminValueWire) => Extract<AdminValueWire, { verb: Verb }> {
		return (value) => {
			if (value.verb !== verb) {
				throw invalidInput(operation)
			}
			return value as Extract<AdminValueWire, { verb: Verb }>
		}
	}

	function sourceAccessOf(wire: SourceAccessWire): SourceAccessReport {
		return {
			access: wire.access,
			operation: wire.operationId === null ? null : (wire.operationId as OperationId)
		}
	}

	// `ActivationRef` is the C11 shape declared in #migrations/types.ts:
	// { operation: OperationId, planSetDigest, target, targetGenesis }.
	function activationRefOf(wire: ActivationRefWire): ActivationRef {
		return {
			operation: wire.operationId as OperationId,
			planSetDigest: wire.planSetDigest as PlanSetDigest,
			target: identityOf(wire.target),
			targetGenesis: wire.targetGenesis as ActivationRef["targetGenesis"]
		}
	}

	function migrationRefOf(wire: MigrationRefWire): MigrationRef {
		return {
			operation: { identity: identityOf(wire.identity), operation: wire.operationId as OperationId },
			planSetDigest: wire.planSetDigest as PlanSetDigest,
			target: identityOf(wire.target)
		}
	}

	function activationRefWire(operation: string, ref: ActivationRef): ActivationRefWire {
		return {
			operationId: checkedString(operation, ref.operation, 32),
			planSetDigest: checkedString(operation, ref.planSetDigest, 64),
			target: identityWire(operation, ref.target),
			targetGenesis: checkedString(operation, ref.targetGenesis, 64)
		}
	}

	function migrationRefWire(operation: string, ref: MigrationRef): MigrationRefWire {
		return {
			identity: identityWire(operation, ref.operation.identity),
			operationId: checkedString(operation, ref.operation.operation, 32),
			planSetDigest: checkedString(operation, ref.planSetDigest, 64),
			target: identityWire(operation, ref.target)
		}
	}

	function migrateValueOf(operation: string, wire: MigrateValueWire): MigrateValue {
		switch (wire.kind) {
			case "up-to-date":
				return { kind: "up-to-date", binding: bindingOf(wire.binding) }
			case "ready-to-switch":
				return {
					kind: "ready-to-switch",
					deploymentBinding: bindingOf(wire.deploymentBinding),
					activation: activationRefOf(wire.activation)
				}
			case "paused":
				return {
					kind: "paused",
					error: errorOf(operation, wire.error),
					sourceState: sourceAccessOf(wire.sourceState)
				}
		}
	}

	function migrationStatusOf(operation: string, wire: MigrationStatusWire): MigrationStatus {
		switch (wire.kind) {
			case "up-to-date":
				return { kind: "up-to-date", appliedPrefixDigest: wire.appliedPrefixDigest }
			case "pending":
				return { kind: "pending", pending: wire.pending }
			case "in-progress":
				return { kind: "in-progress", operation: migrationRefOf(wire.operationRef).operation }
			case "paused":
				return {
					kind: "paused",
					operation: migrationRefOf(wire.operationRef).operation,
					error: errorOf(operation, wire.error),
					sourceState: sourceAccessOf(wire.sourceState)
				}
			case "ready-to-switch":
				return {
					kind: "ready-to-switch",
					operation: migrationRefOf(wire.operationRef).operation,
					activation: activationRefOf(wire.activation)
				}
			case "activated":
				return {
					kind: "activated",
					operation: migrationRefOf(wire.operationRef).operation,
					target: identityOf(wire.target)
				}
			case "aborted":
				return { kind: "aborted", operation: migrationRefOf(wire.operationRef).operation }
			case "outcome-unknown":
				return {
					kind: "outcome-unknown",
					operation: migrationRefOf(wire.operationRef).operation,
					error: errorOf(operation, wire.error)
				}
			case "drift":
				return { kind: "drift", detail: wire.detail }
			case "database-ahead":
				return { kind: "database-ahead", detail: wire.detail }
		}
	}

	const admin: AdminOperations = {
		checkpoint(binding, options) {
			const operation = "admin.checkpoint"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "checkpoint",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32)
				}),
				(value) => {
					const report = expectVerb(operation, "checkpoint")(value)
					return {
						at: stampOf(report.at),
						state: stateOf(report.state),
						root: report.root as RootId
					} satisfies CheckpointReport
				}
			)
		},
		pinRestorePoint(binding, options) {
			const operation = "admin.pinRestorePoint"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "pin-root",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					label: checkedString(operation, options.label, 256)
				}),
				(value) => {
					const report = expectVerb(operation, "pin-root")(value)
					return {
						root: report.root as RootId,
						at: stampOf(report.at),
						state: stateOf(report.state)
					} satisfies RestorePointReport
				}
			)
		},
		releaseRestorePoint(binding, options) {
			const operation = "admin.releaseRestorePoint"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "release-root",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					root: checkedString(operation, options.root, 128)
				}),
				(value) => {
					const report = expectVerb(operation, "release-root")(value)
					return {
						root: report.root as RootId,
						wasCurrentRecoveryBase: report.wasCurrentRecoveryBase
					} satisfies RootReleaseReport
				}
			)
		},
		rotateReceiptEpoch(binding, options) {
			const operation = "admin.rotateReceiptEpoch"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "rotate-receipt-epoch",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32)
				}),
				(value) => {
					const report = expectVerb(operation, "rotate-receipt-epoch")(value)
					return { openEpoch: report.openEpoch as ReceiptEpoch } satisfies ReceiptRotationReport
				}
			)
		},
		retireReceipts(binding, options) {
			const operation = "admin.retireReceipts"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => {
					if (typeof options.through !== "bigint" || options.through < 0n) {
						throw invalidInput(operation)
					}
					return {
						verb: "retire-receipts",
						binding: bindingWire(operation, binding),
						...schemaField(options.schema),
						operationId: checkedString(operation, options.operationId, 32),
						through: options.through
					}
				},
				(value) => {
					const report = expectVerb(operation, "retire-receipts")(value)
					return { retiredThrough: report.retiredThrough } satisfies ReceiptRetirementReport
				}
			)
		},
		collectGarbage(binding, options) {
			const operation = "admin.collectGarbage"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "collect-garbage",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32)
				}),
				(value) => {
					const report = expectVerb(operation, "collect-garbage")(value)
					return {
						objectEpoch: report.objectEpoch,
						swept: report.swept,
						orphansObserved: report.orphansObserved
					} satisfies GcReport
				}
			)
		},
		backup(binding, options) {
			const operation = "admin.backup"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "backup",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					destination: destinationWire(operation, options.destination)
				}),
				(value) => {
					const report = expectVerb(operation, "backup")(value)
					return {
						manifestDigest: report.manifestDigest,
						objects: report.objects,
						bytes: report.bytes,
						at: stampOf(report.at)
					} satisfies BackupReport
				}
			)
		},
		verifyBackup(destination, options) {
			const operation = "admin.verifyBackup"
			return adminQuery(
				operation,
				options,
				() => ({
					verb: "verify-backup",
					destination: destinationWire(operation, destination),
					...backupField(operation, options.backup)
				}),
				(value) => {
					const report = expectVerb(operation, "verify-backup")(value)
					return {
						identity: identityOf(report.identity),
						at: stampOf(report.at),
						state: stateOf(report.state),
						objects: report.objects,
						bytes: report.bytes,
						manifestDigest: report.manifestDigest
					} satisfies BackupVerification
				}
			)
		},
		restore(source, target, options) {
			const operation = "admin.restore"
			return adminMutation(
				operation,
				operationRef(target, options.operationId),
				options,
				() => ({
					verb: "restore",
					source: destinationWire(operation, source),
					target: bindingWire(operation, target),
					...schemaField(options.schema),
					...backupField(operation, options.backup),
					operationId: checkedString(operation, options.operationId, 32)
				}),
				(value) => {
					const report = expectVerb(operation, "restore")(value)
					return {
						identity: identityOf(report.identity),
						genesis: report.genesis,
						binding: bindingOf(report.binding)
					} satisfies RestoreReport
				}
			)
		},
		erase(binding, options) {
			const operation = "admin.erase"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "erase",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					retainRoots: options.retainRoots.map((root) => checkedString(operation, root, 128))
				}),
				(value) => {
					const report = expectVerb(operation, "erase")(value)
					return {
						tombstoned: report.tombstoned,
						retainedRoots: report.retainedRoots.map((root) => root as RootId),
						residual: report.residual
					} satisfies ErasureReport
				}
			)
		}
	}

	const migrations: MigrationOperations = {
		migrationStatus(binding, plans, work) {
			const operation = "migrations.status"
			return adminQuery(
				operation,
				work,
				() => ({
					verb: "migration-status",
					binding: bindingWire(operation, binding),
					...schemaField(work.schema),
					plans: plansWire(operation, plans)
				}),
				(value) => {
					const report = expectVerb(operation, "migration-status")(value)
					return migrationStatusOf(operation, report.status)
				}
			)
		},
		initialize(binding, plans, options) {
			const operation = "migrations.initialize"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "migration-initialize",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					plans: plansWire(operation, plans)
				}),
				(value) => {
					const report = expectVerb(operation, "migration-initialize")(value)
					return { binding: bindingOf(report.binding), genesis: report.genesis } satisfies InitializeValue
				}
			)
		},
		migrate(binding, plans, options) {
			const operation = "migrations.migrate"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				options,
				() => ({
					verb: "migration-migrate",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedString(operation, options.operationId, 32),
					plans: plansWire(operation, plans),
					to: options.to === undefined ? null : checkedString(operation, options.to, 256)
				}),
				(value) => {
					const report = expectVerb(operation, "migration-migrate")(value)
					return migrateValueOf(operation, report.value)
				}
			)
		},
		activateMigration(ref, options) {
			const operation = "migrations.activate"
			return adminMutation(
				operation,
				{ identity: ref.target, operation: ref.operation },
				options,
				() => ({
					verb: "migration-activate",
					ref: activationRefWire(operation, ref),
					...targetFields(operation, options)
				}),
				(value) => {
					const report = expectVerb(operation, "migration-activate")(value)
					return {
						target: identityOf(report.target),
						accessMode: report.accessMode,
						operation: report.operationId as OperationId,
						activatedNow: report.activatedNow
					} satisfies ActivationReport
				}
			)
		},
		abortMigration(ref, options) {
			const operation = "migrations.abort"
			return adminMutation(
				operation,
				ref.operation,
				options,
				() => ({
					verb: "migration-abort",
					ref: migrationRefWire(operation, ref),
					...targetFields(operation, options)
				}),
				(value) => {
					const report = expectVerb(operation, "migration-abort")(value)
					return {
						target: identityOf(report.target),
						targetFenced: report.targetFenced,
						sourceAccess: report.sourceAccess
					} satisfies AbortReport
				}
			)
		}
	}

	return {
		LocalHistory,
		HostedHistory,
		Command: CommandNamespace,
		TenantCache: TenantCacheNamespace,
		admin,
		migrations
	}
}
