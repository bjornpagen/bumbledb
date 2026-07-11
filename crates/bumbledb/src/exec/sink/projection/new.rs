use crate::exec::sink::{extend_sources, FindSpec, ProjSource, ProjectionSink};
use crate::exec::wordmap::WordMap;

impl ProjectionSink {
    /// `sources`: the projected word sources in find-**word** order (see
    /// [`crate::exec::sink::sources_of`]). (Tests; production sinks are
    /// hint-sized.)
    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        Self::with_capacity_hint(slots.into_iter().map(ProjSource::Slot).collect(), 0, false)
    }

    /// Presized construction: `hint` is the plan's
    /// output-cardinality estimate — the seen-set allocates once instead
    /// of rehash-doubling through the first measured execution.
    /// `disjoint` is the rule-disjointness proof
    /// (docs/architecture/40-execution.md § set semantics): the cross-rule
    /// guard is dropped — [`Self::aim`] drains the map per rule — while
    /// per-rule dedup stays, as the semantics require.
    #[must_use]
    pub fn with_capacity_hint(sources: Vec<ProjSource>, hint: usize, disjoint: bool) -> Self {
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
            disjoint,
            rows: Vec::new(),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    /// Re-aims the projected sources at one rule's binding layout (the
    /// rule loop, docs/architecture/40-execution.md): the head's word
    /// arity is fixed — types and widths are the head's — but each rule
    /// supplies its own slots. Spanning regime: the seen-set is untouched
    /// — its keys are projected (head-shaped) tuples, rule-independent by
    /// construction, and its spanning rules IS the union. Disjoint
    /// regime: the finished rule's tuples drain into [`Self::rows`] and
    /// the map clears — the theorem proved cross-rule collisions
    /// impossible, so the guard the spanning map provided held nothing,
    /// and the next rule dedups only against itself. Single-rule sinks
    /// are built aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec]) {
        if self.disjoint {
            let rows = &mut self.rows;
            for (key, ()) in self.seen.iter() {
                rows.extend_from_slice(key);
            }
            self.seen.clear();
        }
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
    /// host sorts): drained rows of finished disjoint rules, then the
    /// live seen-set (all of it, when spanning).
    pub fn rows(&self) -> impl Iterator<Item = &[u64]> {
        self.rows
            .chunks_exact(self.scratch.len())
            .chain(self.seen.iter().map(|(key, ())| key))
    }

    /// Distinct rows held (finalize's reservation).
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len() / self.scratch.len() + self.seen.len()
    }

    /// Whether no rows landed (clippy's `len` companion).
    #[must_use]
    #[allow(dead_code)]
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

    /// The differential guard's override: back to the spanning regime,
    /// so a covered query runs both ways — the elision is *never*
    /// semantic, and forced-off results must be byte-identical.
    #[cfg(test)]
    pub fn force_spanning(&mut self) {
        debug_assert!(self.rows.is_empty(), "override before the execution");
        self.disjoint = false;
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.rows.clear();
        self.seen.clear();
        self.ray = None;
    }
}
