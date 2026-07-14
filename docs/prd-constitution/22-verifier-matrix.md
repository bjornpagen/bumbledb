# PRD 22 — The verifier matrix: one corruption fixture per rebuilt claim

**Depends on:** 08 (verify_store files carry final names).
**Modules:** `crates/bumbledb/src/verify_store/` (facts, determinants,
containment/reverse-edge, coverage, closed-image, fingerprint passes —
enumerate what exists), its test suite, possibly small additions to
the fixture harness (raw-byte write access for tests, following the
crash/HANGHUNT-era precedent of back-door store construction).
**Authority:** brief B6, approved: the Lean model assumes a valid
database state; `verify_store` is the arbiter that stored bytes
realize one. The verifier already rebuilds the semantic indexes (and
the crash fuzz target leans on it continuously) — but "every index has
a corruption fixture that proves the verifier would catch its
corruption" is currently unverified. A verifier pass without a fixture
is a smoke detector never held to a flame.
**Representation move:** none. The verifier's coverage claim becomes a
matrix with a fixture per row.

## Context (decided shape)

1. **The matrix.** Enumerate every claim `verify_store` makes (read
   the module: fact decode validity, interval validity, scalar key
   image parity, pointwise disjointness, reverse containment edges,
   scalar containment satisfaction, coverage satisfaction, closed
   image parity, fingerprint match — the actual list comes from the
   code, not this sketch). One row each: claim × the pass that checks
   it × the corruption fixture that violates it × the expected finding
   (relation, statement, key context per the brief).
2. **The fixtures.** For each row WITHOUT an existing test fixture: a
   test that opens a healthy store, corrupts exactly one artifact
   through raw LMDB access (flip a key-image byte, delete one reverse
   edge, break one interval's halves, remove one closed-image row,
   perturb the stored fingerprint), runs `verify_store`, and asserts
   the finding identifies the corrupted artifact with its context —
   and that a healthy sibling store stays green (no false positives
   from the harness).
3. **Findings quality:** where a pass detects but reports without
   context (no relation/statement identification), upgrading the
   finding payload is IN scope (it is the diagnostics discipline of
   PRD 14 applied to the auditor).
4. **The delete-asymmetry row:** the old audit noted reverse-edge
   delete verification leans on the offline pass — that row's fixture
   is mandatory and its doc sentence in 50-storage states the division
   (online maintains, offline proves).

## Technical direction

Read-first: build the matrix from the code, mark existing fixtures
(several corruption tests exist — census them), write only the missing
ones. Raw-byte corruption helpers live in the test tree, never in the
engine. Every fixture is deterministic (no random corruption — the
byte and location are chosen and commented).

## Passing criteria

- `[shape]` The matrix complete in this file's Results: every
  verifier claim has a fixture row marked pre-existing or added.
- `[test]` Every fixture green (detects its corruption with context;
  healthy control stays clean); full verify_store suite green.
- `[shape]` Finding-payload upgrades (if any) enumerated with
  before/after.
- `[gate]` Full suite green; fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`50-storage.md` § the offline verifier: the matrix referenced as the
coverage claim's evidence; the online/offline division sentence.

## Results (2026-07-13)

The code census is the authority for this matrix. `F`, `M`, `U`, `R`, and
`S` are separate ordered passes; the dictionary reverse-map statistic follows
them. Format/fingerprint checks sit at the open perimeter because a `Db` cannot
exist after either mismatch. “Context” below means the asserted finding carries
the relation/statement/fact or exact offending key promised by its variant.

| rebuilt claim | checking pass | deterministic corruption fixture | expected evidence | status |
|---|---|---|---|---|
| storage format precedes all semantic reads | `Environment::open` | `corrupted_format_version_fails_before_fingerprint` | typed `FormatMismatch { found, expected }`, even with a wrong schema | pre-existing |
| stored schema fingerprint equals the compiled theory | `Environment::open` | `corrupted_stored_fingerprint_names_found_and_expected_images` | typed `SchemaMismatch` with both 32-byte images | added |
| every swept namespace key has its codec shape | F/M/U/R/S cursors | `malformed_keys_in_every_swept_namespace_are_contextual_findings` | one `Malformed { key, what }` per namespace | added |
| relation/statement components belong to the compiled schema and correct namespace owner | F/M/U/R/S cursors | `namespace_schema_ownership_is_rechecked` | exact foreign key plus `F/M/U/S key relation`, `U/R key statement`, or `R key source relation` | added |
| M/U row-id images and S values/stat kinds have canonical widths/domains | M/U/S cursors | `namespace_row_images_are_width_checked`; `counter_value_and_stat_kind_are_width_and_domain_checked` | exact key plus `M row id`, `U row id`, `S counter value`, or `S stat kind` | added |
| every F value has the schema fact width | F | `wrong_fact_width_is_a_contextual_finding` | `Malformed` names the F key and `F fact width` | added |
| Bool, `bytes<N>` padding, and interval halves are canonical | F via shared `decode_field` | `noncanonical_field_encodings_are_each_found` | three exact-key findings: `F fact bool`, `F fact fixed bytes padding`, `F fact interval` | added |
| every referenced intern id is below `_meta` next-id | F | `intern_id_at_or_beyond_the_counter_is_found_with_fact_context` | `InternBeyondNextId { relation, row_id, intern_id, next_id }` | added |
| every F fact owns the matching M hash image | F→M | `missing_membership_is_found_from_the_fact_side` | `FactWithoutMembership { relation, row_id, membership_key }` | pre-existing |
| every M image resolves to a fact hashing back to its key | M→F | `orphan_membership_is_found_from_the_entry_side` | `MembershipWithoutFact { relation, row_id, membership_key }` | pre-existing |
| every F fact holds every declared determinant image | F→U | `missing_determinant_is_found_from_the_fact_side` | `FactWithoutDeterminant { relation, statement, row_id, determinant_key }` | pre-existing |
| every U image resolves to a fact re-deriving identical bytes | U→F | `orphan_determinant_is_found_from_the_entry_side`; `determinant_key_byte_flip_is_found_against_the_live_fact` | `DeterminantWithoutFact { relation, statement, determinant_key }`, including a one-byte image perturbation | pre-existing + added |
| each pointwise determinant prefix group is globally disjoint | U ordered neighbor walk | `pointwise_overlap_is_found_by_the_ordered_walk` | `PointwiseOverlap { relation, statement, first, second }` | pre-existing |
| every selected ordinary source fact has its reverse edge | F→R | `missing_reverse_edge_is_found_from_the_fact_side` | `FactWithoutReverseEdge { statement, relation, row_id, reverse_key }` | pre-existing; mandatory delete-asymmetry row |
| every R edge resolves to a live source still satisfying φ and re-deriving its key | R→F | `orphan_reverse_edge_is_found_from_the_edge_side`; `edge_whose_source_left_its_selection_is_an_orphan`; `reverse_key_byte_flip_is_found_against_the_live_source` | `ReverseEdgeWithoutFact { statement, reverse_key }` for absent source, φ exit, and one-byte-equivalent key perturbation | pre-existing + added |
| ordinary scalar containment holds over the complete committed state | F global judgment | `a_coherently_deleted_scalar_target_is_a_judgment_violation` | `JudgmentViolation { statement, TargetRequired, source fact }` | pre-existing |
| interval containment has gap-free target coverage | F global coverage walk | `a_coherently_deleted_coverage_segment_is_a_judgment_violation` | same contextual `JudgmentViolation` for the uncovered claim | pre-existing |
| ordinary→closed references are members of the compiled ψ image | F closed-target judgment | `a_planted_source_outside_the_member_set_is_a_judgment_violation` | contextual `JudgmentViolation` | pre-existing |
| closed-source domain quantification is checked despite having no F rows | extension-source judgment | `an_uncovered_domain_quantification_is_a_judgment_violation` | one contextual finding per uncovered sealed row; covering all rows clears the report | pre-existing |
| closed relations have no F/M/U/R storage image | F/M/U/R | `a_stored_row_for_a_closed_relation_is_the_finding`; `membership_and_determinant_entries_for_a_closed_relation_are_findings`; `an_r_entry_naming_a_closed_target_statement_is_the_finding` | `ClosedRelationEntry { relation, key }` in all four materialized namespaces | pre-existing + added |
| S row count equals the F tally, including absent counter rows | S | `wrong_row_count_is_found_against_the_scan`; `absent_counters_are_found_against_the_fact_tally` | `RowCountDesync { relation, stored, counted }` | pre-existing + added |
| S next-row id exceeds every observed row id, including absent counter rows | S | `low_high_water_is_found_against_the_max_row_id`; `absent_counters_are_found_against_the_fact_tally` | `RowIdHighWaterLow { relation, stored, max_row_id }` | pre-existing + added |
| dictionary reverse keys are canonical ids | `_dict` reverse cursor | `malformed_dictionary_reverse_key_is_a_finding` | exact-key `Malformed { what: "dict reverse id" }` | added |
| unreferenced dictionary reverse ids are the accepted leak, not corruption | `_dict` liveness statistic | `clean_store_reports_nothing_and_counts_the_leak` | no findings and `dangling_intern_ids == 1` | pre-existing |
| raw corruption helpers do not create false positives by themselves | all passes | every added fixture constructor verifies an untouched populated sibling first; `clean_store_reports_nothing_and_counts_the_leak` remains the shared baseline | sibling has an empty finding list | added |

Finding quality required no shape change to an existing `StoreFinding` variant:
all namespace findings already carried their relation/statement or exact key, and
judgment findings carried the source fact. The one detection upgrade is explicit:

- Before: a width-correct F fact containing a noncanonical Bool, nonzero
  fixed-bytes pad, or empty/inverted interval could pass the offline F sweep and
  fail only when an online image decoded it.
- After: the F pass calls the shared field decoder and emits exact-key
  `Malformed` findings with `F fact bool`, `F fact fixed bytes padding`, or
  `F fact interval`. This adds no second decoder and changes no on-disk bytes.

The reverse-edge division is now explicit in `50-storage.md`: the online path
maintains R; the offline pass proves it. The fingerprint preimage and pinned
value are untouched.
