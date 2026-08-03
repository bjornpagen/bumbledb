## The architecture estate still declares interval-overlap joins O(n)-decided with the range-accelerator OPEN item armed — the shipped max-end index (be405715) is recorded nowhere in docs/architecture

incoherence | medium | CONFIRMED | overlap-join-live
outcome: fixed 6914471b

### Summary

Commit be405715 (2026-07-24) shipped the order-based interval-overlap accelerator — a per-key-group start-sorted max-end index that replaces the `Σ n_k²` all-pairs walk with `~O(log n_k + out)` per outer row for connected Allen masks with a const operand — and both code headers record it as the discharge of the 40-execution range-accelerator OPEN item. The architecture estate was never amended: three places (plus one cross-reference) still state the pre-discharge world where overlap joins are O(n) by decision, the OPEN item is armed "on violation", and the only recorded trigger-day candidate is the determinant skip scan. This is a direct violation of doc law rule 5 (README.md:28-29: implementation-contradicting docs are amended in the same change or the code doesn't land), and it leaves the OPEN roster (README.md:25-26: "An OPEN item is a real state") describing a trigger that already fired.

### Evidence (verified file:line)

Code claims discharge:
- `crates/bumbledb/src/exec/run/overlap_leaf.rs:1-8` — "(ruled 2026-07-23; finding 012 — the 40-execution range-accelerator OPEN item, **discharged**): ... the start-sorted max-end index (`interval::overlap`) replaces the `Σ n_k²` all-pairs walk with `~O(log n_k + out)` per outer row."
- `crates/bumbledb/src/interval/overlap.rs:1-8` — "the `docs/architecture/40-execution.md` range-accelerator OPEN item, **armed 'on violation' and tripped at bench scale by `t2_overlap_join`**."
- Commit `be405715` — adds `interval/overlap.rs` (370 lines), `overlap_leaf.rs` (166 lines), 471 lines of interval tests; wired live at `crates/bumbledb/src/exec/run/run_node.rs:140-180` (`overlap_enumerate` / `overlap_gather` dispatch).

Docs claim the opposite:
- `docs/architecture/40-execution.md:90-94` — "**Time-range scans, point-membership scans, and interval-overlap joins are O(n)** (image scan + filter) in v0 — decided; ... the range-accelerator OPEN item ... triggers on violation."
- `docs/architecture/40-execution.md:99-105` — "Candidate mechanism recorded for trigger day: **determinant skip scan**" — the shipped max-end index is absent; grep for `max-end` / `OverlapCache` / `order-based` over `docs/architecture/` returns nothing.
- `docs/architecture/README.md:84-90` — OPEN roster bullet: "overlap scans are O(n) by decision; accelerators return only with a benchmark that demands them" (the benchmark came — `t2_overlap_join` — and the accelerator returned).
- `docs/architecture/00-product.md:97-98` — "O(n) time-range, membership, and overlap scans must fit this budget or the range/stabbing-accelerator OPEN item triggers."
- `docs/architecture/40-execution.md:551` — a fourth stale citation: the grounding refusal is justified "like the range accelerator's trigger discipline", citing the discipline as still standing.

Scope refinements:
- The mechanism's single doc-side trace is `docs/feature-register.md:275` (verdict 18, temporal capacity laws), which cites "the order-based per-key overlap index (finding 012)" as an already-understood mechanism — confirming the register knows the index exists while the architecture roster does not.
- The partial-closure framing is correct: only overlap **joins** (connected mask ⊆ INTERSECTS, one side an outer-binding constant, above the crossover) are accelerated; interval **stabbing** and time-range scans remain O(n) — `docs/architecture/60-validation.md:341` still lists `busy_scan` as "the range-accelerator trigger's evidence."

### Failure scenario / impact

The next latency-budget or Hunt conversation reads the roster (README.md:84-90), concludes no interval accelerator exists, and either re-arms the trigger for machinery already in the tree or implements the recorded candidate (determinant skip scan) instead of extending the shipped index. Conversely, a reader of `overlap_leaf.rs`'s "discharged" header concludes the whole OPEN item — including stabbing, which `busy_scan` still exercises as O(n) — is closed. Every verify-against-in-repo-papers audit trips on the same contradiction until one side is amended.

### Suggested fix

One doc commit, per doc law rule 5's spirit (amended now, since the code already landed):
- `40-execution.md:90-106` splits the decided O(n) claim: interval-overlap **joins** with a connected mask and const operand now take the order-based per-group max-end index (`interval/overlap.rs`, crossover-gated, the mask staying data through the classify kernels); time-range and stabbing scans remain O(n) with the OPEN item narrowed to them. Fix the stale exemplar at line 551 to cite the narrowed item.
- `README.md:84-90` roster bullet re-scopes to the stabbing/range residue and records the fired trigger's outcome in present tense (no history — just the current mechanism plus the narrowed OPEN state).
- `00-product.md:97-98` re-scopes the budget sentence to the residue.
- `overlap_leaf.rs` / `interval/overlap.rs` headers stand as-is once the docs match; optionally soften "discharged" to "narrowed" since stabbing remains open.