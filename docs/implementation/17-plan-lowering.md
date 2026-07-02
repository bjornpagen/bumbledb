# PRD 17 — binary2fj, factor(), and the ValidatedPlan

Authority: `docs/architecture/30-execution.md` (the paper's core, adopted;
ValidatedPlan contents), Free Join paper §3.2–§3.3, §4.1 (`docs/free-join-paper/…/tex/`
— read `03-free-join.tex` and `04-optimizations.tex` before implementing).

## Purpose

Lower the join order to a validated Free Join plan: nodes, subatoms, covers, trie
schemas, residual placement.

## Technical direction

- `plan::fj`. Types: `Subatom { occ: OccId, vars: SmallVec-ish Vec<VarId> }`,
  `Node { subatoms: Vec<Subatom>, covers: Vec<u8 /*index*/>, residuals: Vec<usize> }`,
  `FjPlan { nodes: Vec<Node> }`. Plain Vecs — **no fixed-capacity silent-drop
  containers** (post-mortem §35 is the cautionary tale; capacity bugs must be
  impossible, not silent).
- `binary2fj(&NormalizedQuery, &JoinOrder) -> FjPlan`: transcribe the paper's Fig. 7
  algorithm exactly (first node = full atom of the first occurrence; each subsequent
  occurrence contributes a probe subatom on vars ∩ available, then opens a node with
  its remaining vars).
- `factor(&mut FjPlan)`: the paper's Fig. 8 conservative hoist — reverse traversal,
  move a subatom to the previous node iff its vars ⊆ available-before-node and the
  previous node lacks that occurrence, stopping per-node on first non-hoistable
  (preserve probe order). Use an incremental legality check, not re-validation per
  move (post-mortem §38).
- Cover enumeration per §4.4: all subatoms containing every new var of their node.
- Residual placement: each residual attaches to the earliest node where both VarIds
  are bound (computable from per-node available-var sets).
- Trie schemas per occurrence: the sequence of its subatoms' var-lists in node order
  (§3.3 build-phase rule) — recorded for PRD 18.
- `validate(FjPlan, &NormalizedQuery) -> ValidatedPlan`: partition property per
  occurrence (its subatoms' vars partition its var set), per-node occurrence
  uniqueness, non-empty cover sets, residual bind-coverage. `ValidatedPlan` (sealed)
  additionally carries: binding-slot layout (dense VarId-indexed), the
  **provably-distinct-bindings flag** (every occurrence's bound fields cover one of
  its unique constraints — schema check), per-node available/new var sets, and the
  PRD 16 estimates.

## Non-goals

Execution. COLT. Cover *choice* (runtime, PRD 19).

## Passing criteria

- Unit tests: the paper's clover-query example — binary2fj on order [R,S,T] yields
  `[[R(x,a),S(x)],[S(b),T(x)],[T(c)]]` and factor() yields
  `[[R(x,a),S(x),T(x)],[S(b)],[T(c)]]` (transcribe from §3–§4 as the fixture); the
  chain-query example from §4.1 matches the paper's output; trie schemas match §3.3's
  worked examples (R vector; S map→vector; T map→map for the triangle plan); cover
  sets on the GJ-style plan match the paper's "for the first node we could have also
  chosen S(x) or T(x)"; residuals place at first-both-bound node; a self-join plan
  validates (occurrence-quantified); distinct-bindings flag set for a
  serial-bound fixture and clear when a non-unique field is the only binding.
- Global commands green.
