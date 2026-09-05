/**
 * The staged evolution history behind `bumbledb/migrations/` — each stage
 * is exactly what `src/db/schema.ts` exported at that commit, retained so
 * the whole generated chain is reproducible from source
 * (`scripts/generate-history.ts` replays the stages through the real
 * generator). This mirrors the log package's recorded example history
 * (`ts-log/test/migrations-example.ts` — the P10 handoff), moved onto
 * application-owned `Id128` identity.
 *
 * Stage 0 → `0000-initialize`          create Note, no seeds
 * Stage 1 → `0001-note-pinned`         add Note.pinned, backfill(false)
 * Stage 2 → `0002-create-tag-seed-tag` new Tag relation, declarative seeds
 *                                      with EXPLICITLY supplied Id128s
 * Stage 3 → `0003-note-text`           rename Note.body → Note.text
 * Stage 4 → `0004-outbox-attachment`   new empty Outbox + Attachment
 *                                      relations (auto-inferred; no intent)
 *
 * In an ordinary repo only the CURRENT stage lives in schema.ts and the
 * intent is deleted once its plan is recorded; the stage roster here is
 * the reproducibility fixture, not an app import.
 *
 * Field-arithmetic convert
 * (`Scalar.add(Scalar.field("units"), Scalar.u64(1n))`) lives on the
 * Learning packed consumer so this App schema stays intact.
 */
import { bool, contained, Id128, id128, key, on, relation, Scalar, schema, str, u64 } from "@bjornpagen/bumbledb"
import { backfill, migrationIntent, renameField, seed } from "@bjornpagen/bumbledb-log/schema"
import { Result } from "effect"

function idOf(hex: string): Id128 {
	const parsed = Id128.fromHex(hex)
	if (Result.isFailure(parsed)) {
		throw new Error(`evolution-stages: invalid Id128 literal ${hex}`)
	}
	return parsed.success
}

// --- Stage 0: the initial application schema -------------------------------

export const Note0 = relation("Note", { id: id128, body: str })
export const App0 = schema("App", { Note: Note0 }, [key(Note0, ["id"])])

// --- Stage 1: add a required field; backfill is declared, never guessed ----

export const Note1 = relation("Note", { id: id128, body: str, pinned: bool })
export const App1 = schema("App", { Note: Note1 }, [key(Note1, ["id"])])

export const evolution1 = migrationIntent(App1, [backfill(Note1, "pinned", Scalar.bool(false))])

// --- Stage 2: a new relation with declarative seed rows ---------------------
// Seed identity is EXPLICIT application-owned Id128 data: a migration can
// never call an ID generator per row.

export const Tag2 = relation("Tag", { id: id128, name: str })
export const Note2 = Note1
export const App2 = schema("App", { Note: Note2, Tag: Tag2 }, [key(Note2, ["id"]), key(Tag2, ["id"])])

export const tagSeeds = [
	{ id: idOf("00000000000000000000000000000001"), name: "inbox" },
	{ id: idOf("00000000000000000000000000000002"), name: "archive" }
] as const

export const evolution2 = migrationIntent(App2, [seed(Tag2, tagSeeds)])

// --- Stage 3: a field rename requires explicit identity intent -------------

export const Note3 = relation("Note", { id: id128, text: str, pinned: bool })
export const Tag3 = Tag2
export const App3 = schema("App", { Note: Note3, Tag: Tag3 }, [key(Note3, ["id"]), key(Tag3, ["id"])])

export const evolution3 = migrationIntent(App3, [renameField(Note3, "body", "text")])

// --- Stage 4: new empty relations are inferred automatically ---------------
// (the current stage; identical to src/db/schema.ts)

export const Note4 = Note3
export const Tag4 = Tag3
export const Outbox4 = relation("Outbox", { id: id128, note: id128, kind: str })
export const Attachment4 = relation("Attachment", {
	id: id128,
	note: id128,
	key: str,
	bytes: u64
})
export const App4 = schema("App", { Note: Note4, Tag: Tag4, Outbox: Outbox4, Attachment: Attachment4 }, [
	key(Note4, ["id"]),
	key(Tag4, ["id"]),
	key(Outbox4, ["id"]),
	key(Attachment4, ["id"]),
	contained(on(Attachment4, "note"), on(Note4, "id"))
])

/** The ordered generation inputs `scripts/generate-history.ts` replays. */
export const stages = [
	{ label: "initialize", schema: App0, intent: undefined },
	{ label: "note-pinned", schema: App1, intent: evolution1 },
	{ label: "create-tag-seed-tag", schema: App2, intent: evolution2 },
	{ label: "note-text", schema: App3, intent: evolution3 },
	{ label: "outbox-attachment", schema: App4, intent: undefined }
] as const
