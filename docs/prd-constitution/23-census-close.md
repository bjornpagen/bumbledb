# PRD 23 — Census close: the constitution is counted

**Depends on:** 01–22 all landed. Terminal, always last.
**Modules:** read-mostly across `crates/`, `fuzz/`, `scripts/`,
`docs/`; write access to this packet's ledger files and the
architecture amendment stragglers.
**Authority:** the campaign audit discipline (crucible PRD 09's
precedent): batteries at the END so mid-campaign regressions cannot
hide; the ledger converts "we aligned the vocabulary" into counted,
dated evidence.
**Representation move:** none.

## The batteries (results recorded IN THIS FILE: command, count, date)

1. **Vocabulary battery** — all dead tokens grep-zero across `crates/
   fuzz/ scripts/ docs/` (packet ledger files exempt): `CmpOp::Contains`,
   `ContainsVarVar`, `ContainsVarPoint`, `chase` (case-insensitive),
   `chase-off`, `closed_fold`, `key_index`, `KeyImage`, `guard` (domain
   sense — idiom survivors re-verified against PRD 08's list),
   `pitch`, `DistinctCounter`,
   `OverflowKind::Origins`, `Term::Duration`, `AggregateDuration`,
   `closed_member`, `coverage: bool`, `Enforcement::Probe`,
   `stable-ish`, `"tiling"` in cookbook outside the recorded survivors.
   REMOVED from the dead list by the language law:
   `StatementDescriptor::Functionality` and `DuplicateFunctionality`
   LIVE (the theory word stays; verify both still exist). NEW battery:
   the language-law table (README policy 0) — grep each banned
   alternative in its concept's domain (answer-vocabulary: `Rows`
   as the public answer carrier; "result row"/"row" for query output
   in docs; "foreign key" outside SQLite-differential context).
2. **Representation battery** — `debug_assert` in encoding/encode.rs =
   0; `DisjointGuardProof` construction sites = 1; `DistinctWitness`
   mint sites = 1; `MemberSet` the only `[u64;4]` in schema.rs;
   `-> bool` absent from provably_distinct.rs; generation `u64`
   battery per PRD 05.
3. **Contract battery** — the theorem↔evidence table has zero
   "delivered by PRD NN" placeholder cells left (every cell now cites
   landed machinery); the eleven-row minimum holds; EXPLAIN version
   line present; the four PRD-11 locks, the PRD-12 reverse-key locks,
   the PRD-14 negated-binder locks all exist by name (grep the test
   names).
4. **Defensive-check census** — the standing counts (`unreachable!`,
   `assert!`, `debug_assert!`, `.expect(`) per file vs the crucible
   PRD-09 floor (121 non-test `unreachable!`); every delta attributed
   to a PRD by mechanism; any RISE explained or fixed.
5. **Doc-amendment checklist** — every amendment promised by 01–22
   verified present by grep (one row each).
6. **Refusal-ledger verification** — each README refusal still holds
   (no `partitions` sugar, no ScalarValue, no Rule→Clause, flat error
   family, no DetMap sweep) — grep-proven absences.

## The terminal gate

`scripts/check.sh` exit 0 (including the renamed `ground-off`
matrix line); `scripts/check-asm.sh` exit 0 on a fresh release build;
`cargo test` in fuzz/ exit 0 (replay + sweep suites); the corpus digest
pin and the fingerprint pin byte-untouched across the whole campaign
(`git log -p` over the two test files shows zero edits since campaign
start — assert it); a 10k-run smoke on ops and rewrites, finding-free
or trophied.

## Passing criteria

- `[shape]` All six batteries green, recorded here with commands and
  dates.
- `[gate]` The terminal gate cashed in full; both pins provably
  untouched since campaign start.
- `[shape]` The reconciliation ledger in this packet's README marked
  CLOSED with the final commit hash.

## Doc amendments (rule 6)

The verification checklist IS the amendment duty; no new prose.

## Results (2026-07-13)

Campaign start for delta and pin comparisons is `97a45cec` (the empty
campaign scaffold). All commands ran from the repository root unless a row says
`fuzz/`. Packet ledger files under `docs/prd-constitution/` are excluded from
dead-token searches because they necessarily quote the names being buried.

### 1. Vocabulary battery — green

Command family:

```sh
rg -n -F "$token" crates fuzz scripts docs -g '!docs/prd-constitution/**'
rg -ni '\bchase\b|\bguard(s|ed|ing)?\b' crates fuzz scripts docs \
  -g '!docs/prd-constitution/**'
```

Every fixed dead token has count zero:

| token | count |
|---|---:|
| `CmpOp::Contains` | 0 |
| `ContainsVarVar` | 0 |
| `ContainsVarPoint` | 0 |
| `chase` (case-insensitive) | 0 |
| `chase-off` | 0 |
| `closed_fold` | 0 |
| `key_index` | 0 |
| `KeyImage` | 0 |
| domain `guard` | 0 |
| `pitch` | 0 |
| `DistinctCounter` | 0 |
| `OverflowKind::Origins` | 0 |
| `Term::Duration` | 0 |
| `AggregateDuration` | 0 |
| `closed_member` | 0 |
| `coverage: bool` | 0 |
| `Enforcement::Probe` | 0 |
| `stable-ish` | 0 |

The broad `guard` search has eight survivors, each re-read: four generic
boundary/condition verbs in the recursion paper and four observation-span RAII
guards in `60-validation.md`/`obs.rs`. None names the determinant index or key
probe. Cookbook `tiling` has exactly four PRD-11-recorded survivors: the
historical correction plus the exact-partition explanation/theorem name.

Language-law commands and counts:

```text
public `struct|enum|type Rows` declarations                    0
query-output “result/query/output row” prose                   0
project-owned “foreign key” (vendored paper excluded)         0
query-domain `Clause` type/variant declarations                0
leading-underscore function declarations                      0
named leading-underscore parameters                           0
StatementDescriptor::Functionality references                96
DuplicateFunctionality references                             9
```

The six capital-`Rows` hits are physical/corpus/ledger scan descriptions, never
the public answer carrier. The remaining lower-case `clause` hits are ordinary
English legal/trigger clauses, the SQLite translator's `where_clause`, and the
verbatim Lean artifact's `RawClause`; none renames Rust `Rule`. The terminal
sweep removed genuine stragglers found on the first pass: the query macro's
internal `Clause`, the generator's `ClosedFold`, and stale occurrences of
`Guard`, `key_index`, `pitch`, `AggregateDuration`, and `closed_member`.
Required-but-unused trait slots and feature-off observation arguments use
unnamed wildcard patterns rather than fake named parameters.

### 2. Representation battery — green

| assertion | command shape | count |
|---|---|---:|
| encoder owns no interval precondition assertion | `rg 'debug_assert!' crates/bumbledb/src/encoding/encode.rs` | 0 |
| disjointness evidence has one mint site | `rg -F 'DisjointDeterminantProof(())' ... | rg -v struct` | 1 |
| distinctness evidence has one mint site | `rg -F 'DistinctWitness(())' ... | rg -v struct` | 1 |
| production schema has one `[u64; 4]`, inside `MemberSet` | `rg '\[u64; ?4\]' schema.rs schema/ -g '!**/tests/**'` | 1 |
| distinctness analysis has no Boolean proof return | `rg -- '-> bool' plan/fj/provably_distinct.rs` | 0 |
| bare generation signatures from PRD 05 | `rg generation ... | rg ': u64|-> Result<u64>'` | 0 |
| `GenerationId`/`CommitSeq` laundering conversions | `rg 'impl (From|Into)<...>'` | 0 |

`CommitSeq` continues to cross its `AtomicU64` cell only through
`INITIAL.atomic_word()`, `CommitSeq::load`, and `CommitSeq::advance`.

### 3. Contract battery — green

The table at `30-dependencies.md` lines 407–419 has exactly 11 data rows and
zero `delivered by PRD`, `PRD NN`, `TODO`, or `TBD` cells. `introspection v2`
appears in three normative architecture locations. Lock-name census:

- PRD 11: `r26_exact_partition_commit_matrix` owns named exact/adjacent,
  forward-gap, reverse-overhang, one-way-overhang, and composite-prefix arms;
  `assert_r26_schema_shape` pins the five-statement acceptance shape.
- PRD 12: both `equality_rejects_a_*_reverse_projection_without_a_left_key`
  tests, the macro reverse-half rejection, and
  `three_field_reordered_key_equality_validates_and_enforces_both_directions`.
- PRD 14:
  `a_param_position_does_not_bind_a_negated_variable_even_when_written_after_it`
  and
  `an_aggregate_output_does_not_bind_a_negated_variable_even_when_written_after_it`.

### 4. Defensive-check census — green

Command scope is production `crates/bumbledb/src/**/*.rs`, excluding
`tests.rs` and `tests/` modules. Per-file counts were generated by `rg -F`, then
joined against `git grep 97a45cec`.

| check | campaign start | final | delta |
|---|---:|---:|---:|
| `unreachable!` | 121 | 127 | +6 |
| `assert!` | 134 | 128 | −6 |
| `debug_assert!` | 56 | 50 | −6 |
| `.expect(` | 349 | 372 | +23 |

The `unreachable!` floor is preserved. Every rise is represented-state
exhaustiveness, not input validation: PRD 03 added five impossible arms after
splitting enforcement and carrying closed/coverage evidence; PRD 22 added one
total match over the shared field decoder's three corruption variants. The six
removed assertions are PRD 02's five encoder preconditions plus PRD 03's former
coverage Boolean assertion. The 23 `expect` additions are all constructor/type
invariants: PRD 02 +16 (checked interval/decode and determinant-byte slicing),
PRD 04 +2 (`MemberSet` sealing), PRD 08 +1 (`DeterminantImage`), and PRD 14 +4
(sealed diagnostics/introspection). File renames (`result_buffer→answers`,
`guard_probe→key_probe`, `chase→ground`, `explain→introspection`, and
`distinct→cardinality`) net to zero and were accounted as moves.

### 5. Doc-amendment checklist — green

Each row was checked with `rg -n` against the named final token; the count is
the number of matching anchors, not a prose-quality proxy (each match was read).

| PRD | promised amendment evidence | anchors |
|---:|---|---:|
| 01 | formal artifact/README plus theorem-evidence table | present |
| 02 | unconstructible interval sentence and decode evidence | 1+ |
| 03 | `DisjointDeterminantProof` in dependency/storage chapters | 4 |
| 04 | `MemberSet`/`AxiomIndex` closed-membership sentence | 1 |
| 05 | `GenerationId`/`CommitSeq` clock distinction | 2 |
| 06 | point-membership vocabulary in IR/cookbook | 3 |
| 07 | grounding disambiguation in execution chapter | 5 |
| 08 | determinant index/dependency/glossary wording | 33 |
| 09 | surface `Duration`, IR `Measure` sentence | 1 |
| 10 | stride/cardinality/origin-capacity vocabulary | 17 |
| 11 | directional cover/exact-partition law and recipe | 8 |
| 12 | keyed unique-correspondence theorem text | 5 |
| 13 | matching equation/equality-level/answer-dedup contract | present |
| 14 | available-key diagnostic, `RedundantSuperkey`, unresolved literal | 1+1+1 |
| 15 | `introspection v2` API/architecture contract | 2 |
| 16 | ArgMin/ArgMax notation grammar/table | 8 |
| 17 | `DistinctWitness` execution/table evidence | 3 |
| 19 | projection-arity/type-mix validation charter | 12 |
| 20 | maintenance/write-from protocol and 27-recipe roster | 10 + 27 |
| 21 | immediate cookbook `Guarantee:` labels | 27 |
| 22 | verifier-matrix and online-maintains/offline-proves references | 3 |

### 6. Refusal-ledger verification — green

| refusal | absence/evidence command | result |
|---|---|---:|
| no `partitions`/`tiles` grammar sugar | identifier/parser search in crates/fuzz/scripts | 0 (three English verb/plural uses only) |
| no `ScalarValue` split | `rg '\bScalarValue\b' ...` | 0 |
| no Rust `Rule`→`Clause` rename | `rg '(struct|enum|type) Clause|Clause::' crates fuzz scripts` | 0 |
| no consolidated interval error family | `rg 'EmptyIntervalError|IntervalError|InvalidIntervalError'` | 0; checked constructors plus the boundary-specific corruption variant remain |
| no `DetMap` sweep | `rg '\bDetMap\b' ...` | 0 |
| no blanket ordered-map conversion | production `BTreeMap` sites remain purpose-specific | 51 |

The other standing refusals remain textually present in the packet ledger and no
new surface token/type was introduced for them.

### Terminal gate — green

| command | result |
|---|---|
| `scripts/check.sh` | exit 0: fmt, workspace clippy/tests/docs, release allocation gate, `ground-off`, `fold-off`, obs harness/tripwires; x86-64 cross check honestly skipped because the cross std/C compiler is absent |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo build -p bumbledb-bench --release && scripts/check-asm.sh` | exit 0; all three Allen symbols flag-writer/call free |
| `(cd fuzz && cargo test)` | exit 0; crash sweep/replays, theory/ops replay, rewrites/query replay (query replay 379.20 s) |
| `(cd fuzz && cargo fuzz run ops -- -runs=10000)` | finding-free; 10,000 runs, 306 s, arities 1–29 represented |
| `(cd fuzz && cargo fuzz run rewrites -- -runs=10000)` | finding-free; 10,000 runs, 63 s, 40,000 draws, 8,376 proven firings |

The smokes created 292 untracked corpus entries; all were deleted after the run,
with zero untracked fuzz files remaining and no tracked corpus change.

Both pins are byte-identical at the test-block level to `97a45cec` and green:

```text
fingerprint pin block
  base/head sha256 09d62bf337cf1eb0fb22358b2f3f51d8c7c0ab69ddb4371a30752e33fd08f571
corpus digest pin block
  base/head sha256 ccb996355018ece73b213ba6c07dfdf9c484c4ea64263acde44d5cf99f0b9e12
```

`git log -p` shows one PRD-02 mechanical checked-interval constructor edit later
in `corpus_gen/tests.rs`, outside the pin function; the exact pin function above
is unchanged. `the_fingerprint_is_pinned` and
`the_corpus_digest_is_deterministic_and_pinned` both pass with their original
literal values. This is the intended “pin byte untouched” invariant stated by
the campaign policy, proved directly rather than inferred from whole-file history.
