# lean-013: `InteriorEnv` is total over all `InteriorId`s — unread ids silently denote empty

- **Severity:** medium
- **Tree:** lean
- **Status:** WONTFIX
- **Source:** audit/lean.md M7

Refused under CONTRACT §C5 (ruling R-DENSE). The total environment (`Denotation.lean:691-697` — `InteriorEnv : InteriorId → Set AnswerTuple`, unread ids empty) is the deliberately-kept model of the open boundary: the spec models the hostile object the frozen corpus feeds, the phantom-read semantics (out-of-range interior reads denote empty, exact agreement with or without the screen) is a *recorded* behavior of the model, and keeping denotations total lets every theorem carry named premises instead of dependent indices across a 24k-line proof tree (Insights 15/16 — the `Fin`-scoped environment's bookkeeping costs more than the branches it deletes). The dual coordinates that made this feel dangerous (the unspent `sourcesInRange`/`interiorsDag` screens) die via lean-004; the totality itself stays.
