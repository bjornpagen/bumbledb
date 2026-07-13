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
  `interval<i64>`, `interval<u64>` — the six-type roster; the inline `enum`
  production is deleted vocabulary (a vocabulary is a closed relation, and
  the word diagnoses its own replacement at expansion). `as` is legal on u64, i64,
  `bytes<N>`, and intervals (the newtype wraps the engine value; rustc polices
  domains — `10-data-model.md`; bytes and interval newtypes derive no order —
  both refusals are semantics, `10-data-model.md`).
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
  the `= { … }` extension block is required and non-empty. Each row is
  `Handle` or `Handle { column: literal, … }` with every declared column
  present exactly once — duplicate handles, missing/extra/duplicate columns,
  and type-mismatched literals are expansion errors naming the offender. Row
  literals ride the selection-literal machine (same typing, same errors). In
  statement selections a bare handle is legal on any field whose newtype is a
  closed relation's handle newtype (`| status == Frozen`), resolving to the
  handle's declaration-order row id at expansion exactly as field names
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
  `==` for bidirectional. Projection lists are positional between the two sides;
  selections follow `|` as comma-separated `field == literal` pairs; literals are
  closed-relation handles, integer literals, `true`/`false`, string/byte literals, and
  `start..end` interval literals (half-open). The macro emits descriptors directly —
  relation/field names resolve to declaration-order ids at expansion time, so an
  unresolvable name is a compile error naming the relation and field — and
  performs no semantic validation beyond parse shape and name-to-id resolution:
  the schema validation boundary (`30-dependencies.md` roster) is the judge, and
  everything semantic beyond names is a normal typed error with the statement
  rendered back.
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
`Theory`) remains the *data* schema surface — the bench crate, the oracle, and any
future binding that needs runtime schemas — existing, not blessed.

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
fields with their ids stated explicitly, each field's structural type, and each
closed relation's **extension table**
(relation → handle → declaration-order row id → (column, value) pairs), so foreign
surfaces see the vocabulary without touching Rust. A foreign host gets the same
numbers as data. No serde anywhere (the dependency law): a downstream binding
serializes the value however it likes; the engine never learns the wire format.
Both are emission; the grammar is untouched.

## Environment lifecycle

- `Db::open(path, Ledger)` — no tuning parameters: map size, max readers, and LMDB
  flags are internal (fsync durability per `00-product.md`); the schema definition
  (`Theory` — the macro's header struct, or a runtime-built `SchemaDescriptor`,
  which implements the trait as itself) is validated here (typed `SchemaError` on an
  invalid declaration) and what gets fingerprint-verified. Open verifies format
  version, then schema fingerprint; each mismatch is a typed hard failure.
  `Db::create(path, Ledger)`
  initializes a fresh environment with the schema's fingerprint — and **refuses a
  directory that already holds any LMDB environment** (`AlreadyInitialized`): a
  bumbledb one (re-writing `_meta` counters over live data would be silent corruption,
  so create is exactly as non-destructive as open) or a foreign one (any other named
  database, or a non-empty unnamed root). The one exception is a half-created bumbledb
  store — a crash between directory creation and the meta commit leaves an empty root,
  and create recovers it.
- One process, one handle (`00-product.md`): every open holds an exclusive advisory
  lock on `<dir>/bumbledb.lock`; a second live handle on the same path — in this
  process or another — is `EnvironmentLocked` at open time. The handle is shareable
  across threads; drop closes and releases the lock.
- Dev-reset conveniences (delete + recreate) are host-side; production open never
  destroys data.

## Transactions

- `db.read(|snap| ...)` — one LMDB read snapshot; executes *prepared* queries
  (`db.prepare(&Query)` is the sole entry — pin-at-prepare, `40-execution.md`); sees
  a consistent generation (the snapshot-sourced tx id, `50-storage.md`). A prepared
  query executes only against snapshots of the database that prepared it — every
  execution entry checks the environment's process-unique instance id first and
  returns `ForeignPreparedQuery` on a foreign snapshot (plan, statistics, and view
  memo all belong to the preparing environment).
- `prepared.staleness(&snap)` — the plan-drift signal, the pin-at-prepare decision's
  compensating control (`20-query-ir.md`): per participating occurrence, the row
  count the plan was costed with against the snapshot's live `S` counter (one O(1)
  get each, ≤ 20 by the roster cap), each ratio
  `max(live, pinned) / max(1, min(live, pinned))` so shrink and growth both read as
  drift ≥ 1, plus the max. Pull-based and engine-policy-free: the engine never calls
  it and holds no thresholds — the host owns policy (`00-product.md`). There is no
  universal reprepare ratio: the 2026-07-12 estimator diagnosis found fresh-plan
  execution-work ratios vary by query class up to 4761.9×, so a fixed cutoff cannot
  separate drift from estimation shape. Hosts may compare this raw signal across
  generations using workload-specific evidence. Same
  foreign-snapshot guard as execution; it allocates — a diagnostic surface, never a
  warm-path call. Negated and grounding-eliminated occurrences earn no statistics read
  at prepare and so carry no pin; guard probes pin nothing. The stats/EXPLAIN
  surface (`Snapshot::profile`) carries the same pin record per occurrence —
  "estimated from (pinned rows at prepare)" — so a drifted plan is visible in one
  read of the existing report.
- `db.write(|tx| ...)` — the single writer; commits on `Ok`, aborts on `Err`/panic.
  Non-reentrant: a nested `write` from within a write closure on the same thread
  panics with a named message ("nested Db::write") rather than self-deadlocking on
  the writer mutex forever.
  Write operations: typed `alloc::<NewType>()` via the generated `Fresh` newtypes
  (untyped: `alloc_at(FreshField) -> u64`, taking the witness
  `Schema::fresh_field(relation, field)` resolves — see the ETL section) — fresh
  minting, insert new rows
  without reading a max (`10-data-model.md`); `insert(&fact) -> bool` (changed-state
  report); `delete(&fact) -> bool`; `_dyn` forms of both for ETL tooling.
  `FreshExhausted` raises eagerly at the `alloc` call (the sequence state is knowable
  immediately), not at commit. Bulk import is `Db::bulk_load` — a `Db`-level method,
  not a write-closure operation (see the ETL section).
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
  guard gets (allocation-free, no images, no plans); dragging the image cache and
  executor into the write path is the refused half. **Alternative:** keep the pure
  two-transaction idiom. **Why it lost:** the surveyed workloads' upserts and
  check-then-act guards are exactly the shape that needs a read of the state being
  written, and the two-txn idiom reintroduces the TOCTOU the single-writer design
  exists to kill (safe only under host-side write ordering nobody polices).
  **Reverses if:** never — the guards are already read inside commit; this exposes
  the same gets one phase earlier. The ruling's **compensating control for
  query-driven writes** is the generation witness (§ conditional writes below):
  read on a snapshot, write through `write_from`.
- **The transaction is a delta** (`50-storage.md`): operations are in-memory set
  arithmetic; operation order is semantically irrelevant; nothing touches LMDB until
  commit, and an abort never wrote anything. `delete(old); insert(new)` in either
  order is the blessed mutation idiom — a host-side `replace()` helper is optional
  sugar, not an engine operation (closed decision).
- **Dependencies are judged at commit against the final state**
  (`30-dependencies.md`): the `CommitRejected` error surfaces from the commit, not
  from the offending call site, carrying the COMPLETE violation set — every violated
  statement, cited once (per direction for a containment), in materialized statement
  order — each citation with the statement id (renderable back to the algebra
  through the schema) and the offending fact's bytes. The whole transaction aborts.

## Conditional writes — the generation witness

The persisted clock is the nominal public `GenerationId`, including the
`Db::generation` diagnostic accessor and both `GenerationMoved` fields; it is
never a bare integer in the engine API. The parked-reader cache uses a separate,
crate-private `CommitSeq` clock that resets at process open. The two clocks have
different lifetimes and cannot be compared or converted into one another.

The writer mutex serializes write *transactions*, not read-compute-write
*sequences*: query-driven writes — update-where-predicate, insert-select,
everything SQL spells with data-modifying CTEs — must read on a snapshot first,
then write, and two host threads interleaving snapshot-read → compute → write can
clobber each other's premises. The answer is representation, not control flow: a
snapshot already knows its generation, so *nothing changed since I looked* is a
proposition the commit checks in one integer compare.

- `db.write_from(&snap, |tx| ...)` — `db.write`, conditional on a witness:
  identical in every respect except one compare inside the writer's critical
  section. If a state-changing commit has landed since the witness snapshot's
  generation, the transaction aborts **before any page is touched** with the typed
  `GenerationMoved { witnessed, current }` (ids, never strings); the delta drops
  exactly as any abort does, and the closure never ran. The environment-identity
  guard runs first, exactly as prepared queries run it at every execution entry —
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
- **The two guards compose into the complete conditional-write vocabulary:** the
  witness is the scan-shaped guard (premises from full queries, whole-snapshot
  precision), WriteTx point reads remain the key-shaped guard (per-fact
  precision, zero retries, race-free by construction inside one transaction).
  *Read the model, propose a delta, commit iff the model you read is still the
  model.*
- **The three idioms**, each query → compute → `write_from` → host retry:
  - *Update-where:* query the matching facts on a snapshot, compute their
    replacements, `write_from(&snap)` doing `delete(old); insert(new)` per fact.
  - *Insert-select:* query the source rows, compute the derived facts,
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
  (`str` → `&'a str`; one lifetime iff the relation has one — `bytes<N>` is
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
- Query results: one concrete `ResultBuffer` (decided: columnar cells + a byte heap,
  no caller-buffer trait) — rows of decoded values (String decoded from intern
  ids at materialization, into the buffer's byte heap; `bytes<N>` re-assembled
  from its inline slot words with no dictionary touch; intervals as start/end word
  pairs), a `rows()` iterator, and column metadata via
  `PreparedQuery::predicate()` — the predicate the query defines
  (`20-query-ir.md` § the query shape) is the **buffer-typing authority**:
  one signature column per head position, result type plus producing fold,
  sealed at validation and read by every consumer (the buffer itself stays
  typeless: stamping owned types per execution would allocate on the warm
  path). Contract on `Err`: the
  buffer's contents are unspecified — ignore `out` when `execute` errors; the
  snapshot stays usable. Results are **sets**: unordered; the host sorts. Zero-alloc
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

- **Open errors:** `FormatMismatch`, `SchemaMismatch`, `Io`, `Lmdb`.
- **Schema errors** (declaration boundary, `30-dependencies.md` roster included):
  typed, enumerated, returned from `Db::create`/`Db::open` — where the definition's
  descriptor is validated — before any environment exists.
- **Validation errors** (IR boundary, `20-query-ir.md` roster): typed, enumerated,
  returned at prepare time.
- **Runtime query errors:** `Overflow` (aggregate range check), `Corruption` (hard
  error, never a skip — `50-storage.md`). They abort the query; the read transaction
  remains usable.
- **Write errors:** `CommitRejected` (raised at commit, against the final state,
  carrying the complete violation set in statement order), `GenerationMoved`
  (the witness compare, § conditional writes — carrying the two generations),
  `ForeignSnapshot` (a witness of another database), `FreshExhausted`,
  `Corruption`, `Io`/`Lmdb`. Any error aborts the whole write transaction — and
  since the transaction is a delta, an aborted transaction never touched LMDB at all.
- Error payloads carry ids, not formatted strings, on hot paths (allocation contract).

## ETL / migration surface

Schema change = ETL into a new database (`10-data-model.md`) — the only path from
any other format, stated. The **export
surface is a full-relation scan**: `snap.scan(relation)` yields *dynamic* facts
(`Result<Vec<Value>>` — per-item corruption is a hard error and the stream fuses)
over `F` in row_id order (a storage iteration, not a query — streams, not sets); the
typed sibling `snap.scan_facts::<F>()` decodes into the generated structs. The
dynamic form pairs with `Db::bulk_load(relation, facts)`: chunks of 4096 per
transaction, each chunk atomic, prior chunks committed on failure with the committed
count carried on `BulkLoadError` — and kept through `?`: the conversion into the
workspace error lands in `Error::BulkLoad { committed, error }`, never dropping the
count (it is the resumability payload the type exists for). The returned/carried
count is **facts that changed
state** (idempotent re-inserts are consumed but not counted) — changed-not-consumed
semantics, stated. Mis-shaped dynamic facts (including out-of-range relation ids)
are typed `FactShape` errors (decided: ETL input is data, not code — no panics on the
import path). Interval fields accept only the checked `Interval<T>` carried by
`Value`, so `start ≥ end` cannot enter this path. Explicit fresh values preserve
identity (high-water advances past them). Untyped fresh minting is
resolve-once/mint-per-row:
`Schema::fresh_field(relation, field) -> Result<FreshField, FactShapeError>`
validates the ids and the `Fresh` generation once and returns a `Copy` witness
(private fields, one construction site — the type is the proof);
`tx.alloc_at(witness)` mints with no generation re-check anywhere on the path
(decided: a per-call typed error inside the mint was rejected — it validates on
every call and throws the proof away). **Import order under bidirectional statements is
the importer's obligation:** a `==` statement's cluster must land within one chunk's
transaction, so the documented import order is dependency-cluster order — parent and
arm facts interleaved — and a straddled cluster fails its chunk loudly
(`50-storage.md`). `Fact::encode_read`'s reader-side encode is host-reachable
surface — a stated decision: it reports "this fact cannot exist" for never-interned
values and is the membership-probe building block. `Db::compact` is safe concurrent
with a writer (LMDB's copy transaction reads one consistent snapshot; the copy simply
omits later commits). Backup = quiesced file copy (`50-storage.md`).

## Observability

Two feature-gated surfaces, both compiling to nothing under default features
(`00-product.md`: no always-on instrumentation in release paths): the `alloc-counter`
feature registers the counting allocator (events + bytes + current live bytes, the gate's and
the benchmark's memory truth), and the `trace` feature enables `bumbledb::obs` —
explicit per-thread capture of nanosecond spans and point events over every prepare/
execute/commit phase, drained by tooling into Chrome-trace artifacts. Always
available: `snap.explain(..)` (rendered report — it opens with the query in the
rule notation, `20-query-ir.md` § the renderer; `PreparedQuery::rendered_query`
exposes the same string) and the structured execution-stats surface it is built
from. For a query prepare *rejected* there is no handle to ask:
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
- Zero-default aggregates: the host maps an absent aggregate row to 0 where the
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

## OPEN (this doc's honest list)

Resolved by ruling or implementation (recorded above): the `ResultBuffer` shape;
the dynamic-fact ETL form; EXPLAIN's surface (`snap.explain(&mut prepared, params)
-> (ResultBuffer, String)` — ANALYZE semantics, rendered-text report); WriteTx point
reads (decided).

Still open:

- Ordering/limit conveniences on results (host-side; shape undecided).
- The typed signature for multi-key `tx.get` disambiguation when a relation carries
  several key FDs over the same newtype (the `_dyn` form is unambiguous today;
  the typed sugar waits for real usage).
- Multi-process story (closed as out-of-envelope for v0; future item lives here).
