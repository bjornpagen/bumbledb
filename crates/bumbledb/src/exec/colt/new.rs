use super::{BoundView, Colt, NodeState, Positions, SelectionLevel, View};

impl Colt {
    pub(super) fn bound_view(&self) -> &BoundView {
        self.view.bound().expect("execute binds the COLT view")
    }

    /// Whether the bound input has any rows, before prefix selection.
    /// Unbound is a caller error, not an empty relation.
    pub(crate) fn is_empty(&self) -> bool {
        self.bound_view().len() == 0
    }

    #[must_use]
    pub fn new(view: View, selections: &[SelectionLevel], join_schema: Vec<Vec<usize>>) -> Self {
        let schema_columns: Vec<Vec<usize>> = selections
            .iter()
            .map(|level| level.columns().to_vec())
            .chain(join_schema)
            .collect();
        Self {
            view,
            selection_kinds: selections.iter().map(SelectionLevel::kind).collect(),
            union_mark: None,
            select_hits: Vec::new(),
            select_positions: Vec::new(),
            start: Self::initial_start(selections.len()),
            schema_columns,
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            chunk_positions: Vec::new(),
            first_chunk_cap: u8::try_from(super::FIRST_CHUNK_CAP).expect("fits u8"),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
            scratch: Vec::new(),
            stage_keys: Vec::new(),
            stage_positions: Vec::new(),
            epoch: 0,
            work: None,
        }
    }

    #[must_use]
    pub fn unbound_sibling(&self) -> Self {
        Self {
            view: View::Unbound,
            selection_kinds: self.selection_kinds.clone(),
            union_mark: None,
            select_hits: Vec::new(),
            select_positions: Vec::new(),
            start: Self::initial_start(self.selection_depth()),
            schema_columns: self.schema_columns.clone(),
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            chunk_positions: Vec::new(),
            first_chunk_cap: u8::try_from(super::FIRST_CHUNK_CAP).expect("fits u8"),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
            scratch: Vec::new(),
            stage_keys: Vec::new(),
            stage_positions: Vec::new(),
            epoch: 0,
            work: self.work.clone(),
        }
    }

    /// Move cached view/trie contents without moving either slot's current
    /// operation binding. Pool storage moves with its owning trie.
    pub(crate) fn swap_contents_preserving_work(&mut self, other: &mut Self) {
        std::mem::swap(self, other);
        std::mem::swap(&mut self.work, &mut other.work);
    }

    pub fn reset(&mut self, view: View) -> View {
        let old = std::mem::replace(&mut self.view, view);
        self.nodes.clear();
        self.nodes.push(NodeState::Unforced(Positions::Root));
        self.chunks.clear();
        self.chunk_positions.clear();
        self.maps.clear();
        self.ctrl.clear();
        self.buckets.clear();
        self.dense.clear();
        self.union_mark = None;
        self.start = Self::initial_start(self.selection_depth());

        self.epoch = (self.epoch + 1) % 128;
        old
    }

    /// Drop execution storage without rebuilding the compiled trie shape.
    /// The next bind calls `reset`, restoring the root and a fresh epoch.
    pub(crate) fn release_memory(&mut self) {
        self.view = View::Unbound;
        self.union_mark = None;
        self.select_hits = Vec::new();
        self.select_positions = Vec::new();
        self.nodes = Vec::new();
        self.chunks = Vec::new();
        self.chunk_positions = Vec::new();
        self.maps = Vec::new();
        self.ctrl = Vec::new();
        self.buckets = Vec::new();
        self.dense = Vec::new();
        self.scratch = Vec::new();
        self.stage_keys = Vec::new();
        self.stage_positions = Vec::new();
        self.work = None;
        self.start = Self::initial_start(self.selection_depth());
        self.epoch = (self.epoch + 1) % 128;
    }
}
