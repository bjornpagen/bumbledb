# CONTAMINATED — excluded and counted

This run stays committed as the honest record; this marker file is the
machine-readable exclusion (`scripts/bench_viz.py` skips a run dir
carrying `CONTAMINATED.md` and counts it in the chart footers — the
contamination record is data ON the pin, never a footnote someone must
remember).

**What happened (owner ruling, 2026-07-20):** the night ran under the
`--shared` protocol (boosted QoS, the owner's background agents live on
the same machine). During bench-durable-r2's write window, agent load
landed on the box and hit the run's write families. The trace is in the
report itself: `all_win: false` — `entries_for_account_set` records the
night's only `LOSS` verdict, absent in r1 and r3 on the identical
binary, corpus, and seed.

**The doctrine** (docs/architecture/61-bench-lanes.md, the shared-machine
ruling): interleaved A/B sampling and contamination exclude-and-count
are the honesty floor. Headline numbers derive min-over-clean — the
durable pool is r1 + r3; the ephemeral pool keeps all three runs. This
run's numbers are quoted nowhere.
