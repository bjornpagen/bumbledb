/**
 * The driver's error identities, on the SDK's `@superbuilders/errors`
 * idiom: exported sentinel values checked with `errors.is`, structured
 * causes carried as data properties read back with the `*Of` accessors —
 * never by message-string matching. There is deliberately no
 * `ErrAlreadyApplied`: the state it would name is absorbed by idempotent
 * replay (L10) and never surfaces.
 */

import * as errors from "@superbuilders/errors"
import type { Braid } from "#descriptor.ts"
import type { Generation } from "#keys.ts"

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

/** The chain discipline, one identity with three proved causes. */
const ErrChainMismatch = errors.new("bumbledb-log chainMismatch: the batch violates the braid's chain discipline")

/** Bounded live-tip losses exhausted — an operational signal, not an outcome arm. */
const ErrContention = errors.new("bumbledb-log contention: consecutive live-tip losses exhausted the bound")

/** The vendor channel: I/O and store infrastructure failures. */
const ErrStore = errors.new("bumbledb-log store failure")

/**
 * Decode refusal kinds carry the cross-implementation identity names the
 * Rust driver's `DecodeError::identity` pins — the conformance corpus
 * compares them string for string. The tail kinds are this driver's own
 * replica-boundary refusals.
 */
type RefusalCause =
	| { readonly kind: "Truncated"; readonly at: string }
	| { readonly kind: "BadMagic" }
	| { readonly kind: "Version"; readonly version: number }
	| { readonly kind: "Flags"; readonly flags: number }
	| { readonly kind: "FingerprintMismatch"; readonly carried: string; readonly expected: string }
	| { readonly kind: "UnknownBraid"; readonly braid: number }
	| { readonly kind: "UnknownOpKind"; readonly op: number; readonly opKind: number }
	| { readonly kind: "UnknownRelation"; readonly op: number; readonly relation: number }
	| { readonly kind: "ClosedRelation"; readonly op: number; readonly relation: number }
	| { readonly kind: "OpRelationOutsideBraid"; readonly op: number; readonly relation: number; readonly braid: Braid }
	| { readonly kind: "TagMismatch"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "BoolByte"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "InvalidUtf8"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "EmptyInterval"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "IntervalOverflow"; readonly relation: string; readonly row: number; readonly field: string }
	| { readonly kind: "TrailingBytes"; readonly bytes: number }
	| { readonly kind: "ManifestShape" }
	| { readonly kind: "ManifestVersion"; readonly version: number }
	| { readonly kind: "CheckpointShape" }
	| { readonly kind: "CheckpointBraids"; readonly carried: readonly string[]; readonly derived: readonly string[] }
	| { readonly kind: "CheckpointDigest"; readonly expected: string; readonly computed: string }
	| { readonly kind: "NoOpSlot"; readonly braid: Braid; readonly slot: Generation; readonly writer: bigint }

type ChainCause = "prev" | "slot" | "timestamp"

interface ChainMismatchData {
	readonly cause: ChainCause
	readonly braid: Braid
	readonly slot: Generation
	readonly writer: bigint
}

type ContentionCause =
	| {
			readonly kind: "hot-key"
			readonly statement: string
			readonly determinants: ReadonlyArray<Readonly<Record<string, unknown>>>
	  }
	| { readonly kind: "slot-race"; readonly tip: Generation }

interface ContentionData {
	readonly braid: Braid
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

/** Every store failure wraps the exported sentinel itself, so
 *  `errors.is(e, ErrStore)` matches by identity; the vendor error's
 *  message rides the detail verbatim. */
function wrapStore(inner: Error, detail: string): Error {
	return errors.wrap(ErrStore, `${detail}: ${inner.message}`)
}

export type { ChainCause, ChainMismatchData, ContentionCause, ContentionData, RefusalCause }
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
	refusalOf,
	refuse,
	refuseChain,
	throwContention,
	wrapStore
}
