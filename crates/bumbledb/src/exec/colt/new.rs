use super::{Colt, View, Cursor, NodeRef, NodeState, Positions};

impl Colt {
    /// Builds the root over a view: O(1) — nothing decodes until a force.
    /// `selections` are the image columns of the occurrence's Eq-constant
    /// predicates, in plan order; `join_schema` the join levels below them.
    #[must_use]
    pub fn new(view: View, selections: &[usize], join_schema: Vec<Vec<usize>>) -> Self {
        let selection_levels = selections.len();
        let schema_columns: Vec<Vec<usize>> = selections
            .iter()
            .map(|column| vec![*column])
            .chain(join_schema)
            .collect();
        Self {
            view,
            selection_levels,
            start: Cursor::Node(NodeRef(0)),
            selected: selection_levels == 0,
            schema_columns,
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
            scratch: Vec::new(),
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
            start: Cursor::Node(NodeRef(0)),
            selected: self.selection_levels == 0,
            schema_columns: self.schema_columns.clone(),
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
            scratch: Vec::new(),
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
        self.maps.clear();
        self.ctrl.clear();
        self.buckets.clear();
        self.dense.clear();
        self.start = Cursor::Node(NodeRef(0));
        self.selected = self.selection_levels == 0;
        old
    }
}
