/**
 * The CREATING theory of the committed legacy-store fixture
 * (`test/fixtures/legacy-store`) — the schema the store at that directory
 * was created under, mirrored exactly so a fingerprint-matching `Db.open`
 * adopts it (back-fills the descriptor).
 *
 * Fixture provenance: regenerated 2026-07-24 at storage format v6 (the
 * R16 format bump orphaned the original v5 store, which was created by
 * the last pre-descriptor engine at commit 2ac52712) — a throwaway
 * generator created the store through this SDK under this identical
 * theory, committed one `Doc` row plus one `Tagged` row, and a raw-LMDB
 * step then deleted the persisted `_meta` descriptor key (the same
 * surgery as the engine's test-only `strip_schema_descriptor_for_tests`),
 * reproducing the exact on-disk shape of a pre-descriptor store at the
 * current format:
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
