use crate::exec::sink::{extend_sources, FindSpec, ProjSource, ProjectionSink};
use crate::exec::wordmap::WordMap;

impl ProjectionSink {
    /// `sources`: the projected word sources in find-**word** order (see
    /// [`crate::exec::sink::sources_of`]). (Tests; production sinks are
    /// hint-sized.)
    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        Self::with_capacity_hint(slots.into_iter().map(ProjSource::Slot).collect(), 0)
    }

    /// Presized construction: `hint` is the plan's
    /// output-cardinality estimate — the seen-set allocates once instead
    /// of rehash-doubling through the first measured execution.
    #[must_use]
    pub fn with_capacity_hint(sources: Vec<ProjSource>, hint: usize) -> Self {
        let arity = sources.len();
        let has_measures = sources
            .iter()
            .any(|s| matches!(s, ProjSource::Measure { .. }));
        Self {
            sources,
            has_measures,
            ray: None,
            measured_sources: Vec::new(),
            seen: WordMap::with_capacity_hint(arity, hint),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    /// Re-aims the projected sources at one rule's binding layout (the
    /// rule loop, docs/architecture/40-execution.md): the head's word
    /// arity is fixed — types and widths are the head's — but each rule
    /// supplies its own slots. The seen-set is untouched: its keys are
    /// projected (head-shaped) tuples, rule-independent by construction,
    /// and its spanning rules IS the union. Single-rule sinks are built
    /// aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec]) {
        extend_sources(finds, &mut self.sources);
        self.has_measures = self
            .sources
            .iter()
            .any(|s| matches!(s, ProjSource::Measure { .. }));
        debug_assert_eq!(
            self.sources.len(),
            self.scratch.len(),
            "one head, fixed word arity"
        );
    }

    /// The distinct projected tuples, unordered (results are sets; the
    /// host sorts).
    pub fn rows(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.iter().map(|(key, ())| key)
    }

    /// Distinct rows held (finalize's reservation).
    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether no rows landed (clippy's `len` companion).
    #[must_use]
    #[expect(
        dead_code,
        reason = "the companion API documents and preserves the type contract"
    )]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The measure poison: the first ray a projected measure reached —
    /// the execution's answer is the typed
    /// [`crate::Error::MeasureOfRay`], checked after the rule loop.
    #[must_use]
    pub fn measure_of_ray(&self) -> Option<[u64; 2]> {
        self.ray
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
        self.ray = None;
    }
}
