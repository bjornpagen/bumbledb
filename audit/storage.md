# Storage / delta / commit / encoding — 0.13 pre-publish audit

Working tree + HEAD. Scope: `crates/bumbledb/src/storage/**`, `encoding/**`, `image/**`, commit applier / judgment / plan, delta insert/delete/determinants. Representation-first (SPOV 1–3). Context: 0.13 deleted public `bulk_load`; resume ETL is a host loop of `write`.

**Counts:** 1 ship-blocker · 9 should-fix-before-0.13 · 6 later

---

### STOR-01 (severity: ship-blocker)
- **Location.** Published contract and in-engine comments still describe the deleted bulk lane: `docs/architecture/70-api.md` (§ ETL, `Db::bulk_load` / `bulk_load_dyn`, chunks of 4096, `BulkLoadError` / `Error::BulkLoad`, `tx.alloc_at`); `docs/architecture/50-storage.md` (lines 497–511, 4096-fact chunks as Lean `scanLoad` operationalization); `docs/architecture/10-data-model.md`; `docs/cookbook.md`; public rustdoc `api/db/snapshot.rs` (`scan_facts` still pairs with [`Db::bulk_load`]); production comments `storage/commit/applier.rs` (`put_data`, `Db::bulk_load`/`bulk_load_dyn` + “4096-fact chunking breaks ordering”); `storage/delta/insert.rs` (“`Db::bulk_load` chunks route through here”). In-tree benches already call `write`+`insert_dyn` but still *name* `bulk_load` (`bumbledb-bench` corpus/load/writes/stress). `Error::BulkLoad` and `BULK_CHUNK` are gone from the crate.
- **Illegal state.** A host following the architecture docs constructs `Db::bulk_load` / `BulkLoadError` / `alloc` — types that do not exist. The comments next to the applier’s put stream still claim a bulk lane and a 4096-fact engine chunk as live mechanism.
- **Why representation.** The 0.13 write surface is one algebra: `WriteTx::{insert,insert_dyn,delete,delete_dyn,reserve}` inside `Db::write`. Chunking, resume counts, and `BulkLoadError` were a second named path over the same delta. Deleting the type without deleting the contract leaves two stories for one machine.
- **Better representation.** One ETL story: host loop of `write` over a collection; resume is host state (committed prefix), not an engine type. Strike `bulk_load`, `4096`, `BulkLoadError`, and `alloc` from docs/rustdoc/comments; keep the MDB_APPEND gravestone as “this put stream is not append-ordered” without naming a dead API.
- **Evidence.** `git grep bulk_load` in `crates/bumbledb/src` hits only those three comments/rustdoc. `70-api.md` still says “chunks of 4096 per transaction” and “conversion into … `Error::BulkLoad { committed, error }`”. C ABI comments already record the retirement (`bumbledb-c/src/lib.rs`).

---

### STOR-02 (severity: should-fix-before-0.13)
- **Location.** `WriteTx` in `api/db.rs`: `applied: bool` + `poisoned: Option<String>`; `refuse_poisoned` / `poison`; `Db::write` Ok-path clones `poisoned` into `Error::TransactionPoisoned { message: String }`. Set in all four collection methods (`insert.rs`, `insert_dyn.rs`, `delete.rs`, `delete_dyn.rs`) on *any* `Ok` from delta apply, including `changed == 0`.
- **Illegal state.** Two independent flags admit four states; only three are meaningful (`Clean`, `Applied`, `Poisoned`). `applied=false ∧ poisoned=Some` is unrepresentable only because `poison` guards `if self.applied`. `applied=true` with an empty delta is representable: a redundant insert/delete sets the flag without entering a fact. `poisoned: String` discards the original `Error` kind — a later `Ok` abort cannot be matched as `Lmdb` / `FactShape`.
- **Why representation.** Classic SPOV 2: flags + guards instead of a sum type. The poison policy (prefix entered ⇒ later mutation and a catching `Ok` still abort) is the state machine; booleans do not carry it.
- **Better representation.** `enum WritePhase { Clean, Applied, Poisoned(Error) }`. Transition `Clean --successful apply (did or not? only did)--> Applied`, `Applied --apply Err--> Poisoned(err)`. `refuse` is a match; `TransactionPoisoned` holds the original error, not `err.to_string()`. Do not mark `applied` on a no-op (`did == false` / delete short-circuit `None`).
- **Evidence.** `poison` only inserts when `applied`; insert sets `applied = true` before testing `did`. Shape-fail test (`a_shape_failure_does_not_poison_the_write`) relies on parse-before-apply *not* going through `poison` — a second policy encoded as control flow, not as the phase type.

---

### STOR-03 (severity: should-fix-before-0.13)
- **Location.** `WriteTx::insert` / `insert_dyn` / `delete` / `delete_dyn` — four copies of refuse-poisoned, empty-collection, refuse-closed, per-row apply, `applied`/`poison`/`MutationReport`. Dyn lanes also `collect()` the iterator, `parse_dyn_row` every row, then intern+encode.
- **Illegal state.** Two user-facing algebras (typed vs dyn, insert vs delete) implemented as four interpreters. Drift is representable: typed streams; dyn materializes the whole collection; typed has no whole-collection parse (shape is unrepresentable); dyn parse-then-apply so `FactShape` does not poison; delete short-circuits unknown interns without `applied`, insert no-ops still set `applied`.
- **Why representation.** Bulk vs write was the same delta with extra names. 0.13 inlined bulk into collection methods and copied the loop four times instead of one collection applicator over `Disposition` + intern mode.
- **Better representation.** One `apply_collection(disposition, rows)` that takes an iterator of already-encoded fact bytes (typed encode, or dyn `ParsedRow`). Poison/report live in that one place. Dyn parse is a parser to `Box<[ParsedRow]>`, not a second walk of `Value`.
- **Evidence.** The four files are line-for-line the same state machine with inverted intern (mint vs resolve) and inverted delta call. `InternMode::Mint` is now `dead_code` because `insert_dyn` inlined minting.

---

### STOR-04 (severity: should-fix-before-0.13)
- **Location.** `api/db/encode_dyn.rs`: `parse_dyn_row` then, in the apply loop, `dyn_value_refs` which repeats arity + `value_matches_parsing`. String’s accepted `&str` from the first walk is thrown away; intern happens on a second parse.
- **Illegal state.** A row can pass `parse_dyn_row` and fail `dyn_value_refs` (or intern) — two boundaries, one of which is a validator returning `()`. Mid-collection intern failure poisons; the first walk cannot carry a proof that intern is the only remaining effect.
- **Why representation.** Parse-don’t-validate: the shape check learns the row is well-typed and drops the knowledge. Every apply-site re-checks.
- **Better representation.** `parse_dyn_row` returns `ParsedRow` (arity-correct `ValueRef`s with `String` still as `&str` / intern-pending). Apply only interns and `encode_fact`. `dyn_value_refs` is the parser, not a second validator; `parse_dyn_row` as `()` goes away.
- **Evidence.** Comment at `parse_dyn_row`: “Shape-check … without interning: arity and type-kind only. Apply (intern + encode) happens after.” That is validate-then-interpret.

---

### STOR-05 (severity: should-fix-before-0.13)
- **Location.** `InternMode::{Mint, Resolve}` in `api/db.rs`; `WriteTx::encode_dyn`; only remaining caller `contains_dyn` (`get.rs`) uses `Resolve`. `Mint` is `#[expect(dead_code)]`.
- **Illegal state.** A mint switch that no write path constructs. `insert_dyn` mints by calling `intern_str` beside `encode_dyn`, so Mint/Resolve is no longer the write-path type — it is a leftover tag.
- **Why representation.** The bulk/dyn encode function was the one intern interpreter. Collection insert stole the mint arm and left the enum as a zombie.
- **Better representation.** Delete `InternMode` and `encode_dyn`, or keep `encode_dyn` as resolve-only for `contains_dyn`/`get_dyn` and drop the mode parameter. Mint is `insert_dyn`’s parser, not a flag.
- **Evidence.** Comment on `Mint`: “insert_dyn mints after parse-then-apply rather than constructing this arm; keep it as the encode_dyn mint switch.”

---

### STOR-06 (severity: should-fix-before-0.13)
- **Location.** `FreshRange` (`api/db/mutation.rs`): empty is `[0, 0)`. `WriteTx::reserve` / `reserve_at` (`alloc.rs`) return that sentinel when `count == 0` without reading `Q`. `WriteDelta::reserve` takes `count: u64` with `debug_assert!(count > 0)`; `from_raw(start, start + count)` is unchecked `end >= start`.
- **Illegal state.** Empty range’s `start_raw()` is `0`, which is a legal minted id. `is_empty()` is a guard every caller must remember (“Not a minted id when the range is empty”). Storage `reserve(0)` in release still reads `Q` and returns `next` without advancing — empty is representable at the delta layer. `len = end - start` underflows if `end < start` ever leaked through `from_raw`.
- **Why representation.** Dijkstra: empty is `[a, a)` at the actual `a`, or a sum type. Sentinel `[0, 0)` makes 0 special. `count: u64` plus a debug_assert is a validator; `NonZeroU64` is a parser.
- **Better representation.** `enum FreshRange<T> { Empty, Ids { start, end_exclusive } }` with `start()` only on `Ids`. Storage `reserve(..., count: NonZeroU64)`. API `count == 0` maps to `Empty` without pretending the sequence is at 0. `from_raw` requires `end >= start` (or is only `Ids::new`).
- **Evidence.** `reserve` docs: “`count == 0` is the empty range `[0, 0)` and does not read or advance the sequence. `count == 1` is the old scalar mint.” `Q` itself is already exclusive-next (half-open); the empty API range does not sit on that coordinate.

---

### STOR-07 (severity: should-fix-before-0.13)
- **Location.** `commit/judgment.rs`: `Checker::check_scalar` vs `check_scalar_sorted`. Same key derivations, same fresh-row vs `U` get, same `check_fact` / `check_segment` verdicts; the get is either `data.get` or `SortedGets::get`.
- **Illegal state.** Two implementations of one probe. A future change to fresh-row width, σ-empty skip, or miss verdict can land in one arm only. `unreachable!("closed-target…")` in `check_source` is the same pattern: the worklist is untyped `EdgeOp`, then re-classified.
- **Why representation.** T8 sorted gets are a *coordinate* on the cursor (ascending keys), not a second judgment. Dual functions are extra names for one algebra.
- **Better representation.** `check_scalar` takes a `Get` trait / `enum ProbeGet { Exact, Sorted(&mut SortedGets) }`. `EdgeOp` is `Scalar { .. } | Coverage { disjoint, source_tail, .. }` so `check_source` does not rematch `Enforcement` or `unreachable` Closed.
- **Evidence.** Comment on `check_scalar_sorted`: “the same key derivations and the same verdicts, the one get answered by the caller’s `SortedGets`.” `check_source` still `match &statement.enforcement` with a Closed `unreachable!`.

---

### STOR-08 (severity: should-fix-before-0.13)
- **Location.** `judgment.rs` `collect`: `Err(Error::CommitRejected { violations })` is flattened into `Vec<Violation>`; probes convict via `Probe::unsatisfied` / `check_capacity` wrapping a singleton `Violations::one` in `Error`. `judge` then `Violations::seal`. Applier phase 2 uses a parallel `Vec<Violation>` without the Error round-trip.
- **Illegal state.** A containment miss is not an engine failure; it is a citation. Encoding it as `Error` then decoding in `collect` (and propagating “real” errors on the other arm) is an accidental interpreter. Two collectors (applier `violations`, judgment `violations`) with two sealing sites (`apply.rs` after phase 2, `judge` after phase 3).
- **Why representation.** Greenspun: control flow grew a bug-ridden Result protocol for “record and continue.” The plan already *is* the table of checks; the outcome of a probe should be data (`Pass | Cite(Violation)`), with `Result` reserved for corruption/storage.
- **Better representation.** `enum ProbeOutcome { Pass, Cite(Violation) }`. `Checker` methods return that. `collect` is `extend`. One `Vec<Violation>` threaded from apply through judge, sealed once. `Error::CommitRejected` is born only at the commit boundary.
- **Evidence.** `collect` matches `Error::CommitRejected` vs `other`. Phase 2 already records directly into `applier.violations` — the judgment side reinvented the same collector as error-channel.

---

### STOR-09 (severity: should-fix-before-0.13)
- **Location.** `commit/plan.rs` `DependentCheck { psi_qualified: bool }`; `FactOp` Insert vs Delete duplicating `relation/fact/fact_hash/determinants/edges` with extra names `capacity_keys` vs `capacity_edges` / `memberships` / `fresh_row`. `check_source` rebuilds a worklist of `(&EdgeOp, &[u8])` then sorts — the plan stored edges on ops unordered for T8.
- **Illegal state.** `psi_qualified: true` on a non-reestablished tuple is only prevented by the `if reestablished` ladder in `target_checks`. `FactOp::edges()` on a Delete is legal; `capacity_edges()` on a Delete returns `&[]` (a silent empty, not a type error). Closed containments are `MembershipOp` on insert and absent on delete — recovered by `else { continue }` in judgment.
- **Why representation.** The plan is supposed to be “representation over control flow applied to the write path.” Booleans and shared methods reintroduce the branches the plan was meant to delete. T8 sort is a second pass because insert ops do not carry a key-sorted probe list.
- **Better representation.** `enum Dependent { Unconditional { containment }, PsiQualified { containment } }`. `FactOp` as common header + `enum Body { Delete { capacity_keys }, Insert { fresh_row, memberships, capacity_edges } }`. Source probes as a plan-owned `Box<[ProbeOp]>` already sorted (T8 is data, not a judgment-phase sort).
- **Evidence.** `target_checks` `match &selections… { Empty => continue, Never => false, Compare(_) => true }`. `capacity_edges()` Delete arm: `&[]`.

---

### STOR-10 (severity: should-fix-before-0.13)
- **Location.** `image/decode.rs` `decode_fact`: `WrongFactWidth { row_id: position as u64 }` where `position` is a dense scan ordinal. `fill_columns` binds `(_row_id, fact_bytes)` and drops the `F` id. General interval arm re-checks `start_word >= end_word` instead of `decode_interval_u64` / `decode_interval_i64`; Bool uses `decode_bool`; fixed-interval correctly calls `decode_fixed_interval_start`.
- **Illegal state.** After deletes, ordinal ≠ row id. The corruption payload names the wrong coordinate — hosts/sweeper comparing to `F` keys disagree. Interval emptiness is validated twice (encoding codec vs image kernel) with a word compare that does not produce `InvalidInterval`’s 16-byte payload the same way as the codec on I64 (biased words vs decoded bounds — they coincide for the compare, but the error bytes are the raw halves, which is fine; the dual *path* is the defect).
- **Why representation.** Positions and row ids are different coordinates (image docs: “row ids exist only in LMDB keys and never appear in images”). Stuffing a position into a `row_id` field is a sentinel/alias. Dual decoders are extra names for one codec.
- **Better representation.** Thread `row_id` from `scan` into `decode_fact` for the error; or a `WrongFactWidth { relation, ordinal, row_id, … }`. Interval/bool fill calls the encoding decoders (already done for fixed-interval and bool). One codec, two sinks (image slabs vs `ValueRef`).
- **Evidence.** `fill_columns` comment: “The row id is discarded at this boundary.” `WrongFactWidth` is the same type `check_width` fills with a real `F` row id (`read/check_width.rs`).

---

### STOR-11 (severity: later)
- **Location.** `storage/keys.rs` `parse_fact_key` / `parse_membership_key` / `parse_fresh_key` / `parse_stat_key` drop the first byte without checking `NS_*`. Only `parse_reverse_key` checks `NS_REVERSE`. `read/scan.rs` `parse_facts` treats parse failure as `"F key length"`.
- **Illegal state.** A 13-byte non-`F` key parses as a fact key. Prefix cursors make this unreachable on the hot path; the codec still does not return a proof-carrying `FactKey`.
- **Why representation.** Validation of length without parsing the tag. A typed `FactKey { relation, row_id }` minted only when `key[0] == NS_FACT` carries the namespace in the type.
- **Better representation.** Each parser checks its tag (or `split_first` + compare) and returns a newtype. Scan’s error becomes `"F key shape"`, not length-only.
- **Evidence.** `parse_reverse_key` is the one parser that already does this. `parse_fact_key` tests round-trip writers, not foreign tags.

---

### STOR-12 (severity: later)
- **Location.** `WriteDelta` `TupleOwners = Vec<(ArenaSlice, Disposition)>`; `determinant_overlay` last-insert-wins (`rev().find_map` Insert). Comments: “two pending inserts of one tuple are commit-doomed but representable.” Cancel-vs-`Absent` is already correct (finding 097): empty owners ⇒ map miss ⇒ committed state, not `Absent`.
- **Illegal state.** Two live inserts of one determinant coexist in the overlay. Point reads during the doomed transaction answer the later fact; commit then rejects. `Absent` vs miss is a good sum (`DeterminantOverlay::{Present, Absent}` vs `None`); the Vec is the leftover bag.
- **Why representation.** Functionality is judged at commit by design (operation order irrelevant) — essential. The overlay’s last-wins rule is an accidental interpreter over a bag that should be `enum Owners { Insert(slice), Deletes(NonEmpty<slice>) }`. A second Insert replaces, or becomes `Conflict`, rather than pushing.
- **Better representation.** At most one Insert per tuple in the overlay. Second insert of a different fact for the same key is still in the *fact* map (commit will convict); overlay need not be a history log.
- **Evidence.** `determinants.rs` overlay docs spell last-recorded Insert. Delta tests pin miss vs `Absent` carefully (`delta/tests.rs`); they do not pin two-insert overlay as unrepresentable.

---

### STOR-13 (severity: later)
- **Location.** `delta/insert.rs` and `delta/delete.rs` — identical four-case table (`pending same` no-op, `pending opposite` cancel, `None` + committed probe record or no-op), inverted `Disposition` and `row_count_delta` sign. `present()` exists (`insert.rs`) and `contains()` wraps it (`accessors.rs`); insert/delete re-inline the membership probe instead of calling `present`.
- **Illegal state.** The 4-case net-disposition algebra can drift per method. `present` is a third copy of “disposition else M probe.”
- **Why representation.** Insert and delete are one operation parameterized by `Disposition`, not two names. (Essential: intern mint vs resolve stays at the API layer, STOR-03.)
- **Better representation.** `fn apply(&mut self, view, rel, bytes, want: Disposition) -> Result<bool>` with cancel = `want` opposite. `present` is the membership read both use.
- **Evidence.** The two `match self.facts.get` blocks are the same table with `Insert`/`Delete` swapped.

---

### STOR-14 (severity: later)
- **Location.** `delta/intern.rs`: dictionary exhaustion is `assert!(next != SENTINEL_ID, "dictionary id space exhausted…")`. `WriteDelta::reserve` / fresh `Q` use typed `Error::FreshExhausted`. Same sentinel (`u64::MAX` never minted — dict `SENTINEL_ID`, interval ray, intern miss).
- **Illegal state.** One exhaustion is a panic (documented as 2^64 mints / txn), the other a typed error. Hostile or buggy `dict_next` at `MAX` after a successful read panics in a write closure.
- **Why representation.** The miss sentinel is one coordinate; exhaustion should be one error type at every allocator (`Q`, dict, row-id if ever host-jumpable).
- **Better representation.** `Error::InternExhausted` (or reuse a single `IdSpaceExhausted { space }`) at the mint site, matching `FreshExhausted`.
- **Evidence.** `intern` comment: “the assert below can therefore fire only for genuine in-memory exhaustion — 2^64 mints in one transaction — which is a documented panic, not data.”

---

### STOR-15 (severity: later)
- **Location.** Capacity window: `check_capacity` / `measure_children` use inclusive-inclusive integers (`measure < lo` or `measure > hi`). Tests/comments spell `0..0` (`commit/tests/marks.rs`: “both ends inclusive, `0..0`”). Intervals elsewhere are half-open `[s, e)`.
- **Illegal state.** `0..0` in Rust is the empty half-open range; the capacity window `{0}` is the singleton `0`. Callers/readers mixing the two coordinates will treat an exclusion floor/ceiling of 0 as empty.
- **Why representation.** Off-by-one is a coordinate choice. Count windows are closed integer intervals; time intervals are half-open. Spelling both with `..` is the special case.
- **Better representation.** A `CapWindow { lo, hi: BoundCeiling }` with `contains(measure)` and docs `[lo, hi]` / `[lo, ∞)`. Tests say `{0}` or `[0, 0]`, never `0..0`.
- **Evidence.** `exceeds_ceiling`: `measure > n`. Floor: `measure < lo`. Comment in `exclusion_window_admits_non_members`.

---

### STOR-16 (severity: later)
- **Location.** `image/build.rs` `build` vs `append`: duplicated claimed-`S` read, `_data` entry ceiling, `CounterDesync`, `field_types` collect, `allocate`, `decode_plan`, `fill_columns`, `RowCountMismatch` if `position != row_count`. `append` adds prefix column copy + distinct extend.
- **Illegal state.** Two fill pipelines. A change to the reopen-trust ceiling or decode kernel can land in one path (the bulk-vs-write shape at image scale: full rebuild vs tail append).
- **Why representation.** Append is build with a prefix image and a `from_row_id` — same algebra, extra names. The lineage claim belongs in a `enum ImageFill { ScanAll, Append { base, from_row_id } }`.
- **Better representation.** One `fill_frame(...)` used by both; `append` only copies columns and extends distincts.
- **Evidence.** The ceiling block is copy-pasted with the same comments (“The same reopen-trust ceiling as `build`”).

---

## Not defects (representation already right)

- **Half-open intervals on `U` / coverage / neighbors.** `pe > s`, `ns < e`, `ss < te && ts < se`, predecessor `pred_end > source_start` — adjacency is not overlap. `scan_from`’s inclusive `u64::MAX` is the unrepresentable `MAX+1` exclusive end.
- **`Q` / row-id high-water.** Stored value is exclusive next; missing = 0; `reserve` returns inclusive start of `[start, start+count)`.
- **`DeterminantOverlay::{Present, Absent}` vs map miss.** Cancel removes overlay entries; `Absent` is not written on cancel (finding 097). Point reads match post-commit.
- **`CommitPlan` as data.** Deletes-then-inserts, selections pre-encoded, capacity/target check sets — the write path already moved bookkeeping out of the applier. Remaining flags (`psi_qualified`, untyped `EdgeOp`, Error-channel citations) are the leftover control flow.
- **Image `View::{Unbound, Bound}`.** Three-variant (really two) instead of a sentinel empty vector — SPOV 2 already applied.

## HEAD vs working tree (this packet)

Working tree *is* the 0.13 write surface: `bulk_load`/`BULK_CHUNK`/`BulkLoadError` deleted; `alloc` → `reserve(count)`; collection `insert`/`delete`; `applied`/`poisoned`; `MutationReport` / `FreshRange`. Storage comments and architecture docs were not updated with that deletion — STOR-01.
