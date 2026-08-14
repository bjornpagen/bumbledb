# engine-024: `run_ray_probes` and `run_interior_ray_probes` copy the latch protocol

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(1af537e5)
- **Source:** audit/engine.md F24
- **Depends on:** engine-001 (probes become per-stage data), engine-013 (image fill unified)

## The bug

`execute.rs:189-256` (`run_ray_probes`, main) and `reach.rs:212-280` (`run_interior_ray_probes`) are the same protocol line for line: resolve interns, fast-eligible check, `resolve_filters` with the same `ResolutionState` latch, `bindings.resize`, `RayArbiter::new`, `run_join`, `measure_of_ray` raise. Diffs: the interior copy calls `fill_plan_images` first, takes `&self.derived.occ_images` instead of `&[]`, and does a `std::mem::take`/restore dance on the probe sets to satisfy borrows.

## Why it's wrong

Two copies of a nontrivial protocol (latch discipline + error-path restore) exist because interiors are a sidecar (engine-001) with their own `ray_probes` field the main loop can't reach generically (Insight 2: the duplicated copy is where the next latch fix gets applied to only one). Rec's lack of probes is correct (`MeasureInRec` refuses measure through the cycle) but is currently a *missing field* rather than a fact of the rec-arm type.

## The fix

- ONE `fn run_ray_probes(probes: &mut [RayProbeSet], images: &[...], ctx, ...) -> Result<()>` free function (or method on a stage struct), called by main with the main images and by each interior stage with its filled images. The take/restore dance dissolves when the pipeline arm (engine-001) lets the caller borrow the stage's probes and the shared scratch disjointly.
- Each prepared stage that can carry measure conditions owns its `ray_probes` (main, `PreparedInterior`) — already true; the RUNNER unifies. `RecArm` has no probes field (engine-002's type) — the absence is structural, no comment apology needed.
- Latch counting (`unresolved_literals` decrement) happens once at the shared runner's exit, same arithmetic.

## Acceptance criteria

- [ ] One copy: `rg -n 'fn run_interior_ray_probes' crates/bumbledb/src` → no matches; the probe protocol body exists once (`rg -c 'RayArbiter::new' crates/bumbledb/src/api/prepared` → 1).
- [ ] Unchanged tests: all measure/ray tests green UNCHANGED (Ray raises identically from main and interior probes; `MeasureOfRay { start, end }` payloads identical).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- R6 probe semantics locked (probes run after the stage's rule loop; rays raise, never emit). Lands after engine-001/engine-013.
