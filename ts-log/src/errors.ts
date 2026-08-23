/**
 * The driver's error identities, on the SDK's `@superbuilders/errors`
 * idiom: exported sentinel values checked with `errors.is`, structured
 * causes carried as data properties read back with the `*Of` accessors —
 * never by message-string matching. There is deliberately no
 * `ErrAlreadyApplied`: the state it would name is absorbed by idempotent
 * replay (L10) and never surfaces.
 */

import * as errors from "@superbuilders/errors"

/**
 * Typed refusal of a protocol object before any apply: batch shape,
 * version, fingerprint, manifest shape, checkpoint braid-set drift —
 * one identity, cause data per site.
 */
const ErrRefused = errors.new("bumbledb-log refused")

/** `commit` recorded ops in more than one braid; `commitSplit` is the verb. */
const ErrSpanningCommit = errors.new(
	"bumbledb-log spanningCommit: the recorded ops span braids — commitSplit is the explicit verb"
)

/** 404 at or below the current checkpoint's vector: the tail was gc'd. */
const ErrGapDetected = errors.new("bumbledb-log gapDetected: the log tail below the checkpoint vector was collected")

/** A rejected replay on a store that passed the wholeness check. */
const ErrReplayDiverged = errors.new(
	"bumbledb-log replayDiverged: a published batch rejected during steady-state replay"
)

/** Recomputed footprint differs from the published section. */
const ErrFootprintMismatch = errors.new(
	"bumbledb-log footprintMismatch: the published footprint section is not the recomputation of the ops"
)

/** The chain discipline, one identity with three proved causes. */
const ErrChainMismatch = errors.new("bumbledb-log chainMismatch: the batch violates the braid's chain discipline")

/** Bounded live-tip losses exhausted — an operational signal, not an outcome arm. */
const ErrContention = errors.new("bumbledb-log contention: consecutive live-tip losses exhausted the bound")

/** The vendor channel: I/O and store infrastructure failures. */
const ErrStore = errors.new("bumbledb-log store failure")

type RefusalCause =
	| { readonly kind: "magic" }
	| { readonly kind: "version"; readonly version: number }
	| { readonly kind: "flags"; readonly flags: number }
	| { readonly kind: "fingerprint"; readonly carried: string; readonly expected: string }
	| { readonly kind: "braid-unknown"; readonly braid: number }
	| { readonly kind: "op-kind"; readonly opKind: number }
	| { readonly kind: "op-relation"; readonly relation: number; readonly braid: string }
	| { readonly kind: "row-shape"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "truncated"; readonly at: string }
	| { readonly kind: "trailing"; readonly bytes: number }
	| { readonly kind: "footprint-order"; readonly index: number }
	| { readonly kind: "footprint-shape"; readonly index: number }
	| { readonly kind: "manifest-shape" }
	| { readonly kind: "manifest-version"; readonly version: number }
	| { readonly kind: "checkpoint-shape" }
	| { readonly kind: "checkpoint-braids"; readonly carried: readonly string[]; readonly derived: readonly string[] }
	| { readonly kind: "checkpoint-digest"; readonly expected: string; readonly computed: string }
	| { readonly kind: "no-op-slot"; readonly braid: string; readonly slot: bigint; readonly writer: bigint }

type ChainCause = "prev" | "slot" | "timestamp"

interface ChainMismatchData {
	readonly cause: ChainCause
	readonly braid: string
	readonly slot: bigint
	readonly writer: bigint
}

type ContentionCause =
	| { readonly kind: "hot-key"; readonly statement: number; readonly determinants: readonly unknown[] }
	| { readonly kind: "slot-race"; readonly tip: bigint }

interface ContentionData {
	readonly braid: string
	readonly cause: ContentionCause
}

const refusalData = new WeakMap<Error, RefusalCause>()
const chainData = new WeakMap<Error, ChainMismatchData>()
const contentionData = new WeakMap<Error, ContentionData>()

function refuse(cause: RefusalCause, detail: string): never {
	const error = errors.wrap(ErrRefused, detail)
	refusalData.set(error, cause)
	throw error
}

function refuseChain(data: ChainMismatchData, detail: string): never {
	const error = errors.wrap(ErrChainMismatch, detail)
	chainData.set(error, data)
	throw error
}

function throwContention(data: ContentionData, detail: string): never {
	const error = errors.wrap(ErrContention, detail)
	contentionData.set(error, data)
	throw error
}

function refusalOf(error: Error): RefusalCause | undefined {
	return refusalData.get(error)
}

function chainMismatchOf(error: Error): ChainMismatchData | undefined {
	return chainData.get(error)
}

function contentionOf(error: Error): ContentionData | undefined {
	return contentionData.get(error)
}

const storeMarked = new WeakSet<Error>()

function wrapStore(inner: Error, detail: string): Error {
	const error = errors.wrap(inner, `${ErrStore.message}: ${detail}`)
	storeMarked.add(error)
	return error
}

function isStoreFailure(error: Error): boolean {
	let cursor: Error | undefined = error
	while (cursor !== undefined) {
		if (cursor === ErrStore || storeMarked.has(cursor)) {
			return true
		}
		cursor = cursor.cause instanceof Error ? cursor.cause : undefined
	}
	return false
}

export type { ChainCause, ChainMismatchData, ContentionCause, ContentionData, RefusalCause }
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
	isStoreFailure,
	refusalOf,
	refuse,
	refuseChain,
	throwContention,
	wrapStore
}
