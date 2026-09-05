/**
 * Publication certainty, receipts and admin outcomes as owned immutable
 * data. Every terminal arm is a durable decision in the log; errors ride
 * inside the certainty unions (`not-submitted`/`outcome-unknown`) rather
 * than replacing them. `SubmitOutcome`'s E is `never` — interruption and
 * finalizer defects stay in the fiber's `Cause` and are never rewritten to
 * `not-submitted`. Admin operations use the same three-way certainty with
 * their existing protocol operation identities; there is no new journal.
 */
import type { Violation } from "@bjornpagen/bumbledb"
import type { LogError } from "#errors.ts"
import type {
	CommandRef,
	DatabaseIdentity,
	DecisionStamp,
	OperationId,
	OperationRef,
	PlanSetDigest,
	ReceiptEpoch,
	RootId,
	StateStamp
} from "#identity.ts"
// The generated plan/manifest/contract DATA shapes are C11 joint property
// declared once in `#migrations/types.ts` (P10, mirroring P09's native
// codec); this module imports them instead of declaring a second roster.
import type { ActivationRef, GeneratedMigrations } from "#migrations/types.ts"
import type { HistoryBinding } from "#options.ts"

/** Bounded caller-declared scalar result metadata (core canonical scalars). */
export type CommandScalar = bigint | number | string | boolean | Uint8Array
export type CommandResult = Readonly<Record<string, CommandScalar>>

export interface ChangeSummary {
	readonly added: bigint
	readonly removed: bigint
}

export type TerminalOutcome =
	| { readonly kind: "committed"; readonly changed: ChangeSummary; readonly result: CommandResult }
	| { readonly kind: "no-change"; readonly result: CommandResult }
	| { readonly kind: "precondition-failed"; readonly expected: StateStamp; readonly observed: StateStamp }
	| { readonly kind: "invariant-rejected"; readonly violations: readonly Violation[] }

export interface TerminalReceipt {
	readonly command: CommandRef
	readonly decisionAt: DecisionStamp
	readonly stateAt: StateStamp
	readonly outcome: TerminalOutcome
}

/**
 * This invocation's materialization, not durable receipt content. A
 * resolved receipt does not prove the local cache reached that decision.
 */
export type LocalMaterializationHealth =
	| { readonly kind: "ready"; readonly at: DecisionStamp }
	| { readonly kind: "unavailable"; readonly error: LogError }

export type SubmitOutcome =
	| { readonly kind: "decided"; readonly receipt: TerminalReceipt; readonly localHealth: LocalMaterializationHealth }
	| { readonly kind: "not-submitted"; readonly command: CommandRef; readonly error: LogError }
	| { readonly kind: "outcome-unknown"; readonly command: CommandRef; readonly error: LogError }

export type ResolveOutcome =
	| { readonly kind: "found"; readonly receipt: TerminalReceipt }
	| { readonly kind: "not-recorded-at"; readonly decisionAt: DecisionStamp }
	| { readonly kind: "command-epoch-closed" }
	| { readonly kind: "receipt-expired-unknown" }

export type AccessMode = "active" | "frozen" | "deleted"

export interface ReceiptPolicyReport {
	readonly openEpoch: ReceiptEpoch
	readonly retiredThrough: bigint
}

/** The bounded history health snapshot (chapter 22 evidence list). */
export interface HistoryInspection {
	readonly identity: DatabaseIdentity
	readonly accessMode: AccessMode
	readonly headRevision: bigint
	readonly decision: DecisionStamp
	readonly state: StateStamp
	readonly receipts: ReceiptPolicyReport
	readonly tail: { readonly count: bigint; readonly bytes: bigint }
	readonly unknownCommands: { readonly count: bigint; readonly oldestMillis: number | null }
	readonly roots: { readonly count: number; readonly capacity: number }
	readonly gc: "idle" | "marking" | "sweeping"
	readonly lastMaintenanceError: string | null
	readonly accounted: { readonly diskBytes: bigint; readonly workingBytes: bigint }
	readonly operations: { readonly queued: bigint; readonly active: bigint }
}

export type CacheSlotState = "opening" | "ready" | "closing" | "faulted"

export interface CacheSlotReport {
	/** The fixed-width binding digest, never a raw tenant label. */
	readonly binding: string
	readonly state: CacheSlotState
	readonly borrows: number
	readonly diskBytes: bigint
}

export interface CacheInspection {
	readonly openCount: number
	readonly opening: number
	readonly budget: { readonly bytes: bigint; readonly maxOpen: number }
	readonly evictions: bigint
	readonly slots: readonly CacheSlotReport[]
}

// ── Admin certainty ────────────────────────────────────────────────────────

/**
 * completed = the reported transition/status is known (possibly paused);
 * not-started = this invocation performed no authoritative mutation;
 * outcome-unknown = uncertainty remains until status resolves it. The ref
 * is the operation's existing protocol identity, derived before dispatch.
 */
export type AdminOutcome<Value> =
	| { readonly kind: "completed"; readonly ref: OperationRef; readonly value: Value }
	| { readonly kind: "not-started"; readonly ref: OperationRef; readonly error: LogError }
	| { readonly kind: "outcome-unknown"; readonly ref: OperationRef; readonly error: LogError }

export interface CheckpointReport {
	readonly at: DecisionStamp
	readonly state: StateStamp
	readonly root: RootId
}

export interface RestorePointReport {
	readonly root: RootId
	readonly at: DecisionStamp
	readonly state: StateStamp
}

export interface RootReleaseReport {
	readonly root: RootId
	/** The lost recovery capability, reported before collection. */
	readonly wasCurrentRecoveryBase: boolean
}

export interface ReceiptRotationReport {
	readonly openEpoch: ReceiptEpoch
}

export interface ReceiptRetirementReport {
	readonly retiredThrough: bigint
}

export interface GcReport {
	readonly objectEpoch: bigint
	readonly swept: bigint
	readonly orphansObserved: bigint
}

export interface BackupReport {
	readonly manifestDigest: string
	readonly objects: bigint
	readonly bytes: bigint
	readonly at: DecisionStamp
}

export interface BackupVerification {
	readonly identity: DatabaseIdentity
	readonly at: DecisionStamp
	readonly state: StateStamp
	readonly objects: bigint
	readonly bytes: bigint
	readonly manifestDigest: string
}

export interface RestoreReport {
	/** Writable restore is a new incarnation; read-only grants none. */
	readonly identity: DatabaseIdentity
	readonly genesis: string
	readonly binding: HistoryBinding
}

export interface ResidualCopy {
	readonly kind: string
	readonly location: string
}

export interface ErasureReport {
	readonly tombstoned: boolean
	readonly retainedRoots: readonly RootId[]
	/** Residual copies reported honestly, never a secure-erase claim. */
	readonly residual: readonly ResidualCopy[]
}

// ── Migration workflow values (chapters 22/33, C11) ────────────────────────

export type { ActivationRef, GeneratedMigrations }

/** Binds abort to the exact operation, plan set and planned target. */
export interface MigrationRef {
	readonly operation: OperationRef
	readonly planSetDigest: PlanSetDigest
	readonly target: DatabaseIdentity
}

export interface SourceAccessReport {
	readonly access: AccessMode
	readonly operation: OperationId | null
}

export type MigrateValue =
	| { readonly kind: "up-to-date"; readonly binding: HistoryBinding }
	| {
			readonly kind: "ready-to-switch"
			readonly deploymentBinding: HistoryBinding
			readonly activation: ActivationRef
	  }
	| { readonly kind: "paused"; readonly error: LogError; readonly sourceState: SourceAccessReport }

export interface InitializeValue {
	readonly binding: HistoryBinding
	readonly genesis: string
}

export interface ActivationReport {
	readonly target: DatabaseIdentity
	readonly accessMode: AccessMode
	readonly operation: OperationId
	/** True when this invocation performed the one-time transition. */
	readonly activatedNow: boolean
}

export interface AbortReport {
	readonly target: DatabaseIdentity
	readonly targetFenced: boolean
	readonly sourceAccess: AccessMode
}

export type MigrationStatus =
	| { readonly kind: "up-to-date"; readonly appliedPrefixDigest: string }
	| { readonly kind: "pending"; readonly pending: readonly string[] }
	| { readonly kind: "in-progress"; readonly operation: OperationRef }
	| {
			readonly kind: "paused"
			readonly operation: OperationRef
			readonly error: LogError
			readonly sourceState: SourceAccessReport
	  }
	| { readonly kind: "ready-to-switch"; readonly operation: OperationRef; readonly activation: ActivationRef }
	| { readonly kind: "activated"; readonly operation: OperationRef; readonly target: DatabaseIdentity }
	| { readonly kind: "aborted"; readonly operation: OperationRef }
	| { readonly kind: "outcome-unknown"; readonly operation: OperationRef; readonly error: LogError }
	| { readonly kind: "drift"; readonly detail: string }
	| { readonly kind: "database-ahead"; readonly detail: string }
