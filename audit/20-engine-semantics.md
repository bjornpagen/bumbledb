# Engine semantics, durable state, and type boundaries

Audit date: 2026-09-04. Scope: `bumbledb`, `bumbledb-theory`, schema-generated fact interfaces, and their relationship to the newer log-backed product. This is a review, not an implementation patch. Findings refer to the working-tree sources present during the audit, not an assumed clean revision.

## Executive judgment

The core has a real, coherent design: facts form sets; a transaction proposes a final state; schema statements judge that state; reads see an immutable snapshot; and representations are intended to become trustworthy after one parsing boundary. The architecture is substantially more deliberate than a thin key/value wrapper.

The most important problems are at the boundaries of that design. A public unchecked interval constructor breaks the very invariant downstream code relies on; concurrent compaction copies a state and its generation from different snapshots; and the escaped-ID theorem does not describe an abrupt process exit. These are more consequential than style cleanup. Separately, the engine's append-only string dictionary is a deliberate compression choice with an undeveloped erasure and lifetime-storage contract.

No Rust memory-exploitation claim is made here. The invalid-input examples below are ordinary small Rust API tests. No production source, existing fixture, or benchmark source was changed.

## Finding inventory

| ID | Priority | Kind | Confidence | Finding |
|---|---|---|---|---|
| ENG-001 | P1 | Correctness / persistent corruption | Reproduced | Safe unchecked interval construction reaches committed facts |
| ENG-002 | P2 | Public API integrity boundary | Reproduced | A custom safe `Fact` implementation can commit noncanonical fact bytes |
| ENG-003 | P1 | Backup / snapshot correctness | Reproduced | Concurrent compaction labels newer data with an older generation |
| ENG-004 | P1 | Durability-contract gap | Reproduced | Escaped fresh IDs can be reissued after abrupt process exit |
| ENG-005 | P2 | Semantic diagnostics | Reproduced | The complete-key-violation collector misses conflicts between refused fresh-ID inserts |
| ENG-006 | P2 | Data lifecycle / architecture | Reproduced | Deleted strings remain live dictionary content after compaction |
| ENG-007 | P2 | Failure reporting / durability | Confirmed static | A rejected write discards failure of its fresh-ID burn |
| ENG-008 | P2 | Capability / build boundary | Confirmed static | Benchmark-only no-sync constructors are ordinary public production APIs |

P1 denotes a high-priority correctness or production-contract blocker in the affected use case. P2 denotes a material issue or explicit product decision, not necessarily a defect in every supported deployment. ENG-006 is an intentional implementation choice whose consequences need a product contract; it is not presented as an accidental dictionary-delete bug.

## ENG-001 — Safe unchecked interval construction can commit invalid intervals

**Evidence:** `crates/bumbledb-theory/src/interval.rs:77-82`; `crates/bumbledb-theory/src/schema.rs:122-155`; `crates/bumbledb/src/api/db/mutation_core.rs:743-755`.

`Interval::new` rejects `start >= end`, but `Interval::__ground_axiom` is a public, safe, const constructor that assigns the two fields without checking. `#[doc(hidden)]` hides documentation, not access. A downstream Rust caller can create `(9, 1)` without implementing a custom codec, using unsafe code, or modifying the engine.

The unrestricted interval arm of `value_matches` checks the enum/type pairing, not the bounds. This would be sound if the host type's construction boundary were closed. It is not. The generated typed insert path also trusts the interval and writes its bounds.

**Reproduction:** declare `relation Window { during: interval<u64> }`; insert `Window { during: Interval::__ground_axiom(9, 1) }` using the normal `Db::write` / `tx.insert` APIs. Observed:

```text
invalid interval bounds=(9, 1)
invalid interval admission=Ok(Accepted(Committed { value: (), generation: GenerationId(2) }))
invalid interval scan=[Err(Corruption(InvalidInterval([0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1])))]
```

**Impact:** a safe, successfully admitted write creates state that subsequent reads diagnose as corruption. It invalidates assumptions used by temporal keys, interval image decoding, and theorem-to-implementation arguments. This is a type-boundary failure, not a numerical corner case of Allen's algebra.

**Recommendation:** make the macro construction path enforce the same ordering invariant as ordinary construction. A const-capable checked/asserting construction mechanism, specialized to the supported scalar domains if necessary, can retain compile-time ground-axiom construction. Do not treat a hidden public method as a sealed constructor. Once construction is closed, avoid scattering redundant checks throughout hot execution loops.

**Regression requirements:** empty and reversed intervals through every public constructor; generated typed writes and dynamic values; both signed and unsigned types; fixed-width types; closed relation ground axioms. Assert rejection before persistence and successful readback of the previous database state. The fixed unsigned matcher currently subtracts without independently checking ordering; invalid constructed inputs must not turn that into a panic path.

## ENG-002 — `Fact` is an extensible safe trait, but insertion treats it as a trusted codec proof

**Evidence:** `crates/bumbledb/src/api/db.rs:148-175`; `crates/bumbledb/src/api/db/mutation_core.rs:363-399`, `416-419`, `743-755`; `crates/bumbledb/src/storage/commit/applier.rs:194-210`.

`Fact` is public and unsealed. The type parameter ties a fact to a schema marker, but a downstream implementation still chooses `RELATION` and writes arbitrary bytes to a `Vec<u8>`. `load` converts any successful `encode_insert` return directly into `ApplyRow::Ready`. The collection path retains those bytes and the commit path persists them. There is no canonical-byte parse between the extensible codec and the trusted internal representation.

**Reproduction:** an ordinary custom implementation for a schema containing `relation Flag { ok: bool }` returned `Ok(())` after writing the single byte `2`. It committed successfully; a normal dynamic scan returned `Corruption(InvalidBool(2))`.

```text
custom invalid bool admission=Ok(Accepted(Committed { value: (), generation: GenerationId(3) }))
invalid bool scan=[Err(Corruption(InvalidBool(2)))]
```

**Important distinction:** generated implementations normally encode correctly, and Rust's safe trait system does not automatically promise that an arbitrary custom implementation obeys database semantics. This is not proof of Rust undefined behavior. It is a public contract mismatch: the implementation uses successful encoding as a canonicality witness, while the public trait neither carries that witness nor clearly identifies and specifies a trusted implementer contract. A future custom codec or wrapper can corrupt a store accidentally.

**Recommendation:** choose one explicit policy. Either support custom fact implementations through a checked encoded-fact boundary, or make generated/trusted codecs a clearly separated capability whose exact obligations are stated and enforced as far as practical. Do not simply mark every downstream consumer unsafe. A generated optimized path and a checked extensible path can preserve the performance goal.

**Regression requirements:** downstream integration tests, not only internal tests, with wrong width, noncanonical bool, invalid interval, bad fixed-byte padding, wrong relation, and invalid string reference. They should produce a typed pre-commit refusal or fail to compile, not admitted corrupt state.

## ENG-003 — Concurrent compaction associates data with the wrong generation

**Evidence:** `crates/bumbledb/src/api/db/maintain.rs:25-29`; `crates/bumbledb/src/storage/env/publish.rs:77-82`, `249-260`, `266-288`.

`PublishCatalog::store` obtains a generation in a short-lived read transaction. Later, after staging-directory creation and destination-environment opening, `write_catalog` begins another source read transaction and copies that snapshot's data and dictionary. The new metadata is written with the earlier captured generation.

The source remains internally consistent within the second snapshot; the defect is that its persisted generation describes a different snapshot. No writer lock or held source transaction connects the two reads.

**Reproduction:** in a new store, one background thread inserted exactly one `Entry` per successful transaction. Therefore the source row count always equaled its generation. Another thread ran ten normal `compact` operations and reopened every destination. All ten copies mismatched:

```text
copy 0: rows=2,   generation=0
copy 1: rows=14,  generation=12
copy 2: rows=25,  generation=21
copy 3: rows=42,  generation=38
copy 4: rows=63,  generation=56
copy 5: rows=79,  generation=76
copy 6: rows=98,  generation=94
copy 7: rows=117, generation=108
copy 8: rows=144, generation=134
copy 9: rows=160, generation=152
```

**Impact:** backup manifests, synchronization watermarks, change detection, and any application treating generation as the identity of a durable state receive false metadata. The copied database's next generation advances from the older number. This does not imply an existing `Witness` from the source is accepted by the destination: process-distinct catalog identity correctly blocks that separate hazard.

**Recommendation:** capture generation and catalog content from the same held source snapshot. Prefer making the publication source own/borrow that snapshot, or reading the generation inside the snapshot used for copying. A writer-wide compaction lock would also synchronize the reads but unnecessarily harms concurrent writes.

**Regression requirements:** a deterministic barrier between source-publication preparation and copy; commit a new fact while paused; assert copied generation and data correspond to one source snapshot. Keep a concurrent stress variant. Verify compacted dictionary-next and fresh counters come from that same snapshot too.

## ENG-004 — Escaped fresh IDs are not crash-durable reservations

**Evidence:** `crates/bumbledb/src/storage/delta/alloc.rs:12-27`; `crates/bumbledb/src/api/db/write.rs:14-52`, `178-205`; `lean/Bumbledb/Txn/Fresh.lean:6-22`, `74-94`.

`reserve` advances an in-memory high-water mark and returns IDs inside the write closure. Persistence happens at commit or through the abort/unwind burn. Abrupt termination does not run the guard. The formal model represents completed transaction transitions and records an I/O-failure narrowing, but it does not cover interruption after an ID escapes and before that transition is persisted.

**Reproduction:** a child process created a fresh database, reserved one `EntryId`, printed it, and exited directly from the closure, deliberately without running destructors. The parent reopened the database and reserved again:

```text
abrupt child=exit status: 7 output=escaped_id=EntryId(0)
id_after_reopen=Some(EntryId(0))
```

This is a small process-lifecycle test of the same relevant condition as a killed process; it is not a power-loss test. No already-committed ID was shown to be reused.

**Impact:** if an application transmits or otherwise persists an ID before the enclosing transaction finishes, the engine can later allocate the same ID for another entity. That matters for offline application workflows, external side effects, and the strength of the advertised never-reissue law. If IDs are explicitly provisional until transaction return, the implementation can be valid under that narrower contract—but the distinction must be visible to embedding callers.

**Recommendation:** choose the contract deliberately. Either require IDs to remain provisional until an acknowledged transaction outcome and document the crash boundary, or persist reservation blocks before making them externally usable. Another option is an external identity scheme with a crash-independent uniqueness domain. Coordinate this with log command construction; do not add a second inconsistent fresh-allocation authority.

**Regression requirements:** abrupt-exit points after reserve, after an explicit fresh-value supply, before ordinary commit, during abort burn, and after successful transaction return. State separately what survives process failure and what survives machine failure. Model an interrupted transaction transition rather than citing a completed-transition theorem as coverage for it.

## ENG-005 — Complete key rejection diagnostics are incomplete for refused fresh landings

**Evidence:** `crates/bumbledb/src/storage/commit/apply.rs:18-31`; `crates/bumbledb/src/storage/commit/applier.rs:99-152`, `166-185`.

An insert whose carried fresh row ID already exists becomes `Landing::Refused`. The code still examines its remaining keys, but only free landings populate temporary determinant entries. Thus two refused inserts that conflict with each other on another key can both observe that determinant as absent. Neither records the additional violation.

**Reproduction:** declare `Person(id fresh, email u64)` with `Person(email) -> Person`. Begin with `(1,10)` and `(2,20)`. Propose both `(1,99)` and `(2,99)` without deleting the incumbents. The proposal violates both the fresh-ID key and the email key. The actual rejection cited only the fresh-ID key (`statement ids=[1]` in the combined test schema); the email-key statement was absent.

**Impact:** the write still rejects, so this is not an invalid-state acceptance bug. It contradicts the documented complete set of violated key statements, creates avoidable fix-one-then-discover-another application loops, and weakens equivalence between heap admission and store delta admission.

**Recommendation:** separate proposed-state key evidence from whether a physical row can be installed at its carried ID. The rejection collector needs to compare all surviving proposed facts, including physically refused landings. Another valid choice is a narrower documented diagnostic contract, but it should be an explicit semantic decision and shared across admission paths.

**Regression requirements:** two refused fresh landings sharing another scalar key; a mix of refused/free landings in both canonical orderings; multiple extra keys; pointwise overlaps; heap-builder versus store-delta comparison of violated statement IDs. Test the complete set of statements, not merely that rejection occurred.

## ENG-006 — Deletion and compaction do not erase dictionary text

**Evidence:** `crates/bumbledb/src/storage/dict.rs:5-12`; `crates/bumbledb/src/storage/commit/write.rs:308-312`; `crates/bumbledb/src/storage/commit/applier.rs:35-79`; `crates/bumbledb/src/storage/env/publish.rs:277-279`.

The dictionary is deliberately append-only and stores raw text under reverse intern IDs. Deletion removes facts and their indexes, not dictionary values. Store compaction copies the whole dictionary, including strings no live fact references.

**Reproduction:** insert and delete one unique text value, verify the relation has zero rows, compact to a new directory, reopen, and use the public codec lookup on the deleted text:

```text
deleted_text_count=0 dictionary_id=Some(InternId(0))
compacted_deleted_text_count=0 dictionary_id=Some(InternId(0))
```

The raw string is still a live reverse-dictionary value, not merely stale bytes in an LMDB free page. Consequently ordinary compaction cannot provide erasure.

**Impact:** lifetime dictionary size tracks distinct text ever inserted rather than current live text. Free-form application text, personal data, and replacement-heavy workloads do not necessarily match the assumption that strings are repeated low-cardinality labels. Per-tenant deployment reduces the blast radius but does not solve in-tenant retention or user-level erasure. Hosted logs, snapshots, backups, and caches introduce additional retention surfaces requiring separate policy.

**Recommendation:** make data lifetime a first-class contract. Consider separate interned-symbol and ordinary-text storage classes, live-dictionary rebuilding during a new-identity export, or a reclamation scheme that never reuses intern IDs and invalidates affected caches correctly. Whole-tenant encryption-key destruction may address whole-tenant erasure but is not equivalent to deleting one user's data within a tenant. Preserve the current append-only fast path only where its lifetime-space cost is appropriate.

**Regression requirements:** repeated insert/delete of unique text; replace a large free-form field repeatedly; compaction size versus live data size; reachability of removed text after the product's designated erasure operation; export/reimport behavior; prepared literal and parameter caches across any reclamation boundary. Do not promise secure erasure merely because logical count is zero.

## ENG-007 — A rejected transaction hides failure to persist its fresh-ID burn

**Evidence:** `crates/bumbledb/src/storage/commit/write.rs:99-113`; compare the explicit error handling at `crates/bumbledb/src/api/db/write.rs:192-200`.

`commit` computes `flush_escaped_fresh_ids` for a rejected proposal. If that flush returns an error, the `Admission::Rejected` branch ignores it and returns an ordinary semantic rejection. The infrastructure-error branch handles `flush`; the rejected branch does not.

**Confidence:** confirmed by control-flow inspection, not by an external disk-failure injection. The existing in-process pending-mark mechanism does retain the failed burn and blocks/retries the next write. This reduces the immediate risk; it does not report the current durability failure or survive loss of the environment/process.

**Impact:** application code can believe it received a fully handled logical rejection when the engine also encountered a persistence failure affecting escaped IDs. Monitoring cannot distinguish the outcomes from the returned result. This combines badly with dropping/reopening a handle or a subsequent crash.

**Recommendation:** make the outcome policy explicit and consistent: either return an infrastructure error while retaining semantic citations as context, or expose a compound outcome that does not hide failed persistence. `Drop` during unwinding must remain best-effort, but ordinary rejected-return paths need not discard the error.

**Regression requirements:** use the existing fresh-flush failure test seam with a reserve plus a schema-rejected proposal. Assert the result reports the failed burn, pending marks remain protected in process, retries preserve monotonicity, and reopen behavior matches the documented contract.

## ENG-008 — No-sync benchmark capability is not isolated from production API use

**Evidence:** `crates/bumbledb/src/api/db/open.rs:39-55`, `100-107`; `crates/bumbledb/src/storage/env/open_env.rs:39-46`; contrast the actual feature gate on `create_store_without_admission` at `open.rs:58-62`.

`create_nosync`, `open_nosync`, and `from_instance_nosync` are public in normal builds. Their only boundary is `#[doc(hidden)]`. The no-sync lane sets `EnvFlags::NO_SYNC`, explicitly giving up machine-crash durability. Calling one is intentional, so this is not a claim that ordinary `Db::open` secretly disables syncing. It is a claim that the source's “bench-only” boundary is documentary rather than structural.

**Recommendation:** gate benchmark-only durability weakening behind an explicitly named opt-in feature or separate test/benchmark support crate. If it is a supported production mode, expose a first-class durability policy with clear guarantees and surface it in diagnostics. Test default downstream builds cannot accidentally use benchmark-only constructors.

## Philosophical decisions that should be preserved—and completed

### Final-state judgment is the strongest part of the design

The delta computes net insert/delete dispositions before LMDB mutation. Commit deletes first, inserts second, judges the final candidate, then commits counters and facts together. This permits ordinary relational state transitions that a statement-at-a-time SQL-like API would reject midway. The design should remain a state-proposal system, not gradually accumulate imperative escape hatches around admission.

However, three kinds of result must stay distinct: a semantic refusal, an infrastructure failure, and an unacknowledged or interrupted outcome. ENG-007 is an example of why collapsing those distinctions undermines the architecture.

### “Parse once, then trust” needs an actual closed boundary

Private representations, sealed schema validation, typed interval construction, and catalog identity are good tools. Public hidden constructors and extensible raw codecs are not automatically proofs. The project should maintain a short inventory of every method that creates an allegedly validated value and the exact evidence required at that method. ENG-001 and ENG-002 should be fixed at their creation boundary, not with ad hoc checks sprinkled through execution.

### Identity has several domains; keep them separate

Fresh numeric values are writable application values, not globally unique identities. Their uniqueness is enforced by ordinary keys. Catalog identity prevents a prepared query or witness from crossing environments. Storage generation names a state change within a catalog's durable history. Intern IDs name dictionary entries. Closed-row IDs are declaration-order vocabulary identities. Confusing these domains will be particularly dangerous in per-tenant hosting, export/reimport, and schema evolution. ENG-003 is precisely a failure to preserve one such domain association.

### Schema identity is intentionally strict, but applications still evolve

`schema/fingerprint.rs:44-116` hashes relation/field names, ordering, types, extension rows, and materialized statements. `Db::open` validates and requires the matching schema. This is valuable corruption defense, not itself a bug. The missing product-level decision is how a real application migrates many tenants while keeping command replay, closed vocabulary IDs, and constraints meaningful. A migration facility should operate on versioned theories and admitted state transforms, with explicit old/new identity, not weaken fingerprint checking to make upgrades convenient.

### Logical sets do not settle physical lifetime

Idempotent fact insertion and canonical set answers do not answer how long text, snapshots, images, logs, reservation gaps, or deleted entity identifiers live. The embedded and hosted products need one explicit lifetime policy covering them. The immutable/read-heavy architecture is a strength only if retention is deliberate and bounded for the intended application workload.

## Areas inspected without a confirmed defect

- The read/write LMDB transaction ownership uses `heed`'s owned static read transactions rather than a local ad hoc lifetime transmute. No new lifetime-unsoundness claim was established.
- Writer reentrancy is checked before the writer mutex; recovery from a panicking write does not automatically poison all future writes.
- A reader can overlap a commit, but generation-stamped parked-reader reuse is checked against the published generation. The inspected race windows did not establish a post-return stale-read bug.
- Image-cache advancement distinguishes deletions from append bases and checks inserted floors, preserving readers pinned to older images. See the performance audit for costs, not a claimed correctness failure here.
- Fresh exhaustion uses checked reservation arithmetic; explicit maximal values exhaust rather than wrap the generator. ENG-004 concerns persistence timing, not arithmetic wraparound.
- Hash membership and string interning use cryptographic digests without a full collision-disambiguation structure. No practical collision bug was demonstrated; deciding whether “set semantics” is mathematical or cryptographic-assumption-level exactness is a specification choice, not a P1 exploit finding.
- Shape errors from dynamic input are intentionally distinguished from failures after mutation has entered a batch. Existing tests explicitly document that a shape miss need not poison a transaction; this was not reported as an accidental missing poison.

## Test record and limitations

An external test crate was created at `/tmp/bumbledb-engine-audit.6iFaKq`; it depended on the working-tree engine by path. Its harness and database directories are outside the repository. The successful command was:

```text
cargo run --manifest-path /tmp/bumbledb-engine-audit.6iFaKq/Cargo.toml
```

The final run exited 0 and produced the results quoted above plus QRY-001 in `21-query-runtime.md`. An earlier compile attempt required an explicit closure return type for the abrupt-exit test; it performed no database run. The final small test harness emitted an unused-must-use warning on a control-flow path that exits before returning; the production dependency built successfully. The external crate resolved its own dependency lockfile, so this is not a claim that every workspace lockfile/build matrix was tested. No destructive resource-exhaustion experiment, physical power-cut test, corrupted-production-store test, or memory-exploitation work was performed.

This report is not a proof of absence of other bugs. Its scope is the high-leverage semantic and durable-state boundary review; the companion reports cover log protocol, hosting, bindings, performance, and broader assurance.
