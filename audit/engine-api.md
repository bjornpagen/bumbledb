# Engine public API and mutation algebra — pre-publish audit (0.13)

Lens: data representation determines complexity; branches that guard illegal states belong in the type; special cases belong to the representation (half-open ranges, one algebra, parse-don't-validate). Compared committed HEAD (scalar `insert`/`delete`/`alloc`, `Db::bulk_load`, `Error::BulkLoad`) to the working tree (collection-valued insert/delete, `reserve` → `FreshRange`, `MutationReport`, `Db::write` as the only commit, parse-then-apply poison).

No `insertMany` / `InsertBatch` names remain in the engine. Scalar overloads are gone. `Error::BulkLoad` / `BulkLoadError` are gone from the type. The leftovers are representation, not naming.

---

### API-1 (severity: ship-blocker)

- **Location:** `crates/bumbledb/src/api/db/mutation.rs` — `FreshRange`, `FreshRange::empty`, `FreshRange::start`, `FreshRange::start_raw`, `FreshRange::get`; `crates/bumbledb/src/api/db/alloc.rs` — `WriteTx::reserve` / `reserve_at` (`count == 0` → `[0, 0)`).
- **Illegal state / extra branch / special case:** Empty range is encoded as the sentinel `[0, 0)`. `start()` / `start_raw()` still return `T` / `u64`. `0` is also the first legal minted id. `get(0)` on empty returns `None`; `start()` on empty returns `T::from_fresh(0)`. Two answers for “first id,” and the typed one fabricates a minted newtype. Callers are told to check `is_empty()` first (validate, then use).
- **Why the representation is wrong:** Half-open ranges and sentinels. `Interval<T>` in the same crate makes empty unrepresentable (`start < end` is the constructor). Fresh ids reused the C/TS wire encoding (`[0, 0)`) as the Rust type, so emptiness is a runtime flag on a pair of `u64`s that already mean “minted bounds.” Null: `0` means both “no ids” and “the first id.” Parse-don't-validate: `start() -> T` throws away the proof that the range was nonempty.
- **Better representation:** An enum, not a sentinel pair:

  ```rust
  pub enum FreshRange<T> {
      Empty,
      NonEmpty { start: T, count: NonZeroU64 },
  }
  ```

  `start` / `iter` / `get` live only on `NonEmpty` (or return `Option<T>`). Empty does not mention `0`. FFI maps `Empty` → `{0, 0}` at the C/TS boundary; that encoding is not the engine type. `reserve(0)` returns `FreshRange::Empty` without a `count == 0` branch that builds a fake interval.
- **Evidence:**

```26:104:crates/bumbledb/src/api/db/mutation.rs
/// Half-open fresh-id range `[start, end_exclusive)` from one `reserve`.
/// `len()` is `end_exclusive - start`. The empty range is `[0, 0)` and
/// does not read or advance the sequence.
pub struct FreshRange<T> {
    start: u64,
    end_exclusive: u64,
    marker: PhantomData<fn() -> T>,
}
// ...
    pub const fn start_raw(self) -> u64 {
        self.start
    }
    /// Inclusive start, wrapped as `T`. Not a minted id when empty.
    pub fn start(self) -> T {
        T::from_fresh(self.start)
    }
    /// The id at `index`, or `None` if `index >= len`.
    pub fn get(self, index: u64) -> Option<T> {
        match self.start.checked_add(index) {
            Some(raw) if raw < self.end_exclusive => Some(T::from_fresh(raw)),
            _ => None,
        }
    }
```

```46:50:crates/bumbledb/src/api/db/alloc.rs
        self.refuse_poisoned()?;
        if count == 0 {
            return Ok(FreshRange::empty());
        }
```

Contrast: `Interval::new` returns `Option`; empty is unrepresentable (`crates/bumbledb-theory/src/interval.rs`).

---

### API-2 (severity: ship-blocker)

- **Location:** `crates/bumbledb/src/error.rs` — `Error` module law, `Error::TransactionPoisoned`; `crates/bumbledb/src/api/db.rs` — `WriteTx::{applied, poisoned}`, `poison`, `refuse_poisoned`; `crates/bumbledb/src/api/db/write.rs` — `Db::write` Ok-after-catch inspect.
- **Illegal state / extra branch / special case:** Poison is two flags: `applied: bool` + `poisoned: Option<String>` (four states; `applied == false && poisoned.is_some()` is illegal and only avoided by `poison()`’s `if self.applied`). The public error is a **Display string**. Every later mutation clones that string. `Db::write` on `Ok` after a caught apply failure re-wraps the string as `TransactionPoisoned`, so the original typed error is gone. Hosts branch `TransactionPoisoned` vs `Lmdb`/`FactShape`/… and cannot match the cause. HEAD’s `Error::BulkLoad { committed, error: Box<Error> }` nested the typed cause; this replaces that with a string.
- **Why the representation is wrong:** Illegal states (boolean flags). Parse-don't-validate (stringify, then the host re-parses Display). The crate’s own taxonomy: “Payloads carry ids and owned fact bytes, never formatted strings.” `TransactionPoisoned { message: String }` is the one write error that violates that law. Control flow (`refuse_poisoned` on every entry, `write` inspecting poison even on `Ok`) is downstream of a missing typestate.
- **Better representation:** One apply state, original error preserved:

  ```rust
  enum ApplyState {
      Virgin,
      Applied,
      Poisoned(Arc<Error>),
  }
  enum Error {
      // ...
      TransactionPoisoned { cause: Arc<Error> },
  }
  ```

  Shape failure stays `Virgin` (no poison). First apply failure after `Applied` returns `*cause` (the original) and stores `Poisoned`. Later mutation returns `TransactionPoisoned { cause }`. `Db::write` on `Ok` + `Poisoned` returns `TransactionPoisoned` carrying the same `Arc`. No string. After 0.13 this field layout is a breaking change.
- **Evidence:**

```1:7:crates/bumbledb/src/error.rs
//! Payloads carry
//! ids and owned fact bytes, never formatted strings — no `format!` runs on
//! a hot path; `Display` formats lazily when the host actually prints.
```

```467:490:crates/bumbledb/src/api/db.rs
    /// Set after any fact has entered the delta via insert/delete.
    applied: bool,
    /// Display of the first apply failure after a prefix entered.
    poisoned: Option<String>,
    // ...
    fn poison(&mut self, err: crate::error::Error) -> crate::error::Error {
        if self.applied {
            self.poisoned.get_or_insert_with(|| err.to_string());
        }
        err
    }
```

```1351:1358:crates/bumbledb/src/error.rs
    TransactionPoisoned {
        /// Display of the original apply error.
        message: String,
    },
```

```291:300:crates/bumbledb/src/api/db/write.rs
            Ok(value) => {
                if let Some(message) = burn.tx().poisoned.clone() {
                    let WriteTx { view, delta, .. } = burn.disarm();
                    drop(view);
                    let error = Error::TransactionPoisoned { message };
```

---

### API-3 (severity: ship-blocker)

- **Location:** Public contract still names the removed API: `docs/architecture/70-api.md` (cited by `lib.rs` as the embedding surface), `docs/architecture/10-data-model.md`, `docs/cookbook.md`; public rustdoc `Snapshot::scan_facts` (`crates/bumbledb/src/api/db/snapshot.rs`). Working-tree engine: no `Db::bulk_load`, no `WriteTx::alloc`, no `Error::BulkLoad`, no `BulkLoadError` export.
- **Illegal state / extra branch / special case:** The published algebra is still the HEAD dual: scalar `insert(&fact) -> bool`, `alloc() -> T`, `Db::bulk_load` chunking, `Error::BulkLoad { committed, error }`. Hosts following the normative docs will not compile. `scan_facts` rustdoc links `[`Db::bulk_load`]` — a removed item.
- **Why the representation is wrong:** Accidental complexity left in the contract after the type changed. One algebra (collection insert + `Db::write`) is not one algebra if the document that `lib.rs` defers to still teaches two (write-tx insert vs `Db::bulk_load`; `alloc` vs `reserve`). Shipping 0.13.0 with that document is shipping the wrong API.
- **Better representation:** The docs are the other face of the type. Rewrite 70-api / 10-data-model / cookbook / `scan_facts` to `insert`/`delete` collections, `reserve`/`FreshRange`, `MutationReport`, `insert_dyn` under `Db::write`. Delete every `bulk_load` / `alloc` / `BulkLoad` spelling from the embedding contract. (In-crate `pub(crate)` comments in `storage/` are not API; they are the same leftover.)
- **Evidence:**

```531:540:docs/architecture/70-api.md
  Write operations: typed `alloc::<NewType>()` via the generated `Fresh` newtypes
  (untyped: `alloc_at(FreshField<S>) -> u64`, ...
  `insert(&fact) -> bool` (changed-state
  report); `delete(&fact) -> bool`; `_dyn` forms of both for ETL tooling.
  ...
  Bulk import is `Db::bulk_load` (typed) /
  `Db::bulk_load_dyn` (the ETL/FFI lane)
```

```853:862:docs/architecture/70-api.md
`Db::bulk_load(facts)` takes an iterator of **generated fact structs** ...
conversion into the workspace error lands in `Error::BulkLoad { committed, error }`,
```

```290:305:docs/architecture/10-data-model.md
      let id: AccountId = tx.alloc()?;             // mints the next AccountId value
      tx.insert(&Account { id, holder, status })?; // insert always takes complete facts
  ...
  `alloc` is the only generator; `insert` is always full-fact
```

```348:351:crates/bumbledb/src/api/db/snapshot.rs
    /// The typed sibling of [`Snapshot::scan`]: ...
    /// remains the ETL pairing for [`Db::bulk_load`]; this one is for
    /// hosts that want their own types back.
```

`lib.rs` crate rustdoc and re-exports already describe `reserve` / `insert([&fact])` / `insert_dyn`. The architecture docs and `scan_facts` did not move with the types.

---

### API-4 (severity: should-fix-before-0.13)

- **Location:** `FreshRange::end_exclusive` / `end_exclusive_raw` (`mutation.rs`); `reserve_at` → `FreshRange<u64>` (`alloc.rs`).
- **Illegal state / extra branch / special case:** Exclusive end is not a minted id. `end_exclusive()` wraps it as `T` anyway. For `[start, start+count)`, `T::from_fresh(end_exclusive)` is the next *unissued* id, typed as if it were reserved. `start()` / `start_raw()` / `end_exclusive()` / `end_exclusive_raw()` are four getters for two numbers. `reserve` returns `FreshRange<T: Fresh>` (`start()` available); `reserve_at` returns `FreshRange<u64>` (`u64: Fresh` does not hold, so only `*_raw()`). One struct, two APIs, because the phantom is sometimes a newtype and sometimes a raw word.
- **Why the representation is wrong:** Half-open: the exclusive bound is not an element of the range; giving it type `T` says it is. Dual `start()` / `start_raw()` discards the newtype proof on the typed path (`HolderId` → `u64`) and is the only path on the dyn range. Special case belongs to the representation: dyn vs typed should be `FreshRange<T>` vs a raw range type (or a `FreshId` trait both implement), not one struct with a dead `start()` on `u64`.
- **Better representation:** Do not wrap the exclusive bound as `T`. Iterate / `get` / `len`. If FFI needs `end_exclusive: u64`, convert at the boundary. Split `FreshRange<T: Fresh>` (typed ids) from `RawRange { start, count }` for `reserve_at`. One method name `start` → `Option<T>` (or only on `NonEmpty`, API-1).
- **Evidence:**

```84:95:crates/bumbledb/src/api/db/mutation.rs
    /// Inclusive start, wrapped as `T`. Not a minted id when empty.
    pub fn start(self) -> T { T::from_fresh(self.start) }
    /// Exclusive end, wrapped as `T`. Not a minted id.
    pub fn end_exclusive(self) -> T { T::from_fresh(self.end_exclusive) }
```

```81:81:crates/bumbledb/src/api/db/alloc.rs
    pub fn reserve_at(&mut self, field: FreshField<S>, count: u64) -> Result<FreshRange<u64>>
```

Removing `start_raw` after 0.13 is a breaking change on every typed caller that copied the dyn spelling.

---

### API-5 (severity: should-fix-before-0.13)

- **Location:** `MutationReport` (`mutation.rs`); return type of `insert` / `delete` / `insert_dyn` / `delete_dyn`.
- **Illegal state / extra branch / special case:** Two public independent `u64`s. `{ submitted: 0, changed: 3 }` and `{ submitted: 1, changed: 2 }` are constructible. The engine never reads the struct back, but the type is the public report. `EMPTY` is a named special case of `0, 0` — the algebra already unified empty as length-0; the const is fine, the unconstrained pair is not.
- **Why the representation is wrong:** Illegal states. The invariant is `changed <= submitted`. Independent fields make that a comment. Length-1 `{ submitted: 1, changed: 0|1 }` replaced `bool`; the pair must not be wider than that algebra.
- **Better representation:** Private fields, a single constructor `fn report(submitted: u64, changed: u64) -> Self` that debug-asserts (engine) / saturates (never from hosts). Or `struct MutationReport { submitted: u64, changed: u64 }` with `changed` a `u64` only via `from_counts` that requires `changed <= submitted`. Do not `#[derive(Default)]` as a second path around the constructor (or make `Default` call `EMPTY`).
- **Evidence:**

```10:23:crates/bumbledb/src/api/db/mutation.rs
pub struct MutationReport {
    pub submitted: u64,
    pub changed: u64,
}
impl MutationReport {
    pub const EMPTY: Self = Self { submitted: 0, changed: 0 };
}
```

C `bdb_mutation_report` and TS `MutationReport` copy the unconstrained pair; fix the engine type first.

---

### API-6 (severity: should-fix-before-0.13)

- **Location:** `WriteTx::insert` / `delete` (`insert.rs`, `delete.rs`); `insert_dyn` / `delete_dyn` (`Row, I`). Documented turbofish: `tx.insert::<Holder, _>([])`.
- **Illegal state / extra branch / special case:** Empty is lawful and carries no `F`, and `MutationReport` does not mention `F`, so inference fails. Named type parameter `I` forces a *second* turbofish hole. This is HEAD `bulk_load<'f, F, I>`’s signature moved onto `insert` after `bulk_load` died. `insert_dyn` repeats `Row, I`.
- **Why the representation is wrong:** Inference pain from extra type parameters is a representation smell: the collection type is not part of the algebra, and empty does not parse to a relation. Accidental complexity leftover of `BulkLoad`. Parametricity: `I` is not used except as `IntoIterator`; naming it makes callers name it.
- **Better representation:** `fn insert<'f, F: Fact<'f, Schema = S> + 'f>(&mut self, facts: impl IntoIterator<Item = &'f F>)` — empty is `tx.insert::<Holder>([])`. Same for delete / dyn (`impl IntoIterator<Item = impl AsRef<[Value]>>` so empty is `insert_dyn(rel, [])` with `rel` already naming the relation). Do not add a dummy `F` on `MutationReport` just to infer; the iterator item is the parse.
- **Evidence:**

```10:21:crates/bumbledb/src/api/db/insert.rs
    /// An empty typed collection cannot infer `F`: `tx.insert::<Holder, _>([])`.
    pub fn insert<'f, F, I>(&mut self, facts: I) -> Result<MutationReport>
    where
        F: Fact<'f, Schema = S> + 'f,
        I: IntoIterator<Item = &'f F>,
```

Git diff: HEAD `bulk_load<'f, F, I>(..., I: IntoIterator<Item = F>)` is the source of `I`. After 0.13, `insert::<Holder, _>` is frozen into host code.

---

### API-7 (severity: should-fix-before-0.13)

- **Location:** `InternMode::Mint` (`api/db.rs`); `WriteTx::encode_dyn` (`encode_dyn.rs`); `parse_dyn_row` vs `dyn_value_refs` (`insert_dyn.rs`, `delete_dyn.rs`, `encode_dyn.rs`); `contains_dyn` still calls `encode_dyn(..., InternMode::Resolve)`.
- **Illegal state / extra branch / special case:** Parse-then-apply split `insert_dyn` off `encode_dyn`, then left `InternMode::Mint` as a `#[expect(dead_code)]` arm “kept as the encode_dyn mint switch.” Two dynamic encode algebras: (1) `parse_dyn_row` then `dyn_value_refs` (insert/delete), (2) `encode_dyn` + `InternMode` (contains). `parse_dyn_row` runs `value_matches_parsing` and **discards** `Ok(Some(&str))`; apply calls `value_matches_parsing` again.
- **Why the representation is wrong:** Dead enum arm is an illegal state of the mint/resolve sum (only `Resolve` is live). Validation that discards proof: the parse pass throws away UTF-8/`&str` so apply re-checks. Control flow (`Mint` vs `Resolve` match, double walk) is downstream of not having a parsed row type. One algebra, not two.
- **Better representation:** Delete `InternMode`. `encode_dyn` only resolves (point reads). Insert/delete parse once into a `ParsedDynRow` (arity + per-field refs or intern-ready `&str`), then apply consumes that proof (intern + `delta.insert`). No second `value_matches_parsing`. No `Mint` arm.
- **Evidence:**

```525:536:crates/bumbledb/src/api/db.rs
enum InternMode {
    /// Insert interned strings into the pending dictionary. `insert_dyn`
    /// mints through `intern_str` after parse-then-apply rather than
    /// constructing this arm; keep it as the encode_dyn mint switch.
    #[expect(dead_code, reason = "insert_dyn mints after parse-then-apply")]
    Mint,
    Resolve,
}
```

```96:120:crates/bumbledb/src/api/db/encode_dyn.rs
/// Shape-check a dynamic row without interning: arity and type-kind only.
/// Apply (intern + encode) happens after every row in the collection parses.
pub(super) fn parse_dyn_row(...) -> Result<()> {
    // value_matches_parsing — result dropped except Err
}
```

```39:51:crates/bumbledb/src/api/db/insert_dyn.rs
        for row in &rows {
            parse_dyn_row(rel, row.as_ref(), fields)?;
        }
        // ... then dyn_value_refs → value_matches_parsing again, then intern
```

---

### API-8 (severity: should-fix-before-0.13)

- **Location:** `insert` / `delete` / `reserve` / `reserve_at` — `refuse_poisoned()?` before the empty short-circuit.
- **Illegal state / extra branch / special case:** Empty collection is documented as “no engine request” / `count == 0` does not read Q. Poison is documented as “later **mutation**.” Empty still hits `refuse_poisoned` and returns `TransactionPoisoned` instead of `EMPTY` / `FreshRange::empty()`.
- **Why the representation is wrong:** The algebra unified empty as length-0 (not a write). A poison branch then re-special-cases empty as a mutation. If empty is not a request, it is not a write; if poison is typestate on the transaction object, empty should not be a method on a live `WriteTx` either — pick one representation. Mixing them is two algebras.
- **Better representation:** With API-2’s `ApplyState`, either (a) empty short-circuits *before* poison (empty is not a write — matches the collection law), or (b) `WriteTx` becomes a typestate where a poisoned tx is a different type and no methods including empty exist. Do not check poison then pretend empty is lawful.
- **Evidence:**

```23:27:crates/bumbledb/src/api/db/insert.rs
        self.refuse_poisoned()?;
        let mut iter = facts.into_iter();
        let Some(first) = iter.next() else {
            return Ok(MutationReport::EMPTY);
        };
```

```47:50:crates/bumbledb/src/api/db/alloc.rs
        self.refuse_poisoned()?;
        if count == 0 {
            return Ok(FreshRange::empty());
        }
```

---

### API-9 (severity: later)

- **Location:** `Fact::encode_delete` / `encode_read` / `Key::determinant_read` / `determinant_write` (`api/db.rs`); `WriteTx::get_dyn_into` (`get.rs`).
- **Illegal state / extra branch / special case:** Public `Result<bool>` where `Ok(false)` means “cannot exist / miss” and `Err` means infrastructure. `false` is a sentinel for absence. `get_dyn` already returns `Result<Option<Vec<Value>>>`; `get_dyn_into` reintroduces the bool (`Ok(true)` hit, `Ok(false)` miss, `out` empty).
- **Why the representation is wrong:** Parse-don't-validate / null: a bool that means “encoded” vs “absent” is an `Option` wearing a bool. Callers re-branch on `false` after `?`. Pre-existing; not introduced by 0.13, but 0.13 made `insert`/`delete` return `MutationReport` while encode/contains still speak bool.
- **Better representation:** `Result<Option<()>>` or `enum Encode { Absent, Written }` for encode; keep `get_dyn_into` as `Result<Option<()>>` (hit vs miss) so `out` is only meaningful on hit. `contains` can stay `Result<bool>` (membership *is* a bool) once encode is `Option`.
- **Evidence:**

```115:125:crates/bumbledb/src/api/db.rs
    /// `Ok(false)` means a
    /// string or bytes value is known to neither: the fact provably
    /// cannot exist in base or delta, the delete is a no-op
    fn encode_delete(&self, tx: &WriteTx<'_, Self::Schema>, out: &mut Vec<u8>) -> Result<bool>;
```

```314:325:crates/bumbledb/src/api/db/get.rs
    /// `Ok(true)` = hit, `out` holds the fact's fields ...
    /// `Ok(false)` = no fact, `out` empty.
    pub fn get_dyn_into(...) -> Result<bool>
```

---

### API-10 (severity: later)

- **Location:** `FunctionalityViolation::incumbent` (`error.rs`).
- **Illegal state / extra branch / special case:** The enum already has `Scalar` vs `Pointwise { incumbent }`. `incumbent()` returns `Option<&[u8]>` — `None` means scalar *or* you forgot which arm you had. Two shapes, then an accessor that re-encodes the discriminant as null.
- **Why the representation is wrong:** Option that means two things (no incumbent because scalar vs missing). Match the enum; do not project it back to `Option`.
- **Better representation:** Delete `incumbent()`. Callers match `FunctionalityViolation`. If a bindings layer needs a slice, it matches once at the boundary.
- **Evidence:**

```1002:1008:crates/bumbledb/src/error.rs
    pub fn incumbent(&self) -> Option<&[u8]> {
        match self {
            Self::Scalar { .. } => None,
            Self::Pointwise { incumbent, .. } => Some(incumbent),
        }
    }
```

---

### API-11 (severity: later)

- **Location:** `Violations::cited_facts` (`error.rs`).
- **Illegal state / extra branch / special case:** `&[]` means (1) `Citations` — never decorated, (2) out-of-range index, (3) a decorated citation that happened to cite zero facts. Hosts cannot tell “sweeper replay” from “no facts for this citation” from “bad index” without matching the enum *and* checking length.
- **Why the representation is wrong:** Null / sentinel empty slice. Validation that discards proof: decoration is a `Violations` arm, then `cited_facts` throws the arm away.
- **Better representation:** `cited_facts` → `Option<&[CitedFact]>` (`None` = not decorated). Out-of-range is `None` or panics (programmer index). Make `Decorated` the only public commit-rejection form; keep `Citations` `pub(crate)` for the sweeper.
- **Evidence:**

```1150:1159:crates/bumbledb/src/error.rs
    /// Empty for a set no decode pass decorated (the sweeper's re-play
    /// findings) and for an out-of-range index.
    pub fn cited_facts(&self, index: usize) -> &[CitedFact] {
        match self {
            Self::Citations(_) => &[],
            Self::Decorated { cited, .. } => cited.get(index).map_or(&[], AsRef::as_ref),
        }
    }
```

---

### API-12 (severity: later)

- **Location:** `WriteTx::contains` (`get.rs`); `Db::write_from` rustdoc (`write.rs`) vs `Snapshot::generation` (`api/db.rs`). Module file `api/db/alloc.rs` still named `alloc` after `reserve`.
- **Illegal state / extra branch / special case:** `contains` remains scalar `Result<bool>` while insert/delete speak `MutationReport`; rustdoc still calls contains “the read-only sibling” of the changed report — the sibling type moved. `write_from` documents “`Snapshot` exposes no `generation()` accessor”; `Snapshot::generation` is `pub`. Filename `alloc.rs` is the HEAD method.
- **Why the representation is wrong:** Stale duals (scalar membership vs collection mutation is *essential* for a point probe — the defect is the sibling comment and the false “unrepresentable generation” claim). `alloc.rs` is accidental leftover naming, not a public path.
- **Better representation:** Point membership stays `contains(&F) -> Result<bool>` (one fact, one bit). Fix the sibling sentence. Either remove `Snapshot::generation` (generation only via `Witness` + `Db::generation`, as the rustdoc claims) or delete the claim. Rename `alloc.rs` → `reserve.rs`.
- **Evidence:**

```164:171:crates/bumbledb/src/api/db/get.rs
    /// The read-only sibling of [`WriteTx::insert`]/
    /// [`WriteTx::delete`]'s changed report
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool>
```

```173:177:crates/bumbledb/src/api/db/write.rs
    /// `Snapshot`
    /// exposes no `generation()` accessor: the witness consumes the
    /// generation internally
```

```434:436:crates/bumbledb/src/api/db.rs
    pub fn generation(&self) -> Result<crate::GenerationId> {
        self.txn.generation()
    }
```

---

## Counts

| Severity | Count | IDs |
|---|---|---|
| ship-blocker | 3 | API-1, API-2, API-3 |
| should-fix-before-0.13 | 5 | API-4, API-5, API-6, API-7, API-8 |
| later | 4 | API-9, API-10, API-11, API-12 |

**3 ship-blocker, 5 should-fix-before-0.13, 4 later.**
