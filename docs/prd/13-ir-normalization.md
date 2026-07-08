# PRD 13 — Normalization and lowering

**Depends on:** 12.
**Modules:** `crates/bumbledb/src/ir/normalize/` (all files).
**Authority:** `docs/architecture/20-query-ir.md` (§ normalization — the five-step lowering).

## Goal

Normalization lowers the richer IR surface to paper form plus the two new output
kinds: membership/interval **per-atom filters** and **anti-probe filters** in the
residual list.

## Technical direction

1. Steps 1–3 (occurrence numbering, per-atom filter lowering, same-atom
   comparison lowering) extend to the new material:
   - A membership binding with a **constant** point (literal or scalar param)
     lowers to a per-atom range filter: `start_word ≤ p AND p < end_word` over the
     interval field's two column words. Represent as a new per-atom filter kind
     (alongside the existing `FieldsCompare`/range residual kinds in the view
     filter vocabulary): `PointIn { field, point: ResolvedWordSource }`.
   - A `ParamSet` binding on a **scalar** field lowers to the selection-level
     machinery (`PlanOccurrence::selections` — extend the selection resolution to
     carry a set of words; executor side is PRD 17). On an **interval** field
     (point set): a per-atom filter `AnyPointIn { field, set }`.
   - Same-atom `Overlaps`/`Contains` (both sides fields of one occurrence) lower
     to word-comparison filters over the four (or three) words:
     `Overlaps(a, b)` ≡ `a.start < b.end AND b.start < a.end`;
     `Contains(a, b: interval)` ≡ `a.start ≤ b.start AND b.end ≤ a.end`;
     `Contains(a, p: point)` ≡ `a.start ≤ p AND p < a.end`.
     Encode these as compositions of the existing word-filter primitives — do not
     invent a mini-expression tree; three fixed shapes as three filter kinds is
     the representation-over-control-flow answer.
2. **Step 4 (new): negated atoms** are numbered as occurrences (a distinct
   occurrence-id space or a flag on the occurrence table — keep one table with a
   `polarity` field; plan validity quantifies over positive occurrences only) and
   each lowers to an **anti-probe residual**: `{ occurrence, probe_bindings }`
   attached, like comparisons, to the earliest node where all its variables are
   bound (the attachment computation itself is plan-time — PRD 15; normalization's
   job is producing the anti-probe descriptor with its variable set). Negated-atom
   literal/param/set/membership bindings become the anti-probe's own filter list
   (evaluated inside the probe).
3. **Cross-atom interval residuals** (`Overlaps`/`Contains`/membership between
   different occurrences' variables): residual comparisons whose operands are
   interval-typed variables reference **two binding slots** each. Decide the slot
   representation here, once: an interval-typed variable occupies **two
   consecutive u64 slots** (start, end) in the VarId-indexed slot array; the slot
   layout map (built during normalization/plan witness) records widths. Every
   residual kind above decomposes into word comparisons against slot pairs.
   Document this in a comment as the load-bearing layout decision consumed by
   PRDs 15/16/17/18.
4. Output shape: distinct-variable positive occurrences + per-atom filters +
   residual list (word comparisons + anti-probes) — nothing single-occurrence
   survives to residuals (extend the existing assertion).

## Out of scope

Plan-node attachment (15), execution (16–17), slot-array allocation (existing
executor machinery, adjusted in 16/18).

## Passing criteria

- `[shape]` The three interval filter shapes exist as fixed kinds; no expression
  tree type was introduced.
- `[shape]` The occurrence table has a polarity field; no separate negated table.
- `[shape]` The two-slot interval variable layout is decided in exactly one place
  and exported to the plan witness.
- `[test]` Lowering goldens: (a) constant-point membership → `PointIn` filter;
  (b) same-atom Overlaps → the three-word filter composition; (c) negated atom
  with a literal binding → anti-probe descriptor whose filter list carries the
  literal; (d) cross-atom Overlaps → residual referencing two slot pairs;
  (e) scalar ParamSet binding → selection-level with set marker.
- `[test]` The single-occurrence-residual assertion still holds across the new
  kinds (property test over randomized lowerable queries if the existing harness
  has one; otherwise targeted cases).
