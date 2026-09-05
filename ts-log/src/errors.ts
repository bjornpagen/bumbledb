/**
 * The driver's typed Effect error identities. Tagged classes carry their
 * structured causes directly; no shared sentinel or side-table is needed.
 * Causes may also be read back with the `*Of` accessors —
 * never by message-string matching. There is deliberately no
 * `ErrAlreadyApplied`: the state it would name is absorbed by idempotent
 * replay (L10) and never surfaces. ManifestMissing, OverWidth,
 * RefillNeeded, Exhausted, and SlotRetired are named sums of their own: a
 * replica without a manifest, a draw past the lease width, a cached id
 * block too short for the draw, the u64 ceiling, and a below-floor
 * create. Exhaustion means the id space is spent — a refill is never an
 * exhaustion. An unproved conditional write is the store verbs' own
 * `ambiguous` outcome arm, never an error identity.
 */

import * as Data from "effect/Data"
import type { Braid } from "#descriptor.ts"
import type { Generation, StoreKey } from "#keys.ts"

/**
 * Typed refusal of a protocol object before any apply: batch shape,
 * version, fingerprint, manifest shape, checkpoint braid-set drift —
 * one identity, cause data per site.
 */
// Explicit constructor types keep declaration emit on Effect's public exports
// even when a locally linked core makes the compiler deduplicate peer copies.
const RefusedBase: ReturnType<typeof Data.TaggedError<"LogRefused">> = Data.TaggedError("LogRefused")
class ErrRefused extends RefusedBase<{
	readonly message: string
	readonly reason: RefusalCause | null
}> {}

/** `commit` recorded ops in more than one braid; `commitSplit` is the verb. */
const ErrSpanningCommitBase: ReturnType<typeof Data.TaggedError<"LogSpanningCommit">> =
	Data.TaggedError("LogSpanningCommit")
class ErrSpanningCommit extends ErrSpanningCommitBase<{ readonly message: string }> {}

/** A rejected replay on a store that passed the wholeness check. */
const ErrReplayDivergedBase: ReturnType<typeof Data.TaggedError<"LogReplayDiverged">> =
	Data.TaggedError("LogReplayDiverged")
class ErrReplayDiverged extends ErrReplayDivergedBase<{ readonly message: string }> {}

/** The chain discipline, one identity with three proved causes. */
const ErrChainMismatchBase: ReturnType<typeof Data.TaggedError<"LogChainMismatch">> =
	Data.TaggedError("LogChainMismatch")
class ErrChainMismatch extends ErrChainMismatchBase<{
	readonly message: string
	readonly detail: ChainMismatchData
}> {}

/** Bounded live-tip losses exhausted — an operational signal, not an outcome arm. */
const ErrContentionBase: ReturnType<typeof Data.TaggedError<"LogContention">> = Data.TaggedError("LogContention")
class ErrContention extends ErrContentionBase<{
	readonly message: string
	readonly detail: ContentionData
}> {}

/** A replica (or any read-role handle) found no manifest; only the writer births a store. */
const ErrManifestMissingBase: ReturnType<typeof Data.TaggedError<"LogManifestMissing">> =
	Data.TaggedError("LogManifestMissing")
export class ErrManifestMissing extends ErrManifestMissingBase<{ readonly message: string }> {}

/** A single id draw larger than one lease width; the width is the one block size. */
const ErrOverWidthBase: ReturnType<typeof Data.TaggedError<"LogOverWidth">> = Data.TaggedError("LogOverWidth")
class ErrOverWidth extends ErrOverWidthBase<{
	readonly message: string
	readonly detail: OverWidthData
}> {}

/**
 * The cached id block is too short for the draw — the writer's signal
 * to lease a fresh block. A refill is not an exhaustion: the id space
 * still has room; only the cache is short.
 */
const ErrRefillNeededBase: ReturnType<typeof Data.TaggedError<"LogRefillNeeded">> = Data.TaggedError("LogRefillNeeded")
class ErrRefillNeeded extends ErrRefillNeededBase<{
	readonly message: string
	readonly detail: RefillNeededData
}> {}

/** The next lease would leave u64 — the id space is spent. */
const ErrExhaustedBase: ReturnType<typeof Data.TaggedError<"LogExhausted">> = Data.TaggedError("LogExhausted")
class ErrExhausted extends ErrExhaustedBase<{
	readonly message: string
	readonly detail: ExhaustedData
}> {}

/** A put_create at a slot below the published checkpoint vector. */
const ErrSlotRetiredBase: ReturnType<typeof Data.TaggedError<"LogSlotRetired">> = Data.TaggedError("LogSlotRetired")
class ErrSlotRetired extends ErrSlotRetiredBase<{
	readonly message: string
	readonly detail: SlotRetiredData
}> {}

/** The vendor channel: I/O and store infrastructure failures. */
const ErrStoreBase: ReturnType<typeof Data.TaggedError<"LogStoreFailure">> = Data.TaggedError("LogStoreFailure")
class ErrStore extends ErrStoreBase<{ readonly message: string; readonly cause: unknown }> {}

/** Invalid caller-supplied protocol descriptions, before native admission. */
const LogInputErrorBase: ReturnType<typeof Data.TaggedError<"LogInputError">> = Data.TaggedError("LogInputError")
export class LogInputError extends LogInputErrorBase<{ readonly message: string }> {}
/** A contextual local operation failure; preserves the original thrown value. */
const LogOperationErrorBase: ReturnType<typeof Data.TaggedError<"LogOperationError">> =
	Data.TaggedError("LogOperationError")
export class LogOperationError extends LogOperationErrorBase<{
	readonly message: string
	readonly cause: unknown
}> {}

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
	| { readonly kind: "NonCanonicalF64" }
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
	// `LeaseRefusal` arms with tagged errors of their own.
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

function refuse(cause: RefusalCause, detail: string): never {
	throw new ErrRefused({ reason: cause, message: detail })
}

function refuseChain(data: ChainMismatchData, detail: string): never {
	throw new ErrChainMismatch({ detail: data, message: detail })
}

function throwContention(data: ContentionData, detail: string): never {
	throw new ErrContention({ detail: data, message: detail })
}

function refuseManifestMissing(detail: string): never {
	throw new ErrManifestMissing({ message: detail })
}

function isManifestMissing(error: unknown): boolean {
	return error instanceof ErrManifestMissing
}

function refuseOverWidth(data: OverWidthData, detail: string): never {
	throw new ErrOverWidth({ detail: data, message: detail })
}

function throwRefillNeeded(data: RefillNeededData, detail: string): never {
	throw new ErrRefillNeeded({ detail: data, message: detail })
}

function refuseExhausted(data: ExhaustedData, detail: string): never {
	throw new ErrExhausted({ detail: data, message: detail })
}

function refuseSlotRetired(data: SlotRetiredData, detail: string): never {
	throw new ErrSlotRetired({ detail: data, message: detail })
}

function refusalOf(error: unknown): RefusalCause | undefined {
	return error instanceof ErrRefused ? (error.reason ?? undefined) : undefined
}

function chainMismatchOf(error: unknown): ChainMismatchData | undefined {
	return error instanceof ErrChainMismatch ? error.detail : undefined
}

function contentionOf(error: unknown): ContentionData | undefined {
	return error instanceof ErrContention ? error.detail : undefined
}

function overWidthOf(error: unknown): OverWidthData | undefined {
	return error instanceof ErrOverWidth ? error.detail : undefined
}

function refillNeededOf(error: unknown): RefillNeededData | undefined {
	return error instanceof ErrRefillNeeded ? error.detail : undefined
}

function exhaustedOf(error: unknown): ExhaustedData | undefined {
	return error instanceof ErrExhausted ? error.detail : undefined
}

function slotRetiredOf(error: unknown): SlotRetiredData | undefined {
	return error instanceof ErrSlotRetired ? error.detail : undefined
}

/** Store failures retain provider metadata in cause, without string-based classification. */
function wrapStore(inner: unknown, detail: string): ErrStore {
	return new ErrStore({ cause: inner, message: `${detail}: ${inner instanceof Error ? inner.message : String(inner)}` })
}

export type {
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
	chainMismatchOf,
	contentionOf,
	ErrChainMismatch,
	ErrContention,
	ErrExhausted,
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
	throwContention,
	throwRefillNeeded,
	wrapStore
}
