# Admitted instances and catalog lifetimes

## Ruling

The engine reads one admitted relational instance.

An admitted instance can have either of these physical durations:

- `OwnedInstance<S>` owns one immutable packed catalog in the heap.
- `ReadInstance<'txn, S>` owns one LMDB read transaction that borrows its
  environment.

The two types expose the same instance algebra. They do not share one concrete
storage representation. Generic internal algorithms provide the shared
implementation.

Heap construction and durable mutation also have different durations:

- `InstanceBuilder<S>` owns an unproved candidate built from an empty base.
- `WriteTx<'db, S>` borrows an admitted store and one LMDB write transaction.

They share one private collection-mutation algebra. They do not collapse into
one public `Admitting` type.

Admission is a parse into an accepted type. It is not a Boolean property on a
mutable object.

One law governs every choice in this proposal: complexity moves into the
representation, never into the control flow. Where the audit of the previous
draft found a guard, this draft deletes the state the guard was checking.
Each section names the states it makes unrepresentable.

This proposal makes breaking changes. It provides no aliases for retired
names. It does not preserve the 0.14 API. It ships as **0.15.0** in Rust,
TypeScript, and C lockstep. The C ABI version becomes **3**. The owner-gated
1.0.0 close is untouched.

The Lean specification and the Rust engine move in lockstep. Every
implementation step that changes a judgment ships with its named Lean
artifact in the same change, and the conformance oracle gates the merge.

A final representation audit of every Rust data structure feeds this
revision. Findings coupled to the cutover are absorbed below (§Representation
debt); two independent passes are split into sibling proposals —
[`exec-representation.md`](exec-representation.md) and
[`interior-debt.md`](interior-debt.md) — and gate nothing here.

## Deleted states

The representation is the feature. This table is the contract page: each
representation and the state or guard it deletes.

| Representation | State made unrepresentable / guard deleted |
|---|---|
| `Admission<T>` sum | An unadmitted value flowing where an admitted one is required; violations smuggled through the error channel. |
| `Check::{Holds, Violated}` | An error value used as a semantic verdict inside checkers. |
| `InstanceBuilder` with no query methods | Querying an unproved candidate. |
| `OwnedInstance` / `ReadInstance` with no mutation methods | Mutating an admitted instance. |
| Format-8 version gate | Opening a store without admission provenance. |
| Staging directory + atomic rename | A half-created store at a destination path. `Error::NotInitialized` and its healing branch are deleted with the state. |
| Synchronous callback reads (TS and C) | A read lease crossing `await` or escaping its callback. |
| `Executor::execute`'s signature | A join kernel that can read storage, identity, or a dictionary. |
| Codec capabilities (`CodecRead`/`CodecWrite`) | A generated fact codec that branches on the storage backend. Parametricity makes the branch untypable, not just unwritten. |
| Collection-only mutation verbs | Scalar mutation as a special case beside batch mutation. |
| Materialized fresh-key statements | Fresh-row collision as a separate check; it is an ordinary functionality violation. |
| One-discriminant TS `WriteOutcome` | Narrowing a write result on two different keys. |
| Exported TS error values | Host code matching error message strings. |
| `PutOutcome::{Inserted, Occupied}` | A Boolean whose polarity callers must remember. |
| Roster by exhaustive `StatementView` match | A statement form silently absent from complete admission; the omission fails to compile. |
| `StatementRef` as the one stored statement identity | A statement id whose arm must be re-derived — nine refutable narrowings, one silent fall-through, one runtime re-proof of a validation-minted pairing. |
| One `(CatalogIdentity, ViewEpoch)` validity stamp | A second process clock (`CommitSeq`) counting the same event; an `Option` whose `None` means three unrelated things. |
| `ConditionalWrite::Moved` | An expected compare-and-swap answer traveling the error channel. |
| `Violations` with private variants | A host-constructible empty, unsorted, or mismatched rejection payload. |
| One handle-phase word per FFI handle | Liveness, re-entrancy, and exclusivity as eight independent atomics whose illegal pairs are re-checked at four sites. |
| Refcounted C ref slots | An alive bit read out of a freed allocation. |
| One wire-tag table under the `tags.json` golden | Outcome kinds drifting between Rust string literals and hand-typed TS unions. |
| Typed per-field codec reads | A macro-emitted `unreachable!("schema-typed")` per field per direction. |

## Scope

This proposal adds all of the following:

- An allocation-bounded heap construction path.
- A packed immutable heap catalog.
- Complete initial admission from an unproved candidate.
- One generic catalog read and write vocabulary.
- Query preparation and execution over heap and LMDB catalogs.
- Raw catalog persistence from an admitted heap instance.
- Exact Rust, TypeScript, and C ownership surfaces.
- A new storage format whose ordinary open path may trust admission provenance.
- A reified publication protocol whose crash matrix is an iteration over data.
- A store-birth error taxonomy cutover.
- The Lean lockstep artifacts: complete-roster definitions, agreement
  theorems, the obligation partition theorem, and a complete-admission
  conformance lane.

This proposal does not add a general exact-cover dependency. Bumbledb admission
proves only the theory Bumbledb declares today.

This proposal does not add mutable heap instances. A host creates a new builder
when it needs a new heap instance.

This proposal does not add a WASM engine. The first implementation uses the
existing Rust engine through the native TypeScript binding.

## Current facts

The design starts from the code that exists at 0.14.0. Every row is verified
against the tree.

| Current fact | Consequence |
|---|---|
| `Snapshot<'db, S>` owns a `ReadTxn<'db>` in `crates/bumbledb/src/api/db.rs`. | A borrowed LMDB instance must remain closure-scoped. |
| `WriteTx<'a, S>` owns an in-memory `WriteDelta<'a>` over a read view; `WriteDelta<'s>` carries a borrowed `&'s Schema`. | The existing durable mutation algebra remains useful. Removing the borrowed schema field makes the delta lifetime-free. |
| `Db::write` returns `Result<R>` — the callback value only; the generation is consumed internally. TS `write` returns the generation and discards the callback value. C `write` returns only a status. | `Committed<R> { value, generation }` is new capability on all three surfaces, not a rename. |
| `write_from` takes `&Snapshot<'_, S>`; `write_from_witness` takes `Witness<S>` **by value**, with a recorded ruling that "the move is the API" — and `Witness` simultaneously derives `Copy`, so the recorded linearity is already void in the type. | The collapse onto one borrowed, cloneable witness is an overturning of a ruling the type never enforced, recorded below — not a preservation of it. |
| `plan_commit` enumerates inserted sources, disestablished target determinants, and touched capacity parents. | The current judgment is incremental. It is not a complete initial judgment. |
| The empty-delta shortcut keys on `delta.is_empty()` and still flushes escaped fresh floors under four pinned laws (flush before the generation read; the in-process floor rises before the disk write; a failed flush parks, retries at the next write begin, and poisons `reserve`; the panicking path burns through the `EscapedIdBurn` drop guard). | Any restructure of `commit()` around `Admission` carries all four laws unchanged. |
| The incremental restriction theorems assume the base already satisfies the theory. | A raw empty base cannot use the empty-delta shortcut. |
| `verify_store` walks every ordinary source, every closed source, every ordinary capacity parent, and every closed capacity parent — and it is `#[doc(hidden)]`. Functionality has no dedicated pass there; its completeness lives in the F/U namespace walks. | The codebase contains the semantic roster needed for complete admission, but not as one admission pass and not as public surface. |
| `Error::CommitRejected` is public API **and** internal control flow: checkers mint it per probe, `collect` destructures it, and the sweeper's finding path destructures it again. | Removing it rewires the checkers and the sweeper together, not just the public enum. |
| `PreparedQuery` stores `env_instance` — a process-local monotone counter, not a pointer — and performs a runtime owner check. | Ordinary Rust lifetimes do not prove value identity. A replacement still needs exact owner provenance. |
| `PreparedQuery<'s, S>` borrows `Schema`. The N-API crate transmutes it to `'static` under an owning `Arc` (`ts/crate/src/lib.rs:1486`). | Owning the shared schema in the prepared value removes the self-reference and deletes the transmute. |
| Plan drift is `PreparedQuery::staleness(&Snapshot)` returning `Staleness { per_occurrence: Box<[OccurrenceDrift]>, max_ratio }` — `#[doc(hidden)]`, harness-only. Nothing named `PlanDrift` exists, and `Db` has no drift accessor. | The drift surface keeps its names and hidden status; only its receiver changes with the rename. |
| The dense join kernel is `Executor::execute(plan, colts, bindings, sink, counters)` and is transaction-free. Today's `run_join` above it binds `ReadTxn`, `ImageCache`, and a generation. | Catalog abstraction belongs in the binding layer around the kernel. The purity gate names the kernel, not `run_join`. |
| `ViewGeneration { Storage(GenerationId), Closed }` already exists and threads through `ViewMemo`, `ParkedView`, and source dedup. | `ViewEpoch` is a rename-and-extend of that type, not a new one. |
| A concrete `struct SortedGets<'a>` already exists in `storage/commit/judgment.rs` (the T8 ascending-probe walker). | The trait takes the name; the existing struct becomes its LMDB implementation. |
| `_data` stores `F`, `M`, `U`, `R`, `S`, and `Q`. `_dict` stores both dictionary directions. `_meta` stores exactly six keys: format, fingerprint, generation, dictionary next-id, kind, and descriptor. The ephemeral dirty marker is deliberately a **sibling file**, never a `_meta` key, because the marker must be readable before any LMDB page is trusted. | Persistence must preserve the data and dictionary namespaces while synthesizing fresh metadata. No lifecycle state enters `_meta`. |
| `Db::create` and `Db::ephemeral` initialize into an existing **empty** directory today, and `create` heals a half-created store (`Error::NotInitialized`). `compact` refuses an existing destination with an untyped `Io(AlreadyExists)`. | The path-must-not-exist rule is a behavior break for mkdir-then-create hosts, and an error-taxonomy cutover, both recorded below. |
| `Db::ephemeral` is an explicit NOSYNC LMDB store kind with reopen, dirty-marker recovery, C, and benchmark semantics. The crashpoint test harness was owner-killed; `crashpoint!` sites are no-op atomicity names. | Ephemeral is not an in-memory mode and this proposal keeps it. Crash coverage must be rebuilt as data, not resurrected as a macro sweep. |
| TS reads are scope-shaped both ways today: `db.read(fn)` and a `using`-disposable `ReadScope` handle from `db.read()`, backed by a snapshot **worker** that owns the real `Db::read` closure across a request channel. The runtime thenable probe exists on the write path only. Exported error values exist (`ErrGenerationMoved`, `ErrNewtypeMismatch`); everything else is anonymous. | The worker is sound — it is deleted because the handle-shaped read is deleted, not because it fabricated a lifetime. The thenable probe extends to reads. The error-value idiom generalizes. |
| C surfaces rejection as `BDB_ERROR_KIND_COMMIT_REJECTED` through `bdb_error`; `bdb_abi_version()` returns 2 and nothing enforces it; snapshot-named functions exist (`bdb_db_read`, `bdb_snapshot_*`, `bdb_db_write_from`). | The rejection-as-status inversion, the kind-enum deltas, and the ABI bump are enumerated below. |
| There is no `Db::memory()` and no current public `InstanceBuilder`, `OwnedInstance`, `Instance` trait, `CatalogRead`, `FrozenCatalog`, `MutationCore`, or `CatalogIdentity`. | These names describe new API, not renames. |

The relevant implementation seams are:

- [`crates/bumbledb/src/storage/commit/write.rs`](../crates/bumbledb/src/storage/commit/write.rs)
- [`crates/bumbledb/src/storage/commit/plan.rs`](../crates/bumbledb/src/storage/commit/plan.rs)
- [`crates/bumbledb/src/storage/commit/judgment.rs`](../crates/bumbledb/src/storage/commit/judgment.rs)
- [`crates/bumbledb/src/verify_store.rs`](../crates/bumbledb/src/verify_store.rs)
- [`crates/bumbledb/src/image/build.rs`](../crates/bumbledb/src/image/build.rs)
- [`crates/bumbledb/src/exec/run/execute.rs`](../crates/bumbledb/src/exec/run/execute.rs)
- [`crates/bumbledb/src/api/prepared.rs`](../crates/bumbledb/src/api/prepared.rs)
- [`crates/bumbledb/src/api/db/write.rs`](../crates/bumbledb/src/api/db/write.rs)
- [`crates/bumbledb/src/error.rs`](../crates/bumbledb/src/error.rs)
- [`crates/bumbledb/src/storage/env.rs`](../crates/bumbledb/src/storage/env.rs)
- [`ts/src/db.ts`](../ts/src/db.ts)
- [`ts/crate/src/lib.rs`](../ts/crate/src/lib.rs)
- [`crates/bumbledb-c/src/db.rs`](../crates/bumbledb-c/src/db.rs)
- [`lean/Bumbledb/Txn.lean`](../lean/Bumbledb/Txn.lean)
- [`lean/Bumbledb/Decide.lean`](../lean/Bumbledb/Decide.lean)
- [`lean/Bumbledb/Admission.lean`](../lean/Bumbledb/Admission.lean)

## Semantic law

Let $T$ be a validated theory and $I$ be a finite candidate instance.

Initial admission has this meaning:

$$
\mathrm{admit}(T,I)=
\begin{cases}
\mathrm{Accepted}(\langle I,\;\mathrm{holds}(T,I)\rangle) &
  \text{if }\mathrm{holds}(T,I),\\
\mathrm{Rejected}(V_T(I)) & \text{otherwise.}
\end{cases}
$$

$V_T(I)$ is the complete violation set of the first failing phase. The key
phase preempts the statement phase. This is the same two-phase meaning as
Lean `Txn.judge`.

Incremental admission has a stronger input:

$$
s : \mathrm{State}(T), \qquad s.\mathrm{models} : \mathrm{holds}(T,s.\mathrm{inst}).
$$

For delta $d$, the incremental checker may inspect only the delta-derived
obligation roster because the base proof discharges every untouched
obligation.

These are two enumeration strategies for one judgment. They are not two
theories and not two violation types.

## Exact result representation

The engine result alias fixes the infrastructure error type. Theory rejection
therefore sits inside the successful outer result:

```rust
pub type Result<T> = std::result::Result<T, Error>;

pub enum Admission<T> {
    Accepted(T),
    Rejected(Violations),
}
```

The layers have distinct meanings:

- `Err(Error)` means schema, shape, poison, storage, corruption, resource, or
  host misuse failure. `Error::TransactionPoisoned` stays here. The
  unmeasurable ray (`Error::CapacityRayMeasure`, `Error::MeasureOfRay`) stays
  here on the commit path; the sweeper's existing demotion of it to a finding
  is unchanged.
- `Ok(Admission::Rejected(violations))` means the candidate formed correctly
  and failed the declared theory.
- `Ok(Admission::Accepted(value))` carries the only public value that may be
  treated as admitted.

`Error::CommitRejected` leaves the public API **and** the internal control
flow. Today checkers mint it per probe, `collect` destructures it back into
the collector, and `verify_store`'s finding path destructures it a second
time. All of those sites move onto a witnessed judgment:

```rust
enum Check {
    Holds,
    Violated(Violation),
}
```

The violation collector consumes `Check` values and seals the same sorted,
deduplicated `Violations` type for initial and incremental admission. The
sweeper's finding path consumes `Check` too; error-as-verdict does not
survive anywhere.

Violation citation carries a policy, not an accident:

- **Direction.** Complete admission cites containments in their own
  source-to-target orientation — the sweeper's existing convention, because a
  candidate, like a committed store, has no just-inserted side. The
  incremental checker keeps its two-direction citations. `Violations`' dedup
  key `(StatementId, Option<Direction>)` is unchanged.
- **Decoration.** Cited facts decode at rejection time, inside the boundary
  where provisional intern ids still resolve. Decoration is best-effort: a
  decode failure degrades the citation, and it never converts a `Rejected`
  into an `Err`. This carries the current `decorate_rejected` law forward
  verbatim.

Durable writes preserve the callback value only on acceptance:

```rust
pub struct Committed<R> {
    pub value: R,
    pub generation: GenerationId,
}

pub fn write<R>(
    &self,
    body: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
) -> Result<Admission<Committed<R>>>;
```

An aborting callback returns an outer error or host abort status. A rejected
write returns no callback value. Fresh values that escaped through host side
effects remain burned by the current never-reissue discipline — all four
pinned escaped-floor laws (flush before the generation read, in-process floor
before the disk write, parked retry poisoning `reserve`, the `EscapedIdBurn`
drop guard) carry over unchanged, including on the empty-delta shortcut.
The write *exit* is reified the way publication is reified below: today the
burn-disarm-drop-flush block is pasted three times inside `Db::write`, with
the ordering laws living in comments and in the declaration order of locals.
It becomes one `WriteEnd { Committed(..), Poisoned(..), Aborted(..) }` sum
folded through a single burn-and-flush step, so each law has exactly one
home and the three copies cannot drift.

Conditional writes borrow a structural witness:

```rust
#[derive(Clone)]
pub struct Witness<S> {
    identity: CatalogIdentity,
    generation: GenerationId,
    marker: PhantomData<fn() -> S>,
}

pub enum ConditionalWrite<R> {
    Accepted(Committed<R>),
    Rejected(Violations),
    Moved {
        witnessed: GenerationId,
        current: GenerationId,
    },
}

pub fn write_from<R>(
    &self,
    witness: &Witness<S>,
    body: impl FnOnce(&mut WriteTx<'_, S>) -> Result<R>,
) -> Result<ConditionalWrite<R>>;
```

The conditional verb has one more proved outcome than the plain one, and it
is an *outcome*, not an error. A compare-and-swap whose comparison fails did
not fail; it reported — the current `GenerationMoved` doc concedes it
("retry is host policy"). `Error::GenerationMoved` is therefore deleted with
`Error::CommitRejected`: the moved generation becomes the `Moved` arm,
carrying the two generations a retry loop actually reads. The criterion that
separates it from the errors that stay: the caller *proceeds on the data in
the answer* (re-read, re-witness, retry). `FreshExhausted` and
`DerivedBudgetExceeded` stay errors deliberately — their work is abandoned,
not answered — and `CapacityRayMeasure` stays by its recorded C10 ruling (an
undefined measure is not a verdict).

The current `Witness<S>` already carries exactly this payload — an instance
token and a generation — and is spent by move under a recorded ruling that
"the move is the API." **That ruling is overturned here, and it was already
void**: today's `Witness` derives `Copy`, so the move consumes nothing and
the recorded linearity has never been enforced by the type. The witness's
data is the entire read premise; the generation comparison inside the writer
critical section is the real guard; linearity was control-flow discipline
duplicating what the representation already proves. A witness is evidence,
and evidence does not wear out. `Witness` becomes `Clone` (not `Copy` — the
identity is an `Arc`), `write_from` borrows it, and a host may justify many
conditional writes from one read. The stale `#[expect(needless_pass_by_value)]`
and its reason text are deleted in the same change.

`ReadInstance::witness` mints the witness from the same transaction that
served the read. The witness may outlive the read callback. It is not a
public generation integer. `write_from` compares `CatalogIdentity` first —
a foreign witness is host misuse and stays `Err(ForeignWitness)`. It compares
the generation inside the writer critical section second — a moved generation
is the `Ok(ConditionalWrite::Moved { .. })` answer. Both are decided before
the callback runs.

The old `write_from(&Snapshot, ...)` and `write_from_witness` pair collapses to
this one operation. The witness is the one representation of the read premise
across Rust, TypeScript, and C. `Error::ForeignSnapshot` is replaced by
`Error::ForeignWitness`.

## Public Rust representations

### Heap construction

```rust
pub struct InstanceBuilder<S> {
    mutation: MutationCore<HeapMutation, S>,
}

struct HeapMutation {
    stage: HeapStage,
}
```

`InstanceBuilder<S>` owns all mutable construction state. It offers collection
`load`, collection `delete`, `reserve`, overlay `contains`, and overlay keyed
`get`. `WriteTx` retains the collection name `insert` in place of `load`.
The two names enter the same private collection protocol. The builder offers no
query preparation or execution.

`admit` consumes the builder:

```rust
impl<S: Theory> InstanceBuilder<S> {
    pub fn new(theory: S) -> Result<Self>;

    pub fn load<'f, F>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport>
    where
        F: Fact<'f, Schema = S>;

    pub fn delete<'f, F>(
        &mut self,
        facts: impl IntoIterator<Item = &'f F>,
    ) -> Result<MutationReport>
    where
        F: Fact<'f, Schema = S>;

    pub fn reserve<T: Fresh<Schema = S>>(
        &mut self,
        count: u64,
    ) -> Result<FreshRange<T>>;

    pub fn admit(self) -> Result<Admission<OwnedInstance<S>>>;
}
```

`MutationReport { submitted, changed }` is the existing type; the builder and
`WriteTx` return the same one. The whole input collection is parsed before any
member mutates the builder. The empty collection is a no-op. One row is the
singleton collection. No scalar mutation API exists.

`InstanceBuilder::load_dyn` and `delete_dyn` accept row-major dynamic
collections. `WriteTx::insert_dyn` and `delete_dyn` retain the same shape. All
four methods parse the full collection into accepted row variants before they
apply any member. N-API columns and C row-major buffers lower into that same
accepted collection representation.

### Durable mutation

```rust
pub struct WriteTx<'db, S> {
    mutation: MutationCore<StoreMutation<'db>, S>,
}

struct StoreMutation<'db> {
    view: ReadTxn<'db>,
    delta: WriteDelta,
}
```

`WriteTx` retains the current closure-scoped LMDB lifecycle. Its `insert` has
the same collection semantics as builder `load`. Its other collection verbs
match the builder. It uses incremental admission because its base came from a
new-format admitted store.

One private `MutationCore<M, S>` owns the `Arc<Schema>`, reusable encode
scratch, phase, and collection protocol. Its `M` representation owns:

- Net fact dispositions.
- Pending dictionary interns.
- Fresh reservations.
- Base lookup, if a base exists.

The generic core coordinates:

- Parse-all-first collection application.
- Overlay membership and keyed lookup.
- Clean, applied, and poisoned construction phases.

`HeapMutation` uses the chunked heap stage. `StoreMutation` uses the existing
delta over a read transaction. `WriteDelta` drops its borrowed schema field;
operations receive `&Schema` from `MutationCore`. This makes the delta
lifetime-free and avoids a self-reference in the owning builder.

The LMDB write transaction still begins only after the write callback finishes
and the commit plan exists. `LmdbWriteCatalog` wraps that commit transaction.
The generic core does not extend the LMDB write-lock duration.

The mutation-state parameter supplies committed lookup and counter floors. It
does not erase the different public lifecycles.

### Admitted instances

```rust
pub struct OwnedInstance<S> {
    core: InstanceCore<FrozenSource, S>,
}

pub struct ReadInstance<'txn, S> {
    core: InstanceCore<LmdbSource<'txn>, S>,
    thread_bound: PhantomData<Rc<()>>,
}
```

`OwnedInstance<S>` owns:

- An `Arc<Schema>`.
- A `FrozenCatalog`.
- A process-local `CatalogIdentity`.
- Lazy frozen image slots.

`ReadInstance<'txn, S>` owns the closure's `ReadTxn<'txn>`. The transaction
borrows the environment. `LmdbSource` borrows the owner **once** — it does
not reproduce today's `Snapshot` shape, which dissolves its owner into three
parallel projections (`&ImageCache`, `&Schema`, `&ScratchPool`) plus a
phantom that exists only because the owner was dissolved. One owner borrow
carries the cache, the scratch pool, the schema, and the typestate together.
The instance owns cheap clones of the store's `Arc<Schema>` and
`CatalogIdentity`.

Both delegate to generic `InstanceCore<C, S>` algorithms. There is no
`CatalogView` field, no `enum Store` in the engine, and no GAT-bearing trait
object.

The durable read boundary remains lexical:

```rust
impl<S> Db<S> {
    pub fn read<R>(
        &self,
        body: impl FnOnce(&ReadInstance<'_, S>) -> Result<R>,
    ) -> Result<R>;
}

impl<S> ReadInstance<'_, S> {
    pub fn witness(&self) -> Result<Witness<S>>;
}
```

The read transaction parks only after `body` returns and the instance is
invalidated by Rust lifetime. The parked-reader optimization remains behind
`Db::read`.

A sealed public trait permits generic host code without permitting foreign
implementations:

```rust
pub trait Instance<S>: private::Sealed {
    fn prepare(&self, query: &Query) -> Result<PreparedQuery<S>>;
    fn execute(
        &self,
        prepared: &mut PreparedQuery<S>,
        params: &[ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()>;
    // Plus: scan, scan_facts, contains, contains_dyn, get, get_dyn,
    // row_count, profile.
}
```

The roster is exact about provenance:

- `scan`, `scan_facts`, `contains`, `contains_dyn`, `get`, `get_dyn`, and
  `profile` lift from today's `Snapshot`.
- `execute` is one entry point. Today's `execute`/`execute_args` twins
  collapse onto it: `ParamArg::Scalar` already embeds `BindValue`, so the
  scalar-only spelling was a duplicate entry, and duplicates are control
  flow. The collect wrappers ride the one entry.
- `row_count(relation)` is **new public API**. Nothing named `row_count`
  exists on `Snapshot` today; the count reaches only the planner through
  crate-internal paths. It is admitted here because a host sizing its reads
  is a legitimate consumer of a number the engine already maintains.
- `profile` is **promoted** from `#[doc(hidden)]`. It stays a diagnostic
  whose stats shape is explicitly unfrozen, and the promotion is stated
  rather than smuggled. The promotion is gated on fixing the shape first:
  `RuleStats` today is one tag beside four fields that must be empty under it
  (hand-filled `Vec::new()` at two sites) with `key_probe: Option<..>` as the
  real discriminant, and `KeyProbeStats.hit` is derived **two different
  ways** at its two construction sites. It becomes
  `enum RuleStats { KeyProbe { .. }, FreeJoin { .. } }` — the shape
  `StatsBody` already uses one level up — with one derivation of `hit`,
  before any of it goes public.

Static dispatch selects `FrozenSource` or `LmdbSource`. The dense executor sees
the same relation-image types in both cases.

### Borrowed facts and owned answers

`Fact<'a>` continues to borrow variable-width values from the method receiver:

```rust
fn get<'a, K>(&'a self, key: K) -> Result<Option<K::Fact>>
where
    K: Key<'a, Schema = S>;
```

`'a` is the borrow of `self`. It is not widened to the LMDB transaction
lifetime.

Dynamic FFI scans continue to copy variable-width values. `Answers` continues
to own finalized string and byte payloads. No TypeScript or C value borrows an
LMDB page or a frozen-catalog byte range.

### Fact codec capabilities

The current generated `Fact` implementation has five methods that name
`Snapshot` and `WriteTx` nominally: `encode_write`, `encode_delete`,
`encode_read`, `decode`, and `decode_write`. That representation cannot encode
a fact through `InstanceBuilder` and cannot decode a fact through
`OwnedInstance` — and because the receivers are nominal, nothing but
convention stops a codec from depending on its backend.

The cutover replaces the nominal receivers with sealed codec capabilities:

```rust
#[doc(hidden)]
pub trait CodecRead<S>: private::Sealed {
    fn schema(&self) -> &Schema;
    fn lookup_str(&self, value: &str) -> Result<Option<InternId>>;
    fn resolve_str<'a>(&'a self, id: InternId) -> Result<&'a str>;
}

#[doc(hidden)]
pub trait CodecWrite<S>: CodecRead<S> {
    fn intern_str(&mut self, value: &str) -> Result<InternId>;
}
```

The names describe dependencies. They do not describe storage backends. This
is parametricity doing the work a review comment used to do: a codec generic
over `CodecRead` cannot branch on the concrete catalog because the type gives
it nothing to branch on.

The method mapping is total — all five current methods are accounted for:

- `encode_write` becomes `encode_insert` over `CodecWrite`.
- `encode_read` and `encode_delete` collapse into one `encode_probe` over
  `CodecRead`.
- `decode` and `decode_write` collapse into one `decode` over `CodecRead`.
  Their current difference is resolution **order**, not shape: `decode_write`
  resolves pending interns before the committed dictionary. That order moves
  into the implementations — `MutationCore`'s `CodecRead` resolves pending
  interns first (the existing `pending_raw` mechanism), an admitted
  instance's `CodecRead` resolves only its catalog dictionary. The order is
  carried by the representation, so generated code cannot get it wrong.

```rust
pub trait Fact<'a>: Sized {
    type Schema;
    const RELATION: RelationId;

    fn encode_insert<C>(
        &self,
        context: &mut C,
        out: &mut Vec<u8>,
    ) -> Result<()>
    where
        C: CodecWrite<Self::Schema>;

    fn encode_probe<C>(
        &self,
        context: &C,
        out: &mut Vec<u8>,
    ) -> Result<Probe>
    where
        C: CodecRead<Self::Schema>;

    fn decode<C>(context: &'a C, fact: &[u8]) -> Result<Self>
    where
        C: CodecRead<Self::Schema>;
}
```

`Probe` is the house `PutOutcome` rule applied to the codec — the current
`Result<bool>` returns whose `false` means "a dictionary miss proves the fact
absent" become a named sum:

```rust
pub enum Probe {
    Encoded,
    ProvablyAbsent,
}
```

`Key<'a>` receives the same change. Its `determinant_read` and
`determinant_write` pair collapses into one determinant encoder generic over
`CodecRead`, returning the same `Probe`. The derive macro has four emission
sites for that pair (fresh newtypes and generated key structs); all four move
to the one generic emission.

The codec cutover also settles the value vocabulary it rides on, because this
is the one pass that touches every emission site:

- **`InternId(u64)` replaces bare `u64` dictionary ids.** The reserved
  sentinel (`u64::MAX`, asserted never minted) becomes an associated
  constant with one owner; today it is duplicated as a literal in
  `read_meta.rs` and guarded by a runtime `assert!` in the mint path.
- **`ValueType` and `TypeDesc` merge.** They are the same seven-arm enum
  spelled twice with a sixteen-site re-tagging conversion between them;
  `ValueType` gains `Copy` and `TypeDesc` is deleted.
- **`ValueRef` drops its `Fixed*` arms.** Fixedness already lives in the
  layout; carrying it again on the value makes
  `encode_fact(&[IntervalU64], fixed_interval_layout)` a silent 16-bytes-
  into-8 corruption. Field writers take the layout's type; the width has one
  home.
- **Typed per-field decode.** `decode_field` already knows each field's arm
  from the layout and discards that proof into the wide sum, forcing the
  macro to emit `unreachable!("schema-typed")` per field per direction.
  `CodecRead` grows typed entry points (`decode_u64_field`,
  `decode_interval_field`, …); the macro emits direct reads with no match
  and no panic branch.
- **`Value::String` carries `Box<str>`.** UTF-8-ness is proved where the
  value is born instead of re-derived and discarded at eight boundaries
  (`ValueMismatch::Utf8`, `value_matches_parsing`, one `expect`, one
  `.ok()?` all delete). `FixedBytes` rides the already-checked
  `FixedBytesValue` form the encoding layer defines.

Dynamic row parsing uses the same codec capabilities. Typed rows, dynamic rows,
row-major C batches, and TypeScript columns therefore converge before
mutation. No host transport receives its own dictionary or fact codec.

### Auto-traits

The implementation pins these contracts with compile tests:

- `OwnedInstance<S>` is `Send + Sync`.
- `ReadInstance<'txn, S>` is `!Send + !Sync`.
- `InstanceBuilder<S>` is `Send + !Sync`.
- `WriteTx<'db, S>` is `!Send + !Sync`.
- `PreparedQuery<S>` is `Send + !Sync`.

The explicit thread-bound marker on `ReadInstance` prevents accidental claims
based on a future dependency's auto-trait change. `InstanceBuilder`'s `Send`
is load-bearing: the TypeScript binding hands the builder to an async native
task for admission.

## Complete initial admission

### Why `judge(empty_delta)` is wrong

The current commit plan enumerates only obligations touched by a delta. An
empty plan therefore enumerates nothing.

That misses at least these lawful schema shapes:

- A closed source row requiring an ordinary target row.
- A closed capacity parent with a positive floor and no children.
- A pre-existing ordinary source obligation that no delta fact touches.
- An ordinary capacity parent whose child group is empty and whose floor is
  positive.

The incremental checker is sound only because `State.models` proves those
untouched obligations already held. `InstanceBuilder` has no such premise.

### Complete obligation roster

The roster is not a hand-list. `CompleteObligations` is derived by an
**exhaustive match over the materialized statement spine** — every
`StatementView` arm, every enforcement arm inside it. A statement form absent
from the roster is a compile error, not an audit finding. The previous draft
enumerated the roster in prose; prose rosters rot, and this one already had a
special case the representation had dissolved.

The input is already a validated `Schema`. Schema validation has discharged
the instance-independent obligations — closed functionality, closed-to-closed
containments, closed-constant capacity — by refuting self-refuting theories
at declaration time. Complete admission enumerates every obligation whose
truth can still depend on the candidate's ordinary facts. The Lean bridge
composes those two witnesses through the partition theorem named below.

The key phase checks every functionality statement:

1. Every scalar functionality determinant of every ordinary fact.
2. Every pointwise functionality group of every ordinary fact.

This **includes the materialized fresh-key statements**. Lean already proves
the fresh key is an ordinary functionality statement riding plain `holds` —
values that happen to have a generator, never a generator law
(`lean/Bumbledb/Txn/Fresh.lean`). A "fresh-row identity collision" is a
scalar functionality violation of that materialized statement; listing it as
a third check would resurrect a special case the representation deleted.

The statement phase runs only if the key phase has no violation. It checks,
per containment statement, under **both** enforcement arms — the scalar probe
and the interval coverage sweep:

1. Every ordinary source fact satisfying each containment source selection.
2. Every sealed row of each closed source satisfying that selection.

and per capacity statement:

3. Every ordinary parent fact satisfying each capacity target selection.
4. Every sealed row of each closed capacity parent satisfying that selection.

This roster includes empty child groups. A positive floor can therefore fail
without any child fact existing. Interval positions on closed containments
are refused at validation today, so sealed rows meet only scalar probes; if
that refusal ever lifts, the exhaustive match forces this roster to grow with
it.

The complete and incremental enumerators feed the same `Checker`, `Probe`,
coverage sweep, capacity measure, selection encoding, and `Violations`
collector. The implementation must not copy those semantic mechanisms out of
`judgment.rs` or `verify_store`.

`verify_store` remains a coherence and corruption sweep, and remains
`#[doc(hidden)]` harness surface — this proposal does not promote it.
Complete admission factors the sweep's statement-phase roster into the
reusable `CompleteObligations` iterator. The key phase is not factored from
the sweep (there, functionality completeness lives in the F/U namespace
walks); complete admission's key phase falls out of the canonical merge
instead, with identical verdicts as a gate. Store findings are never
admission violations.

### Initial pipeline

```text
InstanceBuilder
    │ collection parse and net dispositions
    ▼
HeapStage
    │ canonical fact order and deterministic row-id assignment
    ▼
CandidateRuns
    │ derive F/M/U/R/S/Q and dictionary entries
    │ collect the complete key phase
    ├── key violations ──► Rejected(Violations)
    ▼
CandidateCatalog
    │ complete containment and capacity roster
    ├── statement violations ──► Rejected(Violations)
    ▼
FrozenCatalog
    │ attach schema, identity, and lazy image slots
    ▼
Accepted(OwnedInstance)
```

`CandidateCatalog` is readable but not public and not admitted. The successful
typestate conversion into `FrozenCatalog` is zero-copy. Rejection discards the
candidate after decorating cited facts through the still-live candidate
dictionary, under the best-effort decoration policy above.

No `Instance` value exists before the complete judgment succeeds.

### Incremental pipeline

```text
WriteTx over an admitted format-8 base
    │ collection parse and net dispositions
    ▼
CommitPlan
    │ delete phase, insert phase, key conflict collection
    ├── key violations ──► Rejected(Violations)
    ▼
LmdbWriteCatalog read-your-writes view
    │ inserted sources
    │ disestablished target determinants
    │ touched capacity parents
    ├── statement violations ──► Rejected(Violations)
    ▼
counter and dictionary flush
    ▼
generation advance and LMDB commit
    ▼
Accepted(Committed<R>)
```

The empty-delta shortcut remains legal only here. Its base is already admitted.
It still flushes escaped fresh floors under the four pinned laws.

## Catalog algebra

### Pinned compiler facts

The repository pins `nightly-2026-08-15`, currently
`rustc 1.99.0-nightly (d453bdd8f 2026-08-14)`.

The catalog design uses GATs. GATs themselves are stable. This repository does
not restrict the surrounding design to stable Rust. Two relevant limitations
still reproduce on the pinned nightly:

- A trait containing a GAT is not dyn-compatible and produces `E0038`.
- A broad bound such as `for<'a> I::Item<'a>: Debug` still implies an
  erroneous `'static` requirement and produces `E0597`.

Every GAT therefore carries the required bound that `Self` outlives its catalog
borrow. Catalog algorithms remain generic. Bounds stay on the method or
concrete use site. The design does not place broad higher-ranked bounds on
lending items.

The lending cursors use direct loops. They do not depend on `filter`-style
adapters whose returned items keep a mutable borrow across loop iterations.

### Ordering is representation, not denotation

This section's vocabulary is ordered on purpose, and the order means nothing.

The codebase holds three distinct things that could be called "ordering," and
they must not be confused:

1. **Removed ordering features.** ArgMax/ArgMin and CountDistinct are killed;
   Min/Max in capacity law position is refused because such windows are not
   delta-restrictable; answer ordering has never existed — "answers are a
   set — the host sorts" is recorded policy in the bench lanes and the
   binding alike.
2. **Retained value-order semantics.** `Min`/`Max` folds over orderable
   types, `WordCmp` comparison predicates, half-open interval membership,
   Allen configurations, the coverage sweep, and `Duration` capacity
   measures. These are order over the value domain — the time axis is the
   problem, and no representation dissolves essential complexity.
3. **Representational order.** The byte-ordered catalog, value-faithful key
   codecs, the canonical `(relation, fact_hash)` commit order, sorted
   deduplicated `Violations`, the materialized statement spine, sorted probe
   groups, and everything this section adds: sorted runs, `FrozenMap`,
   neighbor probes, monotone gets.

Class 3 is a chosen coordinate system, not a semantic claim. In it, equality
is byte equality, duplicates are adjacent so dedup is structural, pointwise
overlap is neighbor adjacency, and coverage gaps are visible to one linear
sweep. Remove the order and each of those becomes a guard or a probe loop.
Lean pins the split in writing: the denotation is order-free by construction
(`Delta` is set algebra; "the final-state judgment is order-free";
"sortedness and dedup are representation" — `lean/Bumbledb/Txn.lean`). The
ordered algebra below is entirely class 3.

### Ordered map contracts

```rust
pub(crate) struct Entry<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

pub(crate) struct Bounds<'a> {
    pub start: Bound<&'a [u8]>,
    pub end: Bound<&'a [u8]>,
}

#[derive(Clone, Copy)]
pub(crate) enum CatalogMap {
    Data,
    Dictionary,
}

pub(crate) trait ReadCursor {
    fn next(&mut self) -> Result<Option<Entry<'_>>>;
}

pub(crate) trait WriteCursor: ReadCursor {
    fn del_current(&mut self) -> Result<()>;
}

pub(crate) trait SortedGets {
    type Value<'a>: AsRef<[u8]>
    where
        Self: 'a;

    fn reset(&mut self);

    fn get<'a>(&'a mut self, key: &[u8])
        -> Result<Option<Self::Value<'a>>>;
}

pub(crate) trait OrderedRead {
    type Value<'a>: AsRef<[u8]>
    where
        Self: 'a;

    type Range<'catalog, 'bounds>: ReadCursor
    where
        Self: 'catalog;

    type Gets<'a>: SortedGets
    where
        Self: 'a;

    fn get<'a>(&'a self, map: CatalogMap, key: &[u8])
        -> Result<Option<Self::Value<'a>>>;

    fn lower<'a>(&'a self, map: CatalogMap, key: &[u8])
        -> Result<Option<Entry<'a>>>;

    fn greater<'a>(&'a self, map: CatalogMap, key: &[u8])
        -> Result<Option<Entry<'a>>>;

    fn greater_or_equal<'a>(&'a self, map: CatalogMap, key: &[u8])
        -> Result<Option<Entry<'a>>>;

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<Self::Range<'catalog, 'bounds>>;

    fn sorted_gets<'a>(&'a self, map: CatalogMap)
        -> Result<Self::Gets<'a>>;

    fn len(&self, map: CatalogMap) -> Result<u64>;
}

pub(crate) trait OrderedWrite: OrderedRead {
    type WriteRange<'catalog, 'bounds>: WriteCursor
    where
        Self: 'catalog;

    fn put(
        &mut self,
        map: CatalogMap,
        key: &[u8],
        value: &[u8],
    ) -> Result<()>;

    fn put_no_overwrite(
        &mut self,
        map: CatalogMap,
        key: &[u8],
        value: &[u8],
    ) -> Result<PutOutcome>;

    fn delete(&mut self, map: CatalogMap, key: &[u8]) -> Result<bool>;

    fn range_mut<'catalog, 'bounds>(
        &'catalog mut self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    )
        -> Result<Self::WriteRange<'catalog, 'bounds>>;
}
```

`PutOutcome` is an enum with `Inserted` and `Occupied`. It is not a Boolean
whose polarity callers must remember.

`del_current` may run exactly once after a yielded entry and before the next
cursor move. Calling it in any other state is an internal misuse error.

The range GAT carries separate catalog and bound lifetimes. An LMDB cursor may
retain the terminal bound while it walks. A frozen cursor may reduce both
bounds to indices during construction. The trait does not force either backend
to allocate owned bound bytes.

`SortedGets` names an existing mechanism. The concrete T8 ascending-probe
walker in `judgment.rs` today is a struct of the same name; the trait takes
the name, and that struct becomes the LMDB implementation behind
`LmdbReadCatalog::Gets`. `reset` starts a new monotone group. `get` requires
nondecreasing keys until the next reset. Debug builds assert that
precondition. The trait landing absorbs the walker's two representation
debts: its two `Option` fields encode a three-state cursor whose
`(None, Some(_))` cell is representable and silently ignored — the state
becomes one sum — and its group-boundary reset currently lives in the
*caller* (`check_source`'s `group: Option<ContainmentId>` plus a manual
`reset()`); the trait's `get` takes the group key, so the walker owns its own
soundness.

### Catalog contracts

```rust
pub(crate) struct FactEntry<'a> {
    pub row: u64,
    pub bytes: &'a [u8],
}

pub(crate) trait FactCursor {
    fn next(&mut self) -> Result<Option<FactEntry<'_>>>;
}

pub(crate) trait CatalogRead: OrderedRead {
    type Facts<'a>: FactCursor
    where
        Self: 'a;

    fn scan_facts<'a>(&'a self, relation: RelationId)
        -> Result<Self::Facts<'a>>;

    fn fetch_fact<'a>(&'a self, relation: RelationId, row: u64)
        -> Result<Option<Self::Value<'a>>>;
    fn membership_row(&self, relation: RelationId, hash: &[u8; 32])
        -> Result<Option<u64>>;
    fn determinant_row(&self, key: &[u8]) -> Result<Option<u64>>;

    fn row_count(&self, relation: RelationId) -> Result<u64>;
    fn row_id_high_water(&self, relation: RelationId) -> Result<u64>;
    fn fresh_next(&self, relation: RelationId, field: FieldId)
        -> Result<u64>;

    fn dict_lookup(&self, raw: &[u8]) -> Result<Option<u64>>;
    fn dict_resolve<'a>(&'a self, id: u64)
        -> Result<Self::Value<'a>>;
    fn dict_next_id(&self) -> Result<u64>;
}

pub(crate) trait CatalogWrite: CatalogRead + OrderedWrite {
    fn set_row_count(&mut self, relation: RelationId, value: u64)
        -> Result<()>;
    fn set_row_id_high_water(&mut self, relation: RelationId, value: u64)
        -> Result<()>;
    fn set_fresh_next(&mut self, relation: RelationId, field: FieldId, value: u64)
        -> Result<()>;
    fn set_dict_next_id(&mut self, value: u64) -> Result<()>;
}
```

`Bounds::all()` is the unbounded pair used for raw export. `CatalogMap` names
the two physical ordered maps. It does not name a storage
backend. `LmdbReadCatalog`, `LmdbWriteCatalog`, and `FrozenCatalog` dispatch that
namespace inside their own representation. `determinant_row` takes a composed
key because the caller has already prefixed relation and statement — the
current `determinant_row_for_key` shape. The total entry count consumers read
is `OrderedRead::len(CatalogMap::Data)` — `mdb_stat` on the LMDB side, the
record count on the frozen side. An earlier draft of this proposal carried a
separate `data_entries` method beside `len`; that was a duplicated truth
inside the contract itself, and it is deleted.

The contract includes all current consumers:

- `F` scan and fetch.
- `M` membership probe by 32-byte fact hash.
- `U` determinant probe.
- `R` prefix walks.
- `S` row count and row-id high-water.
- `Q` next value by relation and field.
- Dictionary forward lookup and reverse resolution.
- Dictionary next-id.
- Raw ordered enumeration of all `_data` and `_dict` entries.
- Total entry count for image allocation ceilings.
- Lower, greater, and greater-or-equal neighbor probes.
- Mutable range deletion through `del_current`.
- `put_no_overwrite`.
- Reusable monotone `SortedGets`.

Raw persistence enumerates
`range(CatalogMap::Data, Bounds::all())` and
`range(CatalogMap::Dictionary, Bounds::all())`. It never enumerates source
`_meta`.

`LmdbReadCatalog<'txn>`, `LmdbWriteCatalog<'txn>`, and `FrozenCatalog`
implement this vocabulary. `CandidateCatalog` implements read only after its
key phase succeeds.

The checker, applier, image builder, key-probe executor, binder, finalizer, and
store sweeper are generic over the smallest catalog capability they use.

No `dyn CatalogRead` exists. No `enum { Lmdb, Frozen }` reaches those
algorithms. An enum is permitted only in a cold FFI adapter that must erase a C
or JavaScript ownership distinction.

## Packed heap representation

### Refused baseline

`BTreeMap<Vec<u8>, Vec<u8>>` is not an acceptable heap catalog. It allocates
per key, per value, and per tree node. Initial admission would then retain all
of these at once:

- The input fact arena.
- Net-disposition indexes.
- Pending dictionary state.
- Per-fact commit-plan boxes.
- `F`, `M`, `U`, and `R` map nodes.
- `S` and `Q` entries.
- Relation images.

That representation reproduces the multi-gigabyte peaks observed in Primer's
own normalization loads even when the final encoded catalog is compact. The
Primer normalization corpus in the sibling `primer-spec` repository is the
reference workload; the acceptance gates name it.

### Staging representation

`HeapStage` uses these owned regions:

- A chunked fact-byte arena.
- A compact `FactRef` table containing relation, hash, byte offset, and length.
- One open-addressed identity index over `(relation, fact_hash)`.
- A chunked dictionary-byte arena.
- Compact dictionary forward slots containing hash, id, offset, and length.
- Dense fresh-counter floors indexed by the schema's fresh-field roster.

No fact and no dictionary value owns its own heap allocation. Chunk ownership
replaces a general object pool. Buffers move forward between phases and are
reused at one known representation boundary.

### Canonical run construction

Admission consumes `HeapStage`.

1. Sort `FactRef` values by the existing deterministic
   `(relation, fact_hash)` order.
2. Assign non-fresh row ids in that order.
3. Preserve the first fresh value as the row id on fresh-keyed relations.
4. Stream-derive encoded `F`, `M`, `U`, and `R` entries.
5. Emit bounded sorted runs into chunk arenas.
6. Detect duplicate exact keys and pointwise-neighbor overlaps while merging.
7. Mark staged dictionary ids referenced by net-live ordinary facts.
8. Emit only those dictionary entries in both directions.
9. Emit `S` row counts, `S` high-waters, `Q` floors, and the dictionary
   next-id above every emitted id.
10. Merge each namespace once into its frozen map.

Step 6 **is** the complete key phase: in canonical order, a duplicate exact
key is adjacent and a pointwise overlap is a neighbor. The coordinate system
does the checking.

A string interned only by a canceled or deleted fact does not enter the frozen
dictionary. No dictionary id escapes the builder API. Admission may therefore
discard dead intern entries and close unused tail holes. It does not renumber
an id embedded in a live fact.

The initial path does not allocate the current boxed `CommitPlan` per fact.
`MutationCore` shares collection semantics with durable writes. The heap
freezer uses a streaming full-build representation because its base and its
obligation roster are different.

### Frozen map

Each frozen ordered map has this shape:

```rust
struct FrozenMap {
    records: Box<[u8]>,
    offsets: Box<[u64]>,
}
```

Each record contains:

```text
key_len: u32 | value_len: u64 | key bytes | value bytes
```

`offsets[i]` locates record `i`. Keys are strictly increasing. Binary search
parses only the compared record headers. A range cursor stores two indices.
Borrowed values point into `records`.

`u64` offsets and value lengths avoid a 4 GiB catalog or dictionary-value
ceiling. Keys remain bounded by the existing key codec and fit `u32`.

`FrozenCatalog` contains one `FrozenMap` for `_data`, one for `_dict`, and the
dictionary next-id. It does not contain `_meta`.

### Peak-memory contract

Let:

- $A$ be staged fact and dictionary arena capacity.
- $I$ be compact staging-index capacity.
- $R$ be sorted-run bytes and run descriptors.
- $F$ be frozen record bytes and offsets.
- $J$ be complete-judgment scratch.
- $G$ be the relation images actually demanded after admission.
- $X$ be prepared-plan, execution-scratch, and answer capacity.

The implementation records the five admission quantities and the two later
execution quantities. Its admission phase ordering must satisfy:

$$
P_{\mathrm{admit}} \le \max(A+I+R,\; A+R+F+J).
$$

The first term bounds staging and run emission; the second bounds the merge,
where runs drain as frozen records form and judgment scratch is live. The
staging identity index drops before complete judgment. Relation images do not
build during admission. Query-time peak is measured separately against
$F+G+X$.

The release gate rejects any implementation that retains **any** of
`WriteDelta`, a boxed `CommitPlan`, or per-entry map nodes concurrently with
the frozen catalog.

## Preparation and execution

### Schema ownership

`Db`, `OwnedInstance`, and `PreparedQuery` share `Arc<Schema>`.

`PreparedQuery<S>` no longer has a schema lifetime. The TypeScript binding no
longer transmutes `PreparedQuery<'_>` to `PreparedQuery<'static>`
(`ts/crate/src/lib.rs:1486` is deleted). Drop order is not used as a
substitute for a representable owner.

### Exact prepared provenance

A lifetime does not identify a Rust value. Two unrelated instances can be
borrowed for the same inferred lifetime. Therefore
`PreparedQuery<'store, S>` would not make A-on-B execution fail to compile.

Each owner instead mints this private structural reference:

```rust
#[derive(Clone)]
struct CatalogIdentity(Arc<CatalogIdentityCell>);

impl CatalogIdentity {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
```

The identity is:

- Process-local.
- Not persisted.
- Not derived from semantic payload.
- Shared by every read instance of one open `Db`.
- Unique to one `OwnedInstance`.
- Held by every prepared query.
- Held by every retained conditional-write witness.

`execute`, `profile`, and staleness inspection compare identity before
binding, cache access, or scratch mutation. A mismatch returns
`Error::ForeignPreparedQuery`.

This check lives at the execution boundary. It does not live in the join
kernel, a COLT, an image, or a row loop.

A rank-2 generative brand could remove this check only by forcing every owned
instance operation into a lexical scope. That would make the Rust owned handle,
TypeScript handle, and C handle materially worse while the FFI boundaries still
needed runtime identity. This proposal chooses the one honest boundary check.

### Reusable durable plans

`Db::prepare` remains. It prepares against a short current read transaction and
returns a reusable `PreparedQuery<S>`.

The plan may execute across later generations of the same open store. The
drift surface keeps its exact current names and hidden status:
`PreparedQuery::staleness` stays `#[doc(hidden)]` harness-only, and its
receiver becomes `&ReadInstance<'_, S>`. Nothing named `PlanDrift` exists and
nothing new is invented here — but the result gains its honest sum. Today
`max_ratio == 1.0` means both "nothing drifted" and "nothing was pinned" (a
key probe reads no statistics), an in-band sentinel a host cannot
disambiguate, and `per_occurrence`'s "occurrence-id order" is a comment on a
type that dropped the id:

```rust
pub enum Staleness {
    NoStatistics,
    Measured { per_occurrence: Box<[OccurrenceDrift]>, max_ratio: f64 },
}
```

with `OccurrenceDrift` regaining its `occ_id`, so the ordering claim is data. Staleness is not fabricated for
`OwnedInstance`: its source never changes, so its drift is identically zero
and the method does not exist there.

`OwnedInstance::prepare` returns the same prepared type, bound to the owned
identity.

This preserves the current durable reuse product. It avoids forcing hosts to
replan inside every read callback.

### Epoch representation

View memoization uses the closed sum that actually exists, renamed and
extended. `ViewGeneration { Storage(GenerationId), Closed }` becomes:

```rust
enum ViewEpoch {
    Closed,
    Frozen,
    Store(GenerationId),
}
```

The rename does not thread a new name through the old shape — it unifies the
carriers. `ViewMemo` today holds the active binding as parallel same-length
vectors (`colts`, `generation: Vec<Option<ViewGeneration>>`, `filters`,
`parked`, `spare_buffers`) whose `None` means three unrelated things (never
executed, just parked, derived-occurrence-never-uses-generations) — while
`ParkedView` is the exact right record one field over. The cutover keeps
`colts` separate (the kernel takes `&mut [Colt]`) and collapses the rest to
one per-occurrence slot:

```rust
enum Binding {
    Unbound,
    Derived,
    Bound { epoch: ViewEpoch, filters: Vec<FilterPredicate>, last_used: u64 },
}
```

Three meanings become three arms; park/unpark becomes a move between
identical shapes; the `expect("a parked hit implies an executed active
binding")` deletes.

Identity is checked before the memo uses this value. `Frozen` therefore
cannot alias another owned instance. It is not a dummy generation zero.

Closed images remain theory-constant. Frozen ordinary images live in lazy
`OnceLock<Arc<RelationImage>>` slots — the closed-relation slot mechanism the
image cache already uses, applied to a new epoch. Store ordinary images remain
in the generation-aware `ImageCache`. The cache's own shape aligns with the
epoch sum: today closed relations live in a *second* `HashMap` beside the
generation map, re-dispatched by a schema-body match and a tautological
`expect`; the cutover makes it one `Box<[RelationSlot]>` indexed by
`RelationId`, whose arms mirror `ViewEpoch` — one three-way partition, not
two independent ones.

The stamp is the **only** validity token in the layer. `CommitSeq` — the
process-local second clock behind the parked reader — is retired: it advances
at exactly one site, inside the same branch that advances the image cache on
`GenerationId`, under the environment's one exclusive writer, so within a
process it counts the same event twice. Its recorded ruling justifies
non-comparability with `GenerationId`, not a second counter's existence. The
parked reader keys its validity on `(CatalogIdentity, GenerationId)` like
everything else.

### Execution boundary

The instance execution layer performs these operations:

```text
owner check             CatalogIdentity
parameter binding       CatalogRead::dict_lookup
key probe               M/U/F catalog reads
ordinary image binding  Frozen image slots or store ImageCache
closed image binding    schema synthesis
join                    Arc<RelationImage> only
interior and rec        prepared-query transient images only
finalization            CatalogRead::dict_resolve into Answers
```

The image-binding layer is generic over `FrozenSource` or `LmdbSource`. It is
today's `run_join` stratum, and it legitimately holds a catalog. The dense
join kernel below it is `Executor::execute`, and its purity is a **type
fact**, not a review convention: its signature is
`(plan, colts, bindings, sink, counters)` and names no catalog, transaction,
identity, dictionary, or store-kind type. The gate below checks the
signature, because the signature is the machine-checked constraint.

Key probes remain catalog point reads. They do not scan relation images.
Interior and recursive images remain execution-local. They never enter a
catalog or an owner image cache.

## Store birth, open, and persistence

### Format 8 admission provenance

The storage format increments from 7 to 8, and the format ledger in
`storage/env.rs` gains the v8 row: admission provenance, the store-birth
protocol, and the six-key `_meta` roster unchanged.

Format 8 means every ordinary writable handle began from one of these sources:

- A complete admission of the empty candidate.
- A raw copy from an admitted `OwnedInstance`.
- A raw compact copy from an admitted format-8 store.
- A successful incremental commit from an already admitted format-8 base.

The format version is the provenance boundary. Ordinary `Db::open` refuses
every earlier version. It may trust `holds` only after version, kind, database
roster, fingerprint, and descriptor checks succeed — the descriptor check is
new work at open; today open writes the descriptor and never reads it.

No old store is silently blessed by open. A full `verify_store` pass is useful
harness diagnostics, but it does not upgrade or admit an old store.

### Create

`Db::create(path, theory)` has this result:

```rust
Result<Admission<Db<S>>>
```

It first complete-admits the empty candidate without touching `path`. If empty
does not satisfy the theory, it returns `Rejected` and creates no visible
store. If the path already exists — including as an empty directory — it
returns `DestinationExists`.

A theory that needs initial facts uses:

```text
InstanceBuilder::new
    → load
    → admit
    → Db::from_instance
```

`Db::ephemeral` follows the same empty-admission rule for a new ephemeral
store. Its breaking signature is also `Result<Admission<Db<S>>>`; reopening an
existing admitted format-8 ephemeral store returns the accepted arm. An
existing empty directory is not an implicit create request.

### Store-birth error taxonomy

The path-must-not-exist rule is a behavior change, and the error enum changes
with it:

- **`Error::DestinationExists { path }` is added.** It fires from `create`,
  new `ephemeral`, `from_instance`, `ephemeral_from_instance`, and `compact`
  for any existing destination, including an empty directory. It replaces
  `Error::AlreadyInitialized` at every fresh-store constructor and replaces
  `compact`'s untyped `Io(AlreadyExists)` string — payloads carry paths, not
  formatted prose.
- **`Error::NotInitialized` is deleted.** It named the half-created store
  that `create` used to heal. Under staging plus atomic rename, a
  half-created store cannot exist at a destination path; the state is
  unrepresentable and the healing branch goes with it. Hosts that pre-create
  directories break, deliberately: an existing path is evidence of a
  previous claim on the name, and the engine refuses to guess.
- **`Error::PublishedButUnsynced { path, source }` is added** (below).
- `Error::TransactionPoisoned` and `Error::ForeignPreparedQuery` are
  unchanged. `Error::ForeignSnapshot` becomes `Error::ForeignWitness`.
  `Error::CommitRejected` **and** `Error::GenerationMoved` are deleted —
  both are proved outcomes, and both move into the outcome sums
  (`Admission::Rejected`, `ConditionalWrite::Moved`).

### `Db::from_instance`

```rust
pub fn from_instance(
    path: &Path,
    instance: &OwnedInstance<S>,
) -> Result<Db<S>>;
```

The method accepts only `OwnedInstance` in this proposal. Store-to-store copy
continues to use `compact` or explicit ETL.

`from_instance` opens one new LMDB write transaction and copies:

- Every encoded `_data` entry, including `F`, `M`, `U`, `R`, `S`, and `Q`.
- Every encoded `_dict` forward and reverse entry.

It preserves:

- Fact bytes.
- Row ids.
- Row-id high-waters.
- Fresh next values.
- Dictionary ids.
- Dictionary next-id.

It does not copy relation images. It does not reinsert facts. It does not mint
new row ids. It does not re-intern strings. It does not run judgment again.

The destination `_meta` block is synthesized fresh, and it is exactly the
existing six keys — no lifecycle key exists or is added:

- Format version 8.
- `StoreKind::Durable`.
- The instance schema fingerprint.
- The canonical schema descriptor.
- `GenerationId::initial()` with value zero.
- The copied dictionary next-id.

Separately from `_meta`: no dirty-marker sibling file exists at the
destination. The marker is deliberately a file, never a `_meta` key, because
it must be readable before any LMDB page is trusted; a fresh durable store
simply has none.

Source `_meta` is never copied because `FrozenCatalog` has none.

An ephemeral sibling, `Db::ephemeral_from_instance`, performs the same raw copy
but synthesizes `StoreKind::Ephemeral` and arms the marker-file lifecycle only
after the destination is complete.

### One store birth

Store birth has one implementation. `publish(source, kind, path)` folds the
`PublishStep` list below over a catalog source; the public constructors are
spellings of it:

| Constructor | Source | Kind |
|---|---|---|
| `Db::create` | the admitted empty candidate | Durable |
| `Db::ephemeral` (fresh path) | the admitted empty candidate | Ephemeral |
| `Db::from_instance` | an `OwnedInstance` | Durable |
| `Db::ephemeral_from_instance` | an `OwnedInstance` | Ephemeral |
| `Db::compact` | an open format-8 store's compacted copy | the source's kind |

`Db::create` is literally `InstanceBuilder::new(theory) → admit → publish` —
composition, not a sibling code path kept in agreement by review. Two
consequences fall out:

- `Db::create` never classifies a destination's meta block at all: the
  `DestinationExists` precondition runs before any LMDB environment opens,
  so `MetaBlock::HalfCreated` loses its create-path consumer along with the
  healing branch it fed.
- The current create-versus-compact divergence — one initializes in place,
  one direct-copies with its own fsync chain, each with its own
  existing-path behavior — stops being expressible. One publication, one
  crash story, one refusal.

### Atomic publication as data

Every new store path must not exist. This rule covers `create`, new
`ephemeral`, `from_instance`, `ephemeral_from_instance`, and `compact`
destinations — `compact` adopts this protocol wholesale, replacing its
current direct copy. The precondition makes directory publication one atomic
rename rather than an in-place initialization protocol.

The protocol is a value, not a comment. The constructor folds over a reified
step list:

```rust
enum PublishStep {
    CreateStaging,     // sibling dir: <name>.staging.<nonce>
    WriteCatalog,      // one LMDB write txn: data, dictionary, fresh meta
    CommitAndClose,    // commit, close the environment
    SyncStagingFiles,  // fsync files and the staging directory
    Rename,            // atomic rename to the destination
    SyncParent,        // fsync the destination parent
}
```

The owner-killed crashpoint macro harness is not resurrected. Reifying the
protocol replaces it with something better: the crash matrix is an iteration
over data. For every proper prefix of the step list, the test executes the
prefix, kills, and asserts the postcondition — the destination path does not
exist for every prefix ending before `Rename`, and a complete format-8 store
exists at the destination for every prefix ending at or after it. A new step
extends the matrix by construction; a forgotten crash case is a missing enum
arm, which the exhaustive fold refuses to compile.

`sync_dirent_chain` remains the directory-durability mechanism inside
`SyncStagingFiles` and `SyncParent`.

Staging hygiene:

- The staging directory is a sibling of the destination named
  `<name>.staging.<nonce>`.
- A failure before `Rename` leaves only that recognizable staging directory.
- Constructors never delete a path they did not create in the current call —
  the refusal-never-mutates law extends to staging orphans. A stale staging
  directory is documented as caller-removable by its name pattern.

`Rename` is the publication linearization point. A failure after rename
cannot honestly return an ordinary pre-publication error. The API returns
`Error::PublishedButUnsynced { path, source }`. The destination contains a
complete format-8 store, but the directory entry lacks a confirmed
machine-crash durability witness. The implementation does not rename it back
or delete it. A caller may open the visible destination or repair the
directory sync under explicit recovery policy.

### One parsed store metadata

The `_meta` block gains a value type. Today it is six loose key constants,
each hand-paired with a diagnostic string at every read site, **validated and
discarded** — `check_format_version -> Result<()>` and
`check_fingerprint -> Result<()>` throw away what they read — while the type
actually named `MetaBlock` holds a database *handle*, not the block's
contents. The open-precedence law ("ONE check precedence") is enforced by
convention across three hand-written sequences: ordinary open, the ephemeral
probe, and exhume.

Format 8 lands `parse_meta`, the parse-don't-validate form:

```rust
struct StoreMeta {
    version: FormatVersion,
    kind: StoreKind,
    fingerprint: SchemaFingerprint,
    generation: GenerationId,
    dict_next: InternId,
    descriptor: DescriptorBytes,
}
```

One `MetaKey` table owns each key's byte, diagnostic name, and codec.
`parse_meta` returns the whole block typed; the precedence law becomes the
parser's field order, and the three hand-written sequences collapse into one.
Downstream, the bare-bit boundary between "read the store" and "know the
store" closes:

- `probe_ephemeral_kind` stops verifying four facts and returning one `bool`
  the caller stores as `has_meta`; it returns
  `EphemeralTarget::{Fresh, Existing(StoreMeta)}`, and the flagged reopen
  re-assembles what the probe parsed instead of re-verifying it.
- `marker_shields_durable` stops reading the on-disk `StoreKind` and
  discarding it — the kind it read rides the verdict — and the
  `Err(NotInitialized)` it uses internally as a control-flow token deletes
  with the variant.
- The three-boolean ephemeral classifier (`crashed` / `has_data` /
  `has_meta`: eight combinations, four legal, one suppression guard) becomes
  one classification computed once:
  `Fresh | ExistingVerified(StoreMeta) | CrashVictim`.
- `EnvMode` stops being filled in two phases. Today `initialize` hardcodes
  `EnvMode::Durable` even for an ephemeral store and `arm_ephemeral` rewrites
  the field later behind an `unreachable!`; the marker path threads into
  assembly so the finished mode is constructed once and the two-phase window
  — where `Drop`'s crash contract reads the wrong mode — is unrepresentable.
- `ExhumedEnvironment`'s `(fingerprint, descriptor)` pair — a hash and its
  own preimage as two independently settable fields — becomes the
  hash-verified `SelfDescription` the parse mints.

### Format cutover

Format-7 durable and ephemeral stores fail every open surface with
`FormatMismatch`. This includes ordinary open, ephemeral reopen, exhume,
compact source open, and host bindings.

The release contains no format-7 decoder and no migration mode. A caller
rebuilds a format-8 store from source evidence. The engine never attempts to
infer the new admission provenance from old bytes.

This is an intentional compatibility break. It keeps the trusted open path and
the read-only path on one storage grammar.

`Exhumed` opens format 8 only. Its dynamic scan surface delegates to a
`ReadInstance<SchemaDescriptor>` inside its existing lexical callback. It does
not expose a second borrowed catalog type.

## Ephemeral stores

This proposal retains `Db::ephemeral`.

Ephemeral is an on-disk or ramdisk LMDB store with an explicit NOSYNC contract.
It supports repeated incremental writes, concurrent MVCC reads, clean reopen,
dirty-marker recovery, C embedding, and benchmark parity. Its crash coverage
rides the dirty-marker tests and the publication crash matrix above — the
old crashpoint harness is gone and stays gone. An immutable `OwnedInstance`
does not replace these semantics.

Ephemeral stays **one verb** while durable is two (`create` / `open`), and
the asymmetry is essential rather than accidental: dirty-marker recovery must
wipe and re-initialize under a single call. An `ephemeral_open` that
wiped-and-refused would mutate on refusal — forbidden by the
refusal-never-mutates law — and one that refused-without-wiping would strand
garbage that a separate `ephemeral_create` then refuses to clobber. The
branch on filesystem state *is* the crash-recovery problem, not a
representation artifact; splitting the verb would hide the branch inside a
worse protocol.

The three durations are therefore distinct:

| Representation | Mutable after admission | Backing | Machine-crash promise |
|---|---:|---|---|
| `OwnedInstance` | No | Heap | Process lifetime only |
| Durable `Db` | Yes | LMDB | Fsync durability |
| Ephemeral `Db` | Yes | LMDB NOSYNC | No survival promise |

`StoreKind` remains the persisted two-arm disk sum. Heap ownership does not
become a third `StoreKind` because a heap instance has no store metadata.

Deleting ephemeral would be a separate product proposal. It would have to
delete the Rust and C constructors, the kind metadata arm, dirty-marker
recovery, the differential oracle, and benchmark lanes together. TypeScript
deliberately has no ephemeral surface today and gains none here.

## TypeScript surface

### Exact types

`Violation<Rels>` already exists as the discriminated union of the five
violation bodies; the rejected arm carries an array of it. There is no
plural `Violations` type in TypeScript and none is added.

```ts
export type Admission<Rels extends SchemaRelations, T> =
	| { readonly tag: "accepted"; readonly value: T }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }

export interface Committed<T> {
	readonly value: T
	readonly generation: bigint
}

export interface Witness<Rels extends SchemaRelations> {
	readonly [witnessTypes]: Rels
}

type AbandonedArm<R> = R extends Abandon<infer P>
	? { readonly tag: "abandoned"; readonly abandoned: P }
	: never

export type WriteOutcome<Rels extends SchemaRelations, R> =
	| { readonly tag: "accepted"; readonly value: Committed<Exclude<R, Abandon<unknown>>> }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }
	| AbandonedArm<R>

export type WriteFromOutcome<Rels extends SchemaRelations, R> =
	| WriteOutcome<Rels, R>
	| { readonly tag: "moved"; readonly witnessed: bigint; readonly current: bigint }
```

One discriminant. Today's `WriteResult` narrows on `.ok` and then on property
presence; the abandoned arm is **retagged** from `{ ok: false; abandoned }`
to `{ tag: "abandoned"; abandoned }` so the whole outcome narrows on `tag`.
The R10 conditional-distribution property is preserved exactly: a callback
with no `Abandon` arm contributes `never` and the abandoned arm vanishes from
the sum. Double-keyed narrowing is a deleted state.

`Db.create` returns `Promise<Admission<Rels, Db<Rels>>>`. `Db.open` returns
`Promise<Db<Rels>>`. `Db.fromInstance` returns `Promise<Db<Rels>>` because its
input is already admitted.

The discriminant represents two proved outcomes. The design does not claim
that every tagged union is validation. The law is that the accepted arm
carries the admitted type and no other arm does.

Writes return `WriteOutcome<Rels, R>`. The SDK retains its explicit
`abandon(payload)` sentinel and the `abandonMark` unique-symbol probe. An
abandoned callback returns the separate abandoned arm because no candidate
reached judgment. A thrown callback remains an exception.

`Db.writeFrom` accepts `Witness<Rels>`. It has the same result as `Db.write`.
`witnessTypes` is a non-exported `unique symbol` on the `abandonMark`
precedent — a type-level brand; the runtime check is the native token. Native
code checks `CatalogIdentity` and generation before it invokes the callback.
A witness may cross an `await`; a `ReadInstance` may not.

```ts
type SyncResult<R> = R extends PromiseLike<unknown> ? never : R

interface Db<Rels extends SchemaRelations> {
	read<R>(body: (instance: ReadInstance<Rels>, witness: Witness<Rels>) => SyncResult<R>): SyncResult<R>
	write<R>(body: (tx: WriteTx<Rels>) => SyncResult<R>): WriteOutcome<Rels, SyncResult<R>>
	writeFrom<R>(
		witness: Witness<Rels>,
		body: (tx: WriteTx<Rels>) => SyncResult<R>
	): WriteFromOutcome<Rels, SyncResult<R>>
}
```

```ts
const builder = InstanceBuilder.create(schema)
builder.load(Account, accountColumns)

const admission = await builder.admit()
if (admission.tag === "rejected") {
	return report(admission.violations)
}

const instance = admission.value
const prepared = instance.prepare(query)
const rows = instance.execute(prepared, params)

await Db.fromInstance(path, instance)
```

`admit` consumes the native builder and runs as an **async native task** off
the JavaScript thread — this is what `InstanceBuilder: Send` is for; a
multi-gigabyte admission never blocks the event loop. Every later builder
call throws the typed spent-handle error. The example calls it exactly once
and narrows the union before using the instance.

Objects and columns are two transports for the same collection load. Both
enter one native parse-all-first batch path. Neither creates a second
mutation or judgment algebra.

### Error identity as data

The SDK's precedent is `ErrGenerationMoved` and `ErrNewtypeMismatch`:
exported error values a host compares by identity. That idiom generalizes;
message-string matching is a deleted state. The cutover exports:

- `ErrAsyncCallback` — a read or write callback returned a thenable.
- `ErrSpentHandle` — a consumed builder or disposed handle was used.
- `ErrUseAfterScope` — a stashed read instance or write tx was used after
  its callback returned.
- `ErrForeignPrepared` — a prepared query met a foreign instance.
- `ErrForeignWitness` — a witness met a foreign store.

`ErrGenerationMoved` is **deleted**: a moved generation is the
`{ tag: "moved" }` arm of `WriteFromOutcome`, not an exception. The retry
loop that today wraps `writeFrom` in try/catch becomes a match on the one
discriminant.

Error identity is *forced*, not curated. The C bridge already maps
`bumbledb::Error` exhaustively (`kind_of`, deliberately wildcard-free), so a
new engine variant fails compile there — while the napi bridge flattens every
error to `format!("bumbledb: {error}")`, so the same new variant silently
becomes an opaque string. The cutover gives the napi side the same forced
shape: an exhaustive `Error → kind` table in `wire_tags!`, carried across the
wire as `{ kind, message }`, entered into the `tags.json` golden beside the
existing twenty tables. The per-function outcome unions' `kind` literals
(`"schemaError"`, `"irError"`, `"generationMoved"`, …) and every new `tag`
discriminant this proposal introduces enter the same golden — the one place
where Rust↔TS drift is a compile failure instead of an audit finding.
`ExhumeOutcome::Refused { kind: &'static str, .. }` stops being stringly and
becomes three variants like its `OpenOutcome` sibling.

### Borrowed store reads

```ts
db.read((instance, witness) => {
	const rows = instance.execute(prepared, params)
	return consume(rows, witness)
})
```

`ReadInstance` is a synchronous callback capability. It cannot be acquired as
a handle. The following surfaces are deleted:

- `db.read()` without a callback.
- `using snap = db.read()`.
- `ReadScope`.
- Snapshot close and disposal.
- The snapshot worker and its request channel.

The callback type rejects Promise-like returns. The runtime also checks the
returned value with the existing thenable probe and throws `ErrAsyncCallback`
— on **both** verbs; today the probe guards only writes. TypeScript
conditional types alone are not a safety boundary.

The capability invalidates in `finally` before `db.read` returns. Every method
checks liveness. A stashed capability throws `ErrUseAfterScope`. The witness
is a distinct owning native token. Invalidating the read capability does not
invalidate a witness returned by the callback.

The N-API implementation invokes the JavaScript callback synchronously inside
the Rust `Db::read` closure on the JavaScript thread. It never transmutes a
`ReadInstance` lifetime to `'static`.

The existing snapshot worker is sound: it owns the real `Db::read` closure
across a request channel and keeps the transaction alive. It is deleted
because the new API no longer exposes an independently owned read handle, not
because the old worker fabricated an LMDB lifetime. The per-read worker
thread goes with it.

### Prepared ownership

The TypeScript SDK keeps its current private owner-token check. Native execution
also checks `CatalogIdentity`. The duplicated boundary checks are intentional:

- The SDK produces an immediate host-quality error for a foreign value.
- Native code remains safe against forged, old, or alternate bindings.

Prepared values own their `Arc<Schema>` and native catalog identity. No native
schema lifetime is erased; the `'static` transmute in the binding is deleted.

`InstanceBuilder`, `OwnedInstance`, and `Witness` implement `Symbol.dispose`.
Disposal is idempotent. Every later method throws `ErrSpentHandle`. A native
finalizer remains the backstop on all three.

The binding reports frozen catalog and native image capacity to V8 as external
memory. It adjusts the count when lazy images appear and when their last native
owner drops. This prevents the large Rust allocations from remaining invisible
to garbage collection pressure. Explicit disposal remains the deterministic
release path.

An undisposed `OwnedInstance` may be called across `await` because it owns its
catalog. Of the instance resources, a prepared query owns only schema and
identity. Its plan, scratch, and memoized images remain its own resources. It
does not keep a disposed instance's frozen catalog alive.

## C surface

C receives two owning heap handles, one witness handle, and one common
borrowed adapter:

```c
typedef struct bdb_instance_builder bdb_instance_builder;
typedef struct bdb_owned_instance bdb_owned_instance;
typedef struct bdb_instance_ref bdb_instance_ref;
typedef struct bdb_witness bdb_witness;
```

`bdb_instance_ref` is valid only during either callback:

```c
typedef uint32_t (*bdb_db_read_callback)(
    void *context,
    const bdb_instance_ref *instance,
    const bdb_witness *witness
);

typedef uint32_t (*bdb_owned_instance_read_callback)(
    void *context,
    const bdb_instance_ref *instance
);
```

Both callbacks receive the same query surface. Internally the adapter may use a
cold two-arm enum to dispatch to the two monomorphized Rust wrappers. That enum
does not enter catalog algorithms or the join kernel.

One C function does not accept two unrelated opaque owning pointer types.
Callers borrow an owned instance through `bdb_owned_instance_read` when they
need the common `bdb_instance_ref` surface.

The read callback's witness is borrowed for the callback duration. A caller
uses `bdb_witness_retain` to create an owning witness that may escape — sound
because the engine witness is now `Clone`; retention is a clone, not a
fabrication. Both a live borrowed witness and a retained witness may enter
`bdb_db_write_from`. `bdb_witness_destroy` releases only a retained witness.

Initial admission returns an exact tagged union:

```c
typedef enum bdb_admission_tag {
    BDB_ADMISSION_EMPTY = 0,
    BDB_ADMISSION_ACCEPTED = 1,
    BDB_ADMISSION_REJECTED = 2,
    BDB_ADMISSION_MOVED = 3
} bdb_admission_tag;

typedef struct bdb_instance_admission {
    bdb_admission_tag tag;
    union {
        bdb_owned_instance *accepted;
        bdb_violations *rejected;
    } value;
} bdb_instance_admission;

typedef struct bdb_db_admission {
    bdb_admission_tag tag;
    union {
        bdb_db *accepted;
        bdb_violations *rejected;
    } value;
} bdb_db_admission;

typedef struct bdb_write_admission {
    bdb_admission_tag tag;
    union {
        uint64_t accepted_generation;
        bdb_violations *rejected;
        struct {
            uint64_t witnessed;
            uint64_t current;
        } moved;
    } value;
} bdb_write_admission;
```

The admit call takes `bdb_instance_builder **`. It nulls the caller's pointer
after consuming the builder on every outcome.

`bdb_db_create` and `bdb_db_ephemeral` keep the ABI's one convention — every
fallible export returns `bdb_status` and writes through out-parameters — and
fill a `bdb_db_admission` out-parameter on every OK result. Reopening an
existing format-8 ephemeral store fills the accepted arm. `bdb_db_open` also
keeps the convention: it returns `bdb_status` and fills `bdb_db **out`
directly, with **no** admission union, because format-8 open carries
admission provenance and has nothing to adjudicate.

`bdb_db_write` and `bdb_db_write_from` fill `bdb_write_admission`. A theory
rejection returns `BDB_STATUS_OK` with the rejected arm — rejection is a
proved outcome, not an infrastructure failure, and the status protocol law in
the crate preamble is rewritten to say so. An infrastructure failure returns
a non-OK status and `bdb_error`. A callback abort returns
`BDB_STATUS_ABORTED` and no admission value. C callbacks have no return
payload, so the accepted arm carries only the committed generation.

Every tagged output starts in the documented `BDB_ADMISSION_EMPTY` state. C
cannot make an uninitialized out-struct unrepresentable; tag zero makes it
detectable, which is the C-expressible approximation. Each accepted handle
and each rejected `bdb_violations` value has one matching destroy function —
`bdb_violations` becomes an owning handle, no longer a borrowed rendering of
`bdb_error`. No union arm is inspected unless the outer C status is
`BDB_STATUS_OK`. The empty tag is never returned with `BDB_STATUS_OK`.

The `bdb_error_kind` deltas are enumerated, and the deliberately
wildcard-free kind mapping forces every site:

- `BDB_ERROR_KIND_COMMIT_REJECTED` is deleted with `Error::CommitRejected`.
- The `GenerationMoved` kind is deleted with `Error::GenerationMoved`, and
  with it `bdb_error`'s `generation_moved` side-car payload — the two
  generations now live in `bdb_write_admission`'s `moved` arm, where only
  `bdb_db_write_from` can mint them.
- `BDB_ERROR_KIND_FOREIGN_SNAPSHOT` becomes `BDB_ERROR_KIND_FOREIGN_WITNESS`.
- `BDB_ERROR_KIND_FOREIGN_PREPARED` is unchanged.
- Kinds for `DestinationExists` and `PublishedButUnsynced` are added;
  the kind for `NotInitialized` is deleted.
- Bridge-minted kinds stop impersonating engine kinds: the bridge's own
  refusals (`fail_locked`'s four busy-handle sites, `fail_shape`'s eighteen
  marshal sites) today mint `ENVIRONMENT_LOCKED` and `FACT_SHAPE` — the same
  kinds the engine produces — with only message prose as the discriminant.
  Errors gain an origin (`BDB_ERROR_ORIGIN_ENGINE` / `BDB_ERROR_ORIGIN_BRIDGE`),
  and the busy-handle refusals become typed bridge kinds rather than
  borrowed engine vocabulary.

C cannot express a lexical lifetime, so the bridge carries handle state — but
it carries it **once**, not eight times. Today `bdb_db` spends eight
independent atomics (`in_write`, `in_read`, `snapshot_slot.{snap, alive}`,
`tx_slot.{tx, db, alive, in_op}`) on a machine with roughly five legal
states; `alive` duplicates `ptr != null`, and the illegal pairs are
re-checked at four sites. The cutover replaces the flags with one phase word
per handle — `Idle | Reading | Writing | WritingBusy` — with CAS transitions.
A null pointer *is* the dead state; "busy" is a state, not a second flag.

Ref slots are **refcounted**. Today the alive bit lives inside the very
allocation whose liveness is in question: a ref stashed past
`bdb_db_destroy` dereferences freed memory to read its own alive flag — the
one genuine use-after-free window on the boundary — and a retained witness
would widen it. Retained refs and witnesses hold a refcounted slot that
outlives the handle, so "used after its owner died" is a checkable
`BDB_STATUS_MISUSE`, never UB.

Refs are minted **per callback**, not parked in one inline slot. The current
single-slot representation forces the bridge to refuse a second concurrent
read and to report its own refusal as `ENVIRONMENT_LOCKED` — a representation
limit dressed as an engine capability limit. Per-callback refs restore the
engine's real MVCC read concurrency through C, and the four bridge-minted
`ENVIRONMENT_LOCKED` refusals delete (they become typed bridge kinds under
the error-origin split above where a refusal genuinely remains).

Every handle pair is owner-checked at the bridge. `bdb_tx_ref` already
carries its owner pointer; snapshot refs, prepared handles, and witnesses
gain the same token — the `CatalogIdentity` address — so
`write_from(db_a, witness_b)` and `execute(instance_a, prepared_b)` are
refused before entering the engine, which checks again. This mirrors the
TypeScript dual-check rationale.

The callback exit becomes a sum on both bridges. Today the C bridge smuggles
an `Error::Io("bumbledb-c callback abort")` sentinel past the engine and
reconciles it with two side booleans (`aborted`, `misuse`) — eight states for
four outcomes — and an engine error raised after an abort is swallowed into a
clean `BDB_STATUS_ABORTED`. The closure returns `Exit::{Proceed, Abort,
Misuse}` threaded as data; the sentinel string and both flags delete, and a
real engine failure during abort teardown reports as what it is. Each host
language keeps its native decline spelling — Rust propagates the caller's own
error with `?`, TypeScript returns the `abandon` sentinel, C returns a
control code — one semantic, three surface spellings, one internal
representation.

Three smaller shapes ride the same ABI bump. `bdb_fresh_range` gains the tag
every other payload already has — today `{0, 0}` is in-band empty where zero
is also the first legal minted id, the one sentinel left on the surface.
`bdb_violation` stops flattening the three-arm rendered violation into
`kind + direction-with-a-None-arm + has_measure + two words` — representable
illegal cells like `(Capacity, has_measure: false)` — and carries per-kind
payload arms under its existing `kind`; `bdb_error`'s optional side-car
payloads fold into the kind the same way. The boundary keeps one boolean
spelling instead of today's two (`uint8_t` inbound, C `bool` outbound). The
bridge's `guard` narrows so a body cannot return `BDB_STATUS_ERROR` without
an error written, and entries with no error out-param get a statusless guard
instead of passing a null pointer in-band. On the napi side, the surviving
`TxReq`/`TxReply` worker protocol stops pairing request to reply by
convention — the reply channel rides inside the request, so a protocol
mismatch is unrepresentable rather than misreported as a dead worker.

`bdb_abi_version()` returns 3. The snapshot-named functions are removed
rather than aliased. The cbindgen export list gains the admission tag, the
three admission unions, the witness handle, the error origin, and the tagged
fresh range, and `bumbledb_c.h` is regenerated.

## Lean correspondence

Lean already separates raw instances from admitted states:

```lean
def Instance : Type := RelId → Set Fact

structure State (T : Theory) where
  inst : Instance
  models : holds T inst
```

`InstanceBuilder` corresponds to a finite raw `Instance` under construction.
`OwnedInstance` and `ReadInstance` correspond to `State T` with different
physical durations.

`Txn.judge T I` already defines complete initial admission over a raw
instance. `Txn.commit s d` defines admission over a proved state and a delta.
`Txn.keyViolationSet` and `Txn.statementViolationSet` are the `Set`-valued
phase citations, and `Decide.lean`'s executable twins bridge to them
membership-for-membership (`mem_keyViolationsB`, `mem_statementViolationsB`).

`Txn/DeltaRestriction.lean` proves why the incremental roster is equivalent to
the whole judgment only under the pre-state premise. The theorem
`Bumbledb.Countermodels.incremental_verdict_needs_holds` proves that removing
the premise is unsound.

`Admission.lean`'s `touched_delta_bounded` obligation forces every touched
key to be delta-bounded. That is a **structural refusal** of
answer-dependent statement forms: a general exact-cover form cannot enter
admission without reworking that contract, so the Primer boundary below is a
theorem-shaped consequence, not a scoping preference.

Implementation work adds these named artifacts, in lockstep with the Rust
steps that consume them:

- **L1 — complete-roster definitions.** `Set`-valued
  `Txn.completeKeyViolations` and `Txn.completeStatementViolations` over a
  raw `Instance`, mirroring the engine's roster derivation. Ships with Rust
  step 6.
- **L2 — phase agreement.** Set equalities against the existing citations:

  ```lean
  completeKeyViolations T I = keyViolationSet T I
  completeStatementViolations T I = statementViolationSet T I
  ```

  Executable twins ride `Decide.lean`'s membership idiom, matching the
  `mem_keyViolationsB` shape over `RowInstance` and `W.den`. Ships with Rust
  step 6.
- **L3 — the roster bridge.** `completeRosterPasses T I ↔ holds T I`. Ships
  with Rust step 8.
- **L4 — the obligation partition theorem.** The "two witnesses" claim gets
  a Lean home: schema validation's closed-constant refutations plus the
  instance-dependent complete roster compose to `holds`. Without this
  theorem, L3 cannot be proved for a roster that skips validation-discharged
  obligations. Ships with Rust step 6.
- **L5 — the complete-admission conformance lane.** `judgeB` stays the
  differential oracle. The incremental lane's recorded scope fences exclude
  fixture classes — closed-source containments among them — precisely
  because the engine verdict is delta-restricted and a whole-state oracle
  would mismatch a correct engine. The complete verdict is not
  delta-restricted, so those fences **lift** for the new lane: generated
  worlds including closed-source containment fixtures run through complete
  admission against `judgeB`. One of this proposal's four motivating shapes
  lives in exactly that formerly fenced class. Ships with Rust step 8 and
  gates the merge.

The executable `judgeB` remains the differential semantic oracle. The packed
catalog is a representation refinement, not a new denotation.

## Primer boundary

Bumbledb currently proves its declared functionality, pointwise
functionality, containment, coverage, and capacity statements. Mirrored
containments remain two containment statements with a sealed pairing.

Primer's normalization ledger needs stronger application equations. The
current schema in sibling path
`primer-spec/src/theory/ledger/normalization.ts` does not encode all of these:

- The disposition arms totally cover every `InputFact`.
- The disposition arms are pairwise disjoint.
- Every `DirectImageFact` has a producing `OutputFactInput` witness — the
  schema encodes only the reverse containment.
- The witness-input projection equals the required core and overlay source
  roster.

Therefore Bumbledb admission alone does not certify Primer's normalization
exact covers — and by `touched_delta_bounded`, it structurally cannot without
a reworked admission contract.

The correct application pipeline is:

```text
InstanceBuilder
    → Bumbledb admission
    → OwnedInstance<NormalizationLedger>
    → exact-cover defect queries
    → application Admission<CertifiedNormalization>
```

`CertifiedNormalization` owns the admitted instance and an application
certificate tied to its `CatalogIdentity`. The application checker returns its
own exact sum of accepted certificate or structured exact-cover violations.
It never reads absence as success; each defect query participates in an
explicit equality or disjointness judgment.

If exact cover becomes an upstream dependency form, it needs a separate
proposal that extends the schema vocabulary, Lean `holds`, the executable
judge, `Admission.lean`'s touched-key contract, the incremental restriction,
the complete roster, the violation type, and the ordered-oracle cost proof
together. This instance-lifetime proposal does not smuggle that feature into
`admit`.

## Representation debt absorbed by this cutover

A final audit swept every Rust data structure in the tree for
representation-principle violations — flags encoding state machines, proofs
validated and discarded, discriminants duplicated, sentinels in-band,
invariants carried by comments. Findings **coupled to this cutover** — the
steps below rewrite the same structures, or the format-8 / ABI-3 window is
the only cheap moment to change persisted or exported shapes — are absorbed
here. Two genuinely independent passes are split into sibling proposals and
do not gate this release:

- [`proposals/exec-representation.md`](exec-representation.md) — one
  predicate algebra over operand providers, plus the executor/plan/sink/colt
  interior sums. Crate-private, no format or ABI coupling.
- [`proposals/interior-debt.md`](interior-debt.md) — small
  cutover-independent interior cleanups (bare-bit probe returns, missing
  newtypes, parallel-array zips, string-keyed observability events).

### One statement spine

The schema has one typed spine — `StatementRef` / `StatementView` over three
homogeneous arenas, the right shape, correctly built — and one untyped shadow,
`StatementId`, that trails it through the whole engine. Because the arm tag
lives in `Schema::order` and not in the id, every consumer that already knows
the arm must resolve and refutably re-narrow: three `unreachable!`s in
rejection decoration, the corruption arms in the coverage walk, a **silent
fall-through in the fresh-row probe path** on a resolve failure, and a
runtime re-proof in `judgment.rs` of the arena↔spine pairing that validation
minted. The cutover makes the arm-tagged reference the one stored statement
identity:

- `Violation.statement` becomes `StatementRef`; `EdgeOp` drops its second id;
  `ContainmentStatement.mirror: Option<StatementId>` becomes
  `pairing: Pairing::{OneWay, Mirror(ContainmentId)}` — typed to the arena,
  no re-resolution through `StatementView`.
- The persisted `R`/`U` key statement slots become typed. These are stored
  bytes: **format 8 is the only window**, which is why this lands here and
  not in a sibling pass. The roster then walks reverse edges without
  inheriting the narrowings it was designed to delete.
- `StatementId` demotes to what its own doc says it is — the
  materialized-order ordinal for fingerprints, rendering, and host citation.
- Three roster-adjacent shapes seal at validation with it: the per-key
  maintenance plan (`FreshRow | Scalar | Pointwise { tail, proof }`), so the
  roster's key phase matches data instead of asking
  `as_fresh_row().is_some()` an eleventh time; the closed-side check split
  (`EncodableCheck`), so the sealed-row walk cannot answer a
  validator-refuted case with a silent `false`; and the coverage arm carries
  its `target_tail`, deleting two `key_tail().expect()`s. The
  `key_permutation` index gives way to the sealed permuted projection it was
  an index into.

Lean already has this shape: the model's `Statement` is one inductive sum,
and the shadow spine was engine-only. Deleting it aligns representation with
denotation; no new Lean obligation arises.

### One verdict shape

`Check` and `Admission` fixed the verdict's *channel*. The audit showed the
verdict's *shape* is spelled four more times, and one claimed invariant is
false at the boundary:

- **`Violations` privatizes its representation.** Its doc claims empty,
  unsorted, or duplicated sets are "unrepresentable" — but the enum's
  variants are public and `#[non_exhaustive]` appears nowhere, so
  `Violations::Citations(Box::new([]))` compiles in host code today. While
  it was `Error::CommitRejected`'s private cargo that was a style hole;
  as `Admission::Rejected`'s public payload it is a soundness hole. Private
  fields, sealed constructors only. `Decorated`'s two parallel boxes (equal
  length enforced by `assert_eq!` under a `# Panics` doc) become one
  `Box<[(Violation, Box<[CitedFact]>)]>`.
- **`Violation` gets one body**: `{ statement: StatementRef, fact, detail }`.
  The per-arm duplicated fields and the three match-to-project accessors
  fold; `FunctionalityViolation` becomes
  `{ statement, fact, conflict: Conflict::{Scalar, Pointwise(incumbent)} }`.
- **`CitedFact` carries its invariants.** Its field-count and per-field type
  laws are comment-only today, and the renderer trusts them and panics on
  breach; a `pub(crate)` constructor taking the relation layout carries the
  proof instead.
- **`StoreFinding` embeds instead of transcribing.** Five variant pairs are
  field-for-field identical with `Violation`/`CorruptionError` arms (one of
  them one fact in *three* shapes); they become
  `StoreFinding::Judgment(Violation)` and
  `StoreFinding::Corruption(CorruptionError)`, deleting the sweeper's ~40
  lines of transcription. The finding's `direction` field — hardcodable to
  exactly one value, documented as such — deletes; the sweep's citation
  orientation is a property of the sweep.
- **`StoreReport` puts its verdict in the type**:
  `StoreVerdict::{Coherent { dangling_intern_ids }, Desynced { findings, .. }}`
  — today the verdict is `findings.is_empty()` and an explicitly
  not-a-finding statistic shares the struct with the findings.
- **One roster, three projections.** The violation roster exists today in
  four unreconciled shapes: the Rust sum, the sweeper's two-arm subset, a
  renderer that flattens the sum back into `Option` fields, and a five-arm
  TS union whose mirror-orientation axis exists in no Rust type (it is
  re-inferred inside the renderer). The mirror orientation becomes a
  `Violation` field; the wire, C, and TS forms become mechanical projections
  of the one Rust shape, pinned by the `tags.json` golden.
- **The `Option` verdicts around the checker collapse.** `judge`'s
  `Result<Option<Violations>>`, `Violations::seal`'s `Option` return, and
  `apply`'s duplicate seal-and-throw all become the one `Admission`-shaped
  judgment, with `Check` as its per-probe element and `collect` reduced to
  `extend`.

### The commit-path ledger

Steps 5 and 6 rewrite the commit path; these land with them, so partiality
and duplication do not survive into the rewritten judgment:

| Today | Becomes |
|---|---|
| `FactOp` re-tags the disposition its container already names — two applier `unreachable!`s, one **silently skipped judgment** in `check_source`, six enum-straddling accessors, and a `capacity_edges() -> &[]` sentinel | `DeleteOp` / `InsertOp` over one `FactCore`; per-arm fields; total appliers |
| `EdgeOp` / `MarkEdgeOp` / `CapacityKeyOp` — three shapes for one key-symmetric `R` write, two byte-identical delete loops | one `RKeyOp` per phase: insert carries `MarkWeight` (containments take `Unit`), delete is key-only |
| the coverage walk's `affected: BTreeSet<(ContainmentId, &[u8])>` stores raw bytes and re-parses, re-resolves, and re-checks what its own scan loop just proved — four unreachable-by-construction corruption arms | a parsed `AffectedSource` element minted where the data enters; the walk has nothing left to re-validate |
| `DependentCheck.psi_qualified: bool` — one bool for two documented falsehoods, whose matched payload the plan drops and the judgment re-fetches | `Owed::{Unconditional, IfEstablisherFails(&SelectionCheck)}` |
| `Probe` flattens four statement-determined fields; its construction match is copy-pasted at three sites while the existing `Enforcement::target_key()` helper goes uncalled | one `Probe` minted from the sealed statement, probe kind resolved at validation |
| `skip_puts: bool` beside `row_id` — a three-state landing smeared over a flag, three interleaved guards | `Landing::{Free(u64), Refused(u64)}`; the put helpers take `Free` by type |
| `MembershipOp.axiom: Option<AxiomIndex>` — a half-decided closed-target verdict phase 3 finishes with zero reads, behind a silent-skip narrowing | the plan carries the finished `Check` |
| `check_target` classifies survivors by an if/else chain asking relation-body questions per check | `Survivors` sealed on the statement at validation; one match over named arms |
| `plan_commit(delta, schema, selections)` — three arguments that must agree, tied by nothing | `Selections` carries the schema it was encoded from; the rest derive |
| `Applied<'env>` exposes a committable transaction with no evidence phase 3 ran — the five-phase order lives in module prose | the typestate chain the incremental pipeline diagram already draws: `Applied::judge → Judged::finish` |
| `FactScratch` — one five-`Vec` bag shared across dispositions, "empty between facts" enforced by matching drains; a stray push leaks into the **next op** silently | per-disposition scratch; the leak is unrepresentable |
| `DeterminantOp::tail()` re-projects its own discriminant into `Option`, and the applier reconstructs the violation variant from `Option<incumbent>` control flow | one match per arm, each minting its violation directly |
| `WriteDelta.pending_interns` beside `dict_next: Option<u64>` — one truth in two fields, flushed as two independent facts | `interns: Option<PendingInterns { next_id, entries }>` — the counter cannot advance without its entries |
| the C17 slot law re-dispatched per walked entry through a hoisted `bool` mirroring `SealedWeight` | `SlotShape::{Empty, Word}` resolved once per statement |

### The prepared-pipeline ledger

Steps 3 and 9 rebuild preparation and execution; these land with them:

| Today | Becomes |
|---|---|
| the direct point-probe lane spread across a pipeline arm test, `interiors.is_empty()`, `rules.len() == 1`, and an `Option` — re-detected at five sites under a comment claiming it is "parsed at build … not re-detected" | a third `PreparedPipeline::PointProbe` arm sealed by build; the `Option`, two predicates, one `unreachable!`, one `debug_assert!` delete |
| latch state as `unresolved_literals: u32` (decremented with `saturating_sub` — arithmetic that distrusts itself) beside per-rule `ResolutionState`, with the predicate spelled at four sites and introspection re-deriving the set by walking plans | one `Latch::{Pending(NonZeroU32), Latched}` returned by the resolver as its proof |
| `Answers.arity` written at three sites and never by `clear()`; `len()` guards with `unwrap_or(0)`; `is_empty()` disagrees with `len() == 0` in the not-yet-executed state | `begin(arity)` stamps and clears together; the shape has one writer |
| `MutationReport { pub submitted, pub changed }` — `changed <= submitted` is a `debug_assert!` and hosts can construct the illegal state this proposal then reuses on new builder surface | private fields and accessors, sealed before reuse |
| `Query` / `ValidatedQuery` / `InteriorSignatures` duplicate three-to-five arm-independent fields per arm behind fifteen match-to-project accessors, and `ValidatedQuery` stores two derived fields whose justifying citations (`engine-003`, `engine-028`) **exist nowhere in the repository** | shared fields hoisted into the struct, the sum reduced to what varies; the derived fields become methods and the phantom citations delete |
| `Query::head` re-states the fold discriminant already in `rules[*].finds`, policed by two mismatch error variants | the head is derived; both error variants delete |
| `AggOp` — public, structurally identical to `HeadOp` and `AggKind`, with zero engine consumers — plus `fold_seen`/`pack_seen` boolean pairs re-testing exclusivity validation proved | `AggOp` deletes; the companion regime is `Companions::{None, Folds, OnePack}` |
| `RecPingPong` spells one parity as a `bool`, four named fields, and an array index | two `[T; 2]` arrays indexed by one phase |
| `run_rules` / `run_rules_cq_profile` — one loop protocol written twice, differing only in per-rule counter choice | counter choice as data; one loop |
| `fact_by_key` matches the same `PointRead` twice, with an `unreachable!` closed arm on the second pass | a nested sum makes the second match total |
| `ReadScratch` is a named struct on the read side and two loose fields on the write side, against R15's recorded symmetry | one `ReadScratch` on both sides |

### Identity residue

`CatalogIdentity` does not sit beside the old identity — it deletes it. The
process-local `NEXT_INSTANCE: AtomicU64` counter, its documented-but-
unenforced zero sentinel ("0 stays 'no environment' forever", with no
consumer anywhere), and the bare `u64` it threads through `Witness` and
`PreparedQuery` all go. `CommitSeq` retires with them (see the epoch
section). After this cutover there is exactly one identity representation
and one epoch representation in the engine.

## Implementation sequence

No intermediate step is released as a public compatibility layer. Steps that
change a judgment name their Lean twin and land with it.

1. **Result representation.** Add `Admission<T>` and `Check`. Rewire every
   error-as-verdict site — checkers, `collect`, and the sweeper's finding
   path. Remove `Error::CommitRejected` from the public enum and from
   internal control flow. Record the decoration and direction policies.
   Land the one verdict shape: `Violations` privatized and re-paired,
   `Violation`'s single body over `StatementRef`, `CitedFact`'s sealed
   constructor, `StoreFinding` embedding, `StoreVerdict`, and the
   `Option`-verdict collapse.
2. **Error taxonomy.** Add `DestinationExists` and `PublishedButUnsynced`.
   Delete `NotInitialized`. Delete `GenerationMoved` (it returns as
   `ConditionalWrite::Moved` at step 12). Rename `ForeignSnapshot` to
   `ForeignWitness`. Reify the write exit as `WriteEnd`. Queue the C kind
   deltas and the origin split for step 14.
3. **Prepared ownership.** Move schema ownership to `Arc<Schema>`. Replace the
   raw environment integer with `CatalogIdentity` and delete `NEXT_INSTANCE`.
   Delete the N-API schema lifetime transmute. Collapse
   `execute`/`execute_args`. Absorb the prepared-pipeline ledger's build-time
   shapes: the `PointProbe` arm, the `Latch` verdict, `Answers::begin`, the
   sealed `MutationReport`, the `RuleStats` sum ahead of `profile`'s
   promotion, the `Staleness` sum, and the `Query`/`ValidatedQuery` field
   hoisting.
4. **Ordered algebra.** Land `OrderedRead`, `OrderedWrite`, lending cursors,
   and the `SortedGets` trait — the existing T8 struct becomes its LMDB
   implementation. Implement over LMDB first.
5. **Catalog algebra.** Retarget fact reads, dictionary reads, image build,
   key probes, judgment, applier, and sweeper to generic catalog capabilities.
   Replace the five nominal `Fact` codec methods and the `Key` determinant
   pair with `CodecRead`/`CodecWrite` emissions at all four macro sites. Land
   the codec value vocabulary: `InternId`, the `ValueType`/`TypeDesc` merge,
   `ValueRef`'s `Fixed*` deletion, typed per-field decode, `Box<str>`. Keep
   dense execution on images.
6. **Obligation split.** Extract `IncrementalObligations` from the commit
   plan. Derive `CompleteObligations` by exhaustive `StatementView` match,
   reusing the sweeper's statement-phase mechanisms. Feed both into one
   checker. Land the statement spine (`StatementRef` as stored identity,
   `Pairing`), the validation-sealed per-key plans, `EncodableCheck`,
   `Survivors`, and the commit-path ledger; the persisted `R`/`U` slot
   retyping rides step 10's format bump. **Lean lockstep: L1, L2, L4.**
7. **Heap stage.** Add `MutationCore<HeapMutation, S>`, compact staging
   arenas, and `InstanceBuilder`.
8. **Packed freeze.** Add bounded sorted runs, merge-time key phase,
   `CandidateCatalog`, `FrozenMap`, and complete statement admission.
   **Lean lockstep: L3, L5 — the complete conformance lane gates this step's
   merge.**
9. **Owned execution.** Add `OwnedInstance`, frozen image slots, the
   `ViewEpoch` rename-and-extend, generic prepare, bind, key probe, image
   bind, join, and finalize. Collapse `ViewMemo` onto the per-occurrence
   `Binding` slot, land `RelationSlot`, and retire `CommitSeq`. Move
   `staleness` onto `&ReadInstance`.
10. **Format 8.** Make create complete-admit empty. Refuse earlier formats on
    every open surface. Add the open-time descriptor check. Extend the format
    ledger. Delete legacy format decoding.
11. **Persistence.** Add raw data/dictionary export, fresh `_meta` synthesis,
    `parse_meta`/`StoreMeta` with the `MetaKey` table, the ephemeral
    classifier sum and threaded `EnvMode`, the reified `PublishStep`
    protocol, the prefix crash matrix, staging hygiene, and the one
    `publish` implementation behind all five constructors — `compact`
    included.
12. **Rust API cutover.** Rename the borrowed query surface to `ReadInstance`.
    Remove snapshot names. Change create and write to nested admission sums
    and `write_from` to `ConditionalWrite`. Collapse conditional writes onto
    the cloneable borrowed `Witness<S>`, recording the overturned spent-move
    ruling and deleting its stale `#[expect]`.
13. **TypeScript cutover.** Add builder and owned handles with async native
    admission. Replace `ReadScope`, the handle read, and the snapshot worker
    with the synchronous callback. Extend the thenable probe to reads. Retag
    the abandoned arm and add the `moved` arm. Export the error values and
    land the forced napi kind table plus the outcome-kind rows in the
    `tags.json` golden. Type the surviving `TxReq`/`TxReply` pairing. Add
    witness ownership, disposal, and V8 external-memory accounting.
14. **C ABI cutover.** Add builder, owned handle, common borrowed instance
    ref, witness handle, and tagged admissions with the `moved` arm. Land the
    one handle-phase word, refcounted ref slots, per-callback refs, owner
    tokens on every handle pair, the `Exit` callback sum, the error-origin
    split, per-kind violation arms, the tagged fresh range, and the narrowed
    guards. Apply the kind-enum deltas. Bump `bdb_abi_version` to 3 and
    regenerate `bumbledb_c.h`.
15. **Documentation cutover.** Update architecture, API, storage, validation,
    benchmark, publishing, and cookbook documents in the same release,
    including the format ledger and the conformance fence table.

Steps 4 through 9 may live behind crate-private code during development. There
is no public half-state in which a format-7 store is treated as admitted or a
mutable candidate can execute a query.

## Acceptance gates

### Semantic gates

- Complete admission agrees with Lean `judgeB` over generated finite worlds,
  **including the fixture classes the incremental lane fences out for
  delta-restriction reasons** — closed-source containments among them.
- The key phase returns the complete key-violation set and preempts statements.
- The statement phase returns the complete containment and capacity violation
  set, under both containment enforcement arms.
- An empty delta on an unproved candidate does not take the incremental
  shortcut.
- Closed source to missing ordinary target rejects.
- Closed positive-floor parent with zero children rejects.
- Ordinary positive-floor parent with zero children rejects.
- A fresh-row collision rejects as a scalar functionality violation of the
  materialized fresh-key statement — no separate violation kind exists.
- Every accepted public instance satisfies the declared Bumbledb theory.

### Lean gates

- L1 through L5 exist under their stated names and build.
- The phase-agreement equalities hold as `Set` equalities; the executable
  twins hold membership-for-membership.
- The partition theorem composes the validation witness and the complete
  roster into `holds`.
- The complete-admission conformance lane is green and required by CI.

### Catalog gates

- LMDB and frozen catalogs agree on fact scans, point reads, neighbors, ranges,
  sorted gets, counts, counters, and dictionary operations.
- Initial heap construction and the internal LMDB reference harness produce
  byte-identical `F`, `M`, `U`, `R`, `S`, `Q`, and dictionary entries for the
  same canonical input.
- Mutable cursor deletion and `put_no_overwrite` have identical verdicts.
- No GAT-bearing catalog trait is converted to `dyn`.
- Compiler probes for the pinned nightly cover GAT returns, lending range
  cursors, mutable deletion cursors, and the absence of broad HRTBs.

### Provenance gates

- Rust execution rejects a prepared query from another `OwnedInstance` even
  when both values have the same lexical borrow lifetime.
- Rust execution rejects a prepared query from another `Db` with the same
  schema.
- Rust conditional write rejects a witness from another `Db` before generation
  comparison or callback execution.
- One witness justifies two sequential conditional writes when the generation
  has not moved — reuse is the contract, not an accident.
- A retained TypeScript or C witness remains usable after its read callback.
- TypeScript and C reject foreign prepared handles before native execution.
- `Executor::execute`'s signature names no catalog, transaction, identity,
  dictionary, or store-kind type. The gate checks the signature; the image
  binding layer above it is the only stratum that touches a catalog.

### Representation gates

- `Violations` cannot be constructed unsealed from host code — pinned by a
  compile-fail test.
- The deleted narrowings are gone: no applier disposition refutation, no
  coverage-walk corruption arm for scan-minted keys, no fresh-row probe
  fall-through, no rejection-decoration `unreachable!`, no macro-emitted
  `unreachable!("schema-typed")`.
- One statement identity: nothing outside fingerprinting, rendering, and
  host citation stores a bare `StatementId`.
- One process clock: `CommitSeq` is absent; the parked reader keys on
  `(CatalogIdentity, GenerationId)`.
- `KeyProbeStats.hit` has exactly one derivation site.
- The burn-flush write exit has exactly one implementation.
- The `tags.json` golden covers every outcome kind and every `tag`
  discriminant this proposal introduces; the Rust, wire, TS, and C violation
  rosters are pinned equal by one conformance test.
- A C ref stashed past `bdb_db_destroy` answers `BDB_STATUS_MISUSE` — pinned
  under the sanitizer lane, never use-after-free.
- Two concurrent C read callbacks on one `bdb_db` both succeed.
- `bdb_fresh_range`'s empty arm is a tag; `{0, 0}` is unspellable as empty.

### Persistence gates

- Raw persistence preserves every source `_data` and `_dict` byte.
- Raw persistence preserves row ids, `S`, `Q`, dictionary ids, and dictionary
  next-id.
- Destination `_meta` is fresh and contains exactly the six keys: format 8,
  the selected disk kind, schema fingerprint, canonical descriptor, initial
  generation, and copied dictionary next-id. No lifecycle key exists.
- Source metadata and images never copy.
- Every fresh-store constructor rejects an existing destination, including an
  empty directory, with `DestinationExists` — `compact` included.
- The crash matrix executes every proper prefix of the `PublishStep` list:
  no prefix ending before `Rename` exposes the destination path; every prefix
  ending at or after `Rename` leaves a complete, openable format-8 store.
- A post-rename sync failure returns `PublishedButUnsynced` and never removes
  the visible destination.
- A pre-rename failure leaves only a `<name>.staging.<nonce>` directory, and
  no constructor ever deletes one it did not create in the current call.
- Every format-7 open surface refuses.
- No release artifact contains a format-7 decoder or migration branch.

### Host gates

- TypeScript rejects native and structural thenables returned from **read and
  write** callbacks at runtime, with `ErrAsyncCallback`.
- A stashed TypeScript read instance throws `ErrUseAfterScope` after callback
  return.
- A stashed C instance ref returns misuse after callback return.
- Disposing a TypeScript builder, owned instance, or witness releases its
  native allocation and makes later calls throw `ErrSpentHandle`.
- `WriteOutcome` narrows on the single `tag` discriminant; the abandoned arm
  still vanishes from the sum when the callback cannot abandon.
- V8 external-memory accounting rises and falls with frozen catalog and native
  image capacity.
- A consumed builder cannot admit twice.
- TypeScript narrows the admission union before exposing instance methods.
- No N-API transmute fabricates a schema or read-instance `'static` lifetime.
- C: no union arm is read without `BDB_STATUS_OK`; the empty tag never
  accompanies `BDB_STATUS_OK`; every deleted or renamed error kind is absent
  from the regenerated header.

### Allocation and performance gates

- Allocation census demonstrates no allocation per frozen catalog entry.
- Admission telemetry reports $A$, $I$, $R$, $F$, $J$, and observed peak RSS.
- Observed phase peaks respect the declared peak-memory equation within
  allocator and runtime overhead measured by an empty-process control.
- The full Primer normalization corpus — sourced from the sibling
  `primer-spec` repository — completes through load, complete admit, keyed
  reads, representative joins, and raw persistence.
- The full Primer lane records wall time, CPU time, peak RSS, frozen bytes,
  image bytes, prepared/scratch/answer capacity, entry count, and allocation
  count.
- The API does not release until that lane shows no unexplained superlinear
  growth as fact count scales through at least four corpus prefixes.

## Refused designs

- `Instance<'static, S>` for heap ownership.
- `Box::leak` or a self-referential owned-and-borrowed instance.
- One public `Admitting` type with a heap-or-LMDB flag.
- One concrete `CatalogView` with an undocumented vtable or store enum.
- `dyn CatalogRead` on a GAT-bearing path.
- `PreparedQuery<'a, S>` as proof of exact instance identity.
- Deleting every prepared owner check.
- Replanning every durable query inside every read callback without a measured
  product decision.
- Dummy generation zero for frozen instances.
- View memoization that aliases different owned instances.
- Query methods on `InstanceBuilder` or `WriteTx`.
- Mutation methods on `OwnedInstance` or `ReadInstance`.
- Initial admission through the delta-restricted empty-plan judgment.
- A second checker or a reduced heap-only theory.
- Treating `verify_store` findings as admission violations.
- A per-entry `BTreeMap<Vec<u8>, Vec<u8>>` heap catalog.
- Persistence by decoded fact reinsertion, JS object round-trip, or relation
  images.
- Copying source `_meta` into a destination store.
- Blessing a format-7 store during ordinary open.
- Retaining a format-7 decoder in exhume, compact, or a migration branch.
- Healing a half-created destination instead of making the state
  unrepresentable.
- A spent-by-move witness — overturned here; evidence does not wear out.
- Hand-enumerated obligation rosters beside the statement spine.
- A host result union with two narrowing discriminants.
- Error identity by message-string matching.
- Type-level-only async guards without the runtime probe.
- Resurrecting the crashpoint macro harness instead of reifying the
  publication protocol.
- A fresh-row violation kind distinct from scalar functionality.
- An expected compare-and-swap answer spelled as an exception.
- A second process-local clock beside the generation.
- Re-deriving a statement's arm from an untagged id.
- A public enum whose variants void its own sealing claim.
- Parallel arrays on public payloads whose equal length is an assert.
- An alive bit stored inside the allocation whose death it reports.
- Bridge refusals minted with engine error kinds.
- Outcome kinds and wire tags outside the `tags.json` golden.
- In-band `{0, 0}` sentinels on new ABI payloads.
- Stored derived fields justified by citations that do not resolve.
- Splitting `Db::ephemeral` into create/open verbs — the wipe-and-initialize
  recovery is one atomic operation under the refusal-never-mutates law.
- Calling the existing ephemeral store an in-memory mode or a durability lie.
- Claiming Bumbledb admission proves Primer exact covers that the schema does
  not declare.

The representation is the feature. An accepted value carries the admission
fact. A builder cannot be queried. A read instance cannot escape its storage
lease. A packed catalog cannot be mutated. A half-created store cannot occupy
a destination path. A codec cannot see its backend. The kernel cannot see
storage. The runtime owner token records the one identity fact Rust lifetimes
do not express. The complete roster closes the one premise the incremental
checker legitimately assumes — and is derived, not listed, so it cannot
silently reopen.
