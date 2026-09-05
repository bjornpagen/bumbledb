/**
 * Reusable typed query templates — inert schema-level values shared by
 * every tenant's snapshots. No tenant rows, no live handles, no I/O at
 * import.
 */
import { query, v } from "@bjornpagen/bumbledb"
import { App, Attachment, Note, Outbox } from "./schema.ts"

/** Every note fact, full row shape (encodable via the Note relation). */
export const allNotes = query(App).rule((r) => {
	const { id, text, pinned } = v(Note)
	return r.match(Note, { id, text, pinned }).find({ id, text, pinned })
})

/** One note by id, full row shape; parameters infer `{ id: Id128 }`. */
export const noteById = query(App).rule((r) => {
	const { id, text, pinned } = v(Note)
	return r
		.match(Note, { id, text, pinned })
		.where(r.eq(id, r.param("id")))
		.find({ id, text, pinned })
})

/** Pending outbox work, full row shape for the dispatcher. */
export const pendingOutbox = query(App).rule((r) => {
	const { id, note, kind } = v(Outbox)
	return r.match(Outbox, { id, note, kind }).find({ id, note, kind })
})

/** Attachments of one note. */
export const attachmentsFor = query(App).rule((r) => {
	const { id, note, key, bytes } = v(Attachment)
	return r
		.match(Attachment, { id, note, key, bytes })
		.where(r.eq(note, r.param("note")))
		.find({ id, note, key, bytes })
})
