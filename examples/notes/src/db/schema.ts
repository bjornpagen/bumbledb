import { bool, contained, key, on, relation, schema, str, u64, uuid } from "@bjornpagen/bumbledb"

export const Note = relation("Note", { id: uuid, text: str, pinned: bool })
export const NoteById = key(Note, ["id"])

/** Labels; migration 0001 seeds the defaults. */
export const Tag = relation("Tag", { id: uuid, name: str })

/**
 * Pending external effects, committed in the same command as the change that needs them; the
 * dispatcher performs each and retires its row. Not contained in Note: a dispatch may outlive it.
 */
export const Outbox = relation("Outbox", { id: uuid, note: uuid, kind: str })

/**
 * A blob reference. The content-addressed blob is uploaded first and the reference committed
 * second, so a crash between them leaves an orphan upload, never a dangling reference.
 */
export const Attachment = relation("Attachment", { id: uuid, note: uuid, key: str, bytes: u64 })

export const App = schema("App", { Note, Tag, Outbox, Attachment }, [
	NoteById,
	key(Tag, ["id"]),
	key(Outbox, ["id"]),
	key(Attachment, ["id"]),
	contained(on(Attachment, "note"), on(Note, "id"))
])
