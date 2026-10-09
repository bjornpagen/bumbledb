# engine-query lane board

Owns: `crates/bumbledb/src/{ir*, plan*, exec.rs, exec/** except kernel and sink/aggregate, image*,
api/prepared.rs, api/prepared/** except computed}`, `tests/{adversarial_ir, point_reads,
reach_finalize_hunt}.rs`.

Items: tests cull, A (plan validator), C3, C4, C5 (query side), C11, C12, C13, ValidationError,
E5 (lowering/fold/residual), E8, E3 (consume numeric's check), C7 adapt, L.

## Status

| Item | Status |
|---|---|
| Tests: delete timing pins and host-pinned falsifiers | landed `5f6f95be6` |
| E3: guard block and `numeric_outputs` deleted | landed `7067683fb` |
| C7: `plan/ground.rs` under `feature = "testing"` | landed `23e4ef997` |
| ValidationError step 1 (exact copy at `crate::ir::validate::error::ValidationError`) | landed `72b084c8c`; waiting on engine-storage step 2 |
| C4 step 1: `AggregateSink::spill` is `#[allow(dead_code)]` | landed `b6c7d95a5` |
| C3: delete cursor fallback | in progress |
| A: plan validator `.expect` → `Err` | todo (lands with the ValidationError fold) |
| C4: delete scratch tier and spills | todo (staged with numeric, see requests) |
| C5 query side | waiting on engine-storage C5 |
| E5, E8 | todo |
| C11, C12, C13 | todo |
| adversarial_ir sharding | todo |

## Requests to other lanes

### engine-storage

1. ~~`Error::Capacity`~~ landed, thanks.
2. **`exec::scratch` users.** C4 deletes `exec/scratch.rs` and `exec/scratch/`. Outside my paths it
   is used by `work.rs` (`pub use crate::exec::scratch::{...}`), `lib.rs` (`pub use work::{Scratch*}`),
   `storage/store/verify.rs:148` and `schema/judge/grouped.rs`. Please drop those uses; I delete
   the module once HEAD no longer names it.
3. **`ValidationError` step 2 (ready now).** Delete the definition in `error.rs` and the
   `impl Display for ValidationError` in `error/display.rs`, then
   `pub use crate::ir::validate::error::ValidationError;` in `error.rs`. My copy is variant-for-variant
   identical and carries its own `Display`, so every existing construction site compiles. I then
   fold variants (step 3) and announce the final shape here.
4. ~~`testing` feature~~ switched in `plan/ground.rs`. `lib.rs` still re-exports
   `with_grounding_disabled` under `feature = "ground-off"`; please flip it to `testing`.
5. **C5.** Announce the canonical closed-row API; I switch `image/build.rs` `synthesize_closed`
   and `plan/ground/evaluate` and delete `image/decode.rs`.

### numeric

1. **C3 (please, small).** `api/prepared/tests/float_aggregates.rs:190-192` calls
   `force_cursor_fallback(true)`. The fallback is deleted; please drop that comparison (the resident
   run is the only path). Until then `PreparedQuery::force_cursor_fallback` stays as a hidden no-op.
2. **C4, my APIs your files call.** Please stop calling these; I delete them after:
   - `ProjectionSink::force_spill` (`api/prepared/computed/tests.rs:77`): drop the `spill` axis.
   - `SpillSet` is renamed `SeenSet` (no spill tier). I add `SeenSet` plus a one-commit alias
     `SpillSet = SeenSet`; please switch `aggregate/new.rs` to `SeenSet`, then I delete the alias.
3. **C4, your APIs my files call.** `api/prepared/reach.rs` calls `AggregateSink::stream_finalize`
   (scratch destination) and `resident_row_bound`. Proposal: add `#[allow(dead_code)]` to
   `stream_finalize`; I switch the stage seal to `finalize_into` only, refusing with
   `Error::Capacity(Capacity::ResidentRows)` when `group_count()` exceeds `u32`; then you delete
   `stream_finalize` with the rest of the group spill. `resident_row_bound` is no longer needed by me
   after that.
4. **Spill field.** Agreed: deleting `AggregateSink::spill` (my `sink.rs`) and `spill: None` (your
   `aggregate/new.rs`) is one atomic edit; the consolidator does it unless one of us gets both files.
5. **E5 MIN/MAX lowering.** Will do when you announce `AggSpec::Float { op: Min | Max }`.

## API changes (announcements)

- `crate::ir::validate::error::ValidationError` exists (exact copy of `crate::error::ValidationError`).
- `PreparedQuery` no longer enters `NumericalGuard`; `numeric_outputs` is gone.
- `plan::ground::with_grounding_disabled` is compiled under `any(test, feature = "testing")`.
