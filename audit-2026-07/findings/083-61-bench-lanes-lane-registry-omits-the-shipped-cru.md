## 61-bench-lanes lane registry omits the shipped crud and lawful lanes it exists to register

category: incoherence | severity: medium | verdict: CONFIRMED | finder: r2:docs-vs-code-drift
outcome: fixed 5fb96177

### Summary

`docs/architecture/61-bench-lanes.md` declares itself "the normative name registry for every Report-class benchmark lane" and rules "No lane is added without a row in the registry below" (line 13), naming `scripts/bench-night.sh`'s `lane_table()` as "the executable twin of this table" (line 61). But the two home-turf lanes `crud` and `lawful` — shipped subcommands, run in the pinned 2026-07-20 night, published in the root README, and fully specified in `60-validation.md` — have no row in the lane table, no artifact contract, and no chart-inventory rows. Their shipped artifacts also falsify the chapter's universal discriminant law. The registry is violated by its own shipped estate — the exact drift class the chapter says its named readers exist to prevent. The doc is not merely stale relative to the lanes: it names `crud` and `lawful` in its own prose while excluding them from its registry.

### Evidence (all verified directly)

- **The registry law:** `docs/architecture/61-bench-lanes.md:3-13` — "The normative name registry for every Report-class benchmark lane… No lane is added without a row in the registry below."
- **The lane table has no crud/lawful rows:** `docs/architecture/61-bench-lanes.md:67-78` — rows are `bench-durable-r1..r3`, `bench-ephemeral-r1..r3`, `scenarios`, `sweep-commit`, `storage`, `curves`, `cold-warm-memo`, `write-throughput`, `adversarial`, `churn`. No `crud`, no `lawful`.
- **The executable twin has them:** `scripts/bench-night.sh:101` — `PROBED=" storage curves writes crud lawful adversarial churn "`; `scripts/bench-night.sh:118-119` — `crud|$OUT/crud/crud.json|"$BIN" crud --out "$OUT/crud"` and `lawful|$OUT/lawful/lawful.json|"$BIN" lawful --out "$OUT/lawful"`.
- **The viz ingests and charts them:** `scripts/bench_viz.py:337-338` (`load_crud_report` / `load_lawful_report`), `:382-383` (`NIGHT_LANE_REPORTS` entries `("crud/crud.json", "crud_report", load_crud_report)` and `("lawful/lawful.json", "lawful_report", load_lawful_report)`), `:1747-1753` (`ChartSpec("world-crud.svg", ("crud_report",), …)` and `ChartSpec("world-lawful.svg", ("lawful_report",), …)`).
- **The discriminant law is falsified:** `docs/architecture/61-bench-lanes.md:117-118` — "Every expansion-lane artifact is a `report.json` whose top level carries `"lane": "<id>"`". Direct read of `bench-out/night-2026-07-20/crud/crud.json`: top-level keys `['world','seed','provenance','lanes','poststate']`, no `"lane"` key, `"world": "crud"`; `lawful/lawful.json` likewise (`+ 'enforcement'`), `"world": "lawful"`. The doc's escape hatch — the canonical-path flag-fed enumeration at lines 132-137 — lists only `storage/`, `writes/`, `curves/`, `churn/`; the code's `NIGHT_LANE_REPORTS` carries six entries including the two crud/lawful paths the doc omits.
- **The chart inventory misattributes and omits:** `docs/architecture/61-bench-lanes.md:252` — the `world-<world>.svg` row claims source "`scenarios.json` preferred, `scenarios.md` fallback", but `world-crud.svg` / `world-lawful.svg` are sourced from `crud_report` / `lawful_report` via `home_turf_render` (`bench_viz.py:1487, 1747-1753`), and there are no dedicated rows for them.
- **The lanes are normative and published:** `docs/architecture/60-validation.md:670` — "## The home-turf worlds: crud and lawful" (the full spec: families, gates, parity configs, poststate verification); `README.md:200-236` — the published home-turf section embedding `assets/world-crud.svg` and `assets/world-lawful.svg` with numbers (crud geomean 0.51×, lawful 0.32× per README:590).
- **The doc knows the lanes exist:** `61-bench-lanes.md:35-37` (the Decision paragraph names "the ones we lose (`crud`, `lawful`, …)") and `:57` ("writes and crud verify post-state by full-scan body-multiset comparison") — prose references with no registry row.

### Failure scenario

An agent auditing or extending the lane estate against the chapter's own law sees a registry in which crud and lawful do not exist: it either flags the bench-night.sh rows as unregistered rogue lanes or writes a new lane without understanding the world-keyed artifact family. A tool written to the documented discriminant contract (top-level `"lane"` on every expansion-lane report) rejects or misclassifies `crud.json` and `lawful.json` — the two artifacts the pinned night actually produced and the README publishes. This is a doc-vs-code drift in the one chapter whose stated purpose is preventing exactly this drift.

### Suggested fix

Add `crud` and `lawful` rows to the lane table (world = the home-turf worlds, subcommand `crud --out` / `lawful --out`, artifacts `crud.json` / `lawful.json`, class Report, parity citing `60-validation.md` § the home-turf worlds). Record their `"world"`-keyed artifact contract beside the discriminant law and add their canonical paths to the flag-fed enumeration (lines 132-137) so the doc's ingestion list matches `NIGHT_LANE_REPORTS`. Add `world-crud.svg` and `world-lawful.svg` rows to the chart inventory sourced from `crud_report` / `lawful_report`, distinct from the scenarios-sourced `world-<world>.svg` row.
