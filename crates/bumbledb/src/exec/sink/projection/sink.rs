use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, LeafSource, Sink};
use crate::exec::sink::ProjectionSink;
use crate::image::ColumnView;

impl Sink for ProjectionSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        if self.has_measures {
            return self.emit_measured(bindings);
        }
        for (i, source) in self.sources.iter().enumerate() {
            self.scratch[i] = bindings.get(source.plain_slot());
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
        if self.has_measures {
            return self.emit_batch_measured(batch, stop_on_skip);
        }
        // Sources resolved at batch entry (per-slot, not per-row); the
        // outer values refresh per batch (bindings vary per parent), the
        // row loop touches only the varying key words and the seen-set.
        for (i, source) in self.sources.iter().enumerate() {
            self.batch_sources[i] = match batch.source_of(source.plain_slot()) {
                LeafSource::Key(word) => Some(word),
                LeafSource::Outer => None,
            };
        }
        for (i, source) in self.sources.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = batch.bindings.get(source.plain_slot());
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
        if self.has_measures {
            return self.begin_scan_measured(scan);
        }
        for (i, source) in self.sources.iter().enumerate() {
            let slot = source.plain_slot();
            self.batch_sources[i] = scan.key_slots.iter().position(|k| *k == slot);
        }
        for (i, source) in self.sources.iter().enumerate() {
            if self.batch_sources[i].is_none() {
                self.scratch[i] = scan.bindings.get(source.plain_slot());
            }
        }
        self.scan_count = 0;
        true
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        if self.has_measures {
            return self.scan_run_measured(scan, run);
        }
        self.scan_count += run.len() as u64;
        // Direct per-row inserts, like every sink path (measured):
        // the pipeline ping-pong
        // measured as pure overhead everywhere — here first (range +10%
        // while it was here: a projection scan's inserts are nearly all
        // first-sight misses, whose predicted exit branch exposes no
        // hash latency), then on the dedup paths (the in-shape
        // measurement). Run-length-adaptive column resolution
        // splits the arms: big runs resolve each column once
        // (column-hoisted), fanout-sized runs resolve per position.
        let seen = &mut self.seen;
        let scratch = &mut self.scratch;
        let sources = &self.batch_sources;
        if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD {
            // Column-hoisted emit (the gather kernels' idiom — columns
            // outer, positions inner): each projected leaf column
            // resolves its view once and writes the run's span into the
            // row-major staging rows; outer slots broadcast their
            // prefilled scratch word. No fixed-width scratch exists —
            // the staging buffer is `run × arity` words (retained
            // capacity), so the projection width is unbounded by
            // construction.
            let arity = sources.len();
            let rows = &mut self.scan_rows;
            rows.resize(run.len() * arity, 0);
            for (i, source) in sources.iter().enumerate() {
                if let Some(word) = *source {
                    match (scan.colt.suffix_column(scan.level, word), run) {
                        (ColumnView::Words(w), SuffixRun::Identity { start, len }) => {
                            for (k, value) in w[start..start + len].iter().enumerate() {
                                rows[k * arity + i] = *value;
                            }
                        }
                        (ColumnView::Words(w), SuffixRun::Positions(positions)) => {
                            for (k, position) in positions.iter().enumerate() {
                                rows[k * arity + i] = w[*position as usize];
                            }
                        }
                        (ColumnView::Bytes(bytes), SuffixRun::Identity { start, len }) => {
                            for (k, value) in bytes[start..start + len].iter().enumerate() {
                                rows[k * arity + i] = u64::from(*value);
                            }
                        }
                        (ColumnView::Bytes(bytes), SuffixRun::Positions(positions)) => {
                            for (k, position) in positions.iter().enumerate() {
                                rows[k * arity + i] = u64::from(bytes[*position as usize]);
                            }
                        }
                    }
                } else {
                    let word = scratch[i];
                    for row in rows.chunks_exact_mut(arity) {
                        row[i] = word;
                    }
                }
            }
            for row in rows.chunks_exact(arity) {
                seen.insert(row);
            }
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
