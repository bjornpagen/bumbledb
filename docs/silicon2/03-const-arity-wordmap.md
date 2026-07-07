# PRD 03 — Const-arity wordmap internals: delete the 1.9× genericity tax

## Purpose

Exp 15 decomposed the stats family's ~11 ns/row dedup floor: **5.6 ns
is the true floor; +4.3–4.9 ns is runtime-arity slice genericity.** The
tax is not `bcmp` calls (those died in the campaign) — it is the
inlined general-length compare/copy ladder, the `slot × arity`
multiplies on every slab index, and above all that runtime arity blocks
LLVM's automatic prefix-hash hoisting and gather-hash fusion, which the
compiler performs FOR FREE under const-generic key width (hand-fused
variants measured redundant or worse). Projected engine effect: stats
−0.8 ms of its 1.87 ms wall. The same law produced colt's
`probe_walk::<A>` monomorphs in the campaign; this PRD applies it to
the wordmap, whose keys are the sink's group tuples and full bindings.

## Technical direction

`crates/bumbledb/src/exec/wordmap.rs` (core),
`crates/bumbledb/src/exec/sink.rs` (call sites unchanged — the dispatch
is internal).

- **Const-generic core, runtime shell.** Keep `WordMap<V>`'s public
  shape (runtime `arity` field, same API — the sinks construct with
  runtime widths: group arity, binding `slot_count`). Internally, add
  `#[inline(always)] fn probe_core<const K: usize>(&self, key, hash)`
  and `fn insert_core<const K: usize>(...)` where K fixes: the hash
  loop (`hash_words` unrolled over K words — LLVM then hoists prefix
  hashes of batch-constant words and fuses gathers, per exp 15), the
  key compare (K straight-line word compares), the key copy (K stores),
  and the slab indexing (`idx * (K)` strength-reduced). Dispatch ONCE
  per operation at the public entry:
  `match self.arity { 1 => self.insert_core::<1>(..), 2 => ..::<2>,
  3 => ..::<3>, 4 => ..::<4>, 6 => ..::<6>, 8 => ..::<8>,
  _ => self.insert_core_dyn(..) }` — the match is a predictable branch
  (same arm every call from a given sink); the dyn fallback keeps the
  current code for exotic widths (5, 7, > 8).
- **Which widths get monomorphs**: {1, 2, 3, 4, 6, 8} — group keys are
  1–4; bench full-binding widths are 2–6; 8 is headroom. Verify the
  bench families' actual widths at prepare (log once under
  `cfg(test)`) and record them in `## Result`; if a bench family lands
  on the dyn arm, add its width.
- **`grow()` rehash** goes through the same dispatch (it re-probes
  every key).
- **The differential-vs-reference corpus is the law**: it already
  sweeps arities {1, 2, 4} — extend to {3, 6, 8} and a dyn-arm width
  (5), including growth boundaries and clear cycles. The false-tag
  contract tests re-run unchanged (hash values must be IDENTICAL under
  the unrolled hash — same fold order, same constants; assert this
  explicitly: `hash_core::<K>(key) == hash_words(key)` property test).
- **Disassembly gates** (extend `check-asm.sh`): the K=4 insert
  monomorph contains no `bl` (fully inlined per-element path — same
  probe-class list as the campaign's gate), no runtime-length compare
  ladder (mechanically: no `cmp` on the arity register inside the walk
  loop — assert the arity register is consumed only at the dispatch
  site; simplest robust form: the monomorph symbols, if outlined,
  contain no `udiv`/loop-over-arity shapes; pragmatically gate "no bl,
  no bcmp" and verify the ladder's absence by eye once, recorded).
- **Do NOT change geometry or probe shape** here — load factor, window
  probe, mirror, dense list all stay; this PRD is purely the
  monomorphization. (Shape changes are PRD 04.)

## Passing requirements

1. Measured (vs post-02, min-of-3): **stats p50 ≤ 1,200 µs** (from
   1,872 — exp 15's −0.8 ms projection with headroom for its own
   caveat about per-batch re-entry costs; documented-miss protocol
   with the traced descend split if it lands 1,200–1,400); skew p95
   improves (its dedup rides the same paths); spread p50 −3% or
   documented; triangle holds or improves (seen-set on its leaf path).
2. An `#[ignore]`d microbench pin: the K=4 monomorphic insert beats the
   dyn arm by ≥ 40% on a 16 MB miss-heavy fill (exp 15 measured 1.9×;
   40% is the conservative floor), proxy-bracketed, per-rep-normalized
   if suspicious.
3. Differential corpus green across {1,2,3,4,5,6,8}; hash-identity
   property test green; false-tag contract green; probe_steps pinned.
4. check-asm additions green; no family regresses > 5%; verify green;
   zero-alloc holds; emits digests unchanged.

## Out of scope

Changing the public WordMap API or sink call sites; probe-shape/pressure
work (04); colt (05/06 — its walks are already monomorphic).
