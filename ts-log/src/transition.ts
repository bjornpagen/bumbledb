import type { AnySchema, ChangeSet, CloseReport } from "@bjornpagen/bumbledb"
import type { Effect, Scope } from "effect"
import type { LogError } from "#errors.ts"
import type { DatabaseIdentity, DecisionDigest, DecisionStamp, OperationId, StateStamp } from "#identity.ts"
import type { LocalBinding } from "#options.ts"
import { log } from "#production.ts"
import type { History, HistoryBorrow, PublishedSnapshot } from "#surface.ts"

/** Retain before dispatch. The commitment identifies application intent; it
 * does not prove which TypeScript ran or whether data was preserved. */
export interface TransitionContract {
	readonly operation: OperationId
	readonly source: DatabaseIdentity
	readonly target: DatabaseIdentity
	readonly commitment: string
}
export interface TransitionCapture {
	readonly contract: TransitionContract
	readonly decision: DecisionStamp
	readonly state: StateStamp
}
/** Native evidence of the one admitted target. Keep bytes for retries;
 * activation validates them rather than trusting the decoded display fields. */
export interface InstalledTransition {
	readonly captured: TransitionCapture
	readonly applicationDigest: string
	readonly bytes: Uint8Array
}
export interface Population<S extends AnySchema> {
	readonly schema: S
	/** Sequential native batches, with complete law judgment at finish. */
	apply(changes: ChangeSet<S>): Effect.Effect<void, LogError>
	/** Consumes all writer aliases. Resolve the contract after an uncertain failure. */
	finish(): Effect.Effect<ReadyTransition, LogError>
	/** Releases the attempt; durable abort/thaw is always explicit. */
	close(): Effect.Effect<CloseReport>
}
export interface ReadyTransition {
	readonly kind: "ready"
	readonly installed: InstalledTransition
	readonly binding: LocalBinding
}
export interface ActivatedTransition {
	readonly kind: "activated"
	readonly contract: TransitionContract
	readonly genesis: DecisionDigest
	readonly binding: LocalBinding
}
export type TransitionResolution =
	| ReadyTransition
	| ActivatedTransition
	| { readonly kind: "uninstalled" }
	| { readonly kind: "aborted" }
export type TransitionStart<S extends AnySchema, T extends AnySchema> =
	| {
			readonly kind: "populating"
			readonly captured: TransitionCapture
			readonly source: PublishedSnapshot<S>
			readonly population: Population<T>
			readonly binding: LocalBinding
	  }
	| (ReadyTransition & { readonly source: PublishedSnapshot<S> })
	| ActivatedTransition
	| { readonly kind: "aborted" }
type Source<S extends AnySchema> = History<S> | HistoryBorrow<S>
export interface TransitionOperations {
	begin<S extends AnySchema, T extends AnySchema>(
		source: Source<S>,
		target: T,
		contract: TransitionContract
	): Effect.Effect<TransitionStart<S, T>, LogError, Scope.Scope>
	resolve<S extends AnySchema, T extends AnySchema>(
		source: Source<S>,
		target: T,
		contract: TransitionContract
	): Effect.Effect<TransitionResolution, LogError>
	/** Open the ready binding using the ordinary LocalHistory API, inspect in
	 * a nested scope, and release that scope before activating. */
	inspect<T extends AnySchema>(
		target: Source<T>,
		installed: InstalledTransition
	): Effect.Effect<PublishedSnapshot<T>, LogError, Scope.Scope>
	activate<S extends AnySchema, T extends AnySchema>(
		source: Source<S>,
		target: T,
		installed: InstalledTransition
	): Effect.Effect<ActivatedTransition, LogError>
	abort<S extends AnySchema, T extends AnySchema>(
		source: Source<S>,
		target: T,
		contract: TransitionContract
	): Effect.Effect<void, LogError>
}
export const Transition: TransitionOperations = log.Transition
