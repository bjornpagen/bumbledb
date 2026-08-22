use crate::exec::sink::aggregate::{parse_finds, parse_finds_into};
use crate::exec::sink::{FindSpec, ProjectionSink, ProjectionSources, SinkSpec, sources_of};
use crate::exec::wordmap::WordMap;

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
            seen: WordMap::with_capacity_hint(arity, hint),
            scratch: vec![0; arity],
            batch_sources: vec![crate::exec::run::LeafSource::Outer; arity],
            scan_rows: Vec::new(),
            scan_count: 0,
        }
    }

    #[must_use]
    pub fn with_capacity_hint(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        let (parsed, measures) = parse_finds(finds, slot_count);
        let sources = sources_of(&parsed, &measures);
        let mut sink = Self::with_capacity_hint_sources(sources, hint);
        sink.finds = parsed;
        sink
    }

    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        let mut measures = Vec::new();
        parse_finds_into(finds, slot_count, &mut self.finds, &mut measures);
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

    pub fn answers(&self) -> impl Iterator<Item = &[u64]> {
        self.seen.iter().map(|(key, ())| key)
    }

    pub fn answers_since(&self, watermark: usize) -> impl Iterator<Item = &[u64]> {
        self.seen.iter_since(watermark).map(|(key, ())| key)
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
