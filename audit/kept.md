# Kept — standing do-not-fix rulings

Shapes audited and deliberately left alone. Each row is a ruling with its
reason; do not re-file these without new evidence. Essential complexity is
not a defect (REQUIRED-READING, Insight 16).

| Shape | Ruling |
| --- | --- |
| `OwnedInstance` vs `ReadInstance` vs `WriteTx` vs `InstanceBuilder` | Essential durations. One public `Admitting` type is refused by the proposal. |
| `staleness` on `&ReadInstance` only | The heap source never changes; fabricating drift for it is a dummy clock. (Distinct from `profile` — see the contested half of [09](09-profile-stats.md).) |
| Lean `structure Snapshot` | The mathematical consistent-state premise, not the deleted Rust API. Prose citations of the old API are [17](17-docs-vocabulary.md)'s to fix. |
| `ArenaSlice` without an arena tag | One arena per delta today; tag it the day a second arena exists. |
| `Db.generation: AtomicU64` | A cache of the persisted `GenerationId` for the parked-reader compare — not a second clock; it can never diverge under the one exclusive writer. Horizon: if it ever diverges, it dies. |
| Format-7 anything | Refused on every open surface; no decoder exists to keep. |
| TS `FreshRangeWire { empty: bool, … }` beside the C tagged union | JS has no C tagged union; `{ empty: true }` is the JS parse of the same sum. Do not force `{0,0}` into TS or a C bool into the ABI. |
| `dict` sentinel resolution of a miss | Recorded per-operator miss semantics; the id now has a typed home (`InternId`). |
| `read/scan.rs` inclusive upper bound | The prefix-vs-range cursor divergence is deliberately observable on corrupt keys (recorded). |
| `WordSet` zero-sentinel-plus-flag | Memory-justified and recorded; `NonZeroU64` slots would double the occupancy cost. |
| `Interval::MAX_END` as the unbounded ray | "∞ is a value of the representation" — recorded and priced (four guards, accepted). |
| `WriteDelta<'s>` schema borrow | Keep **accepted** this pass — see [05](05-writedelta-lifetime.md) for the cost argument and the reopening horizon. |
| C abort riding the one `Result<()>` write channel | Keep accepted; the *spelling* of the rider is narrowed in [13](13-c-exit-threading.md). |
| Store `scan` lending through the owned txn | Keep accepted for the scan half; the rest of [06](06-instance-one-body.md) stays open. |
