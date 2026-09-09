use crate::error::Result;
use crate::exec::scratch::{ScratchAppend, ScratchMapId, ScratchRelation};
use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{
    FindSpec, ProjectionSink, ResidentRows, SpillSet, StageRowVisit, encode_stage_row,
    extend_sources, sources_of,
};

impl ProjectionSink {
    #[cfg(test)]
    pub(crate) fn output_hashing_is_elided(&self) -> bool {
        self.seen.unique_rows.is_some()
    }

    /// Install the exact rule/head proof before execution. The caller must
    /// not share this sink across union arms, stages or recursive iterations.
    pub(crate) fn elide_output_hashing(
        &mut self,
        witness: crate::plan::fj::ProjectionDistinctWitness,
    ) {
        self.seen.elide_output_hashing(witness);
    }

    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        let slot_count = slots.iter().max().map_or(0, |slot| slot + 1);
        Self::from_sources(slots, slot_count)
    }

    #[must_use]
    fn from_sources(sources: Vec<usize>, slot_count: usize) -> Self {
        let arity = sources.len();
        Self {
            finds: Vec::new(),
            sources,
            seen: SpillSet::new(arity, true),
            scratch: vec![0; arity],
            batch_route: super::ProjectionRoute::new(arity, slot_count),
            scan_route: super::ProjectionRoute::new(arity, slot_count),
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    #[must_use]
    pub fn from_finds(finds: &[FindSpec], slot_count: usize) -> Self {
        let parsed = parse_finds(finds);
        let sources = sources_of(&parsed);
        let mut sink = Self::from_sources(sources, slot_count);
        sink.finds = parsed;
        sink
    }

    pub fn aim(&mut self, finds: &[FindSpec]) {
        self.scan_route.clear();
        self.batch_route.clear();
        parse_finds_into(finds, &mut self.finds);
        extend_sources(&self.finds, &mut self.sources);
        debug_assert_eq!(
            self.sources.len(),
            self.scratch.len(),
            "one head, fixed word arity"
        );
    }

    /// RAM-tier answers (the main sink's warm finalize fill). Spilled
    /// sinks drain through [`Self::for_each_answer`]/[`Self::drain_since`],
    /// never this iterator — callers branch on [`Self::spilled`] first.
    pub fn answers(
        &self,
    ) -> ResidentRows<impl Iterator<Item = &[u64]> + Clone, impl Iterator<Item = &[u64]> + Clone>
    {
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

    /// Stream answers from `since` through one [`ScratchAppend`] on `dest`,
    /// one encoded row per append starting at `start_seq`. `dest` must be
    /// created by the caller — this method
    /// never `force_spill`s. Tiny outputs stay on dest's RAM tier.
    /// Failure returns immediately and drops the visitor (no `finish`).
    /// `retain` transfers any payload ownership before its row is appended.
    /// Returns the number of rows written.
    /// # Errors
    /// Sticky sink failure, stopped work, or a refused scratch append.
    pub(crate) fn stream_into_scratch(
        &mut self,
        dest: &mut ScratchRelation,
        since: usize,
        start_seq: u64,
        mut retain: impl FnMut(&[u64]) -> Result<()>,
    ) -> Result<u64> {
        let mut seq = start_seq;
        let mut encoded = Vec::new();
        let mut append = ScratchAppend::new(dest);
        let streamed = self.drain_since(since, &mut |row| {
            retain(row)?;
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

    /// Install this execution's cancellation context (None for standalone kernels).
    pub(crate) fn begin(&mut self, work: Option<crate::work::WorkContext>) {
        self.seen.begin(work);
    }

    #[must_use]
    pub(crate) fn spilled(&self) -> bool {
        self.seen.spilled()
    }

    /// Exercise the disk representation without inventing a resource policy.
    #[cfg(test)]
    pub(crate) fn force_spill(&mut self) -> crate::error::Result<()> {
        self.seen.spill()
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
        self.scratch.resize(self.sources.len(), 0);
    }

    pub(crate) fn release_memory(&mut self) {
        self.seen.release_memory();
        self.scratch = Vec::new();
        self.batch_route = super::ProjectionRoute::default();
        self.scan_route = super::ProjectionRoute::default();
        self.scan_rows = Vec::new();
        self.scan_count = 0;
    }
}
