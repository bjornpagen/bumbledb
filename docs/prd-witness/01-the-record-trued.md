# PRD 01 — The record trued: the pipelined executor enters the doc

**Depends on:** baseline only.
**Modules:** `docs/architecture/*.md`, comment-only edits in
`crates/bumbledb/src/exec/run/run_node.rs`, `crates/bumbledb/src/exec/`
(dead cross-references), `crates/bumbledb-bench/Cargo.toml`,
`crates/bumbledb/src/api/prepared/tests/chase.rs`,
`crates/bumbledb-bench/src/querygen/shapes_closed.rs`,
`crates/bumbledb/src/exec/kernel/allen.rs`.
**Authority:** `docs/architecture/README.md` rule "when code and these
docs disagree, one of them is wrong and the repo is broken"; the 2026-07-12
audit (Free Join fidelity + rulings tracks).
**Representation move:** none — this is the record repaying its debt. The
one large unrecorded divergence from the Free Join paper gets its Deviation
block, and every identified doc/comment lie gets trued. This PRD is the
set's only doc-led PRD, sanctioned because the audit's finding IS the doc.

## Context (decided shape)

The shipped executor is not the paper's §3.3 per-tuple recursion: middle
nodes `pump` pending binding-rows and carried cursor sets, `probe_pass`
probes siblings across parent entries in shared batches, and the D2
subtree skip is implemented as **origin cancellation below an absorb
node** (`exec/run/pump.rs`, `probe_pass.rs`, `cancel.rs`,
`pipe_tables.rs`). The code records itself honestly (`exec/run.rs` header:
"the paper's cross-node-entry accumulation caveat is retired"); the doc
still narrates recursion. Soundness argument to record verbatim: late
cancellations re-emit into the spanning seen-set, so cancellation is pure
work-skipping — set semantics make it correctness-free.

## Technical direction

1. `docs/architecture/40-execution.md`:
   - The adopted-core narration ("recurse node by node… backtrack
     restores"; "bindings written in place by the recursion"; "the
     recursion unwinds on the sink's first-emit signal") is rewritten to
     the pipeline shape: pump / probe_pass / absorb node / origin
     cancellation, with a **Deviation:** block naming the paper's §3.3
     recursion as the departed baseline and the soundness argument above.
   - The D4 vectorization caveat "large batches are reliably available
     only at the root; cross-node-entry batch accumulation is future
     work, not assumed" is retired (it is false; deep nodes see full
     batches) — the caveat's retirement is stated, not silently deleted.
   - The "`binary2fj` + conservative `factor()` — exactly per paper"
     bullet gains one sentence: `factor()` corrects Fig. 8's literal
     pseudocode (which would visit the cover first, fail the
     `α.vars ⊆ avs(φ)` test, and abandon the node against the paper's own
     worked example); the engine starts candidates at index 1 per the
     paper's prose intent.
2. Comment truth, code side:
   - `exec/run/run_node.rs` (two sites near the cover-choice function):
     the comments describing the REJECTED label-first rule ("prefer the
     smallest Exact, else the smallest Estimate") are rewritten to the
     magnitude-first rule the code implements and the doc records.
   - Sweep every `30-execution` cross-reference in product code and the
     normative architecture record (the preflight found more than the
     audit's ≈10-site estimate:
     `exec/colt.rs`, `exec/run/run_node.rs`, `exec/kernel.rs`,
     `image/view.rs`, `ir/validate.rs`, and any grep finds) to
     `40-execution`.
   - `crates/bumbledb-bench/Cargo.toml` dev-dep comment: the dual-run
     chase differential lives at `src/differential/tests/chase.rs`, not
     `naive/tests/chase.rs`.
   - `api/prepared/tests/chase.rs` "(docs/prd — the chase surface)" →
     `docs/architecture/40-execution.md § the chase`.
   - `querygen/shapes_closed.rs` "adversarially covered before the fold
     lands" → the fold landed; present tense.
   - `exec/kernel/allen.rs` "PRD 16's calendar family" → "the calendar
     family".
3. Chapter staleness (each item one surgical edit):
   - `docs/architecture/README.md`: delete the "standing exception —
     the docs lead the code… the work plan is `docs/prd/`" paragraph
     (the directory does not exist; the code caught up). Add the chase
     interval-pair OPEN item to the aggregate OPEN list (rule 4 claims
     the list is complete; `40-execution.md` § the chase carries the item
     but the list omits it).
   - `docs/architecture/00-product.md`: the enum is not "mid-funeral";
     rewrite to past tense pointing at 10-data-model's obituary.
   - `docs/architecture/60-validation.md`: the ledger `Families:` prose
     lists ~11; the gate registry enforces 15 — add `containment_walk`,
     `stats`, `string`, `skew` to the normative list.
   - `docs/architecture/20-query-ir.md`: "seven types" → "six types" in
     the Eq/Ne legality sentence (count prose only; the variant set is
     already correct).
4. The p99 budget scoped (rulings audit item): `00-product.md` and
   `60-validation.md` state that the 10 ms warm-p99 budget **binds at
   scale L** (no L corpus has been generated) and is informational at S;
   `00-product.md`'s intra-query single-threading reversal trigger gains
   the sentence that the trigger is NOT armed by S-scale budget misses.
   The bench report's "FAIL (informational below scale L)" display is
   thereby documented, not changed.

## Passing criteria

- `[shape]` `grep -rn "30-execution" crates docs/architecture README.md`
  → zero hits. Historical research substrate and this execution packet
  are not normative record and are intentionally outside the search.
- `[shape]` `grep -rn "docs/prd\b\|docs/prd/" crates docs/architecture
  README.md` → zero hits; the architecture README carries no "standing
  exception". The execution packet is excluded because it necessarily
  names the retired path while specifying its removal.
- `[shape]` `40-execution.md` contains a Deviation block naming the §3.3
  recursion, the pump/probe_pass/absorb-node shape, the origin-cancellation
  soundness argument, and the retired D4 caveat; no sentence in the
  chapter describes per-tuple recursion as the shipped executor.
- `[shape]` The two `run_node.rs` cover-choice comments state
  magnitude-first; `grep -n "smallest Exact" crates` → zero hits.
- `[shape]` 60-validation's ledger family prose names all 15 gate
  families; 20-query-ir says six types; 00-product speaks of the enum in
  past tense; the p99 scale-L scoping sentences exist in both chapters.
- `[gate]` Workspace gates green at campaign close (comment/doc-only PRD;
  the gate is drift protection).

## Doc amendments (rule 5)

This PRD is its own amendment list; nothing rides elsewhere.
