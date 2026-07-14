//! The measured emit paths: the per-row twins of the projection sink's
//! fast paths, entered only when the head projects a measure
//! ([`crate::exec::sink::ProjSource::Measure`]). Correctness-first by the
//! standing rule — the gathered/strided shapes stay scalar until
//! measured; the dense measure kernel lives on the filter path
//! (`crate::exec::kernel::filter_duration_range_u64`). A ray poisons the
//! sink ([`ProjectionSink::measure_of_ray`]) and every later row of the
//! execution is dropped — the error path owes no speed.

use crate::exec::colt::SuffixRun;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafScan, LeafSource};
use crate::exec::sink::{MeasuredSource, ProjSource, ProjectionSink, ProjectionSources, measure};
use crate::image::ColumnView;

impl ProjectionSink {
    /// The per-binding measured emit (key probes and tests).
    pub(super) fn emit_measured(&mut self, bindings: &Bindings) -> Flow {
        if self.ray.is_some() {
            return Flow::Continue;
        }
        let ProjectionSources::Measured(sources) = &self.sources else {
            return Flow::Continue;
        };
        for (i, source) in sources.iter().enumerate() {
            self.scratch[i] = match *source {
                ProjSource::Slot(slot) => bindings.get(slot),
                ProjSource::Measure { start } => {
                    let (s, e) = (bindings.get(start), bindings.get(start + 1));
                    let Some(duration) = measure(s, e) else {
                        self.ray = Some([s, e]);
                        return Flow::Continue;
                    };
                    duration
                }
            };
        }
        self.seen.insert(&self.scratch);
        Flow::SkipSuffix
    }

    /// Resolves the measured sources against one batch/scan shape:
    /// `key_of` maps a binding slot to its varying word index (batch key
    /// word or leaf column), `outer` reads a constant outer slot. Outer
    /// measures compute once here — a ray among them poisons and returns
    /// `Err` (the whole batch is constant-ray).
    fn resolve_measured(
        &mut self,
        key_of: impl Fn(usize) -> Option<usize>,
        outer: impl Fn(usize) -> u64,
    ) -> Result<(), ()> {
        self.measured_sources.clear();
        let ProjectionSources::Measured(sources) = &self.sources else {
            return Ok(());
        };
        for (i, source) in sources.iter().enumerate() {
            let resolved = match *source {
                ProjSource::Slot(slot) => {
                    if let Some(word) = key_of(slot) {
                        MeasuredSource::Key(word)
                    } else {
                        self.scratch[i] = outer(slot);
                        MeasuredSource::Const
                    }
                }
                ProjSource::Measure { start } => match (key_of(start), key_of(start + 1)) {
                    (Some(start_word), Some(end_word)) => {
                        MeasuredSource::MeasureKeys(start_word, end_word)
                    }
                    (None, None) => {
                        let (s, e) = (outer(start), outer(start + 1));
                        let Some(duration) = measure(s, e) else {
                            self.ray = Some([s, e]);
                            return Err(());
                        };
                        self.scratch[i] = duration;
                        MeasuredSource::Const
                    }
                    // An interval variable binds atomically: its two
                    // words are both cover keys or both outer.
                    _ => unreachable!("an interval variable binds both words together"),
                },
            };
            self.measured_sources.push(resolved);
        }
        Ok(())
    }

    /// The measured batch emit — [`ProjectionSink::emit_batch`]'s
    /// per-row twin.
    pub(super) fn emit_batch_measured(
        &mut self,
        batch: &LeafBatch<'_>,
        stop_on_skip: bool,
    ) -> Flow {
        if self.ray.is_some() {
            return Flow::Continue;
        }
        let resolved = self.resolve_measured(
            |slot| match batch.source_of(slot) {
                LeafSource::Key(word) => Some(word),
                LeafSource::Outer => None,
            },
            |slot| batch.bindings.get(slot),
        );
        if resolved.is_err() {
            return Flow::Continue;
        }
        for &entry in batch.survivors {
            for (i, source) in self.measured_sources.iter().enumerate() {
                self.scratch[i] = match *source {
                    MeasuredSource::Const => continue,
                    MeasuredSource::Key(word) => batch.key(entry, word),
                    MeasuredSource::MeasureKeys(start_word, end_word) => {
                        let (s, e) = (batch.key(entry, start_word), batch.key(entry, end_word));
                        let Some(duration) = measure(s, e) else {
                            self.ray = Some([s, e]);
                            return Flow::Continue;
                        };
                        duration
                    }
                };
            }
            self.seen.insert(&self.scratch);
            if stop_on_skip {
                // First-emit semantics (see `Sink::emit`).
                return Flow::SkipSuffix;
            }
        }
        Flow::Continue
    }

    /// The measured scan open — outer words (measures included) resolve
    /// once; a constant-ray batch poisons here and `scan_run_measured`
    /// consumes the runs without inserting.
    pub(super) fn begin_scan_measured(&mut self, scan: &LeafScan<'_>) -> bool {
        self.scan_count = 0;
        if self.ray.is_some() {
            return true;
        }
        let _ = self.resolve_measured(
            |slot| scan.key_slots.iter().position(|k| *k == slot),
            |slot| scan.bindings.get(slot),
        );
        true
    }

    /// The measured scan run — per-position, columns resolved per read
    /// (the scalar arm; the projection scan's leaf keys are width-1 by
    /// the fast-path gate, so measure halves are outer in practice and
    /// this loop is Const-dominated).
    pub(super) fn scan_run_measured(&mut self, scan: &LeafScan<'_>, run: SuffixRun<'_>) {
        self.scan_count += run.len() as u64;
        if self.ray.is_some() {
            return;
        }
        let word_at = |word: usize, position: u32| match scan.colt.suffix_column(scan.level, word) {
            ColumnView::Words(w) => w[position as usize],
            ColumnView::Bytes(b) => u64::from(b[position as usize]),
        };
        let mut each = |position: u32| {
            for (i, source) in self.measured_sources.iter().enumerate() {
                self.scratch[i] = match *source {
                    MeasuredSource::Const => continue,
                    MeasuredSource::Key(word) => word_at(word, position),
                    MeasuredSource::MeasureKeys(start_word, end_word) => {
                        let (s, e) = (word_at(start_word, position), word_at(end_word, position));
                        let Some(duration) = measure(s, e) else {
                            self.ray = Some([s, e]);
                            return false;
                        };
                        duration
                    }
                };
            }
            self.seen.insert(&self.scratch);
            true
        };
        match run {
            SuffixRun::Identity { start, len } => {
                for position in start..start + len {
                    if !each(u32::try_from(position).expect("positions fit u32")) {
                        return;
                    }
                }
            }
            SuffixRun::Positions(positions) => {
                for &position in positions {
                    if !each(position) {
                        return;
                    }
                }
            }
        }
    }
}
