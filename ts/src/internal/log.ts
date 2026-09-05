/**
 * @bjornpagen/bumbledb/internal/log — the exact-version cross-package seam
 * consumed by `@bjornpagen/bumbledb-log`. Unsupported application surface;
 * not a security boundary. Native kind/runtime/owner/state validation remains
 * mandatory on every verb. Real types are shipped; this is not stripInternal.
 */
export type { ChangeDraft, ChangeSet } from "#changes.ts"
export { ChangeSet, internalChanges } from "#changes.ts"
export type { ExecutionSession, QueryReader } from "#db.ts"
export { Db, internalPublishedReader } from "#db.ts"
export type { CompiledSchema, SchemaId } from "#compile.ts"
export { Schema } from "#compile.ts"
export { lower } from "#lower.ts"
export { internalMigrationRead, internalMigrationSchema } from "#migration.ts"
export {
	internalBlake3,
	internalDescriptor,
	internalLogIdentities,
	internalLogSchema,
	nativeBindingIsLoaded
} from "#native.ts"
export type { CompleteResult } from "#result.ts"
export type { ExecutionPolicy, NativeRuntimeOptions, RepositoryLock } from "#runtime.ts"
export {
	deliveryResultBytes,
	finalizeClose,
	hashChunk,
	internalAcquireRepositoryLock,
	NativeRuntime,
	nativeOperation,
	nativeOperationWith,
	policyWire,
	runtimeHandle
} from "#runtime.ts"
export type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
export { CloseFailure, DbError, dbError, runtimeErrorCodes } from "#runtime-errors.ts"
export type {
	Capability,
	CloseDrain,
	CloseWire,
	DirectoryHandle,
	InspectionWire,
	NativeKind,
	RepositoryLockHandle,
	OperationHandle,
	PolicyWire,
	ResourceHeader,
	RuntimeHandle
} from "#runtime-native.ts"
export { runtimeNative } from "#runtime-native.ts"
