# Interior representation debt

## Ruling

Small, cutover-independent representation cleanups across storage, schema,
IR, and the error/observability layers. Every item is crate-private or
behavior-preserving: no storage format change (all persisted bytes are
identical), no ABI change, no public API change. Nothing here gates
[`instance-lifetime.md`](instance-lifetime.md) or
[`exec-representation.md`](exec-representation.md); items land in any order,
one per change.

The shared shape of these findings: **the boundary between "read the store"
and "know the store" crossed with bare bits.** Probes return `bool`, checks
return `()`, parsers return raw bytes, writers return lengths — six
read-and-forget shapes, each forcing the caller to re-derive or re-assert
what was already proved.

## Storage

1. **`check_width` validates and discards** (`storage/read/check_width.rs:5`,
   five call sites). Each caller receives `()` and hands out a bare `&[u8]`;
   the decoder then re-asserts the width with a `debug_assert_eq!` — in
   release, a wrong-width slice is an index panic instead of the typed
   `WrongFactWidth` the check already knows how to mint. Becomes
   `check_width(..) -> Result<FactView<'a>>` bundling `{ bytes, layout }`;
   the field readers take `FactView` and the loose `(layout, bytes)` pairing
   at `determinant_image` closes.
2. **`parse_stat_key` returns the raw discriminant byte**
   (`storage/keys.rs:449`), so the sweeper re-declares `StatKind`'s
   discriminants as local consts and matches on `u8`. Becomes
   `StatKind::from_byte` and
   `parse_stat_key -> Option<(RelationId, StatEntry)>` with
   `StatEntry::{Known(StatKind), Unknown(u8)}` — the sweeper needs the
   unknown arm; nobody needs the bare byte.
3. **Namespace tags are six loose `pub const u8`** (`storage/keys.rs:207`).
   `KeyWriter::new` admits any byte; two parsers discard the tag entirely
   (`split_first`), so an `M` key parses cleanly as a fact key when lengths
   line up. Becomes `#[repr(u8)] enum Namespace` with `TryFrom<u8>`; one
   parse narrows the tag, each parser takes the narrowed value. `StatKind`
   twenty lines above is the in-file precedent.
4. **Fixed-width key writers return a length** (`keys.rs:270`, eight call
   sites). Every caller carries a `(buf, len)` pair for a compile-time-
   constant width plus a `debug_assert_eq!`, and the redundancy already
   shows drift (`&key[..len]` vs `&key`). `finish` returns the slice; the
   four fixed key families return arrays by value; eight asserts delete.
   The recorded no-oversized-zeroing rule is strengthened, not broken —
   only the variable-width `U`/`R`/prefix writers keep the buffer form.
5. **`WriteDelta::apply` returns one bit for three outcomes**
   (`storage/delta/insert.rs:44`): no-op, cancel (an entry *disappears*),
   record (an entry *appears*) — cancel and record both `true`, and the api
   layer merges a fourth provenance into `false`. Becomes
   `enum DeltaEffect { NoOp, Recorded, Cancelled }`; `MutationReport.changed`
   gains a named source. `CommitReport` and `ApplyRow`, one module away, are
   the precedent.
6. **`TupleOwners::cancel(&mut self, ..) -> bool`** with inverted polarity
   and a caller obligation to remove the emptied entry — the exact state the
   type's own doc calls unrepresentable. Becomes
   `fn cancel(self, ..) -> Option<Self>`; the map entry's removal is the
   type's word.
7. **The twin fresh maps** (`storage/env.rs:272-276`): `escaped_fresh` and
   `pending_fresh_flush` are the same key/value shape holding two states of
   one monotone lattice, with the join hand-written four times and the retry
   state encoded as map emptiness. The floors stay (their monotone law is
   recorded and Lean-pinned); the representation becomes a `FreshMarks`
   newtype owning `join`, with `enum FlushState { Clean, Parked(FreshMarks) }`.
8. **`writer_thread: AtomicU64` with `0 = none`** kept legal only because the
   key mint starts at 1. Becomes `ThreadKey(NonZeroU64)` stored as
   `AtomicU64` with a typed accessor.
9. **`FreshRange` has three overflow disciplines for one bound**
   (`api/db/mutation.rs:118-161`): `end_exclusive_raw` checks, `ids()` and
   `iter()` don't. The single construction site settles it once.
10. **`ArenaSlice` carries no arena provenance** (`arena.rs:12`) — safe today
    because exactly one arena exists per delta; worth a generation tag the
    day a second arena appears. Recorded here so the invariant is a ledger
    row, not folklore.

## Schema and IR

11. **Descriptor wire tags exist only as bare literals in two files**
    (`schema/fingerprint.rs` pushes `0,2,3,4,5,6,7`;
    `schema/descriptor_codec.rs` matches the same literals 200 lines away).
    One `#[repr(u8)]` discriminant (or `tag()`/`TryFrom<u8>` pair) per
    encoded sum, shared by both halves — **bytes unchanged**, so the
    descriptor identity and every fingerprint are untouched; only the two
    hand-synchronized spellings merge. The `Option<Bound>` ceiling's nested
    two-level tag becomes the four-arm sum the sealed side already has.
12. **`AxiomIndex(u16)` over a 256-element domain** (`schema.rs:320-359`):
    `contains` is total by silence, `insert` partial by panic, and the bound
    lives in a contract comment. Becomes `AxiomIndex(u8)` — both total, one
    narrowing layer deletes.
13. **`IntervalTail` is the fifth interval-shape vocabulary**
    (`schema.rs:193-230`), re-deriving widths the type system already knows.
    After the cutover's `ValueType`/`TypeDesc` merge it becomes the
    interval-restricted `ValueType`, and `bytes()`/`words()` collapse into
    the one width owner.
14. **`Side` has three canonical forms distinguished by nothing in the type**
    (raw / literal-sorted / fully normalized), with "THE statement identity"
    being the third and the sealed arenas holding the second. A
    `NormalizedSide` newtype whose only constructor sorts makes the
    identity form the only spellable one where identity is compared.
15. **`refuse_poisoned` runs before the empty short-circuit**
    (`api/db/apply.rs:51-55`), so `insert([])` on a poisoned transaction
    reports `TransactionPoisoned` although "empty is no engine request" —
    the recorded API-8 ordering, still inverted. Swap the two lines.

## Error and observability

16. **`Error` cannot derive `Clone`/`PartialEq`, and its hand-written 40-arm
    `Clone` is lossy** (`error/convert.rs:74-172`): two foreign payloads
    (`std::io::Error`, `heed::Error`) force an 88-line transcription that
    maps two distinct causes onto a third — `x.clone() != x`. The payloads
    become owned records (`IoFailure { kind, raw_os }`, a small
    `LmdbFailure` sum); the derives return; `convert.rs`'s transcription
    deletes.
17. **The same `Error` tag is matched exhaustively at four sites** (`Clone`,
    `source`, `Display`, the C kind map) — four coordinated edits per new
    variant, one carrying a clippy-acknowledged "table written as control
    flow". One per-variant descriptor table; four matches become one.
18. **Seven index spaces ride raw `usize`** across ~45 error fields
    (`index`, `find`, `atom`, `row`, `rule`, `position`, `element`) while
    six id spaces are newtyped; two variants take same-typed arguments in
    either order silently, and one coordinate system is defined only in a
    doc comment. `AtomIndex` / `FindIndex` / `RuleIndex` / `RowIndex`
    newtypes; the prose ordering law becomes the constructor.
19. **Nine spellings of "the witnessed value against its bound"**
    (`(expected, actual)`, `(found, expected)`, `(claimed, witness)`,
    `(stored, counted)`, …) with operand order flipping between variants.
    `Mismatch<T> { witnessed, required }` and `Exceeded<T> { observed,
    ceiling }`; the field-name lottery deletes.
20. **`obs` events are string-keyed with prose-defined payloads**
    (`obs.rs:50-72`): `name: &'static str` plus `(a0, a1)` whose meanings
    live per-name in doc comments, `0` as the in-band unused sentinel — and
    an aborted pass records `a0 = 0`, indistinguishable from a clean empty
    pass. A `TracePoint` enum per event (the file already pins one table
    with a `const` assert — the right instinct, applied to one of sixty);
    labels derive as `Category::label` already does.
21. **`decode_field` returns the 19-variant `CorruptionError` when it can
    produce four** (`verify_store/facts.rs:67-81`), so the caller proves the
    subset with an `unreachable!` and then discards the typed evidence bytes
    for a static label. A four-variant `FieldDecodeError` with
    `From<FieldDecodeError> for CorruptionError`; the `unreachable!` becomes
    exhaustiveness and the evidence survives. (The static label itself is
    justified by the recorded diagnosis-not-payload ruling; the discarded
    payload and the `unreachable!` are not.)
22. **`FactShapeError` mixes fact shape with dyn-surface id resolution**
    (`UnknownRelation`, `NotAKeyStatement` are not fact shapes). Split
    `DynIdError` out, or rename the type to match its roster.
23. **`AllocSnapshot` mixes two time bases in one flat struct and documents
    a counter that does not exist** (`alloc_counter.rs:114-127`): four
    window-relative fields beside one absolute, distinguished only by a
    comment, and a module doc promising a `peak-live` that no static backs.
    `{ window: AllocWindow, absolute: AllocAbsolute }`, and either implement
    the peak or delete the claim — the instance-lifetime gates report peak
    RSS from the process, not from this module, so the phantom doc is pure
    drift.

## Kept, with their recorded reasons

Audited and left alone — the ruling answers the question:

- `dict::SENTINEL_ID` as an in-band miss value (per-operator miss semantics
  fall out of ordinary word comparison; recorded). The cutover's `InternId`
  newtype gives the sentinel one typed home; the mechanism stands.
- `read/scan.rs`'s inclusive upper bound (the prefix-vs-range cursor
  divergence is deliberately observable on corrupt keys; recorded).
- `WordSet`'s zero-sentinel-plus-flag (a second occupancy array is the
  alternative; memory-justified and recorded).
- `Interval::MAX_END` as the unbounded ray ("∞ is a value of the
  representation"; recorded — and priced: it costs four guards, which the
  ruling accepts).
- `CommitPlan.inserted`'s derived byte-sorted index (recorded: the ops sit
  in commit order, which is not byte order, and the plan must stay immutable
  for bounded re-runs).

## Gates

- Behavior-preserving: every store written before and after any item is
  byte-identical; the conformance and scenario lanes are unchanged.
- Each item deletes at least one `debug_assert`, `unreachable!`, polarity
  comment, or hand-synchronized constant pair — that deletion is the review
  evidence the representation landed.
- No new `pub` surface appears anywhere in this pass.
