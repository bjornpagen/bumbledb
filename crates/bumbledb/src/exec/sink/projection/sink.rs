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

        let seen = &mut self.seen;
        let scratch = &mut self.scratch;
        let sources = &self.batch_sources;
        if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD {
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
