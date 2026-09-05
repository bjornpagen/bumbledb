import { Effect } from "effect"
/**
 * Shared close/drain adapters for scoped core owners (drafts, changes,
 * snapshots, sessions, results, cursors, databases). One policy, chapter
 * 35's: early `close()` starts/joins the native close transition and
 * returns the honest {@link CloseReport}; a scope FINALIZER runs the same
 * close and surfaces `incomplete`/`failed` as a structured `CloseFailure`
 * DEFECT in the finalizer Cause — never catch-and-log, never false
 * quiescence. Teardown uses the runtime's reserved cleanup envelope
 * natively; repeated close joins the same stored transition (idempotent).
 */
import type { CloseWire } from "#runtime-native.ts"
import { finalizeClose } from "#runtime.ts"
import type { CloseReport } from "#runtime-errors.ts"
import { DbError } from "#runtime-errors.ts"

function reportOf(operation: string, wire: CloseWire): CloseReport {
	if (wire.kind === "failed") {
		return { kind: "failed", error: new DbError({ operation, reason: { _tag: "Internal" } }) }
	}
	return wire
}

/**
 * Adapts one native close verb to `Effect<CloseReport>`: registration is
 * synchronous, completion resumes exactly once, and the whole wait is
 * uninterruptible — a close is a bounded registration/drain handshake, not
 * long maskable work (the native side owns the cleanup deadline).
 */
function drainClose(operation: string, start: (callback: (report: CloseWire) => void) => void): Effect.Effect<CloseReport> {
	return Effect.callback<CloseReport>((resume) => {
		try {
			start((report) => resume(Effect.succeed(reportOf(operation, report))))
		} catch (cause) {
			resume(
				Effect.succeed({
					kind: "failed",
					error: cause instanceof DbError ? cause : new DbError({ operation, reason: { _tag: "Internal" } })
				})
			)
		}
	}).pipe(Effect.uninterruptible)
}

/** The finalizer policy: run the close, then die on incomplete/failed (E stays never). */
function releaseOwner(
	operation: string,
	start: (callback: (report: CloseWire) => void) => void
): Effect.Effect<void> {
	return drainClose(operation, start).pipe(Effect.flatMap((report) => finalizeClose(operation, report)))
}

export { drainClose, releaseOwner }
