/**
 * The driver's error identities, on the SDK's `@superbuilders/errors`
 * idiom: exported sentinel values checked with `errors.is`, structured
 * causes carried as data properties read back with the `*Of` accessors —
 * never by message-string matching. There is deliberately no
 * `ErrAlreadyApplied`: the state it would name is absorbed by idempotent
 * replay (L10) and never surfaces. ManifestMissing, Ambiguous, OverWidth,
 * Exhausted, and SlotRetired are named sums of their own: a replica
 * without a manifest, an unproved conditional write, a draw past the
 * lease width or the u64 ceiling, and a below-floor create.
 */

import * as errors from "@superbuilders/errors"
import type { Braid } from "#descriptor.ts"
import type { Generation, StoreKey } from "#keys.ts"

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

/** A replica (or any read-role handle) found no manifest; only the writer births a store. */
export const ErrManifestMissing = errors.new("bumbledb-log manifestMissing: the store has no manifest")

/**
 * A conditional write the transport cannot prove (S3 409, timeout, a
 * retried PUT). The machine GET-verifies; this identity is the unproved
 * arm, never a proved Exists or Moved.
 */
const ErrAmbiguous = errors.new("bumbledb-log ambiguous: the conditional write is unproved")

/** A single id draw larger than one lease width; the width is the one block size. */
const ErrOverWidth = errors.new("bumbledb-log overWidth: the draw exceeds the lease width")

/** The next lease would leave u64 — the id space is spent. */
const ErrExhausted = errors.new("bumbledb-log exhausted: the id space is spent")

/** A put_create at a slot below the published checkpoint vector. */
const ErrSlotRetired = errors.new("bumbledb-log slotRetired: the slot is retired")

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
	| { readonly kind: "DigestWidth" }
	| { readonly kind: "Arity"; readonly op: number; readonly relation: string; readonly row: number }
	| { readonly kind: "Malformed"; readonly at: number }
	| { readonly kind: "Overflow" }
	| { readonly kind: "ManifestShape" }
	| { readonly kind: "ManifestVersion"; readonly version: number }
	| { readonly kind: "SidecarShape" }
	| { readonly kind: "SidecarVersion"; readonly version: number }
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

type AmbiguousVerb = "create" | "swap"

interface AmbiguousData {
	readonly verb: AmbiguousVerb
	readonly key: StoreKey
}

interface OverWidthData {
	readonly requested: bigint
}

interface ExhaustedData {
	readonly relation: number
	readonly field: number
}

interface SlotRetiredData {
	readonly braid: Braid
	readonly slot: Generation
}

/** The id-lease algebra's refusal arms: OverWidth | Exhausted. */
type LeaseRefusal =
	| { readonly kind: "OverWidth"; readonly requested: bigint }
	| { readonly kind: "Exhausted"; readonly relation: number; readonly field: number }

const refusalData = new WeakMap<Error, RefusalCause>()
const chainData = new WeakMap<Error, ChainMismatchData>()
const contentionData = new WeakMap<Error, ContentionData>()
const ambiguousData = new WeakMap<Error, AmbiguousData>()
const overWidthData = new WeakMap<Error, OverWidthData>()
const exhaustedData = new WeakMap<Error, ExhaustedData>()
const slotRetiredData = new WeakMap<Error, SlotRetiredData>()

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

function refuseManifestMissing(detail: string): never {
	throw errors.wrap(ErrManifestMissing, detail)
}

function throwAmbiguous(data: AmbiguousData, detail: string): never {
	const error = errors.wrap(ErrAmbiguous, detail)
	ambiguousData.set(error, data)
	throw error
}

function refuseOverWidth(data: OverWidthData, detail: string): never {
	const error = errors.wrap(ErrOverWidth, detail)
	overWidthData.set(error, data)
	throw error
}

function refuseExhausted(data: ExhaustedData, detail: string): never {
	const error = errors.wrap(ErrExhausted, detail)
	exhaustedData.set(error, data)
	throw error
}

function refuseSlotRetired(data: SlotRetiredData, detail: string): never {
	const error = errors.wrap(ErrSlotRetired, detail)
	slotRetiredData.set(error, data)
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

function ambiguousOf(error: Error): AmbiguousData | undefined {
	return ambiguousData.get(error)
}

function overWidthOf(error: Error): OverWidthData | undefined {
	return overWidthData.get(error)
}

function exhaustedOf(error: Error): ExhaustedData | undefined {
	return exhaustedData.get(error)
}

function slotRetiredOf(error: Error): SlotRetiredData | undefined {
	return slotRetiredData.get(error)
}

/** Every store failure wraps the exported sentinel itself, so
 *  `errors.is(e, ErrStore)` matches by identity; the vendor error's
 *  message rides the detail verbatim. */
function wrapStore(inner: Error, detail: string): Error {
	return errors.wrap(ErrStore, `${detail}: ${inner.message}`)
}

export type {
	AmbiguousData,
	AmbiguousVerb,
	ChainCause,
	ChainMismatchData,
	ContentionCause,
	ContentionData,
	ExhaustedData,
	LeaseRefusal,
	OverWidthData,
	RefusalCause,
	SlotRetiredData
}
export {
	ambiguousOf,
	chainMismatchOf,
	contentionOf,
	ErrAmbiguous,
	ErrChainMismatch,
	ErrContention,
	ErrExhausted,
	ErrGapDetected,
	ErrOverWidth,
	ErrRefused,
	ErrReplayDiverged,
	ErrSlotRetired,
	ErrSpanningCommit,
	ErrStore,
	exhaustedOf,
	overWidthOf,
	refusalOf,
	refuse,
	refuseChain,
	refuseExhausted,
	refuseManifestMissing,
	refuseOverWidth,
	refuseSlotRetired,
	slotRetiredOf,
	throwAmbiguous,
	throwContention,
	wrapStore
}
