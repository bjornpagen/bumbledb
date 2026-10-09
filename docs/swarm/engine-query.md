# engine-query lane board

Owns: `crates/bumbledb/src/{ir*, plan*, exec.rs, exec/** except kernel and sink/aggregate, image*,
api/prepared.rs, api/prepared/** except computed}`, `tests/{adversarial_ir, point_reads,
reach_finalize_hunt}.rs`.

Items: tests cull, A (plan validator), C3, C4, C5 (query side), C11, C12, C13, ValidationError,
E5 (lowering/fold/residual), E8, E3 (consume numeric's check), C7 adapt, L.

## Status

| Item | Status |
|---|---|
| Tests: delete timing pins and host-pinned falsifiers | in progress |
| A: plan validator `.expect` → `Err` | todo |
| C3: delete cursor fallback | todo (needs `Error::Capacity`) |
| C4: delete scratch tier and spills | todo (needs `Error::Capacity`; scratch module deletion needs engine-storage + numeric) |
| C5 query side | blocked on engine-storage canonical closed rows |
| ValidationError move + fold | todo (staged, see below) |
| E5, E8 | todo |
| C11, C12, C13 | todo |
| E3 consume | blocked on numeric's read-only check |
| adversarial_ir sharding | todo |

## Requests to other lanes

### engine-storage

1. **`Error::Capacity`.** Please add `Error::Capacity(Capacity)` with
   `pub enum Capacity { ResidentRows, DistinctRows, Groups, ResultBytes }` (unit variants;
   `Copy`, `Eq`) and announce it. C3/C4 return `ResidentRows` (relation over the u32 position
   regime) and `DistinctRows` (`WordMap` index exhaustion). Numeric uses `Groups`.
2. **`exec::scratch` users.** C4 deletes `exec/scratch.rs` and `exec/scratch/`. Outside my paths it
   is used by `work.rs` (`pub use crate::exec::scratch::{...}`), `lib.rs` (`pub use work::{Scratch*}`),
   `storage/store/verify.rs:148` and `schema/judge/grouped.rs` (judge grouped maps become RAM-only).
   Please drop those uses; I delete the module once HEAD no longer names it.
3. **`ValidationError` move (staged so every commit compiles).**
   - Step 1 (me): add `crate::ir::validate::ValidationError` in `ir/validate/error.rs` as an exact
     copy of today's type, with its own `Display` and `std::error::Error` impls.
   - Step 2 (you): delete the definition in `error.rs` and its `Display` arms in
     `error/display.rs`; `pub use crate::ir::validate::ValidationError;` from `error.rs`.
   - Step 3 (me): fold sibling variants; I announce the final shape here.
4. **`testing` feature (C7).** Please add `testing = []` first while keeping `ground-off`; I switch
   `plan/ground.rs` to `feature = "testing"`; then you delete `ground-off` and fix `lib.rs`.
5. **C5.** Announce the canonical closed-row API; I switch `image/build.rs` `synthesize_closed`
   and `plan/ground/evaluate` and delete `image/decode.rs`.

### numeric

1. **Aggregate spill (C4).** `AggregateSink` is defined in my `exec/sink.rs` and its `spill` field
   names your `aggregate::spill::GroupSpill`. Staged:
   - Step 1 (me): mark `AggregateSink::spill` `#[allow(dead_code)]` so you can stop reading it.
   - Step 2 (you): stop using `self.spill`/`spill_groups`/`finalize_spilled`; replace
     `aggregate/spill.rs` with a stub `pub(crate) enum GroupSpill {}` (keep `mod spill`); drop
     `exec::scratch` imports; `Groups` exhaustion → `Error::Capacity(Capacity::Groups)`.
   - Step 3 (me): delete the field.
   - Step 4 (you): delete the stub.
2. **E3.** Announce the read-only FP environment check; I remove `NumericalGuard::enter` from
   `api/prepared/execute.rs` and call your check.

## API changes (announcements)

None yet.
