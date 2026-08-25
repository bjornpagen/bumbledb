//! Braid derivation: connected components of the statement graph —
//! ordinary relations as nodes, an edge wherever a containment or
//! capacity statement relates two of them, functionality statements as
//! self-loops, closed relations and closed-target statements
//! contributing nothing. Statements never span components, so braids
//! never conflict (L9); a pure function of the descriptor, implemented
//! twice and pinned by the codec goldens.

use std::collections::BTreeMap;

use bumbledb::schema::{RelationId, SchemaDescriptor, StatementDescriptor, StatementId};

/// A braid's identity: the smallest relation id in its component,
/// rendered as eight lowercase hex digits behind a `c` prefix. Minted
/// only by [`braids`] — a raw u32 enters through [`Braids::parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BraidId(RelationId);

impl BraidId {
    #[must_use]
    pub const fn relation(self) -> RelationId {
        self.0
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0.0
    }

    /// The u32 the wire writes for this braid.
    #[must_use]
    pub(crate) const fn from_raw(raw: u32) -> Self {
        Self(RelationId(raw))
    }
}

impl std::fmt::Display for BraidId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c{:08x}", self.0.0)
    }
}

/// The braid map plus the serial-at-statements: key and capacity
/// statements whose determinant projection is empty name a single
/// global group, so the braid degenerates to a serial log at that
/// statement — returned as typed data for the schema author to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Braids {
    by_relation: Box<[Option<BraidId>]>,
    serial_at: Box<[StatementId]>,
}

impl Braids {
    /// The braid of an ordinary relation; closed and unknown relations
    /// have none.
    ///
    /// # Panics
    #[must_use]
    pub fn braid_of(&self, relation: RelationId) -> Option<BraidId> {
        self.by_relation
            .get(usize::try_from(relation.0).expect("u32 fits usize"))
            .copied()
            .flatten()
    }

    /// Parses a wire u32 into a braid id: valid only when the relation
    /// it names is the smallest in its own component.
    #[must_use]
    pub fn parse(&self, raw: u32) -> Option<BraidId> {
        let relation = RelationId(raw);
        self.braid_of(relation)
            .filter(|braid| braid.relation() == relation)
    }

    /// Statements at which the braid is serial: the empty-determinant
    /// degeneracy, reified as data rather than warned about.
    #[must_use]
    pub const fn serial_at(&self) -> &[StatementId] {
        &self.serial_at
    }

    /// The components, keyed by braid id.
    ///
    /// # Panics
    #[must_use]
    pub fn components(&self) -> BTreeMap<BraidId, Vec<RelationId>> {
        let mut components: BTreeMap<BraidId, Vec<RelationId>> = BTreeMap::new();
        for (index, braid) in self.by_relation.iter().enumerate() {
            if let Some(braid) = braid {
                components.entry(*braid).or_default().push(RelationId(
                    u32::try_from(index).expect("relation count fits u32"),
                ));
            }
        }
        components
    }
}

fn find(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        let (low, high) = if ra < rb { (ra, rb) } else { (rb, ra) };
        parent[high] = low;
    }
}

/// Derives the braid decomposition and the serial-at-statements from
/// the descriptor.
///
/// # Panics
#[must_use]
pub fn braids(descriptor: &SchemaDescriptor) -> Braids {
    let count = descriptor.relations.len();
    let ordinary: Vec<bool> = descriptor
        .relations
        .iter()
        .map(|relation| relation.extension.is_none())
        .collect();
    let mut parent: Vec<usize> = (0..count).collect();
    let mut serial_at: Vec<StatementId> = Vec::new();

    let statements = descriptor.materialized_statements();
    for (index, statement) in statements.iter().enumerate() {
        let id = StatementId(u16::try_from(index).expect("statement count fits u16"));
        match statement {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                let node = usize::try_from(relation.0).expect("u32 fits usize");
                if ordinary.get(node) == Some(&true) && projection.is_empty() {
                    serial_at.push(id);
                }
            }
            StatementDescriptor::Containment { source, target } => {
                let s = usize::try_from(source.relation.0).expect("u32 fits usize");
                let t = usize::try_from(target.relation.0).expect("u32 fits usize");
                if ordinary.get(s) == Some(&true) && ordinary.get(t) == Some(&true) {
                    union(&mut parent, s, t);
                }
            }
            StatementDescriptor::Capacity { target, source, .. } => {
                let s = usize::try_from(source.relation.0).expect("u32 fits usize");
                let t = usize::try_from(target.relation.0).expect("u32 fits usize");
                if ordinary.get(t) == Some(&true) {
                    if ordinary.get(s) == Some(&true) {
                        union(&mut parent, s, t);
                    }
                    if target.projection.is_empty() {
                        serial_at.push(id);
                    }
                }
            }
        }
    }

    let mut smallest: Vec<Option<u32>> = vec![None; count];
    for (node, is_ordinary) in ordinary.iter().enumerate() {
        if *is_ordinary {
            let root = find(&mut parent, node);
            let id = u32::try_from(node).expect("relation count fits u32");
            let slot = &mut smallest[root];
            *slot = Some(slot.map_or(id, |current| current.min(id)));
        }
    }

    let by_relation: Vec<Option<BraidId>> = (0..count)
        .map(|node| {
            if ordinary[node] {
                let root = find(&mut parent, node);
                smallest[root].map(|id| BraidId(RelationId(id)))
            } else {
                None
            }
        })
        .collect();

    Braids {
        by_relation: by_relation.into_boxed_slice(),
        serial_at: serial_at.into_boxed_slice(),
    }
}
