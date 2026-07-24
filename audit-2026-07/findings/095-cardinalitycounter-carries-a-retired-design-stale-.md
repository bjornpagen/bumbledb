## CardinalityCounter carries a retired design: stale shared-counter doc, dead memset, two-array set representation

category: incoherence | severity: low | verdict: CONFIRMED | finder: engine:storage
outcome: fixed 537064d8

### Summary

`crates/bumbledb/src/image/cardinality.rs` still carries the documentation and clearing logic of a retired eager design. The struct doc calls `CardinalityCounter` "the **build-time** distinct counter: a power-of-two open-addressed word set **sized once** for the row count and **memset-cleared per column**" — but under the current lazy design a fresh counter is constructed inside each per-column `get_or_init` closure, so the counter is never shared, never reused, and the `self.occupied.fill(false)` at the top of `count_words` always runs on memory that `vec![false; capacity]` just zeroed. Every first-demanded word column also pays a fresh ≥18 bytes/row allocation (up to ~2× after power-of-two rounding) that the retired design amortized into one scratch set.

### Evidence

All verified by reading the file and the git history:

- `cardinality.rs:18-19` — `*self.distincts[column].get_or_init(|| match self.column(column) { ColumnView::Words(words) => CardinalityCounter::new(self.row_count).count_words(words), ... })`. A fresh counter per column; `CardinalityCounter` is private to this module and this is the only word-path construction site (grep confirms no other users).
- `cardinality.rs:25-26` — the stale doc: "The build-time distinct counter: a power-of-two open-addressed word set sized once for the row count and memset-cleared per column."
- `cardinality.rs:36-37` vs `cardinality.rs:42` — `slots: vec![0; capacity], occupied: vec![false; capacity]` in `new()`, immediately followed (on the only call path) by `self.occupied.fill(false)`: dead work on freshly-zeroed memory.
- **Git archaeology confirms the "retired design" narrative exactly.** `git log -S "memset-cleared per column"` traces the phrase to commit `08bbfab0` ("Give the planner honest cardinalities (perf PRD 07)"), where the build path did `let mut counter = DistinctCounter::new(row_count);` once and mapped it across every word column eagerly inside the build window — there the doc was true and the per-column clear was load-bearing. When the design became lazy (per-column `OnceLock`, already so by the `18bd9d85` restyle), the doc and the clear survived unmodified. The module header at `cardinality.rs:1-3` ("computed on first demand and memoized") now directly contradicts the struct doc twenty lines below it.
- `plan/selectivity.rs:295,302` — the planner demands `image.cardinality(column)` per column on the estimate path, including a `.map(|column| image.cardinality(column))` over a field's column span (interval fields have two word columns each).
- Checked against `docs/architecture/40-execution.md` (selectivity ladder, "resident-image distinct counts (peeked, never built)"): the lazy memoized design itself is the documented contract — the finding is not that laziness is wrong, but that the code's comments and clearing logic describe the eager predecessor.

### Bench impact

Not a correctness bug. On a wide relation (e.g. 12 word columns × 150k rows: capacity = 2^19, slots 4 MiB + occupied 512 KiB ≈ 4.5 MB per column), the first planner estimate that touches all columns performs 12 separate ~4.5 MB zeroing allocations plus 12 dead 512 KiB memsets, where the 08bbfab0 design performed one allocation total. Memoization on the image's `OnceLock` makes this a once-per-column-per-image cost, not per-query — hence low severity — but this is a hidden hot-path allocation profile the stale doc actively misdescribes to the next reader ("sized once" is false). The parallel `slots`/`occupied` two-vector shape is also the kind of flag-array a better representation erases (representation-first doctrine, `docs/design/representation-first.md`): the occupancy bit is a second array standing in for an in-band encoding.

### Suggested fix

Minimum: delete the dead `self.occupied.fill(false)` (line 42) and rewrite the struct doc to state the per-column-construction reality ("built fresh per first-demanded column, memoized on the image"). Better: fold `occupied` into `slots` — either a reserved-sentinel scheme (0 is a legal word, so remap it or reserve one slot) or a generation-stamp word — so the set is one array and the occupancy flag stops existing as separate state. If build-window profiling ever shows the first-estimate allocation spike, restore the shared shape as one lazy scratch at the image level with a per-column clear — which is exactly the design the current doc already describes.
