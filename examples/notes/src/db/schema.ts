/**
 * The application's CURRENT schema — ordinary typed declarations shared by
 * the app's queries AND the migration generator (chapter 33: the same
 * module, no generated runtime-type layer between them). Importing this
 * module constructs inert metadata only; no database work happens here.
 *
 * This is the FINAL stage of the staged evolution history in
 * `evolution-stages.ts` (0000-initialize → 0004-outbox-attachment). There
 * is deliberately NO pending `migrationIntent` export here: intent is
 * generation input, consumed when its plan is recorded, then removed —
 * leaving a stale intent in place refuses the next generate/check run.
 * When the schema evolves again, add the typed intent beside the changed
 * declarations, run `bumbledb-log generate`, review the emitted plan, and
 * delete the intent.
 */
import { bool, contained, id128, key, on, relation, schema, str, u64 } from "@bjornpagen/bumbledb"

/** A user's note. `text` was renamed from `body` in plan 0003. */
export const Note = relation("Note", { id: id128, text: str, pinned: bool })

/** Fixed labels, seeded declaratively in plan 0002. */
export const Tag = relation("Tag", { id: id128, name: str })

/**
 * The application outbox (OPS-003): a pending external effect recorded
 * ATOMICALLY with the domain change that requires it. The dispatcher
 * (`scripts/dispatch-outbox.ts`) performs the effect and deletes the row
 * in a separate idempotent command. Deliberately NOT contained in Note:
 * a pending dispatch may outlive its note.
 */
export const Outbox = relation("Outbox", { id: id128, note: id128, kind: str })

/**
 * A blob reference. The immutable blob is uploaded FIRST (content-addressed
 * S3 key, app-owned bucket); the fact referencing it commits second, so a
 * crash between the two leaves an orphan upload, never a dangling
 * reference (OPS-003 "immutable blob first, reference commit second").
 */
export const Attachment = relation("Attachment", {
	id: id128,
	note: id128,
	key: str,
	bytes: u64
})

export const App = schema("App", { Note, Tag, Outbox, Attachment }, [
	key(Note, ["id"]),
	key(Tag, ["id"]),
	key(Outbox, ["id"]),
	key(Attachment, ["id"]),
	contained(on(Attachment, "note"), on(Note, "id"))
])
