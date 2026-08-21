# 80 — Acceptance: both upstream lists, the ten laws, the digests, the order

Nothing in this set is done until the conditions below hold with evidence
attached — a primerlane report (10), a pinned test, or Primer's own
verifier run. Assertions without attribution do not close conditions
(house law).

## The baseline being beaten (the accepted upstream run)

| Quantity | Baseline |
| --- | ---: |
| Bumbledb persistence (3,993,828 facts, 39 relations) | 27.61 s |
| Primer build total | 57.72 s |
| Full verifier total | 58.02 s |
| Peak resident set | 7.22 GiB |
| Count readback (39 full-binding aggregate queries, 1.68 GB store) | ~250 ms |
| Profile owners | 12,235 `commit` · 6,142 `record` · 3,107 GC · 2,810 `readScan` samples |

## Upstream report 1 — collection transport and exact count

| # | Condition (verbatim) | Owner | Evidence |
| --- | --- | --- | --- |
| 1 | The public typed collection-write algebra remains coherent | 20 | API-surface tests: `insert`/`load` take `Iterable<Fact<R>>`, one spelling, typed facts in / one report out |
| 2 | Existing write, poison, admission, and snapshot laws remain unchanged | 20, 30, 40 | the ten-law table below; the untouched `apply_collection`/commit pipeline; the full existing suites green |
| 3 | Primer needs no Bumbledb-specific packing code | 20, 70 (D10) | Primer's runtime.ts carries no transpose, no batch size, no column assembly |
| 4 | The 3,993,828-fact persistence phase falls materially below 27.61 s | 10, 20, 30 | primerlane before/after table + Primer's `verify:learning-commons` timing |
| 5 | Exact relation counts do not materialize fact rows | 40 | count lane: zero decode, zero per-call engine allocation; `readScan`/`factOf` absent from the readback profile |
| 6 | Peak live memory falls materially below 7.22 GiB | 20, 30 | alloc-census peak-live on the primerlane; Primer max-RSS |
| 7 | The full verifier improves from 58.02 s | all | Primer's verifier wall clock |
| 8 | All three canonical digests remain unchanged | all | **stop-ship invariant, below** |

Also from report 1's body, closed by 50: the SDK admits the output of
generic `v(relation)` as the full binding of that same generic relation —
no cast, no per-relation query declarations, no suppression (pins in 50;
D12).

## Upstream report 2 — containment target-key parity (chosen branch: target-must-be-key)

| # | Condition (verbatim) | Owner | Evidence |
| --- | --- | --- | --- |
| 1 | The value-level `schema()` check must reject the statement | 60 §1 | the parity suite's refused rows, TS side |
| 2 | The type-level law check should reject it when the tuple is statically known | 60 §2 | `TargetKeyWall` compile-error pins — or the recorded G3 refusal with numbers |
| 3 | `lower()` must never emit it as an engine-admissible schema | 60 §3 | totality inherited from `schema()`'s refusal; parity suite constructs `lower()` inputs only through `schema()` |
| 4 | The diagnostic must name the target relation, target projection, and available keys | 60 §1, §4 | diagnostic golden pins at both tiers — names, canonical rendering, available-keys list, pointwise hint |

Plus the report's refusals, standing as gravestones (70): no synthesized
key, no forced FD, no silent admission of general INDs.

## The ten preserved laws (report 1, verbatim, with owners)

| Law | Owner | Where proved |
| --- | --- | --- |
| 1. complete collection passes shape parsing before the first row enters the delta | 20 | the constructor IS the parse; unconstructible ⇒ unenterable |
| 2. empty collections remain lawful | 20 | `rows: 0` pin |
| 3. exact `submitted`/`changed` | 20 | mutation-report pins over the new transport |
| 4. failure after an applied prefix poisons exactly as now | 20 | `apply_collection` untouched; existing poison suites |
| 5. closed-relation refusal remains typed | 20 | same judgment, same order, existing pins |
| 6. interning preserves exact string equality | 30 | memo-is-a-read-cache argument + identical-ids pin |
| 7. fresh marks advance under the existing rules | 20 | `advance_fresh_marks` untouched |
| 8. commit admission and violations unchanged | 20, 30 | commit pipeline out of scope; suites green |
| 9. read scopes continue to own snapshot lifetime | 40 | `count` is a lease method, `assertLive`-guarded |
| 10. exact count observes the same snapshot as `scan` | 40 | same-lease equality pin + held-lease generation pin |

## The stop-ship invariant

The three Primer canonical digests are byte-identical before and after
every landing in this set:

- Source IR `27202ace4da1317a592f523c80431c38670d9ec04796b80f0eac2eae6ff0b3d1`
- Standards Evidence IR `efa086b986b1bb7839b45c1407fabc649e2d400e8b3aaf61197fc987e4dc1706`
- normalization ledger `cc1b3ee64ecb01c69acbb4633f4ea961c5a5420da17d1e04568661dd5d6f49d7`

Nothing in this set touches fact semantics, so any drift is a defect in
the change, categorically — investigate, never rebless. Same rule for the
parity suite: a case where `schema()` and `Db.create()` disagree after 60
lands is a stop-ship, whichever direction it disagrees in.

## Rollout: three waves, representation first

The dispatch elaboration lives in [90-rollout.md](90-rollout.md); this is
the binding order. The representation is not the last, carefully-earned
step — it is the centerpiece, built from hour zero, with measurement as a
parallel refutation rig (G1) rather than a serializing bake-off. A lane's
deletions (70) land inside the lane, in the same commits as their
replacements — never behind them, never in a deprecation window.

- **Wave 1 (all parallel, immediately):**
  the accepted collection at R2 in the engine (20, Lane 1); the
  attribution rig + R0 baseline + D3/D4 hygiene (10, Lane 0); the
  committed-string memo (30, Lane 2); exact count (40, Lane 4); the
  binding law (50, Lane 5); containment parity (60, Lane 6). Lanes 4/5/6
  are independent and merge as they finish.
- **Wave 2 (after Lanes 0 and 1):** the crossing (Lane 3) — the bridge
  and TS write path rebuilt on the collection, carrying D1/D2/D6/D8 in
  the same commits. Merge is blocked by **G1 confirmed** and **G2
  confirmed** on the primerlane; a refuted gate re-litigates the pin with
  numbers, it does not spawn a fallback mode.
- **Wave 3:** the 0.16.0 release lane (Lane 7) — lockstep version bumps,
  the `PUBLISHING.md` 0.16.0 entry, acceptance tables filled with lane
  receipts — and the Primer adoption PR (D10/D11/D12 in one sweep,
  `verify:learning-commons` run, before/after table attached, both
  upstream reports closed with numbers). Publishing itself is owner
  ceremony (`ts/PUBLISHING.md`) and is out of fleet scope.

## 0.16.0 — the one-representation release

One release carries the whole set (the D1/D2 break ships in the same
release as the transport that replaces it). Lockstep bump — one spelling,
five manifests: `crates/bumbledb`, `crates/bumbledb-c`, `ts/crate`,
`ts/package.json`, `ts/npm/darwin-arm64` — `0.15.0 → 0.16.0`
(`ts/scripts/build.ts` owns the lockstep gate; npm main is the source of
truth). Storage stays **format 8** — `count` reads a stat every format-8
store already maintains, so existing stores open unchanged and there is no
migration. The C ABI stays **3** — no C surface moves in this release;
`bumbledb-c` rides the lockstep and is named a future adopter of the
collection lane. The one breaking change is TypeScript-only and is D1:
`ColumnBatch` and the column transport are gone; the `PUBLISHING.md`
entry for 0.16.0 names it and its replacement in the same sentence.

## Verification commands

```sh
cargo test -p bumbledb
```

```sh
cargo run -p bumbledb-bench -- primerlane --report
```

```sh
pnpm --dir ts run test
```

```sh
pnpm --dir ts run typecheck
```

```sh
pnpm run verify:learning-commons
```

(the last from `primer-spec`, after the adoption PR; expected: acceptance
conditions 4–8 of report 1, digests identical.)
