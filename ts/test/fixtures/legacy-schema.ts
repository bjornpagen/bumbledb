/**
 * The CREATING theory of the committed legacy-store fixture
 * (`test/fixtures/legacy-store`) — the schema the store at that directory
 * was created under, mirrored exactly so a fingerprint-matching `Db.open`
 * adopts it (back-fills the descriptor).
 *
 * Fixture provenance: the store was generated ONCE against the bumbledb
 * engine at commit 2ac52712 — the last commit BEFORE self-describing
 * stores (c79c2b38) — by a throwaway generator declaring this identical
 * theory through the engine's `schema!` macro (storage format v5 on both
 * sides; macro- and spec-declared identical theories fingerprint equal,
 * pinned by the engine's `tests/schema_spec.rs`), creating the store,
 * and committing one `Doc` row plus one `Tagged` row:
 *
 *   relation Doc    { id: u64 as DocId, fresh, title: str }
 *   relation Tagged { doc: u64 as DocId, tag: str }
 *   Tagged(doc) <= Doc(id);
 *
 *   Doc    { id: <minted 0>, title: "the record outlives the schema" }
 *   Tagged { doc: <that id>, tag: "legacy" }
 *
 * The committed directory holds `data.mdb` only: LMDB's `lock.mdb` reader
 * table and the empty `bumbledb.lock` advisory file are per-open artifacts
 * the engine recreates, so they are stripped from the fixture.
 */

import { contained, on, relation, schema, str, u64 } from "#index.ts"

const Doc = relation("Doc", { id: u64.fresh, title: str })
const Tagged = relation("Tagged", { doc: u64, tag: str })

const legacySchema = schema("Legacy", { Doc, Tagged }, [contained(on(Tagged, "doc"), on(Doc, "id"))])

export { Doc, legacySchema, Tagged }
