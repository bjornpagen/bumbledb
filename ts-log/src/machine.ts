/**
 * The one thin Effect layer over the internal native history machine.
 * Everything here is language adaptation: capability bookkeeping, wire
 * conversion, certainty preservation and scope ownership. No protocol
 * transition, CAS, catch-up, retry loop, checkpoint, GC step, lock, or
 * cache-eviction decision is implemented in JavaScript — those live in the
 * Rust machine reached through the shared executor. The factory takes the
 * wire and the core integration seam so authored tests can drive the layer
 * deterministically; production binds the real addon in `#production.ts`.
 */
import type { AnySchema, DbError, NativeRuntime, SchemaId } from "@bjornpagen/bumbledb"
import { Uuid } from "@bjornpagen/bumbledb"
import type {
	Capability,
	ChangeSet,
	CompleteResult,
	QueryReader,
	RuntimeHandle,
	SnapshotHandle
} from "@bjornpagen/bumbledb/internal/log"
import type { Scope } from "effect"
import { Effect, Result } from "effect"
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
	ReceiptEpoch,
	RequestId,
	RootId,
	StateStamp
} from "#identity.ts"
import type {
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
	DestinationWire,
	ErrorWire,
	FreshnessWire,
	HealthWire,
	HistoryCapability,
	HistoryHandleWire,
	HistoryInspectionWire,
	HistoryRequestWire,
	HistoryResultWire,
	InstalledTransitionWire,
	LogNative,
	OutcomeWire,
	PopulationHandle,
	PreconditionWire,
	ProvenanceWire,
	ReceiptWire,
	ResultWire,
	SealRequestWire,
	StampWire,
	StateWire,
	SubmitWire,
	TransitionCaptureWire,
	TransitionContractWire,
	TransitionRequestWire,
	TransitionResultWire
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
	ReadOptions,
	SubmitOptions,
	TenantCacheOptions
} from "#options.ts"
import type {
	AdminOutcome,
	BackupReport,
	BackupVerification,
	CacheInspection,
	CheckpointReport,
	CommandResult,
	CommandScalar,
	ErasureReport,
	GcReport,
	HistoryInspection,
	LocalMaterializationHealth,
	ReceiptRetirementReport,
	ReceiptRotationReport,
	ResolveOutcome,
	RestorePointReport,
	RestoreReport,
	RootReleaseReport,
	SubmitOutcome,
	TerminalOutcome,
	TerminalReceipt
} from "#outcome.ts"
import type { Command, CommandInput, History, HistoryBorrow, PublishedSnapshot, TenantCache } from "#surface.ts"
import type {
	ActivatedTransition,
	InstalledTransition,
	Population,
	ReadyTransition,
	TransitionCapture,
	TransitionContract,
	TransitionOperations,
	TransitionResolution,
	TransitionStart
} from "#transition.ts"

// ── Shared core capabilities ─────────────────────────────────────────────

export interface PublishedReadCapability<S extends AnySchema> {
	/** Self-contained (closure-captured) methods; never `this`-dependent. */
	readonly get: QueryReader<S>["get"]
	/** Core QueryReader.execute — yields the CompleteResult owner. */
	readonly execute: QueryReader<S>["execute"]
	readonly prepare: QueryReader<S>["prepare"]
}

/** Published execute result: the core CompleteResult, not a copied page. */
export type PublishedCompleteResult<A> = CompleteResult<A>

/** The exact shape the core's landed `internalChanges` accessor returns. */
export interface CoreChangesView {
	readonly handle: unknown
	readonly schemaId: SchemaId
}

export interface CoreIntegration {
	/**
	 * Wrap a published core snapshot Capability in the exact core QueryReader
	 * (`internalPublishedReader` in production).
	 */
	reader<S extends AnySchema>(core: SnapshotHandle | Capability, schema: S): PublishedReadCapability<S>
	/**
	 * The core's private ChangeSet registry accessor (`internalChanges`):
	 * `undefined` for a foreign dynamic object; `handle` is the retained native change — the
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
		storage: wire.storage,
		operations: { queued: wire.queued, active: wire.active }
	}
}

function cacheInspectionOf(wire: CacheInspectionWire): CacheInspection {
	return {
		openCount: wire.openCount,
		opening: wire.opening,
		maxOpen: wire.maxOpen,
		evictions: wire.evictions,
		slots: wire.slots.map((slot) => ({
			binding: slot.binding,
			state: slot.state,
			borrows: slot.borrows
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

function checkedUuid(operation: string, value: string): string {
	if (!Uuid.isUuid(value)) {
		throw invalidInput(operation)
	}
	return value
}

function identityWire(operation: string, identity: DatabaseIdentity) {
	return {
		databaseId: checkedUuid(operation, identity.databaseId),
		incarnationId: checkedUuid(operation, identity.incarnationId),
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
		sessionToken:
			credentials.sessionToken === undefined ? null : checkedString(operation, credentials.sessionToken, 4096)
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
		requestId: checkedUuid(operation, ref.id.requestId),
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

function preconditionWire(operation: string, precondition: import("#surface.ts").Precondition): PreconditionWire {
	if (precondition.kind === "blind") {
		return { kind: "blind" }
	}
	if (precondition.kind !== "exact-state") {
		throw invalidInput(operation)
	}
	return {
		kind: "exact-state",
		incarnation: checkedUuid(operation, precondition.at.incarnation),
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
		operationId: checkedUuid(operation, creation.operationId),
		artifact: creation.artifact
	}
}

// ── The machine ────────────────────────────────────────────────────────────

interface CommandEntry {
	readonly handle: CommandWire["command"]
	readonly state: { closed: boolean }
}

export interface LogMachine {
	readonly Transition: TransitionOperations
	readonly LocalHistory: {
		open<S extends AnySchema>(binding: LocalBinding, schema: S): OpenEffect<S>
		create<S extends AnySchema>(binding: LocalBinding, schema: S, options: LocalCreateOptions): OpenEffect<S>
	}
	readonly HostedHistory: {
		open<S extends AnySchema>(binding: HostedBinding, schema: S, options?: HostedOpenOptions): OpenEffect<S>
		create<S extends AnySchema>(binding: HostedBinding, schema: S, options: HostedCreateOptions): OpenEffect<S>
	}
	readonly Command: {
		seal<S extends AnySchema>(input: CommandInput<S>): Effect.Effect<Command<S>, LogError, Scope.Scope>
		encode<S extends AnySchema>(command: Command<S>): Effect.Effect<Uint8Array, LogError>
		decode<S extends AnySchema>(
			bytes: Uint8Array,
			schema: S
		): Effect.Effect<Command<S>, LogError, NativeRuntimeService | Scope.Scope>
	}
	readonly TenantCache: {
		make<S extends AnySchema>(
			schema: S,
			options: TenantCacheOptions
		): Effect.Effect<TenantCache<S>, LogError, NativeRuntimeService | Scope.Scope>
	}
	readonly admin: AdminOperations
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

export interface AdminIdentityOptions extends TenantOpenOptions {
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
		options: { readonly backup?: OperationId }
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

export function makeLogMachine(wire: LogWire, core: CoreIntegration): LogMachine {
	const cancel: CancelVerb = (operation, callback) => wire.runtimeCancel(operation, callback)
	const commandEntries = new WeakMap<object, CommandEntry>()
	const historyEntries = new WeakMap<object, { readonly capability: HistoryCapability; readonly schema: AnySchema }>()

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
		snapshot: SnapshotHandle,
		provenance: ProvenanceWire
	): PublishedSnapshot<S> {
		const capability = core.reader(snapshot, schema)
		return {
			identity: identityOf(provenance.identity),
			decisionStamp: stampOf(provenance.decision),
			stateStamp: stateOf(provenance.state),
			freshness: freshnessOf(provenance.freshness),
			get: capability.get,
			execute: capability.execute,
			prepare: capability.prepare,
			close: () => drainClose("PublishedSnapshot.close", (callback) => wire.runtimeSnapshotClose(snapshot, callback))
		}
	}

	interface FacadeMembers<S extends AnySchema> {
		readonly identity: DatabaseIdentity
		readonly receiptEpoch: ReceiptEpoch
		snapshot(options: ReadOptions): Effect.Effect<PublishedSnapshot<S>, LogError, Scope.Scope>
		submit(command: Command<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome>
		resolve(ref: CommandRef): Effect.Effect<ResolveOutcome, LogError>
		inspect(): Effect.Effect<HistoryInspection, LogError>
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
					localHealth: healthOf(operation, outcome.localHealth)
				}
			case "not-submitted":
				return {
					kind: "not-submitted",
					command: ref,
					error: errorOf(operation, outcome.error)
				}
			case "outcome-unknown":
				return {
					kind: "outcome-unknown",
					command: ref,
					error: errorOf(operation, outcome.error)
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
					(callback) => wire.logHistoryCall(capability, request, callback),
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
					return call(operation, request, (result) => {
						if (result.verb !== "snapshot") {
							throw invalidInput(operation)
						}
						return makeSnapshot(schema, result.snapshot, result.provenance)
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
							error: closedHandle(operation)
						})
					}
					return certaintyOperation(
						operation,
						cancel,
						(callback) =>
							wire.logHistoryCall(
								capability,
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
							error
						}),
						(error): SubmitOutcome => ({
							kind: "outcome-unknown",
							command: ref,
							error
						})
					)
				})
			},
			resolve(ref: CommandRef) {
				const operation = "History.resolve"
				return Effect.suspend(() => {
					let wireRef: CommandRefWire
					try {
						wireRef = refWire(operation, ref)
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return call(operation, { verb: "resolve", ref: wireRef }, (result) => decodeResolve(operation, result))
				})
			},
			inspect() {
				const operation = "History.inspect"
				return call(operation, { verb: "inspect" }, (result) => {
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
		const history: History<S> = {
			...members,
			close: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("History.close", (callback) => wire.logHistoryClose(handle.history, callback))
				})
		}
		historyEntries.set(history, { capability: handle.history, schema })
		return history
	}

	function makeBorrow<S extends AnySchema>(schema: S, handle: HistoryHandleWire): HistoryBorrow<S> {
		const state = { closed: false }
		const members = makeFacadeMembers(schema, handle.history, handle.meta, state)
		const history: HistoryBorrow<S> = {
			...members,
			release: () =>
				Effect.suspend(() => {
					state.closed = true
					return drainClose("HistoryBorrow.release", (callback) => wire.logBorrowRelease(handle.history, callback))
				})
		}
		historyEntries.set(history, { capability: handle.history, schema })
		return history
	}

	function openHistory<S extends AnySchema>(
		operation: string,
		kind: "local" | "hosted",
		mode: "open" | "create",
		binding: HistoryBinding,
		schema: S,
		options: HostedOpenOptions & { readonly creation?: CreationOptions }
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
		open: (binding, schema) => openHistory("LocalHistory.open", "local", "open", binding, schema, {}),
		create: (binding, schema, options) =>
			openHistory("LocalHistory.create", "local", "create", binding, schema, options)
	}

	const HostedHistory: LogMachine["HostedHistory"] = {
		open: (binding, schema, options = {}) =>
			openHistory("HostedHistory.open", "hosted", "open", binding, schema, options),
		create: (binding, schema, options) =>
			openHistory("HostedHistory.create", "hosted", "create", binding, schema, options)
	}

	const CommandNamespace: LogMachine["Command"] = {
		seal<S extends AnySchema>(input: CommandInput<S>) {
			const operation = "Command.seal"
			const acquire = Effect.suspend(() => {
				// The exact core registry accessor: a foreign dynamic object
				// refuses before any native dispatch (the Db.apply pattern);
				// a scope/change schema mismatch refuses before native work.
				// Native admission owns the ChangeSet's lifetime.
				const internal = core.changes(input.changes)
				if (internal === undefined) {
					return Effect.fail<LogError>(invalidInput(operation))
				}
				let request: SealRequestWire
				try {
					if (String(internal.schemaId) !== String(input.scope.schemaId)) {
						throw invalidInput(operation)
					}
					request = {
						scope: identityWire(operation, input.scope),
						receiptEpoch: input.id.receiptEpoch,
						requestId: checkedUuid(operation, input.id.requestId),
						precondition: preconditionWire(operation, input.precondition),
						result: resultWire(operation, input.result)
					}
				} catch (cause) {
					return Effect.fail(logFailure(operation, cause))
				}
				return logOperation(
					operation,
					cancel,
					(callback) => wire.logCommandSeal(internal.handle, request, callback),
					wire.logCommandTake,
					(cw) => makeCommand<S>(cw)
				)
			})
			return scopedResource(operation, acquire, (command) => command.close())
		},
		encode<S extends AnySchema>(command: Command<S>) {
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
					(callback) => wire.logCommandEncode(entry.handle, callback),
					wire.logBytesTake,
					(bytes) => bytes
				)
			})
		},
		decode<S extends AnySchema>(bytes: Uint8Array, schema: S) {
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
						(callback) => wire.logCommandDecode(runtime, bytes, core.schemaSpec(schema), callback),
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
						request = {
							maxOpen: checkedCount(operation, options.maxOpen, 0xffffffff),
							schema: core.schemaSpec(schema)
						}
					} catch (cause) {
						return Effect.fail(logFailure(operation, cause))
					}
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logCacheMake(runtime, request, callback),
						wire.logCacheTake,
						(cache) => makeCache(schema, cache)
					)
				})
				return yield* scopedResource(operation, acquire, (cache) => cache.close())
			})
		}
	}

	function makeCache<S extends AnySchema>(schema: S, cache: CacheHandle): TenantCache<S> {
		const state = { closed: false }
		return {
			acquire(binding: HistoryBinding) {
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
						(callback) => wire.logCacheAcquire(cache, { binding: wireBinding }, callback),
						wire.logBorrowTake,
						(handle) => makeBorrow(schema, handle)
					)
				})
				return scopedResource(operation, acquire, (borrow) => borrow.release())
			},
			inspect() {
				const operation = "TenantCache.inspect"
				return Effect.suspend(() => {
					if (state.closed) {
						return Effect.fail(closedHandle(operation))
					}
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logCacheInspect(cache, callback),
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
						(callback) => wire.logCacheEvict(cache, { binding: wireBinding }, callback),
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
					value: decode(result.value)
				}
			case "not-started":
				return {
					kind: "not-started",
					ref,
					error: errorOf(operation, result.error)
				}
			case "outcome-unknown":
				return {
					kind: "outcome-unknown",
					ref,
					error: errorOf(operation, result.error)
				}
			case "report":
				// A read-only certainty from a mutating verb is a wire defect.
				throw invalidInput(operation)
		}
	}

	function adminMutation<Value>(
		operation: string,
		ref: OperationRef,
		request: () => AdminRequestWire,
		decode: (value: AdminValueWire) => Value
	): Effect.Effect<AdminOutcome<Value>, never, NativeRuntimeService> {
		return Effect.gen(function* () {
			const runtime = yield* Effect.result(core.runtime())
			if (Result.isFailure(runtime)) {
				return {
					kind: "not-started",
					ref,
					error: runtime.failure
				} satisfies AdminOutcome<Value>
			}
			// `ref` is the caller-supplied operationId, fixed before dispatch.
			// Interrupt after the native lease is outcome-unknown under this
			// same ref; retry status/the same operationId, never a new mint.
			return yield* certaintyOperation(
				operation,
				cancel,
				(callback) => wire.logAdmin(runtime.success, request(), callback),
				wire.logAdminTake,
				(result) => decodeAdmin(operation, ref, result, decode),
				(error): AdminOutcome<Value> => ({ kind: "not-started", ref, error }),
				(error): AdminOutcome<Value> => ({
					kind: "outcome-unknown",
					ref,
					error
				})
			)
		})
	}

	function adminQuery<Value>(
		operation: string,
		request: () => AdminRequestWire,
		decode: (value: AdminValueWire) => Value
	): Effect.Effect<Value, LogError, NativeRuntimeService> {
		return Effect.gen(function* () {
			const runtime = yield* core.runtime()
			return yield* logOperation(
				operation,
				cancel,
				(callback) => wire.logAdmin(runtime, request(), callback),
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

	/** The backup operation id, when supplied (canonical UUID, checked outbound). */
	function backupField(operation: string, backup: OperationId | undefined): { readonly backup?: string } {
		return backup === undefined ? {} : { backup: checkedUuid(operation, backup) }
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

	const admin: AdminOperations = {
		checkpoint(binding, options) {
			const operation = "admin.checkpoint"
			return adminMutation(
				operation,
				operationRef(binding, options.operationId),
				() => ({
					verb: "checkpoint",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId)
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
				() => ({
					verb: "pin-root",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId),
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
				() => ({
					verb: "release-root",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId),
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
				() => ({
					verb: "rotate-receipt-epoch",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId)
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
				() => {
					if (typeof options.through !== "bigint" || options.through < 0n) {
						throw invalidInput(operation)
					}
					return {
						verb: "retire-receipts",
						binding: bindingWire(operation, binding),
						...schemaField(options.schema),
						operationId: checkedUuid(operation, options.operationId),
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
				() => ({
					verb: "collect-garbage",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId)
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
				() => ({
					verb: "backup",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId),
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
				() => ({
					verb: "restore",
					source: destinationWire(operation, source),
					target: bindingWire(operation, target),
					...schemaField(options.schema),
					...backupField(operation, options.backup),
					operationId: checkedUuid(operation, options.operationId)
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
				() => ({
					verb: "erase",
					binding: bindingWire(operation, binding),
					...schemaField(options.schema),
					operationId: checkedUuid(operation, options.operationId),
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
	function contractWire(operation: string, contract: TransitionContract): TransitionContractWire {
		return {
			operationId: checkedUuid(operation, contract.operation),
			source: identityWire(operation, contract.source),
			target: identityWire(operation, contract.target),
			commitment: checkedString(operation, contract.commitment, 64)
		}
	}
	function contractOf(contract: TransitionContractWire): TransitionContract {
		return {
			operation: contract.operationId as OperationId,
			source: identityOf(contract.source),
			target: identityOf(contract.target),
			commitment: contract.commitment
		}
	}
	function captureOf(captured: TransitionCaptureWire): TransitionCapture {
		return {
			contract: contractOf(captured.contract),
			decision: stampOf(captured.decision),
			state: stateOf(captured.state)
		}
	}
	function installedOf(installed: InstalledTransitionWire): InstalledTransition {
		return {
			captured: captureOf(installed.captured),
			applicationDigest: installed.applicationDigest,
			bytes: installed.bytes
		}
	}
	function readyOf(result: Extract<TransitionResultWire, { kind: "ready" }>): ReadyTransition {
		return {
			kind: "ready",
			installed: installedOf(result.installed),
			binding: {
				kind: "local",
				directory: result.directory,
				identity: identityOf(result.installed.captured.contract.target)
			}
		}
	}
	function resolutionOf(operation: string, result: TransitionResultWire): TransitionResolution {
		switch (result.kind) {
			case "ready":
				return readyOf(result)
			case "activated":
				return {
					kind: "activated",
					contract: contractOf(result.contract),
					genesis: result.genesis as DecisionDigest,
					binding: { kind: "local", directory: result.directory, identity: identityOf(result.contract.target) }
				}
			case "uninstalled":
				return { kind: "uninstalled" }
			case "aborted":
				return { kind: "aborted" }
			default:
				throw invalidInput(operation)
		}
	}
	function transitionCall<A>(
		operation: string,
		history: object,
		request: () => TransitionRequestWire,
		accept: (result: TransitionResultWire) => A
	): Effect.Effect<A, LogError> {
		return Effect.suspend(() => {
			const entry = historyEntries.get(history)
			if (entry === undefined) return Effect.fail(invalidInput(operation))
			let input: TransitionRequestWire
			try {
				input = request()
			} catch (cause) {
				return Effect.fail(logFailure(operation, cause))
			}
			return logOperation(
				operation,
				cancel,
				(callback) => wire.logTransitionCall(entry.capability, input, callback),
				wire.logTransitionResult,
				accept
			)
		})
	}
	function makePopulation<T extends AnySchema>(schema: T, handle: PopulationHandle): Population<T> {
		return {
			schema,
			apply(changes) {
				const operation = "Population.apply"
				return Effect.suspend(() => {
					const entry = core.changes(changes)
					if (entry === undefined) return Effect.fail(invalidInput(operation))
					return logOperation(
						operation,
						cancel,
						(callback) => wire.logPopulationApply(handle, entry.handle, callback),
						wire.logTransitionResult,
						(result) => {
							if (result.kind !== "applied") throw invalidInput(operation)
						}
					)
				})
			},
			finish() {
				const operation = "Population.finish"
				return logOperation(
					operation,
					cancel,
					(callback) => wire.logPopulationFinish(handle, callback),
					wire.logTransitionResult,
					(result) => {
						if (result.kind !== "ready") throw invalidInput(operation)
						return readyOf(result)
					}
				)
			},
			close: () => drainClose("Population.close", (callback) => wire.logPopulationClose(handle, callback))
		}
	}
	const TransitionNamespace: TransitionOperations = {
		begin<S extends AnySchema, T extends AnySchema>(
			source: History<S> | HistoryBorrow<S>,
			target: T,
			contract: TransitionContract
		) {
			const operation = "Transition.begin"
			const acquire = transitionCall(
				operation,
				source,
				() => ({ verb: "begin", contract: contractWire(operation, contract), schema: core.schemaSpec(target) }),
				(result): TransitionStart<S, T> => {
					if (result.kind === "activated") return resolutionOf(operation, result) as ActivatedTransition
					if (result.kind === "aborted") return { kind: "aborted" }
					if (result.kind !== "ready" && result.kind !== "populating") throw invalidInput(operation)
					const entry = historyEntries.get(source)
					if (entry === undefined || result.source === undefined) throw invalidInput(operation)
					const reader = makeSnapshot(entry.schema as S, result.source.snapshot, result.source.provenance)
					if (result.kind === "ready") return { ...readyOf(result), source: reader }
					return {
						kind: "populating",
						captured: captureOf(result.captured),
						source: reader,
						population: makePopulation(target, result.population),
						binding: {
							kind: "local",
							directory: result.directory,
							identity: identityOf(result.captured.contract.target)
						}
					}
				}
			)
			return scopedResource(operation, acquire, (value) =>
				Effect.gen(function* () {
					if (value.kind === "populating") {
						const population = yield* value.population.close()
						const source = yield* value.source.close()
						return population.kind === "closed" ? source : population
					}
					if (value.kind === "ready") return yield* value.source.close()
					return { kind: "closed" as const }
				})
			)
		},
		resolve(source, target, contract) {
			const operation = "Transition.resolve"
			return transitionCall(
				operation,
				source,
				() => ({ verb: "resolve", contract: contractWire(operation, contract), schema: core.schemaSpec(target) }),
				(result) => resolutionOf(operation, result)
			)
		},
		inspect<T extends AnySchema>(target: History<T> | HistoryBorrow<T>, installed: InstalledTransition) {
			const operation = "Transition.inspect"
			const acquire = transitionCall(
				operation,
				target,
				() => ({ verb: "inspect", evidence: installed.bytes }),
				(result) => {
					const entry = historyEntries.get(target)
					if (entry === undefined || result.kind !== "ready" || result.source === undefined)
						throw invalidInput(operation)
					return makeSnapshot(entry.schema as T, result.source.snapshot, result.source.provenance)
				}
			)
			return scopedResource(operation, acquire, (reader) => reader.close())
		},
		activate(source, target, installed) {
			const operation = "Transition.activate"
			return transitionCall(
				operation,
				source,
				() => ({ verb: "activate", evidence: installed.bytes, schema: core.schemaSpec(target) }),
				(result) => {
					const value = resolutionOf(operation, result)
					if (value.kind !== "activated") throw invalidInput(operation)
					return value
				}
			)
		},
		abort(source, target, contract) {
			const operation = "Transition.abort"
			return transitionCall(
				operation,
				source,
				() => ({ verb: "abort", contract: contractWire(operation, contract), schema: core.schemaSpec(target) }),
				(result) => {
					if (result.kind !== "aborted") throw invalidInput(operation)
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
		Transition: TransitionNamespace
	}
}
