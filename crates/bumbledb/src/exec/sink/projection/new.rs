use crate::exec::sink::{FindSpec, ProjectionSink};
use crate::exec::wordmap::WordMap;

impl ProjectionSink {
    /// `slots`: the projected binding slots in find-**word** order — an
    /// interval find contributes both its consecutive slots (the
    /// `SlotWidth` layout; callers expand widths through the plan's layout
    /// map). (Tests; production sinks are hint-sized.)
    #[cfg(test)]
    #[must_use]
    pub fn new(slots: Vec<usize>) -> Self {
        Self::with_capacity_hint(slots, 0)
    }

    /// Presized construction: `hint` is the plan's
    /// output-cardinality estimate — the seen-set allocates once instead
    /// of rehash-doubling through the first measured execution.
    #[must_use]
    pub fn with_capacity_hint(slots: Vec<usize>, hint: usize) -> Self {
        let arity = slots.len();
        Self {
            slots,
            seen: WordMap::with_capacity_hint(arity, hint),
            scratch: vec![0; arity],
            batch_sources: vec![None; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    /// Re-aims the projected slots at one rule's binding layout (the
    /// rule loop, docs/architecture/40-execution.md): the head's word
    /// arity is fixed — types and widths are the head's — but each rule
    /// supplies its own slots. The seen-set is untouched: its keys are
    /// projected (head-shaped) tuples, rule-independent by construction,
    /// and its spanning rules IS the union. Single-rule sinks are built
    /// aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec]) {
        self.slots.clear();
        self.slots.extend(finds.iter().flat_map(|spec| match spec {
            FindSpec::Var { slot, width } => *slot..slot + width,
            FindSpec::Agg { .. } | FindSpec::Arg { .. } => {
                unreachable!("projection sinks project plain variables")
            }
        }));
        debug_assert_eq!(
            self.slots.len(),
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
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.seen.len() == 0
    }

    /// Empties the sink for the next execution, retaining capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
    }
}
