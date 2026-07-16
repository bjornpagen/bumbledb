# TODO — the road to 1.0.0 (handoff PRD)

**Status:** this is a handoff. Nothing below is done yet. It captures the entire
agreed plan to drive bumbledb to a zero-known-issues state so the owner can
release 1.0.0 at will.

**What 1.0.0 means here (owner-ruled):** the known-issues list is *empty* — not
triaged, not ranked, empty. Every known defect fixed, every measured-but-unclaimed
win claimed or refuted, every unexplained behavior explained, every OPEN-ledger row
fixed/refuted/fired/declined. This work is **necessary but not sufficient**: the
only thing that releases 1.0.0 is the owner's personal decision. This document
clears the floor beneath that decision; it does not make it.

---

## 0. Where things stand (2026-07-16)

- **Main is at `ee5d1de4`** ("the bindings contract: SchemaSpec parity, the dyn
  surface, violations rendered as data"), pushed. The layer-law perf campaign
  (T1/T5/T6/T7 wins + T2/T3/T4/T8 gravestones + T9 displaced lanes + alloc census
  harness) landed before it. ALL-WIN re-earned, all five README charts current.
- **`docs/reports/` and `docs/brainlift-sources/` were PURGED** (owner ruling:
  reports was never a sanctioned normative home — a drift-prone projection of the
  code at one instant; the only authorities are the code, the lean spec, and the
  normative docs under `docs/architecture/`). The layer-law campaign record,
  phase-R/R6 record, and ephemeral characterization now live **only in git
  history** (recover with `git show <sha>:docs/reports/...`; the campaign record
  was `docs/reports/perf-campaign-layer-law.md`, committed then purged around
  commit `3e0f1662`). The improvement specs below re-embed everything needed so no
  archaeology is required.
- **The rebirth workflow is in flight** in primer (`graph-builder-rebirth` branch):
  rebuilding graph-builder's entire working state on bumbledb. Engine + SDK PRDs
  01–08 are done; driver/ETL/TUI/supervisor/funeral/verify remain. Its FFI builds
  against bumbledb's working tree pinned at `ee5d1de4` via a snapshot copy at
  `../primer/.claude/worktrees/bumbledb`.
- **The TypeScript SDK exists**: `../primer/.claude/worktrees/rebirth/packages/bumbledb`,
  currently named `@superbuilders/bumbledb`, `private: true`, ~6k lines, 65/65
  tests. It is a real type-theoretic kernel (brands, `relation()`, `closed()`,
  `schema()`, the five-constructor `count` vocabulary enforcing the ban table
  representationally, `Db` runtime with rejection-as-data, query surface as
  Datalog values). The raw napi bridge (`crate/src/{lib,marshal}.rs`) is the dumb
  schema-directed marshaler; names resolve TS-side via the manifest, ids cross the
  FFI.

### Hard sequencing constraint

**Do NOT merge the campaign into `main` until the rebirth's PRD-18 lands green**
(funeral done, primer `pnpm typecheck`/`pnpm knip` green). The rebirth builds
against main's working tree; advancing main mid-run breaks its gates. All campaign
work accumulates on a branch in a dedicated worktree; main stays pristine at
`ee5d1de4` until rebirth-green AND owner-go. (Docs-only commits like this TODO are
safe — they don't touch cargo/gates.)

---

## 1. Execution discipline (applies to every phase)

- **Worktrees, always.** Main stays open for parallel real work. Create an
  integration worktree (e.g. `git -C <repo> worktree add
  ../bumbledb-worktrees/campaign-1.0.0 -b campaign/zero-known-issues`); every
  fan-out agent gets its own worktree; assemble/merge on the campaign branch, never
  in main's working tree.
- **Measurement law (the M2 Max ledger's own discipline — violating it voids a
  verdict).** All timing through `scripts/measure.sh` (the machine-wide mutex).
  Absolute numbers under co-tenancy are VOID; only **interleaved same-session A/B**
  ratios are valid (±2% band). Fresh data per rep (TAGE memorizes benchmarks, 4.7×
  bias). Trace counts, never assume them. **Disassembly-gate** every codegen claim
  (`scripts/check-asm.sh`; LLVM substitutes memcpy/autovectorization/reassociation).
  State the tier (L1/L2/DRAM/displaced) of every number. The judgment layer is
  `docs/reference/apple-silicon-performance.md`.
- **The landing bar for any perf/behavior twin ("clear win"):** (1) semantics
  untouched — `cargo test -p bumbledb` + conformance + naive-parity + lean
  three-way green for anything touching execution; lean/ untouched unless the item
  is a lean item; (2) predicted sign outside the ±2% band at the regime that
  matters (family-level ≥5%, or a kernel win with demonstrated family neutrality);
  (3) no other family loses >2% (interleaved spot-check); (4) fmt + clippy -D +
  check-asm if kernels moved + alloc gate if hot paths moved. A LOSS/NEUTRAL twin
  lands NOTHING but its **gravestone** (the experiment + numbers + a recorded
  paragraph) — a measured refutation empties a known-issue line as legitimately as
  a fix.
- **Adversarial review** of every claimed win before merge: an independent agent
  re-runs the falsifier, re-checks semantics, re-reads the disassembly, audits new
  `unsafe`. Merge gates block on majors.
- **Line numbers below have DRIFTED** (the bindings commit + doc edits moved things
  since the campaign). Locate every target by symbol/behavior with grep; treat the
  cited lines as approximate.
- **Push discipline:** push each ready commit to origin immediately (owner
  standing rule); don't batch at milestones. Gates before pushes; a red gate is
  fixed forward.
- Gates: `scripts/check.sh` (workspace + fuzz clippy + crashpoint sweeps + kill
  smoke + alloc gate + feature matrices), `scripts/lean.sh` (lake build +
  spec-census + three-way conformance), `scripts/spec-census.sh` (the
  `path.rs::symbol` citation resolver — check (d) — will refuse a merge that
  orphans a lean citation).

---

## 2. What to unify vs. what to pin (the verdict table)

The principle: **unify** when two implementations of one judgment share a language
and a failure mode; **pin** (golden test) when the duplication is a deliberate
oracle or an unavoidable language boundary. Deleting an oracle is not removing
debt — it is removing a detector.

| Duplicated knowledge | Where | Verdict | Why |
|---|---|---|---|
| The lowering (name→id resolution, closed-shift arithmetic, declaration-order id minting, canonical-utterance ban table) | Rust macro expansion **and** `schema/spec.rs::descriptor()` | **UNIFY (§3, the refactor)** | Two implementations of one judgment, one language, one binary. The only inexcusable duplication. |
| Canonical spellings (renderer) | engine `schema/render.rs` **and** TS `renderStatement`/`renderWindow` | **Pin, don't unify** | TS renders *pre-open* errors the engine can't (no store yet); the manifest already ships engine-rendered spellings. Golden: TS render == manifest render for every construct. |
| Ban table, TS-side (`count.ts` five constructors) | TS | **Two-tier by design, don't unify** | TS enforces representationally (banned spellings have no constructor) at the earliest boundary; the engine lowering remains the law (a hostile FFI spec still refuses). Both tiers earn their keep. |
| Materialization mirror (`materializedEntries` at open) | engine **and** TS | **Don't unify — it's an oracle** | Detects theory drift between hosts at open. Unifying deletes the check. |
| Value vocabulary / marshaling | Rust `Value` ⇄ TS brands ⇄ `marshal.rs` | **Don't unify — bilingual boundary** | Pin by schema-directed marshaling + the cross-host fingerprint test (§6). |
| Query IR mirror | `ir::Program` ⇄ TS `ProgramIr` | **Don't unify — bijection, pin by golden** | IR is frozen; pin = TS-lowered IR accepted by engine `prepare` for every construct. |

---

## 3. PHASE A1/A2 — the clean refactor: `bumbledb-theory` + macro rewire

**Goal:** one lowering, shared by macro and spec; the macro's duplicate
resolution/ban-table code deleted (grep-provably gone). Fires immediately — it is
disjoint from the rebirth (primer-side) and mostly disjoint from the perf twins
(exec-side).

### A1 — extract `crates/bumbledb-theory` (zero LMDB/exec deps)

**Moves into the new crate:** the id types (`RelationId`, `FieldId`,
`StatementId`, `Generation`), `ValueType`, `LiteralSet`, `Side`, `Row`, the four
descriptor types + `SchemaDescriptor`, `Value` (`value.rs`), `Interval<T>` +
width law (`interval.rs`), and `SchemaSpec` + `SpecIssue`/`SchemaSpecError` + the
one lowering (`schema/spec.rs`'s `descriptor()` resolution + ban table).

**Stays in `bumbledb`:** `schema/validate.rs` (the admission boundary),
fingerprint, renderer, manifest, encoding, exec, storage.

**The facade ruling (this is the zero-debt answer to "shims"):** root re-exports
(`bumbledb::Value`, `bumbledb::SchemaSpec`, `bumbledb::SchemaDescriptor`, …) are
the **permanent public API**, documented in `70-api.md` — hosts depend on one
crate. Re-export as public surface = feature. Re-export as an *internal* crutch =
debt: internal engine code must import `bumbledb_theory::` directly, and **zero
internal shim usage may survive** the refactor. To keep the parallel perf branches
merging cleanly, the facade must preserve every currently-valid import path
(`crate::value::Value`, `crate::schema::SchemaDescriptor`, etc.) so branches cut
from `ee5d1de4` still compile after merge.

### A2 — rewire the macro through the shared lowering

`bumbledb-macros` gains a dep on `bumbledb-theory` (legal; no cycle —
theory has no macro dep).

1. Parse tokens → build a `SchemaSpec` **plus a span table** keyed by the same
   structural indices `SpecIssue` carries. (First task: confirm every
   `SpecIssue`/`SchemaSpecError` variant carries a structural index for
   span-mapping; enrich the type if any variant lacks one. `spec.rs` already
   exposes `issues() -> &[SpecIssue]` — this is the hook.)
2. Run the shared lowering **at expansion time**. Map each `SpecIssue` → its span →
   `compile_error!` naming the canonical form at the offending token.
3. Emit the lowered `SchemaDescriptor` as const token code (a
   `descriptor_tokens(&SchemaDescriptor) -> TokenStream` in the macros crate;
   reuse the existing macro's const-construction idiom for `Box<str>`/`Vec`
   literals). **Delete the macro's own resolution and ban-table code entirely** —
   verify by grep that no second copy remains. Type-provider emission (structs,
   newtypes, closed enums, weld tests) is unchanged.

### A locks (must survive)

- All schema compile-fail fixtures (`crates/bumbledb/tests/schema-compile-fail/`,
  the `schema_compile_fail.rs` roster count assert — currently 22) still name the
  canonical form at the right span; message churn deliberate-only.
- The macro-vs-spec fingerprint-parity test (`crates/bumbledb/tests/schema_spec.rs`
  — builds a theory with every construct both ways, asserts equal descriptor +
  equal fingerprint) survives as the standing regression pin. Parity becomes
  *structural* but stays *pinned*.
- Lean census `path.rs::symbol` citations swept to the new crate paths
  (`scripts/spec-census.sh` green).
- `70-api.md`'s `SchemaSpec` bindings-contract section updated for the facade.
- Gates: `check.sh`, `lean.sh`, trybuild suite, fingerprint parity, one-run ALL-WIN
  sanity (code moved; behavior must not — prove it).

**Named risks:** `Value`'s reach into encoding trait impls (facade must cover impls,
not just types); const-context emission of descriptors (already solved by today's
macro — reuse); span-mapping completeness (enumerate every `SpecIssue` variant).

---

## 4. PHASE A3 — the complete improvement ledger (NOTHING deferred)

Every item is a twin under §1's landing bar: own worktree, adversarial review,
merge behind gates. A measured refutation (gravestone) closes the line as
legitimately as a fix. **Perf details re-embedded here because `docs/reports` was
purged.**

### W1 — fixpoint incremental accumulator (the biggest single lever)
- **File:** `crates/bumbledb/src/api/prepared/fixpoint.rs` (~line 435, the
  `round_acc[p] = Some(... .refill(...))` loop; `round_acc` declared ~156, cleared
  ~187).
- **Defect:** every fixpoint round refills `round_acc` from the FULL accumulated
  seen-set — O(n²/2) row-copies over an n-round chain. Measured **95.6% of
  closure_depth's wall** (21.1 ms of 22.1 ms; join work is 3.5%).
- **Fix:** incremental accumulator — append the round delta into the standing half
  instead of full rebuild.
- **Done:** ~20× on deep closures; A/B + conformance + naive-parity + lean
  three-way green (recursion is in all three oracles); no regression on shallow
  closures.

### W2 — leaf batching + slot-copy elimination (hot core; owns 35–74% of join families)
- **File:** `crates/bumbledb/src/exec/run/probe_pass.rs` — the leaf `load_row`
  full-slot copy per survivor (~line 523) and the middle-node
  `pending_bindings.extend_from_slice` full slot row per routed survivor (~line
  557).
- **Defect:** the leaf runs **per parent (batch=1)** with a full slot-row copy per
  survivor; middle nodes copy full slot rows per routed survivor. This is the
  descend-exclusive bucket that owns 35–74% of every join family (spread, skew,
  triangle, chain, containment_walk, entries, rsvp_union, busy_scan, slot_scan,
  free_busy, mandate_overlap).
- **Fix:** per-batch leaf grouping; copy only changed slots.
- **Done:** 10–25% on spread/skew/chain-class; bit-identical answer sets
  (conformance + naive parity); no other family regresses.

### W3 — finalize column-major dispatch (owns 39–62% of four families)
- **File:** `crates/bumbledb/src/api/prepared/finalize.rs` (~lines 65 and 91, the
  per-row `match &column.ty` + tagged-cell push).
- **Defect:** per row × per column `match column.ty` + tagged enum cell push;
  12–24 ns/row. Owns 62/45/43/39% of containment_walk/free_busy/rsvp_union/range.
- **Fix:** hoist the column dispatch out of the row loop — column-major fill or a
  pre-resolved writer per column. The `Answers` public shape is untouched (layout
  change only).
- **Done:** 30–60% of finalize's share where it dominates; identical answer bytes.

### W4 — redundant zero-fill before full overwrite (trivial)
- **Files:** `crates/bumbledb/src/exec/run/probe_pass.rs` (7 `resize(n, 0)` sites
  on mask/allen_gather buffers, all fully overwritten after) and
  `crates/bumbledb/src/exec/kernel/allen.rs` (~lines 105/117, `codes`/`keep`).
- **Defect:** `_platform_memset` measured 3.7% of meets_chain; every element is
  unconditionally written afterward.
- **Fix:** `resize`-without-zero (`set_len` under the module's unsafe discipline
  after reserving) or equivalent — the buffers are write-before-read.
- **Done:** the memset disappears from the profile; identical results.

### W5 — dense-fold redundant accumulator copies
- **File:** `crates/bumbledb/src/exec/kernel/fold.rs`.
- **Defect:** LLVM keeps 3 redundant `mov.16b` accumulator copies per iteration
  (~12% extra vector µops, L1-relevance only).
- **Fix:** restructure the accumulator so LLVM stops copying (disasm-gated);
  or gravestone if LLVM won't cooperate.
- **Done:** kernel µop reduction shown in disassembly + A/B; or recorded refutation.

### W6 — stride-padder re-run post-T1 (a recorded re-open trigger, now DUE)
- **File:** `crates/bumbledb/src/image.rs` (`StridePadder`, `PAD_TOLERANCE = 384`);
  falsifier `crates/bumbledb/src/image/tests/stride_ab.rs`.
- **Context:** T4 refuted the 2 KiB widening at image pitches and recorded a
  re-open trigger: **re-run once T1's tighter multi-column kernels land** (they
  have). Pure pow-2 pitches measured 1.25–1.8× on tight kernels (family-invisible
  at the time).
- **Fix:** re-run `stride_ab` against the T1-reshaped scan kernels; land whatever
  pad rule they now demand, or re-earn the refutation with the new kernels cited.
- **Done:** verdict recorded either way; if a rule lands, ≥3× on the pathological
  residue and no regression on healthy strides.

### W7 — prefetch-gate WATCH ablation (a recorded caveat, now measurable)
- **File:** `crates/bumbledb/src/exec/run/run.rs` (`PREFETCH_WIDTH_FLOOR`),
  `crates/bumbledb/src/exec/colt/prefetch.rs`.
- **Context:** the gate is width-only; the ledger says the real gate is working-set
  tier. The tier ablation measured nothing at the campaign's floor maps, but T9's
  displaced lanes now make the DRAM/displaced regime measurable.
- **Fix:** re-ablate the tier gate on the displaced lanes; record the verdict
  (keep width-only, or add the tier gate if it now pays).
- **Done:** ablation re-run on displaced lanes, verdict recorded.

### W8 — T8 commit-size sweep (a gravestone owed its curve)
- **Files:** `crates/bumbledb/src/storage/commit/{apply,judgment}.rs`; corpus =
  `crates/bumbledb-bench/src/windowed.rs`.
- **Context:** T8 found key-sorted judgment probe order indistinguishable at bench
  commit sizes but never swept commit size to find where it starts paying.
- **Fix:** sweep commit size (touched-parent count) on ephemeral stores; land the
  sort if realistic sizes benefit (mind determinism — the citation-order contract
  in `lean/Main.lean` `RVerdict` and `error.rs` `Violations::seal`; the seal sorts,
  so probe order should be invisible — PROVE it with conformance + multi-violation
  fixtures). Otherwise the gravestone gains its measured curve.
- **Done:** the sweep exists; landed-if-wins or curve recorded.

### W9 — bimodality mechanism hunt (no unexplained behavior ships)
- **Symptom:** `slot_booking_overlap` and `postings_without_tag` flip between two
  performance modes across whole bench processes (per-pair A/B ratios 0.34–2.01 on
  *identical binaries*). The charts' min-of-3 selects the fast mode symmetrically,
  so it is not a regression — but it is unexplained.
- **Fix:** name the mechanism (candidates: LMDB store page-state across process
  restarts; the 35% code-placement relink lottery; something else). Fix if
  engine-side; pin as external/environmental if not.
- **Done:** mechanism named; fixed or pinned-as-external. Nothing unexplained
  remains.

### W10 — allocation-census hoistables
- **Files (verify by grep — plan-side):** `storage/commit/plan.rs` (edges `Vec`
  ~305, determinants collect ~275, `into_boxed_slice` shrink-realloc ~329, ops-vec
  growth reallocs ~210), `storage/keys.rs` (~37, `DeterminantImage` 8-byte tiny
  boxes), `storage/commit/judgment.rs` (~390, `check_target` scratch sets).
- **Context:** warm *execute* is already zero-alloc census-wide (the gate's floor
  holds). These are the plan/commit-side hoistables the census flagged; none
  load-bearing at current scale, but 1.0.0 = zero known.
- **Fix:** hoist/pool each; the `alloc_census` harness
  (`crates/bumbledb/tests/alloc_census.rs`) proves the delta.
- **Done:** census shows the reduction; correctness untouched.

### W11 — lean proof: FilterPredicate transport / range-summary narrowing
- **File:** `lean/Bumbledb/Exec/Rewrites.lean` (the range-summary narrowing left
  from the proof-debt pass).
- **Context:** the word-level range-summary lemma is proved; the transport to
  `FilterPredicate` lists over `Values` (order-embedding encodings + emit splice)
  is a stated narrowing, not a theorem. The fold-off differential is its empirical
  arm.
- **Fix:** attempt the full theorem in earnest under the **zero-sorry law** (no
  `sorry`/`admit`/`axiom` ever). Either a completed theorem, or the exact stuck
  goal state recorded verbatim and the narrowing kept (a narrowing is the spec's
  sanctioned form — but it must be *earned* by a real attempt, not assumed).
- **Done:** `lake build` green, `grep -rn sorry lean/` empty; theorem or recorded
  goal state.

### Hygiene (fold into A3)
- Prune stale `perf/*`, `fix/*`, `bench/*`, `test/*`, `freeze/*`, `lean/*` branches
  and any `/tmp/*-twin` worktrees.
- Sweep lean census citations to new crate paths (with A1).
- OPEN-ledger final pass (`70-api.md`): every row must end fixed/refuted/fired/
  declined — nothing "pending."

---

## 5. PHASE A-FUZZ — max the machine, make generation smarter

**Owner ask:** the fuzzers should run blazing on ALL cores (M2 Max = 12), not two,
and the generative fuzzer should be smarter (more likely to find bugs).

1. **All-cores by default.** Audit `scripts/fuzz.sh` (default is `FUZZ_WORKERS=12`
   already — find where the effective parallelism drops to ~2: the crash-sweep
   lane, `-fork=` count, the ASAN lane, or a co-tenant default). Make true
   all-cores (12) the real default across every lane; verify with a short run that
   all cores saturate (`sample`/`top` shows ~1200% CPU). Keep a `FUZZ_WORKERS`
   override for co-tenant sessions.
2. **Smarter generative fuzzing.** The whole point of generative fuzzing is raising
   the probability of reaching deep engine bugs. Today many inputs bounce off the
   parser/validator. Make generation **structure-aware**: bias the `Arbitrary`
   impls (in `fuzz/src`) toward *well-formed-but-adversarial* schemas and queries
   (valid ids, resolvable names, in-roster handles, legal-but-extreme windows and
   selections), add a dictionary of interesting values, seed the corpus from the
   conformance cases (`lean/conformance/cases/`), and measure the improvement by
   coverage-per-exec and corpus growth vs. the current impls. Targets:
   theory/ops/query/rewrites/crash.
3. **Sequencing (device honesty):** harden + verify-saturation NOW as part of the
   campaign, but run the long **all-cores hunt AFTER the perf A/B sessions land** —
   a 12-core fuzz storm swamps interleaved measurements far past the ±2% ambient
   band. The blazing multi-hour hunt is a dedicated session on the idle machine;
   any finding is triaged per the fuzzing charter (stop, minimize, regression test
   or environmental disposition, `fuzz/SESSIONS.md` row, delete artifact).

---

## 6. PHASE B — primer: rename + repoint (GATED on rebirth PRD-18 green)

1. Rename the SDK **`@superbuilders/bumbledb` → `@bjornpagen/bumbledb`** everywhere
   (package.json, every import specifier across the rebuilt pipeline, turbo refs,
   lockfile). Rationale: the SDK scope follows the engine's owner
   (`bjornpagen/bumbledb`), not the consuming org. `private: true` STAYS (no
   registry yet).
2. Repoint `packages/bumbledb/crate` path-dep at real refactored bumbledb main;
   **retire the snapshot copy** at `../primer/.claude/worktrees/bumbledb` (it is
   debt the moment main is consumable). Expected near-zero diff (the facade's
   purpose). Rebuild the native module; SDK tests green; primer typecheck/knip green.
3. Add the two pins the SDK owes:
   - **TS-render ⇄ manifest-render golden:** TS `renderStatement`/`renderWindow`
     output equals the engine-rendered spelling the manifest ships, for every
     construct.
   - **The cross-host fingerprint lock:** a JS-created store (via `SchemaSpec`)
     opened from Rust via an identical `schema!` theory — assert fingerprint
     equality across the FFI. The one test neither surface can fake.

---

## 7. PHASE C — the re-upstream census (GATED on B)

Reader fanout over the rebuilt pipeline (rebirth PRDs 09–17 outputs) hunting engine
workarounds — marshaling contortions, missing manifest data, rejection-wire gaps,
delta-model friction in the repair loop (PRD-14 is the stress case). **Every
finding lands engine-first** (bumbledb, through full gates, pushed), SDK adapts
after — primer never patches around the engine.

**The OPEN-ledger sugar, judged against the real consumer:** `insert_all`,
multi-key typed `get`, `FromAnswers`/answer sorting, `write_from` retry helper —
if graph-builder's actual code reached for it, the trigger fired and it lands NOW;
if the real consumer never reached for it, it is *declined vocabulary* under the
owner's own ratified trigger law, and the ledger records the verdict either way.
This is the one category where "not built" is a correct *resolved* state — unfired
speculative sugar would itself be debt.

---

## 8. PHASE D — 1.0.0 close (GATED on C)

1. Full gates + fuzz smoke on the post-refactor, post-census tree.
2. **Full bench session + all five charts regenerated.** Unlike prior re-earns,
   numbers are EXPECTED to move (W1–W3 are collectively bigger than the whole
   layer-law campaign) — re-true the README claims and any surviving normative
   numbers, don't just re-earn ALL-WIN.
3. OPEN ledger final state (every row fixed/refuted/fired/declined).
4. Version bump to `1.0.0` (workspace Cargo.toml), commit summary, prep the
   annotated tag. **The owner pushes the tag** — the release ceremony is the
   owner's, and 1.0.0 is the owner's decision, not a gate's.

---

## 9. PHASE E — npm, literally last (owner's explicit word only)

Only after: the tag exists, the SDK has survived real graph-builder production
runs, and the name/scope/API have had their full window to change. Then: flip
`private`, version pinned to the tagged engine, provenance on, publish
`@bjornpagen/bumbledb`. Maximally reversible until this one step, which is why it
goes last.

---

## 10. Dependency graph (real dependencies only — no phasing theater)

```
A (refactor ∥ improvements ∥ fuzz-harden)  — fires now, on a worktree, main untouched
        │  (measurement mutex serializes only the TIMING sessions)
        ▼
  merge campaign → main   — GATED: rebirth PRD-18 green
        ▼
B (primer rename/repoint) → C (census) → D (1.0.0 close, owner tags) → E (npm, owner word)

fuzz all-cores HUNT: after A's perf A/B sessions land (idle machine), before D.
```

Within any phase: gates before pushes; pushes immediate; merges by the orchestrator
on the campaign branch; adversarial review on every claimed win.

**Exit criterion (the release floor):** grep the repo for a known defect, a
measured-but-unclaimed win, an unexplained behavior, or an unresolved ledger row —
and find nothing. Then it is the owner's call.
