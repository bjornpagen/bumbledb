/**
 * The complete example repo history handed to P13 (chapter 33's Drizzle-like
 * authorship flow, staged): one app schema evolving from nothing through
 * field/backfill/seed/rename evolution, each stage being exactly what the
 * app's `src/db/schema.ts` would export at that commit. The flow test
 * (`migrations-flow.test.ts`) generates the full four-plan repo history from
 * these stages and asserts every artifact; P13 regenerates the same history
 * with the production codec once the native entrypoints are wired (real
 * digests replace the scripted ones — the authored source is identical).
 *
 * Stage 0 → `0000-initialize`         create Note, seed nothing
 * Stage 1 → `0001-note-pinned`        add Note.pinned with backfill(false)
 *                                     (chapter 33's worked example)
 * Stage 2 → `0002-create-tag-seed-tag` new Tag relation with declarative seeds
 * Stage 3 → `0003-note`               rename Note.body → Note.text
 *
 * Intent expressions are the core `ScalarExpr` data roster (C01); the
 * `literal` spelling below is the recorded C01 assumption in
 * implementation/packets/P10.md until P07's constructor export lands.
 */
import { bool, key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import { backfill, migrationIntent, renameField, seed } from "#migrations/intent.ts"

// --- Stage 0: the initial application schema -------------------------------

export const Note0 = relation("Note", { id: u64, body: str })
export const App0 = schema("App", { Note: Note0 }, [key(Note0, ["id"])])

// --- Stage 1: add a required field; backfill is declared, never guessed ----

export const Note1 = relation("Note", { id: u64, body: str, pinned: bool })
export const App1 = schema("App", { Note: Note1 }, [key(Note1, ["id"])])

/** The typed literal AST (core ScalarExpr spelling), not a callback. */
export const pinnedDefault = { kind: "literal", value: { bool: false } } as const

export const evolution1 = migrationIntent(App1, [backfill(Note1, "pinned", pinnedDefault)])

// --- Stage 2: a new relation with declarative seed rows --------------------

export const Tag2 = relation("Tag", { id: u64, name: str })
export const Note2 = Note1
export const App2 = schema("App", { Note: Note2, Tag: Tag2 }, [key(Note2, ["id"]), key(Tag2, ["id"])])

export const tagSeeds = [
	{ id: 1n, name: "inbox" },
	{ id: 2n, name: "archive" }
] as const

export const evolution2 = migrationIntent(App2, [seed(Tag2, tagSeeds)])

// --- Stage 3: a field rename requires explicit identity intent -------------

export const Note3 = relation("Note", { id: u64, text: str, pinned: bool })
export const Tag3 = Tag2
export const App3 = schema("App", { Note: Note3, Tag: Tag3 }, [key(Note3, ["id"]), key(Tag3, ["id"])])

export const evolution3 = migrationIntent(App3, [renameField(Note3, "body", "text")])
