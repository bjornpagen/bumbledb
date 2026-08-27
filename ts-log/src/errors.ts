/**
 * The driver's error identities, on the SDK's `@superbuilders/errors`
 * idiom: exported sentinel values checked with `errors.is`, structured
 * causes carried as data properties read back with the `*Of` accessors —
 * never by message-string matching. There is deliberately no
 * `ErrAlreadyApplied`: the state it would name is absorbed by idempotent
 * replay (L10) and never surfaces. ManifestMissing, Ambiguous, OverWidth,
 * RefillNeeded, Exhausted, and SlotRetired are named sums of their own: a
 * replica without a manifest, an unproved conditional write, a draw past
 * the lease width, a cached id block too short for the draw, the u64
 * ceiling, and a below-floor create. Exhaustion means the id space is
 * spent — a refill is never an exhaustion.
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

/**
 * The cached id block is too short for the draw — the writer's signal
 * to lease a fresh block. A refill is not an exhaustion: the id space
 * still has room; only the cache is short.
 */
const ErrRefillNeeded = errors.new("bumbledb-log refillNeeded: the cached id block cannot cover the draw")

/** The next lease would leave u64 — the id space is spent. */
const ErrExhausted = errors.new("bumbledb-log exhausted: the id space is spent")

/** A put_create at a slot below the published checkpoint vector. */
const ErrSlotRetired = errors.new("bumbledb-log slotRetired: the slot is retired")

/** The vendor channel: I/O and store infrastructure failures. */
const ErrStore = errors.new("bumbledb-log store failure")

/**
 * The typed causes behind `ErrRefused`, one arm per identity. Every
 * `kind` is either a row of the generated identity table
 * (`crates/bumbledb-log/conformance/v3/identities.json`, spelled by the
 * core's one speller per family) or one of the three host-side kinds at
 * the tail, which cross no bridge. The boundary carries
 * `{ kind, message }` only, so a bridge-minted cause is the bare kind —
 * the detail rides the message — and a payload field exists only where
 * the minting seat owns the datum: a held document's version byte
 * (`version`, byte 0 of every v:3 document) and length (`at`), the
 * codec seat's own arity coordinates, the manifest audit's
 * fingerprints, the counter object's key. The raw id of an
 * `UnknownBraid` rides the message.
 */
type RefusalCause =
	// The batch grammar's rows (`batchDecode` | `batchEncode`), bridge
	// order. `Version` carries the version byte where a document seat
	// holds the bytes; `FingerprintMismatch` carries both digests where
	// the replica's manifest audit owns them; `Arity` carries its
	// coordinates where the codec seat's pre-bridge cell gate owns them.
	| { readonly kind: "Truncated" }
	| { readonly kind: "BadMagic" }
	| { readonly kind: "Version"; readonly version?: number }
	| { readonly kind: "Flags" }
	| { readonly kind: "FingerprintMismatch"; readonly carried?: string; readonly expected?: string }
	| { readonly kind: "UnknownBraid" }
	| { readonly kind: "UnknownOpKind" }
	| { readonly kind: "UnknownRelation" }
	| { readonly kind: "ClosedRelation" }
	| { readonly kind: "OpRelationOutsideBraid" }
	| { readonly kind: "TagMismatch" }
	| { readonly kind: "BoolByte" }
	| { readonly kind: "InvalidUtf8" }
	| { readonly kind: "EmptyInterval" }
	| { readonly kind: "IntervalOverflow" }
	| { readonly kind: "TrailingBytes" }
	| { readonly kind: "Arity"; readonly op?: number; readonly relation?: string; readonly row?: number }
	| { readonly kind: "Value" }
	| { readonly kind: "TooManyOps" }
	| { readonly kind: "TooManyRows" }
	// The document grammars' own rows (`manifest` | `checkpoint` |
	// `sidecar`): the seat holds the refused bytes, so `Malformed`
	// carries the document's length.
	| { readonly kind: "Malformed"; readonly at: number }
	| { readonly kind: "Overflow" }
	| { readonly kind: "BraidSet" }
	// The id-lease counter's row (`counter`): the writer's parse gate
	// owns the counter object's key. `Exhausted` and `OverWidth` are
	// `LeaseRefusal` arms with sentinels of their own.
	| { readonly kind: "Counter"; readonly key: StoreKey }
	// Host-side kinds with no table row: `DigestWidth` refuses a short
	// digest at the seat before a value crosses corrupted (the corpus
	// pins the name); `CheckpointDigest` and `NoOpSlot` are the replica
	// machine's own refusals — the machines keep two executors and no
	// bridge family.
	| { readonly kind: "DigestWidth" }
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

interface RefillNeededData {
	readonly relation: number
	readonly field: number
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

/** The id-lease algebra's refusal sum — the identity table's `counter`
 *  family, arm for arm: Counter | Exhausted | OverWidth. `Counter`
 *  carries the counter object's key, the datum this driver's parse gate
 *  owns (the core's twin carries relation and field). A cache refill is
 *  no arm: the shared algebra has none. */
type LeaseRefusal =
	| { readonly kind: "Counter"; readonly key: StoreKey }
	| { readonly kind: "Exhausted"; readonly relation: number; readonly field: number }
	| { readonly kind: "OverWidth"; readonly requested: bigint }

const refusalData = new WeakMap<Error, RefusalCause>()
const chainData = new WeakMap<Error, ChainMismatchData>()
const contentionData = new WeakMap<Error, ContentionData>()
const ambiguousData = new WeakMap<Error, AmbiguousData>()
const overWidthData = new WeakMap<Error, OverWidthData>()
const refillNeededData = new WeakMap<Error, RefillNeededData>()
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

function isManifestMissing(error: unknown): boolean {
	return error instanceof Error && errors.is(error, ErrManifestMissing)
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

function throwRefillNeeded(data: RefillNeededData, detail: string): never {
	const error = errors.wrap(ErrRefillNeeded, detail)
	refillNeededData.set(error, data)
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

function refillNeededOf(error: Error): RefillNeededData | undefined {
	return refillNeededData.get(error)
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
	RefillNeededData,
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
	ErrRefillNeeded,
	ErrRefused,
	ErrReplayDiverged,
	ErrSlotRetired,
	ErrSpanningCommit,
	ErrStore,
	exhaustedOf,
	isManifestMissing,
	overWidthOf,
	refillNeededOf,
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
	throwRefillNeeded,
	wrapStore
}
