/**
 * @bjornpagen/bumbledb-log — braided object-store replication for
 * bumbledb, a thin peer of the engine SDK. Public surface: the pure
 * protocol pair (`encodeBatch`/`decodeBatch`, `braidsOf`) mirrored
 * byte-exactly against the Rust driver, the five-verb object store
 * with its tier-1 `fsStore`, and `openReplica`/`openWriter`/
 * `openTenants` composed from the engine SDK's own verbs — the replica
 * hands out the SDK's `Db`, and no engine surface is duplicated.
 */

export type { Braid } from "#braids.ts"
export { braidsOf, serialAtStatementsOf } from "#braids.ts"
export type { BatchHeader, ChainEntry, DecodedBatch, Op } from "#codec.ts"
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
	ErrRefused,
	ErrReplayDiverged,
	ErrSpanningCommit,
	ErrStore,
	refusalOf
} from "#errors.ts"
export type { OpenReplicaOptions, Replica } from "#replica.ts"
export { openReplica } from "#replica.ts"
export type { Create, Fetched, ObjectStore, Poll, Swap } from "#store.ts"
export { fsStore } from "#store.ts"
export type { OpenTenantsOptions, Tenants } from "#tenants.ts"
export { openTenants } from "#tenants.ts"
export type { Interval, Value } from "#value.ts"
export type { Batch, BraidOutcome, Commit, CommitSplit, Durability, Writer } from "#writer.ts"
export { openWriter } from "#writer.ts"
