# PRD 22 — Borrowed structs, borrowed params, and the named-schema typestate

**Depends on:** 20 (lands on the converged macro/api idioms; runs BEFORE 21 so
the bench pass cleans this surface's fallout instead of being re-churned by it).
**Owner-approved (2026-07-09), one macro-surface PRD:** three breaking changes
that touch the same emission code ship together. The axiom licenses the break
("compatibility is never a design input").
**Modules:** `crates/bumbledb-macros/src/lib.rs` (emission), `crates/bumbledb/src/api/`
(Db/WriteTx/Snapshot/PreparedQuery generics, bind surface, typed get/insert
paths), `crates/bumbledb/src/schema.rs` (the schema trait), everything downstream
that constructs facts or binds params (bench crate included), README example.
**Authority:** `70-api.md`, `10-data-model.md` (names live in the host),
`00-product.md` (representation over control flow; the surveyed-precedent style
of decision blocks).

## Item 1 — Borrowed structs (one decision, both directions)

The macro maps `str` → `String` and `bytes` → `Box<[u8]>`/`Vec<u8>` today, so a
host holding `&str` allocates pure ceremony: insert reads the owned field once
as a borrow (`intern_str_write(tx, &self.field)`) before the engine's arena copy
— which stays — and typed `get` allocates a fresh `String` per str field per
read out of the LMDB mmap, which callers compare and drop. Both allocations are
provably useless. Precedent: SQLite's `sqlite3_column_text` and LMDB's `get`
return borrows valid until the statement/txn ends as the *only* option, validity
stated in prose; here the lifetime parameter *is* that prose, compile-checked.

- **Emission:** `str` → `&'a str`, `bytes` → `&'a [u8]`; a struct with any
  variable-width field gains one lifetime (`Holder<'a>`); all-fixed-width
  structs stay lifetime-free (and `Copy` where they already derive it).
  Variable-width `as` newtypes wrap the borrowed form.
- **Insert:** takes the struct at any lifetime — encode paths already read the
  fields as borrows; nothing else changes. The README example becomes
  `name: "alice"` (no `.into()`), and the README is updated in this PRD.
- **Typed get:** returns views borrowed at the txn lifetime
  (`tx.get::<Account>(id) -> Result<Option<Account<'_>>>`; snapshot-side typed
  reads likewise). The resolve path must handle **both borrow sources**: the
  committed dictionary (mmap pages, txn-stable by LMDB CoW) and this
  transaction's pending interns (delta arena). UTF-8 validation at resolve as
  today (parse, don't validate).
- **Trait shape:** pick the minimal `Fact`-trait design that expresses "the
  struct is generic over a lifetime" — a lifetime-parameterized impl
  (`impl Fact for Account<'_>`) with decode returning the borrowed struct at the
  resolver's lifetime, or a GAT if impossible without one. Encode-side bounds
  must not force `'static`. Justify the choice in the module doc; do not build
  parallel owned twins (one option, not two — no modes).

## Item 2 — Borrowed params (the small sibling)

Bind-time `str`/`bytes` payloads by reference: the engine only hashes and probes
them (per-execution intern lookup), so owned payloads buy nothing. `ir::Value`
stays owned — IR literals are long-lived query data; only the **bind surface**
borrows (`ParamArg<'a>` gains borrowed variable-width payloads, or a
`BindValue<'a>` — follow whichever reads cleaner against PRD 20's converged bind
path). Warm re-bind allocates nothing host-side.

## Item 3 — Named-schema ZST + typestate

- **Grammar header:** `pub Ledger;` as the first item in `schema!` → the macro
  emits `pub struct Ledger;` implementing the schema trait (name it to avoid
  colliding with the sealed `schema::Schema` — e.g. `SchemaDef` with
  `fn descriptor() -> SchemaDescriptor`). Call sites become
  `Db::create(path, Ledger)` — a value you named, visible at the use site.
- **Deleted, not deprecated:** the magic `pub fn schema()`, any
  OnceLock/lazy-static plumbing, the panic-on-invalid-declaration (validation
  moves into `create`/`open` and surfaces as the typed `SchemaError` the error
  path already carries), and the one-invocation-per-module limit (multiple
  schemas per module now coexist — their ZSTs disambiguate).
- **Typestate:** `Db<S>` phantom generic, threaded through
  `WriteTx`/`Snapshot`/`PreparedQuery`; `Fact` gains `type Schema`; write/read
  operations bound `F: Fact<Schema = S>`. Inserting a schema-A struct into a
  schema-B database becomes a **compile error** — closing the real cross-schema
  `RelationId`-aliasing hole that today is caught only by a lucky width
  mismatch. Inference hides the parameter at call sites; the compile_fail
  doctest pins the cross-schema rejection.

## Passing criteria

- `[shape]` No `String`/owned-bytes field in any macro-emitted struct; no owned
  twins or modes; `schema()`/OnceLock/invocation-limit gone (grep); `Db`,
  `WriteTx`, `Snapshot`, `PreparedQuery` carry the phantom `S`; `ir::Value`
  unchanged.
- `[test]` Insert + typed get of a str-bearing relation performs zero
  host-visible allocations (counting-allocator harness around construct + insert
  and around get + compare; engine arena/delta copies sanctioned); get's borrows
  resolve correctly from BOTH sources (committed dict and same-txn pending
  intern — the read-your-writes string case).
- `[test]` Warm param re-bind with borrowed str payloads allocates zero on the
  host side (extend the existing bind tests).
- `[test]` compile_fail: schema-A fact into schema-B database; two schemas in
  one module coexist (positive test).
- `[test]` An invalid declaration surfaces as typed `SchemaError` from
  `Db::create` (no panic path remains).
- `[shape]` README example uses `name: "alice"` and `Db::create(path, Ledger)`.
- `[gate]` fmt/clippy/test workspace green; alloc gate + escalating variant
  unchanged; differential oracle untouched semantically (bench crate adapts to
  the new surface — construction sites only).

## Doc amendments (rule 5)

`70-api.md`: the schema! grammar gains the header; the transactions/facts
sections show borrowed structs and the typestate; the decision block (borrowed
variable-width types on the fact/param surfaces; ownership is an explicit host
act; alternative = the owned surface, why it lost = four ceremony allocations
serving no engine purpose and prose-stated validity where types can state it;
reverses-if = a real host profile shows `to_owned()` dominating).
`20-query-ir.md`: one sentence noting `ir::Value` stays owned by decision
(IR literals are long-lived data; only the bind surface borrows).
