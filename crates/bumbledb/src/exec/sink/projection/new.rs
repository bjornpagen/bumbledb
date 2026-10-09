use crate::error::Result;
use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{
    FindSpec, ProjectionSink, ResidentRows, SeenSet, StageRowVisit, extend_sources, sources_of,
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
        Self::with_capacity_hint_sources(slots, slot_count, 0)
    }

    #[must_use]
    fn with_capacity_hint_sources(sources: Vec<usize>, slot_count: usize, hint: usize) -> Self {
        let arity = sources.len();
        Self {
            finds: Vec::new(),
            sources,
            seen: SeenSet::with_capacity_hint(arity, hint, true),
            scratch: vec![0; arity],
            batch_route: super::ProjectionRoute::new(arity, slot_count),
            scan_route: super::ProjectionRoute::new(arity, slot_count),
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    #[must_use]
    pub fn with_capacity_hint(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        let parsed = parse_finds(finds);
        let sources = sources_of(&parsed);
        let mut sink = Self::with_capacity_hint_sources(sources, slot_count, hint);
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

    /// Answers in insertion order (the main sink's warm finalize fill).
    pub fn answers(
        &self,
    ) -> ResidentRows<impl Iterator<Item = &[u64]> + Clone, impl Iterator<Item = &[u64]> + Clone>
    {
        self.seen.ram_iter_since(0)
    }

    /// Insertion-ordered fallible drain.
    /// # Errors
    /// A sticky sink failure or the visitor's failure.
    #[cfg(test)]
    pub(crate) fn for_each_answer(
        &mut self,
        visit: &mut dyn FnMut(&[u64]) -> Result<()>,
    ) -> Result<()> {
        self.seen.for_each_since(0, &mut |row| {
            visit(row)?;
            Ok(true)
        })
    }

    /// Insertion-ordered drain from `since`. Fallible and early-stoppable
    /// (`Ok(false)`); stage refills propagate `Err` immediately.
    /// # Errors
    /// As [`Self::for_each_answer`].
    pub(crate) fn drain_since(&mut self, since: usize, visit: StageRowVisit<'_>) -> Result<()> {
        self.seen.for_each_since(since, visit)
    }

    /// Install this execution's cancellation context (None for standalone kernels).
    pub(crate) fn begin(&mut self, work: Option<crate::work::WorkContext>) {
        self.seen.begin(work);
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
