/** Exact-version internal bridge, shared by core and log. Not a public API. */
import { native } from "#native.ts"
import type {
	DbHandle,
	FactValue,
	LogChainKind,
	LogCommandKind,
	LogCommandMetadata,
	LogCommandRef,
	LogCondition,
	LogDecisionStamp,
	LogIdentity,
	LogLimits,
	LogOutcomeWire,
	LogSchemaHandle,
	LogStateStamp,
	OwnedHandle,
	ParsedQuery,
	QueryParam,
	Violation,
	WitnessHandle
} from "#native.ts"
import type { SchemaSpec } from "#spec.ts"

export interface RuntimeHandle {
	readonly __runtime: unique symbol
}
export interface OperationHandle {
	readonly __operation: unique symbol
}
export interface DirectoryHandle {
	readonly __directory: unique symbol
}
/** A worker-affine READ session: one pinned coherent generation. */
export interface SessionHandle {
	readonly __session: unique symbol
}
/** A worker-affine WRITE session: the one exclusive engine writer. */
export interface WriterHandle {
	readonly __writer: unique symbol
}

export type ManagedDbOutcome =
	| { readonly tag: "accepted"; readonly db: DbHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| {
			readonly tag: "refused"
			readonly kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch" | "destinationExists"
			readonly message: string
	  }

export interface SessionOpenedWire {
	readonly session: SessionHandle
	readonly witness: WitnessHandle
	readonly generation: bigint
}

export type PreparedOutcome =
	| { readonly ok: true; readonly prepared: bigint }
	| { readonly ok: false; readonly kind: "irError"; readonly message: string }

export interface MutationWire {
	readonly submitted: bigint
	readonly changed: bigint
}

export type WriteOutcomeWire =
	| { readonly tag: "accepted"; readonly generation: bigint }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "abandoned" }
	| { readonly tag: "moved"; readonly witnessed: bigint; readonly current: bigint }

export type AdmitOutcome =
	| { readonly tag: "accepted"; readonly value: OwnedHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }

// The 0.x five-verb JS filesystem transport (`runtimeFs`/`runtimeFsTake`,
// FsRequestWire/FsResultWire) is DELETED with the TS CAS authority: all
// object-store work is native (C07, P05's store rewrite); the log machine
// drives FsStore/S3Store inside `ts/crate` and no JS layer holds a
// conditional-store verb anymore.

export type LogTakeWire =
	| { readonly ok: false; readonly kind: LogCommandKind | LogChainKind; readonly message: string }
	| {
			/** Command sealed: envelope bytes + reference. */
			readonly ok: true
			readonly bytes: Uint8Array
			readonly ref: LogCommandRef
	  }
	| {
			/** Command parsed. */
			readonly ok: true
			readonly identity: LogIdentity
			readonly receiptEpoch: bigint
			readonly requestId: string
			readonly condition: LogCondition
			readonly changes: Uint8Array
			readonly result: Uint8Array
			readonly ref: LogCommandRef
	  }
	| {
			/** Decision framed. */
			readonly ok: true
			readonly bytes: Uint8Array
			readonly digest: Uint8Array
	  }
	| {
			/** Decision decoded (and chain-verified when a parent was given). */
			readonly ok: true
			readonly identity: LogIdentity
			readonly seq: bigint
			readonly parent: LogDecisionStamp
			readonly beforeState: LogStateStamp
			readonly afterState: LogStateStamp
			readonly commandBytes: Uint8Array
			readonly command: LogCommandRef
			readonly outcome: LogOutcomeWire
			readonly digest: Uint8Array
	  }

export interface LogDecisionParts {
	readonly identity: LogIdentity
	readonly seq: bigint
	readonly parent: LogDecisionStamp
	readonly beforeState: LogStateStamp
	readonly afterState: LogStateStamp
	readonly outcome: LogOutcomeWire
}

export interface PolicyWire {
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly rows: bigint
	readonly workUnits: bigint
	readonly timeoutMs: number
}

export interface OptionsWire {
	readonly workers: number
	readonly queueCapacity: number
	readonly cleanupCapacity: number
	readonly ownerCapacity: number
	readonly nativeHandleCapacity: number
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly chunkBytes: bigint
	readonly cleanupTimeoutMs: number
}

export interface InspectionWire {
	readonly phase: "open" | "closing" | "closed"
	readonly queued: bigint
	readonly active: bigint
	readonly retained: bigint
	readonly owners: bigint
	readonly databases: bigint
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
}

export type CloseWire =
	| { readonly kind: "closed" }
	| { readonly kind: "incomplete"; readonly outstanding: InspectionWire }
	| { readonly kind: "failed" }

interface RuntimeNative {
	runtimeErrorCodes(): readonly string[]
	runtimeOpen(options: OptionsWire): RuntimeHandle
	runtimeReady(runtime: RuntimeHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeHash(runtime: RuntimeHandle, policy: PolicyWire, bytes: Uint8Array, callback: () => void): OperationHandle
	runtimeTake(operation: OperationHandle): Uint8Array | null
	runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void): void
	runtimeClose(runtime: RuntimeHandle, callback: (report: CloseWire) => void): void
	runtimeInspect(runtime: RuntimeHandle): InspectionWire
	runtimeDirectoryAcquire(runtime: RuntimeHandle, policy: PolicyWire, path: string, callback: () => void): OperationHandle
	runtimeDirectoryTake(operation: OperationHandle): DirectoryHandle
	runtimeDirectoryBegin(owner: DirectoryHandle, policy: PolicyWire): OperationHandle
	runtimeDirectoryCheck(operation: OperationHandle): void
	runtimeDirectoryEnd(operation: OperationHandle): void
	runtimeDirectoryClose(owner: DirectoryHandle, remove: boolean, callback: (report: CloseWire) => void): void
	runtimeDirectoryDbOpen(owner: DirectoryHandle, policy: PolicyWire, childName: string, spec: SchemaSpec, create: boolean, callback: () => void): OperationHandle
	runtimeDbTake(operation: OperationHandle): ManagedDbOutcome
	runtimeManagedDbClose(db: DbHandle, callback: (report: CloseWire) => void): void
	/** Managed publish: attach an admitted OwnedInstance to the directory owner. Take with `runtimeDbTake`. */
	runtimeDirectoryPublish(owner: DirectoryHandle, policy: PolicyWire, childName: string, instance: OwnedHandle, callback: () => void): OperationHandle

	// --- worker-affine sessions (C09): the !Send read instance / write
	// transaction / prepared queries live on one owning thread; only owned
	// data crosses; every verb is a registered bounded operation. ---
	runtimeDbSession(db: DbHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeSessionTake(operation: OperationHandle): SessionOpenedWire
	runtimeDbWriter(db: DbHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeDbWriterFrom(db: DbHandle, witness: WitnessHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeWriterTake(operation: OperationHandle): WriterHandle
	runtimeSessionClose(session: SessionHandle, callback: (report: CloseWire) => void): void
	runtimeWriterClose(writer: WriterHandle, callback: (report: CloseWire) => void): void
	runtimeSessionScan(session: SessionHandle, policy: PolicyWire, relationId: number, callback: () => void): OperationHandle
	runtimeSessionCount(session: SessionHandle, policy: PolicyWire, relationId: number, callback: () => void): OperationHandle
	runtimeSessionContains(session: SessionHandle, policy: PolicyWire, relationId: number, values: readonly FactValue[], callback: () => void): OperationHandle
	runtimeSessionGet(session: SessionHandle, policy: PolicyWire, relationId: number, keyStatementId: number, keyValues: readonly FactValue[], callback: () => void): OperationHandle
	runtimeSessionQuery(session: SessionHandle, policy: PolicyWire, query: ParsedQuery, params: readonly QueryParam[], callback: () => void): OperationHandle
	runtimeSessionPrepare(session: SessionHandle, policy: PolicyWire, query: ParsedQuery, callback: () => void): OperationHandle
	/** Prepared-id execution (worker-session lane). The db-bridge's `runtimeSessionExecute` (db-native.ts) is the ParsedQuery form; this is the retained-prepared twin under its own name. */
	runtimeSessionExecutePrepared(session: SessionHandle, policy: PolicyWire, prepared: bigint, params: readonly QueryParam[], callback: () => void): OperationHandle
	runtimeSessionPreparedClose(session: SessionHandle, policy: PolicyWire, prepared: bigint, callback: () => void): OperationHandle
	runtimeWriteInsert(writer: WriterHandle, policy: PolicyWire, relationId: number, rows: bigint, cells: readonly FactValue[], callback: () => void): OperationHandle
	runtimeWriteDelete(writer: WriterHandle, policy: PolicyWire, relationId: number, rows: bigint, cells: readonly FactValue[], callback: () => void): OperationHandle
	runtimeWriteContains(writer: WriterHandle, policy: PolicyWire, relationId: number, values: readonly FactValue[], callback: () => void): OperationHandle
	runtimeWriteGet(writer: WriterHandle, policy: PolicyWire, relationId: number, keyStatementId: number, keyValues: readonly FactValue[], callback: () => void): OperationHandle
	runtimeWriteFinish(writer: WriterHandle, policy: PolicyWire, commit: boolean, callback: () => void): OperationHandle

	// --- typed one-shot takes for the session/pool outputs ---
	runtimeRowsTake(operation: OperationHandle): FactValue[][]
	runtimeRowTake(operation: OperationHandle): FactValue[] | null
	runtimeBoolTake(operation: OperationHandle): boolean
	runtimeCountTake(operation: OperationHandle): bigint
	runtimePreparedTake(operation: OperationHandle): PreparedOutcome
	runtimeMutationTake(operation: OperationHandle): MutationWire
	runtimeWriteTake(operation: OperationHandle): WriteOutcomeWire

	// --- builder admission and owned-instance work on the one executor ---
	runtimeBuilderAdmit(runtime: RuntimeHandle, builder: import("#native.ts").BuilderHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeAdmitTake(operation: OperationHandle): AdmitOutcome
	runtimeOwnedScan(runtime: RuntimeHandle, instance: OwnedHandle, policy: PolicyWire, relationId: number, callback: () => void): OperationHandle
	runtimeOwnedQuery(runtime: RuntimeHandle, instance: OwnedHandle, policy: PolicyWire, query: ParsedQuery, params: readonly QueryParam[], callback: () => void): OperationHandle

	// --- successor log grammar on the executor (charged hashing over
	// whole canonical change payloads; C06 through C09) ---
	runtimeLogCommandSeal(runtime: RuntimeHandle, schema: LogSchemaHandle, policy: PolicyWire, metadata: LogCommandMetadata, changes: Uint8Array, result: Uint8Array | null, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogCommandParse(runtime: RuntimeHandle, schema: LogSchemaHandle, policy: PolicyWire, bytes: Uint8Array, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogDecisionEncode(runtime: RuntimeHandle, policy: PolicyWire, parts: LogDecisionParts, commandBytes: Uint8Array, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogDecisionDecode(runtime: RuntimeHandle, policy: PolicyWire, bytes: Uint8Array, parent: LogDecisionStamp | null, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogTake(operation: OperationHandle): LogTakeWire
}

// The checked source/fresh-addon roster test pins this private declaration.
export const runtimeNative = native as typeof native & RuntimeNative
