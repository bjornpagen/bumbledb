# 05 — `WriteDelta` still borrows its schema

- **Status:** **keep** — schema on the delta is the LMDB-view borrow the
  wrap does not own. Lifetime-free `WriteDelta` is hundreds of
  `WriteDelta::new(&schema)` sites, not a discarded proof. Brooks:
  `MutationCore` already owns `Arc<Schema>`; collapsing the delta's
  `&Schema` now would invent a second construction protocol.
- **Severity:** should-fix.
- **Supersedes:** PROP-003, SPINE-11.
- **Adjudication (third pass): keep ACCEPTED.** The cost argument is
  Insight 15 applied honestly, and the hazard shrank with the wrap: the
  delta's `&Schema` now borrows the `Db`'s schema, not a sibling field of
  its own owner, so no self-reference is representable. Horizon recorded:
  if a schema-owning struct ever embeds the delta, this reopens as a
  blocker.

## Principle

One owner per fact (Insight 4's duplicated-discriminant corollary). The
schema now lives on `MutationCore` (`Arc<Schema>`); the delta carrying its
own `&'s Schema` is a second copy of one truth, and the lifetime it drags
through `WriteTx<'a>`, `plan_commit`, `FinalStateView`, and the arena-borrowed
op slices is the self-reference hazard the proposal named.

## Evidence

- `crates/bumbledb/src/storage/delta.rs:144-145` —
  `pub struct WriteDelta<'s> { schema: &'s Schema, arena: Arena, … }`.
- Proposal §Durable mutation: "`WriteDelta` drops its borrowed schema field;
  operations receive `&Schema` from `MutationCore`. This makes the delta
  lifetime-free and avoids a self-reference in the owning builder."

## The fix

1. Delete the `schema` field; `WriteDelta` becomes lifetime-free (`'s`
   disappears from the struct; op-slice lifetimes stay tied to the arena,
   which the delta owns).
2. Every delta operation that read `self.schema` takes `&Schema` as a
   parameter; `MutationCore` (the one owner) passes it at each call site —
   the call sites already have `self.schema` in scope.
3. `WriteTx<'a>`'s remaining `'a` is then honestly the LMDB view lifetime
   only; `into_store` returns `(ReadTxn<'a>, WriteDelta)`.

## Acceptance

- `WriteDelta` has no lifetime parameter and no schema field.
- `plan_commit(&delta, &selections)`-shaped call sites take the schema from
  exactly one place (`Selections`/`MutationCore`), never from the delta.
- Workspace tests green; the delta's net-disposition and intern tests
  unchanged (representation move, zero behavior).
