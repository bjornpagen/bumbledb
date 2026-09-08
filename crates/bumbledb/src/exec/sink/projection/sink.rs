use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, ScanOffer, Sink};
use crate::exec::sink::ProjectionSink;
use crate::image::ColumnView;
use std::mem::MaybeUninit;

impl Sink for ProjectionSink {
    fn retains_binding_slot(&self, slot: usize) -> bool {
        self.sources.contains(&slot)
    }

    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for (i, source) in self.sources.iter().enumerate() {
            self.scratch[i] = bindings.get(*source);
        }
        self.seen.insert(&self.scratch);

        Flow::from_sink_progress(ProjectionSink::progress(self)).or_skip(Flow::SkipSuffix)
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        let emitted = self.project_batch(batch);
        Flow::from_sink_progress(ProjectionSink::progress(self)).or_skip(emitted)
    }

    fn emit_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        let emitted = self.project_batch_until_skip(batch);
        Flow::from_sink_progress(ProjectionSink::progress(self)).or_skip(emitted)
    }

    fn skip_capability(&self) -> crate::exec::run::SkipCapability {
        crate::exec::run::SkipCapability::Licensed
    }

    fn prepare_scan(&mut self, key_slots: &[usize]) {
        self.scan_route.prepare(key_slots, &self.sources);
    }

    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        // Unprepared direct callers, or a changed aim, use the ordinary
        // batch path until the executor installs this rule's scan layout.
        if !self.scan_route.matches(scan.key_slots, self.scratch.len()) {
            return ScanOffer::Declined;
        }
        for &(output, slot) in &self.scan_route.outer {
            self.scratch[output] = scan.bindings.get(slot);
        }
        self.scan_count = 0;
        ScanOffer::Open
    }

    #[expect(
        unsafe_code,
        reason = "The prepared route initializes each output slot exactly once"
    )]
    fn scan_run(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;

        let seen = &mut self.seen;
        let scratch = &mut self.scratch;
        let route = &self.scan_route;
        if run.len() >= crate::exec::SCAN_HOIST_THRESHOLD && !scratch.is_empty() {
            let arity = scratch.len();
            assert_eq!(
                route.keys.len() + route.outer.len(),
                arity,
                "prepared scan route"
            );
            if seen.unique_rows.is_some() {
                // SAFETY: prepare_scan partitions every output slot into
                // exactly one key or outer route. The gather only writes
                // initialized words, including when a source check panics.
                unsafe {
                    seen.insert_unique_with(run.len(), scratch, |offset, out| {
                        gather_scan_rows(scan, run, route, arity, offset, out);
                    });
                }
                return;
            }
            let rows = &mut self.scan_rows;
            // Executor scans already use this window. Enforce it here too
            // so direct callers cannot grow scratch to the physical run.
            let window = crate::exec::sink::STEP_QUANTUM as usize;
            for offset in (0..run.len()).step_by(window) {
                let words = (run.len() - offset)
                    .min(window)
                    .checked_mul(arity)
                    .expect("scan gather width fits usize");
                rows.clear();
                if rows.capacity() < words {
                    let maximum = window
                        .checked_mul(arity)
                        .expect("scan gather window fits usize");
                    // Amortize progressively larger short runs, but never
                    // retain more than one gather window.
                    let capacity = rows.capacity().saturating_mul(2).max(words).min(maximum);
                    rows.reserve_exact(capacity);
                }
                gather_scan_rows(
                    scan,
                    run,
                    route,
                    arity,
                    offset,
                    &mut rows.spare_capacity_mut()[..words],
                );
                // SAFETY: the prepared route covers every slot in every row.
                // Publish only after all columns initialize successfully.
                unsafe { rows.set_len(words) };
                seen.insert_hashed_rows(rows);
                if seen.error.is_some() {
                    break;
                }
            }
        } else {
            run_positions(run, &mut |position: u32| {
                for &(i, word) in &route.keys {
                    scratch[i] = match scan.colt.suffix_column(scan.level, word) {
                        ColumnView::Words(w) => w[position as usize],
                        ColumnView::Bytes(b) => u64::from(b[position as usize]),
                    };
                }
                seen.insert(scratch);
            });
        }
    }

    fn end_scan(&mut self, _: &LeafScan<'_>) -> u64 {
        self.scan_count
    }

    fn progress(&self) -> crate::exec::sink::SinkProgress {
        ProjectionSink::progress(self)
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        ProjectionSink::take_error(self)
    }
}

impl ProjectionSink {
    fn prepare_plain_batch_sources(&mut self, batch: &LeafBatch<'_>) {
        if !self
            .batch_route
            .matches(batch.key_slots, self.sources.len())
        {
            self.batch_route.prepare(batch.key_slots, &self.sources);
        }
        for &(output, slot) in &self.batch_route.outer {
            self.scratch[output] = batch.bindings.get(slot);
        }
    }

    #[expect(
        unsafe_code,
        reason = "The prepared route initializes every generated output slot"
    )]
    fn project_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.prepare_plain_batch_sources(batch);
        let route = &self.batch_route;
        let keys = &route.keys;
        let scratch = &mut self.scratch[..];
        let seen = &mut self.seen;
        if seen.unique_rows.is_some() && batch.survivors.len() >= crate::exec::SCAN_HOIST_THRESHOLD
        {
            let arity = scratch.len();
            assert_eq!(
                route.keys.len() + route.outer.len(),
                arity,
                "prepared batch route"
            );
            // SAFETY: prepare enumerates each output exactly once. Both
            // loops only initialize slots, and cover the whole target.
            unsafe {
                seen.insert_unique_with(batch.survivors.len(), scratch, |offset, out| {
                    // Batch keys are row-major, unlike a scan's columns.
                    // Preserve that locality while filling the final rows.
                    for (row, &entry) in out.chunks_exact_mut(arity).zip(&batch.survivors[offset..])
                    {
                        for &(output, word) in keys {
                            row[output].write(batch.key(entry, word));
                        }
                    }
                    for &(output, slot) in &route.outer {
                        let word = batch.bindings.get(slot);
                        for row in out.chunks_exact_mut(arity) {
                            row[output].write(word);
                        }
                    }
                });
            }
            return Flow::Continue;
        }
        for &entry in batch.survivors {
            for &(output, word) in keys {
                scratch[output] = batch.key(entry, word);
            }
            seen.insert(scratch);
        }
        Flow::Continue
    }

    /// Licensed-projection first-emit unwind. `SkipSuffix` after the first
    fn project_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.prepare_plain_batch_sources(batch);
        let keys = &self.batch_route.keys;
        let scratch = &mut self.scratch[..];
        let seen = &mut self.seen;
        let Some(&entry) = batch.survivors.first() else {
            return Flow::Continue;
        };
        for &(output, word) in keys {
            scratch[output] = batch.key(entry, word);
        }
        seen.insert(scratch);
        Flow::SkipSuffix
    }
}

/// Column-major gather into either the retained dense result or the one
/// reusable hash-input batch. No zero fill or intermediate row copy.
/// `ProjectionRoute::prepare` covers each output exactly once; this writer
/// only initializes words and leaves already initialized words valid on panic.
fn gather_scan_rows(
    scan: &LeafScan<'_>,
    run: SuffixRun<'_>,
    route: &super::ProjectionRoute,
    arity: usize,
    offset: usize,
    out: &mut [MaybeUninit<u64>],
) {
    debug_assert_eq!(route.keys.len() + route.outer.len(), arity);
    let len = out.len() / arity;
    for &(i, word) in &route.keys {
        let rows = out.chunks_exact_mut(arity);
        match (scan.colt.suffix_column(scan.level, word), run) {
            (ColumnView::Words(words), SuffixRun::Identity { start, .. }) => {
                for (row, &word) in rows.zip(&words[start + offset..][..len]) {
                    row[i].write(word);
                }
            }
            (ColumnView::Words(words), SuffixRun::Positions(positions)) => {
                for (row, &position) in rows.zip(&positions[offset..][..len]) {
                    row[i].write(words[position as usize]);
                }
            }
            (ColumnView::Bytes(bytes), SuffixRun::Identity { start, .. }) => {
                for (row, &byte) in rows.zip(&bytes[start + offset..][..len]) {
                    row[i].write(u64::from(byte));
                }
            }
            (ColumnView::Bytes(bytes), SuffixRun::Positions(positions)) => {
                for (row, &position) in rows.zip(&positions[offset..][..len]) {
                    row[i].write(u64::from(bytes[position as usize]));
                }
            }
        }
    }
    for &(i, slot) in &route.outer {
        let word = scan.bindings.get(slot);
        for row in out.chunks_exact_mut(arity) {
            row[i].write(word);
        }
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
