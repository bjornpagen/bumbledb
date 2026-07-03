# PRD 18 — The report: gates, budgets, artifacts

Authority: `00-product.md` success criteria 2–3 (every family must win; medians;
the manually re-run report; p99 ≤ 10 ms warm), `50-validation.md` (versioned
query list in-repo).

## Purpose

One run → one self-contained, versionable artifact: the comparison tables, the
gate verdicts, the budget checks, allocation and execution statistics, flame
summaries, and full provenance — the thing a human reads before making (or
refusing) the claim.

## Technical direction

- `report::RunReport` (plain data): provenance {crate version, engine git rev via
  `option_env!("BUMBLEDB_GIT_REV")` with a build-script-free fallback of
  "unknown" — the CLI passes `-e` env at invocation instead: read
  `std::process::Command("git", ["rev-parse","HEAD"])` at *runtime* from the
  repo dir, "unknown" outside a repo; document the choice}, timestamp (ISO-8601
  from `SystemTime`, hand-formatted), host (`sysctl -n machdep.cpu.brand_string`
  via Command, best-effort), config {scale, seed, samples}, corpus digest,
  verify stamp; per read family: bumbledb `Stats`, sqlite `Stats`,
  `ratio_p50: f64`, verdict `Win | Loss | ReportOnly`, `alloc:
  Option<AllocSnapshot>`, `exec: ExecDigest` {worst node est-vs-actual factor,
  cover histogram condensed, emits}, budget `p99_within_10ms: bool`; per write
  family: both `Stats` (or ours-only for cold), facts/sec where applicable;
  store numbers {db file size, sqlite file size, cache resident images/bytes};
  flame summaries when traced.
- Renderers, both hand-rolled:
  - `report::to_markdown(&RunReport) -> String` — sections: Provenance, Gate
    verdict (ALL-WIN or the failing families, plus the p99 budget line), the
    read-family table (`family | ours p50/p95/p99 | sqlite p50/p95/p99 | ratio |
    verdict`), write table, allocation table (families × alloc/dealloc counts &
    bytes — read families must show 0/0), execution digest table, store sizes,
    flame top-10 per traced family.
  - `report::to_json(&RunReport) -> String` — hand-rolled emitter (the trace
    writer's string/number helpers extracted into a tiny shared `json` module in
    this PRD): every field, machine-consumable for future diffing.
- `report::write_artifacts(&RunReport, out_dir)`: `report.md`, `report.json`,
  and — the versioned-query-list requirement — `QUERIES.md` from PRD 14's
  renderer. The human copies artifacts into the repo when publishing a run
  (the tool never writes into `docs/` itself).
- Gate logic pinned here: `Win` ⇔ ours p50 strictly < theirs p50; overall
  ALL-WIN ⇔ every `Kind::Gate` family wins; budget ⇔ every gate family's ours
  p99 ≤ 10 ms at scale L (at S/M the line prints as informational). Exit-code
  mapping is PRD 19's.

## Non-goals

Historical diffing, charts, HTML. Writing anything outside `out_dir`.

## Passing criteria

- Unit tests: markdown golden for a synthetic two-family RunReport (all
  sections present, table alignment stable); JSON output round-trip-checked
  structurally (hand-rolled `json` module has its own escaping tests: quotes,
  backslash, control chars, non-ASCII memo); verdict logic table-tested (win /
  tie=loss / loss; budget pass/fail at boundary 10 ms exactly ⇒ pass on ≤);
  `write_artifacts` creates exactly the three files; QUERIES.md content equals
  PRD 14's renderer.
- `scripts/check.sh` green.
