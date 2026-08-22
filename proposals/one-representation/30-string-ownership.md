# 30 — String ownership: one copy per string, one probe per distinct string

Strings are the write path's variable-width population and the upstream
report demands their measurement and ownership story separately. Today one
string cell can be copied three times and probed N times (V5). This
document pins the target: **one copy** (JS → the collection's arena) and
**one dictionary probe per distinct string per transaction**, with the
engine's dictionary semantics — ids, equality, flush discipline —
byte-identically unchanged.

## Baseline (what one string cell costs today)

1. **Copy 1:** NAPI UTF-8 read into a fresh `String`
   (`schema_value`, `ts/crate/src/marshal.rs`).
2. **Copy 2:** `String → Box<str>` into `Value::String`.
3. **Probe, per occurrence:** `WriteDelta::intern`
   (`crates/bumbledb/src/storage/delta/intern.rs`) checks the pending map
   first — so a string **minted this transaction** is memoized — but a
   string already **committed** misses pending and pays
   `dict::forward_key` = blake3(bytes) + one LMDB B-tree get on **every
   occurrence**. Against Primer's 1.68 GB store with a Zipf vocabulary,
   that is millions of redundant probes per run.
4. **Copy 3, per distinct novel string:** `PendingInterns::insert` boxes
   the bytes again (`Box::from(raw)`).

The builder lane hides cost 3 (a fresh build's strings are all pending →
memoized) and keeps costs 1, 2, 4. The delta lane against an existing
store pays all four. Both lanes are measured separately in 10
(component 4, `INTERN_PROBE`).

## Target representation

- **One arena.** The accepted collection owns one string arena (one
  growable UTF-8 buffer); cells reference `(offset, len)` spans. Copy 2's
  `Box<str>` (`Value::String` boxing on the collection lane) disappears.
  Copies per occurrence AS BUILT: **2** — the safe NAPI surface has no
  read-into-buffer, so a string cell crosses with ONE transient NAPI
  `String` (`Unknown::cast::<String>`), then one copy into the arena
  (`push_str`). The sys-level single copy —
  `napi_get_value_string_utf8` written directly into the arena — is the
  NAMED FOLLOW-UP, not a claim this build makes. Well-formedness stays
  where it is judged today — the JS marshal seam (`cellOf`'s
  `isWellFormed` refusal), the one seam every write crosses.
- **One probe per distinct string per transaction.** `WriteDelta` gains a
  committed-hit memo beside `PendingInterns`:
  `HashMap<[u8; 32], InternId>` keyed by the blake3 of the string bytes.
  The lookup order becomes: pending map → committed memo → committed dict
  (blake3 once per occurrence — it was already computed for
  `forward_key` on every committed probe today — then one LMDB get only
  on first sight, memo insert on hit). Keying by the hash instead of the
  bytes is deliberate: zero byte copies, and hash-equality-as-identity is
  already the storage law for facts ("hash equality *is* fact equality",
  the collision axiom in `storage/delta.rs`); the dictionary's own forward
  key trusts the same 32 bytes.
- **Pending mints keep their owned copy.** `PendingInterns`' `Box<[u8]>`
  per novel string stays: those bytes ARE the commit-time flush
  representation (`flush_counters` writes them to `_dict`), bounded by the
  novel population. One owned form with one consumer is not waste.

## What must not move (the laws)

- **Law 6 — exact string equality.** Interning equality remains byte
  equality through the same forward map. The memo is a cache of *reads*
  proved stable by the single-writer discipline (the committed dictionary
  is frozen for the transaction's lifetime — the same argument
  `intern.rs` already makes for the pending-first order). It can never
  witness a value the dict would not.
- **Batch-locality.** Arena spans and memo entries are transaction-local
  transport. They are dropped with the delta and are **never** a database
  identifier — the upstream report's explicit line. Nothing about them is
  serialized, fingerprinted, or observable through any read surface.
- **Id discipline.** Mint order, next-id advancement, the
  dropped-delta-recycles-provisional-ids law, and the no-GC leak-by-design
  ruling are untouched — the memo sits strictly on the read side of
  `resolve`.
- **Rejection decode.** `pending_raw`'s linear scan and the
  violation-decoration path are cold and unchanged.

## Gate and bound (from 10, G2)

The memo must win on the delta lane (component 4 wall time and LMDB get
count down, nothing else up). Its live memory is bounded by distinct
committed strings *touched by the transaction* — for a pathological
transaction touching tens of millions of distinct committed strings, the
memo's 40 bytes/entry could matter; G2 pins the observed envelope on the
primerlane and, only if that envelope is exceeded in practice, a capacity
bound (plain LRU is forbidden — an eviction policy is a mode; the bound,
if ever needed, is a documented fixed capacity with miss-through, chosen
by measurement and pinned in this file by amendment).

## Acceptance (this doc's share)

- Allocation census (10): string-cell copies per occurrence = 2 (one
  transient NAPI `String`, one arena copy — down from 3; the safe NAPI
  surface has no read-into-buffer, so the sys-level
  `napi_get_value_string_utf8`-into-arena single copy is recorded as the
  named follow-up, never silently claimed); dictionary LMDB gets per
  distinct committed string per transaction = 1.
- A pinned engine test: interning a committed string twice in one
  transaction issues one catalog probe (observable via `INTERN_PROBE`'s
  probes/hits args); ids returned are identical to today's byte-for-byte.
- Primer digests unchanged (interning is invisible to fact bytes — any
  drift here is a stop-ship, 80).
