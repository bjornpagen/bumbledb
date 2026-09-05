import { Effect, Option, Stream } from "effect"
import { drainClose, releaseOwner } from "#close.ts"
import type { CursorHandle, ResultHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import type { FindColumn } from "#query/atom.ts"
import { decodeAnswers } from "#query/run.ts"
import type { CellValue } from "#rows.ts"
import type { CloseReport } from "#runtime-errors.ts"
import type { DbError } from "#runtime-errors.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import { nativeOperationWith, policyWire } from "#runtime.ts"

/**
 * `CompleteResult<A>` — the sealed owner of one COMPLETED query answer
 * (C05): published only after all evaluation/finalization succeeded,
 * possibly backed by temporary LMDB scratch. Owned and independent of its
 * source snapshot/session.
 *
 * `collect` materializes a bounded owned array and leaves the result
 * available; it refuses (`ResourceLimit`) before allocating past
 * `maxBytes`, and a cap failure leaves the sealed backing available for
 * `pages`. `pages` is chapter 35's ONE-SHOT consuming stream over the
 * completed result: its first execution atomically spends the result and
 * moves the backing storage into a private cursor owned by the stream's
 * scope — construction alone spends nothing, a second run fails
 * `SpentHandle`, and a run after the result's scope closed fails
 * `ClosedHandle`. Early take, downstream failure and interruption all
 * close/drain the private cursor; EOF cleanup is identical. There is no
 * public cursor, `next`, AsyncIterable, clone or second streaming API.
 */
interface CompleteResult<A> {
	collect(options: { readonly maxBytes: bigint }): Effect.Effect<ReadonlyArray<A>, DbError>
	pages(options: { readonly pageBytes: bigint }): Stream.Stream<ReadonlyArray<A>, DbError>
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
 * execution completes. `basePolicy` is the executing call's policy; collect
 * and page pulls run under it with the byte cap swapped for the caller's
 * `maxBytes`/`pageBytes` (delivery backpressure bounds transport, never the
 * query work already done).
 */
function makeCompleteResult<A>(
	handle: ResultHandle,
	finds: readonly FindColumn[],
	basePolicy: ExecutionPolicy
): CompleteResult<A> {
	function cappedWire(operation: string, resultBytes: bigint) {
		return policyWire({ ...basePolicy, resultBytes }, operation)
	}
	const value: CompleteResult<A> = {
		collect(options) {
			return Effect.suspend(() =>
				nativeOperationWith(
					"CompleteResult.collect",
					(callback) => dbNative.runtimeResultCollect(handle, cappedWire("CompleteResult.collect", options.maxBytes), callback),
					dbNative.runtimeRowsTake,
					(rows) => decodePage<A>(finds, rows)
				)
			)
		},
		pages(options) {
			return Stream.unwrap(
				Effect.gen(function* () {
					// Atomic spend: the transfer refuses (SpentHandle) on a
					// second run or a concurrent collect/transfer race
					// before touching the backing. The cursor belongs to
					// the STREAM's scope from this point.
					const cursor: CursorHandle = yield* Effect.acquireRelease(
						nativeOperationWith(
							"CompleteResult.pages",
							(callback) => dbNative.runtimeResultCursor(handle, cappedWire("CompleteResult.pages", options.pageBytes), callback),
							dbNative.runtimeCursorTake,
							(taken) => taken
						),
						(taken) => releaseOwner("ResultCursor.close", (callback) => dbNative.runtimeCursorClose(taken, callback)),
						{ interruptible: true }
					)
					return Stream.paginate(undefined, () =>
						nativeOperationWith(
							"CompleteResult.page",
							(callback) => dbNative.runtimeCursorNext(cursor, cappedWire("CompleteResult.page", options.pageBytes), callback),
							dbNative.runtimePageTake,
							(page) => page
						).pipe(
							Effect.map((page) => {
								if (page === null) {
									// Terminal EOF: emit nothing further; the
									// stream scope's finalizer joins the
									// cursor close (identical cleanup to
									// early termination).
									return [[], Option.none<undefined>()] as const
								}
								// One OWNED page array per element — pages,
								// not rows (paginate emits the elements of
								// the returned array; the singleton array
								// carries the page).
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
