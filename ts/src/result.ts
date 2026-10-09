import { Effect, Option, Stream } from "effect"
import type { DbError } from "./errors.ts"
import type { CursorRef, ResultRef } from "./native/addon.ts"
import { addon } from "./native/addon.ts"
import { call, scoped } from "./native/op.ts"
import type { FindColumn } from "./query/atom.ts"
import { decodeAnswers } from "./query/run.ts"
import type { CellValue } from "./rows.ts"

/**
 * One complete answer set, sealed after evaluation and independent of its snapshot. `collect`
 * reads every row and leaves the result readable. `pages` spends the result once through
 * bounded batches; the stream owns its cursor, so ending it early releases the cursor, and
 * running it a second time fails with `SpentHandle`.
 */
interface CompleteResult<A> {
	collect(): Effect.Effect<ReadonlyArray<A>, DbError>
	pages(): Stream.Stream<ReadonlyArray<A>, DbError>
}

function decodePage<A>(finds: readonly FindColumn[], rows: readonly (readonly CellValue[])[]): ReadonlyArray<A> {
	return Object.freeze(decodeAnswers<A>(finds, rows))
}

class CompleteResultLive<A> implements CompleteResult<A> {
	readonly #handle: ResultRef
	readonly #finds: readonly FindColumn[]

	constructor(handle: ResultRef, finds: readonly FindColumn[]) {
		this.#handle = handle
		this.#finds = finds
	}

	collect(): Effect.Effect<ReadonlyArray<A>, DbError> {
		const handle = this.#handle
		const finds = this.#finds
		return call(
			"CompleteResult.collect",
			(done) => addon.runtimeResultCollect(handle, done),
			(lease) => decodePage<A>(finds, addon.runtimeRowsTake(lease))
		).pipe(Effect.withSpan("CompleteResult.collect"))
	}

	pages(): Stream.Stream<ReadonlyArray<A>, DbError> {
		const handle = this.#handle
		const finds = this.#finds
		return Stream.unwrap(
			Effect.gen(function* () {
				const cursor: CursorRef = yield* scoped(
					"ResultCursor.release",
					call("CompleteResult.pages", (done) => addon.runtimeResultCursor(handle, done), addon.runtimeCursorTake),
					(owned) => (done) => addon.runtimeCursorClose(owned, done)
				)
				return Stream.paginate(undefined, () =>
					call("CompleteResult.page", (done) => addon.runtimeCursorNext(cursor, done), addon.runtimePageTake).pipe(
						Effect.map((page) =>
							page === null
								? ([[], Option.none<undefined>()] as const)
								: ([[decodePage<A>(finds, page)], Option.some(undefined)] as const)
						)
					)
				)
			})
		).pipe(Stream.withSpan("CompleteResult.pages"))
	}

	/** The native result behind `value`, when `value` is a result this SDK made. */
	static handle(value: object): ResultRef | undefined {
		return #handle in value ? (value as CompleteResultLive<unknown>).#handle : undefined
	}
}

export type { CompleteResult }
export { CompleteResultLive }
