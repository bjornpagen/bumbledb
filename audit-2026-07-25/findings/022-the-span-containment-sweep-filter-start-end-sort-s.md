## The span containment sweep is duplicated verbatim between fold_stacks and FlameSummary::compute

unification | medium | CONFIRMED | cross-branching-new
outcome: fixed ce8e12b8

### Summary

`crates/bumbledb-bench/src/trace_out/fold.rs` and `crates/bumbledb-bench/src/trace_out/flame_summary.rs` carry two hand-synchronized copies of the same containment sweep — the Phase/zero-duration filter, the `(start, -end)` sort, the stack-pop walk, and the direct-child `child_ns` charging. The fold's own doc declares the two must be "identical", and the report doc declares the `.folded` artifact "matches the embedded flame summary" — but the invariant lives in two places with no shared code and no test comparing the two outputs.

### Evidence (verified)

- **Identical filter + sort key.** fold.rs:17-21:
  ```rust
  .filter(|e| e.dur_ns > 0 && e.cat != Category::Phase)
  ...
  spans.sort_by_key(|e| (e.start_ns, std::cmp::Reverse(e.start_ns + e.dur_ns)));
  ```
  flame_summary.rs:15-19 is byte-identical for both lines.
- **Identical stack sweep.** fold.rs:26-42 and flame_summary.rs:21-34 both run the same `while let Some(&top) = stack.last() { if spans[top].start_ns + spans[top].dur_ns <= event.start_ns { stack.pop() } else { break } }` walk with the same `child_ns[parent] += event.dur_ns` and `stack.push(index)`. The only difference is fold.rs additionally building `path_of` strings inside the loop — the containment logic itself is a verbatim copy.
- **The invariant is documented, not enforced.** fold.rs:6-13: "Enclosure is the same containment sweep the flame summary charges self time by … its charge is its duration minus its DIRECT children's — identical". write_chrome.rs:89-91: "The fold charges the ENGINE tree only, matching the embedded flame summary."
- **No gate.** trace_out/tests.rs tests the two independently on different fixtures (`the_flame_summary_computes_exact_self_time` at line 75, `fold_stacks_charges_self_time_by_enclosure_path` at line 132); the integration test at line 183 checks folded lines root at `execute` and end in a parseable count, but never compares fold per-path self totals against `FlameSummary` per-name `self_ns` on the same capture. A one-sided edit to either sweep passes the whole suite.

### Failure scenario / impact

A future fix lands in one copy only — e.g. a sort tie-break change, or excluding a new synthetic category from containment. The flamegraph SVG (rendered from `<stem>.folded`) and the report's embedded top-10 flame table then disagree on self-time for the same capture. Since the two artifacts are explicitly cross-referenced as one view of one capture (write_chrome.rs:85-91), the desynchronization is silent: no test fails, and anyone reconciling the SVG against the report table debugs a phantom discrepancy.

### Suggested fix

Extract one containment function in `trace_out` — e.g. `fn parented(events: &[TraceEvent]) -> (Vec<&TraceEvent>, Vec<Option<usize>> /* parent */, Vec<u64> /* child_ns */)` holding the filter, sort, and stack sweep. `fold_stacks` builds `path_of` from `parent`; `FlameSummary::compute` aggregates `dur_ns - child_ns` by name. One containment definition, and the `.folded` artifact and the embedded flame table agree by construction. As a belt-and-braces gate, add one test asserting the fold's summed self-ns per leaf name equals the FlameSummary `self_ns` for the same synthetic capture.