/** Current application declarations; Log snapshots these values directly. */
import { bool, contained, uuid, key, on, relation, schema, str, u64 } from "@bjornpagen/bumbledb"

/** A user's note. */
export const Note = relation("Note", { id: uuid, text: str, pinned: bool })
export const NoteById = key(Note, ["id"])

/** Application labels, populated by explicit initialization. */
export const Tag = relation("Tag", { id: uuid, name: str })

/**
 * The application outbox (OPS-003): a pending external effect recorded
 * ATOMICALLY with the domain change that requires it. The dispatcher
 * (`scripts/dispatch-outbox.ts`) performs the effect and deletes the row
 * in a separate idempotent command. Deliberately NOT contained in Note:
 * a pending dispatch may outlive its note.
 */
export const Outbox = relation("Outbox", { id: uuid, note: uuid, kind: str })

/**
 * A blob reference. The immutable blob is uploaded FIRST (content-addressed
 * S3 key, app-owned bucket); the fact referencing it commits second, so a
 * crash between the two leaves an orphan upload, never a dangling
 * reference (OPS-003 "immutable blob first, reference commit second").
 */
export const Attachment = relation("Attachment", {
	id: uuid,
	note: uuid,
	key: str,
	bytes: u64
})

export const App = schema("App", { Note, Tag, Outbox, Attachment }, [
	NoteById,
	key(Tag, ["id"]),
	key(Outbox, ["id"]),
	key(Attachment, ["id"]),
	contained(on(Attachment, "note"), on(Note, "id"))
])
