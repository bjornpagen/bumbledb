/**
 * Owned sealed commands. `seal` retains the exact native core `ChangeSet`
 * (no second row walk), checks its schema against the command scope on the
 * bounded worker, and yields an immutable ref before any submission.
 * `encode`/`decode` are the one bounded versioned native command codec for
 * cross-process resubmission — a ref alone resolves uncertainty but cannot
 * reconstruct a missing payload. Copy `command.ref` and retain it with the
 * original intent BEFORE dispatch.
 */

import type { AnySchema, NativeRuntime } from "@bjornpagen/bumbledb"
import type { Effect, Scope } from "effect"
import type { LogError } from "#errors.ts"
import { log } from "#production.ts"
import type { CommandInput, Command as CommandInterface } from "#surface.ts"

/** The owned sealed command value; `ref` is copyable before dispatch. */
export type Command<S extends AnySchema> = CommandInterface<S>

export const Command: {
	seal<S extends AnySchema>(input: CommandInput<S>): Effect.Effect<Command<S>, LogError, Scope.Scope>
	encode<S extends AnySchema>(command: Command<S>): Effect.Effect<Uint8Array, LogError>
	decode<S extends AnySchema>(
		bytes: Uint8Array,
		schema: S
	): Effect.Effect<Command<S>, LogError, NativeRuntime | Scope.Scope>
} = log.Command
