# PRD 27 — The Schema Macro

Authority: `docs/architecture/10-data-model.md` (schema in Rust, host newtypes),
`60-api.md` (facts as generated structs, host-side sugar).

## Purpose

The declarative schema surface: one macro invocation generates descriptors, typed fact
structs, newtypes, and the encode boundary.

## Technical direction

- `bumbledb-macros` (proc-macro crate from PRD 00). A `schema! { ... }` macro with a
  small, rigid grammar (this is Rust-side declaration, not a query language):

  ```rust
  schema! {
      relation Account {
          id:     u64 as AccountId, serial, unique,   // `serial` implies unique; writing it is optional
          holder: u64 as HolderId,  fk(Holder.id),
          status: enum Status { Active, Closed },
      }
      relation Holder { id: u64 as HolderId, serial, name: str }
  }
  ```

  Types: `bool`, `u64`, `i64`, `str`, `bytes`, inline `enum Name { ... }` (the Name
  names the generated Rust enum only — engine identity is the variant list,
  structural); `as NewType` generates `struct AccountId(pub u64)` newtypes (the
  nominal layer, host-side only); `unique(...)`/`fk(Rel.field or Rel.constraint)`
  clauses for compound constraints.
- Expansion: a `fn schema() -> Schema` calling PRD 02's validated constructor (macro
  does **no** validation logic of its own — errors surface as PRD 02's typed errors
  via a compile-fail `const` evaluation where feasible, else at first runtime call;
  keep it simple: runtime construction, memoized in a `OnceLock`); per relation a
  `struct Account { pub id: AccountId, ... }` with `fn encode(&self, &Schema, &mut
  Vec<u8>)` to canonical fact bytes and `fn decode(...)`; generated Rust enums map
  variant ↔ ordinal.
- Interning at the encode boundary: generated structs hold `&str`/`String` fields;
  encoding against a write context interns; against a read context looks up (miss ⇒
  the caller's concern). Provide both entry points; keep the generated code thin —
  all real logic lives in the library crate (declaration resolution in
  `schema::runtime`, context helpers in `api::db::plumbing`, both re-exported for the
  expansion through the `#[doc(hidden)] bumbledb::__private` module — PRD 28's surface
  trim leaves nothing else public for generated code to call), the macro emits calls,
  not logic.

## Non-goals

Query macros. Migrations of any kind (never). Attribute-macro alternatives.

## Passing criteria

- Unit tests (in a test consumer crate or integration test): the example schema
  expands, constructs, and fingerprints identically to a hand-built PRD 02 descriptor
  of the same declaration (the equivalence test — macro output is *exactly* sugar);
  newtypes are distinct Rust types (a compile-fail doctest mixing AccountId/HolderId);
  round-trip struct → fact_bytes → struct incl. enum and string fields; `serial`
  field generates the auto-unique (visible in descriptor); compound unique + fk
  clauses land correctly.
- `trybuild`-style compile-fail tests are optional; the doctest compile-fail for
  newtype confusion is required.
- Global commands green.
