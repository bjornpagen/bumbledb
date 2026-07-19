# PRD-U5 — Lean reconciliation

Wave 2 · Repo: bumbledb (`lean/`, findings) · depends on: U1, U2 (the engine
end state must exist before it is judged)

## Objective

The three surfaces — the Lean spec (`lean/`), the Rust engine, and
`docs/architecture/` — agree after the wave's engine changes, **or every
disagreement is a documented finding with evidence of which side strayed**.
Lean is the semantic authority; a code-vs-lean disagreement is NEVER silently
fixed toward whichever side is easier.

## Work

1. **Sweep the wave's semantic deltas against the spec.** U1 and U2 were
   chosen to be semantics-preserving; prove it, don't assume it:
   - U1: does anything in `lean/` state or depend on the ephemeral kind's
     flags, map size, or the capacity contract? (Expected: no — durability
     flags are below the spec's abstraction line. Verify by grep + reading
     the storage-adjacent theorems, and say so.)
   - U2 kill 5 (format-version check unification): the doc-lawed refusal
     ORDER (`FormatMismatch` → `StoreKindMismatch` → `SchemaMismatch`) — if
     the spec or conformance suite pins it, confirm the unified helper
     preserves it.
   - U2 kills in exec/image (ZST counters facade, `apply_infallible` guard,
     `refill`≡`append`, scan delegation): confirm none crosses a theorem's
     statement; the three-way conformance test is the referee.
2. **Run the full battery**: `scripts/lean.sh` (build, zero-sorry/axiom,
   spec-census, conformance, three-way) — green is the floor, not the proof;
   the battery does not see prose drift.
3. **The census's standing lean-adjacent questions**, dispositioned:
   - The key-probe/Free Join classifier is Lean-proved ("Reverses if: never")
     — untouched by this wave; confirm no U2 kill grazed `exec/dispatch/`.
   - If M later merges a measure-or-merge twin (rulings 6–8), the merge is
     semantics-identical by the differential oracle — note here that U5 does
     NOT pre-clear it; M's merge path re-runs this PRD's checklist.
4. **Findings protocol.** Each disagreement found: a finding in the PR body
   (surface A says X at cite, surface B says Y at cite, evidence of which
   strayed and since when — git archaeology allowed), plus a TODO.md entry if
   it outlives the wave. Fixes to the STRAYED side only, each its own commit,
   each citing the finding.

## Passing criteria

- `scripts/lean.sh` exit 0; `scripts/spec-census.sh` exit 0.
- A written disposition (in the PR body) for every item in Work 1 and 3 —
  "verified silent" is acceptable ONLY with the grep/read evidence stated.
- Zero silent cross-surface fixes: every reconciling edit names its finding.
- Zero new axioms, zero sorries, no theorem weakened or deleted.
