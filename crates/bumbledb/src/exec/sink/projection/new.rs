use crate::error::Result;
use crate::exec::scratch::{ScratchAppend, ScratchMapId, ScratchRelation};
use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{
    FindSpec, ProjectionSink, ProjectionSources, SinkBudget, SinkSpec, SpillSet, StageRowVisit,
    encode_stage_row, sources_of,
};
use crate::work::WorkContext;

impl ProjectionSink {
    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        Self::with_capacity_hint_sources(ProjectionSources::Plain(slots), 0)
    }

    #[must_use]
    fn with_capacity_hint_sources(sources: ProjectionSources, hint: usize) -> Self {
        let arity = match &sources {
            ProjectionSources::Plain(slots) => slots.len(),
        };
        Self {
            finds: Vec::new(),
            sources,
            seen: SpillSet::with_capacity_hint(arity, hint, true),
            scratch: vec![0; arity],
            batch_sources: vec![crate::exec::run::LeafSource::Outer; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    #[must_use]
    pub fn with_capacity_hint(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        let parsed = parse_finds(finds, slot_count);
        let sources = sources_of(&parsed);
        let mut sink = Self::with_capacity_hint_sources(sources, hint);
        sink.finds = parsed;
        sink
    }

    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        parse_finds_into(finds, slot_count, &mut self.finds);
        match &mut self.sources {
            ProjectionSources::Plain(slots) => {
                slots.clear();
                for find in &self.finds {
                    if let SinkSpec::Var { slot, width } = find {
                        slots.extend(*slot..slot + width);
                    }
                }
            }
        }
        debug_assert_eq!(
            match &self.sources {
                ProjectionSources::Plain(slots) => slots.len(),
            },
            self.scratch.len(),
            "one head, fixed word arity"
        );
    }

    /// RAM-tier answers (the main sink's warm finalize fill). Spilled
    /// sinks drain through [`Self::for_each_answer`]; budgeted sinks
    /// (main, interior stages, the reach driver) that crossed their
    /// allowance drain through [`Self::for_each_answer`]/
    /// [`Self::drain_since`], never this iterator — callers branch on
    /// [`Self::spilled`] first.
    pub fn answers(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.ram_iter_since(0)
    }

    /// Insertion-ordered drain across both tiers (finalize's spilled arm).
    /// # Errors
    /// A sticky sink failure, scratch read failure, stopped work, or the
    /// visitor's failure.
    pub(crate) fn for_each_answer(
        &mut self,
        visit: &mut dyn FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        self.seen.for_each_since(0, &mut |row| {
            visit(row)?;
            Ok(true)
        })
    }

    /// Insertion-ordered drain from `since` across both tiers. Fallible
    /// and early-stoppable (`Ok(false)`). L05 refill/seal must propagate
    /// `Err` immediately — do not collect then write.
    /// # Errors
    /// As [`Self::for_each_answer`].
    pub(crate) fn drain_since(&mut self, since: usize, visit: StageRowVisit<'_>) -> Result<()> {
        self.seen.for_each_since(since, visit)
    }

    /// Admit a RAM-first dest for [`Self::stream_into_scratch`]. Does not
    /// open a scratch environment.
    #[must_use]
    pub(crate) fn admit_dest(work: &WorkContext, ram_bytes: usize) -> ScratchRelation {
        ScratchRelation::new(work, ram_bytes)
    }

    /// Stream answers from `since` through one [`ScratchAppend`] on `dest`,
    /// one encoded row per append starting at `start_seq`. `dest` must be
    /// admitted (`admit_dest` / `ScratchRelation::new`) — this method
    /// never `force_spill`s. Tiny outputs stay on dest's RAM tier.
    /// Failure returns immediately and drops the visitor (no `finish`).
    /// Returns the number of rows written.
    /// # Errors
    /// Sticky sink failure, stopped work, or a refused scratch append.
    pub(crate) fn stream_into_scratch(
        &mut self,
        dest: &mut ScratchRelation,
        since: usize,
        start_seq: u64,
    ) -> Result<u64> {
        let mut seq = start_seq;
        let mut encoded = Vec::new();
        let mut append = ScratchAppend::new(dest);
        let streamed = self.drain_since(since, &mut |row| {
            encode_stage_row(row, &mut encoded);
            append.append(ScratchMapId::Default, &seq.to_be_bytes(), &encoded)?;
            seq += 1;
            Ok(true)
        });
        match streamed {
            Ok(()) => {
                append.finish()?;
                Ok(seq - start_seq)
            }
            Err(error) => {
                drop(append);
                Err(error)
            }
        }
    }

    /// Install this execution's allowance (None = RAM-only stage sink).
    pub(crate) fn begin(&mut self, budget: Option<SinkBudget>) {
        self.seen.begin(budget);
    }

    #[must_use]
    pub(crate) fn spilled(&self) -> bool {
        self.seen.spilled()
    }

    /// The sticky failure recorded by the infallible emit path, if any.
    pub(crate) fn take_error(&mut self) -> Option<crate::error::Error> {
        self.seen.take_error()
    }

    /// L05 Continue/Stop/Error. Finish is the successful drain L05 observes.
    #[must_use]
    pub(crate) fn progress(&self) -> crate::exec::sink::SinkProgress {
        self.seen.progress()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    #[must_use]
    #[expect(
        dead_code,
        reason = "the companion API documents and preserves the type contract"
    )]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn reset(&mut self) {
        self.seen.clear();
    }
}
