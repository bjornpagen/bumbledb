# Spec-fidelity review 07 — `Exec/Dedup.lean` ↔ the seen-sets, the union regime, and the witness spends

**Pairing:** #7 (PRD 15). **Normative side:** `lean/Bumbledb/Exec/Dedup.lean`.
**Rust surface:** `crates/bumbledb/src/exec/sink.rs`, `exec/sink/aggregate/{new,fold_row}.rs`,
`exec/sink/projection/sink.rs`, `plan/fj/provably_distinct.rs`, `plan/fj/provably_disjoint.rs`,
`api/prepared/build.rs`, `api/prepared/introspect.rs`.

## Per-theorem fidelity table

| Lean statement | Implementing code | Verdict |
|---|---|---|
| `seenFold` / `seenfold_is_set_semantics` (Dedup.lean:126-254) | `WordMap` insert-if-absent: projection `seen.insert` (exec/sink/projection/sink.rs:14,61,157,169; field exec/sink.rs:251), aggregate seen path (exec/sink/aggregate/fold_row.rs:36-45); reset once per execution, never per rule (exec/sink/aggregate/new.rs:343-352; Dedup.lean:134-135) | FAITHFUL — first-occurrence filtering, span-wide seen-set, no merge node exists |
| `Term.pins` (Dedup.lean:266-281) | pinned-field screen: `vars` ∪ `Eq`-to-constant filters (provably_distinct.rs:42-58); sets excluded (rs:28-31, catch-all rs:57) | FAITHFUL for `var`/`param`/`lit`(Word/Byte/Interval/PendingIntern)/`paramSet`/`measure`; see D1 for `Const::Words` |
| `BoundFieldsCoverKey` / `CoversKey` (Dedup.lean:304-316) | key coverage over declared keys (provably_distinct.rs:60-66); participation screen `role.participates()` = `Positive` only (rs:39; ir/normalize.rs:103-105) | FAITHFUL — negated bind nothing (model: `r.atoms` is the positive list); eliminated/folded discharge is PRD 08's (recorded, rs:17-20 / Dedup.lean:57-60) |
| `distinct_witness_licence` + countermodel (Dedup.lean:389-421; Countermodels.lean:764-786) | only mint `provably_distinct` (rs:32-69, private-field unit `DistinctWitness(())` rs:11); only elided entry `without_seen_set` requires the witness by value (new.rs:137-145); regime sum `DedupRegime` (new.rs:363-368); invariant `seen.is_none() == distinct_witness.is_some()` (new.rs:308-314); single-rule-only spend (build.rs:105-108) | FAITHFUL — countermodel exists and `provably_distinct` refuses the unkeyed rule (Countermodels.lean:768) |
| `union_regime_head_projection` (Dedup.lean:520-541) | `union_spans`/`union_key_spans` (new.rs:379-399), key assembly `dedup_key` (fold_row.rs:167-182), mandatory for every multi-rule sink (`for_union`, new.rs:131-133; build.rs:105-106) | FAITHFUL — head-shaped, rule-independent key; see D3 for the nullary-Count absence |
| `disjoint_witness_licence` (Dedup.lean:491-503) | mint build.rs:56,630-645; gated to >1 surviving rules build.rs:91; stored api/prepared.rs:179; read ONLY by introspect.rs:242,294-306; `make_sink` never consults it (build.rs:667-675); refutation cited exists (docs/architecture/40-execution.md:273) | FAITHFUL — spent diagnostically only, exactly as the Lean records |
| `ArmPin` / `ProvablyDisjointRules` / `syntactic_disjointness_sound` (Dedup.lean:572-633) | `provably_disjoint_rules` (provably_disjoint.rs:46-73), `pair_disjoint` (rs:78-90), `pinned_fields` (rs:112-121), `provably_different` (rs:126-145), `key_flows_to_common_head` (rs:154-172), `head_reads` (rs:188-203) | FAITHFUL per pair; see D2 for the program-level key quantifier |

Every file:line citation embedded in Dedup.lean's module doc was re-verified against the sources and is accurate (rs:11, 17-20, 28-31, 32-69, 42-45, 46-58, 60-66; disjoint rs:46-73, 78-90, 112-121, 126-145, 154-172, 188-203, 162; sink.rs:6-18, 384-388, 390-398; new.rs:138). Bridge oracle tests all exist (sink/tests/projection.rs:99, sink/tests/semantics.rs:70, api/prepared/tests/aggregates.rs:88, api/prepared/tests/disjoint.rs:98, sink/tests/aggregate.rs:714, api/prepared/tests/rules.rs:176).

## Divergences

### D1 — class (b): `Const::Words` is excluded from the pinned-field screen; the recorded reading is silent on it
- Rust: the `Eq`-pin arm admits `Word | Byte | Interval | Param | PendingIntern` (provably_distinct.rs:50-56); `Const::Words` — the multi-word `bytes<N>` literal, a genuine single-value pin compared word-wise under `Eq` (image/view.rs:38-43) — falls to the catch-all (rs:57) and never counts toward key coverage.
- Lean: `Term.pins` marks every `lit` as pinning (Dedup.lean:267), and the recorded reading attributes the exclusions to sets only ("EXCLUDES sets: set-bound fields pin nothing", Dedup.lean:54-55).
- Effect: conservative in the sound direction only (fewer `DistinctWitness` mints → seen-set retained). Not a bug; an unrecorded narrowing of the mint. Notably asymmetric: `provably_different` DOES compare `Const::Words` payloads (provably_disjoint.rs:137).

### D2 — class (b), with a class (c) recorded-reading overclaim: the Lean fixes ONE key `K` program-wide; the code chooses a key per rule pair
- Lean: `ProvablyDisjointRules q R fld K` quantifies a single `K` over every pair (Dedup.lean:584-586), and `syntactic_disjointness_sound` takes one `Functionality (I R) K` hypothesis (Dedup.lean:628-633). The module doc claims "`ProvablyDisjointRules` models exactly this rule" (Dedup.lean:80).
- Rust: `key_flows_to_common_head` existentially picks any declared key of `R` inside `pair_disjoint` (provably_disjoint.rs:162, invoked per pair at rs:66-72), jointly with the occurrence pair — different pairs may be discharged by different keys.
- Effect: acceptances using heterogeneous keys across pairs fall outside the theorem's statement (semantically still sound — every declared key holds on committed instances via PRD 03/09 — but the theorem as proved does not cover them). The "exactly" claim is the (c)-flavored part. Diagnostic-only stakes: the witness is never spent by execution.

### D3 — class (b): the nullary `Count`'s key-column absence is unmodeled in theorem 5
- Rust: `union_span` maps `Agg { over_slot: None }` to `None` — the nullary Count contributes no words to the union dedup key (new.rs:388-390; the Rust doc calls it "the naive model's constant filler, represented as absence", new.rs:373-378).
- Lean: theorem 5's finds are all `VarId`s (`(rule e).finds.map (bind e)`, Dedup.lean:527); a keyless head position is unrepresentable, and Dedup.lean's union bridge note (Dedup.lean:42-48) does not record the absence.
- Effect: sound (omitting a constant column never changes key equality); a modeling-vocabulary gap, not a behavior gap. Same vocabulary note applies to `Pack`'s raw-claim span and the derived measure word — both are head-projection values, consistent with the theorem's reading.

## Adversarial readings performed (no divergence found)

- **D2 suffix-skip evidence pair:** the projection sink reports staleness on FIRST emit (projection/sink.rs:15-23); legality lives in the executor's per-node sink-relevance bits (run.rs:26-29, 105-123), aggregates never skip and are skip-absorbing (sink.rs:296-301). Outside Dedup.lean by recorded narrowing #1 (event arrival is mechanism); the Bridge oracle test covers the pair.
- **Multi-rule fold regimes:** `rules.len() > 1` forces `SinkProgram::Union` unconditionally (build.rs:105-108) — a per-rule `DistinctWitness` can never elide a multi-rule seen-set; `aim` rebuilds slot tables but carries the shared maps (new.rs:267-295); subsumption/death shrinking to one rule correctly re-enters the single-rule regime with that rule's own witness.
- **`dedup_key` under union vs single:** single-rule keys the whole `binding_scratch` including derived measure words (fold_row.rs:180 — an injective extension of the model's `slots.map σ`, sound); union keys exactly the head-projection spans (fold_row.rs:172-179).
- **Witness portability:** `DistinctWitness` is `Copy` with no rule identity; the spend is guarded structurally (the single-rule arm uses that rule's own mint, build.rs:108; per-rule mint at build.rs:326). No misuse path exists today.

## GRADE: A−

No class (a) finding survived adversarial reading: every elision is witness-gated exactly as the licences demand, the union regime's key is head-shaped and mandatory, the `DisjointWitness` is provably diagnostic-only, and every one of the Lean's recorded citations checks out against the sources line-for-line. The deductions are three benign gaps: one unrecorded conservative narrowing of the distinct mint (D1), one theorem whose program-level key quantifier is narrower than the code's rule together with an "exactly" overclaim in the recorded reading (D2), and one modeling-vocabulary absence for keyless head positions (D3). All three are sound-direction; none changes an answer.
