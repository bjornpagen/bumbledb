# PRD-I3 — The filter-mask decider twin

Repo: bumbledb (`crates/bumbledb/src/exec/kernel/`, test-side only) · depends
on: — · the masked kernel is TEST-LOCAL: nothing enters the product kernel
surface while the delete fork is gated · gates: `scripts/check.sh`; the timed
run is Wave M (`scripts/measure.sh`, idle machine, owner go).

## Objective

Price the validity-mask tax at its *cheapest* surface, once, so the gated
delete fork (§00-README ruling 2) reopens — if it ever does — with the number
already in hand. The fork's cost has two components: (i) the per-chunk
validity-AND on the filter surface (claimed ≈ free), and (ii) the
dense→gathered degradation on Identity/fold paths (already priced: 8.8 vs
4.0–4.6 rows/ns, `exec/kernel.rs:30-33`). This twin targets (i), because an
unfavorable result there kills the whole mask route in one afternoon — if the
mask is not free where it is cheapest, nothing downstream matters.

## The spec

**Kernel:** `filter_eq_u64` (`exec/kernel/filter.rs:29`) — representative of
the six survivor-producing filter kernels, the shape where the
one-op-per-chunk claim lives, with an existing measured anchor (the stride_ab
filter-surface 1.00×).

**Twin form:** an `#[ignore]`d in-process interleaved falsifier test beside the
kernel — the house pattern (`exec/colt/tests/pins.rs:108-121`,
`image/tests/stride_ab.rs`: "laid out side by side in ONE process"). Three
arms, line-for-line:

- **A (shipped):** `filter_eq_u64(col, v, out)` as-is.
- **B (masked, all-live):** a test-local `filter_eq_u64_masked(col, v,
  validity, out)` with `bits &= validity_bits(chunk)` fused after
  `to_bitmask()`, bitmap all-ones — isolates the pure tax at density 1.0 (the
  u64-word bitmap extraction across 4-lane chunk boundaries + the AND + the
  second load stream; zero semantic effect).
- **C (masked, holed):** same kernel, validity at 1/64 and 1/8 dead,
  uniform-random — the realistic tombstone regime; checks the survivor-write
  path under holes.

Arms interleaved per-draw, min-of-5 medians per arm. **Agreement asserted**
against a masked scalar reference twin (bit-identity — house law; the reference
and the property test are part of this PRD even though the kernel is
test-local). Two tiers: L2-resident (~2 MB column) and DRAM (~100 MB).
Selectivities 1% and 50% (survivor-write-bound vs scan-bound).

**Placement law:** the masked body lives in the test module (or a
`#[cfg(test)]` sibling), never in `exec/kernel/filter.rs`'s public surface —
`scripts/check-asm.sh`'s flag-free law audits the release disassembly of
shipped kernels, and a gated fork ships nothing. If placement forces it into
the compiled-for-release surface, that is a blocker to report, not a rule to
bend.

**Invocation (Wave M):**
`scripts/measure.sh cargo test --release -p bumbledb filter_mask_twin -- --ignored --nocapture`
— the measurement mutex and clock-proxy discipline inherited from the pins.rs
pattern; the test prints ratios; the verdict is read from the measured run,
never asserted as a timing assertion in the test.

## The decision rule (recorded verbatim with the result)

- **DISSOLVES the fork's kernel-tax half:** B/A ≤ ~1.03 on both tiers (inside
  the harness's demonstrated noise band — cf. the stride filter surface 1.00×).
  The filter surface's share of the re-earn bill becomes a formality; the
  fork's real cost concentrates entirely in the already-priced dense→gathered
  conversion, and the argument moves to workload arithmetic, not kernels.
- **CONFIRMS the fork's death:** B/A ≥ ~1.10 at either tier. The mask is
  expensive at its cheapest surface; compact-on-delete wins outright and the
  mask design dies without anyone touching folds, Allen, or NEON.
- **In between:** escalate to the second twin — the fold surface
  (`fold_sum_u64_dense` vs `fold_sum_u64_idx` over a live-position list at
  densities 1.0/0.99/0.9/0.5) — before any design decision.

**Honesty clause:** a favorable filter result does NOT clear the fold/Identity
surface — that degradation is real and already measured at ~2×. The filter twin
is the cheap FIRST decider because only one of its outcomes is survivable for
the mask route.

## The verdict filing

Whatever the outcome, the number and verdict are filed in the design notes
(the 00-README's ruling-2 trigger record — or its successor doc once this
packet is deleted), with tier, machine conditions, and the decision-rule branch
taken. The gated fork's reopen procedure cites this filing.

## Passing criteria

- The `#[ignore]`d twin exists beside the kernel, three arms, both tiers, both
  selectivities; the masked reference twin's bit-identity property test runs in
  the NORMAL (non-ignored) suite and is green.
- Nothing enters the shipped kernel surface: `scripts/check-asm.sh` output
  unchanged; no new public items in `exec/kernel/`.
- `scripts/check.sh` exit 0.
- Wave M (owner go, idle machine): the twin run under `scripts/measure.sh`,
  ratios recorded, the decision-rule branch named, the verdict filed. No perf
  claim asserted anywhere before that run — pending-measurement until then.

## Size

**S–M.** One test file (three arms + reference + property test), zero product
diff. The fold-surface escalation twin, if triggered, is its own follow-on.
