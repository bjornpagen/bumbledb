use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{
    FindSpec, ProjectionSink, ProjectionSources, SinkSpec, extend_sources, sources_of,
};
use crate::exec::wordmap::WordMap;

impl ProjectionSink {
    /// `sources`: the projected word sources in find-**word** order (see
    /// [`crate::exec::sink::sources_of`]). (Tests; production sinks are
    /// hint-sized.)
    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        Self::with_capacity_hint_sources(ProjectionSources::Plain(slots), 0)
    }

    /// Presized construction: `hint` is the plan's
    /// output-cardinality estimate — the seen-set allocates once instead
    /// of rehash-doubling through the first measured execution.
    #[must_use]
    fn with_capacity_hint_sources(sources: ProjectionSources, hint: usize) -> Self {
        let arity = match &sources {
            ProjectionSources::Plain(slots) => slots.len(),
            ProjectionSources::Measured(sources) => sources.len(),
        };
        Self {
            finds: Vec::new(),
            measures: Vec::new(),
            sources,
            ray: None,
            measured_sources: Vec::new(),
            seen: WordMap::with_capacity_hint(arity, hint),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    /// Parses prepare's find vocabulary once, then projects the parsed
    /// `Var` specs through the projection sink's word-source vocabulary.
    #[must_use]
    pub fn with_capacity_hint(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        let (parsed, measures) = parse_finds(finds, slot_count);
        let sources = sources_of(&parsed, &measures);
        let mut sink = Self::with_capacity_hint_sources(sources, hint);
        sink.finds = parsed;
        sink.measures = measures;
        sink
    }

    /// Re-aims the projected sources at one rule's binding layout (the
    /// rule loop, docs/architecture/40-execution.md): the head's word
    /// arity is fixed — types and widths are the head's — but each rule
    /// supplies its own slots. The seen-set is untouched: its keys are
    /// projected (head-shaped) tuples, rule-independent by construction,
    /// and its spanning rules IS the union. Single-rule sinks are built
    /// aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        parse_finds_into(finds, slot_count, &mut self.finds, &mut self.measures);
        match (&mut self.sources, self.measures.is_empty()) {
            (ProjectionSources::Plain(slots), true) => {
                slots.clear();
                for find in &self.finds {
                    if let SinkSpec::Var { slot, width } = find {
                        slots.extend(*slot..slot + width);
                    }
                }
            }
            (ProjectionSources::Measured(sources), false) => {
                extend_sources(&self.finds, &self.measures, sources);
            }
            // Head alignment makes a regime change impossible in valid
            // multi-rule programs. Stay total defensively; this cold
            // replacement may allocate only for malformed internal data.
            (sources, _) => *sources = sources_of(&self.finds, &self.measures),
        }
        debug_assert_eq!(
            match &self.sources {
                ProjectionSources::Plain(slots) => slots.len(),
                ProjectionSources::Measured(sources) => sources.len(),
            },
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
