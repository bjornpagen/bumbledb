# Post-Mortem of the v1–v5 Implementations (May 2026)

Provenance: this review was originally a 34-file dump (`todo/`) produced 2026-07-02 by a
full-codebase audit of the v5 tree at `465e3d4` (= `1b65ae8^`). The dump was purged from
the working tree without ever being committed; this document re-materializes it from the
session record, with evidence preserved and fix-checklists condensed. File:line
references are into the v5 source, readable via `git show '465e3d4:<path>'`.

## §00 Summary — where the AI build went wrong

Eight days of history (2026-05-17 → 05-24, 291 commits, ~36/day), abandoned mid-PRD;
the final uncommitted change renamed a trace field `family`→`derivation`. Totals:
116,033 lines of Rust written, 93,800 deleted (81% discarded); 46,801 doc lines written,
46,000 deleted (98%); 25+ PRD suites created and deleted; schema label reached "v5",
storage format 6.

Five root causes: (1) **doc-driven, not measurement-driven** — the storage layout was a
literal transcription of ROSETTA_STONE's namespace table, and the measured disaster (80%
of query time in base-image loading) was a direct consequence of the doc's own layout;
fixes patched around it instead of questioning it. (2) **Planning theater** — as much
effort on PRDs about the code as on code; every suite ended in a compliance gate
ratcheting process, not outcomes. (3) **The hard parts were fake, the ceremony real** —
correct Free Join kernel, but a cost model of five hardcoded constants and a
"vectorized" mode that ran the scalar path per tuple; meanwhile per-tuple trace spans
and allocation-counter plumbing (~17% of the engine) were fully built. (4)
**Self-imposed gates shaped the code** — a 500-line-per-file gate fragmented the code
into ~70 files just under the limit. (5) **Nothing pushed back on plausible
elaboration** — single-variant enums with unreachable guards, dead API surface, dual
identity systems, tests asserting string constants differ.

What was good and kept: order-preserving encodings, blake3 schema fingerprinting, the
Free Join plan formalism/validation/recursive executor/lazy COLT force, the
exact-SQLite-oracle discipline, fixed-width interned encoding and contiguous column
images, `InlineTuple`/`TupleBatch`.

## §01–03 Process findings

**§01 PRD churn.** 25+ suites (`paper-alignment`, `performance-hardening`,
`allocation-redesign`, `final-collapse`, `thermonuclear`, `set-engine-rebase`,
`set-native-rewrite`, `v2-cleanup`, `v3-job-recovery`, `v4-benchmark-bugfix`,
`v5-algorithm-deletion`, `v6-mechanical-performance`, kill lists…), 350+ doc files
written and deleted. PRD residue leaked into code: `Error::Unavailable { prd }`
(`crates/bumbledb-lmdb/src/error.rs:15-21`), test dirs named `prd07`/`prd08`/`prd09`.

**§02 Line-count gate.** `scripts/check-line-counts.sh` failed CI over 500 lines/file;
production files clustered at 470–491 lines; `run.rs` existed to forward 11 arguments;
`storage_v5.rs`/`storage_v5_codec.rs`/`storage_v5_meta.rs` split one write path three ways.

**§03 Doc-driven storage.** The ten-namespace layout `T H L P C Q U R A S` transcribed
ROSETTA_STONE.md:147-161; a conformance test asserted the namespace bytes spell
"THLPCQURAS" (`storage_format_tests.rs:62-82`). The thermonuclear README recorded the
consequence: 235.06 ms of 291.91 ms total execution (80.5%) in `BaseImageLoad`.

## §10–18 bumbledb-core findings

**§10 Dual identity.** `TypedRelationAtom { relation_id: usize, relation: String }`
stored identity twice (`query_ir.rs:87-89`); linear-scan name resolution everywhere
(`schema/descriptors.rs:31,159`, `query_builder.rs:164`); `QueryBuilder` kept
`variables: Vec<TypedVariable>` plus a parallel `variable_ids: Vec<(String, usize)>`
map (`query_builder.rs:57-58,270-274`); `TypedVariable.id` stored its own index.

**§11 Dead API.** `QueryBuilder::relation()` verbatim alias of `rel()` with identical
doc comment, zero callers (`query_builder.rs:90-96`); `is_interned_placeholder()` zero
external callers; `SchemaFingerprint` Display/Debug never exercised (all consumers
unwrapped `.0`); `EnumVariantDescriptor` never constructed outside tests — fixtures used
`EnumDescriptor::codes` which fabricated `format!("code_{code}")` names hashed into the
persistent fingerprint; thirteen convenience constructors for one external consumer.

**§12 Single-variant enums.** `ForeignKeyAction` had one variant yet
`validation.rs:166-172` checked `!= Restrict` — statically unreachable, with a
hand-written error message for an unimplemented feature. `TypedFindTerm` = one-variant
enum. `FieldGeneration::SerialSequence` cross-checked 19 lines against
`ValueType::Serial`, two redundant representations of one fact.

**§13 IR gaps.** `Literal` had `Bool | Integer | String` — **no Bytes variant**
(`query_ir.rs:7`), so a Bytes field could never be bound to a literal; no test noticed.
`merge_types` was `==` in costume (`query_builder.rs:429`);
`foreign_key_types_compatible` likewise (`validation.rs:313-315`). `finish()` validated
nothing (unground comparison-only variables, empty find passed through);
`finish(&mut self)` left a zombie builder.

**§14 Layout leak.** `ValueType::encoded_width()` hardcoded storage-format knowledge
(strings as 8-byte intern ids) into the logical type enum (`descriptors.rs:236`).

**§15 Error variant abuse.** `VariableTypeConflict { variable: "comparison" }` — a fake
variable literally named "comparison" (`query_builder.rs:109-113`);
`LiteralTypeMismatch { expected: format!("orderable type, got {}") }` with no literal
involved; `Display for ValueType` was Debug-as-Display while a private pretty renderer
(`AccountId@Account`) existed in parallel; `#![allow(clippy::result_large_err)]`.

**§16 Duplicate types.** `TypedVariable` ≡ `TypedInput` field-for-field;
`TypedTerm` = `TypedOperand` + Wildcard.

**§17 Trivia tests.** `v5_schema_label_differs_from_v4_label` asserted one string
literal ≠ another (`canonical.rs:165-171`); tests asserting derived PartialEq and
constants-return-constants; stamped `let Err(e) = r else { return }` rituals; **no**
canonical-bytes golden test, no injectivity test, no Bytes-literal test.

**§18 Clone-happy validation.** Every uniqueness check cloned owned Strings into
`BTreeSet<String>` on the success path (`validation.rs:22,42,60,143,243`).

## §20–28 Storage findings

**§20 Values stored 4–6×.** Every field value durably persisted in: `T` key
(`storage_format.rs:119`), `H` value (`:127`), `C` value (`:151`), `A` key (`:258`),
plus U/R guard keys. Each C cell: 17-byte key for 1–8 byte value; loading N fields = N
full prefix scans. The killer: `fact_bytes` was already a fixed-width row and
`EncodedFact::field_bytes` sliced any column in O(1) (`storage_v5_codec.rs:37-48`) —
one row-major scan would have replaced T-as-storage, H, P, and all of C.

**§21 Write amplification.** ~15 puts + ~6 gets per 3-field fact
(`storage_v5.rs:161-208`): T,H,L,P + C per field + **A per field unconditionally**
(`:193-203`) + guards + serial + **three hot-counter RMWs per fact** (`fact_count`,
`next_row_id`, `storage_tx_id`: `storage_v5_meta.rs:91-127`) + 3 puts per new interned
string. `bulk_load` = loop of single inserts (`storage_v5.rs:124-136`), monotonic keys
never used for append mode; post-load report did a full O(dict) scan to count entries.

**§22 Dead namespaces.** `H` written every insert (`storage_v5.rs:173-177`), read only
by `#[cfg(test)]` `debug_relation_facts`; `P` existed solely so delete could recover
row_id after the v6 re-key (`:250-261`); `T` key duplicated `H` value byte-for-byte;
`row_handles` loaded/cached in every image though every production use was `.len()`.

**§23 Accelerators half-used.** `A` maintained for every field of every relation on
every insert/delete, consulted only for Eq filters (`load.rs:216-225`); range filters
never used it despite value-ordered keys; matching row_ids fully materialized before a
`> 4096` magic cutoff discarded them (`load.rs:99-121`); accelerator-seeded loads
fabricated a constant column (filter value repeated N times, `load.rs:244-250`) then
re-ran the Eq filter against the fabricated data.

**§24 Filtered loads reintroduced point-gets.** Accelerator-seeded loads did one LMDB
get per candidate row per non-primary field (`load.rs:139-145`);
`load_selected_column_values_by_key` one get and one Vec per row (`load.rs:461-479`);
`load_live_handles_by_row_id` point-gets for handles nobody consumed;
`load_filter_primary_column` duplicated `load_column_values` minus validation and never
cached, so repeated filtered queries re-scanned while unfiltered loads hit cache.

**§25 4 KiB key memset.** `StorageKey` was `[u8; 4096]` zero-initialized per
construction (`storage_format.rs:45-61`); no bounds check (panic via slice index);
derived `Ord` compared `len` before bytes — disagreeing with LMDB byte order.

**§26 The cache that wasn't.** `BaseImageCache` documented "process-local," keyed by
fingerprint + tx-id (`base_image.rs:82-109,170-172`) — but constructed fresh inside
every `ReadTxn` (`lib.rs:180-186`), so the invalidation machinery guarded a cross-txn
cache that didn't exist; blake3 fingerprint recomputed per image request; O(n) linear
map lookups; a test pinned the cache as txn-local (`base_image_tests.rs:430-446`).
**v2's architecture doc had specified the cross-txn cache; v5 silently regressed it.**

**§27 Constraint copy-paste.** write/delete/check for unique and the three reverse-FK
functions were six-way copy-paste of one loop (`storage_v5.rs:263-398`); hand-sliced
5-byte prefixes duplicated across files.

**§28 Defensive hot paths.** Dictionary double-read per string (forward get + reverse
get + memcmp against blake3 collisions, `storage_v5_codec.rs:437-452`); three corruption
checks per cell per image build; per-fact `BTreeSet` allocation for field validation;
`Fact::new` O(F²); `FieldScope::insert` silently dropped field_id ≥ 256
(`base_image.rs:129-134`); blanket `#![allow(dead_code)]` on three modules;
`relation_base_image` dead in production.

## §30–40 Query-engine findings

**§30 Fake cost model.** Binary plan = `deterministic_binary_plan`: left-deep in user
clause order, no reordering (`planner.rs:209-230`). Scoring
(`planner_select.rs:454-468`): hardcoded base constants per derivation
(FilterAnchored 50 … BinaryDerived 1000) + `rows` (sum of ALL relations — identical
across candidates) + node count + projection width (identical). Stats theater: every
distinct estimate = `row_count.max(1)` (`:450-452`); `skew_ratio` computed, never read;
`accelerator_entries` hardcoded 0. This is why q09/q16/q24 burned ~32k binding conflicts
each before returning zero rows.

**§31 Fake vectorization.** `execute_vectorized_cover_loop`
(`runtime_vectorized.rs:59-117`) filled a 64-tuple batch then ran the identical scalar
path per tuple; each `fill_batch` constructed a fresh ~8.3 KB `TupleBatch` by value;
default `Scalar`; production consumer: only the bench runner; baseline recorded
`batches_yielded = 0`.

**§32 Tracing in hot loops.** `choose_cover` unconditionally pushed a heap-allocated
`CoverChoiceEvent{candidates: Vec}` per node entry per binding **in release builds**
(`cover.rs:68,97-102`); `QUERY_TRACING_ENABLED` included all debug builds
(`trace.rs:9`) so every probe/binding allocated a `format!` label + `TraceSpan`;
`PlanRewriteStep` carried before/after `format!("{plan:?}")` snapshots of a ~21 KB
structure per factorization attempt; explain emitted self-congratulatory constants.
~1,100 of ~6,500 production lines were trace/explain plumbing.

**§33 COLT force double-pass.** Two full passes over offsets — decode+hash to count,
then re-decode+re-hash to write into pre-reserved contiguous ranges
(`colt.rs:262-319`); scratch Vecs allocated per force despite the arena; the PRD had
flagged `child_counts` and the fix was never made.

**§34 Re-hash after enumerate.** Cover iteration enumerated map keys (`ght.rs:33-56`)
then `bind_cover_tuple` re-probed the same map for the child the entry already pointed
at (`runtime.rs:417-421`); `source_for` re-checked atom identity per probe per tuple
(`runtime_frame.rs:92-97`); per-key width re-verification (`runtime_keys.rs:18-23`).

**§35 Silent capacity cliffs.** `IdList::push` (cap 4), `SubatomList`, `NodeList` all
silently dropped elements past capacity (`compact.rs:35-40`); any atom with ≥5 variables
silently lost one and mis-failed downstream; `MAX_PLAN_NODES=16` broke singleton plans
at 17 variables; a default `NodeList` was a ~21 KB stack array copied by value through
candidate generation (`#[allow(clippy::large_stack_arrays)]` at `compact.rs:234`);
`EncodedValueSlot` errored >16 bytes at runtime instead of plan-time.

**§36 UnsafeCell aliasing UB.** `ColtSource::state_mut` handed out `&mut ColtState`
from `&self` (`colt.rs:334-341`); `try_for_each_vector_tuple` held `&ColtState`
references live across the user callback which re-entered `state_mut()`
(`ght.rs:141-190`) — violating the module's own safety comment; the iterated offsets
slice could dangle on arena reallocation. Would fail Miri.

**§37 Silent row loss.** Decode failures skipped rows silently: `continue` in `force()`
(`colt.rs:268-272`), `.is_ok() &&` filters (`ght.rs:157-163`), `let _ =` on batch push
(`ght.rs:206,217`) — corrupt data shrank query results instead of erroring.

**§38 Revalidation everywhere.** Full plan validation ran inside `factor_plan` after
every attempted subatom move (result discarded, `binary2fj.rs:167`), again in
`candidate()` (`planner_select.rs:271`), again at execution (`executor.rs:59`);
`binary2fj` invoked twice for two identical copies (`planner_select.rs:121-122`).

**§39 Checklist layering.** `FactorizedProjectionSink` = copy of `ProjectionSink` whose
only difference was incrementing `expansions_avoided` — factorized nothing
(`sink.rs:229-294`); `run.rs` 66-line argument forwarder; ~250 of executor.rs's 417
lines were 8 near-identical test harness wrappers; test-only modes (`PlanMode::Force*`,
`PredicateMode::ResidualOnly`, `CoverPolicy::StaticFirst`, `OutputMode::Factorized`);
dead arena API incl. `push_offset_to_child` with O(n²) growth (`arena.rs:285-311,
426-448`); ~785 lines of tests asserting instrumentation rather than semantics.

**§40 Cover-choice skew.** `key_count()` compared `Exact(distinct keys)` from forced
maps against `Estimate(offset count including duplicates)` from unforced vectors as the
same quantity (`ght.rs:99-106`, `cover.rs:106-110`) — systematically penalizing honest
exact counts and biasing toward duplicate-heavy vectors.
