# Instance as the engine; LMDB as a lifetime

STATUS: PROPOSAL (not an implementation). This packet is the thing reviewers attack. It does not land code. Names churn. No current public name is reserved.

## Naming

Today’s types are accidents of duration. This proposal’s types are the product.

| Today | This proposal | Is |
|---|---|---|
| `Snapshot` | `Instance<'store, S>` | Borrowed engine view. Retired, not aliased. |
| *(no public heap type)* | `OwnedInstance<S>` | Owns the RAM catalog; `as_instance()` reborrows `Instance<'_>`. Not `'static`. |
| `WriteTx` / `InstanceBuilder` | `Admitting` | Mutable catalog. Load collections, `reserve`, overlay get. Not queryable. |
| `commit` / `seal` | `admit` | Parse: `Admitting` → `Instance \| Violations`. The only `Instance` constructor. |
| `Db` | `Durable` | Filesystem duration: path, writer mutex, fsync, MVCC. Lends `&Instance`. |
| persist / `createFrom` / `bulk_load` | `Durable::from_instance` | Copy encoded F/M/U/R/S/Q/dict. Not images. |
| `writable: bool` | `Catalog` / `CatalogMut` | Freeze is a type change. |
| `{ ok: bool }` | `Instance \| Violations` | Parse, don’t validate. |
| `env_instance` / `ForeignPreparedQuery` | `PreparedQuery<'store, S>` | Mixing A-on-B does not compile. |
| `PreparedQuery::staleness` | *(deleted)* | No writer on owned; lease-scoped plan on durable. |
| `ReadScope` / `using snap` | synchronous `read` callback | Borrowed TS/C read. Breaks 0.14 `snap.execute`. |
| `Db.memory()` / ephemeral | *(not a product)* | NOSYNC LMDB. Heap `Catalog` is the RAM lifetime. |
| `NaiveDb` | *(not the engine)* | Other oracle. Do not promote. |

`ReadTxn` stays the physical mmap lease, owned by the `Durable::read` stack, not by `Instance`. File citations below name today’s modules; they are not the product vocabulary.

## 1. Claim

The engine is `Instance`: an admitted, queryable, immutable state. Lean already names it — `State T` is an instance **carrying** its proof that it models the theory (`inst` plus `models : holds T inst`, [`lean/Bumbledb/Txn.lean`](lean/Bumbledb/Txn.lean) lines 145–154). The host never exposed that object because the only constructor today is “an LMDB write transaction succeeded.” That is a duration accident, not a type. LMDB is **not a second engine**. It is porous duration metadata around a lifetime: bytes that outlive the process, mapped into the address space, leased by `ReadTxn<'env>` ([`crates/bumbledb/src/storage/env.rs`](crates/bumbledb/src/storage/env.rs) lines 458–466), and lent into the engine as `&Instance<'store, S>` from that lease (or from `OwnedInstance`’s heap). Same methods. No `if lmdb` in plan, exec, judgment, or image build.

Rust already has this pattern. `Fact<'a>` is lifetime-parameterized on purpose ([`crates/bumbledb/src/api/db.rs`](crates/bumbledb/src/api/db.rs) lines 13–30: “the borrowed struct is the struct, and ownership is an explicit host act”). Today’s `Snapshot<'db, S>` already *is* that view, misnamed, and it wrongly *owns* the `ReadTxn` (same file, lines 407–417). The public name is `Instance`. RAM vs LMDB are two lifetimes of one `Catalog`, not two engines (Insight 16). `Durable` (today’s `Db`) is the durable lease: path, writer mutex, fsync, MVCC. Heap `OwnedInstance` is the other duration. Fsync lives on `Durable`, not on `Instance`. Today’s `Db.memory()` / `Db::ephemeral` are not a product — ephemeral is NOSYNC LMDB, a duration that lies about fsync.

Illegal states become unrepresentable with types and lifetimes, not flags. You cannot query `Admitting`. You cannot write an `Instance`. You cannot intern through a frozen `Catalog`. An empty catalog that does not `holds` is not an `Instance` and not a `Durable`. Parse, don’t validate.

## 2. Lean correspondence

Lean’s data model is already lifetime-free and storage-free. [`lean/Bumbledb/Schema.lean`](lean/Bumbledb/Schema.lean) line 571: `def Instance : Type := RelId → Set Fact`. A theory’s denotation at a closed relation is a constant of the theory, instance-independent (`den_closed_constant`, same file lines 595–606). `holds T I` is “every declared statement’s judgment holds of the final state” ([`lean/Bumbledb/Dependencies.lean`](lean/Bumbledb/Dependencies.lean) lines 282–292). Keys prove uniqueness, never existence — `functionality_of_empty` (same file, lines 47–50 and 394–399): every key holds of the empty fact set. Containment from an empty ordinary source is vacuous. An empty instance is therefore **lawful** for a large class of theories; it is not a special case to skip.

The lifecycle ([`lean/Bumbledb/Txn.lean`](lean/Bumbledb/Txn.lean)):

- `State T` = admitted instance carrying `models : holds T inst` (lines 145–154). No constructor mints one from an unjudged instance. `committed_states_model` (lines 426–438) is a field projection, which is the design working.
- `Delta` is a set pair, `adds` / `removes` (lines 158–166). Order does not exist in this representation. Bridge: today’s [`storage/delta.rs`](crates/bumbledb/src/storage/delta.rs) `WriteDelta`.
- `apply s d` is `(base \ removes) ∪ adds` (lines 168–172). Order-free by construction.
- Lean `commit s d := judge T (apply s d)` (lines 361–369). Accept iff `holds`; else the failing phase’s complete violation set. Bridge: today’s [`storage/commit/write.rs`](crates/bumbledb/src/storage/commit/write.rs) `commit()`, [`judgment.rs`](crates/bumbledb/src/storage/commit/judgment.rs) `judge`. Host name: `admit`.
- Empty collection is a no-op (`insert_is_fold` / `delete_is_fold`, lines 206–236). Singleton is `fs = [f]`. One algebra.

An empty-base admit is a host constructor whose verdict is still Lean `judge`. Lean `commit` is `judge T (apply s d)` with `s : State T` (lines 361–369) — `apply` will not take a raw `∅`. Empty-base admit is `judge T (applyOps emptyInstance ops)` (lines 201–204). `emptyInstance` is [`Countermodels.lean`](lean/Bumbledb/Countermodels.lean) line 561. Schema validation refuses closed-to-closed (`ClosedStatementRefuted`, [`validate.rs`](crates/bumbledb/src/schema/validate.rs) around 681, 750, 772). Closed-source → ordinary-target waits for admit (`validate.rs` 750–751: the target can shrink).

**Ruling:** `admit` is the only `Instance` constructor. `Durable::create` is `admit(∅)` then bind catalog bytes at a path. If ¬`holds T ∅`, create returns `Violations` — the same object empty heap `admit` returns. An unadmitted empty catalog is unrepresentable as `Instance` and unrepresentable as `Durable`. Do not document an exception.

`judge`’s signature is the FinalStateView law: a theory and **one** final instance. Operation order is not a parameter ([`Txn.lean`](lean/Bumbledb/Txn.lean) lines 306–325; [`judgment.rs`](crates/bumbledb/src/storage/commit/judgment.rs) lines 57–65: “operation order is no longer representable here”). The host type we have not exposed is exactly `State T`. `Instance` is `State T` whose bytes happen to be mmapped or heap-owned. That is the whole feature: mint `State` in-process without a filesystem, and stop pretending the mmap is the engine. The word Snapshot is retired.

## 3. Lifetime model

Illustrative Rust. It must compile in spirit. Names can move; the borrows cannot. Today’s `Snapshot` is the wrong word and is **retired**, not aliased.

### 3.1 The engine type is `Instance<'store, S>`

The engine is a **borrowed view** of an admitted catalog. It does not own a path, a writer, or a generation clock. It does not own the `ReadTxn`. The stack frame that called `Durable::read` owns the mmap lease; `OwnedInstance` owns the heap catalog. `Instance` is always a borrow of one of those two owners.

```rust
/// `'store` is the catalog **byte lease**:
/// - Durable: borrow of the `ReadTxn` sitting in `Durable::read`'s stack frame
///   (mmap pages, txn-stable by CoW)
/// - Owned:   borrow of `OwnedInstance`'s frozen `RamCatalog`
///
/// `'store` is never the `Durable` handle's lifetime, never `'static`,
/// never heed's parked `RoTxn<'static>`.
/// S is schema typestate.
pub struct Instance<'store, S> {
    catalog: CatalogView<'store>, // opaque. Exec does not match a store-kind enum.
    schema: &'store Schema,
    images: ImageAccess<'store>,  // Durable: &ImageCache + this generation; owned: PinnedImages. Join sees Arc only.
    marker: PhantomData<fn() -> S>,
}

impl<'store, S> Instance<'store, S> {
    fn prepare(&self, q: &Query) -> Result<PreparedQuery<'store, S>>;
    fn execute(&self, pq: &mut PreparedQuery<'store, S>, params: &[BindValue<'_>], out: &mut Answers) -> Result<()>;
    // scan / contains / get. No write. No generation. No witness. No staleness.
}
```

No `InstanceId` on `Instance`. Write-from identity is `Witness` (§3.6), not a field the plan compares.

Today’s `Snapshot<'db, S>` ([`db.rs`](crates/bumbledb/src/api/db.rs) lines 407–417) **is** this type, misnamed, and it **owns** a `ReadTxn` — that ownership is the defect. The public name **is** `Instance`. There is no `type Snapshot = Instance`, no deprecation window in the design, no second execute site under the old word.

`PreparedQuery<'store, S>` borrows this view (§7). Mixing A on B does not compile. Today’s `Db::prepare` / `Db::execute` open a hidden txn so the plan escapes `'store` — duration sugar that dies.

`CatalogView` is how bytes are reached. Store-kind matching is unrepresentable here. `from_instance` at the duration boundary may copy from the concrete catalog; exec, plan, judgment, and image build do not see a variant. `CatalogRef { Borrowed, Owned }` on the engine type is `if lmdb` with extra steps — refused.

### 3.2 Three things today’s `'db` conflated

Today `Snapshot<'db, S>` owns `ReadTxn<'db>` and borrows `ImageCache` / `Schema` / scratch from `Db` at `'db`. `ReadTxn` owns `RoTxn<'static, WithoutTls>` ([`env.rs`](crates/bumbledb/src/storage/env.rs) lines 458–466). `Fact::decode` takes `&'a Snapshot<'_>` ([`db.rs`](crates/bumbledb/src/api/db.rs) line 147). Those are three different facts:

| Thing | What it is | What it is not |
|---|---|---|
| `'store` on `Instance<'store, S>` | Catalog byte lease (stack `ReadTxn` borrow, or `&OwnedInstance`) | `Durable`’s handle lifetime; heed’s parked `'static` |
| `RoTxn<'static>` | Owned heed txn so `ParkedReader` can hold it across `Durable::read` calls ([`txn.rs`](crates/bumbledb/src/storage/env/txn.rs) lines 8–11; [`db.rs`](crates/bumbledb/src/api/db.rs) lines 357–361) | An Instance; a Fact lifetime; a JS handle |
| `Fact<'a>` | Variable-width fields borrowed from `&'a Instance<'_, S>` (today: `&'a Snapshot<'_>`, [`snapshot.rs`](crates/bumbledb/src/api/db/snapshot.rs) lines 312, 356–358) | Equated to `'store` when `'store` could outlive this view |

Law: **`Durable::read` never returns `Instance`.** It lends `&Instance` to a closure (Rust) or a synchronous callback (C, TS). The stack frame owns the `ReadTxn`. End of closure / callback ends `'store` for that view. Parked reuse wraps the raw txn back into a `ReadTxn` on the next `read`; a parked `RoTxn` is not an Instance ([`read.rs`](crates/bumbledb/src/api/db/read.rs) lines 13–52).

Schema and image-cache pointers on `Durable` outlive any one txn. Instance **reborrows** them at `'store` (the lease), so they cannot be stored as handle-lived Facts.

Do **not** write `Fact<'store>` as if `'a = 'store` always. Today’s `get` / `scan_facts` bind `'a` to `&self`. Keep that. Equating them in prose is how a Fact outlives the mmap lease: `Instance<'db>` with `'db` = the handle, `fn get(&self) -> Fact<'db>`, host stashes the Fact, txn parks or drops, dict bytes dangle. Dynamic `scan` already copies intern bytes into owned `Value`s (`Box::from(dict::resolve(...))`, [`snapshot.rs`](crates/bumbledb/src/api/db/snapshot.rs) lines 126–130). FFI uses that copy. Typed `scan_facts` borrows. Two representations, two methods.

After the rename:

```rust
impl<S> Durable<S> {
    pub fn read<R>(&self, f: impl FnOnce(&Instance<'_, S>, Witness<S>) -> Result<R>) -> Result<R> { ... }
}
```

The `ReadTxn` lives in this function. Step 3 of today’s `read.rs` builds a view, not an owning Snapshot. The closure receives `&Instance` plus a `Witness` minted from that same txn (§3.6). Generation is not an Instance method.

### 3.3 `OwnedInstance<S>` owns the RAM catalog

`'static` on a borrow means the referent lives for the rest of the program. `Box::leak` is a leak. Refused.

`'static` on heed’s `RoTxn` means the txn is owned, so `ParkedReader` can hold it. That encoding stays **inside `Durable`**. It is not Instance’s lifetime and not a JS handle.

`'static` on napi `PreparedQuery` today is a transmute because the handle co-owns `Arc<Db>` and drop-order keeps schema alive ([`ts/crate/src/lib.rs`](ts/crate/src/lib.rs) lines 1475–1489). That is FFI self-ref, not an owned Instance. Do not reuse it as the RAM product.

Cow-in-one-struct (yoke, ouroboros): Instance owns and borrows `RamCatalog`. Self-referential. The codebase has no such pattern. Refused.

`Instance<'static, S>` as the public owned type: teaches `'static` = RAM. Refused.

Two arities of the word `Instance` (`Instance<S>` next to `Instance<'store, S>`, like `Cow`) hide that the owned product is not a borrow. Two names.

```rust
/// Host-owned admitted catalog. The engine view is `as_instance()`.
pub struct OwnedInstance<S> {
    catalog: Box<RamCatalog>,
    schema: Schema,
    images: PinnedImages, // OnceLock per open relation. No generation clock. No dummy GenerationId(0).
    marker: PhantomData<fn() -> S>,
}

impl<S> OwnedInstance<S> {
    pub fn as_instance(&self) -> Instance<'_, S> { /* CatalogView borrows &*self.catalog */ }
    pub fn prepare(&self, q: &Query) -> Result<PreparedQuery<'_, S>> {
        self.as_instance().prepare(q)
    }
    pub fn execute(&self, pq: &mut PreparedQuery<'_, S>, params: &[BindValue<'_>], out: &mut Answers) -> Result<()> {
        self.as_instance().execute(pq, params, out)
    }
    // scan / contains / get: same forwarding
    // no generation, no witness, no write, no intern, no staleness, no identity nonce
}
```

Hosts who `admit` hold `OwnedInstance<S>`. Hosts who `Durable::read` hold `&Instance<'_, S>` for the callback. One engine view, two owners.

Verdict: **owned heap ≠ leak.** `'store` on `Instance<'store, S>` is always a borrow of a catalog someone else owns.

### 3.4 `Admitting` is the mutable type

Not a second builder algebra. Today’s `WriteTx` is Admitting over a borrowed Durable base. The RAM constructor is Admitting over ∅. Same verbs. Queries unrepresentable.

```rust
/// Mutable catalog. Lean insert/delete/reserve. No prepare, no execute, no scan.
pub struct Admitting<S> {
    schema: Schema,      // owned. Not a `'static` borrow.
    catalog: RamCatalog, // or owned LMDB write-txn lease + delta, for Durable::write
    delta: WriteDelta,   // lifetime-free — see below
    phase: WritePhase,
    marker: PhantomData<fn() -> S>,
}

impl<S> Admitting<S> {
    pub fn new(schema: S) -> Result<Self>;
    pub fn load<'f, F: Fact<'f, Schema = S>>(&mut self, facts: impl IntoIterator<Item = &'f F>) -> Result<MutationReport>;
    pub fn delete<'f, F: Fact<'f, Schema = S>>(&mut self, facts: impl IntoIterator<Item = &'f F>) -> Result<MutationReport>;
    pub fn reserve<T: Fresh<Schema = S>>(&mut self, count: u64) -> Result<FreshRange<T>>;
    pub fn contains(&self, fact: &impl Fact<Schema = S>) -> Result<bool>;
    pub fn get(/* keyed, overlay */) -> Result<Option<Fact<'_>>>;
    /// Parse: judge(apply(∅, Δ)) → OwnedInstance | Violations. Consumes self.
    pub fn admit(self) -> Result<OwnedInstance<S>, Violations>;
}
```

Today `WriteDelta<'s>` stores `schema: &'s Schema` ([`delta.rs`](crates/bumbledb/src/storage/delta.rs) lines 123–124). That lifetime is the Db-borrowed schema on today’s `WriteTx`. Putting `WriteDelta<'static>` on a constructor that **owns** `Schema` is the leak reading. **Drop the schema pointer from `WriteDelta`.** Schema lives on the admitting owner. Delta operations take `&Schema` at the call. The delta becomes lifetime-free. The constructor is then one struct with no self-borrow and no `'static`.

`admit` returns `OwnedInstance<S>`. Not `Instance<'static, S>`. Failure is `Violations` (a sum — parse, don’t validate). Poisoned → `TransactionPoisoned`, not a judgment.

`Durable::write` is Admitting over the current generation; admit replaces the duration’s catalog. One algebra, two bases (∅ or borrowed). `Admitting` over a Durable base owns the write-txn lease the way today’s `WriteTx` owns it — the public type still has no `'static` Instance. `Admitting::from(&Instance)` that copies a catalog is the same function with a nonempty base and no path — a copy, not a lifetime trick; not required before the ∅ constructor Primer needs.

Lifetime **ends at admit**. On `Ok`, today’s `commit()` consumes the delta ([`write.rs`](crates/bumbledb/src/storage/commit/write.rs) lines 84–87). On `Err` or panic, `EscapedIdBurn` flushes escaped fresh ids ([`write.rs`](crates/bumbledb/src/api/db/write.rs) lines 40–83). After admit, the world is an Instance again. Do not grow query methods on Admitting.

### 3.5 Dict `resolve`; Answers; `Fact<'a>`

[`dict.rs`](crates/bumbledb/src/storage/dict.rs) line 147: `resolve<'txn>(txn: &'txn ReadTxn<'_>, id) -> &'txn [u8]`. Intern only in write transactions (module doc lines 15–16). Pending interns are `BTreeMap<Box<[u8]>, u64>` ([`intern.rs`](crates/bumbledb/src/storage/delta/intern.rs) lines 34–38, 76) — Box, not Arena. Arena stores fact bytes as **index handles** ([`arena.rs`](crates/bumbledb/src/arena.rs) lines 5–6, 13–16) so chunk realloc does not dangle pointers.

| Phase | Dict | Fact borrow |
|---|---|---|
| Admitting | `intern` mints; resolve pending-first then committed | `Fact::decode_write(&'a Admitting)` → `'admitting`. Pending: Box in the map (stable). Committed: mmap of the base view. |
| Execute (`Instance`) | `resolve` only; intern unrepresentable (`Catalog`, not `CatalogMut`) | `Fact::decode(&'a Instance<'_>)` → `'a`. Dict: lease pages / frozen RAM. |

Keep two decode methods. Unifying them is a flag.

**Answers do not borrow the catalog after finalize.** [`finalize.rs`](crates/bumbledb/src/api/prepared/finalize.rs) lines 41–48 drain, resolve each distinct intern once into the buffer’s byte heap ([`answers.rs`](crates/bumbledb/src/api/prepared/answers.rs) lines 38–40). `AnswerValue<'_>` borrows `Answers`. `execute_collect` outlives the Instance view. FFI ships owned `Answers` (napi already crosses the whole buffer). JS/C never expose `Fact<'a>`.

### 3.6 `Witness` is durable-lease metadata, not an Instance method

[`Witness<S>`](crates/bumbledb/src/api/db/write.rs) lines 86–103: environment identity + `GenerationId`, private fields, minted only by today’s `Snapshot::witness()`. Findings 018/021: a dangling witness of a closed snapshot is unrepresentable because the value **moves**; napi does not store `&Snapshot` ([`ts/crate/src/lib.rs`](ts/crate/src/lib.rs) lines 21–27, 555–557). `write_from` compares inside the writer critical section ([`write.rs`](crates/bumbledb/src/api/db/write.rs) lines 159–170, 256–263). Mismatch is `GenerationMoved`. Hosts cannot fabricate a generation integer.

Generation is an affine exception of “this catalog is one of a sequence at a path” ([`docs/design/representation-first.md`](docs/design/representation-first.md) lines 65–72). It does not belong on the engine type. `Instance` has no `witness()` and no `generation()`. `Durable::read` mints both the view and the witness from the **same** `ReadTxn` and passes them as two arguments. Every durable read is a generation in a sequence, so every `Durable::read` closure receives a `Witness`. Ignoring it is fine. Minting one from `OwnedInstance` is unrepresentable (no method, no closure argument). `write_from` stays on `Durable`.

```rust
pub struct Witness<S> {
    identity: InstanceId, // Durable(env) only — the type does not admit Owned
    generation: GenerationId,
    marker: PhantomData<fn() -> S>,
}

impl<S> Durable<S> {
    pub fn write_from<R>(&self, witness: Witness<S>, f: impl FnOnce(&mut Admitting<S>) -> Result<R>) -> Result<R> { ... }
}
```

`Witness` storing `InstanceId::Owned(_)` is unrepresentable if the constructor only exists on the durable read path. Do not put an `Option<GenerationId>` on Instance. `InstanceId` here is the durable env token for `write_from`. It is not a PreparedQuery field and not on Instance (§7).

### 3.7 Send + Sync

- `Durable`: `Send + Sync` (already true of today’s `Db`, [`db.rs`](crates/bumbledb/src/api/db.rs) lines 3–6, 265–267).
- `OwnedInstance`: `Send + Sync`. Frozen maps, frozen images, frozen schema. Compile-test it.
- `Instance<'store, S>`: **do not claim `Sync`.** A durable view borrows a `ReadTxn` whose `OnceCell<GenerationId>` is `!Sync` ([`env.rs`](crates/bumbledb/src/storage/env.rs) line 465) and whose heed txn is thread-affine. Concurrent `&Instance` on one RoTxn is an LMDB contract violation. Keep `OnceCell` — do not upgrade to `OnceLock` to claim `Sync`. Share RAM by sending `&OwnedInstance` and calling `as_instance()` on the receiving thread.
- `PreparedQuery`: `!Sync` stays ([`prepared.rs`](crates/bumbledb/src/api/prepared.rs) lines 171–182). Compile-fail already in tree.
- `Admitting`: `Send`, `!Sync`. The napi write worker exists because today’s `Db::write` is closure-scoped; Admitting over ∅ is an owned value and does not need that park.

## 4. Catalog

Not a thin `ReadInstance` of four methods. Execution, image build, key-probe, *and* admit all need the vocabulary [`storage/read.rs`](crates/bumbledb/src/storage/read.rs) lines 1–9 already documents, plus `Q` (fresh / row-id high-water, [`50-storage.md`](docs/architecture/50-storage.md) key layout) and `_dict`:

```text
F  image build / fetch / scan / export     read.rs fetch, scan; image/build.rs
M  membership / point lookup               read/fact_row.rs
U  functionality / determinant probe       read determinant_row; key_probe_fact.rs
R  reverse edges (judgment + sweeper)      applier.rs puts/dels; verify_store
S  planner row counts / image sizing       read/row_count.rs
Q  fresh sequences / row-id next           delta/alloc.rs; image cache append boundary
dict  intern (admit) / resolve (execute)   storage/dict.rs
```

Illustrative trait — **engine vocabulary**, not an LMDB wrapper:

```rust
trait Catalog {
    fn scan_f(&self, rel: RelationId) -> impl Iterator<Item = Result<(u64, &[u8])>>; // row_id, fact
    fn membership(&self, rel: RelationId, hash: &[u8; 32]) -> Result<Option<u64>>;
    fn determinant(&self, rel: RelationId, stmt: StatementId, key: &[u8]) -> Result<Option<u64>>;
    fn fetch_f(&self, rel: RelationId, row: u64) -> Result<&[u8]>;
    fn row_count(&self, rel: RelationId) -> Result<u64>; // S
    fn row_id_next(&self, rel: RelationId) -> Result<u64>; // Q
    fn resolve(&self, word: u64) -> Result<&[u8]>;
    fn lookup(&self, bytes: &[u8]) -> Result<Option<u64>>; // committed dict; execute never mints
    fn entry_count(&self) -> Result<u64>; // image ceiling (LMDB: mdb_stat; RAM: map len)
}

/// Encoded ordered map. Apply and Checker are generic over this.
/// Neighbor probes, R-prefix walks, SortedGets, NO_OVERWRITE, cursor-delete live here.
trait OrderedKv {
    fn get(&self, key: &[u8]) -> Result<Option<&[u8]>>;
    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<()>;
    fn put_no_overwrite(&mut self, key: &[u8], val: &[u8]) -> Result<bool>;
    fn del(&mut self, key: &[u8]) -> Result<bool>;
    fn lower_than(&self, key: &[u8]) -> Result<Option<(&[u8], &[u8])>>;
    fn greater_than(&self, key: &[u8]) -> Result<Option<(&[u8], &[u8])>>;
    fn gte(&self, key: &[u8]) -> Result<Option<(&[u8], &[u8])>>;
    fn range_from(&self, start: &[u8]) -> impl Iterator<Item = Result<(&[u8], &[u8])>>;
}

trait CatalogMut: Catalog + OrderedKv {
    fn intern(&mut self, bytes: &[u8]) -> Result<u64>;
}
```

`CatalogMut` is only for `Admitting`. Instance holds `Catalog` (frozen). Freeze is a type change, not a runtime flag `writable: false`. Judge runs **before** freeze, on `CatalogMut` as `Catalog` (write-txn read-your-writes).

**LMDB implements this with heed behind the lifetime.** `LmdbView<'txn>` is `Catalog`. `LmdbMut` is today’s `Applier` + `WriteTxn`. Heed vanishes from `exec/` / `judgment.rs` / `image/build.rs` **if** `OrderedKv` is the admit unbind. A Catalog of only `scan_f` / `membership` / `determinant` cannot express `get_lower_than`, `RoRange`, `NO_OVERWRITE`, or `range_mut`+`del_current`. That is the broken “one judgment, two catalogs” claim.

**RAM implements the same encoded keys** — not decoded `BTreeSet`s. Ordered so neighbor probes are B-tree range. Property: heap `admit(Δ)` vs `Durable` `admit(Δ)` → identical `Violations` or identical F/M/U/R/S/Q/dict bytes. Not vs ephemeral (not a product). **Do not promote `NaiveDb`.**

Refuse `enum Store { Lmdb(Environment), Memory(RamCatalog) }` in exec, plan, judgment, or image build. That enum is the `if lmdb` flag with extra steps. Instance holds an opaque `CatalogView`. A store-kind enum, if it exists at all, is a duration-boundary copy concern (`from_instance`), and nothing in exec matches it.

**Judgment / applier / `verify_store`:** generic `OrderedKv` for `Applier` and `Checker`. Neighbor probes and `SortedGets` are per-edge — do not `dyn` them. `dyn` is fine for `verify_store` (O(store), offline). One `Checker`, never a sweeper copy. Restate “LMDB key uniqueness” as encoded-key uniqueness. `image::build` uses `Catalog::entry_count`, not `read::data_entries` (`mdb_stat` is an `if lmdb` in the image path).

## 5. Admit is the only Instance constructor

One pipeline. Identical `Violations`. No second validator.

```text
Admitting (∅-base or Durable::write over a base Instance)
        │  load / delete / reserve            (collection algebra; overlay contains/get)
        ▼
   WriteDelta          storage/delta.rs  (net dispositions, arena, pending intern)
        │
        ▼
   plan_commit         storage/commit/plan.rs  (pure function of (delta, schema);
                       “before a single LMDB page is touched” — before a single
                       CatalogMut put, after this proposal)
        │
        ▼
   apply(CatalogMut)   storage/commit/apply.rs  (phase 1 deletes, phase 2 inserts;
                       key conflicts record scan-complete, preempt phase 3)
                       heap: scratch map so abort = discard (Durable: drop WriteTxn)
        │
        ▼
   judge(&impl Catalog)  FinalStateView — CatalogMut after apply, BEFORE freeze
        │
        ├─ Violations  →  abort; CatalogMut discarded; Admitting gone
        │
        ▼
   phase 4             S / Q / dict flush into CatalogMut  (heap AND mmap)
        │
        ▼
   freeze              CatalogMut → Catalog  (type change)
        │
        ├─ Durable     phase 5 txn.commit() fsync, commit_bounded (CommitSync)
        │              lives on Durable, not Instance
        │
        ▼
   images              build / synthesize_closed / cache.advance
        │
        ▼
   Instance            prepare, execute, scan, contains, get
```

Today’s `commit()` is this plus phase 4 and phase 5, wrapped in `commit_bounded` for `CommitSync`. Heap `admit` **omits phase 5** (fsync lives on `Durable`). It does not omit judgment or phase 4 (`S`/`Q`/dict must land before freeze — `image::build` sizes from `S`). Empty-delta on an **already-admitted** Durable base skips re-judge (`committed_states_model`) but still flushes `Q`. Empty `admit` of a ∅-base **is** `judge(∅)` plus `Q` flush — not that shortcut. `Durable::create` is that empty admit, then bind. If ¬`holds T ∅`, `Violations`. Heap `admit` must not enter `commit_bounded` (a second apply would double-insert). `WriteDelta::present` / `reserve` / `Selections::encode` retarget from `ReadTxn` to `Catalog`.

Empty `Admitting` + `admit` = `judge(∅)` plus `Q` flush = `Durable::create`. Load then admit = `judge(apply(∅, Δ))`. Collection empty `load([])` is a no-op (`MutationReport::EMPTY`, [`insert.rs`](crates/bumbledb/src/api/db/insert.rs) lines 7–10) and does not change that. Theories that need facts: `Admitting` → `admit` → `Durable::from_instance`. Open of *pre-proposal* empty files that would fail `holds` is migration, not a type exception.

Poison: [`api/db/apply.rs`](crates/bumbledb/src/api/db/apply.rs) lines 41–48, 94–102 — parse-then-apply; a shape failure never leaves `Clean`; after a prefix entered, later mutation is `TransactionPoisoned`. [`insert_dyn.rs`](crates/bumbledb/src/api/db/insert_dyn.rs) lines 11–13: the whole collection is parsed before any row enters the delta. Today’s `InstanceBuilder::insert` uses this exact loop. Internal collection-arena transport must preserve “no prefix on shape failure.” `admit` of Poisoned Admitting is `TransactionPoisoned` ([`write.rs`](crates/bumbledb/src/api/db/write.rs) lines 291–300), not `Violations`. Poison is not a judgment. Two sums: storage/shape vs `holds`.

Closed-relation writes remain a type-level refusal on the write surface ([`db.rs`](crates/bumbledb/src/api/db.rs) lines 476–490), preempting final-state formation — Lean records this as outside the model ([`Txn.lean`](lean/Bumbledb/Txn.lean) lines 92–96). `Admitting::load` of a closed relation is the same error.

### Two encodings of Lean `apply`, two types

Today’s WriteTx overlay ([`get.rs`](crates/bumbledb/src/api/db/get.rs) lines 375–404) is Lean `apply` computed from delta maps + base view, **before** CatalogMut put. Judgment ([`judgment.rs`](crates/bumbledb/src/storage/commit/judgment.rs) lines 57–65) is Lean `apply` materialized on the catalog after phases 1–2. Existing comments call these “the same final-state view” — that is the **semantic** law. Physically they are two representations. Types stay split. No `read_your_writes: bool`. Overlay `Fact<'admitting>` cannot survive admit: admit consumes the delta; `&self` vs `&mut self` already forbids holding a Fact across insert. Judge on `CatalogMut` as `Catalog`, then freeze — freeze first would require a second txn or a flag.

## 6. Durable / LMDB as duration

These are duration, locking, and crash. They must not leak into exec.

| Concern | Where it lives | Not in |
|---|---|---|
| Path | `Environment` / `Durable::create` / `open` | Instance methods |
| Map size 32 GiB, not a knob | [`env.rs`](crates/bumbledb/src/storage/env.rs) `MAP_SIZE` line 198 | exec |
| fsync / `CommitSync` / `commit_bounded` | Durable phase 5 | judgment, heap admit |
| Writer mutex, non-reentrant | `Durable.writer` | Instance (immutable) |
| MVCC reader slots, `ReadersFull` | LMDB max_readers 1024 ([`50-storage.md`](docs/architecture/50-storage.md)) | join kernel |
| Parked txn reuse | `CommitSeq` + `ParkedReader` ([`db.rs`](crates/bumbledb/src/api/db.rs) 66–71, 357–361; [`read.rs`](crates/bumbledb/src/api/db/read.rs)) | Instance API |
| Compact | live-page copy of a Durable, fsync dirent chain | `from_instance` (encoded kv copy) |
| Exhume | [`exhume.rs`](crates/bumbledb/src/api/db/exhume.rs) — read-only, theory-less, no write, no prepare | Instance (Instance always has a theory) |
| Format version, fingerprint, kind byte | `_meta` at open | exec |
| `MDB_MAP_FULL` | commit error; remedy is a new store | RAM (heap OOM is a different ceiling) |

**Ephemeral is not a product.** Today’s `Db.memory()` / `MDB_NOSYNC` is a duration that lies about fsync. Heap Catalog is the RAM lifetime. `Durable` is the fsync lifetime. Do not keep a third `StoreKind`.

**Generation is durable-lease metadata, not an Instance method.**

[`GenerationId`](crates/bumbledb/src/storage/env.rs) lines 125–158 is the persisted `_meta` tx id, advanced on state-changing commits, initial 0 at create. [`CommitSeq`](crates/bumbledb/src/api/db.rs) lines 66–71 is process-local, resets on open, **deliberately not comparable** with `GenerationId`. Image cache keys on snapshot-sourced `GenerationId` ([`image/cache.rs`](crates/bumbledb/src/image/cache.rs) lines 4–17), never on CommitSeq.

Homogeneous-coordinates analogue ([`docs/design/representation-first.md`](docs/design/representation-first.md) lines 65–72): translation is an affine exception until you add a homogeneous 1 and use one matrix multiply. Generation is the affine exception of “this catalog is one of a **sequence** at a path.” An owned Instance is not in a sequence. The homogeneous move for **execute** is the `'store` borrow: mixing prepared-from-A on B does not compile. Putting `InstanceId` on Instance so the plan can `==` is the integer flag with extra steps (§7). Generation stays in `ImageCache` / `Witness`, not on Instance, not in `run_join`. Exec, plan, judgment, and image **build** see `Instance` / `Catalog`, not the mint. The image **cache** on `Durable` still keys LMDB images on generation, because Durable is the sequence. Owned Instance pins images. A `'store`-bound plan sees one epoch, so **`run_join` does not call `txn.generation()`**. If it still does, the affine exception leaked back into the kernel — that is a review reject.

Owned Instance does not need a persisted generation clock. Giving it a dummy `GenerationId(0)` so the cache code can stay unchanged is the homogeneous fake 1 used as a branch in disguise. Cache lives on `Durable`; owned Instance owns `Arc<RelationImage>` slots keyed by `RelationId` only.

`Durable::read` lends `&Instance` of generation N after a successful `create` / `from_instance` / `open` of an admitted store. `GenerationId::initial()` is 0 ([`env.rs`](crates/bumbledb/src/storage/env.rs) lines 149–152). Whether create’s admit stamps 0 or 1 is not load-bearing. After `k` state-changing admits, `read()` lends a view of generation `k` — the `Witness` carries that id, Instance methods do not.

**Persist** = `Durable::from_instance`: copy encoded F/M/U/R/S/Q/dict (+ `_meta`) into a new LMDB env in **one** write txn. No re-judgment (the Instance already carries `holds`). No JS fact round-trip. No image rebuild as source of truth. Q **must** be copied or a later durable `reserve` reissues ids. [`image.rs`](crates/bumbledb/src/image.rs) lines 34–36: “row ids exist only in LMDB keys and never appear in images.” Positions are dense scan ordinals. Deletes punch holes in row-id space; images compact them away. Ordinary relations cannot reconstruct `F` keys from images. `U`/`M`/`R`/`Q`/`S`/dict are not in images at all. “CatalogMut apply of already-judged entries” re-mints row ids and resets `Q`. Refused. Compact is a different sentence: live-page copy of one Durable onto another path. Dispatch on the concrete catalog is allowed **here**. Exec still must not match on store kind.

## 7. Execute / images / prepared

### 7.1 `PreparedQuery<'store, S>` borrows the Instance; execute is one site

Today `PreparedQuery` stores `env_instance: u64` ([`prepared.rs`](crates/bumbledb/src/api/prepared.rs) lines 183–189; [`build.rs`](crates/bumbledb/src/api/prepared/build.rs) line 316). Execute against any other environment is `Error::ForeignPreparedQuery` ([`bind.rs`](crates/bumbledb/src/api/prepared/bind.rs) lines 39–43) — a raw u64 compare. Today’s `Snapshot::execute` then stuffs `ReadTxn` + `ImageCache` into `PreparedQuery::execute` ([`snapshot.rs`](crates/bumbledb/src/api/db/snapshot.rs) lines 17–24). Two sites, an integer identity, a runtime branch.

Bind the plan to the Instance by **type**. `PreparedQuery<'store, S>` borrows `'store`. Mixing A on B does not compile. Delete `env_instance`. Delete engine `ForeignPreparedQuery`. Delete `PreparedQuery::execute(txn, cache)`. `Instance::execute(&self, pq, params, out)` is the only site. Replacing the integer with `InstanceId` (nonce vs env) is the same flag — refused. JS/C cannot express `'store`; mixing there is a **host-boundary** error, not a join-path check and not a reason to keep the integer in Rust.

```rust
impl<'store, S> Instance<'store, S> {
    fn prepare(&self, q: &Query) -> Result<PreparedQuery<'store, S>>;
    fn execute(&self, pq: &mut PreparedQuery<'store, S>, params: &[BindValue<'_>], out: &mut Answers) -> Result<()>;
}
```

What execute passes (no `ReadTxn`, no `ImageCache` in the join kernel):

```text
instance.execute(&mut pq, params, out)
    check: pq already borrows this `'store` — no env_instance integer
    bind / PendingIntern:     Catalog.lookup
    key-probe:                Catalog U/M/F + lookup  (never ImageSource)
    run_join EDB:             Instance::image → Arc<RelationImage>
    run_join Interior/rec:    PreparedQuery transients ('exec ⊂ 'store)
    view memo:                Closed vs ordinary-EDB (no GenerationId)
    finalize / String finds:  Catalog.resolve
```

If `run_join` still calls `txn.generation()`, reject. If execute still lives on `PreparedQuery` taking `ReadTxn`, reject. If `Snapshot::execute` remains a parallel path, reject.

Consequences:

- **Pin-at-prepare across generations dies.** A plan from generation 5 cannot be held until generation 7 because the generation-5 view is gone. That dual was compensated by `PreparedQuery::staleness`. After the bind, pinned `S` and live `S` are the same catalog. **Delete staleness.** Owned Instance has no writer — the type has no staleness, not a method that returns 1.0. Do not put never-stale on `OwnedInstance`.
- **`Durable::prepare` dies as an engine entry.** Today’s `Db::prepare` opens its own txn so the plan escapes `'store`. LMDB: prepare and execute inside one `Durable::read`. Owned: prepare once, execute many — the catalog does not move.
- **View memo does not key on `GenerationId`.** Today’s memo keys parked COLTs on `ViewGeneration::Storage(GenerationId)` ([`prepared.rs`](crates/bumbledb/src/api/prepared.rs) lines 688–695; [`view_memo.rs`](crates/bumbledb/src/api/prepared/view_memo.rs) lines 14–37) because one plan outlived many snapshots. A `'store`-bound plan sees one epoch. Memo keys Closed vs ordinary-EDB / residual filters. Cache on `Durable` still keys generation **inside `Instance::image`**. Closed stays `ViewGeneration::Closed` (theory, not a fabricated storage generation).
- **JS cannot express `'store`.** Mixing is a host-boundary error, not a join-path `Error`. Do not design Rust around `BDB_ERROR_KIND_FOREIGN_PREPARED`. A dummy id of 0 meaning “RAM” would collapse every owned Instance (`Environment::instance` is never 0). Refused because the integer is gone, not because a better integer exists.

### 7.2 Join is images; key-probe is Catalog

The join kernel already consumes `Arc<RelationImage>` ([`api/prepared/run_join.rs`](crates/bumbledb/src/api/prepared/run_join.rs) lines 21–35). [`exec/run.rs`](crates/bumbledb/src/exec/run.rs) lines 1–20: “Everything is a monomorphized generic — no `dyn` anywhere in the hot path.” **Join stays on RelationImage. Zero vtable. Zero Catalog.**

`run_join` still takes `ReadTxn` today for two reasons only: `cache.get_or_build` and `txn.generation()?` (line 37) to stamp the view memo. That claim is true of `run_join` and **false of execute** (bind, key-probe, finalize, `dict::lookup` all take the txn too). After the reveal:

- `Instance::execute` is the one site. It passes `Catalog` to bind / key-probe / finalize, and `Instance::image` to `run_join`. No `ReadTxn`. No `ImageCache` in the join kernel. No `InstanceId`.
- View memo keys Closed vs ordinary-EDB. Not generation: the plan cannot outlive `'store`. `ImageCache` on `Durable` still keys LMDB images on `GenerationId` — inside `Instance::image`, not `run_join`.

**Key-probe** is a Catalog point get (`U`/`M` + `F`, remaining filters on fact bytes). That is not LMDB. The dual with join is unique-key lookup vs conjunctive join — already true on disk. The accidental dual was `ReadTxn`. Do not synthesize key-probe from an image scan. Images have no `U`/`M`, no row ids. Generic `C: Catalog` on the probe; **no `dyn Catalog` inside `run.rs` / COLT.** Classify already refuses closed and Interior for key-probe ([`classify.rs`](crates/bumbledb/src/exec/dispatch/classify.rs) lines 89–103) — those go through images because they have no `U`/`M`. Tests forbid image build on the probe path. RAM implements encoded `U`/`M`. Interiors may still key-probe EDB while joining derived images — one execute, two Catalog operations, no store-kind match.

**Image build** ([`image/build.rs`](crates/bumbledb/src/image/build.rs) line 200 `build(txn, schema, rel)`) scans `F` via Catalog. Generic. Closed relations never touch the scan: `synthesize_closed` ([`image.rs`](crates/bumbledb/src/image.rs) export; [`get_or_build.rs`](crates/bumbledb/src/image/cache/get_or_build.rs) lines 57–61) is theory → image, no catalog, no generation. `scan_f` / `build` are ordinary-relation operations. Closed branches **before** Catalog. Catalog is the store of **instance** facts, not of theory axioms. Instance still answers `scan` / `contains` / `get` on closed rels via the schema’s sealed extension — the same virtual storage.

**Owned images.** Cache lives on `Durable`. Owned Instance pins `Arc<RelationImage>` per `RelationId` (`OnceLock`, like closed images today). Dummy `GenerationId(0)` on owned would alias a never-written Durable (`GenerationId::initial()` is already 0). `Instance::image` is the unbind. Join sees `Arc` either way. Matching `ImageAccess` / `CatalogRef` inside `run_join` is `if lmdb`; matching inside `Instance::image` is allowed.

**Rec/reach transients.** [`api/prepared/reach.rs`](crates/bumbledb/src/api/prepared/reach.rs): derived images live on `PreparedQuery` for an execution. They are not catalog facts. They must not be written to Catalog. They must not enter `ImageCache` or `PinnedImages`. `run_join`’s Interior arm never `get_or_build`s, never `memo.bind`s, never parks ([`run_join.rs`](crates/bumbledb/src/api/prepared/run_join.rs) lines 85–121). Lifetime `'exec` ⊂ `'store`. Instance does not retain them. Unbind between rounds so `TransientImage` can `get_mut`. Dropping the prepared query frees them.

## 8. Host representation

The host must expose the engine types. Names may move. 0.14 call shapes are not a constraint. Collection empty/singleton/many stays because it is Lean `insert_is_fold`, not because the SDK froze it. Columnar is **transport** of that algebra, not a second algebra.

Brooks: the tables are the program. Pike: data dominates. Minsky: illegal states unrepresentable. King: parse, don’t validate. Insight 16: collapse accidental special cases; do not smash essential ones into a flag.

### 8.1 The types

| Type | Is | Is not |
|---|---|---|
| **Admitting** | Mutable catalog. Loads collections. `reserve` if the theory has Fresh. Overlay `contains`/`get`. Poison. Closed-relation refusal. | Queryable. An Instance. A Durable. |
| **Instance** | The only query/scan/get/prepare/execute type. Admitted, immutable, `holds`. Always a borrow (`'store`). | Writable. A duration. A parked worker. The heap owner. |
| **OwnedInstance** | Owns the RAM catalog. Reborrows `Instance<'_>`. `Send + Sync`. May cross `await`. | `'static` Instance. A Durable. A generation. |
| **Durable** | Durable lease: path, writer mutex, fsync, MVCC. `read` lends `&Instance` + `Witness`. `write` is Admitting over that base. | The engine. Heap. Ephemeral. |
| **from_instance** | Give an Instance a durable lifetime. Copy encoded F/M/U/R/S/Q/dict, one txn, no re-judgment. | Re-inserting facts. Images. CatalogMut apply of judged entries. Compact. |

`admit` **parses** Admitting → `Instance | Violations`. A sum. Not `{ ok: bool }` — that discards the proof and forces every caller to switch on a flag King already named. Write’s success is a new current Instance at a path (duration), not an owned catalog — don’t smash admit and write into one result type. Poison is a third summand (`TransactionPoisoned`), not Violations.

In a language without `'a`, the borrowed view is a **synchronous callback argument**, not a `ReadScope` handle you store.

`S` row counts are Catalog facts of the Instance. `instance.row_count(rel)` is a Catalog read, not a join. Empty `r.count()` is empty, not zero — that is why the aggregate is the wrong representation for cardinality.

### 8.2 Rust

```rust
let mut a = Admitting::new(Ledger)?;           // ∅ base; validates descriptor; no judge
a.load([&Account { id, balance }])?;           // singleton
a.load(accounts.iter())?;                      // many
a.load::<Account>([])?;                        // empty
a.reserve::<AccountId>(n)?;                    // minting ∈ Admitting
let owned: OwnedInstance<Ledger> = a.admit()?; // Instance | Violations
// a is gone. owned has no load.

owned.prepare(&q)?;
owned.execute(&mut pq, params, &mut answers)?; // execute is on Instance, not on Prepared
owned.scan(ACCOUNT)?;
owned.contains(&fact)?;
owned.get(id)?;

let durable = Durable::create(path, Ledger)?;  // admit(∅) then bind; Violations if ¬holds
durable.read(|inst: &Instance<'_, Ledger>, w: Witness<Ledger>| {
    inst.execute(&mut pq, params, &mut answers)?;
    Ok(())
})?;
durable.write(|a: &mut Admitting<Ledger>| { a.load([&fact])?; Ok(()) })?;
Durable::from_instance(path2, &owned)?;        // encoded catalog bytes, not images
```

### 8.3 TypeScript — borrowed read is a synchronous callback

JS cannot say `'store`. The representation is a **scoped capability** for the borrow, and a **handle** for the owner. One query type: Instance. Churn `ReadScope`. **Breaks 0.14** `using snap = db.read(); snap.execute(...)`.

```ts
const admitting = Admitting.create(schema)
admitting.load(OutputFactInput, rows)                 // objects
admitting.load(OutputFactInput, columns)              // same collection, columnar transport
const owned: OwnedInstance | Violations = admitting.admit()
// owned.prepare / execute / scan / contains / get  — only on Instance

durable.read((inst, witness) => {
  const pq = inst.prepare(q)
  inst.execute(pq, params)
})  // inst is dead when the callback returns.

const owned = admitting.admit()
await something()
owned.execute(pq, params)  // owned may cross await: it is a handle, not a lease

await Durable.fromInstance(path, owned)
```

Unrepresentable: `const snap = db.read()`, `using snap = db.read(); await x; snap.execute(...)`, `db.read(async inst => { await x; inst.execute(...) })`.

Type the callback so a `Promise`-returning function is not a legal `T` (`T extends Promise<unknown> ? never : T`). An async callback is a parse failure at the SDK boundary, not a parked worker.

Columnar is transport of the same collection — one arena, one `apply_collection`, parse-all-first. Objects remain a legal transport. A second judge path is the illegal state.

Today’s `Db.execute` / `Db.prepare` open an internal snapshot so the plan escapes `'store`. That is a second algorithm. Delete it. Duration sugar that borrows an Instance inside `read` is just `Instance::execute`.

Poisoned Admitting cannot admit-as-Violations. Spent after admit, like today’s Tx latch (JS cannot consume).

Napi borrowed-read: nest the JS callback inside `Durable::read` on the calling thread (the C pattern already in [`bumbledb-c/src/db.rs`](crates/bumbledb-c/src/db.rs) lines 7–18, 64–74). Invalidate the capability when the callback returns. **Delete the snapshot worker.** `Witness` still moves as a value for `writeFrom` (018/021). PreparedQuery remains `!Sync`; execute runs on the thread that holds `&Instance`. Do not transmute `&Instance` to `'static`. The existing `PreparedQuery` transmute is schema-owner erasure under `Arc<Durable>` / `Arc<OwnedInstance>` drop-order — not a snapshot lease.

Owned Instance in napi: `External` owning `OwnedInstance` (or `Arc<OwnedInstance>`). No worker for the lease. GC, no `close()`, no `Symbol.dispose` (like today’s `Db`). That is ownership. Write may still park a worker because `Durable::write` is a duration lock — that is not an Instance lease. Parking a worker on owned Instance or ∅-Admitting teaches “this is a WriteTx of a Db that does not exist.”

### 8.4 C — same types, ABI may churn

This is not 0.14 lockstep. Today’s `bdb_snapshot_ref` is a borrowed Instance capability (`alive` flag, callback-scoped). Rename to `bdb_instance_ref`. Owned Instance is a different type: `bdb_instance *` with `bdb_instance_destroy`. Two representations because C has no lifetimes — not because Snapshot was the product. Execute is `instance_execute(view, prepared, …)` on either view. Prepare is on Instance. `Durable::from_instance` takes an Instance, copies encoded F/M/U/R/S/Q/dict.

Do not ship `owned: bool` on one pointer. Do not ship destroy *and* close. Mixing prepared queries in C is a host-boundary misuse, not a join-path `BDB_ERROR_KIND_FOREIGN_PREPARED` — do not design Rust around that code. Stashing the borrowed pointer is `BDB_STATUS_MISUSE`.

## 9. Primer

Primer depends on `@bjornpagen/bumbledb@0.14.0`. Normalization relations exist. The ledger is **not** in Bumbledb today; only Evidence IR is persisted — 3,993,828 facts, 27.61 s, 7.22 GiB RSS. Witnesses (5,081,392) live in CSR (`dense-binary-relation.ts` `offsets` / `rights`).

**The elegant path:** build columns (or objects — one collection) → `Admitting.load` → `admit` → `Instance | Violations` → query covers / keyed get → `Durable::from_instance` for duration. Exact-cover and `contained` refuse a bad partition at admit. `execute` reports what the schema does not already refuse. After admit, Instance addressing replaces today’s post-seal walks of [`relational-seal.ts`](file:///Users/bjorn/Documents/primer-spec/src/storage/relational-seal.ts).

**CSR during construction is honest.** `link()` ([`ledger.ts`](file:///Users/bjorn/Documents/primer-spec/src/stages/normalization/ledger.ts) 454–479) uses `factAddress` on already-sealed indexes and `witnesses.add` into `DenseBinaryRelationBuilder`. That is reverse adjacency **before** any Bumbledb load. Admitting cannot be queried. CSR does not vanish when Instance ships.

The elegant follow-up is to **churn `link()`**: emit witness columns during construction and `load` them, without an intermediate CSR that exists only because you cannot ask Admitting a join. Possible. Not automatic. Not a reason to put `prepare` on Admitting.

Relational-seal while *minting* refs is the same hole: an index during construction. After admit, Instance get/scan is the index.

Cardinality: `S` already has the number. `instance.row_count(rel)` is the Catalog read Primer asked for as `snapshot.count`. The full-binding `r.count()` fallback is accidental (empty aggregate ≠ zero row).

Instance without a single-arena load does not fix 7.22 GiB if Primer still materializes ~4M JS objects. The arena is transport. Persist without a JS round-trip is duration.

`reserve` is in-scope because Fresh is in the theory — Primer Evidence facts are already complete (explicit ids; high-water advances; persist.ts never calls `reserve`); ledger `inputFact`/`outputFact` are explicit `u64`, not `.fresh`. Omitting `reserve` would make Admitting a subset and invite a later dual. Escaped-id burn: RAM abort has no env to flush; the catalog is discarded. After persist, Q must be in the durable catalog.

## 10. Stress tests

Attempted breaks, not sales. Each hole is closed by a type, accepted as a language limit, or listed in §12. Substance from the four reviews, one section per area.

### 10.1 Adversarial: lifetimes

1. **`Fact<'store>` equated to Instance’s lifetime param.** Closed: methods take `&'a self` and return `Fact<'a>`. `'store` is the byte lease, never the Durable handle.
2. **`CatalogRef { Borrowed, Owned }` on Instance.** Closed: opaque `CatalogView`. Duration-boundary copy may see the concrete catalog. Exec must not.
3. **`admit() -> Instance<'static, S>` / `WriteDelta<'static>`.** Closed: `OwnedInstance<S>`; schema pointer dropped from `WriteDelta`.
4. **`witness()` / `generation()` on Instance.** Closed: `Durable::read` passes `Witness` as a separate value. Owned has no such argument.
5. **napi parked snapshot worker / `using snap` across `await`.** Closed: synchronous callback. Worker deleted. Owned handle may cross `await` because it is owned.
6. **C `alive` bit is a runtime check.** Accepted: C cannot express `'a`. The capability is the flag plus invalidate-on-drop. Not a second close verb.
7. **napi `PreparedQuery` transmute to `'static`.** Residual: existing self-ref under `Arc` drop-order. Extends to `Arc<OwnedInstance>`. Still `unsafe`. Do not use it on the mmap lease.
8. **JS host busy-waits inside a sync callback.** Duration (reader slot held), not unsoundness. Same as a long Rust `read` closure. Async callbacks are unrepresentable.
9. **Two `Durable::read` closures on two threads.** Two leases, two views, possibly two generations. Writer is single. `PreparedQuery` still `!Sync`. Unchanged. Owned: many `&OwnedInstance` / `Instance<'_>` borrows, all the same catalog.
10. **RamCatalog intern after freeze.** Closed: intern is `CatalogMut`. Freeze is a type change. Residual: pin / shrink-to-fit once at freeze so no realloc under `'store`. No `'store` borrows exist during admit (Instance does not exist yet).
11. **`Admitting::from(&Instance)` as a self-borrow.** Open (§12). v1 is ∅-base. If it lands, it is a copy, not `Instance<'static>`.
12. **Empty `Durable::create` vs `holds`.** Closed: `admit` is the only Instance constructor. `Durable::create` is `admit(∅)` then bind. Unadmitted empty is unrepresentable.
13. **`from_instance` during a durable read.** Allowed: the `ReadTxn` is live. Whether v1 takes only `&OwnedInstance` is a persist-lane choice (§12).
14. **Upgrading `OnceCell` to `OnceLock` to claim `Instance: Sync`.** Refused: that makes concurrent RoTxn use representable.
15. **Parked readers + CommitSeq as Instance identity.** Closed: parked reuse is an optimization on `Durable::read`. CommitSeq never leaves Durable. CommitSeq and GenerationId stay incomparable. Commit drops the parked reader so the writer is not blocked by a pinned old generation.

### 10.2 Adversarial: catalog / admit

1. **Empty `Durable::create` that does not `holds`.** An unadmitted empty catalog is not `State T`. Documenting “create is not admit” is a flag. Closed: `admit` is the only constructor. If ¬`holds T ∅`, `Violations` — no lease.
2. **`writable: bool`.** Freeze is `CatalogMut` → `Catalog`. Instance cannot intern. Admitting cannot execute.
3. **RAM vs LMDB as two engines.** Insight 16: two lifetimes of one Catalog. Fsync / `CommitSync` / `commit_bounded` / writer mutex live on `Durable`. Heap admit omits phase 5. Property: heap `admit(Δ)` vs Durable `admit(Δ)` → identical `Violations` or identical encoded F/M/U/R/S/Q/dict. Not vs ephemeral.
4. **Catalog of four methods.** Apply needs `range_mut`+`del_current`, `put_no_overwrite`, neighbor probes. Judgment needs `SortedGets`, coverage `gte`/`lower_than`, R-prefix range, mmap bytes borrowed into a `BTreeSet`. Image build needs `entry_count`, not `mdb_stat`. Close: `OrderedKv`. Generic on `Applier`/`Checker`. No `dyn` on neighbor probes or `SortedGets`. Restate “LMDB key uniqueness” as encoded-key uniqueness. `dyn` is fine for `verify_store` (offline).
5. **Freeze then judge.** Judge needs write-txn read-your-writes. Freeze first would require a second txn or a flag. Closed: judge on `CatalogMut` as `Catalog`, then freeze.
6. **Empty-delta shortcut on ∅-base.** Today’s no-op commit skips judge because the base already `holds`. Empty `admit` of ∅ is `judge(∅)` plus `Q` flush. `Durable::create` is that admit.
7. **`commit_bounded` around heap admit.** Retry would double-insert. Heap apply uses a scratch `CatalogMut`; abort/panic discards it. `commit_bounded` is Durable phase 5 only.
8. **Phase 4 after freeze.** `image::build` sizes from `S`. `Q` must be in the catalog before `from_instance`. Closed: flush `S`/`Q`/dict before freeze, on heap and mmap.
9. **`WriteDelta` / plan still take `ReadTxn`.** Residual until retarget. Design: `Catalog`.
10. **Prepare inside a write.** Closed: Admitting is a different type. Overlay gets stay. If Primer wants to judge a staging set, it `admit`s, then queries the Instance.
11. **Poison / parse-then-apply.** Unify on parse-all-first for `Admitting::load` and the shared collection arena. Typed generated-struct rows are already well-typed per row — poison there is storage failure, not shape. Do not weaken.
12. **Closed `scan_f`.** Closed branches before Catalog. `synthesize_closed` is theory → image. Catalog is instance facts, not axioms.
13. **Empty Instance (no facts) is lawful — iff `holds`.** `functionality_of_empty`. Do not special-case “no facts” in exec. An empty catalog that does not `holds` is not an Instance.

Residuals (not exceptions): `OrderedKv::get` GAT (mmap vs heap `&[u8]`); `SortedGets` reusable cursor vs one-shot iterator; `from_instance` / create generation at birth (0 vs 1 — not load-bearing); open of *pre-proposal* empty files that would fail `holds` (migration).

### 10.3 Adversarial: durable / persist

1. **“The images *are* the instance, persist those.”** Images have no row ids. Closed: `Durable::from_instance` copies encoded F/M/U/R/S/Q/dict. Compact is live-page copy of one Durable — a different sentence.
2. **Persist as CatalogMut apply of judged entries.** Re-mints row ids, re-interns, resets `Q`. Refused.
3. **Persist as `bulk_load`.** Retired `bulk_load` re-inserted facts, chunked, prefix-committed. Persist-by-catalog-bytes is not that dual. The dual returns only if `from_instance` takes a fact iterator, re-judges, or CatalogMut-applies judged entries.
4. **`Durable::create` as a persist-lane hedge around an unadmitted empty.** Closed: create is `admit(∅)` then bind (§5).

### 10.4 Adversarial: execute

1. **“The join kernel never sees LMDB.”** `exec/run.rs` is already images-only. `run_join` is not: it takes `ReadTxn` for `cache.get_or_build` and `txn.generation()?`. `PreparedQuery::execute` also takes that txn for `check_snapshot`, bind, key-probe, finalize. Today’s `Snapshot::execute` is a second site. Close: `Instance::execute` is the one site; see the pass list in §7.1.
2. **`env_instance` + `ForeignPreparedQuery`.** The integer exists because the plan did not borrow the Instance. `InstanceId` on the plan is the same flag. Dummy `0` would collapse every owned Instance. Bind `PreparedQuery<'store, S>`. Mixing does not compile. Delete the engine error. FFI/JS residual is host-boundary, not a join-path branch. Pin-at-prepare across generations dies with that bind.
3. **Dummy `GenerationId(0)` so `ImageCache` can stay.** `GenerationId::initial()` is already 0. Stuffing owned images into the cache at 0 aliases a never-written Durable. Owned ordinary images are `PinnedImages` keyed by `RelationId` only. Cache stays on `Durable`. `advance` is a writer hook. Owned has no writer. Parked view filters still rotate on `(Closed|Edb, filters)` LRU. Stale-generation reaping never fires: the plan cannot outlive the Instance.
4. **Key-probe as an image probe.** Images have no `U`/`M`, no row ids. Tests: key-probe must not build an image. The dual is algebra (unique-key lookup vs conjunctive join). The accidental dual was `ReadTxn`. After Catalog, key-probe is a point get. Join is images. Neither is LMDB.
5. **Staleness on owned as `1.0`.** A method that always returns never-stale is `if ram`. After `'store`-bound plans, borrowed Instance has none either. Delete `PreparedQuery::staleness`.
6. **Rec/reach transients in the cache.** Derived images are `'exec` ⊂ `'store`. Caching them under dummy generation, or pinning them on Instance, is a second store. `run_join` Interior arm is the miss path over a driver-supplied `Arc`.
7. **Generation matching inside COLT / `run_join`.** Leftover affine branch. Generation stays inside `Instance::image` (the cache), not the join kernel. PreparedQuery borrowing `'store` removes generation from the memo entirely. A lifetime `'store` says “these bytes.” It does not name which of several coexisting MVCC versions this is. Collapsing those two questions is the mistake.

Residuals: Catalog needs `lookup` (bind / `PendingIntern`), not only `resolve`. `image::build` uses `read::data_entries` (`mdb_stat` of `_data`) as an allocation ceiling ([`build.rs`](crates/bumbledb/src/image/build.rs) lines 209–224) — keep it behind the LMDB impl, not in `exec/`. Closed `OnceLock`s today live on `ImageCache`; owned pins may duplicate theory-constant images (honest) or Schema grows the slots (optimization). `prepare` peeks the cache and never builds; owned lazy pins mean first prepare is cold, same as cold LMDB. Eager vs lazy at admit is measured, not semantic.

### 10.5 Adversarial: host

1. **Napi worker vs Instance lifetime.** C already nests the host callback inside `read` on the calling thread. TS does the same. The 0.14 worker is the `'static` lie in costume (findings 018/021). Write may still park a worker because `Durable::write` is a duration lock.
2. **ReadScope vs Instance.** Two query types is an illegal state. `ReadScope.generation` was the affine exception on the engine type. Generation is `Witness` / Durable cache metadata. The TS read API is the callback, not a handle.
3. **Execute-on-prepared vs execute-on-Instance.** Prepared is a pinned plan (`!Sync` scratch). Public execute is `instance.execute(prepared, params)`. Snapshot as a second execute site is a dual — delete it. `Prepared.execute` would make the plan look like the store.
4. **C without `'a`.** Essential: capability vs owned handle. Accidental: the names `snapshot` / ABI 2 / lockstep with 0.14. Churn the names. Keep the ownership split.
5. **`ok: bool`.** `{ ok: true, generation } | { ok: false, violations }` is validation. `Instance | Violations` is parse.
6. **Columnar as a second algebra.** Six representations per insert today (JS rows → `Vec<Value>` → parse → encode) is accidental transport. Columns and objects are one collection. Two `judge` paths are essential complexity you must not invent.
7. **`Admitting.load` without a Durable.** Insert without duration. Same poison, closed, collection, overlay. `reserve` stays because Fresh is in the theory.

### 10.6 Refused representations

Not compatibility hedges.

- `Db.memory()` / TS `ephemeral` as the product.
- A second validator or “lite” judgment. One `judge`, one `Violations`.
- Promote `NaiveDb`. Scalar `load(fact)` / scalar insert.
- Query / `prepare` / `execute` / `scan` on Admitting. Write / `load` on Instance.
- `writable: bool` on Catalog. Empty `Durable::create` that does not `holds`. A pre-state.
- Persist by round-tripping JS objects, re-inserting facts, CatalogMut-applying judged entries, or rebuilding from `RelationImage`.
- `enum Store { Lmdb, Memory }` in exec / plan / judgment / image build.
- A 4-method `ReadInstance` that cannot admit or key-probe.
- `Box::leak` to obtain `'static` Instance. Dummy `GenerationId(0)` on owned Instance.
- `dyn Catalog` on the COLT / `run.rs` hot path. Treat `CommitSeq` as generation.
- `{ ok: bool }` as the admit result. `owned: bool` as the C Instance.
- Two query types (`ReadScope` and `Instance`) for the same verbs.
- Park a napi worker inside owned Instance / ∅-Admitting “for consistency.”
- Type-alias `Snapshot = Instance` “until bindings catch up.”
- `env_instance: u64`; dummy `InstanceId(0)` meaning RAM; `InstanceId` on the plan; engine `ForeignPreparedQuery`; `BDB_ERROR_KIND_FOREIGN_PREPARED` on the join path.
- `PreparedQuery::staleness`. `Snapshot::execute` as a second execute site; `PreparedQuery::execute(txn, cache)`.
- View memo keyed on `GenerationId` after the plan cannot outlive `'store`.
- Key-probe synthesized from an image scan. Rec/reach transients in `ImageCache` or `PinnedImages`.
- `Instance::generation` / `Instance::witness`.
- `using snap = db.read(); await x; snap.execute(...)`.
- Upgrade `ReadTxn`’s `OnceCell` to `OnceLock` to claim `Instance: Sync`.

## 11. Implementation sequence

Reveal the engine type hiding inside today’s `Snapshot`, then mint it without a path, then churn hosts onto it. No alias.

1. **Reveal.** Rename `Snapshot` → `Instance`. No alias. The view does not own the `ReadTxn`. `generation` / `witness` move onto `Durable::read`’s second argument. Pin `Fact<'a>` as `&self`, not `'store` = the Durable handle.
2. **Catalog behind the lifetime.** Retarget applier, judgment, image build, key-probe, dict resolve, verify_store. `run_join` takes `Arc<RelationImage>` from `Instance::image`, not `ReadTxn`, not a generation stamp. Heed leaves `exec/` and `judgment.rs`.
3. **Admitting.** Collapse today’s `WriteTx` and `InstanceBuilder`. Freeze at successful `admit` is the type change. Still LMDB-only at this step.
4. **RamCatalog / OrderedKv.** Same encoded keys. Generic `OrderedKv` on apply and Checker. Property: heap `admit(Δ)` vs Durable `admit(Δ)` → identical `Violations` or identical F/M/U/R/S/Q/dict. Not vs ephemeral. Empty Δ on ∅ judges. Heap apply uses a scratch `CatalogMut`; `commit_bounded` does not wrap heap admit.
5. **∅-Admitting / `OwnedInstance`.** `admit` is the only Instance constructor: parses to `Instance | Violations`. Pin or lazy images. `prepare` / `execute` without `ReadTxn`. `PreparedQuery` borrows `'store`. Execute is `Instance::execute`. No staleness. No `Durable::prepare` as a second algorithm. `Durable::create` is this empty admit, then bind.
6. **One collection arena** in napi, shared with `Admitting::load`. Objects and columns. Parse-all-first. Primer Evidence RSS.
7. **`Durable::from_instance`.** Encoded F/M/U/R/S/Q/dict, one LMDB txn, no re-judgment, not images, not CatalogMut apply. Instance gains a durable lifetime.
8. **Host churn.** TS: Instance is the query type; scoped capability for `'a`; `Admitting`; `Instance | Violations`. Break 0.14 `snap.execute`. C: same types, ABI may churn (`bdb_snapshot_ref` → `bdb_instance_ref`). Cookbook: Primer-shaped load → admit → cover.
9. **Crate split** only if heed is gone from `exec/` / `judgment.rs` and a second Catalog impl is real.

Steps 1–3 reveal. 4–5 add a constructor, not a mode. 6–8 churn hosts onto the types.

## 12. Open questions (essential only)

Insight 16: accidental special cases are closed above. What remains cannot be renamed away. Empty create vs `holds`, who owns the RAM catalog, Snapshot-as-alias, and ephemeral-as-product are **not** open.

1. **`OrderedKv::get` GAT.** Mmap vs heap `&[u8]` without a lifetime lie. Shapes the trait every admit path is generic over.
2. **`SortedGets`.** Reusable cursor vs one-shot iterator. Shapes `Checker` on `OrderedKv`.
3. **`Durable::from_instance` v1 input.** `&OwnedInstance` only, or also `&Instance` borrowed from a live `Durable::read` (the `ReadTxn` is live; this is a persist-lane choice, not a lifetime one).
4. **`Admitting::from(&Instance)`.** A catalog copy with a nonempty base and no path. Not a self-borrow, not `Instance<'static>`. v1 is ∅-base; Primer does not need this.

---

The previous sketch treated this as “extract a trait, add a RAM backend.” That is how you get `if lmdb`. The engine is already Instance, hiding inside today’s Snapshot, leased by ReadTxn, proved by `Fact<'a>`. LMDB is how some Instances outlive the process. Reveal that, then mint the same object without a path. Hosts churn until they show those types.
