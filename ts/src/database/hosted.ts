/**
 * The addon's hosted machine as a driver port. A hosted step is never cancelled: its output carries
 * requests the machine then waits on, so every step runs to completion even when its caller is
 * interrupted.
 */
import { Effect } from "effect"
import type { DbError } from "../errors.ts"
import type { ChangesRef, OperationRef } from "../native/addon.ts"
import { addon } from "../native/addon.ts"
import type {
	CopyPair,
	ExternalObject,
	HeadOut,
	HostedHandle,
	IoOutcomeIn,
	SettledOut,
	StepOut
} from "../native/binding.d.ts"
import { call } from "../native/op.ts"
import type { MachinePort, Step } from "./driver.ts"
import type { IoResponse, IoResult } from "./io.ts"

type HostedRef = ExternalObject<HostedHandle>

/** One machine request. `request` ids are 32 hex digits; `revision` present is an exact-revision precondition. */
type HostedRequest =
	| { readonly _tag: "Open" }
	| { readonly _tag: "Sync" }
	| { readonly _tag: "Resolve"; readonly request: string }
	| { readonly _tag: "Freeze"; readonly leaseMillis: bigint }
	| {
			readonly _tag: "Submit"
			readonly request: string
			readonly revision: bigint | null
			readonly changes: ChangesRef
	  }
	| {
			readonly _tag: "Migrate"
			readonly step: number
			readonly base: bigint
			readonly copy: readonly CopyPair[]
			readonly rows: ChangesRef
	  }

function outcomeIn(result: Exclude<IoResult, { readonly _tag: "Body" }>): IoOutcomeIn {
	return result._tag === "Keys" ? { _tag: "Keys", keys: [...result.keys] } : result
}

/** A port over one hosted machine; `onHead` sees every head a step reports. */
function hostedPort(hosted: HostedRef, onHead: (head: HeadOut) => void): MachinePort<HostedRequest, SettledOut> {
	const step = (
		operation: string,
		start: (done: () => void) => OperationRef
	): Effect.Effect<Step<SettledOut>, DbError> =>
		call(operation, start, addon.hostedStepTake).pipe(
			Effect.tap((out: StepOut) =>
				Effect.sync(() => {
					if (out.head !== undefined) onHead(out.head)
				})
			),
			Effect.uninterruptible
		)
	return {
		request: (ticket, request) => {
			switch (request._tag) {
				case "Open":
				case "Sync":
					return step(`Database.${request._tag}`, (done) =>
						addon.hostedStep(hosted, { _tag: request._tag, ticket }, done)
					)
				case "Resolve":
					return step("Database.resolve", (done) =>
						addon.hostedStep(hosted, { _tag: "Resolve", ticket, request: request.request }, done)
					)
				case "Freeze":
					return step("Database.freeze", (done) =>
						addon.hostedStep(hosted, { _tag: "Freeze", ticket, leaseMillis: request.leaseMillis }, done)
					)
				case "Submit":
					return step("Database.submit", (done) =>
						addon.hostedSubmit(hosted, ticket, request.request, request.revision, request.changes, done)
					)
				case "Migrate":
					return step("Database.migrate", (done) =>
						addon.hostedMigrate(hosted, ticket, request.step, request.base, [...request.copy], request.rows, done)
					)
			}
		},
		respond: (response: IoResponse) => {
			const { id, date, result } = response
			return result._tag === "Body"
				? step("Database.respond", (done) =>
						addon.hostedRespondBody(hosted, id, date, result.lastModified, result.bytes, done)
					)
				: step("Database.respond", (done) => addon.hostedRespond(hosted, id, date, outcomeIn(result), done))
		},
		close: step("Database.close", (done) => addon.hostedStep(hosted, { _tag: "Close" }, done))
	}
}

export type { HostedRef, HostedRequest }
export { hostedPort }
