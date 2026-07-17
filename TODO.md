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

**Anchor discipline (this document's own law):** every code target below is
anchored by `path :: symbol`, never by line number — the same ruling the lean
census enforces on the spec's citations (`scripts/spec-census.sh` check (d);
line citations drift silently and were purged from `lean/` for exactly that
reason). Locate targets by symbol with grep. A future edit that re-introduces
`~line NNN` anchors here is regression, not convenience.

---

## 0. Where things stand (2026-07-16, verified against the trees)

- **Main is at `b14ef8a8`** (this document) atop `ee5d1de4` ("the bindings
  contract: SchemaSpec parity, the dyn surface, violations rendered as data"),
  pushed, CI green. The layer-law perf campaign (T1/T5/T6/T7 wins, T2/T3/T4/T8
  gravestones, T9 displaced lanes, the alloc-census harness) landed before it;
  ALL-WIN is re-earned and all five README charts are current.
- **`docs/reports/` and `docs/brainlift-sources/` are PURGED** (owner ruling:
  reports was never a sanctioned normative home — a drift-prone projection of
  the code at one instant; the only authorities are the code, the lean spec, and
  `docs/architecture/`). The campaign record lives only in git history
  (`git show 470c7428:docs/reports/perf-campaign-layer-law.md`). Everything an
  item below needs is re-embedded in the item; no archaeology required.
- **The rebirth is in flight** in primer (`graph-builder-rebirth` branch), and
  further along than the prior draft recorded (state verified against the
  worktree and the planning session's transcript, 2026-07-16 19:45): engine
  PRDs 01–02 landed here as `ee5d1de4`; SDK PRDs 03–08 are done (65/65 tests,
  the newtype migration applied); store schema (with its 17-newtype block),
  prompts, gates, ETL, seats, and the lean refit are done; the driver is
  executing a finish-scoped brief; `driver/supervisor.ts` exists. TUI, the
  funeral, the reviewers, and the fixer remain — §6 (PHASE R) is the full
  completion contract. The TypeScript SDK lives at
  `../primer/.claude/worktrees/rebirth/packages/bumbledb`
  (`@superbuilders/bumbledb`, `private: true`, ~6k lines, **currently
  untracked in that worktree** — it commits with its PRDs). It is a real
  type-theoretic kernel: branded newtypes, `relation()`, `closed()`,
  `schema()`, the five-constructor `count` vocabulary enforcing the ban table
  representationally, `Db` runtime with rejection-as-data, query surface as
  plain-data IR values. The napi bridge (`crate/src/{lib,marshal}.rs`) is the
  dumb schema-directed marshaler; names resolve TS-side via the manifest, ids
  cross the FFI.

### The reconciliation rulings (ratified 2026-07-16 — the SDK's design law)

Ratified in the planning session ("Audit and understand GraphBuilder code")
and already law in the PRD packet (amended on primer main, `549a3e3ce`);
recorded here because Phases R/B/C enforce them:

- **Bijective semantics, never shared syntax.** The TS surface speaks
  TypeScript-idiomatic values (`contained(on(Account, "holder"),
  on(Holder, "id"))`), not the Rust notation; no string parsing anywhere, no
  type-level string parsing (the template-literal design was considered and
  rejected). The bijection is semantic and proven mechanically: both surfaces
  lower to the same `SchemaSpec` descriptor, and identical theories yield
  **identical fingerprints** — the engine cannot tell which language wrote
  the schema. That parity pin is the standing regression test.
- **Declaration-first newtypes, one spelling.** `const AccountId =
  u64.newtype("AccountId")` + `type AccountId = Infer<typeof AccountId>`;
  `.as` is deleted with no alias; `bool`/`str` lack `.newtype`; declared
  newtypes cannot re-brand. The TS layer is the right place for nominal
  typing (owner ruling) — brands are the mechanism, the single declaration
  site is the discipline that structural typing can't supply.
- **The ban table is unwritable, not checked.** `exactly`/`none`/`between`/
  `atLeast`/`atMost` partition the legal windows; banned spellings have no
  constructor. Two-tier enforcement stands (§2): the engine lowering remains
  the law for hostile FFI specs.
- **The ordering law.** Engine (bumbledb) commits always precede the primer
  commits that consume them — the SDK is never built against uncommitted
  engine state.

### The sequencing truth (verified: there is no snapshot)

`../primer/.claude/worktrees/bumbledb` is a **symlink to this repo** (same
inode; verified 2026-07-16). The SDK crate's path dependency
(`bumbledb = { path = "../../../../bumbledb/crates/bumbledb" }`) resolves
through it into **this live working tree**. There is no copy, no pin, no
independent state. Therefore:

- **The merge-back law (owner ruling, NONNEGOTIABLE):** all and any bumbledb
  changes — whoever makes them, from whichever side, primer agents through
  the symlink included — are committed **in this repo, on `main`, pushed**.
  No satellite copy of bumbledb may hold state this repo doesn't. If anyone
  ever replaces the symlink with a real copy or clone, that copy may not be
  deleted or "retired" without a diff-audit and a merge-back of everything it
  holds — deletion without audit is forbidden. As of 2026-07-16 the law is
  satisfied everywhere: the symlink has no independent state by construction,
  the one true worktree (`../bumbledb-worktrees/prd-constitution`) is clean
  with its branch fully merged, and every `codex/*` and `worktree-wf_*`
  branch is an ancestor of `main`.
- **The live-tree compatibility law.** Because the rebirth compiles this tree
  as it stands, every campaign commit must leave the crate surface the SDK's
  napi bridge consumes **compile-compatible** — this is the A1 facade law
  promoted from "nice for the eventual repoint" to a standing condition on
  every push. Docs-only commits are trivially safe; perf twins are internal;
  A1/A2 is exactly why the facade must preserve every import path. Anything
  that would break `packages/bumbledb/crate`'s build waits for rebirth
  PRD-18 green — and the rebirth doubles as a live integration consumer for
  everything else meanwhile.
- **The symlink's lifetime is the rebirth worktree's.** It stays while the
  worktree exists (deleting it mid-run breaks every cargo build the remaining
  agents run) and is deleted **together with the worktree** at the mega-merge
  (§6, gate #10) — after which primer's committed path dep
  (`../../../../bumbledb/crates/bumbledb`) resolves through
  `~/Documents/bumbledb` directly, no hop.
- Phase B (rename + verification) remains **gated on the rebirth tail
  completing** (§6 — the funeral, the reviewers, the fixer, the mega-merge) —
  that gate is about the *consumer* being ready, not about protecting main.

---

## 1. Execution discipline (applies to every phase)

- **No worktrees (owner ruling, standing).** Work directly in this repo, one
  item at a time. Parallelism, where it exists, is conversational — the owner
  and the agent decide together what fires next; there is no fan-out fleet and
  no integration branch. This replaces the prior draft's "worktrees, always"
  wholesale.
- **Design-first (owner ruling, standing).** A ruling an item needs (a default,
  a vocabulary choice, a reversal) is surfaced in conversation with a clear
  recommendation before code is written — never decided silently inside a
  diff. Items below that embed a ruling say so.
- **Push discipline (owner ruling, standing):** every gate-green commit goes to
  `origin/main` immediately, never batched at milestones. A red gate is fixed
  forward. The merge-back law (§0) rides with it: no bumbledb change lives
  anywhere but this repo, on `main`, pushed — nonnegotiable.
- **Measurement law (the M2 Max ledger's discipline — violating it voids a
  verdict).** All timing through `scripts/measure.sh` (the machine-wide mutex).
  Absolute numbers under co-tenancy are VOID; only **interleaved same-session
  A/B** ratios are valid (±2% band). Fresh data per rep (TAGE memorizes
  benchmark inner loops; min-of-reps is a biased estimator, measured 4.7×).
  Trace counts, never assume them. **Disassembly-gate** every codegen claim
  (`scripts/check-asm.sh`; LLVM substitutes memcpy/autovectorization/
  reassociation). State the tier (L1/L2/DRAM/displaced) of every number. The
  judgment layer is `docs/reference/apple-silicon-performance.md`; the bench
  estate operationalizes it (clock-proxy bracketing, device honesty, the
  quantum floor).
- **The landing bar for any perf/behavior twin ("clear win"):**
  1. semantics untouched — `cargo test -p bumbledb` + conformance +
     naive-parity + the lean three-way green for anything touching execution;
     `lean/` untouched unless the item is a lean item (the gate law,
     `docs/architecture/README.md` rule 7: semantics moves lean in the same
     commit — a twin that would touch lean is not a twin, it is a semantics
     change and re-enters through design);
  2. predicted sign outside the ±2% band at the regime that matters
     (family-level ≥5%, or a kernel win with demonstrated family neutrality);
  3. no other family loses >2% (interleaved spot-check);
  4. fmt + clippy -D + check-asm if kernels moved + the alloc gate if hot
     paths moved.
  A LOSS/NEUTRAL twin lands NOTHING but its **gravestone** (the experiment, the
  numbers, a recorded paragraph in the commit) — a measured refutation empties
  a known-issue line as legitimately as a fix.
- **Adversarial review** of every claimed win before merge: an independent
  agent re-runs the falsifier, re-checks semantics, re-reads the disassembly,
  audits new `unsafe`. This is verification, not ceremony; it stays.
- Gates: `scripts/check.sh` (fmt, clippy -D, workspace tests, doc tests, the
  release alloc gate, the feature matrices, fuzz-crate clippy, the crashpoint
  sweeps, the kill smoke, the x86 cross-check), `scripts/lean.sh` (lake build +
  the zero-sorry/zero-axiom batteries + spec-census + the conformance corpus +
  the three-way comparator with its vacuous-pass refusal),
  `scripts/spec-census.sh` standalone when only citations moved.

---

## 2. What to unify vs. what to pin (the verdict table)

The principle: **unify** when two implementations of one judgment share a
language and a failure mode; **pin** (golden test) when the duplication is a
deliberate oracle or an unavoidable language boundary. Deleting an oracle is
not removing debt — it is removing a detector.

| Duplicated knowledge | Where | Verdict | Why |
|---|---|---|---|
| The lowering (name→id resolution, closed-shift arithmetic, declaration-order id minting, canonical-utterance ban table) | Rust macro expansion **and** `schema/spec.rs :: SchemaSpec::descriptor` | **UNIFY (§3)** | Two implementations of one judgment, one language, one binary. The only inexcusable duplication. |
| Canonical spellings (renderer) | engine `schema/render.rs` **and** TS `renderStatement`/`renderWindow` | **Pin, don't unify** | TS renders *pre-open* errors the engine can't (no store yet); the manifest already ships engine-rendered spellings. Golden: TS render == manifest render for every construct. |
| Ban table, TS-side (`count.ts` five constructors) | TS | **Two-tier by design, don't unify** | TS enforces representationally (banned spellings have no constructor) at the earliest boundary; the engine lowering remains the law (a hostile FFI spec still refuses). Both tiers earn their keep. |
| Materialization mirror (`materializedEntries` at open) | engine **and** TS | **Don't unify — it's an oracle** | Detects theory drift between hosts at open. Unifying deletes the check. |
| Value vocabulary / marshaling | Rust `Value` ⇄ TS brands ⇄ `marshal.rs` | **Don't unify — bilingual boundary** | Pin by schema-directed marshaling + the cross-host fingerprint test (§6). |
| Query IR mirror | `ir::Program` ⇄ TS `ProgramIr` | **Don't unify — bijection, pin by golden** | IR is frozen; pin = TS-lowered IR accepted by engine `prepare` for every construct. |

---

## 3. PHASE A1/A2 — the clean refactor: `bumbledb-theory` + macro rewire

**Goal:** one lowering, shared by macro and spec; the macro's duplicate
resolution/ban-table code deleted (grep-provably gone). Fires now — it is
disjoint from the rebirth (primer-side) and mostly disjoint from the perf
twins (exec-side).

### A1 — extract `crates/bumbledb-theory` (zero LMDB/exec deps)

**Moves:** the id types (`RelationId`, `FieldId`, `StatementId`,
`Generation`), `ValueType`, `IntervalElement`, `LiteralSet`, `Side`,
`Row`/`Extension`, the four descriptor types + `SchemaDescriptor` (with
`materialized_statements` — it is pure), `Value`, `Interval<T>` + the width
law, and `SchemaSpec` + `SpecIssue`/`SchemaSpecError` + the one lowering
(`SchemaSpec::descriptor`'s resolution + ban table).

**Stays:** `schema/validate.rs` (the admission boundary and the sealed
`Schema`/enforcement half), fingerprint, renderer, manifest, encoding, exec,
storage.

**The four named frictions (found by reading, not guessed — each needs a
ruling or a mechanical answer before the move):**

1. **`Value::AllenMask` wraps `allen::AllenMask`.** The mask newtype (13-bit,
   checked constructors, `converse`) is theory-shaped and moves with `Value`;
   the *kernel* (`exec/kernel/allen.rs`, the NEON path, `classify`) stays.
   `allen.rs` splits: the mask vocabulary to theory, the classification
   machinery in the engine.
2. **`interval.rs` leaks crate-private engine surface.** `Interval` moves, but
   `pub(crate) mod sweep` (the exec-side coalescing sweep) stays behind, and
   `Interval::bounds` (read by `allen::classify`) needs a sanctioned crossing —
   public on the theory type, or re-plumbed.
3. **`schema.rs` is two modules in one file.** The descriptor half moves; the
   sealed-witness half (`Schema`, `Relation`, `Enforcement`, `MemberSet`,
   `CompiledCheck`, `FreshField<S>`, `IntervalTail` — which calls into
   `encoding`) stays. This is a split, not a file move. `value_matches` (the
   one Value↔ValueType check, shared by IR validation, bind, dyn writes,
   selection encoding) is theory-shaped and moves.
4. **`ValueType::type_desc` is an inherent impl returning
   `encoding::TypeDesc`.** Encoding stays; Rust forbids inherent impls on
   foreign types. Either `TypeDesc` moves too (it is LMDB-free) or the impl
   becomes a free function/extension trait in the engine. Ruling needed;
   recommendation: move `TypeDesc` — it is a pure layout vocabulary.

**The facade ruling (the zero-debt answer to "shims"):** root re-exports
(`bumbledb::Value`, `bumbledb::SchemaSpec`, `bumbledb::SchemaDescriptor`, …)
are the **permanent public API**, documented in `70-api.md` — hosts depend on
one crate. Re-export as public surface = feature. Re-export as an *internal*
crutch = debt: internal engine code imports `bumbledb_theory::` directly, and
**zero internal shim usage may survive** the refactor (grep-enforced). The
facade must preserve every currently-valid import path (`crate::value::Value`,
`crate::schema::SchemaDescriptor`, …) so nothing downstream of the crate root
notices — and the consumer is not hypothetical: the rebirth's napi bridge
compiles this live tree through the symlink (§0), so the facade holds on
**every intermediate commit**, not just at the end. Build the SDK crate as
part of A1's verification, not as an afterthought.

### A2 — rewire the macro through the shared lowering

`bumbledb-macros` gains a dep on `bumbledb-theory` (legal; no cycle — theory
has no macro dep).

1. Parse tokens → build a `SchemaSpec` **plus a span table** keyed by the
   structural indices `SpecIssue` carries. **The audit is done; the type needs
   enrichment first:** 8 of 11 variants carry a statement index
   (`UnknownRelation`, `UnknownField`, the five window variants,
   `DegenerateLiteralSet`); **three carry names only and must gain structural
   indices** — `NotAHandleField`, `UnknownHandle` (both reachable from two
   provenances: statement selections and extension rows, with no
   discriminator), and `DuplicateHandleNewtype`. `UnknownField` and
   `DegenerateLiteralSet` additionally want a (side, occurrence) discriminator,
   or the span table keys `(statement, name) → all spans` and marks them all.
   `SchemaSpec::issues()` is the hook.
2. Run the shared lowering **at expansion time**. Map each `SpecIssue` → its
   span → `compile_error!` naming the canonical form at the offending token.
3. **Literal typing stays an expansion-time error.** Today the macro types
   literals at expansion (`value_expr`) while the spec path defers mismatch to
   `SchemaDescriptor::validate`. The parse→`SchemaSpec` conversion is where
   token literals become typed `Value`s, so the type-mismatch compile error
   survives at that seam — it must not silently degrade to a `Db::create`
   error.
4. Emit the lowered `SchemaDescriptor` as const token code
   (`descriptor_tokens(&SchemaDescriptor) -> TokenStream` in the macros crate;
   the existing `emit_schema_def` is essentially this already — reuse its
   const-construction idiom). **Delete the macro's own resolution and
   ban-table code entirely** (`relation_index`, `field_index`, `closed_map`,
   `admit_window`, the `parse_literals` degenerate-set check, the `==`
   two-containment lowering) — verify by grep that no second copy remains.
   Type-provider emission (structs, newtypes, closed enums, weld tests) is
   unchanged.

### A locks (must survive)

- All schema compile-fail fixtures (`crates/bumbledb/tests/schema-compile-fail/`,
  the `schema_compile_fail.rs` roster count assert — currently 22) still name
  the canonical form at the right span; message churn deliberate-only.
- The macro-vs-spec parity test (`crates/bumbledb/tests/schema_spec.rs` —
  every construct both ways, equal descriptor + equal fingerprint) survives as
  the standing regression pin. Parity becomes *structural* but stays *pinned*.
- **The census sweep is part of A1, same commit:** lean doc-comment
  `path.rs::symbol` citations and `Bridge.lean` mechanism tokens that name
  moved files re-anchor to the new crate paths; `scripts/spec-census.sh`
  resolves paths as suffixes under `crates/*/src`, so a moved file with an
  unswept citation fails the gate — that is the gate working.
- `70-api.md`'s SchemaSpec bindings-contract section updated for the facade
  (docs rule 5: the doc amends in the same change).
- Gates: `check.sh`, `lean.sh`, the trybuild suite, fingerprint parity, one-run
  ALL-WIN sanity (code moved; behavior must not — prove it).

---

## 4. PHASE A3 — the complete improvement ledger (NOTHING deferred)

Every item lands under §1's landing bar: adversarial review, gates, push. A
measured refutation (gravestone) closes the line as legitimately as a fix.
Perf context is re-embedded here because `docs/reports` was purged. All
anchors are symbols; grep for them.

### W1 — fixpoint incremental accumulator (the biggest single lever)
- **Site:** `api/prepared/fixpoint.rs` — the round loop in `run_fixpoint`
  where `round_acc[p]` is refilled from `sink.answers_since(0)`;
  `FixpointScratch::{acc, flip, watermark, round_acc}` and
  `FixpointScratch::begin` are the state.
- **Defect:** every round refills the accumulated image from the FULL seen-set
  — O(n²/2) row-copies over an n-round chain. Measured **95.6% of
  closure_depth's wall** (21.1 ms of 22.1 ms; join work is 3.5%).
- **Fix:** incremental accumulator — append the round delta into a standing
  image instead of rebuilding. **Constraints found by reading:**
  `TransientImage::refill` reuses its slab only when the `Arc` is unique — the
  ping-pong exists precisely so round r builds into the half round r−1's views
  no longer hold; an append-in-place design must either regain uniqueness at
  the same point in the round or grow `TransientImage` an append API with the
  same precondition. The **finished image** (`finished_slot`) is deliberately
  never an alias of the accumulator pool — preserve that separation or
  re-justify it in the same commit.
- **Lean/oracle arm:** recursion is in all three oracles — the conformance
  program cases replay against `Exec/Fixpoint.lean :: program_eval_sound`'s
  fueled fixpoint; naive parity and SQLite (`WITH RECURSIVE` where
  expressible) both gate. Semantics must be bit-identical.
- **Done:** ~20× on deep closures; A/B + all oracles green; no regression on
  shallow closures (closure_fanout is the width control).

### W2 — leaf batching + slot-copy elimination (hot core; owns 35–74% of join families)
- **Site:** `exec/run/probe_pass.rs :: Executor::probe_pass` — the
  survivor-routing loop: the leaf arm's `Bindings::load_row` full-slot copy
  per survivor, and the middle-node arm's
  `pending_bindings.extend_from_slice` full slot row per routed survivor.
- **Defect:** the leaf runs **per parent (batch=1)** with a full slot-row copy
  per survivor; middle nodes copy full rows then overwrite only the cover
  slots. This descend-exclusive bucket owns 35–74% of every join family
  (spread, skew, triangle, chain, containment_walk, entries, rsvp_union,
  busy_scan, slot_scan, free_busy, mandate_overlap).
- **Fix:** per-batch leaf grouping (consecutive survivors sharing a parent
  load the row once); copy only changed slots on the middle arm.
  **Constraint found by reading:** `run_node`'s sibling loop and
  `probe_pass`'s are **deliberate line-parallel twins** (the extraction
  refusal is recorded at both loop heads) — a change here moves its mirror
  there, or the refusal is consciously retired in the same commit, never
  silently. The D2 origin checks, the origin-mint overflow check, and the
  bounded-child flush contract (≤ ~2 batches) live in the same loop and must
  survive shape-identical.
- **Done:** 10–25% on spread/skew/chain-class; bit-identical answer sets
  (conformance + naive parity); no other family regresses.

### W3 — finalize column-major dispatch (owns 39–62% of four families)
- **Site:** `api/prepared/finalize.rs :: {finalize, push_word_answer,
  push_resolved_answer}` — per row × per column `match &column.ty`, and
  `Answers::{word_cell, push_word}` re-match the type per cell (the dispatch
  runs up to three times per cell).
- **Defect:** 12–24 ns/row. Owns 62/45/43/39% of
  containment_walk/free_busy/rsvp_union/range.
- **Fix:** pre-resolve a per-column writer plan at prepare (the
  `PredicateColumn` roster is sealed at validation — the plan is derivable
  once); hoist the dispatch out of the row loop or fill column-major. The
  `Answers` public shape is untouched — cells are row-major
  (`answer * arity + column`), so a column-major fill writes strided cell
  slots; the byte heap is internal and its ranges stay correct.
- **Done:** 30–60% of finalize's share where it dominates; identical answer
  bytes.

### W4 — redundant zero-fill before full overwrite (small, not trivial)
- **Sites:** `exec/run/probe_pass.rs` — seven `resize(n, 0)` sites on
  mask/hash/allen_gather scratch, all write-before-read; and
  `exec/kernel/allen.rs :: {allen_code_batch, allen_filter_batch}` — the
  `codes`/`keep` resize-then-fill pairs.
- **Defect:** `_platform_memset` measured 3.7% of meets_chain; every element
  is unconditionally written afterward.
- **Two constraints the prior draft missed:**
  1. `exec/run/` is **not on the unsafe allowlist**
     (`00-product.md` § the unsafe policy names kernel/colt/wordmap/run-leaf
     paths/image/obs). A `set_len`-shaped fix either lands as a new
     sanctioned, documented-invariant site (a policy amendment — owner
     ruling, same commit amends `00-product.md`) or routes through a helper
     in an allowlisted module. `spare_capacity_mut` without `unsafe` is the
     third option if codegen cooperates — check the disassembly, not the
     vibes.
  2. The measure-residual mask's fill loop has a ray-poison early-`break`
     that leaves a tail unwritten — safe today only because the function
     returns before the mask is read. A resize-without-zero fix must keep
     that path's write-before-read truth, not assume it.
- **Done:** the memset disappears from the profile; identical results; the
  unsafe policy either untouched or amended by ruling.

### W5 — dense-fold redundant accumulator copies
- **Site:** `exec/kernel/fold.rs :: {fold_sum_u64_dense,
  fold_min_max_u64_dense}` — the `[Simd<u64, 2>; 4]` accumulator arrays
  indexed in the loop are the likely cause of LLVM's 3 redundant `mov.16b`
  copies per iteration (~12% extra vector µops; L1-relevance only).
- **Fix:** restructure the accumulators (named locals, or wider vectors) so
  the copies vanish — disasm-gated; the carry-counted exactness law holds
  (bit-identical to any-association i128/u128 folding, pinned by the
  differential property tests). Or gravestone if LLVM won't cooperate.
- **Done:** kernel µop reduction shown in disassembly + A/B; or the recorded
  refutation.
- **Assembler note (2026-07-16):** landed for `fold_sum_u64_dense` (expected
  ~4 fewer vector µops per 8-word iteration); the disasm HALF-REFUTES the
  premise for `fold_min_max_u64_dense` — its `mov.16b`s are init-only, the
  loop was already copy-free. Formal disasm gate + A/B stays with the
  measured stage.

### W6 — stride-padder re-run post-T1 (a recorded re-open trigger, now DUE)
- **Site:** `image.rs :: StridePadder` / `PAD_TOLERANCE` (384; the constant's
  doc comment records the refuted 2 KiB widening and the measured decay
  curve); mechanism `image/stride.rs :: StridePadder::place`; falsifiers
  `image/tests/stride_ab.rs` (four `#[ignore]`d measured falsifiers + two
  always-on layout pins) — the harness is complete, parameterized via
  `with_tolerance`/`image_with_tolerance`.
- **Context:** T4 refuted the widening at image pitches and recorded a
  re-open trigger: re-run once T1's tighter multi-column kernels land (they
  have). Pure pow-2 pitches measured 1.25–1.8× on tight kernels,
  family-invisible at the time.
- **Fix:** re-run `stride_ab` against the T1-reshaped scan kernels; land
  whatever pad rule they now demand through `PAD_TOLERANCE` (the doc comment
  amends with the new numbers), or re-earn the refutation with the new
  kernels cited.
- **Done:** verdict recorded either way; if a rule lands, ≥3× on the
  pathological residue and no regression on healthy strides.

### W7 — prefetch-gate WATCH ablation (a recorded caveat, now measurable)
- **Site:** `exec/run.rs :: PREFETCH_WIDTH_FLOOR` (the module root — the
  prior draft's `run/run.rs` path was stale), gating at the two prefetch
  sites in `probe_pass.rs` and `run_node.rs`;
  `exec/colt/prefetch.rs :: Colt::prefetch_bucket`.
- **Context:** the gate is width-only; the constant's own doc records that
  the footprint-tier ablation measured NOTHING at the campaign's floor maps.
  T9's displaced lanes (`disp_probe*`/`disp_stream*` in the bench) now make
  the DRAM/displaced regime measurable. **The tier signal already exists:**
  both gate sites emit `probe_footprint_bytes` through obs — the ablation
  needs no new plumbing, only a twin that gates on it.
- **Fix:** re-ablate the tier gate on the displaced lanes; keep width-only or
  land the tier gate, verdict recorded in the constant's doc either way.
- **Done:** ablation re-run on displaced lanes, verdict recorded.

### W8 — T8 commit-size sweep (a gravestone owed its curve)
- **Sites:** `storage/commit/judgment.rs :: {judge, check_source}` and
  `storage/commit/apply.rs :: apply`; corpus
  `crates/bumbledb-bench/src/windowed.rs` (the windowed/unwindowed twin
  worlds); no sweep exists yet.
- **Context, sharpened by reading:** the landable sort is the **source side
  only** — `check_source` iterates `plan.inserts` in the delta's
  `(relation, fact_hash)` BTreeMap order, i.e. effectively random U-key
  order; the target and window check lists are already BTree-sorted. T8
  found probe order indistinguishable at bench commit sizes but never swept
  commit size to find where sorting starts paying.
- **The determinism obligation (the citation-order contract, stated
  exactly):** `error.rs :: Violations::seal` stable-sorts and dedups by
  `Violation::citation`, and the lean side compares verdicts by list `BEq`
  (`lean/Main.lean :: RVerdict` — ascending statement indices, each violated
  statement of the failing phase cited once). So the citation *list* is
  probe-order-invariant by construction. **What is NOT invariant is the
  surviving witness:** the stable sort keeps the first-discovered
  `fact`/`incumbent` per citation, so re-ordering probes can change the cited
  fact bytes while the citations stay identical. The sweep's twin must PROVE
  witness stability (or rule witness choice explicitly non-normative) with
  multi-violation fixtures — conformance judgment cases compare the verdict
  whole.
- **Done:** the sweep exists; landed-if-wins or the gravestone gains its
  measured curve.
- **Assembler note (2026-07-16):** the sweep lane exists —
  `bumbledb-bench sweep-commit` (`crates/bumbledb-bench/src/sweep.rs`,
  `driver/sweep_commit.rs`), and witness stability is pinned by
  `crates/bumbledb/tests/witness_stability.rs` (its header documents the
  one assertion a landed source-side sort must flip, plus the
  `pin_hash_model` twin in sweep.rs). The verdict run is the measured
  stage's: `scripts/measure.sh cargo run --release -p bumbledb-bench
  --features obs -- sweep-commit`.

### W9 — bimodality mechanism hunt (no unexplained behavior ships)
- **Symptom:** `slot_booking_overlap` and `postings_without_tag` flip between
  two performance modes across whole bench processes (per-pair A/B ratios
  0.34–2.01 on *identical binaries*). Min-of-3 selects the fast mode
  symmetrically, so it is not a regression — but it is unexplained, and
  nothing in the bench code marks it.
- **Fix:** name the mechanism. Candidates, in falsifiability order: LMDB
  store page-state across process restarts (the two families are the
  report-only lanes most sensitive to B-tree layout); the measured ~35%
  code-placement relink lottery (`m2max` ledger); something else. Fix if
  engine-side; pin as external/environmental (a recorded disposition with
  replay evidence, the fuzz-estate convention) if not.
- **Done:** mechanism named; fixed or pinned-as-external. Nothing unexplained
  remains.

### W10 — allocation-census hoistables
- **Sites (all verified by reading):**
  `storage/commit/plan.rs :: fact_op` (the per-fact `edges`/`memberships`
  Vecs and their `into_boxed_slice` shrink-reallocs; the per-fact
  determinant-ops box and its `DeterminantImage` clones) and
  `plan_commit` (the delta-iterator `collect`s with inexact size hints →
  growth reallocs); `storage/keys.rs :: DeterminantImage` (a typical 8-byte
  determinant clones as a tiny heap Vec — a small-buffer representation is
  the fix shape, **keeping the codec-only constructor discipline** recorded
  on the type); `storage/commit/judgment.rs :: check_target` (the per-call
  `inserted` BTreeSet and the `affected` set owning key copies —
  `plan.inserts` is already sorted, so a sorted-slice binary search can
  delete the first set entirely).
- **Context:** warm *execute* is already zero-alloc census-wide (the gate's
  floor holds). These are plan/commit-side; none load-bearing at current
  scale, but 1.0.0 = zero known.
- **Constraint:** `commit_bounded` re-runs `apply` from the immutable plan on
  `CommitSync` retry — hoisted mutable scratch must keep the attempt closure
  re-entrant. Storage is not on the unsafe allowlist; fixes stay safe Rust.
- **Done:** the census harness (`tests/alloc_census.rs`, the `CENSUS |` and
  `SITE` rows) shows the reduction; correctness untouched.
- **Assembler note (2026-07-16):** landed; one premise correction —
  `plan.inserts` is in `(relation, fact_hash)` order, NOT byte order, so
  the `check_target` fix adds a byte-sorted index on the plan (one exact
  allocation per commit) rather than binary-searching `inserts` directly.
  `delta.rs :: record_determinants` became allocation-free as a side
  effect of the `DeterminantImage` small buffer.

### W11 — lean proof: the FilterPredicate transport (range-summary narrowing)
- **Site:** `lean/Bumbledb/Exec/Rewrites.lean`, § the range-summary fold.
- **Exact state (verified):** the word-level development is COMPLETE —
  `WordRange.narrow_mem`, `fold_mem`, `emit_mem`, the headline
  `range_summary_replacement`, and its two faces `range_pin_subsumes` /
  `range_fold_empty` are proved theorems, case-for-case with
  `ir/normalize/fold.rs` (including the `lt 0`/overflow `mark_empty` edges).
  **The recorded narrowing is precisely the TRANSPORT**, three steps:
  (1) `FilterPredicate`-list semantics over `Values` ↔ `WordBound`-list
  semantics via the order embeddings (`encode_u64_order_embedding` /
  `encode_i64_order_embedding`); (2) the in-place splice discipline (`emit`
  lands bounds at the first constituent's position — the filter-order law);
  (3) the u64/i64 encoding transfer. The fold-off dual-pipeline fuzz
  differential is the narrowing's empirical arm.
- **Fix:** attempt the transport theorem in earnest under the zero-sorry law
  (no `sorry`/`admit`/`axiom`, ever — `lean.sh` batteries 1–2 enforce).
  Either a completed theorem (Bridge ledger row updated, census green), or
  the exact stuck goal state recorded verbatim in the module doc and the
  narrowing kept — a narrowing is the spec's sanctioned form, but it must be
  *earned* by a real attempt, not assumed.
- **Done:** `lake build` green, batteries green; theorem or recorded goal
  state.
- **DONE (2026-07-16):** the transport is a completed theorem —
  `filter_fold_transport` (`lean/Bumbledb/Exec/Rewrites.lean`), Bridge
  ledger row 93, census green. The one honest gap the module doc records:
  the engine-side `fold_occurrence` premise shape (computing the
  replacement from the filter list) is mirrored as data, not executable
  Lean.

### Hygiene (fold into A3, concrete as of 2026-07-16)
- Delete the 23 stale `worktree-wf_*` branches and the 3 `codex/*` branches
  (local; check remotes for the codex three before deleting there).
- Remove the stale worktree at `../bumbledb-worktrees/prd-constitution`
  (`git worktree remove`), consistent with the no-worktrees ruling.
- Clear `/tmp` twin debris: `bench-twin-t3/`, `twin_probepass.txt`,
  `twin.dump`.
- The census citation sweep ships with A1 (see §3 locks).
- OPEN-ledger final pass — see Phase C; every row ends
  fixed/refuted/fired/declined, nothing "pending."

---

## 5. PHASE A-FUZZ — saturate the machine, make generation smarter

**Owner ask:** fuzzers blazing on ALL cores (M2 Max = 12), and the generative
fuzzer more likely to find deep bugs.

**The parallelism finding (investigated, not assumed):** `scripts/fuzz.sh`'s
default IS 12 (`FUZZ_WORKERS:-12` → `-fork=$WORKERS`, every lane, ASAN
included). The observed ~2-core sessions have three real causes:

1. **The co-tenant habit**: every logged session in `fuzz/SESSIONS.md` ran
   with `FUZZ_WORKERS` exported at 2/4/8 — per-session choice that became a
   de facto default. Nothing persistent sets it; the fix is operational, not
   code: the dedicated hunt exports nothing and takes the real default.
2. **Fork-mode's serial-merge floor**: libFuzzer's parent merges each job's
   corpus single-threaded; on short slices with large seed corpora the merge
   dominates (SESSIONS.md shows theory at 1m: 2→4 workers = zero exec
   scaling). The hunt must run LONG slices (≥30m per target) so merge time
   amortizes — the 12m×8 rows already scale far better than the 1m rows.
   Verify saturation empirically: `top` ~1200% during the run.
3. **The crash sweep in `check.sh` is structurally 2-parallel** (two sweep
   parents, each a serial child-spawning loop) — that is the per-commit gate,
   not the hunt; it is fine as it is and is not the thing to "fix."

**Smarter generation (targeted where it actually helps — the audit says the
bounce-off-validator concern is real for exactly two generators):**

- `fuzz/src/theorygen.rs` and the hostile arm of `fuzz/src/irgen.rs` are
  structurally free by design (the validator IS their oracle), but
  acceptance-reachability is luck-biased. Add a **well-formed-but-adversarial
  bias tier**: valid ids, resolvable names, in-roster handles,
  legal-but-extreme windows/selections/widths — hostile values inside
  accepted shapes, so executions reach past the validator into the engine.
  Keep the fully-hostile tier; the mix is the knob.
- `querygen`/`opgen` are already **valid by construction** (illegal cells
  unemittable) — do not "fix" them; a validation failure there is a generator
  bug by contract.
- **Program-shaped hostile IR is a recorded gap** (`fuzz/src/query.rs` notes
  Edb-only atoms today): extend `irgen` over the recursion roster
  (`Idb` atoms, strata shapes, the `MAX_PREDICATES` fence) so the program
  validation roster and `strata` judge get adversarial coverage.
- **Dictionaries + corpus seeding:** a dictionary of interesting values
  (encoding boundaries, `MAX_EXTENSION_ROWS`, width edges 0/64/65, ray
  sentinels), and seed corpora from `lean/conformance/cases/` (270 cases of
  known-interesting structure).
- **Measure the improvement**: coverage-per-exec and corpus growth vs. the
  current generators, recorded in `fuzz/SESSIONS.md` as an A/B session pair —
  a generation change without a measured coverage delta is vibes.

**Sequencing (device honesty):** harden + verify saturation NOW; run the long
all-cores hunt AFTER the perf A/B sessions land — a 12-core fuzz storm swamps
interleaved measurements far past the ±2% band. The hunt is a dedicated
session on the idle machine; findings triage per the fuzzing charter (stop,
minimize, trophy row + regression test or recorded disposition, artifact
deleted).

---

## 6. PHASE R — the rebirth tail (primer-side; the concrete meaning of "rebirth green")

The planning session launched the 18-PRD ultracode workflow and ended
mid-flight after reviving a stalled driver agent (four attempts died on
context exhaustion; the file work survived each time). The workflow was
verified **live** at 2026-07-16 19:45 (worktree files moving, driver finish
brief executing). The owner's merge bar, verbatim: **"everything must be
perfect, make sure all of these defects are fixed before the mega merge back
into main."** One caveat carried from that session: the owner observed a
silent model downgrade near its tail — treat the session's triage lists below
as unverified input and **re-verify every item against the tree** rather than
trusting the record.

### State (verified against the worktree, 2026-07-16 19:45)

- **Done:** engine PRDs 01–02 (`ee5d1de4` here); SDK PRDs 03–08 (65/65, the
  newtype migration — `.as` deleted, declaration-first blocks everywhere);
  store schema, prompts, gates, ETL, seats, lean refit; the unit-prose
  restoration (schema `description`/`scope` columns + enrich contract +
  prompt render + ETL write + both benchmark seeds, 15/15 tests); the
  driver-side unit-prose insert (`dispatch.ts` renders
  `scopeContractProse(unitEmission.scopeContract)`); `driver.ts` imports and
  documents quiescence-gated `etlRun`; `driver/supervisor.ts` exists
  (PRD-17's landing).
- **Remaining:** driver fixture tests + driver typecheck confirmation; the
  TUI (PRD-16 — no `tui/` exists yet); the funeral (PRD-18: the kill list,
  repo-green restoration, the `rg`-sweep assertions); the three read-only
  adversarial reviewers (SDK bijection fidelity vs the engine's normative
  docs; rebuild correctness vs PRDs 09–18; house-law compliance over the
  whole diff); the fixer applying confirmed findings.

### The pre-merge gates (pinned in-session as tasks #8–10; verify each personally)

1. **Gate #8 — ETL wiring:** `runGraphBuilder` calls `etlRun` at quiescence
   (the import and doc exist; verify the call site and its fixture test).
2. **Gate #9 — unit-prose end-to-end:** the restored description/scope path
   flows through the driver insert; the ETL fixture pins
   `course_units.description` = store `unit.description`.
3. **Gate #10 — perfection:** all six green gates + the PRD-18 sweeps + the
   `.as`-straggler check (the only sanctioned survivor is arkregex's
   unrelated `regex.as` escape hatch); every reviewer finding fixed or
   skipped-with-written-reason; the ordering law honored — if the fixer
   touches bumbledb, the engine commit lands and pushes HERE first; only then
   the mega-merge of `graph-builder-rebirth` into primer main; then the
   rebirth worktree AND the bumbledb symlink are deleted **together**.

### Directed reviewer items (from the session's harvest — re-verify, then fix or record)

- The architect's Lean manifest must derive from the same store rows the
  prompt renders (the unit-ref roster drift risk).
- The cartographer's system/task prompt pairing must never mix packages.

### Accepted-by-design roster (recorded, not fixed — re-confirm each reason holds)

- `DiagKind` sealed at 62 handles (a fingerprint change means old stores
  can't reopen — fine: run stores are disposable).
- Mutual recursion unwritable on the query surface (self-recursion only; no
  current consumer needs SCCs).
- No public brand-mint constructor for literals (reads and fresh-mints cover
  every current path).
- The `PreparedQuery` lifetime-erasure pattern in the FFI (standard bindings
  idiom, pinned by Arc ownership — a review lens double-checks it).

### If the workflow dies again

Do not rebuild — the file work survives in the worktree. Resume from the
workflow journal (the planning session's task directory,
`/private/tmp/claude-501/-Users-bjorn-Documents-primer/bfa5cd61-*/`) with
finish-scoped briefs, or hand-finish straight down this section. The gates
above are the completion contract either way; the driver stall taught the
pattern (a heavy PRD re-read from scratch exhausts context — brief the
*finish*, not the build).

---

## 7. PHASE B — primer: rename + verification (GATED on R)

1. Rename the SDK **`@superbuilders/bumbledb` → `@bjornpagen/bumbledb`**
   everywhere (package.json, every import specifier across the rebuilt
   pipeline, turbo refs, lockfile). Rationale: the SDK scope follows the
   engine's owner (`bjornpagen/bumbledb`), not the consuming org.
   `private: true` STAYS (no registry yet).
2. Verify the path-dep's post-merge resolution: with the rebirth worktree and
   symlink gone (R, gate #10), primer main's committed
   `../../../../bumbledb/crates/bumbledb` reaches this repo directly — build
   the native module against the post-refactor engine and prove the A1
   facade's promise (zero behavioral diff). SDK tests green; primer
   typecheck/knip green. Under the merge-back law: if any real copy of
   bumbledb has appeared anywhere by then, diff-audit and merge back before
   anything is deleted.
3. Add the two pins the SDK owes:
   - **TS-render ⇄ manifest-render golden:** TS `renderStatement`/
     `renderWindow` output equals the engine-rendered spelling the manifest
     ships, for every construct.
   - **The cross-host fingerprint lock:** a JS-created store (via
     `SchemaSpec`) opened from Rust via an identical `schema!` theory —
     fingerprint equality asserted across the FFI. The one test neither
     surface can fake.

---

## 8. PHASE C — the re-upstream census (GATED on B)

Read the rebuilt pipeline (rebirth PRDs 09–17 outputs) hunting engine
workarounds — marshaling contortions, missing manifest data, rejection-wire
gaps, delta-model friction in the repair loop (PRD-14 is the stress case).
**Every finding lands engine-first** (bumbledb, through full gates, pushed),
SDK adapts after — primer never patches around the engine.

**The OPEN ledger, judged against the real consumer** (`70-api.md` § the
freeze; current states verified):

| Row | State today | Phase C verdict rule |
|---|---|---|
| `tx.insert_all` batch sugar | deferred, unfired | graph-builder's code reached for it → fires, lands NOW; never reached → **declined vocabulary**, recorded |
| multi-key typed `tx.get` | deferred, unfired (macro emits `FreshKeyed` only for one-fresh-field relations, citing this row) | same trigger law |
| answer sorting / `FromAnswers` | deferred, unfired (host-side, `bumbledb-query` territory) | same trigger law |
| `write_from` retry helper | **resolved by refusal** (host policy; blessed snippet pinned by `bumbledb-query` cookbook test) | re-confirm against the real consumer, close |
| multi-process | resolved-as-deferred with trigger (out of envelope v0) | trigger unchanged, close as-is |

"Not built" is a correct *resolved* state under the owner's ratified trigger
law — unfired speculative sugar would itself be debt.

---

## 9. PHASE D — 1.0.0 close (GATED on C)

1. Full gates + fuzz smoke on the post-refactor, post-census tree.
2. **Full bench session + all five charts regenerated.** Unlike prior
   re-earns, numbers are EXPECTED to move (W1–W3 are collectively bigger than
   the whole layer-law campaign) — re-true the README claims and any surviving
   normative numbers, don't just re-earn ALL-WIN.
3. OPEN ledger final state (every row fixed/refuted/fired/declined).
4. Version bump to `1.0.0` (workspace Cargo.toml), commit summary, prep the
   annotated tag. **The owner pushes the tag** — the release ceremony is the
   owner's, and 1.0.0 is the owner's decision, not a gate's.

---

## 10. PHASE E — npm, literally last (owner's explicit word only)

Only after: the tag exists, the SDK has survived real graph-builder production
runs, and the name/scope/API have had their full window to change. Then: flip
`private`, version pinned to the tagged engine, provenance on, publish
`@bjornpagen/bumbledb`. Maximally reversible until this one step, which is why
it goes last.

---

## 11. Dependency graph (real dependencies only — no phasing theater)

```
A (refactor ∥ improvements ∥ fuzz-harden)   R (rebirth tail, primer-side)
   fires now, directly on main,                in flight NOW — driver finish,
   each item pushed when gate-green            TUI, funeral, reviewers, fixer,
        │                                      gates #8–10, mega-merge,
        │                                      worktree+symlink deleted together
        │  (measure.sh serializes only the     │
        │   TIMING sessions; every push        │
        │   keeps the SDK-consumed surface     │
        │   compile-compatible — the rebirth   │
        │   builds this live tree through      │
        │   the symlink; merge-back law        │
        │   absolute; ordering law: engine     │
        │   commits precede their consumers)   │
        ▼                                      ▼
        └──────────────┬───────────────────────┘
                       ▼
B (primer rename + post-merge verification) — GATED on R
        ▼
C (census) → D (1.0.0 close, owner tags) → E (npm, owner word)

fuzz all-cores HUNT: after A's perf A/B sessions land (idle machine), before D.
```

Within any phase: gates before pushes; pushes immediate; rulings surfaced in
conversation before code; adversarial review on every claimed win.

**Exit criterion (the release floor):** grep the repo for a known defect, a
measured-but-unclaimed win, an unexplained behavior, or an unresolved ledger
row — and find nothing. Then it is the owner's call.
