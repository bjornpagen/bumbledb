import { Effect, Option, Stream } from "effect"
import { drainClose, releaseOwner } from "#close.ts"
import type { CursorHandle, ResultHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import type { FindColumn } from "#query/atom.ts"
import { decodeAnswers } from "#query/run.ts"
import type { CellValue } from "#rows.ts"
import { nativeOperationWith } from "#runtime.ts"
import type { CloseReport, DbError } from "#runtime-errors.ts"

/**
 * One complete answer set, sealed after evaluation and independent of its
 * source snapshot. collect() explicitly materializes all rows into JS and
 * leaves the result available. pages() consumes it once through bounded
 * delivery batches; it does not stream query execution.
 *
 * The stream owns its cursor through Effect scope. Early termination,
 * failure, interruption and EOF close/drain it. Creating the stream does
 * not spend the result; running it twice fails with SpentHandle.
 */
interface CompleteResult<A> {
	collect(): Effect.Effect<ReadonlyArray<A>, DbError>
	pages(): Stream.Stream<ReadonlyArray<A>, DbError>
	close(): Effect.Effect<CloseReport>
}

interface ResultInternal {
	readonly handle: ResultHandle
}

const resultInternals = new WeakMap<object, ResultInternal>()

/** Private cross-module accessor (scope finalizers in db.ts reach the handle). */
function internalResult(value: object): ResultInternal | undefined {
	return resultInternals.get(value)
}

function decodePage<A>(finds: readonly FindColumn[], rows: readonly (readonly CellValue[])[]): ReadonlyArray<A> {
	// Owned ordinary records in declared column order — the same fields and
	// shapes on every page (stable row shape; no Proxy, no per-row fiber).
	return Object.freeze(decodeAnswers<A>(finds, rows))
}

/**
 * Internal constructor: `db.ts` publishes results through this after
 * execution completes. Each delivery has independent cooperative cancellation.
 */
function makeCompleteResult<A>(handle: ResultHandle, finds: readonly FindColumn[]): CompleteResult<A> {
	const value: CompleteResult<A> = {
		collect() {
			return Effect.suspend(() =>
				nativeOperationWith(
					"CompleteResult.collect",
					(callback) => dbNative.runtimeResultCollect(handle, callback),
					dbNative.runtimeRowsTake,
					(rows) => decodePage<A>(finds, rows)
				)
			)
		},
		pages() {
			return Stream.unwrap(
				Effect.gen(function* () {
					const cursor: CursorHandle = yield* Effect.acquireRelease(
						nativeOperationWith(
							"CompleteResult.pages",
							(callback) => dbNative.runtimeResultCursor(handle, callback),
							dbNative.runtimeCursorTake,
							(taken) => taken
						),
						(taken) => releaseOwner("ResultCursor.close", (callback) => dbNative.runtimeCursorClose(taken, callback)),
						{ interruptible: true }
					)
					return Stream.paginate(undefined, () =>
						nativeOperationWith(
							"CompleteResult.page",
							(callback) => dbNative.runtimeCursorNext(cursor, callback),
							dbNative.runtimePageTake,
							(page) => page
						).pipe(
							Effect.map((page) => {
								if (page === null) {
									return [[], Option.none<undefined>()] as const
								}
								return [[decodePage<A>(finds, page)], Option.some(undefined)] as const
							})
						)
					)
				})
			)
		},
		close() {
			return drainClose("CompleteResult.close", (callback) => dbNative.runtimeResultClose(handle, callback))
		}
	}
	Object.freeze(value)
	resultInternals.set(value, { handle })
	return value
}

export type { CompleteResult }
export { internalResult, makeCompleteResult }
