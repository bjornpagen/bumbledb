## The generic-join half of Free Join is unreachable: no plan constructor ever splits a probe subatom, so cyclic queries always execute the binary-join closing probe

category: missing-free-feature | severity: high | verdict: CONFIRMED | finder: perf:rings
outcome: fixed 73215a30 (gj_split + second covers; the step-2 per-forced-map key fence never landed — recorded in TODO)

### Summary

The engine built the entire execution-side apparatus for the generic-join (WCOJ) end of the Free Join plan spectrum — multi-level trie schemas per occurrence, carried cursors across nodes, and per-entry dynamic cover choice by key count — but the only production plan constructors are `binary2fj` (paper Fig. 7) and `factor` (paper Fig. 8), neither of which can split a probe subatom into per-variable pieces. Consequently every cyclic query (triangles: r1, r2, r3, r4, r6-shaped closings, g4/g5) executes the binary-join closing probe: a monolithic multi-word probe into the closing relation's full-map root. The GJ-style split plans that the validator explicitly admits and the executor demonstrably runs (there is a passing end-to-end test on a hand-built split plan) are never produced. A corollary verified during this audit: **every production plan node has exactly one cover**, so the dynamic cover-choice loop in `pump` — the mechanism the Free Join paper says is required "to match the optimality of GJ" — never has a choice in production.

### Evidence

All citations verified directly against the code.

**No producer can emit a split plan:**
- `crates/bumbledb/src/plan/fj/binary2fj.rs:40-53` — the probe subatom takes ALL available vars: `let probe: Vec<VarId> = vars.iter().copied().filter(|v| available.contains(v)).collect(); ... current.subatoms.push(Subatom { occ: next, vars: probe });`
- `crates/bumbledb/src/plan/fj/factor.rs:20-31` — hoists whole subatoms only (`plan.nodes[i].subatoms.remove(1)`), never splits one; a closing atom like T2(v2,v0) can never hoist because v2 is unavailable before its node.
- `crates/bumbledb/src/api/prepared/build.rs:743-744` — `binary2fj` + `factor` is the sole production plan path; a repo-wide grep finds no other non-test `FjPlan` constructor.

**The GJ end is admitted, validated, and executable today:**
- `crates/bumbledb/src/plan/fj/validate.rs:245-267` — the partition check explicitly allows one occurrence's variables spread across subatoms in different nodes (disjoint, union = the occurrence's var set).
- `crates/bumbledb/src/plan/fj/validate.rs:66-73` — trie schemas are derived from subatom var-lists in node order (§3.3 of the paper), so a split occurrence automatically gets a multi-level trie.
- `crates/bumbledb/src/plan/fj/derive_nodes.rs:29-46` — the cover rule's own comment: restricting covers to exactly-the-new-vars "keeps every binary2fj node's opening subatom ... and every GJ-style single-var cover."
- `crates/bumbledb/src/exec/run/pipe_tables.rs:25-44` — `entry_level` counts an occurrence's appearances in earlier nodes and `carried`/`carried_col` route its advanced cursor forward: multi-node occurrences are fully supported.
- `crates/bumbledb/src/exec/run/pump.rs:70-88` — per pending entry, every cover's `key_count` is compared via `better_cover` and the smallest wins — exactly the paper's dynamic cover choice.
- **Test evidence:** `crates/bumbledb/src/exec/run/tests/mechanics.rs:4-66` (`dynamic_cover_prefers_the_forced_small_side`) hand-builds the GJ split plan `[[R(x), S(x)], [R(a)], [S(b)]]`, executes it end-to-end, and asserts correct rows plus the dynamic cover choice; `crates/bumbledb/src/plan/fj/tests/validate.rs:230-260` validates the clover GJ plan with `covers == vec![0, 1, 2]`. The suggested lowering therefore needs zero executor or validator changes.

**Corollary — dynamic cover choice is production-dead:** a cover requires `vars == new_vars` exactly (`derive_nodes.rs:42-44`); every non-opening subatom in a `binary2fj`+`factor` plan carries only previously-available vars, which are disjoint from `new_vars`. So `node.covers` has length 1 in every production plan and the pump.rs:70-88 loop never has an alternative to compare.

**Bench numbers (all re-verified):**
- `crates/bumbledb-bench/src/scenarios/rings/corpus.rs:26-31` — bomb tier 2 m=384, rows = 2m²+3 = 294,915; comment: "m³ ≈ 5.7e7 closing probes".
- `bench-out/night-2026-07-20/scenarios/scenarios.md` line 57 — r4_bomb_t2 p50 = 1,576,952.3 µs ≈ 1.58 s; 1.577e9 ns / 5.66e7 probes ≈ 27.9 ns/probe (DRAM-class).
- `crates/bumbledb/src/exec/colt.rs:158-190` — bucket-of-8 map, stride `8·arity + 8` words, sized to ≤ 0.4 load: 294,915 two-word keys → 131,072 buckets × 24 words × 8 B ≈ 25.2 MB bucket slab for T2's full-map root. A per-v0 submap in a split plan holds ~384 keys (~128 buckets × 16 words × 8 B ≈ 16 KB, L1/L2-resident).
- `crates/bumbledb-bench/src/scenarios/rings.rs:325,349,386` — the bench narrates r1/r4 as "the binary-join exponent" while the engine's own machinery was built to erase it.

**Paper and doc checks (docs/free-join-paper/arXiv-2301.10841v2/tex/):**
- `04-optimizations.tex` (Fig. 8 discussion): the paper's `factor` also hoists whole subatoms — the engine transcribes the paper's published optimizer faithfully. But the same section's COLT part states: "in order [to] match the optimality of GJ, the FJ algorithm needs to choose dynamically the 'cover'", and its worked example is the triangle GJ plan `[[R(x), T(x)], [R(y), S(y)], [S(z), T(z)]]` — a shape this engine validates and executes but never produces.
- `04-optimizations.tex` (clover skew example, lines ~90-105): a binary-shaped FJ plan does n² work where the factored/GJ plan does O(n) — binary-shaped plans carry no AGM bound.
- `06-discussion.tex`: "an optimizer for FJ should smoothly transform a FJ plan to fully explore the design space between the two extremes" — explicitly future work in the paper; this engine inherited the gap.
- `docs/architecture/40-execution.md:1055-1056` — "they order the exhaustive DP while the WCOJ execution bounds the chosen plan's damage": overstated. For binary-shaped FJ plans (all production cyclic plans today) the execution is binary hash join and bounds nothing; only plans at or near the GJ end carry the worst-case-optimal guarantee.

### Bench impact

- **r4_bomb_t2** (measured 1.58 s): the split plan `[[T0(v0,v1), T1(v1), T2(v0)], [T1(v2), T2(v2)]]` keeps the ~5.7e7 probe count (the A/B candidate sets are disjoint, but intersecting two 384-key sets still costs min(m,m) probes per 2-path) — the win is locality: closing probes land in ~16 KB per-v0 submaps instead of the 25.2 MB full-map slab, ~28 ns → ~5 ns per probe class, a multi-x reduction. The asymptotic collapse to O(m²) additionally requires the suggested per-forced-map min/max key fence (the bomb's candidate sets are provably disjoint by range).
- **r1_wash_ring / r2** (measured 62.5 ms p50): a genuine probe-count reduction — node1's cover set becomes {T1(v2), T2(v2)} and pump's existing magnitude-first choice yields cost min(outdeg(v1), indeg(v0)) per 2-path instead of outdeg(v1), the classic WCOJ skew win under the corpus's 15%-hub power law (hub out-degree ~450+ vs normal in-degree ~3).
- r6 and the g4/g5 triangle lanes share the closing-probe shape.

### Suggested fix

Add a plan-lowering step after `factor` for cyclic rules: when a probe subatom carries ≥ 2 variables first bound at different earlier nodes, split it into per-variable lookup subatoms, each placed at the node where its variable is first bound. The trie schema follows automatically from the §3.3 derivation in `validate.rs::build_occurrences`; the partition check, `derive_nodes` covers, `pipe_tables` carried-cursor routing, and `pump`'s dynamic cover choice all already handle the result (proven by `mechanics.rs::dynamic_cover_prefers_the_forced_small_side`). Then, as a second step, add per-forced-map min/max key words at COLT force time as an O(1) disjointness fence before probe batches. Separately, correct the `40-execution.md:1056` claim: binary-shaped FJ plans are not damage-bounded by WCOJ execution.
