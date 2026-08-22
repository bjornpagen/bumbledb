use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, LeafSource, ScanOffer, Sink};
use crate::exec::sink::{ProjectionSink, ProjectionSources};
use crate::image::ColumnView;

impl Sink for ProjectionSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        let ProjectionSources::Plain(sources) = &self.sources;
        for (i, source) in sources.iter().enumerate() {
            self.scratch[i] = bindings.get(*source);
        }
        self.seen.insert(&self.scratch);
        // The doc's first-emit signal (40-execution D2): once a projected
        // tuple lands — new or duplicate — the current suffix can only
        // multiply witnesses. The executor's `SuffixSkip` evidence
        // (run.rs's skip-absorption arm) decides how far the skip
        // unwinds — for projections the bits come from the group key;
        // signaling on the *first* emit (not the
        // first duplicate) saves one full suffix descent per distinct
        // output tuple.
        Flow::SkipSuffix
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.project_batch(batch)
    }

    fn emit_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.project_batch_until_skip(batch)
    }

    fn skip_capability(&self) -> crate::exec::run::SkipCapability {
        crate::exec::run::SkipCapability::Licensed
    }

    /// The projection scan: positions insert straight
    /// into the seen-set — outer slots prefilled once, leaf words read
    /// live from the columns. The executor never opens a scan on a leaf
    /// that could skip (D2 leaves stay on the batch path), so every
    /// position inserts.
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        let ProjectionSources::Plain(sources) = &self.sources;
        for (i, slot) in sources.iter().enumerate() {
            self.batch_sources[i] = scan
                .key_slots
                .iter()
                .position(|k| k == slot)
                .map_or(LeafSource::Outer, LeafSource::Key);
        }
        for (i, slot) in sources.iter().enumerate() {
            if matches!(self.batch_sources[i], LeafSource::Outer) {
                self.scratch[i] = scan.bindings.get(*slot);
            }
        }
        self.scan_count = 0;
        ScanOffer::Open
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
                if let LeafSource::Key(word) = *source {
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
                    if let LeafSource::Key(word) = source {
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

    fn end_scan(&mut self, _: &LeafScan<'_>) -> u64 {
        self.scan_count
    }
}

impl ProjectionSink {
    fn prepare_plain_batch_sources(&mut self, batch: &LeafBatch<'_>) {
        let ProjectionSources::Plain(sources) = &self.sources;
        for (i, source) in sources.iter().enumerate() {
            self.batch_sources[i] = batch.source_of(*source);
        }
        for (i, source) in sources.iter().enumerate() {
            if matches!(self.batch_sources[i], LeafSource::Outer) {
                self.scratch[i] = batch.bindings.get(*source);
            }
        }
    }

    /// Consume every surviving row. Forbidden nodes take this path.
    fn project_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.prepare_plain_batch_sources(batch);
        let batch_sources = &self.batch_sources[..];
        let scratch = &mut self.scratch[..];
        let seen = &mut self.seen;
        for &entry in batch.survivors {
            for (i, source) in batch_sources.iter().enumerate() {
                if let LeafSource::Key(word) = source {
                    scratch[i] = batch.key(entry, *word);
                }
            }
            seen.insert(scratch);
        }
        Flow::Continue
    }

    /// Licensed-projection first-emit unwind. `SkipSuffix` after the first
    /// insert; remaining rows bind nothing sink-relevant.
    fn project_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.prepare_plain_batch_sources(batch);
        let batch_sources = &self.batch_sources[..];
        let scratch = &mut self.scratch[..];
        let seen = &mut self.seen;
        let Some(&entry) = batch.survivors.first() else {
            return Flow::Continue;
        };
        for (i, source) in batch_sources.iter().enumerate() {
            if let LeafSource::Key(word) = source {
                scratch[i] = batch.key(entry, *word);
            }
        }
        seen.insert(scratch);
        Flow::SkipSuffix
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
