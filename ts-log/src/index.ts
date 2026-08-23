/**
 * @bjornpagen/bumbledb-log — braided object-store replication for
 * bumbledb, a thin peer of the engine SDK. Public surface: the pure
 * protocol trio (`encodeBatch`/`decodeBatch`, `footprintOf`,
 * `braidsOf`) mirrored byte-exactly against the Rust driver, the
 * conflict-algebra intersection, the five-verb object store with its
 * tier-1 `fsStore`, and `openReplica`/`openWriter`/`openTenants`
 * composed from the engine SDK's own verbs — the replica hands out the
 * SDK's `Db`, and no engine surface is duplicated.
 */

export type { Braid } from "#braids.ts"
export { braidsOf, serialAtStatementsOf } from "#braids.ts"
export type { BatchHeader, ChainPosition, DecodedBatch } from "#codec.ts"
export { decodeBatch, encodeBatch, footprintSectionsEqual, verifyChain } from "#codec.ts"
export type { LogDescriptor, LogTheory, SerialStatement } from "#descriptor.ts"
export { descriptorOf } from "#descriptor.ts"
export type { ChainCause, ChainMismatchData, ContentionCause, ContentionData, RefusalCause } from "#errors.ts"
export {
	chainMismatchOf,
	contentionOf,
	ErrChainMismatch,
	ErrContention,
	ErrFootprintMismatch,
	ErrGapDetected,
	ErrRefused,
	ErrReplayDiverged,
	ErrSpanningCommit,
	ErrStore,
	refusalOf
} from "#errors.ts"
export type { BatchOp, CapacityInterval, FootprintEntry } from "#footprint.ts"
export { capacityIntervalsOf, footprintOf } from "#footprint.ts"
export type { CapacitySlack, Intersection, SharedCapacityParent, SharedKey } from "#intersect.ts"
export { capacityCommutes, intersectionOf } from "#intersect.ts"
export type { OpenReplicaOptions, Replica } from "#replica.ts"
export { openReplica } from "#replica.ts"
export type { Create, Fetched, ObjectStore, Poll, Swap } from "#store.ts"
export { fsStore } from "#store.ts"
export type { OpenTenantsOptions, Tenants } from "#tenants.ts"
export { openTenants } from "#tenants.ts"
export type { LogInterval, LogValue } from "#value.ts"
export type { BraidOutcome, Commit, CommitSplit, Durability, LogBatch, Writer } from "#writer.ts"
export { openWriter } from "#writer.ts"
