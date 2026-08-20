# 24 — `dyn FnMut` intern resolver in the judgment hot path

- **Status:** **fixed this pass** — `encode_with`/`resolve_side`/`resolve_checks`
  take `F: FnMut(&[u8]) -> Result<Option<InternId>>`; the `InternResolver`
  alias is deleted. Re-verified 2026-08-19: no `dyn` remains in
  `judgment.rs`. Gate: `cargo test -p bumbledb --lib storage::commit::tests`
  (151 passed).
- **Severity:** zero-dyn law + performance, in one change.

## Principle

Insight 9: monomorphization is the engine's dispatch story everywhere else
(`Sink`, `Counters`, `Operands`, the catalog GATs). One `dyn FnMut` in the
judgment path is both a law violation and a per-resolution indirect call on
a hot path.

## Evidence

```rust
// judgment.rs:326
type InternResolver<'a> = dyn FnMut(&[u8]) -> Result<Option<InternId>> + 'a;
```

## The fix

Genericize: the consumers take `F: FnMut(&[u8]) -> Result<Option<InternId>>`
(or the resolver becomes a small concrete enum over its two real sources —
delta-overlay and committed-dict — if exactly two exist; prefer whichever
deletes more indirection). The type alias deletes.

## Acceptance

- No `dyn` in `judgment.rs`; the zero-dyn census (integration gate) passes.
- Judgment lanes byte-identical verdicts; commit-lane timings not worse
  (attribution note in the commit body).

## Adjudication

The filed fix offered `F: FnMut` or a two-arm enum "if exactly two
exist." Three call sites resolve intern bytes:
`Selections::encode` (delta overlay), `encode_committed` (committed
dict), and `encode_lookup` (heap-admit / freeze). A two-arm enum would
leave the third as another trait object or a third arm, so `F: FnMut`
is the form that deletes the indirection. Closures capture by
reference (`delta`, `view`, `stage`) — no boxed resolver state.
