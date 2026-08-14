# lean-012: `Option Rec` + `derivedCount` treat rec-presence as a queryable flag

- **Severity:** medium
- **Tree:** lean
- **Status:** DUPLICATE(lean-001)
- **Source:** audit/lean.md M6

Every site this finding names is deleted by lean-001's sum (and lean-002's decoder work): `Query.derivedCount`'s `usize::from`-style flag arithmetic (`Syntax.lean:324-325` — `q.interiors.length + (if q.rec.isSome then 1 else 0)`), `recLinear`'s vacuous `none => True` arm (`Syntax.lean:483-484`), the `match q.rec` at both evaluators (`Reach.lean:752-756, 773-778`), and `decodeRecOpt`'s `Option` production (`Main.lean:393-398`, which lean-001 retargets to choose the `.cq`/`.reach` constructor — the JSON `rec: null|absent` spelling stays, per CONTRACT §C1). No edit remains that lean-001 does not already own.
