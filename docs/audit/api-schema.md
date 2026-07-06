# API, schema, and encoding correctness audit

Audit date: 2026-07-06. Auditor scope: the public API (`Db`, transactions, results),
the schema layer (descriptors, validation, fingerprint, macro), and value encoding.
Method: every suspected issue below was traced with concrete values through the code
path; findings are separated from unproven observations (none of the latter survived
tracing — they are either promoted to findings or listed under "Checked and sound").

## Scope (files and docs read, with line counts)

Free Join paper (algorithmic authority):
`docs/free-join-paper/arXiv-2301.10841v2/main.tex` (162) and its inputs
`tex/00-abstract.tex` (15), `01-intro.tex` (243), `02-background.tex` (510),
`03-free-join.tex` (608), `04-optimizations.tex` (478), `05-eval.tex` (337),
`06-discussion.tex` (85); `07-relatedworks.tex`/`08-conclusion.tex` are empty and
not `\input` by main.tex (`025-tale.tex` likewise commented out).

Architecture docs (product authority): `README.md` (71), `00-product.md` (186),
`10-data-model.md` (227), `20-query-ir.md` (178), `30-execution.md` (295),
`40-storage.md` (205), `50-validation.md` (179), `60-api.md` (120).

Primary audit targets, read in full:
- `crates/bumbledb/src/api/db.rs` (903)
- `crates/bumbledb/src/api.rs` (5), `crates/bumbledb/src/api/stats.rs` (66)
- `crates/bumbledb/src/schema.rs` (1078), `schema/runtime.rs` (197),
  `schema/fingerprint.rs` (303)
- `crates/bumbledb/src/encoding.rs` (487)
- `crates/bumbledb/src/digest.rs` (42)
- `crates/bumbledb-macros/src/lib.rs` (605)

Supporting code read to verify API contracts end to end:
`api/prepared.rs` (2139; API/bind/finalize portions in full, executor plumbing as
needed), `storage/delta.rs` (524), `storage/commit.rs` (1266), `storage/env.rs`
(492), `storage/dict.rs` (307), `storage/keys.rs` (278), `storage/read.rs` (418),
`image/cache.rs` (450), `arena.rs` (102), `lib.rs` (123), `ir.rs` (value layer,
292), `exec/dispatch.rs` (637), `exec/sink.rs` (accumulator/overflow lines),
`image.rs` (word-encoding contract lines), `tests/schema_macro.rs` (235),
`tests/api.rs` (879, skimmed for contract coverage). Unit tests for
`encoding::`/`schema::` were run (46 passed) to confirm the reading.

## Verdict

The audited surface is in very good shape: the canonical encoding is exactly
order-preserving and round-trip-faithful at every extreme I traced (including
i64::MIN/-1/0/1/i64::MAX and the 256-variant enum boundary), the dense layout and
`decode_field` agree byte-for-byte, the fingerprint is length-prefixed and
alias-free with every semantic input covered, schema validation enforces the full
documented roster (including the FK-target/type/arity rules and the guard-width
ceiling), the `schema!` macro is provably "exactly sugar" (pinned by a
fingerprint-equality test against a hand-built descriptor, and my independent
reading of its codegen agrees with `encoding.rs` byte-for-byte for all six types),
and the delta write path makes operation order genuinely irrelevant with correct
changed-state reports, row-count arithmetic, and commit-time constraint semantics.
I found no CRITICAL or HIGH defects. The one substantive finding is a
MEDIUM-severity contract gap: serial values (and provisional intern ids) minted in
a transaction whose delta nets to no-op are silently dropped at commit even though
the minted id escapes to the host through the write closure's return value, so a
"committed" transaction can hand out an id that a later transaction re-issues —
contradicting the code's own "none of them are observable" claim and a natural
reading of `10-data-model.md`'s never-reissue guarantee. The remainder is a small
set of LOW edge-fragilities and NOTE-level doc gaps.

## Findings

### [MEDIUM] Serial ids minted in a net-no-op committed write escape and are re-issued

- **Where:** `crates/bumbledb/src/storage/commit.rs:214-237` (empty-delta and
  `!applied.changed` paths skip `flush_counters` entirely),
  `crates/bumbledb/src/storage/delta.rs:214-219` (doc comment: "pending
  allocations and interns of an empty delta are deliberately dropped — none of
  them are observable"), `crates/bumbledb/src/api/db.rs:160-187` (`Db::write`
  returns the closure's value after a successful no-op commit).
- **Documented invariant:** `10-data-model.md`: the generator is "monotonic per
  (relation, field), never re-issuing any value observable in a committed state;
  aborted transactions don't advance the committed sequence." The delta's own
  comment claims dropped allocations are unobservable.
- **Concrete failure scenario:**
  ```rust
  let a: HolderId = db.write(|tx| tx.alloc::<HolderId>())?;   // Ok(HolderId(0));
      // delta.facts is empty -> commit() takes the is_empty() path, Q never flushed
  let b: HolderId = db.write(|tx| {
      let id = tx.alloc::<HolderId>()?;                        // HolderId(0) again
      tx.insert(&Holder { id, name: "x".into() })?;
      Ok(id)
  })?;
  assert_eq!(a, b); // two *successful* write transactions returned the same id
  ```
  The same drop happens on the non-empty-but-unchanged path (e.g. a delta whose
  only entries are insert-then-delete of an absent fact plus an `alloc`):
  `commit.rs:229` aborts the txn without flushing `Q`. The minted value is
  observable — it is the return value of a write that reported success. A host
  that persists `a` externally (a log, a file, another store) and treats it as a
  unique key is silently wrong; a host that later inserts a fact carrying `a`
  collides with `b`'s fact (caught as `UniqueViolation`, but pointing at the wrong
  culprit from the host's perspective). This is not engine-state corruption — no
  stored data is wrong — but it is a doc-code contradiction and a real trap on the
  exact "insert new rows without ever reading a max" pattern the feature exists
  for.
- **Fix direction:** either (a) flush dirty `Q`/dict counters even on no-op
  commits (a counters-only LMDB commit that does *not* advance the storage tx id
  or evict images — the generation rule and the cache are unaffected by counter
  writes), or (b) make `WriteTx::alloc` values non-escaping in no-op transactions
  a documented rule in `10-data-model.md`/`60-api.md` and fix the delta.rs
  comment, which is false as written. (a) matches the docs as written.

### [LOW] Deleting a fact with a never-interned string permanently interns it

- **Where:** `crates/bumbledb/src/api/db.rs:549-557` (`delete` deliberately
  encodes through the write context), `storage/delta.rs:91-114` (`intern` mints a
  provisional id on a committed-dict miss), `storage/commit.rs:339-344`
  (`flush_counters` writes *all* pending interns of a changed commit).
- **Documented invariant:** `10-data-model.md` limits the accepted dictionary leak
  to "deleted facts leak their interned values"; the read path's rule is "a miss
  means the literal cannot match any fact — never an insert."
- **Concrete failure scenario:** `db.write(|tx| { tx.delete(&Holder { id, name:
  "typo-xyz".into() })?; tx.insert(&other_fact)?; Ok(()) })` — the delete is a
  correct no-op (`false`), but "typo-xyz" is minted as a pending intern and, because
  the transaction otherwise changed state, flushed to `_dict` forever (no GC by
  design). Write-context encoding *is* required for insert-then-delete
  cancellation, but that only needs the pending-map lookup: a value absent from
  both the pending map and the committed dictionary cannot appear in any existing
  or delta fact, so the delete could resolve the miss to "fact cannot exist — no-op"
  without minting. Pure dictionary growth; no correctness effect on results.
- **Fix direction:** a mint-free lookup path for delete-side encoding (pending map
  → committed dict → miss ⇒ report fact-absent), or document the extra leak class.

### [LOW] Out-of-range `RelationId` panics on the dynamic/ETL surface

- **Where:** `crates/bumbledb/src/schema.rs:213-215` (`Schema::relation` indexes
  `self.relations[id.0 as usize]`), reached from public `WriteTx::insert_dyn` /
  `delete_dyn` (`api/db.rs:567-581` via `encode_dyn`), `Db::bulk_load`
  (`api/db.rs:286`), and `Snapshot::scan` (`api/db.rs:437-438`).
- **Documented invariant:** `60-api.md`: "Mis-shaped dynamic facts are typed
  `FactShape` errors (decided: ETL input is data, not code — no panics on the
  import path)."
- **Concrete failure scenario:** `db.bulk_load(RelationId(999), facts)` or
  `snap.scan(RelationId(999))` — index out of bounds panic, not a typed error.
  `RelationId` is a public tuple struct (`pub u32`), so ETL tooling that derives
  ids from external metadata can construct one. Arity/type/enum/UTF-8 mismatches
  are all typed `FactShape` errors; the relation id is the one input on the same
  surface that panics instead. (`alloc_dyn` on a non-serial field also panics, but
  that one carries an explicit `# Panics` contract — `api/db.rs:515-519`.)
- **Fix direction:** either add an `UnknownRelation` arm to `FactShapeError` and
  bounds-check at the `insert_dyn`/`delete_dyn`/`bulk_load`/`scan` boundary, or
  document the panic as a programmer-error contract like `alloc_dyn`'s.

### [LOW] Nothing binds a `PreparedQuery` to the `Db` it was prepared against

- **Where:** `crates/bumbledb/src/api/db.rs:369-427` (`Snapshot::execute` passes
  only `self.txn`/`self.cache` into `prepared.execute`; the prepared query's own
  `schema` pointer is used for all layout/decoding decisions,
  `api/prepared.rs:259-294, 529-601`).
- **Documented invariant:** implicit in `60-api.md` ("`db.prepare(&Query)` is the
  sole entry", executed inside `db.read`); no cross-handle rule is stated.
- **Concrete failure scenario:** two open handles with *different* schemas (both
  `'static` via the macro): `let p = db_a.prepare(&q)?; db_b.read(|snap|
  snap.execute(&mut p, &[], &mut out))?` compiles and runs — db_b's facts are
  decoded through db_a's layouts. If the fact widths differ this surfaces as
  `Corruption(WrongFactWidth)`; if they coincide (same widths, different field
  meanings) the result is silently wrong data presented as valid. Same-schema
  cross-store execution is well-defined (and arguably useful), so a blanket check
  has a cost; the silent-wrong-data case requires deliberate misuse.
- **Fix direction:** cheapest honest option is a stated rule in `60-api.md`
  ("a prepared query may only execute against snapshots of a database opened with
  the same schema"); a structural option is comparing `std::ptr::eq(prepared.schema,
  snap.schema)` at `Snapshot::execute` (one pointer compare on the warm path).

### [NOTE] `bulk_load`'s `committed` counts changed facts, not consumed facts

`api/db.rs:286-324, 327-336`: `BulkLoadError::committed` (and the `Ok` total) is
the number of facts that *changed state* in committed chunks. With duplicate facts
in the input stream, a resume that skips `committed` items re-processes some
already-committed items — safe, because insert is idempotent and explicit serials
re-advance the high-water — but the field is not "how many input items were
consumed." `60-api.md` says only "the committed count"; the field doc in code is
accurate. Also note `impl From<BulkLoadError> for Error` deliberately drops the
count (documented at `api/db.rs:354-359`). Worth one sentence in `60-api.md` if
resumability is ever relied on.

### [NOTE] `compact()` concurrency is safe but undocumented

`api/db.rs:216-234` + `storage/env.rs:169-173`: `Db::compact` does not take the
writer mutex and may run while a writer is active; this is safe — heed's
`copy_to_file` is LMDB's `mdb_env_copy2`, which snapshots via an internal read
transaction, so the copy is a consistent committed state and never blocks or is
corrupted by the writer. Neither the method doc nor `40-storage.md` (which pins
"backup = quiesced file copy" for the *external-cp* mechanism) states this for
`compact`. One sentence would close it. The copy-refuses-existing-destination and
file+directory fsync behavior match the docs.

### [NOTE] "Constraint ids assigned by declaration order" actually means materialized order

`10-data-model.md` ("Relation/field/constraint ids are assigned by declaration
order and are therefore pinned by the fingerprint") reads as declared order, but
the real rule (`schema.rs:22-26`, `schema.rs:389-398`, mirrored by
`schema/runtime.rs:83-117`) is auto-materialized serial uniques first (in field
declaration order), then declared constraints. The fingerprint serializes the
*materialized* constraint list (`fingerprint.rs:35-66`), and materialization is a
deterministic function of the declaration, so the pinning claim genuinely holds —
verified: FK targets serialize by name, generation flags are hashed, and I found
no two distinct declarations that alias one byte stream. Doc imprecision only.

### [NOTE] `Db::write` is non-reentrant; nested write self-deadlocks

`api/db.rs:160-165`: the writer `Mutex<()>` is a plain non-reentrant std mutex, so
`db.write(|_| db.write(|_| ...))` deadlocks the calling thread forever (LMDB never
enters the picture — the outer guard blocks the inner `lock()`). `db.read` inside
`write` and `write` inside `read` are both fine. Standard behavior, but the doc
comment could say "do not nest write transactions" since panics were considered
carefully enough to clear poisoning.

### [NOTE] `Fact::encode_read` has no engine caller

`api/db.rs:54` declares it, the macro generates it, `tests/schema_macro.rs:215-228`
exercises it — but nothing inside the engine calls it (no typed read-side
membership/point-lookup API exists yet). Its reader is host code, which satisfies
README rule 3 in spirit; recording it here so the surface is a decision, not an
accident. (Its contract — `Ok(false)` with `out` untouched on an intern miss — is
implemented correctly: the early return happens while the values array is built,
before anything is appended.)

### [NOTE] FK field-list duplicate check reuses the `UniqueDuplicateField` error variant

`schema.rs:424-433`: a duplicated field in a *ForeignKey* constraint's list is
correctly rejected, but the error is `SchemaError::UniqueDuplicateField` — a
misleading name in a diagnostic, nothing more (test
`rejects_duplicate_fields_in_an_fk_list` pins the behavior).

## Checked and sound

Encoding and layout:
- `encode_i64` sign-flip bias: byte order equals numeric order across the full
  range — traced i64::MIN → `0x0000…00`, -1 → `0x7FFF…FF`, 0 → `0x8000…00`,
  i64::MAX → `0xFFFF…FF`; round-trip exact for all extremes (unit-tested at
  `encoding.rs:309-367`, re-run during this audit).
- `encode_u64` big-endian order preservation across 0/255/256/2⁶³±1/u64::MAX;
  `decode_bool` strictly 0x00/0x01 (0x02–0xFF are `InvalidBool` corruption, never
  a distinct true); `decode_enum` range-checks against `variant_count` with the
  256-variant boundary exact (`(ordinal as u16) < variant_count`).
- `FactLayout` offsets are exactly cumulative widths with zero padding (1-byte
  fields flush against 8-byte fields, `10-data-model.md`'s dense contract);
  `field_bytes`/`decode_field` slice at exactly those offsets; `encode_fact`
  output equals independent per-field encodings byte-for-byte; the nullary fact
  encodes to zero bytes with a well-defined blake3 identity.
- Every consumer of raw `F` values (`read::fetch`, `read::scan`, and therefore
  `Snapshot::scan`/`scan_facts`/`Fact::decode`) width-checks against the schema
  before `decode_field`, so a truncated stored fact is `Corruption(WrongFactWidth)`,
  never a slice panic or a skip; the scan iterator fuses after the first error.
- `fact_hash` is the full untruncated 32-byte blake3; the collision axiom is
  applied exactly where the docs record it (`M` hits and the dictionary forward
  map trusted without byte verification).
- Column-word conventions are consistent end to end: images store
  `u64::from_be_bytes(canonical bytes)` (biased for I64), `bind_param` produces the
  same biased words for I64 params, guard keys serialize `Const::Word` as BE bytes
  and `Const::Byte` as one byte (= canonical field encodings), filter comparison of
  biased words under unsigned order equals signed order, and
  `ResultBuffer::push_word` un-biases with the same XOR.

Schema layer:
- Validation roster verified against code: duplicate relation names, duplicate
  field names, empty enums, >256-variant enums (256 accepted, 257 rejected),
  duplicate variants, serial-on-non-U64, unknown constraint fields, empty uniques,
  duplicate fields within any constraint list, duplicate constraint names
  (auto-unique collisions included), duplicate unique field-sets (including
  auto-unique vs declared), unknown FK target relation/constraint, FK targeting a
  non-unique, FK arity mismatch, FK positional structural-type mismatch (one
  derived `ValueType` equality *is* the rule — structural enums unify iff variant
  lists match exactly, tested), and the `MAX_GUARD_WIDTH` (492-byte) ceiling
  derived from the Restrict key's 19-byte overhead so oversized guards are
  rejected at declaration, never discovered at write time.
- FK resolution is a second pass over fully-materialized constraint lists, so
  forward references and self-references work; `fk_targeted` is deduplicated and
  drives exactly the Restrict scan set.
- The fingerprint covers every input `10-data-model.md` enumerates (format label,
  relation/field/constraint names and order, full ordered enum variant lists,
  generation flags, FK targets by name); all lists and strings are u32
  length-prefixed (the "AB"+"C" vs "A"+"BC" aliasing test pins it); a golden byte
  stream pins the serialization against drift; field reorder, rename, variant
  add/reorder, constraint field order, FK retarget, and serial toggle all change
  the fingerprint (tested). I could not construct a semantic difference that
  leaves the fingerprint fixed, nor a non-semantic one that changes it (constraint
  names are semantic here: they are the FK-target namespace and error payloads).

The `schema!` macro:
- Generated code is *exactly* sugar: pinned by fingerprint equality against a
  hand-built descriptor covering serial + redundant `unique` (dropped), per-field
  FK, compound unique, and compound FK targeting a compound unique
  (`tests/schema_macro.rs`). My independent reading agrees: `RELATION` = declaration
  index, `Serial::FIELD` = field declaration index, values arrays in declaration
  order through the same `encode_fact`, generated Rust enum ordinals (`self as u8`)
  = engine declaration-order ordinals, decode arms are the exact inverses with
  `from_ordinal` total over the range `decode_field` admits.
- `runtime::constraint_id`'s name-order reconstruction (serial auto-uniques,
  per-field uniques, per-field `{f}_fk`s, compound uniques `a_b`, compound
  `a_b_fk`s) matches `declared_constraints`' emission order and `validate`'s
  auto-first materialization, so FK target ids resolve correctly.
- Grammar edge cases: fields named `serial`/`unique`/`fk` parse correctly (the
  `:` lookahead), field-level vs compound `fk` disambiguated by the `->` probe,
  `as NewType` restricted to u64/i64, `serial` requires a newtype with an
  explanatory panic; conflicting reuse of one newtype or enum name across
  relations is either a macro assert (enum variant mismatch) or an rustc error
  (duplicate/conflicting impls) — nothing silently misencodes. Name-based
  resolution failures panic with the offending name at first `schema()` call, and
  descriptor-level errors surface as the typed `SchemaError` rendering (documented
  contract).

Write path and `Db`:
- Insert/delete interplay traced exhaustively: last-disposition-wins with exact
  changed-state reports; insert-then-delete of an absent fact nets a Delete that
  apply no-ops; delete-then-insert of a present fact nets an Insert that apply
  no-ops (and reports `changed: false`, so the tx id does not advance and the
  image cache stays warm — tested); row-count deltas stay exact under arbitrary
  interleavings including the three-step delete/insert/insert-back sequence
  (net +1 traced by hand); delete of a nonexistent fact is a reported no-op.
- Serial semantics: `alloc` reads `Q` once per (relation, field), a transaction
  sees its own allocations, explicit values below/above the mark advance it via
  `max(mark, value.saturating_add(1))` (explicit u64::MAX is insertable and then
  exhausts the generator with a typed eager `SerialExhausted`); aborted
  transactions re-issue (tested); committed allocations flush and persist across
  reopen (tested). The u64::MAX intern-id sentinel is asserted never-minted on
  both mint paths.
- Commit order (deletes → inserts → forward FK → Restrict → counters → tx id →
  fsync) makes a `U` conflict during inserts a genuine violation; delete/insert of
  the same unique key succeeds in either user order; deleting a target and all its
  referrers in one tx passes; re-supplied keys subtract from the Restrict scan set
  via `inserted_guards`; guard keys are re-derived by slicing (never a scan) and
  fixed-width per constraint so the `R` prefix scan cannot alias across guards;
  M/F/U desync is a hard `MembershipDesync` corruption, never a scrub; FK errors
  name the referring *fact* (fetched inside the still-open txn), never a row id.
- `Db::write` holds the writer mutex across closure + commit + cache eviction, so
  two write closures can never interleave; the read view is opened under the mutex
  and dropped before the LMDB write txn opens, and since no other writer can run,
  the delta's membership probes remain valid at apply time (no TOCTOU). Panic in
  the closure unwinds the delta with LMDB untouched, and the poisoned mutex flag is
  deliberately cleared (correct: the delta died in the unwind).
- `bulk_load`: 4096-fact chunks, each chunk atomic through the same delta path;
  mid-chunk shape errors and commit-time violations abort exactly that chunk with
  prior chunks durable and the changed-count carried (tested at the 4096/4100
  boundary); an input length that is an exact chunk multiple ends with one
  harmless no-op commit; `chunk` folds into `total` only after its commit, so a
  failed commit never counts its partial successes.
- `Db::create` refuses an initialized directory (`AlreadyInitialized`) inside the
  same write txn that would have created `_meta` — create is exactly as
  non-destructive as open; open verifies format version strictly before the schema
  fingerprint (tested with a corrupted version and a wrong schema simultaneously);
  heed's single-open-per-path registry upholds the one sanctioned `unsafe` block's
  safety condition.
- Concurrency claims: `Db` is `Send + Sync` by auto-traits over heed's
  `Env`/`Database`, the `Mutex`-guarded cache, and plain-data `Schema`
  (`assert_send_sync::<Db>` plus a real reader/writer thread-scope test);
  `PreparedQuery` is `!Sync` via `PhantomData<Cell<()>>` with a `compile_fail`
  doctest, and remains `Send` (move-between-threads is the documented contract).
  `ReadTxn::generation` reads the tx id from `_meta` *inside its own snapshot*
  (memoized per snapshot in a `OnceCell`), the cache is keyed on it, eviction runs
  under the writer mutex, and `get_or_build` re-checks `newest` under the insert
  lock — the evicted-generation re-insert race is closed; racing same-generation
  builders converge on one `Arc` (tested); pinned old-generation readers build
  query-locally without polluting the map (tested); a no-op commit invalidates
  nothing (tested).
- Param binding: count and structural type checked per execution through the same
  `value_matches` used by validation and the dynamic write path (kind, enum
  ordinal range, UTF-8); String/Bytes params resolve per execution by read-only
  lookup — a miss under `Eq` (filter or selection) short-circuits to empty, a miss
  under `Ne` resolves to the never-minted sentinel and matches everything (both
  regression-tested); later-interned values are picked up on the next execution of
  the *same* prepared query (tested). Guard probes build keys from the same
  canonical constants, treat misses/filter failures as empty (never an error,
  never an insert — tested), and agree with the Free Join path on the same query.
- Results: `ResultBuffer` clear-retains capacity; `len` = cells/arity with the
  fresh-buffer (arity 0) case defined; string cells are UTF-8-validated at
  materialization (`NonUtf8Intern` corruption otherwise); the finalize intern memo
  resolves each distinct (word, tag) once per finalize and cannot leak a poisoned
  placeholder range across finalizes (cleared on entry; an error aborts the whole
  execute). `column_types()` supplies the typing metadata for empty results.
- Aggregates/stats: Sum accumulates in i128/u128 with exactly one range check at
  finalization mapped to a typed `Overflow { find }` (i128 cannot overflow at ≤10⁷
  facts × i64 values); Count is u64; all `ExecutionStats`/`NodeStats`/`CoverStats`
  fields are u64 with only usize→u64 conversions on a 64-bit-only target (32-bit
  is a compile error in `lib.rs`) — no lossy conversions found.
- `digest.rs` is a faithful streaming blake3 wrapper (streaming ≡ one-shot,
  tested); the dictionary segregates String/Bytes by a tag byte inside the hashed
  key, resolves with a tag-mismatch corruption check, treats an empty reverse
  entry as dangling, and round-trips the empty string correctly (1-byte reverse
  value = tag only).
