use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{
    FindSpec, ProjectionSink, ProjectionSources, SinkBudget, SinkSpec, SpillSet, sources_of,
};

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
    /// sinks drain through [`Self::for_each_answer`]; interior stage sinks
    /// never receive a budget and never spill. The reach driver's sink CAN
    /// spill — its refills use [`Self::drain_since`], never this iterator.
    pub fn answers(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.ram_iter_since(0)
    }

    /// Insertion-ordered drain across both tiers (finalize's spilled arm).
    /// # Errors
    /// A sticky sink failure, scratch read failure, stopped work, or the
    /// visitor's failure.
    pub(crate) fn for_each_answer(
        &mut self,
        visit: &mut dyn FnMut(&[u64]) -> crate::error::Result<()>,
    ) -> crate::error::Result<()> {
        self.seen.for_each_since(0, visit)
    }

    /// Insertion-ordered drain from `since` across both tiers — the reach
    /// driver's Δ/accumulated/seal refills (the rec seen/frontier state
    /// keeps its watermark contract after the RAM→scratch transition).
    /// # Errors
    /// As [`Self::for_each_answer`].
    pub(crate) fn drain_since(
        &mut self,
        since: usize,
        write: &mut dyn FnMut(&[u64]),
    ) -> crate::error::Result<()> {
        self.seen.for_each_since(since, &mut |row| {
            write(row);
            Ok(())
        })
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
