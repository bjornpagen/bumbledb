## Normative docs cite bench-out measurement artifacts deleted by the 2026-07-20 pin swap

category: incoherence | severity: medium | verdict: CONFIRMED | finder: r2:docs-vs-code-drift
outcome: fixed 4de40efd (R21; re-trued whole against the wall-power estate at campaign close after the battery-era retirement f474202a)

### Summary

The 2026-07-20 pin swap (commit `6d5560a8`, "bench-out: the pin swap — the 2026-07-20 shared-machine night replaces every superseded report pin") executed the owner ruling "delete all the old outdated benchmark reports": `bench-out/` now contains only `night-2026-07-20/`. But the architecture docs' load-bearing performance claims still cite the deleted pins as the standing, *committed* evidence. Most sharply, success criterion 2 in `00-product.md` says the ALL-WIN claim is "earned at scale S by the committed `bench-out/` artifacts (engine rev `adac4010`, 2026-07-16...)" — those artifacts are not committed; they exist only in git history. This violates the docs' own charter (rule 5: docs amended in the same change as the code/artifact change, present tense; rule 6: no citing retired material), and nothing enforces it: `scripts/spec-census.sh` checks only `lean/` citations, not artifact paths.

### Evidence (all verified at HEAD)

Tree state and deletion:
- `ls bench-out/` → only `night-2026-07-20/`.
- `git log --diff-filter=D -- bench-out/measure-ephemeral-r6` → `6d5560a8`; its message enumerates the kills: run2/run3 (durable 2026-07-16, the `adac4010`-era pins), eph1..3, eph-nosync-1..3, measure-twins, measure-ephemeral-r6, scen.

Dangling citations in the architecture docs:
- `docs/architecture/00-product.md:396-397` — "earned at scale S by the **committed** `bench-out/` artifacts (engine rev `adac4010`, 2026-07-16: verify-stamped, ALL-WIN every gated family)". The word "committed" is now false.
- `docs/architecture/00-product.md:168` — ephemeral bands (27–52x / 43–70x / 1.1–1.6x) pinned to `bench-out/measure-ephemeral-r6/`.
- `docs/architecture/50-storage.md:130` (`bench-out/measure-twins/`), `:478` and `:514` (`bench-out/measure-ephemeral-r6/`).
- `docs/architecture/70-api.md:370` (`bench-out/measure-ephemeral-r6/`).
- `docs/architecture/README.md:61-65` (OPEN item: "the committed `bench-out/` artifacts, engine rev `adac4010`" plus `bench-out/eph-nosync-*` and `bench-out/measure-ephemeral-r6/`).
- Additional site the finder undercounted: `docs/architecture/40-execution.md:782` (`bench-out/measure-twins/`).
- Code-comment echoes (lower stakes, same dangle): `crates/bumbledb/src/storage/keys.rs:458`, `crates/bumbledb/src/exec/run/leaf.rs:16`, `crates/bumbledb/src/api/prepared/finalize.rs:29`, `crates/bumbledb/tests/ramdisk_phase_r.rs:597`.

Charter and enforcement gap:
- `docs/architecture/README.md` rules 5–7: rule 5 requires the doc amended in the same change ("Docs describe the system in the present tense"); rule 6 forbids citing retired material (allowing a measured *number* as rationale, but these sites cite artifact *paths* as evidence); rule 7's census (`scripts/spec-census.sh`) enforces only `lean/` citation integrity — grep confirms no `bench-out` check — so the dangle passes CI silently.

Replacement evidence exists and is stronger than the docs admit:
- `bench-out/night-2026-07-20/bench-durable-r1/report.json` and `bench-durable-r3/report.json`: `provenance.git_rev = ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49`, `all_win: true` (r2 stays committed as the honest CONTAMINATED record; merged numbers are min-over-clean per the pin-swap commit).
- Caveat for the repin: `bench-out/night-2026-07-20/MANIFEST.txt` records `rev: 4b031a15` (the night-harness rev at manifest time); the reports' provenance rev `ec0b9c75` is the one that describes the measured binary — cite the provenance rev.

### Failure scenario

A reader auditing success criterion 2 (the product's headline "beats SQLite, every family must win" claim) or the ephemeral-kind pricing bands follows the cited paths — `bench-out/measure-ephemeral-r6/`, `bench-out/measure-twins/`, the "committed" adac4010 artifacts — and finds nothing in the tree. Under the repo's own citation discipline (rule 6, and the spec-census principle that citations must resolve), a measured claim whose evidence path dangles is indistinguishable from a fabricated number. The irony is that the true current evidence (night-2026-07-20, all_win: true on every clean run) is better than what the docs cite.

### Suggested fix

Repin in one change, per rule 5:
1. `00-product.md:396` and `README.md:61-65`: re-earn success criterion 2 against `bench-out/night-2026-07-20/` (report provenance rev `ec0b9c75`, min-over-clean across r1/r3 durable + all three ephemeral, r2 excluded-and-counted as CONTAMINATED).
2. The measure-phase citations (`measure-ephemeral-r6/`, `measure-twins/`, `eph-nosync-*` in 00-product.md:168, 50-storage.md:130/478/514, 70-api.md:370, 40-execution.md:782): either restore those pins, or convert the citations to git-history records (name commit `6d5560a8` or the pre-swap tree hash) so the path is explicitly historical rather than dangling.
3. Optionally extend `scripts/spec-census.sh` to verify that any `bench-out/...` path cited in `docs/architecture/*.md` resolves in the tree — the same citation-integrity mechanism rule 7 already runs for `lean/`.
