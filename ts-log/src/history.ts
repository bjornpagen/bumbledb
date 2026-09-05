/**
 * `LocalHistory` and `HostedHistory`: the chapter 30/35 durable envelope
 * around the exact core change/read machinery. `open` of a missing or
 * unreadable configured database never creates a replacement; `create` is
 * the explicit constructor, refuses existing authority, and validates its
 * stable creation identity plus checked initialization artifact instead of
 * fabricating genesis or applied migration history.
 */
import type { Effect, Scope } from "effect"
import type { AnySchema, NativeRuntime } from "@bjornpagen/bumbledb"
import type { LogError } from "#errors.ts"
import type {
	HostedBinding,
	HostedCreateOptions,
	HostedOpenOptions,
	LocalBinding,
	LocalCreateOptions,
	LocalOpenOptions
} from "#options.ts"
import { log } from "#production.ts"
import type { History } from "#surface.ts"

export const LocalHistory: {
	open<S extends AnySchema>(
		binding: LocalBinding,
		schema: S,
		options: LocalOpenOptions
	): Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
	create<S extends AnySchema>(
		binding: LocalBinding,
		schema: S,
		options: LocalCreateOptions
	): Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
} = log.LocalHistory

export const HostedHistory: {
	open<S extends AnySchema>(
		binding: HostedBinding,
		schema: S,
		options: HostedOpenOptions
	): Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
	create<S extends AnySchema>(
		binding: HostedBinding,
		schema: S,
		options: HostedCreateOptions
	): Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
} = log.HostedHistory
