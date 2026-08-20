# 23 — `_meta` collapses to four keys; `EnvMode` deletes; open is one straight line

- **Status:** **fixed this pass** — `_meta` is format/fingerprint/generation/dict-next; `EnvMode` deleted into `lock: File`; tests: `parse_meta_reads_four_keys`, `every_prefix_before_rename_hides_the_destination`, `prefix_at_or_after_rename_is_openable_format_8`.
- **Severity:** purge convergence — the prize the two purges expose.

## Principle

Insight 4 by subtraction: every deleted state deletes its guards. With the
kind byte (20) and the persisted descriptor (21) gone, the store's metadata
is exactly what open must verify and writes must advance — nothing else.

## The shape

- `_meta` roster: **format, fingerprint, generation, dict-next** — four
  keys. `MetaKey` table and `StoreMeta` shrink to match; `parse_meta` loses
  its hardest prongs.
- `EnvMode` deletes as an enum: the durable lock is the only arm left, so
  `Environment` holds a plain `lock: std::fs::File` field. The
  two-phase-fill hazard class is not just fixed but unrepresentable.
- Open = *version → fingerprint → go*. One sequence, one caller, no
  precedence law needed because there is nothing left to order.
- Versioning (owner-ruled): the format constant **stays 8, roster revised
  in place** — 0.15.0 is unpublished; format-8 stores created this week
  rebuild from source, which is the no-migration law working as written.
  The format ledger in `env.rs` records "roster revised pre-publish" on the
  v8 row. `bdb_abi_version()` stays 3, revised in place under the same
  pre-publish rule.

## Acceptance

- A `_meta` roster test pins exactly four keys; `parse_meta` returns the
  four-field `StoreMeta`.
- `grep "enum EnvMode"` is empty.
- Publish prefix crash matrix green over the revised roster.
- All suites green.

## Adjudication

`ReadTxn::schema_descriptor` still reads retired key `[5]` so
`verify_store`'s descriptor↔fingerprint pass compiles (that file is not
this lane). New stores do not write the key; the pass no-ops. Docs and
proposals still name `EnvMode` (lane E).
