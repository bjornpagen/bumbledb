/**
 * Shared QueryReader helpers — the same collect/pages programs run on a
 * core snapshot and on a published log snapshot. No adapter, no scan of
 * a whole relation when a key or template exists.
 */
import type { ExecutionPolicy, Id128, QueryReader } from "@bjornpagen/bumbledb"
import { Effect, Option, Stream } from "effect"
import { allNotes, attachmentsFor, noteById, pendingOutbox } from "./queries.ts"
import { App, Note } from "./schema.ts"

export const listNotes = Effect.fn("reads.listNotes")(
	function* (reader: QueryReader<typeof App>, work: ExecutionPolicy) {
		const result = yield* reader.execute(allNotes, {}, work)
		return yield* result.collect({ maxBytes: work.resultBytes }, work)
	},
	Effect.scoped
)

export const pageNotes = Effect.fn("reads.pageNotes")(
	function* (reader: QueryReader<typeof App>, work: ExecutionPolicy, pageBytes: bigint) {
		const result = yield* reader.execute(allNotes, {}, work)
		return yield* result.pages({ pageBytes }, work).pipe(Stream.runFold(0, (rows, page) => rows + page.length))
	},
	Effect.scoped
)

export const getNote = Effect.fn("reads.getNote")(
	function* (reader: QueryReader<typeof App>, id: Id128, work: ExecutionPolicy) {
		return yield* reader.get(Note, { id }, work)
	}
)

export const findNote = Effect.fn("reads.findNote")(
	function* (reader: QueryReader<typeof App>, id: Id128, work: ExecutionPolicy) {
		const result = yield* reader.execute(noteById, { id }, work)
		const rows = yield* result.collect({ maxBytes: work.resultBytes }, work)
		return rows[0] === undefined ? Option.none() : Option.some(rows[0])
	},
	Effect.scoped
)

export const listPendingOutbox = Effect.fn("reads.listPendingOutbox")(
	function* (reader: QueryReader<typeof App>, work: ExecutionPolicy) {
		const result = yield* reader.execute(pendingOutbox, {}, work)
		return yield* result.collect({ maxBytes: work.resultBytes }, work)
	},
	Effect.scoped
)

export const listAttachments = Effect.fn("reads.listAttachments")(
	function* (reader: QueryReader<typeof App>, note: Id128, work: ExecutionPolicy) {
		const result = yield* reader.execute(attachmentsFor, { note }, work)
		return yield* result.collect({ maxBytes: work.resultBytes }, work)
	},
	Effect.scoped
)
