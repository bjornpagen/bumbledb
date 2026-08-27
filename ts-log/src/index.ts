/**
 * @bjornpagen/bumbledb-log — braided object-store replication for
 * bumbledb, a thin peer of the engine SDK. The protocol grammar has one
 * implementation, `crates/bumbledb-log`, reached through the engine
 * bridge; this package is typed payload construction, the replica and
 * writer machines, the five-verb object store with its tier-1 `fsStore`,
 * in-process `memStore`, and the `s3Store` AWS S3 client, and
 * `openReplica`/`openWriter`/`openTenants` composed from the engine
 * SDK's own verbs — the replica hands out the SDK's `Db`, and no engine
 * surface is duplicated. Engine types (`FactValue`, `IntervalValue`, …)
 * are the peer `@bjornpagen/bumbledb`'s own exports, never re-exported
 * here.
 */

export type { Braid } from "#braids.ts"
export { braid, braidsOf, serialAtStatementsOf } from "#braids.ts"
export type { BatchHeader, ChainEntry, DecodedBatch, EncodeHeader, Op } from "#codec.ts"
export { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
export type { Descriptor, SerialStatement, Theory } from "#descriptor.ts"
export { descriptorOf } from "#descriptor.ts"
export type { ChainCause, ChainMismatchData, ContentionCause, ContentionData, RefusalCause } from "#errors.ts"
export {
	chainMismatchOf,
	contentionOf,
	ErrChainMismatch,
	ErrContention,
	ErrGapDetected,
	ErrManifestMissing,
	ErrRefused,
	ErrReplayDiverged,
	ErrSpanningCommit,
	ErrStore,
	isManifestMissing,
	refusalOf
} from "#errors.ts"
export type { Generation, StoreKey } from "#keys.ts"
export { generation, storeKey } from "#keys.ts"
export type { OpenReplicaOptions, Replica, Waited } from "#replica.ts"
export { openReplica } from "#replica.ts"
export type { Create, Etag, Fetched, ObjectStore, Poll, S3Config, S3Credentials, Swap } from "#store.ts"
export { etag, fsStore, memStore, s3Store } from "#store.ts"
export type { OpenTenantsOptions, Tenants } from "#tenants.ts"
export { openTenants } from "#tenants.ts"
export type { CheckpointOrder } from "#vector.ts"
export { Overflow, Vector } from "#vector.ts"
export type {
	Batch,
	BraidOutcome,
	Commit,
	CommitReceipt,
	CommitSplit,
	Deposition,
	Durability,
	EmptyCommit,
	Landing,
	Writer
} from "#writer.ts"
export { openWriter } from "#writer.ts"
