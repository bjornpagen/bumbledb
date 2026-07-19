# PRD-U6 — Architecture-docs alignment

Wave 2 · Repo: bumbledb (`docs/architecture/`, `README.md`, `scripts/` prose,
`TODO.md`) · depends on: U1–U4 (documents the end state) · executes rulings
10 (allowlist) and 11 (test-scaffolding unsafe)

## Objective

`docs/architecture/` is the spec of record; after the wave it must describe
the tree that exists — present tense, no history outside explicit retraction
records. This PRD sweeps every doc sentence the wave falsified and lands the
two unsafe-policy rulings, which are doc edits.

## Work

1. **The U1 retraction sweep** (the census §1.2 doc inventory, re-verified
   against this tree — U1's commit message carries the final site list):
   - `50-storage.md`: the env-constants paragraph (one 32 GiB map, the
     4 GiB retraction recorded per the flip discipline); the capacity-contract
     spec paragraph retired; the WRITEMAP consequence paragraph rewritten to
     the NOSYNC-only story — 50-storage already records NOSYNC-only as the
     priced fallback; promote it from fallback to the law, with the flip
     recorded as a retraction, never a silent edit.
   - `00-product.md`: the map sentence and the ephemeral carve-out paragraph's
     flag story (the kind survives; the flags changed).
   - `60-validation.md`: the ramdisk sizing note; the kill-sweep description
     (WRITEMAP commit-window → NOSYNC commit-window).
   - `70-api.md`: the map-size "no tuning parameters" sentence; the probe
     note ("no WRITEMAP, so no full-map ftruncate" — now true of every open).
   - `README.md`: the per-store disk-requirements paragraph; the libc
     justification sentence (dies or moves with U1's Cargo.toml outcome).
   - `scripts/ramdisk.sh`: the 5 GiB default's capacity-contract recital —
     re-derive the default size honestly.
2. **The unsafe policy (rulings 10 + 11)** — one edit to
   `00-product.md`'s sanctioned-unsafe section:
   - Drop the stale `exec/run.rs` entry (zero unsafe there today).
   - Add the missing sanctioned modules: `storage/env/open_env.rs` **as it
     stands after U1** (if U1 removed its last unsafe, record that the
     sanction lapsed instead), `alloc_counter.rs` (feature-gated
     GlobalAlloc), `bumbledb-bench/src/clockproxy.rs`.
   - Name the test-scaffolding-unsafe category: inline-reasoned unsafe in
     test code (the six census-listed sites) is a recognized class with its
     own one-sentence law (each site carries its reason; no production
     reachability).
   - Record `ts/crate`'s new regime (U4) and `fuzz/`'s deliberate detachment
     so the policy's map of lint regimes is total.
3. **CI/ops prose**: `60-validation.md`'s operations section gains the ubuntu
   lane and the (now true) Miri cron sentence; any "macOS-only CI" or
   "cron guards it" fiction dies.
4. **The packet-survival check** (house convention: packets die shipped —
   nothing durable may live only in a packet):
   - The copy-on-append ruling record: confirm the durable copies in
     `50-storage.md` § image cache and `40-execution.md` carry the full
     ruling before any future deletion of `docs/prds/incremental-images/`
     (that packet lives on the PR #10 branch; flag, don't touch).
   - THIS packet: confirm rulings 1–14's durable content lives in
     architecture docs / manifests / gravestones, then the packet is eligible
     for deletion at wave close (the deletion itself is the serial
     committer's).
   - `docs/structural-1.0.0/` deletion stays bound to R2 (owner ceremony) —
     out of scope, unchanged.
5. **`TODO.md`**: current-tense update — the wave's rulings recorded, M's
   pending measurements listed, the PR #10 debts flagged (README re-true,
   waveM report.json, the five stale remote branches for the owner).

## Passing criteria

- `grep -rn "preallocat\|WRITEMAP\|ftruncat\|4 GiB" docs/ README.md scripts/`
  hits only retraction records, false friends (the u32 byte-heap 4 GiB), and
  history explicitly marked as such. (This tree never adopted the census's
  "capacity contract" name — the sweep targets are the ftruncate/WRITEMAP/
  preallocation sentences at `50-storage.md:442-450`, `60-validation.md:633`,
  `70-api.md:356`, `README.md:375,438-441`, `scripts/ramdisk.sh:35-47`.)
- The unsafe allowlist matches `grep -rln "unsafe_code" crates/` reality
  exactly (the lint name appears ONLY inside `#[expect(unsafe_code, …)]`
  attributes, so the bare token enumerates the sanctioned files; the
  paren-suffixed spellings miss the multi-line `#[expect(` form) — every
  sanctioned module listed, every listed module sanctioned, the
  test-scaffolding category named.
- `scripts/spec-census.sh` green; `scripts/lean.sh` green (doc citations
  unbroken).
- Present tense throughout; every flip is a recorded retraction.
- The packet-survival checklist written into the PR body with per-ruling
  "durable home: <cite>" lines — the packet may not die before this exists.
