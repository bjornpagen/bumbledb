/**
 * The app's write commands: one core ChangeSet, one sealed log command,
 * retained ref, typed submit certainty. Business idempotency lives here:
 *
 *  - Entity/request identity is CLIENT-SUPPLIED or DETERMINISTICALLY
 *    derived from it (never minted per invocation), so a retried request
 *    seals the byte-identical command — same command ID, same digest —
 *    and the log's receipt lookup deduplicates admission.
 *  - External effects ride the OUTBOX: the pending-effect fact commits
 *    ATOMICALLY with the domain change (one tenant command is one atomic
 *    set effect); `scripts/dispatch-outbox.ts` performs and retires it.
 */
import { createHash } from "node:crypto"
import { ChangeSet, Uuid } from "@bjornpagen/bumbledb"
import type { CommandRef, History, HistoryBorrow, SubmitOptions, SubmitOutcome } from "@bjornpagen/bumbledb-log"
import { Command, RequestId } from "@bjornpagen/bumbledb-log"
import { Effect } from "effect"
import { rememberCommandRef, rememberSubmitOutcome } from "../requests.ts"
import { App, Attachment, Note, NoteById, Outbox } from "./schema.ts"

type Writer =
	| Pick<History<typeof App>, "identity" | "receiptEpoch" | "submit" | "resolve">
	| HistoryBorrow<typeof App>

export const submitOptionsOf = (): SubmitOptions => ({
	attempts: 4,
	backoff: { baseMillis: 50, capMillis: 2_000 }
})

/**
 * Deterministic derived identity: the same source id and role always name
 * the same 16 bytes, so a retried request rebuilds the identical command.
 * App policy (SHA-256 truncation), not a database allocator.
 */
export function derivedId(source: Uuid, role: string): Uuid {
	const digest = createHash("sha256").update(`${source}:${role}`).digest()
	const parsed = Uuid.fromBytes(digest.subarray(0, 16))
	if (parsed._tag !== "Success") {
		throw new Error("derivedId: sixteen digest bytes always parse")
	}
	return parsed.success
}

const requestIdOf = (key: Uuid) =>
	Effect.fromResult(RequestId.from(key)).pipe(Effect.orDie)

/**
 * Seal + retain + submit one command. The ref is persisted BEFORE
 * dispatch; the observed outcome after. Interruption stays in Cause —
 * the persisted ref remains the recovery coordinate.
 */
const sealAndSubmit = Effect.fn("commands.sealAndSubmit")(
	function* (
		writer: Writer,
		tenantId: string,
		requestKey: Uuid,
		changes: ChangeSet<typeof App>,
		resultMeta: Readonly<Record<string, Uuid>>
	) {
		const requestId = yield* requestIdOf(requestKey)
		const command = yield* Command.seal(
			{
				scope: writer.identity,
				id: { receiptEpoch: writer.receiptEpoch, requestId },
				changes,
				precondition: { kind: "blind" },
				result: resultMeta
			}
		)
		yield* rememberCommandRef(tenantId, requestKey, command.ref)
		const outcome: SubmitOutcome = yield* writer.submit(command, submitOptionsOf())
		yield* rememberSubmitOutcome(tenantId, requestKey, outcome)
		return outcome
	}
)

/**
 * Create a note. The outbox fact (kind "note-created") commits in the
 * SAME command, so the pending external effect exists exactly when the
 * note does.
 */
export const createNote = Effect.fn("commands.createNote")(
	function* (writer: Writer, tenantId: string, noteId: Uuid, text: string) {
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Note, [{ id: noteId, text, pinned: false }])
		yield* draft.insert(Outbox, [{ id: derivedId(noteId, "outbox:note-created"), note: noteId, kind: "note-created" }])
		const changes = yield* draft.finish()
		return yield* sealAndSubmit(writer, tenantId, noteId, changes, { note: noteId })
	},
	Effect.scoped
)

/**
 * Witnessed pin toggle: read the published row and its StateStamp under a
 * short scope, then submit exact-state. An intervening net change becomes
 * a durable precondition-failed receipt — a NEW intent (new request key)
 * is required to revise, never a silent overwrite.
 */
export const setPinned = Effect.fn("commands.setPinned")(
	function* (
		writer: Writer & Pick<History<typeof App>, "snapshot">,
		tenantId: string,
		requestKey: Uuid,
		noteId: Uuid,
		pinned: boolean
	) {
		const observed = yield* Effect.scoped(
			Effect.gen(function* () {
				const snapshot = yield* writer.snapshot({ consistency: { kind: "latest" } })
				const previous = yield* snapshot.get(NoteById, { id: noteId })
				return { previous, at: snapshot.stateStamp }
			})
		)
		if (observed.previous._tag === "None") {
			return { kind: "missing" } as const
		}
		const draft = yield* ChangeSet.builder(App)
		yield* draft.delete(Note, [observed.previous.value])
		yield* draft.insert(Note, [{ ...observed.previous.value, pinned }])
		const changes = yield* draft.finish()
		const requestId = yield* requestIdOf(requestKey)
		const command = yield* Command.seal(
			{
				scope: writer.identity,
				id: { receiptEpoch: writer.receiptEpoch, requestId },
				changes,
				precondition: { kind: "exact-state", at: observed.at },
				result: { note: noteId }
			}
		)
		yield* rememberCommandRef(tenantId, requestKey, command.ref)
		const outcome: SubmitOutcome = yield* writer.submit(command, submitOptionsOf())
		yield* rememberSubmitOutcome(tenantId, requestKey, outcome)
		return { kind: "submitted", outcome } as const
	},
	Effect.scoped
)

/**
 * Reference an ALREADY-UPLOADED immutable blob (OPS-003: blob first,
 * reference second). The caller uploads the content-addressed object and
 * passes its verified key/size; a crash before this command leaves an
 * orphan upload, never a dangling reference.
 */
export const addAttachment = Effect.fn("commands.addAttachment")(
	function* (
		writer: Writer,
		tenantId: string,
		noteId: Uuid,
		blob: { readonly key: string; readonly bytes: bigint }
	) {
		const attachmentId = derivedId(noteId, `attachment:${blob.key}`)
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Attachment, [{ id: attachmentId, note: noteId, key: blob.key, bytes: blob.bytes }])
		const changes = yield* draft.finish()
		return yield* sealAndSubmit(writer, tenantId, attachmentId, changes, { attachment: attachmentId, note: noteId })
	},
	Effect.scoped
)

/**
 * Resolve a retained command ref after interruption. Lost ack is not
 * proved loss — the original identity is the only recovery coordinate.
 */
export const resolveCommand = Effect.fn("commands.resolveCommand")(
	function* (writer: Writer, ref: CommandRef) {
		return yield* writer.resolve(ref)
	}
)

/**
 * Retire one dispatched outbox row. Blind delete — deleting an
 * already-deleted fact is a no-change receipt, which is exactly the
 * idempotency the dispatcher needs; the request key derives from the
 * outbox row id, so retries reuse the identical command.
 */
export const retireOutbox = Effect.fn("commands.retireOutbox")(
	function* (
		writer: Writer,
		tenantId: string,
		row: { readonly id: Uuid; readonly note: Uuid; readonly kind: string }
	) {
		const draft = yield* ChangeSet.builder(App)
		yield* draft.delete(Outbox, [row])
		const changes = yield* draft.finish()
		return yield* sealAndSubmit(writer, tenantId, derivedId(row.id, "outbox:retire"), changes, { outbox: row.id })
	},
	Effect.scoped
)
