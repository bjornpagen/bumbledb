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
import { deliveryResultBytes, nativeOperationWith, policyWire } from "#runtime.ts"

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
	collect(options: { readonly maxBytes: bigint }, work: ExecutionPolicy): Effect.Effect<ReadonlyArray<A>, DbError>
	pages(options: { readonly pageBytes: bigint }, work: ExecutionPolicy): Stream.Stream<ReadonlyArray<A>, DbError>
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
 * execution completes. Delivery (`collect`/`pages`) starts a fresh bounded
 * operation under the caller's delivery policy — never the completed query's
 * expired execution deadline.
 */
function makeCompleteResult<A>(
	handle: ResultHandle,
	finds: readonly FindColumn[]
): CompleteResult<A> {
	function deliveryWire(operation: string, delivery: ExecutionPolicy, requested: bigint) {
		return policyWire({ ...delivery, resultBytes: deliveryResultBytes(requested, delivery) }, operation)
	}
	const value: CompleteResult<A> = {
		collect(options, work) {
			return Effect.suspend(() =>
				nativeOperationWith(
					"CompleteResult.collect",
					(callback) =>
						dbNative.runtimeResultCollect(
							handle,
							deliveryWire("CompleteResult.collect", work, options.maxBytes),
							callback
						),
					dbNative.runtimeRowsTake,
					(rows) => decodePage<A>(finds, rows)
				)
			)
		},
		pages(options, work) {
			return Stream.unwrap(
				Effect.gen(function* () {
					const cursor: CursorHandle = yield* Effect.acquireRelease(
						nativeOperationWith(
							"CompleteResult.pages",
							(callback) =>
								dbNative.runtimeResultCursor(
									handle,
									deliveryWire("CompleteResult.pages", work, options.pageBytes),
									callback
								),
							dbNative.runtimeCursorTake,
							(taken) => taken
						),
						(taken) => releaseOwner("ResultCursor.close", (callback) => dbNative.runtimeCursorClose(taken, callback)),
						{ interruptible: true }
					)
					return Stream.paginate(undefined, () =>
						nativeOperationWith(
							"CompleteResult.page",
							(callback) =>
								dbNative.runtimeCursorNext(
									cursor,
									deliveryWire("CompleteResult.page", work, options.pageBytes),
									callback
								),
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
