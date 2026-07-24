use super::{Colt, Cursor, NodeRef, NodeState, Positions, SelectionLevel, View};

impl Colt {
    /// Builds the root over a view: O(1) — nothing decodes until a force.
    /// `selections` are the occurrence's Eq-constant selection levels, in
    /// plan order (image columns plus set-ness — [`SelectionLevel`]);
    /// `join_schema` the join levels below them.
    #[must_use]
    pub fn new(view: View, selections: &[SelectionLevel], join_schema: Vec<Vec<usize>>) -> Self {
        let selection_levels = selections.len();
        let schema_columns: Vec<Vec<usize>> = selections
            .iter()
            .map(|level| level.columns.clone())
            .chain(join_schema)
            .collect();
        Self {
            view,
            selection_levels,
            set_levels: selections.iter().map(|level| level.set).collect(),
            union_mark: None,
            select_hits: Vec::new(),
            select_positions: Vec::new(),
            start: Cursor::Node(NodeRef(0)),
            selected: selection_levels == 0,
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
        }
    }

    /// A structurally identical trie with empty pools over no view — the
    /// shape without the data (reader: the view memo's first park of an
    /// empty slot, inside the sanctioned view-rebuild window).
    #[must_use]
    pub fn unbound_sibling(&self) -> Self {
        Self {
            view: View::Unbound,
            selection_levels: self.selection_levels,
            set_levels: self.set_levels.clone(),
            union_mark: None,
            select_hits: Vec::new(),
            select_positions: Vec::new(),
            start: Cursor::Node(NodeRef(0)),
            selected: self.selection_levels == 0,
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
        }
    }

    /// Swaps in a fresh view for the next execution, clearing every pool
    /// while retaining capacity (post-warmup executions of same-shaped
    /// data allocate nothing here). Returns the old view so its survivor
    /// buffer can be recycled.
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
        self.start = Cursor::Node(NodeRef(0));
        self.selected = self.selection_levels == 0;
        // The epoch advance is what refuses cross-reset resume tokens
        // (the mint sites stamp it into bits 56-62; presentation
        // asserts equality). 7 bits, wrapping.
        self.epoch = (self.epoch + 1) % 128;
        old
    }
}
