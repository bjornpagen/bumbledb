# 70 — Embedding API

The host-facing surface. Guiding rule: the API is plain data in, plain data out —
builders/macros are host-side sugar, never the contract (`20-query-ir.md`). The one
exception with teeth is the `schema!` macro, whose grammar is normative here because
the schema is compiled into the binary (`10-data-model.md`).

## The two surfaces — theory and data

The code/data boundary is logic's own. A schema is the **theory**: signature plus
axioms, fixed at build time, type-providing — which is why `schema!` is
structurally forced (type providers cannot live in expression position) and why it
is Rust's alone. A query is a **sentence in** the theory: a runtime object,
constructed and evaluated — data, in whatever language the host speaks
(`20-query-ir.md`, the surface ruling). The asymmetry is not an ergonomics
compromise; it is the line logic draws between a theory and its formulas. The
notation reflects it: the query notation is the statement grammar's query side,
promoted (`20-query-ir.md` § the query notation).

## The `schema!` grammar (normative)

The invocation's first item is the **header** `pub Name;` — it names the schema.
Then two statement kinds, in any order: **relation blocks** and
**dependency statements** (`30-dependencies.md` owns their semantics; this section
owns the surface).

```rust
bumbledb::schema! {
    pub Ledger;

    closed relation Kind as KindId = { Checking, Savings };

    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        active: interval<i64> as ActiveDuring,
    }

    Account(id) -> Account;                                  // redundant here (fresh implies it) — and rejected as a duplicate
    Account(holder) <= Holder(id);
    Account(id | kind == Savings) == SavingsTerms(account);
}
```

- **Field syntax:** `name: type` with optional `as NewType` and optional `fresh`.
  Types: `bool`, `u64`, `i64`, `str`, `bytes<N>` (N ∈ 1..=64 — the width is
  mandatory and part of the type; bare `bytes` does not parse),
  `interval<i64>`, `interval<u64>`, and the fixed-width family
  `interval<u64, w>` / `interval<i64, w>` (w ≥ 1 an integer literal — the
  width is the type and the encoding stores only the start,
  `10-data-model.md` § the admission rule; `interval<u64, 0>` and the
  trailing-comma `interval<u64, >` are expansion errors naming the field);
  the inline `enum`
  production is deleted vocabulary (a vocabulary is a closed relation, and
  the word diagnoses its own replacement at expansion). `as` is legal on u64, i64,
  `bytes<N>`, and intervals (the newtype wraps the engine value; rustc polices
  domains — `10-data-model.md`; bytes and interval newtypes derive no order —
  both refusals are semantics, `10-data-model.md`). A fixed-width field's
  host type is the same checked `Interval<T>`; the typed write boundary
  checks the declared width (a wide or narrow value is a typed shape
  error — wide values are unrepresentable at the type, never stored).
  `fresh` is legal on `u64` only and auto-materializes `R(field) -> R`.
  **There are no field-level constraint modifiers** — no `unique`, no `fk(...)`;
  those words do not parse.
- **Closed relations** (`10-data-model.md` § closed relations) declare their
  extension in the schema — two tiers, one production:

  ```rust
  closed relation Status as StatusId = { Open, Frozen, Closed };

  closed relation Kind as KindId {
      mastered: bool,
  } = {
      DirectPass { mastered: true },
      Failed     { mastered: false },
  };
  ```

  `closed` is a leading keyword on the `relation` production; `as NewType` is
  **required** (the handle needs a host type); the column block is optional;
  the `= { … }` extension block is required and non-empty. Each ground axiom is
  `Handle` or `Handle { column: literal, … }` with every declared column
  present exactly once — duplicate handles, missing/extra/duplicate columns,
  and type-mismatched literals are expansion errors naming the offender. Ground-axiom
  literals ride the selection-literal machine (same typing, same errors). In
  statement selections a bare handle is legal on any field whose newtype is a
  closed relation's handle newtype (`| status == Frozen`), resolving to the
  handle's declaration-order id at expansion exactly as field names
  resolve to ordinals; a handle on any other field is an expansion error.

  **The emission per closed relation:** the **host enum** (`pub enum Status {
  Open, Frozen, Closed }`) — an *emission, not a type*: the engine's
  vocabulary is relational, and the macro projects it into a Rust enum so
  rustc's pattern checking keeps working — one vocabulary, two checkers, zero
  drift. The weld is `const fn id(self) -> StatusId` and `const fn
  from_id(StatusId) -> Option<Status>` (explicit matches, usable in const
  contexts), and a **weld test is emitted per closed relation** under
  `#[cfg(test)]` (`from_id(h.id()) == Some(h)` for every handle, plus the
  beyond-roster miss), so the weld cannot be forgotten for a new theory. The
  handle newtype (`StatusId(pub u64)`) comes through the ordinary newtype
  machinery; the id constants address the sealed shape (the synthetic `id`
  field at `FieldId(0)`, declared columns shifted). The host enum is the
  constant namespace — no separate per-handle constants exist. **No fact
  struct and no `Fact` impl are emitted** — closed relations are unwritable,
  and a writable struct would be a lie the type system tells; reads go
  through queries and the dyn surface.
- **Dependency statements:** `Rel(fields...) -> Rel;` (FD, key form only),
  `A(fields... | field == Literal, ...) <= B(fields...);` (containment),
  `==` for bidirectional,
  and `B(fields... | ...) <={lo..hi} A(fields... | ...);` (the cardinality
  window — B-family, target-left: the LEFT side is the window's target, the
  per-group parent; the right side is counted. Bounds are non-negative
  integers, `*` for no ceiling; `{n}` is THE exact-count spelling and `{0}`
  the exclusion — the full spelling law is below).
  Projection lists are positional between the two sides;
  selections follow `|` as comma-separated `field == literal` pairs, or
  `field == {A, B}` for a literal-set binding (read disjunctively; a
  one-element set is the bare literal and `{}` does not parse — both are
  expansion errors naming the canonical form); literals are
  closed-relation handles, integer literals, `true`/`false`, string/byte literals, and
  `start..end` interval literals (half-open). The macro emits descriptors directly —
  relation/field names resolve to declaration-order ids at expansion time, so an
  unresolvable name is a compile error naming the relation and field — and
  performs no semantic validation beyond parse shape, name-to-id resolution, and
  the canonical-utterance law's ban table (each a compile-time error at the call
  site): the schema validation boundary (`30-dependencies.md` roster) is the
  judge, and everything semantic beyond names is a normal typed error with the
  statement rendered back.

**The canonical-utterance law** (owner-ruled 2026-07-15, the freeze's statement
surface): **any single statement with two grammatical spellings is an expansion
error naming the canonical form** — one meaning, one spelling. The rationale is
operational, not aesthetic: greps are total (every window is `<={`, every
exclusion is `{0}` — no disguise survives to be missed), the renderer is a
bijection on legal statements (errors cite statements in exactly the spelling
the author can paste back), and the duplicate-statement machinery never faces
two spellings of one judgment. The window ban table, each error naming the
canonical form:

| banned                    | error names                                                       |
| ------------------------- | ----------------------------------------------------------------- |
| `X <={1..*} Y`            | drop the annotation — write `X <= Y`                              |
| `X <={n..n} Y`            | an exact count is written `{n}`                                   |
| `X <={0..0} Y`            | the exclusion is written `{0}`                                    |
| `X <={0..*} Y`            | vacuous — provably says nothing (`cardinality_zero_star`); delete |
| `X <={hi..lo} Y`, hi > lo | inverted, unsatisfiable                                           |
| `f == {A}`                | a one-element set is the bare literal `f == A`                    |
| `{..hi}` / `{lo..}`       | never admitted — bounds are always explicit                       |

The legal survivors, each otherwise unrepresentable: `{n}` exact, `{lo..hi}`
with lo < hi, `{lo..*}` floors (lo ≥ 2), `{0..hi}` ceilings, `{0}` exclusion.
The same law binds the descriptor API at validation
(`CardinalityInvertedWindow` / `CardinalityVacuousWindow` /
`CardinalityContainmentWindow`, `DegenerateSelectionSet` — a sealed schema
holds canonical statements only, so the renderer emits canonical spellings
only). **`==` survives** as a definitional abbreviation (the `fresh`
precedent: an abbreviation whose expansion IS its definition lives; a synonym
dies): `==` IS the two adjacent containments it lowers to (`A <= B` first),
the renderer prints the pair as `==` once, and separate-direction `<=` lines
stay legal (they are two statements, not one utterance). The compile-fail
suite (`crates/bumbledb/tests/schema-compile-fail/`) pins every ban's
diagnostic.
- **The header** `pub Ledger;` expands to `pub struct Ledger;` implementing the
  `Theory` trait (`fn descriptor(self) -> SchemaDescriptor`) — the value the `Db`
  functions take (`Db::create(path, Ledger)`) and the typestate `Db<Ledger>` carries.
  Multiple schemas coexist in one module; their headers disambiguate. There is no
  memoized `schema()` constructor and no panic path: semantic validation runs inside
  `Db::create`/`Db::open` and surfaces as the typed `SchemaError`.
- The macro generates: the header's `Theory` unit struct, relation descriptors,
  dependency statement descriptors, the host newtypes, and per-relation fact structs
  (`Account { id, holder, kind, active }`). **The one variable-width field kind is
  borrowed**: `str` → `&'a str` — a struct with any `str` field gains one lifetime.
  `bytes<N>` → `[u8; N]`: owned, `Copy`, borrow-free (the fixed-width law), so
  all-fixed-width structs stay lifetime-free.

**Decision: the macro surface is the algebra, with no sugar keywords.** Owner ruling
(`30-dependencies.md` records the alternative and its loss). The macro
remains hand-rolled (no syn/quote — the dependency policy, `00-product.md`).

**Decision: the `schema!` grammar is OPEN-ENDED — owner-evolvable, forever**
(owner-ruled 2026-07-10). This is a research database: the dependency calculus is
not done growing (richer statement forms, deeper selections, whatever the theory
needs next), and compatibility is never a design input (`00-product.md`), so the
grammar changes whenever the design improves — the fingerprint makes every
grammar-visible change a new theory, and ETL is the story, exactly as for any other
break. Grammar growth is governed by the **acceptance gate**
(`30-dependencies.md`), not by stability promises: a statement form enters when it
carries an enforcement plan, and by nothing else. The one boundary that holds is
categorical, not temporal: **the macro speaks the theory language — schema and
statements, whatever dependency theory grows into — and never the query language.**
Statements are code; queries are data; that line does not move even as everything
on the theory side of it does. The descriptor path (`SchemaDescriptor` implementing
`Theory`) is the *data* schema surface — the bench crate, the oracle, and any
binding that needs runtime schemas — and its named-plain-data root,
`SchemaSpec`, is frozen as the bindings contract (§ the SchemaSpec bindings
contract below).

## Id constants and the manifest — named data, not ergonomics

The macro emits **declaration-order id constants on the theory**: per relation
(`Ledger::ACCOUNT: RelationId`), per field (`Ledger::ACCOUNT_KIND: FieldId`) —
handles need no constants (the host enum is their namespace,
`Kind::Savings.id()`), names converted to
`SCREAMING_SNAKE` with a collision diagnosed at expansion naming both claimants. The
Rust host never writes a magic number into an `ir::Query` — and a downstream
`query!` macro checks its names through ordinary rustc resolution by emitting paths
to these constants (proc macros cannot see each other's output; the constants are
how a typo'd relation becomes a compile error).

The theory renders a **manifest** (`Theory::manifest()` → `schema::Manifest`): every
name → id pairing as a plain Rust value straight off the descriptor — relations and
fields with their ids stated explicitly, each field's structural type, each
closed relation's **extension table**
(relation → handle → declaration-order id → (column, value) pairs), and the
**statement table** (materialized statement id → form tag (`StatementKind`) →
canonical spelling, through the one renderer), so foreign
surfaces see the vocabulary — and can cite any statement id a rejection or
diagnostic names — without touching Rust. A foreign host gets the same
numbers as data. No serde anywhere (the dependency law): a downstream binding
serializes the value however it likes; the engine never learns the wire format.
Both are emission; the grammar is untouched.

## The SchemaSpec bindings contract (normative)

The runtime descriptor path is public, complete, and FROZEN as the bindings
contract: `schema::SchemaSpec` is the schema as **named plain data** — owned
strings, vectors, integers, and the shared `Value` sum; no serde, no wire
format (a bindings crate marshals it however it likes; the engine never learns
the encoding). The shape mirrors the grammar one-for-one:

- **Relations** (`RelationSpec`): name, fields (`FieldSpec`: name, structural
  `ValueType`, optional host-newtype name, `fresh` mark), and an optional
  extension (`RowSpec` ground axioms — the option is the closedness, both
  tiers through one shape). Newtype names are host-side nominal vocabulary
  carried only for handle resolution; they are dropped at lowering and are
  not fingerprint inputs, exactly as the macro's `as` names are emission.
- **Statements** (`StatementSpec`), tagged by form: `Fd` (no selection — the
  FD-with-selection shape is unrepresentable), `Containment { bidirectional }`
  (`bidirectional: true` IS the `==` spelling, lowered to the two adjacent
  containments, `source <= target` first), and `Cardinality { window }` with
  `WindowSpec` spelling the window exactly as written (`Exact(n)`,
  `Range { lo, hi }`, `Floor(lo)`). Projections are field-name vectors;
  selections are (field, literal-or-set) pairs over `LiteralSpec` — plain
  `Value`s or closed-relation handles by name, resolved through the selected
  field's newtype exactly as the macro resolves bare handles.

`SchemaSpec::descriptor()` lowers to the `SchemaDescriptor` and does exactly
what macro EXPANSION does — name→id resolution (declaration order mints every
id) and the canonical-utterance ban table over window spellings and literal
sets — returning the typed `SchemaSpecError`, which enumerates EVERY
unresolvable name (relation, field, handle) and banned spelling in one pass
(a foreign host repairs its whole spec in one round trip), each window error
naming the canonical form verbatim as the ban table does. Everything semantic
beyond names stays where the macro defers it: `SchemaDescriptor::validate`
inside `Db::create`/`Db::open`, the typed `SchemaError` — the same
two-boundary split.

**Macro and spec produce indistinguishable descriptors**: the same theory
built through either surface validates to the same sealed schema and carries
the same fingerprint (pinned by `tests/schema_spec.rs`, which builds a theory
using every construct — both closed tiers, `fresh`, fixed-width intervals,
all three statement forms, `==`, literal-set selections, every legal window
spelling — both ways and asserts fingerprint equality). The bindings roster
is reachable from the crate root: `Db`, `Snapshot`/`WriteTx`, `Theory`,
`SchemaDescriptor`, `SchemaSpec` + `SchemaSpecError`, `Value`, the `ir`
module, `PreparedQuery`/`Answers`, `SchemaError`, `FactShapeError`,
`Violation`/`Violations`, and `SchemaFingerprint`.

## Environment lifecycle

- `Db::open(path, Ledger)` — no tuning parameters: map size, max readers, and LMDB
  flags are internal (fsync durability on durable stores per `00-product.md`;
  flags are derived from the store KIND, never from a parameter); the schema definition
  (`Theory` — the macro's header struct, or a runtime-built `SchemaDescriptor`,
  which implements the trait as itself) is validated here (typed `SchemaError` on an
  invalid declaration) and what gets fingerprint-verified. Open verifies format
  version, then store kind, then schema fingerprint; each mismatch is a typed
  hard failure.
  `Db::create(path, Ledger)`
  initializes a fresh environment with the schema's fingerprint — and **refuses a
  directory that already holds any LMDB environment** (`AlreadyInitialized`): a
  bumbledb one (re-writing `_meta` counters over live data would be silent corruption,
  so create is exactly as non-destructive as open) or a foreign one (any other named
  database, or a non-empty unnamed root). The one exception is a half-created bumbledb
  store — a crash between directory creation and the meta commit leaves an empty root,
  and create recovers it.
- `Db::ephemeral(path, Ledger)` — the ephemeral store KIND's one constructor
  (`50-storage.md` § the ephemeral store kind; never a flag on `create`/`open`).
  A missing or empty directory initializes a fresh ephemeral store — the kind
  marked in `_meta` at birth — and an existing ephemeral store reopens under the
  same version/kind/fingerprint checks as `open` (create-or-open: a scratch store
  earns the convenience because a mistaken fresh store at a typo'd path destroys
  nothing durable; the dogfooding doctrine, `00-product.md`). The environment
  carries `WRITEMAP|NOSYNC`; every semantic — judgment, point reads, queries,
  locking — is identical to a durable store, and only machine-crash durability is
  renounced, by the store's own on-disk claim. Device-independent:
  ephemeral-on-SSD is legitimate.
- **The cross-open matrix is typed** (`crates/bumbledb/tests/ephemeral.rs`):
  `Db::open` on an ephemeral store and `Db::ephemeral` on a durable store are each
  `StoreKindMismatch { found, expected }`; `Db::create` on any initialized
  directory stays `AlreadyInitialized` (create never reads a store, so the kind
  never gets a say).
- **The two-store staging pattern** (the sighting the surface exists for): build
  an ephemeral store — bulk imports, judged exactly as a durable store judges —
  read/repair until the theory holds, then ETL the survivors into the durable
  store (`snap.scan` → `bulk_load_dyn`, § ETL below) and delete the directory. The
  staging side pays no fullfsync per commit (the small-commit shape measured
  ~75–90x over durable-on-SSD and ~4.2–4.4x over a plain ramdisk store across
  earn sessions, device tax 1.0–1.1x, the R6 lane of
  `crates/bumbledb/tests/ramdisk_phase_r.rs`); the durable side's
  guarantees never dilute because the kinds cannot cross-open.
- One process, one handle (`00-product.md`): every open holds an exclusive advisory
  lock on `<dir>/bumbledb.lock`; a second live handle on the same path — in this
  process or another — is `EnvironmentLocked` at open time. The handle is shareable
  across threads; drop closes and releases the lock. Ephemeral stores included —
  the lock, like every other mechanism, does not vary by kind.
- Dev-reset conveniences (delete + recreate) are host-side; production open never
  destroys data. `Db::ephemeral` never destroys data either — it opens or
  initializes, and deletion of a spent staging store is the host's explicit act.
  Nor does it MUTATE on refusal: an existing data file is probed through a plain
  durable-flagged open (no `WRITEMAP`, so no 4 GiB ftruncate) before the
  ephemeral flags are ever applied, so a refused probe — a durable store, a
  foreign LMDB environment, a stale or forged store — leaves `data.mdb`
  byte-identical (pinned by the byte-identity tests in
  `crates/bumbledb/tests/ephemeral.rs` and `storage/env/tests.rs`).

## Transactions

- `db.read(|snap| ...)` — one LMDB read snapshot; executes *prepared* queries.
  **`db.prepare(...)` is the ONE prepare entry** (the unified-prepare ruling,
  frozen 2026-07-15): it takes `impl Into<ProgramRef<'_>>`, so `db.prepare(&query)`
  and `db.prepare(&program)` both land on it — pin-at-prepare, `40-execution.md`.
  A query is the degenerate one-predicate program
  (`From<Query> for Program` is the owned embedding;
  `lean/Bumbledb/Exec/Fixpoint.lean: degenerate_embedding`); a no-`Idb` program
  prepares as its output predicate's query — byte for byte in the one-predicate
  case, and always carrying the program-global bind contract (params are ONE
  binding surface across predicates, `20-query-ir.md` § engine recursion; the
  query roster never re-judges what the program roster sealed) — and a recursive
  program executes under the fixpoint driver with the host-settable budget
  `prepared.set_fixpoint_budget(rounds, tuples)` — `40-execution.md` § the
  fixpoint driver. **`ProgramRef` borrows by decision, not convenience**: an
  owned `impl Into<Program>` was rejected because the `&Query → Program`
  conversion clones an *unvalidated* condition tree — a recursive `Clone`/`Drop`
  ahead of the iterative nesting screen, exactly the stack exhaustion the
  trust-boundary law refuses (`20-query-ir.md` § validation boundary; found by
  the adversarial sweep the moment the owned spelling was tried). The read
  closure sees
  a consistent generation (the snapshot-sourced tx id, `50-storage.md`) — every
  read is a function of that one state and nothing else
  (`lean/Bumbledb/Txn.lean: snapshot_reads_one_state`). A prepared
  query executes only against snapshots of the database that prepared it — every
  execution entry checks the environment's process-unique instance id first and
  returns `ForeignPreparedQuery` on a foreign snapshot (plan, statistics, and view
  memo all belong to the preparing environment).
- `prepared.staleness(&snap)` — the plan-drift signal, the pin-at-prepare decision's
  compensating control (`20-query-ir.md`): per participating occurrence, the fact
  count the plan was costed with against the snapshot's live `S` counter (one O(1)
  get each, ≤ 20 by the roster cap), each ratio
  `max(live, pinned) / max(1, min(live, pinned))` so shrink and growth both read as
  drift ≥ 1, plus the max. Pull-based and engine-policy-free: the engine never calls
  it and holds no thresholds — the host owns policy (`00-product.md`). There is no
  universal reprepare ratio: the 2026-07-12 estimator diagnosis found fresh-plan
  execution-work ratios vary by query class up to 4761.9×, so a fixed cutoff cannot
  separate drift from estimation shape. Hosts may compare this raw signal across
  generations using workload-specific evidence. Same
  foreign-snapshot check as execution; it allocates — a diagnostic surface, never a
  warm-path call. Negated and grounding-eliminated occurrences earn no statistics read
  at prepare and so carry no pin; key probes pin nothing. The stats/plan introspection
  surface (`Snapshot::profile`) carries the same pin record per occurrence —
  "estimated from (pinned facts at prepare)" — so a drifted plan is visible in one
  read of the existing report.
- `db.write(|tx| ...)` — the single writer; commits on `Ok`, aborts on `Err`/panic.
  Non-reentrant: a nested `write` from within a write closure on the same thread
  panics with a named message ("nested Db::write") rather than self-deadlocking on
  the writer mutex forever.
  Write operations: typed `alloc::<NewType>()` via the generated `Fresh` newtypes
  (untyped: `alloc_at(FreshField<S>) -> u64`, taking the schema-bound witness
  `Db::fresh_field(relation, field)` resolves — see the ETL section) — fresh
  minting, insert new facts
  without reading a max (`10-data-model.md`); `insert(&fact) -> bool` (changed-state
  report); `delete(&fact) -> bool`; `_dyn` forms of both for ETL tooling.
  `FreshExhausted` raises eagerly at the `alloc` call (the sequence state is knowable
  immediately), not at commit. Bulk import is `Db::bulk_load` (typed) /
  `Db::bulk_load_dyn` (the ETL/FFI lane) — `Db`-level methods,
  not write-closure operations (see the ETL section).
- **WriteTx point reads (decision):** `tx.contains(&fact) -> bool` (membership — the `insert`/`delete`
  return value's read-only sibling) and `tx.get::<F>(key) -> Option<F<'_>>` — lookup
  of the full fact through any key FD of its relation (typed via the key's newtype
  signature; `_dyn` form takes relation + statement id + encoded key). The typed get
  returns a **view at the transaction lifetime**: variable-width fields borrow from
  the committed dictionary (mmap pages, txn-stable by LMDB CoW) or this
  transaction's pending interns (the delta arena — read-your-writes included),
  whichever holds the value; a host that keeps a field past the transaction copies
  it explicitly. Both read
  **committed state overlaid with the pending delta** — the same final-state view
  the judgment checker judges (`50-storage.md`), so check-then-act is race-free by
  construction (single writer, one view). **The upsert idiom, blessed:**

  ```rust
  db.write(|tx| {
      match tx.get::<Account>(id)? {
          Some(old) => { tx.delete(&old)?; tx.insert(&Account { balance: old.balance + x, ..old })?; }
          None      => { tx.insert(&Account { id, balance: x, ..default })?; }
      }
      Ok(())
  })?
  ```

  **Full queries inside write transactions remain forbidden** — point reads are
  determinant gets (allocation-free, no images, no plans); dragging the image cache and
  executor into the write path is the refused half. **Alternative:** keep the pure
  two-transaction idiom. **Why it lost:** the surveyed workloads' upserts and
  check-then-act conditions are exactly the shape that needs a read of the state being
  written, and the two-txn idiom reintroduces the TOCTOU the single-writer design
  exists to kill (safe only under host-side write ordering nobody polices).
  **Reverses if:** never — the determinants are already read inside commit; this exposes
  the same gets one phase earlier. The ruling's **compensating control for
  query-driven writes** is the generation witness (§ conditional writes below):
  read on a snapshot, write through `write_from`.
- **The transaction is a delta** (`50-storage.md`): operations are in-memory set
  arithmetic; operation order is semantically irrelevant
  (`lean/Bumbledb/Txn.lean: final_state_judgment_order_free`); nothing touches LMDB until
  commit, and an abort never wrote anything. `delete(old); insert(new)` in either
  order is the blessed mutation idiom — a host-side `replace()` helper is optional
  sugar, not an engine operation (closed decision).
- **Dependencies are judged at commit against the final state**
  (`30-dependencies.md`): the `CommitRejected` error surfaces from the commit, not
  from the offending call site, carrying the failing phase's COMPLETE violation set
  (`lean/Bumbledb/Txn.lean: rejection_is_complete`) — each citation with the
  statement id (renderable back to the algebra through the schema) and the
  offending fact's bytes, in materialized statement order
  (`30-dependencies.md` owns the payload contract). The whole transaction aborts.

## Conditional writes — the generation witness

The persisted clock is the nominal public `GenerationId`, including the
`Db::generation` diagnostic accessor and both `GenerationMoved` fields; it is
never a bare integer in the engine API. The parked-reader cache uses a separate,
crate-private `CommitSeq` clock that resets at process open. The two clocks have
different lifetimes and cannot be compared or converted into one another.

### Derived-fact maintenance protocol (normative)

The host protocol is one explicit retry loop:

1. open a snapshot and run the deriving query against that snapshot;
2. compute the desired derived facts and diff them against the stored derived
   relation as seen by the same snapshot;
3. apply that diff with `db.write_from(&snapshot, |tx| ...)`;
4. on `GenerationMoved`, discard both derivation and diff, open a new snapshot,
   and start again; every other result ends the attempt.

The public write surface has exactly three epistemic classes:

| class | public path | what makes the premise current |
|---|---|---|
| snapshot-derived, generation-witnessed | `Db::write_from` | the snapshot's generation is compared inside the writer critical section before the closure runs |
| final-state point-read inside the write transaction | `Db::write` plus `WriteTx::{contains,get,get_dyn}` | the point read observes base + pending delta while the single-writer lock is held |
| unconditional | `Db::write` without a point-read premise; `Db::bulk_load` / `Db::bulk_load_dyn` | there is no read-derived premise to witness |

**Dependencies prove surviving derived facts sound; the WITNESS proves the
derivation saw the state it claims; nothing proves completeness — recompute
under a new witness**
(`lean/Bumbledb/Txn.lean: derived_soundness_vs_freshness`). In particular, the
engine does not retry, secretly run
a derivation, or claim that a stored relation equals a query result. Automatic
retries and hidden derivation semantics are host policy disguised as engine
behavior; query-defined/materialized-view equality remains D5 territory in the
constitution's refusal ledger. A schema may state one or both ordinary
containment directions when those projections express the intended invariant,
but it never gains an implicit refresh theorem.

The writer mutex serializes write *transactions*, not read-compute-write
*sequences*: query-driven writes — update-where-predicate, insert-select,
everything SQL spells with data-modifying CTEs — must read on a snapshot first,
then write, and two host threads interleaving snapshot-read → compute → write can
clobber each other's premises. The answer is representation, not control flow: a
snapshot already knows its generation, so *nothing changed since I looked* is a
proposition the commit checks in one integer compare.

- `db.write_from(&snap, |tx| ...)` — `db.write`, conditional on a witness:
  identical in every respect except one compare inside the writer's critical
  section (`lean/Bumbledb/Txn.lean: writeFrom_unmoved` — the compare is
  invisible on the success path). If a state-changing commit has landed since
  the witness snapshot's
  generation, the transaction aborts **before any page is touched** with the typed
  `GenerationMoved { witnessed, current }` (ids, never strings); the delta drops
  exactly as any abort does, and the closure never ran
  (`lean/Bumbledb/Txn.lean: writeFrom_moved`; a witness conflict is never a
  dependency failure and vice versa — `witness_conflict_distinct`, the two
  verdicts distinct by constructor). The environment-identity
  check runs first, exactly as prepared queries run it at every execution entry —
  a witness snapshot of another database is the typed `ForeignSnapshot`.
- **The witness is the snapshot, never an integer** (recorded refusal,
  recorded): a snapshot is evidence — its generation was read
  inside its own transaction — where an integer parameter would be a claim a
  caller could fabricate or stale-cache (parse, don't validate). `Snapshot`
  exposes no `generation()` accessor (decided: the witness consumes the
  generation internally; the diagnostics surface is `Db::generation`, and nothing
  more ships until the stats surface wants it).
- **State-changing generations only:** the compare targets the storage tx id —
  the same generation the image cache keys on — and a counters-only/no-op commit
  never advances it, so no-ops trip no witness. The sloppy alternative (any
  commit invalidates) is rejected: it would manufacture spurious retries out of
  no-ops.
- **Retry is host policy.** The engine ships the error, never a loop — the
  staleness-signal doctrine verbatim: the engine's job is to make the condition
  checkable. The host convention is re-run the query → re-compute → `write_from`
  again; conflicts are rare by the bursty-write design point (`00-product.md`).
- **The two conditions compose into the complete conditional-write vocabulary:** the
  witness is the scan-shaped condition (premises from full queries, whole-snapshot
  precision), WriteTx point reads remain the key-shaped condition (per-fact
  precision, zero retries, race-free by construction inside one transaction).
  *Read the model, propose a delta, commit iff the model you read is still the
  model* (`lean/Bumbledb/Txn.lean: writeFrom_unmoved`, `writeFrom_moved`).
- **The three idioms**, each query → compute → `write_from` → host retry:
  - *Update-where:* query the matching facts on a snapshot, compute their
    replacements, `write_from(&snap)` doing `delete(old); insert(new)` per fact.
  - *Insert-select:* query the source answers, compute the derived facts,
    `write_from(&snap)` inserting them — the data-modifying-CTE shapes with the
    premises witnessed instead of locked.
  - *Derived-relation maintenance:* re-run the deriving query, diff against the
    stored relation's current facts, `write_from(&snap)` applying the diff — the
    materialized-view refresh as an ordinary witnessed write
    (`10-data-model.md` § derived relations owns the pattern and its
    statements).

## Facts and results

- The write-side fact representation is the schema-macro-generated struct per relation
  (`Account { id, holder, kind, active }`), carrying host newtypes; the boundary
  encodes to canonical `fact_bytes` (interval newtypes carry `Interval<i64>`/`
  Interval<u64>` values whose `start < end` invariant is enforced at construction —
  parse, don't validate, in the host too). A dynamic (untyped) fact form exists for
  ETL tooling.
- **Borrowed structs:** generated structs carry `str` fields by reference
  (`str` → `&'a str`; a `str`-carrying relation gains one lifetime — `bytes<N>` is
  `[u8; N]`, owned and `Copy`). Insert takes the struct at any lifetime — the encode path
  reads the fields as borrows into the engine's arena copy. Typed reads
  (`tx.get`, `snap.scan_facts`) return views at the resolver's lifetime, UTF-8
  validated at resolve without a copy. There are no owned twins and no modes;
  ownership is an explicit host act (`to_owned()` on the field you keep). The trait
  shape is `impl<'a> Fact<'a> for Account<'a>` (module doc records the
  GAT alternative and why it lost).
- **Decision: borrowed variable-width types on the fact and param surfaces.**
  Ownership is an explicit host act. **Alternative:** the owned surface (`String`/
  `Vec<u8>` fields, owned scalar param payloads). **Why it lost:** four ceremony
  allocations serving no engine purpose — insert read the owned field once as a
  borrow before the arena copy; typed get allocated a fresh `String` per str field
  per read out of the mmap, which callers compared and dropped; scalar str/bytes
  params boxed per bind for a hash-and-probe; and validity was stated in prose
  where a lifetime parameter states it compile-checked (precedent:
  `sqlite3_column_text`, LMDB `get` — borrow-until-txn-end as the only option).
  **Reverses if:** a real host profile shows `to_owned()` dominating — hosts
  overwhelmingly keeping every field they read.
- **Schema typestate:** `Db<S>` carries the schema definition as a phantom
  parameter, threaded through `WriteTx`/`Snapshot`/`PreparedQuery`; `Fact` carries
  `type Schema`, and write/read operations bound `F: Fact<'_, Schema = S>`.
  Inserting a schema-A struct into a schema-B database — or executing a prepared
  query against another schema's snapshot — is a **compile error**, closing the
  cross-schema `RelationId`-aliasing hole that a width mismatch only caught by
  luck. Inference hides the parameter at call sites; same-schema/different-
  environment confusion stays a runtime check (`ForeignPreparedQuery`).
- Query answers: one concrete `Answers` carrier (decided: columnar cells + a byte heap,
  no caller-buffer trait) — answers of decoded values (String decoded from intern
  ids at materialization, into the buffer's byte heap; `bytes<N>` re-assembled
  from its inline slot words with no dictionary touch; intervals as start/end word
  pairs), an `answers()` iterator, and column metadata via
  `PreparedQuery::predicate()` — the predicate the query defines
  (`20-query-ir.md` § the query shape) is the **buffer-typing authority**:
  one signature column per head position, result type plus producing fold,
  sealed at validation and read by every consumer (the buffer itself stays
  typeless: stamping owned types per execution would allocate on the warm
  path). Contract on `Err`: the
  buffer's contents are unspecified — ignore `out` when `execute` errors; the
  snapshot stays usable. Answers form a **set**: unordered; the host sorts. Zero-alloc
  path: caller-provided reusable buffer (`40-execution.md`); convenience path
  allocates a fresh buffer.
- Params are supplied positionally by `ParamId` at execution — scalars as
  `BindValue<'a>` (str/bytes payloads **by reference**: the engine only hashes or
  encodes them to column words, so a warm re-bind allocates nothing host-side —
  and a `bytes<N>` param touches no dictionary, ever; `ir::Value` stays
  owned — IR literals are long-lived query data), **param sets as slices** of owned
  `Value`s (a set is long-lived host data re-bound by reference; deduplicated at
  bind; the documented small-set planning assumption is `20-query-ir.md`'s); count
  and structural types checked at bind time. Each prepared param slot carries one
  sealed spec: scalar/set shape, point-domain status, and mask-ness are structure,
  never parallel flags.

## Errors (taxonomy skeleton)

- **Open errors:** `FormatMismatch`, `StoreKindMismatch { found, expected }` (the
  kind marker read after the version, before the fingerprint — the cross-open
  matrix, § environment lifecycle), `SchemaMismatch`, `AlreadyInitialized`,
  `EnvironmentLocked`, `Io`, `Lmdb`.
- **Schema errors** (declaration boundary, `30-dependencies.md` roster included):
  typed, enumerated, returned from `Db::create`/`Db::open` — where the definition's
  descriptor is validated — before any environment exists.
- **Schema warnings:** an accepted sealed schema exposes `Schema::warnings()`.
  `RedundantSuperkey { relation, key, implied_by }` reports determinant write
  amplification without weakening or disabling either key; warnings are never
  errors and never alter the fingerprint.
- **Validation errors** (IR boundary, `20-query-ir.md` roster): typed, enumerated,
  returned at prepare time.
- **Runtime query errors:** `Overflow` (aggregate range check),
  `FixpointBudgetExceeded { stratum, rounds, tuples }` (a recursive stratum
  crossed the driver's iteration/tuple budget — ids and counts, the documented
  default host-amendable via `set_fixpoint_budget`; `40-execution.md` § the
  fixpoint driver), `Corruption` (hard error, never a skip — `50-storage.md`).
  They abort the query; the read transaction remains usable.
- **Write errors:** `CommitRejected` (raised at commit, against the final state,
  carrying the failing phase's COMPLETE violation set in statement order —
  `lean/Bumbledb/Txn.lean: rejection_is_complete` — with each citation's
  offending facts ALSO decoded to owned values at the rejection boundary,
  renderable to named plain data via `schema::render_rejection`;
  `30-dependencies.md` § rendering the rejection), `GenerationMoved`
  (the witness compare, § conditional writes — carrying the two generations),
  `ForeignSnapshot` (a witness of another database), `FreshExhausted`,
  `FactShape` (the dynamic surface's shape roster — including the dyn-boundary
  foreign-`FreshField` refusal at the mint's sequence init, § ETL),
  `Corruption`, `Io`/`Lmdb`. Any error aborts the whole write transaction — and
  since the transaction is a delta, an aborted transaction never touched LMDB at all.
- Error payloads carry ids, not formatted strings, on hot paths (allocation contract).

## ETL / migration surface

Schema change = ETL into a new database (`10-data-model.md`) — the only path from
any other format, stated. The laws: export→import of a committed state into the
same theory is a no-op (`lean/Bumbledb/Txn.lean: etl_identity`), and a transform
into a new theory either lands already holding it or rejects with the failing
phase's complete
violation set — there is no migrate-now-validate-later state (`etl_lands_valid`).
The **export
surface is a full-relation scan**: `snap.scan(relation)` yields *dynamic* facts
(`Result<Vec<Value>>` — per-item corruption is a hard error and the stream fuses)
over `F` in row_id order (a storage iteration, not a query — streams, not sets); the
typed sibling `snap.scan_facts::<F>()` decodes into the generated structs.

**Bulk import is two lanes over ONE chunking mechanism** (the typed-bulk ruling,
frozen 2026-07-15 — it closed the "typed everywhere except bulk" gap):
`Db::bulk_load(facts)` takes an iterator of **generated fact structs** (the
relation is `F::RELATION` — no id parameter to mismatch), and
`Db::bulk_load_dyn(relation, facts)` takes `Vec<Value>` rows — the ETL/FFI lane
that pairs with `snap.scan`'s dynamic export and with foreign hosts speaking the
manifest's ids. Both lanes share the contract verbatim: chunks of 4096 per
transaction, each chunk atomic and fully judged, prior chunks committed on failure
with the committed count carried on `BulkLoadError` — and kept through `?`: the
conversion into the workspace error lands in `Error::BulkLoad { committed, error }`,
never dropping the count (it is the resumability payload the type exists for). The
returned/carried count is **facts that changed
state** (idempotent re-inserts are consumed but not counted) — changed-not-consumed
semantics, stated. Mis-shaped dynamic facts (including out-of-range relation ids)
are typed `FactShape` errors (decided: ETL input is data, not code — no panics on the
import path); the typed lane makes shape errors unrepresentable and keeps only the
judgment. Interval fields accept only the checked `Interval<T>` carried by
`Value`, so `start ≥ end` cannot enter this path. Explicit fresh values preserve
identity (high-water advances past them). Untyped fresh minting is
resolve-once/mint-per-fact:
`Db::fresh_field(relation, field) -> Result<FreshField<S>, FactShapeError>`
validates the ids and the `Fresh` generation once and returns a `Copy`
**schema-bound** witness (private fields, one construction site, and the
resolving handle's typestate `S` in the witness's type), so handing the witness
to another schema's transaction is a compile error
(`tests/schema-compile-fail/foreign_fresh_witness.rs`); `tx.alloc_at(witness)`
mints with no generation re-check on the steady-state path. (REVERSED
2026-07-15, the cross-schema witness ruling: the original decision — "the type
is the proof", resolved by `Schema::fresh_field` with no re-check anywhere, a
per-call typed error rejected as validating on every call and throwing the
proof away — bound the proof to no schema, so a witness resolved against
schema A reached a database of schema B and release builds silently minted
from a `Q` key of a non-fresh field, breaking `Fresh`'s never-reissue
guarantee. The witness now carries a BINDING proof — the schema in its type,
the hard-structural-typing answer: nominal safety = host Rust newtypes — at
zero mint-path cost. At the dyn boundary, where every `Db<SchemaDescriptor>`
shares one typestate and the binding proves nothing across descriptors, the
mint's per-transaction sequence init re-checks the generation beside the `Q`
read it already does and refuses a foreign witness as the typed `FactShape`
error, never a panic, never a silent mint — pinned by
`a_foreign_witness_is_refused_typed_not_minted`.) **Import order under bidirectional statements is
the importer's obligation:** a `==` statement's cluster must land within one chunk's
transaction, so the documented import order is dependency-cluster order — parent and
arm facts interleaved — and a straddled cluster fails its chunk loudly
(`50-storage.md`). `Fact::encode_read`'s reader-side encode is host-reachable
surface — a stated decision: it reports "this fact cannot exist" for never-interned
values and is the membership-probe building block. `Db::compact` is safe concurrent
with a writer (LMDB's copy transaction reads one consistent snapshot; the copy simply
omits later commits). Backup = quiesced file copy (`50-storage.md`).

## The dyn lane (the schema-generic roster, normative)

A schema-generic bridge — the Node bindings, ETL tooling, any host without the
generated fact structs — drives the FULL write-and-read surface through ids and
`Value` rows alone. The roster, complete:

- **Writes:** `tx.insert_dyn(relation, &[Value])` / `tx.delete_dyn(...)` (the
  delete+insert identity idiom included: explicit fresh values preserve
  identity, high-waters advance past them), `Db::bulk_load_dyn` (§ ETL).
- **Fresh minting:** `Db::fresh_field(relation, field)` resolves the witness
  once; `tx.alloc_at(witness) -> u64` mints per row and RETURNS the minted id
  to the caller — the dyn lane's mint is the same alloc-then-insert shape the
  typed lane uses (`alloc::<NewType>()` then `insert`), so there is no second
  insert-with-omitted-fields spelling (one meaning, one spelling).
- **Point reads, both transaction kinds:** `tx.contains_dyn` / `tx.get_dyn`
  observe the final-state view the judgment judges (base + pending delta);
  `snap.contains_dyn` / `snap.get_dyn` observe the snapshot's committed state.
  `get_dyn` takes `(relation, key statement id, key values in projection
  order)`; closed relations answer from their sealed extensions (virtual
  storage), and an out-of-roster handle word is an honest miss, not an error.
- **Scans:** `snap.scan(relation)` (dynamic export, § ETL).
- **Queries:** prepared queries already take parameter values as plain data at
  execute time (`BindValue` scalars / `ParamArg` sets of `ir::Value`) and
  answers come back as owned decoded rows (`Answers`, one-copy) — confirmed
  complete for schema-generic hosts; no generated type appears anywhere on the
  bind or answer path.

The trust boundary is uniform: malformed arity, wrong value types, non-UTF-8
strings, unknown relation ids, mis-aimed key-statement ids are typed
`FactShape` errors — never panics (the adversarial sweep,
`crates/bumbledb/tests/dyn_surface.rs`). A rejected commit is consumable the
same way: decoded cited facts ride the violation set and
`schema::render_rejection` lowers it to named plain data
(`30-dependencies.md` § rendering the rejection).

## Observability

Two feature-gated surfaces, both compiling to nothing under default features
(`00-product.md`: no always-on instrumentation in release paths): the `alloc-counter`
feature registers the counting allocator (events + bytes + current live bytes, the gate's and
the benchmark's memory truth), and the `trace` feature enables `bumbledb::obs` —
explicit per-thread capture of nanosecond spans and point events over every prepare/
execute/commit phase, drained by tooling into Chrome-trace artifacts. Plan
introspection — EXPLAIN, colloquially — is always available through
`snap.introspect(..)`. It returns an ANALYZE-semantics rendered artifact beginning
with `introspection v3`, then the query in rule notation (`20-query-ir.md` § the
renderer; `PreparedQuery::rendered_query` exposes the same query string), predicate,
plan sections, and diagnostics. `Snapshot::profile` returns the same execution as
structured `ExecutionStats`, carrying `introspection_version: 3`, each rule's
`distinct_bindings` proof status, the same program/node ordering, and — for
recursive programs — the fixpoint driver's per-stratum round records: labeled plan
units (predicate, rule, delta variant), each round's per-predicate delta sizes,
and the union accounting (`40-execution.md` § observability).

Within one version, identical schema fingerprint, canonical query, parameter types,
and feature set produce byte-identical rendered output. Sections are fixed; rules
remain in program order, nodes in plan order, and dead, subsumed, and unresolved-
literal diagnostics in statement order. Any content or ordering change increments
the version in both surfaces. When a String literal still awaits interning, plan
introspection names every pending
literal and states the latch consequence: an unresolved `Eq` literal empties its
rule at execution until latched. The line is derived from the live plan templates
after execution, so it disappears on the execution that resolves and rewrites the
literal (`api/prepared/introspect.rs`, `bind.rs`). For a query prepare *rejected*
there is no handle to ask:
`Db::render_query` renders any query — malformed included, with placeholder
names — so roster errors print beside the query they reject.

## Host-side sugar (blessed patterns, never the contract)

- Newtype wrappers per schema type (`AccountId(u64)`, `Cents(i64)`,
  `ActiveDuring(Interval<i64>)`) — the nominal safety layer (`10-data-model.md`).
- Query-fragment functions as the view layer — *a view is a function returning
  atoms*; the pattern and its worked calendar example (`busy_claims`, one
  fragment spliced positive, negated, and `Pack`-folded) are
  `10-data-model.md` § derived relations.
- The outer-join merge: run the positive and the negated query, concatenate — the
  sanctioned decomposition (`20-query-ir.md`), a two-line host function.
- Zero-default aggregates: the host maps an absent aggregate answer to 0 where the
  domain wants it (`20-query-ir.md` empty-set semantics).
- Downstream query sugar — in any language — lowers to IR data; the engine never
  knows it exists (the permanent surface ruling, `20-query-ir.md`; the
  text-language OPEN item is superseded by it). A typed builder is refused,
  recorded: closures and generics are what a foreign host cannot invoke, and the
  roster's typed errors re-provide the checking for every caller equally. **The
  blessed Rust sugar is `crates/bumbledb-query`'s `query!` macro** — a downstream
  crate on the bench-crate quarantine, lowering the notation (`20-query-ir.md`
  § the query notation) to the `ir::Query` value at compile time and resolving
  names through the emitted id constants.

## Anticipated bindings — punted, recorded

JS/N-API bindings are **punted**: pure anticipation, zero deliverable, and no
engine decision may lean on their existence. The recorded shape for whenever the
owner wants them: a quarantined downstream crate on the bench-crate precedent (it
may hold the N-API dependency; the engine never depends on it), compiling the
application's `schema!` in, exposing prepared-query handles, the dyn read/write
surfaces, and the manifest; marshaling IR-as-data in and result copies out. The
engine-side surface is already correct the day they are wanted: the trust-boundary
law makes foreign IR safe (`20-query-ir.md` § validation boundary), the manifest
carries the ids as data, the memoized one-copy result heap crosses a language
boundary where a borrowed result could not, and the dyn write surface's typed
errors are the portable half of the API.

## The freeze, and the OPEN ledger

**FREEZE IS DECLARED at this commit (2026-07-15).** The surface above — the
`schema!` grammar (owner-evolvable by its own standing ruling), the environment
lifecycle (`create`/`open`/`ephemeral`), the unified `db.prepare`, the
transaction closures with their point reads and the generation witness, the two
bulk lanes, the scan exports, and the error taxonomy — is the v0 embedding API.
Everything below is DEFERRED, each item with the **trigger** that reopens it;
nothing on this list ships without its trigger firing, and nothing off this list
ships without a new ruling.

- **`tx.insert_all` batch sugar** (one call, many typed facts inside a write
  closure). Trigger: **dogfooding pain** — a real host import loop inside
  `db.write` where the per-fact `insert` call reads as noise or measures as
  overhead. Until then the `for` loop is the surface, and bulk import already
  has `Db::bulk_load`.
- **Multi-key typed `tx.get` disambiguation** — the typed signature when a
  relation carries several key FDs over the same newtype. Trigger: a **real
  schema** exhibiting the collision (the `_dyn` form is unambiguous today; the
  typed sugar waits for the usage that names its shape).
- **Answer sorting / `FromAnswers` derive** in `bumbledb-query` (the
  ordering/limit conveniences fold in here — host-side, on the bench-crate
  quarantine like the `query!` macro; answers are sets and the engine never
  orders). Trigger: **week-one dogfooding** — the first real host that sorts
  and destructures `Answers` by hand tells us the derive's shape.
- **`write_from` retry helper.** REFUSED as engine surface — retry is host
  policy and **the host owns the loop** (the staleness-signal doctrine
  verbatim). The blessed host snippet, in full:

  ```rust
  loop {
      let attempt = db.read(|snap| {
          let premises = snap.execute_collect(&mut deriving_query, &args)?; // read
          let delta = compute(&premises);                                   // compute
          db.write_from(snap, |tx| delta.apply(tx))                         // witnessed write
      });
      match attempt {
          Err(bumbledb::Error::GenerationMoved { .. }) => continue, // premises stale: re-derive
          other => break other,                                     // done, or a real error
      }
  }
  ```

  (`crates/bumbledb-query/tests/cookbook.rs` recipe 27 pins the pattern,
  retries counted.) Nothing here can become engine surface without hiding
  policy — the loop's shape IS the host's policy.
- **Multi-process story** (closed as out-of-envelope for v0; the future item
  lives here). Trigger: a second process with a legitimate claim on one store —
  today that is the ETL story's job.

Resolved by ruling or implementation (recorded above): the `Answers` shape;
the dynamic-fact ETL form; plan introspection's versioned surface
(`snap.introspect(&mut prepared, params) -> (Answers, String)` — ANALYZE
semantics, rendered-text report); WriteTx point reads; the unified `prepare`;
the typed bulk lane.
