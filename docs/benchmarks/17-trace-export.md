# PRD 17 — Trace export: Chrome Trace Format and the flame summary

Authority: PRDs 02–04 (the events), README rule 4 (hand-rolled JSON — no serde),
the "full traces, truly data-driven" mandate.

## Purpose

Every captured run becomes an artifact a human opens in Perfetto/chrome://tracing,
plus a terminal flame summary — where-the-time-goes without leaving the repo.

## Technical direction

- `trace_out::write_chrome(events: &[obs::TraceEvent], harness: &[obs::TraceEvent],
  out: &mut impl Write) -> io::Result<()>` emitting the Chrome Trace Event Format:
  a JSON array of complete events —
  `{"name":"…","cat":"…","ph":"X","ts":<start µs, f64 with .3>,"dur":<µs>,
  "pid":1,"tid":<1 engine | 2 harness>,"args":{"a0":n,"a1":n}}` — and instant
  events (`"ph":"i","s":"t"`) for dur-0 events. Hand-rolled writer: names are
  `&'static str` from `obs::names` (assert-only-ASCII in a test rather than
  escaping machinery; one `debug_assert` in the writer for `"` and `\\`).
  Timestamps: ns → µs division with 3 decimals, monotonic as captured.
- `trace_out::FlameSummary`: aggregate by span name — `{ calls, total_ns,
  self_ns, p50_ns, max_ns }` where self = total minus the sum of *directly
  nested* children (compute containment by a stack sweep over start/end order —
  events arrive in drop order; re-sort by (start, -end) and walk). Render as an
  aligned text table sorted by self time, top 24 rows, plus a `total wall` line.
- Bench integration (consumed by PRD 18/19): for each family in a `--trace` run,
  one extra traced warm sample and one traced cold sample (reads only) →
  `out_dir/trace/<family>.warm.json`, `.cold.json`; write families get one
  traced sample each. The flame summary of the warm trace embeds in the report.
- The harness contributes its own spans (Category::Harness): `sample`,
  `touch` — so tool overhead is visible inside the same trace, honestly
  separated by tid.

## Non-goals

Streaming/incremental trace files (drain-then-write; captures are one sample).
Perfetto protobufs (JSON is the format — universally loadable).

## Passing criteria

- Unit tests: writer output for a hand-built 5-event capture is golden
  (byte-exact string) and structurally valid (balanced brackets, one object per
  event, ts monotone nondecreasing in file order); FlameSummary on a synthetic
  nested capture computes exact self/total (outer 100 µs containing inner
  60 µs ⇒ outer self 40 µs); the table render golden; a real captured S-scale
  fk_walk trace contains `execute`, `join`, and either `view_build` or
  `view_memo_hit`, and its summary totals ≈ the `execute` span ±5%.
- `scripts/check.sh` green.
