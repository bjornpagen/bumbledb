/**
 * Private core database bridge roster — the C09/C05 CONSUMER-side pin for
 * the managed runtime verbs the chapter 35 core surface dispatches. Not a
 * public API: nothing here is exported from the package barrel.
 *
 * OWNERSHIP NOTE (recorded in implementation/packets/P07.md): the addon
 * implementation of every verb below is P06R's lane (`ts/crate/src/`,
 * worker-affine sessions per P06.md's reactor design) on top of P02's C04
 * owned snapshots and P03's C05 completed results. This file declares the
 * exact shapes P07 codes against, exactly as `#runtime-native.ts` pins the
 * landed runtime/directory/fs handshake. Every verb runs on the ONE bounded
 * executor (no AsyncTask/libuv bypass), registers under the runtime's
 * operation accounting, takes a `PolicyWire` converted once at admission,
 * and completes through the registered callback; `runtimeCancel` cancels
 * and joins any of them. Close verbs report the real drain outcome through
 * `CloseWire`.
 */
import type { DbHandle, ParsedQuery, QueryParam, SealedDescriptor, Violation } from "#native.ts"
import { native } from "#native.ts"
import type { CloseWire, OperationHandle, PolicyWire, RuntimeHandle } from "#runtime-native.ts"
import type { CellValue } from "#rows.ts"
import type { SchemaSpec } from "#spec.ts"

export interface SnapshotHandle {
	readonly __snapshot: unique symbol
}
export interface SessionHandle {
	readonly __session: unique symbol
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

export type ExpectedWire = { readonly kind: "any" } | { readonly kind: "exact"; readonly store: string; readonly generation: bigint }

export type ApplyOutcomeWire =
	| { readonly tag: "accepted"; readonly witness: WitnessWire }
	| { readonly tag: "no-change"; readonly witness: WitnessWire }
	| { readonly tag: "invariant-rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "moved"; readonly witnessed: WitnessWire; readonly current: WitnessWire }

/** Bounded database diagnostics: measurements, never retained row payloads. */
export interface DbInspectionWire {
	readonly generation: bigint
	readonly mapBytes: bigint
	readonly populatedBytes: bigint
	readonly diskBytes: bigint
	readonly residentEstimateBytes: bigint
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
	/** Charged schema admission/compilation; take yields detached descriptor data. */
	runtimeSchemaCompile(runtime: RuntimeHandle, policy: PolicyWire, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeSchemaTake(operation: OperationHandle): SealedDescriptor

	/** Coherent owned snapshot acquisition off a managed database. */
	runtimeDbSnapshot(db: DbHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeSnapshotTake(operation: OperationHandle): SnapshotWire
	runtimeSnapshotClose(snapshot: SnapshotHandle, callback: (report: CloseWire) => void): void

	/** Reusable snapshot-bound execution session (worker-affine natively). */
	runtimeSnapshotSession(snapshot: SnapshotHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeSessionTake(operation: OperationHandle): SessionHandle
	runtimeSessionClose(session: SessionHandle, callback: (report: CloseWire) => void): void

	/** Exact-key point read; take yields one owned row or null (absent). */
	runtimeSnapshotGet(
		snapshot: SnapshotHandle,
		policy: PolicyWire,
		relationId: number,
		keyStatementId: number,
		keyCells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeRowTake(operation: OperationHandle): readonly CellValue[] | null

	/**
	 * Complete bounded execution. The snapshot variant owns an internal
	 * one-shot session and closes it before publishing the result; the
	 * session variant reuses the caller's session. Either way the result is
	 * sealed and independent only after ALL evaluation succeeded (C05).
	 */
	runtimeSnapshotExecute(
		snapshot: SnapshotHandle,
		policy: PolicyWire,
		query: ParsedQuery,
		params: readonly QueryParam[],
		callback: () => void
	): OperationHandle
	runtimeSessionExecute(
		session: SessionHandle,
		policy: PolicyWire,
		query: ParsedQuery,
		params: readonly QueryParam[],
		callback: () => void
	): OperationHandle
	runtimeResultTake(operation: OperationHandle): ResultHandle
	runtimeResultClose(result: ResultHandle, callback: (report: CloseWire) => void): void

	/**
	 * Bounded total materialization: refuses (ResourceLimit) BEFORE
	 * allocating past `policy.resultBytes`; a cap failure leaves the sealed
	 * backing available. Take yields owned row arrays.
	 */
	runtimeResultCollect(result: ResultHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeRowsTake(operation: OperationHandle): readonly (readonly CellValue[])[]

	/**
	 * Atomic spend: moves the completed result's backing storage into one
	 * private cursor. A second transfer, or transfer racing collect,
	 * refuses (SpentHandle) before touching the backing.
	 */
	runtimeResultCursor(result: ResultHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeCursorTake(operation: OperationHandle): CursorHandle
	/** One owned page bounded by `policy.resultBytes`; null is EOF (cursor storage reclaimed). */
	runtimeCursorNext(cursor: CursorHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimePageTake(operation: OperationHandle): readonly (readonly CellValue[])[] | null
	runtimeCursorClose(cursor: CursorHandle, callback: (report: CloseWire) => void): void

	/** Database-free draft acquisition (schema compiled/checked on the executor). */
	runtimeDraftOpen(runtime: RuntimeHandle, policy: PolicyWire, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeDraftTake(operation: OperationHandle): DraftHandle
	/**
	 * One bounded ingestion chunk (rows × arity cells, row-major, sealed
	 * field order). Chunks share the draft's cumulative aggregate budget —
	 * they never reset it. Failure spends the draft and starts tracked
	 * drain natively.
	 */
	runtimeDraftInsert(
		draft: DraftHandle,
		policy: PolicyWire,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeDraftDelete(
		draft: DraftHandle,
		policy: PolicyWire,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeReportTake(operation: OperationHandle): MutationReportWire
	/** Consumes the draft into an immutable schema-bound ChangeSet (one command, add-wins normalization). */
	runtimeDraftFinish(draft: DraftHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeChangesTake(operation: OperationHandle): ChangesWire
	runtimeDraftClose(draft: DraftHandle, callback: (report: CloseWire) => void): void
	runtimeChangesClose(changes: ChangesHandle, callback: (report: CloseWire) => void): void

	/** One immutable final-state admission/commit under the managed owner. */
	runtimeDbApply(
		db: DbHandle,
		policy: PolicyWire,
		changes: ChangesHandle,
		expected: ExpectedWire,
		callback: () => void
	): OperationHandle
	runtimeApplyTake(operation: OperationHandle): ApplyOutcomeWire

	runtimeDbInspect(db: DbHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeDbInspectTake(operation: OperationHandle): DbInspectionWire

	/** Shared canonical row codec (also the log/migration encoding). */
	runtimeEncodeRows(
		runtime: RuntimeHandle,
		policy: PolicyWire,
		spec: SchemaSpec,
		relationId: number,
		rows: bigint,
		cells: readonly CellValue[],
		callback: () => void
	): OperationHandle
	runtimeBytesTake(operation: OperationHandle): Uint8Array
	runtimeDecodeRows(
		runtime: RuntimeHandle,
		policy: PolicyWire,
		spec: SchemaSpec,
		relationId: number,
		bytes: Uint8Array,
		callback: () => void
	): OperationHandle

	/**
	 * Read-only migration-codec integration (C11; P09's native
	 * `schema_file::{schema_id, render}` / `migration::{plan, manifest}`
	 * lanes reached through the P06 executor). `hashChunk`-shaped: bounded
	 * owned input, bounded owned JSON response bytes, one registered
	 * cancellable operation. Neither verb opens, initializes, freezes or
	 * migrates a database.
	 */
	runtimeMigrationSchema(runtime: RuntimeHandle, policy: PolicyWire, spec: SchemaSpec, callback: () => void): OperationHandle
	runtimeMigrationRead(runtime: RuntimeHandle, policy: PolicyWire, request: Uint8Array, callback: () => void): OperationHandle
}

// The fresh-addon roster test pins this private declaration exactly as it
// pins #runtime-native.ts's; the two casts re-type the SAME single binding.
export const dbNative = native as typeof native & DbBridge
