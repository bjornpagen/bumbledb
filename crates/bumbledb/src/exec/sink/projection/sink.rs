use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, LeafSource, Sink};
use crate::exec::sink::ProjectionSink;
use crate::image::ColumnView;

impl Sink for ProjectionSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for (i, slot) in self.slots.iter().enumerate() {
            self.scratch[i] = bindings.get(*slot);
        }
        self.seen.insert(&self.scratch);
        // The doc's first-emit signal (30-execution D2): once a projected
        // tuple lands — new or duplicate — the current suffix can only
        // multiply witnesses. The executor's sink_relevant gating
        // (run.rs's skip-absorption arm) decides how far the skip
        // unwinds — for projections the bits come from the group key;
        // signaling on the *first* emit (not the
        // first duplicate) saves one full suffix descent per distinct
        // output tuple.
        Flow::SkipSuffix
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
        // Sources resolved at batch entry (per-slot, not per-row); the
        // outer values refresh per batch (bindings vary per parent), the
        // row loop touches only the varying key words and the seen-set.
        for (i, slot) in self.slots.iter().enumerate() {
            self.batch_sources[i] = match batch.source_of(*slot) {
                LeafSource::Key(word) => Some(word),
                LeafSource::Outer => None,
            };
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = batch.bindings.get(*slot);
            }
        }
        // Direct per-row insert — NO hash-ahead pipeline (measured):
        // the ping-pong measured +1.2–2.4 ns/row in
        // this exact shape once the window probe removed the flush
        // exposure it was built to shadow; the deletion IS the
        // optimization.
        // Alias-hoisted locals: the row loop reads
        // `batch_sources` and writes `scratch` — disjoint reborrows
        // taken once keep both headers in registers.
        let batch_sources = &self.batch_sources[..];
        let scratch = &mut self.scratch[..];
        let seen = &mut self.seen;
        for &entry in batch.survivors {
            for (i, source) in batch_sources.iter().enumerate() {
                if let Some(word) = source {
                    scratch[i] = batch.key(entry, *word);
                }
            }
            seen.insert(scratch);
            if stop_on_skip {
                // First-emit semantics (see `emit`): the remaining rows
                // bind nothing sink-relevant — the executor unwinds.
                return Flow::SkipSuffix;
            }
        }
        Flow::Continue
    }

    fn may_skip(&self) -> bool {
        true
    }

    /// The projection scan: positions insert straight
    /// into the seen-set — outer slots prefilled once, leaf words read
    /// live from the columns. The executor never opens a scan on a leaf
    /// that could skip (D2 leaves stay on the batch path), so every
    /// position inserts.
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool {
        for (i, slot) in self.slots.iter().enumerate() {
            self.batch_sources[i] = scan.key_slots.iter().position(|k| k == slot);
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = scan.bindings.get(*slot);
            }
        }
        self.scan_count = 0;
        true
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        // Direct per-row inserts, like every sink path (measured):
        // the pipeline ping-pong
        // measured as pure overhead everywhere — here first (range +10%
        // while it was here: a projection scan's inserts are nearly all
        // first-sight misses, whose predicted exit branch exposes no
        // hash latency), then on the dedup paths (the in-shape
        // measurement). Run-length-adaptive column resolution
        // splits the arms: big runs amortize a hoisted
        // column table, fanout-sized runs resolve per position.
        let seen = &mut self.seen;
        let scratch = &mut self.scratch;
        let sources = &self.batch_sources;
        if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD {
            assert!(sources.len() <= 8, "projection arity cap");
            // Option-free hoist table built by a plain indexed loop
            // (measured): `array::from_fn` refuses to inline its
            // element closure (rust-lang/rust#108765) — measured ~34 ns
            // per 8-entry Option table (eight outlined calls + a 448 B
            // memcpy) vs ~3.4 ns for straight-line stores. `sources[i]`
            // already gates which entries are live; no Option needed.
            let mut cols: [ColumnView<'_>; 8] = [ColumnView::Words(&[]); 8];
            for (i, source) in sources.iter().enumerate() {
                if let Some(word) = *source {
                    cols[i] = scan.colt.suffix_column(scan.level, word);
                }
            }
            run_positions(run, &mut |position: u32| {
                for (i, source) in sources.iter().enumerate() {
                    if source.is_some() {
                        scratch[i] = match cols[i] {
                            ColumnView::Words(w) => w[position as usize],
                            ColumnView::Bytes(b) => u64::from(b[position as usize]),
                        };
                    }
                }
                seen.insert(scratch);
            });
        } else {
            run_positions(run, &mut |position: u32| {
                for (i, source) in sources.iter().enumerate() {
                    if let Some(word) = source {
                        scratch[i] = match scan.colt.suffix_column(scan.level, *word) {
                            ColumnView::Words(w) => w[position as usize],
                            ColumnView::Bytes(b) => u64::from(b[position as usize]),
                        };
                    }
                }
                seen.insert(scratch);
            });
        }
    }

    fn end_scan(&mut self, _scan: &LeafScan<'_>) -> u64 {
        self.scan_count
    }
}

/// Drives `f` over every position of a run (the projection scan's loop).
fn run_positions(run: SuffixRun<'_>, f: &mut impl FnMut(u32)) {
    match run {
        SuffixRun::Identity { start, len } => {
            for position in start..start + len {
                f(u32::try_from(position).expect("positions fit u32"));
            }
        }
        SuffixRun::Positions(positions) => {
            for &position in positions {
                f(position);
            }
        }
    }
}
