# PRD 05 — The Interning Dictionary

Authority: `docs/architecture/10-data-model.md` (interning section: one global dict,
tag segregation, UTF-8 at intern, collision axiom, read-path miss = empty, no GC).

## Purpose

String/Bytes interning: the only variable-length data in the system, reduced to 8-byte
ids at the fact layer.

## Technical direction

- `storage::dict`, over the `_dict` DB. Forward: `blake3(tag_byte ‖ raw_bytes)`
  (full 32-byte key) → `id: u64`. Reverse: `id` → `tag_byte ‖ raw_bytes`. Tag byte:
  0 = String, 1 = Bytes — same raw bytes, different tags, different ids.
- Next-id counter lives in `_meta` (extend PRD 04's meta keys); ids monotonic, never
  reused; interning happens only inside a write transaction, and the id counter joins
  the in-memory-then-flush set in PRD 06 (this PRD may read-modify-write directly; PRD
  06 rehomes the counter — leave a doc-comment marker).
- `intern_str(&WriteTxn-ish, &str) -> u64` (UTF-8 by type — the `&str` boundary IS the
  validation; a `&[u8]` string entry point must not exist), `intern_bytes(...)`;
  **collision axiom**: a forward hit returns the existing id with no verification
  (documented at the call site, citing the doc).
- Read path: `lookup_str(&ReadTxn, &str) -> Option<u64>`, `lookup_bytes(...)` —
  read-only gets; `resolve(id) -> &[u8]` (borrowed from LMDB page, transaction-scoped
  lifetime) returning `Corruption` on a dangling id — never a skip.

## Non-goals

Decode-to-result-buffer conveniences (PRD 25). GC (never).

## Passing criteria

- Unit tests: intern twice → same id; same bytes as String and Bytes → different ids;
  lookup of never-interned value → None; resolve round-trips; resolve of a fabricated
  id → Corruption error; ids strictly increase across interns; an aborted write
  transaction leaves no dictionary entries (LMDB atomicity observed at this layer).
- Global commands green.
