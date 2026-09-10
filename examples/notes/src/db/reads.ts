/**
 * Shared QueryReader helpers — the same collect/pages programs run on a
 * core snapshot and on a published log snapshot. No adapter, no scan of
 * a whole relation when a key or template exists.
 */
import type { Uuid, QueryReader } from "@bjornpagen/bumbledb"
import { Effect, Option, Stream } from "effect"
import { allNotes, attachmentsFor, noteById, pendingOutbox } from "./queries.ts"
import { App, NoteById } from "./schema.ts"

export const listNotes = Effect.fn("reads.listNotes")(
	function* (reader: QueryReader<typeof App>) {
		const result = yield* reader.execute(allNotes, {})
		return yield* result.collect()
	},
	Effect.scoped
)

export const pageNotes = Effect.fn("reads.pageNotes")(
	function* (reader: QueryReader<typeof App>) {
		const result = yield* reader.execute(allNotes, {})
		return yield* result.pages().pipe(Stream.runFold(() => 0, (rows, page) => rows + page.length))
	},
	Effect.scoped
)

export const getNote = Effect.fn("reads.getNote")(
	function* (reader: QueryReader<typeof App>, id: Uuid) {
		return yield* reader.get(NoteById, { id })
	}
)

export const findNote = Effect.fn("reads.findNote")(
	function* (reader: QueryReader<typeof App>, id: Uuid) {
		const result = yield* reader.execute(noteById, { id })
		const rows = yield* result.collect()
		return rows[0] === undefined ? Option.none() : Option.some(rows[0])
	},
	Effect.scoped
)

export const listPendingOutbox = Effect.fn("reads.listPendingOutbox")(
	function* (reader: QueryReader<typeof App>) {
		const result = yield* reader.execute(pendingOutbox, {})
		return yield* result.collect()
	},
	Effect.scoped
)

export const listAttachments = Effect.fn("reads.listAttachments")(
	function* (reader: QueryReader<typeof App>, note: Uuid) {
		const result = yield* reader.execute(attachmentsFor, { note })
		return yield* result.collect()
	},
	Effect.scoped
)
