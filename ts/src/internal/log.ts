/**
 * @bjornpagen/bumbledb/internal/log — the exact-version cross-package seam
 * consumed by `@bjornpagen/bumbledb-log`. Unsupported application surface;
 * not a security boundary. Native kind/runtime/owner/state validation remains
 * mandatory on every verb. Real types are shipped; this is not stripInternal.
 */
export type { ChangeDraft } from "#changes.ts"
export { ChangeSet, internalChanges } from "#changes.ts"
export type { CompiledSchema, SchemaId } from "#compile.ts"
export { Schema } from "#compile.ts"
export type { PreparedQuery, QueryReader } from "#db.ts"
export { Db, internalPublishedReader } from "#db.ts"
export type { SnapshotHandle } from "#db-native.ts"
export { lower } from "#lower.ts"
export {
	internalBlake3,
	internalDescriptor,
	internalLogIdentities,
	internalLogSchema,
	nativeBindingIsLoaded
} from "#native.ts"
export type { CompleteResult } from "#result.ts"
export { factCellsOf } from "#rows.ts"
export type { NativeRuntimeOptions } from "#runtime.ts"
export {
	finalizeClose,
	hashChunk,
	NativeRuntime,
	nativeOperation,
	nativeOperationWith,
	runtimeHandle
} from "#runtime.ts"
export type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
export { CloseFailure, DbError, dbError, runtimeErrorCodes } from "#runtime-errors.ts"
export type {
	Capability,
	CloseWire,
	DirectoryHandle,
	InspectionWire,
	NativeKind,
	OperationHandle,
	RuntimeHandle
} from "#runtime-native.ts"
export { runtimeNative } from "#runtime-native.ts"
export { schemasAgree } from "#schema.ts"
export { internalSchemaBindings, internalSchemaSnapshot } from "#schema-file.ts"
