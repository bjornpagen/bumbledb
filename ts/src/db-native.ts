/**
 * Private database bridge over worker-owned core snapshots and results.
 * The implementation lives in `ts/crate/src/`; this is not a public API.
 * Every verb runs on the shared bounded executor, registers under its
 * operation ownership and completes through the registered callback; `runtimeCancel` cancels
 * and joins any of them. Close verbs report the real drain outcome through
 * `CloseWire`.
 */
import type { DbHandle, ParsedQuery, QueryParam, SealedDescriptor, Violation } from "#native.ts"
import { native } from "#native.ts"
import type { CellValue } from "#rows.ts"
import type { CloseWire, OperationHandle, RuntimeHandle } from "#runtime-native.ts"
import type { SchemaSpec } from "#spec.ts"

export interface SnapshotHandle {
	readonly __snapshot: unique symbol
}
export interface PreparedHandle {
	readonly __prepared: unique symbol
}
export interface ResultHandle {
	readonly __result: unique symbol
}
export interface CursorHandle {
	readonly __cursor: unique symbol
}
export interface DraftHandle {
	readonly __draft: unique symbol
}
export interface ChangesHandle {
	readonly __changes: unique symbol
}

/** The core-local witness: catalog/store identity plus generation, never a StateStamp. */
export interface WitnessWire {
	readonly store: string
	readonly generation: bigint
}

export type ExpectedWire =
	| { readonly kind: "any" }
	| { readonly kind: "exact"; readonly store: string; readonly generation: bigint }

export type ApplyOutcomeWire =
	| { readonly tag: "accepted"; readonly witness: WitnessWire }
	| { readonly tag: "no-change"; readonly witness: WitnessWire }
	| { readonly tag: "invariant-rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "moved"; readonly witnessed: WitnessWire; readonly current: WitnessWire }

/** Bounded database diagnostics: measurements, never retained row payloads. */
export interface DbInspectionWire {
	readonly generation: bigint
	readonly storage: import("#db.ts").StorageInspection
	readonly retainedOperations: bigint
}

export interface SnapshotWire {
	readonly snapshot: SnapshotHandle
	readonly witness: WitnessWire
}

export interface ChangesWire {
	readonly changes: ChangesHandle
	readonly fingerprint: string
}

export interface MutationReportWire {
	readonly submitted: bigint
	readonly changed: bigint
}

interface DbBridge {
	/** Schema admission/compilation; take yields detached descriptor data. */
	runtimeSchemaCompile(runtime: RuntimeHandle, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeSchemaTake(operation: OperationHandle): SealedDescriptor

	/** Coherent owned snapshot acquisition off a managed database. */
	runtimeDbSnapshot(db: DbHandle, callback: () => void): OperationHandle
	runtimeSnapshotTake(operation: OperationHandle): SnapshotWire
	runtimeSnapshotClose(snapshot: SnapshotHandle, callback: (report: CloseWire) => void): void

	/** One compiled query, with an independent share of the snapshot pin. */
	runtimeSnapshotPrepare(snapshot: SnapshotHandle, query: ParsedQuery, callback: () => void): OperationHandle
	runtimePreparedTake(operation: OperationHandle): PreparedHandle
	runtimePreparedClose(prepared: PreparedHandle, callback: (report: CloseWire) => void): void
	runtimePreparedReleaseMemory(prepared: PreparedHandle, callback: () => void): OperationHandle

	/** Exact-key point read; take yields one owned row or null (absent). */
	runtimeSnapshotGet(
		snapshot: SnapshotHandle,
		relationId: number,
		keyStatementId: number,
		keyCells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeRowTake(operation: OperationHandle): readonly CellValue[] | null

	/**
	 * Complete execution. One-shot preparation drops before publication;
	 * explicit preparation reuses its compiled plan and scratch. The result is
	 * sealed and independent only after ALL evaluation succeeded.
	 */
	runtimeSnapshotExecute(
		snapshot: SnapshotHandle,
		query: ParsedQuery,
		params: readonly QueryParam[],
		callback: () => void
	): OperationHandle
	runtimePreparedExecute(prepared: PreparedHandle, params: readonly QueryParam[], callback: () => void): OperationHandle
	runtimeResultTake(operation: OperationHandle): ResultHandle
	runtimeResultClose(result: ResultHandle, callback: (report: CloseWire) => void): void

	/**
	 * Explicit full materialization into final native row arrays.
	 * Failure leaves the sealed backing available.
	 */
	runtimeResultCollect(result: ResultHandle, callback: () => void): OperationHandle
	runtimeRowsTake(operation: OperationHandle): readonly (readonly CellValue[])[]

	/**
	 * Atomic spend: moves the completed result's backing storage into one
	 * private cursor. A second transfer, or transfer racing collect,
	 * refuses (SpentHandle) before touching the backing.
	 */
	runtimeResultCursor(result: ResultHandle, callback: () => void): OperationHandle
	runtimeCursorTake(operation: OperationHandle): CursorHandle
	/** One bounded delivery batch; null is EOF. Execution is already complete. */
	runtimeCursorNext(cursor: CursorHandle, callback: () => void): OperationHandle
	runtimePageTake(operation: OperationHandle): readonly (readonly CellValue[])[] | null
	runtimeCursorClose(cursor: CursorHandle, callback: (report: CloseWire) => void): void

	/** Database-free draft acquisition (schema compiled/checked on the executor). */
	runtimeDraftOpen(runtime: RuntimeHandle, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeDraftTake(operation: OperationHandle): DraftHandle
	/**
	 * One bounded ingestion chunk (rows × arity cells, row-major, sealed
	 * field order). Rows are owned by the draft until finish. Failure spends
	 * the draft and starts tracked
	 * drain natively.
	 */
	runtimeDraftInsert(
		draft: DraftHandle,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeDraftDelete(
		draft: DraftHandle,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeReportTake(operation: OperationHandle): MutationReportWire
	/** Consumes the draft into an immutable schema-bound ChangeSet (one command, add-wins normalization). */
	runtimeDraftFinish(draft: DraftHandle, callback: () => void): OperationHandle
	runtimeChangesTake(operation: OperationHandle): ChangesWire
	runtimeDraftClose(draft: DraftHandle, callback: (report: CloseWire) => void): void
	runtimeChangesClose(changes: ChangesHandle, callback: (report: CloseWire) => void): void

	/** One immutable final-state admission/commit under the managed owner. */
	runtimeDbApply(db: DbHandle, changes: ChangesHandle, expected: ExpectedWire, callback: () => void): OperationHandle
	runtimeApplyTake(operation: OperationHandle): ApplyOutcomeWire

	runtimeDbInspect(db: DbHandle, callback: () => void): OperationHandle
	runtimeDbClearCache(db: DbHandle, callback: () => void): OperationHandle
	runtimeDbInspectTake(operation: OperationHandle): DbInspectionWire

	/** Shared canonical row codec (also the log/migration encoding). */
	runtimeEncodeRows(
		runtime: RuntimeHandle,
		spec: SchemaSpec,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeBytesTake(operation: OperationHandle): Uint8Array
	runtimeDecodeRows(
		runtime: RuntimeHandle,
		spec: SchemaSpec,
		relationId: number,
		bytes: Uint8Array,
		callback: () => void
	): OperationHandle

	/**
	 * Read-only migration-codec integration over native `schema_file` and
	 * `migration::{plan, manifest}` through the shared executor: bounded
	 * owned input, bounded owned JSON response bytes, one registered
	 * cancellable operation. Neither verb opens, initializes, freezes or
	 * migrates a database.
	 */
	runtimeMigrationSchema(runtime: RuntimeHandle, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeMigrationRead(runtime: RuntimeHandle, request: Uint8Array, callback: () => void): OperationHandle
}

// The fresh-addon roster test pins this private declaration exactly as it
// pins #runtime-native.ts's; the two casts re-type the SAME single binding.
export const dbNative = native as typeof native & DbBridge
