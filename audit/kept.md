# Kept — standing do-not-fix rulings

Shapes audited and deliberately left alone. Each row is a ruling with its
reason; do not re-file these without new evidence. Essential complexity is
not a defect (REQUIRED-READING, Insight 16). Self-contained: the issue files
these rulings adjudicated are closed and live in git history.

| Shape | Ruling |
| --- | --- |
| `OwnedInstance` vs `ReadInstance` vs `WriteTx` vs `InstanceBuilder` | Essential durations. One public `Admitting` type is refused. |
| `staleness` on `&ReadInstance` only | The heap source never changes; fabricating drift for it is a dummy clock. |
| Lean `structure Snapshot` | The mathematical consistent-state premise, not the deleted Rust API. |
| `ArenaSlice` without an arena tag | One arena per delta today; tag it the day a second arena exists. |
| `Db.generation: AtomicU64` | A cache of the persisted `GenerationId` for the parked-reader compare — it cannot diverge under the one exclusive writer. Horizon: if it ever diverges, it dies. |
| Format-7 anything | Refused on every open surface; no decoder exists to keep. |
| TS `FreshRangeWire { empty: bool, … }` beside the C tagged union | JS has no C tagged union; `{ empty: true }` is the JS parse of the same sum. |
| `dict` sentinel resolution of a miss | Recorded per-operator miss semantics; the id has a typed home (`InternId`). |
| `read/scan.rs` inclusive upper bound | The prefix-vs-range cursor divergence is deliberately observable on corrupt keys (recorded). |
| `WordSet` zero-sentinel-plus-flag | Memory-justified and recorded. |
| `Interval::MAX_END` as the unbounded ray | "∞ is a value of the representation" — recorded and priced. |
| `WriteDelta<'s>` schema borrow | Keep accepted (cost ruling: hundreds of construction sites; no self-reference is representable now that `MutationCore` owns the delta). Horizon: reopens if a schema-owning struct ever embeds the delta. |
| Store `scan` lending through the owned txn | Keep accepted: a lending cursor cannot borrow a temporary catalog; two scan bodies are the coordinate change. Answer on record if re-attempted: a catalog member built at lease birth. |
| C abort riding the one `Result<()>` write channel | Keep accepted: threading a second exit type through the engine write body would be a second write algebra. The rider's *spelling* (`Hatch(Arc<dyn Any>)`) is reopened by the zero-dyn law of the final pass — the channel ruling stands, the payload changes. |
| `NodeScratch` / kind-grouped `PlanNode` batching | Recorded refusals stand (the grouping is the batching law); only the residual-source *copies* are final-pass work. |
