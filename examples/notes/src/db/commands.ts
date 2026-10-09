/**
 * The app's write commands. Each request's id comes from a client-supplied key (or is derived
 * deterministically from one), so a retried request submits the identical command and the log
 * decides it once. External effects ride the outbox: the pending-effect row commits in the same
 * command as the change that needs it.
 */
import { createHash } from "node:crypto"
import type { Database, SubmitOutcome } from "@bjornpagen/bumbledb"
import { ChangeSet, RequestId, Uuid } from "@bjornpagen/bumbledb"
import { Effect, Option } from "effect"
import { App, Attachment, Note, NoteById, Outbox } from "./schema.ts"

type Db = Database<typeof App>

/** The same source id and role always name the same id, so retries rebuild the same command. */
export function derivedId(source: Uuid, role: string): Uuid {
	const digest = createHash("sha256").update(`${source}:${role}`).digest()
	const parsed = Uuid.fromBytes(digest.subarray(0, 16))
	if (parsed._tag !== "Success") throw new Error("sixteen digest bytes always parse")
	return parsed.success
}

/** A request id is the request key's 32 hex digits. */
export function requestIdOf(key: Uuid): RequestId {
	return RequestId.make(key.replaceAll("-", ""))
}

/** Creates a note and its `note-created` outbox row in one command keyed by the note id. */
export const createNote = Effect.fn("commands.createNote")(
	function* (db: Db, noteId: Uuid, text: string) {
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Note, [{ id: noteId, text, pinned: false }])
		yield* draft.insert(Outbox, [{ id: derivedId(noteId, "outbox:note-created"), note: noteId, kind: "note-created" }])
		return yield* db.submit(yield* draft.finish(), { requestId: requestIdOf(noteId) })
	},
	Effect.scoped
)

/**
 * Sets a note's pin at the revision it was read at. If anything committed in between, the command
 * is decided as a failed precondition; revising needs a new request key.
 */
export const setPinned = Effect.fn("commands.setPinned")(
	function* (db: Db, requestKey: Uuid, noteId: Uuid, pinned: boolean) {
		const reader = yield* db.read("latest")
		const previous = yield* reader.get(NoteById, { id: noteId })
		if (Option.isNone(previous)) return { _tag: "Missing" } as const
		const draft = yield* ChangeSet.builder(App)
		yield* draft.delete(Note, [previous.value])
		yield* draft.insert(Note, [{ ...previous.value, pinned }])
		const outcome: SubmitOutcome = yield* db.submit(yield* draft.finish(), {
			requestId: requestIdOf(requestKey),
			precondition: reader.revision
		})
		return { _tag: "Submitted", outcome } as const
	},
	Effect.scoped
)

/** References an already-uploaded blob. */
export const addAttachment = Effect.fn("commands.addAttachment")(
	function* (db: Db, noteId: Uuid, blob: { readonly key: string; readonly bytes: bigint }) {
		const attachmentId = derivedId(noteId, `attachment:${blob.key}`)
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Attachment, [{ id: attachmentId, note: noteId, key: blob.key, bytes: blob.bytes }])
		return yield* db.submit(yield* draft.finish(), { requestId: requestIdOf(attachmentId) })
	},
	Effect.scoped
)

/** Retires one dispatched outbox row; the request id derives from the row id. */
export const retireOutbox = Effect.fn("commands.retireOutbox")(
	function* (db: Db, row: { readonly id: Uuid; readonly note: Uuid; readonly kind: string }) {
		const draft = yield* ChangeSet.builder(App)
		yield* draft.delete(Outbox, [row])
		return yield* db.submit(yield* draft.finish(), { requestId: requestIdOf(derivedId(row.id, "outbox:retire")) })
	},
	Effect.scoped
)
