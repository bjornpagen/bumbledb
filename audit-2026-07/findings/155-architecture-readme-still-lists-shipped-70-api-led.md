## Architecture README still lists shipped 70-api ledger rows as OPEN sub-items

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:docs-vs-code-drift

### Summary

The architecture index (`docs/architecture/README.md`) carries two stale OPEN entries that contradict the owning doc, `docs/architecture/70-api.md`. The index's last OPEN item points readers to 70-api.md's "own OPEN list (result ordering, multi-key typed `get` sugar, multi-process future)" — but 70-api.md's ledger was declared **CLOSED (2026-07-17)**, and two of the three named rows are terminal: keyed get SHIPPED 2026-07-19 and answer sorting SHIPPED 2026-07-19 (both surfaces verified present in code). Separately, the index's own "Ordering/limit conveniences and top-k pushdown" OPEN item still presents the host-side conveniences as unshipped. This violates the README's own laws: the header ("the documents themselves describe **only the current reality**") and rule 5 ("When implementation contradicts a doc, the doc is amended in the same change").

### Evidence

All citations verified directly against the files:

- `docs/architecture/README.md:133-134` — "**`70-api.md` open sub-items**: see that doc's own OPEN list (result ordering, multi-key typed `get` sugar, multi-process future)."
- `docs/architecture/README.md:76-78` — "**Ordering/limit conveniences and top-k pushdown**: presentation-layer; results are sets, the host sorts. *Trigger: owner pain, or a measured materialize-then-sort latency-budget violation.*"
- `docs/architecture/README.md:4-5` and `:28-29` — the current-reality law and rule 5 the drift violates.
- `docs/architecture/70-api.md:1049` — "**The ledger is CLOSED (2026-07-17)**"; every row "carries its final state".
- `docs/architecture/70-api.md:1089-1090` — multi-key typed get: "**SHIPPED (this wave, 2026-07-19).**"
- `docs/architecture/70-api.md:1111` — answer sorting: "**SHIPPED (2026-07-19, the surface-pair wave)**" (limit REFUSED as surface, `:1121`).
- `docs/architecture/70-api.md:1152-1158` — multi-process: "**CLOSED, trigger intact and unfired (census 2026-07-17)**" — even the one still-live row is not on an "OPEN list"; it is a closed row with a standing trigger.
- Code confirming the ships: `ts/src/order.ts:156` (`export { by, desc }`); `crates/bumbledb-query/src/order.rs:26` (`pub enum SortKey`), `:57` (`pub fn value_cmp`), `:81` (`pub fn by`).
- `docs/feature-register.md` ("FIRED and scheduled" section, ~lines 146-174) — records both triggers as FIRED and both rows shipped 2026-07-19, cross-citing the same pins (`crates/bumbledb/tests/keyed_get.rs`, `ts/test/keyed-get.test.ts`, cookbook recipe 30).

### Failure scenario

An agent (or the owner) triaging OPEN items from the architecture index re-litigates or re-implements result ordering or keyed-get sugar — both already shipped and pinned by tests — or reports the 70-api ledger as open when the owning doc froze and closed it. Under the repo's own rule 4 ("An OPEN item is a real state; the failure mode is code deciding it silently"), a wrong OPEN roster is exactly the index's one job failing.

### Suggested fix

Two edits to `docs/architecture/README.md`:

1. Lines 133-134: delete the "open sub-items / OPEN list" phrasing entirely (70-api.md has no OPEN list — it has a closed ledger). If the multi-process future item deserves index visibility, list it directly as its own entry with its recorded trigger ("a second process with a legitimate claim on one store"), citing the 70-api ledger row.
2. Lines 76-78: reword the ordering OPEN item to its surviving residue only — engine-side top-k pushdown under a measured materialize-then-sort latency-budget violation — noting that the host-side conveniences shipped 2026-07-19 (`ts/src/order.ts`, `bumbledb_query::order`) and limit was refused as surface.
