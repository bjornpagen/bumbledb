//! Safe inspection of completed functions and original support. A snapshot
//! copies only reachable records, preserving sharing, local tables and polarity.
//! Its borrowed views hold no arena locks and cannot forge arena references.
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::arena::{Arena, Operation, Ref, View};
use crate::{Capacity, Control, Error, Event, Result, Space, SpaceId};

#[derive(Debug)]
enum Kind {
    Table { start: usize },
    Split { coordinate: u8, low: Ref, high: Ref },
}

#[derive(Debug)]
struct Record {
    coordinates: u64,
    kind: Kind,
}

#[derive(Debug)]
struct Graph {
    identity: SpaceId,
    measurement: Option<Arc<[u8]>>,
    dimensions: u8,
    order: Vec<u8>,
    support: Ref,
    root: Ref,
    records: Vec<Record>,
    words: Vec<u64>,
}

/// Immutable, owned inspection snapshot of an Event and its original support.
/// It contains only reachable nodes, including shared nodes once. Node identity
/// is local to this snapshot; working order/decoder choices remain observable.
/// Use Event's canonical BEVT encoding for persistence or cross-owner identity.
///
/// ```
/// use bumbledb_event::{BoolOp4, DiagramView, Error, Space, SpaceId};
/// let raw = Space::new(SpaceId([1; 32]), 2, &())?;
/// let legal = raw.table(3, &[0b1001], &())?; // only 00 and 11
/// let space = raw.restrict(&legal, &())?;
/// let event = space.coordinate(0, &())?;
/// let graph = event.diagram(&())?;
/// assert!(!graph.contains(0)?);
/// assert!(graph.contains(3)?);
/// assert_eq!(graph.contains(1), Err(Error::IllegalWorld(1)));
/// let view = graph.root().view();
/// // A borrowed view holds no manager lock: same-space algebra is safe here.
/// assert!(event.apply(BoolOp4::OR, &event.complement(), &())?.is_full());
/// assert!(matches!(view, DiagramView::Table { .. }));
/// assert_eq!(graph.rebuild(&space, &())?, event);
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Diagram(Arc<Graph>);

/// A checked borrowed reference into one inspection snapshot. Equality/hash
/// identify a raw signed node within that snapshot, for traversal memoization.
/// They are not Event equality or persistent identity. Children are raw Boolean
/// functions; a true leaf alone does not establish a legal-world witness.
#[derive(Clone, Copy)]
pub struct DiagramNode<'a> {
    graph: &'a Graph,
    root: Ref,
}

/// Read-only node view. Split children already include the parent's polarity.
/// For a table, truth bit i is `words[i / 64] >> (i % 64) & 1`, XOR
/// `complemented`; i assigns the ascending set bits of `coordinates`.
/// Padding is zero before complement and is never a truth assignment.
#[derive(Debug, Clone, Copy)]
pub enum DiagramView<'a> {
    Constant(bool),
    Table {
        coordinates: u64,
        words: &'a [u64],
        complemented: bool,
    },
    Split {
        coordinate: u8,
        low: DiagramNode<'a>,
        high: DiagramNode<'a>,
    },
}

impl Diagram {
    #[must_use]
    pub fn identity(&self) -> SpaceId {
        self.0.identity
    }

    #[must_use]
    pub fn dimensions(&self) -> u8 {
        self.0.dimensions
    }

    /// Resident split order at capture time; table coordinates use semantic order.
    #[must_use]
    pub fn order(&self) -> &[u8] {
        &self.0.order
    }

    /// Completed membership function. Gate with `support()` for legal witnesses,
    /// quantification, counts or measurements. Decoder aliases have no mass.
    #[must_use]
    pub fn root(&self) -> DiagramNode<'_> {
        self.0.node(self.0.root)
    }

    /// Original legal-code predicate, before decoder completion.
    #[must_use]
    pub fn support(&self) -> DiagramNode<'_> {
        self.0.node(self.0.support)
    }

    /// Number of captured nonconstant records, counting a shared node once.
    #[must_use]
    pub fn records(&self) -> usize {
        self.0.records.len()
    }

    #[must_use]
    pub fn table_words(&self) -> usize {
        self.0.words.len()
    }

    /// Test membership without acquiring a manager lock.
    /// # Errors
    /// Refuses codes outside the original legal support or coordinate extent.
    pub fn contains(&self, world: u64) -> Result<bool> {
        let mask = (1u64 << self.dimensions()) - 1;
        if world & !mask != 0 || !self.support().evaluate(world) {
            return Err(Error::IllegalWorld(world));
        }
        Ok(self.root().evaluate(world))
    }

    /// Reconstruct in an independently owned presentation of the same named
    /// legal space. The target's physical order and decoder may differ.
    /// Original support is checked even for constant Events; a raw node cannot
    /// bypass this admission or be published as an Event by reference alone.
    /// # Errors
    /// Refuses unequal identities, dimensions or support, cancellation or capacity.
    pub fn rebuild(&self, target: &Space, control: &dyn Control) -> Result<Event> {
        if self.identity() != target.identity()
            || self.dimensions() != target.dimensions()
            || self.0.measurement.as_deref() != target.measurement_bytes()
        {
            return Err(Error::SpaceMismatch);
        }
        control.checkpoint()?;
        let mut arena = target.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let mut memo = HashMap::new();
        let support = self.0.rebuild(self.0.support, &mut op, &mut memo)?;
        if support != target.0.support {
            return Err(Error::SpaceMismatch);
        }
        let root = self.0.rebuild(self.0.root, &mut op, &mut memo)?;
        let root = target.0.complete(&mut op, root)?;
        control.checkpoint()?;
        Ok(target.event(root))
    }
}

impl Graph {
    fn node(&self, root: Ref) -> DiagramNode<'_> {
        DiagramNode { graph: self, root }
    }

    fn rebuild(
        &self,
        root: Ref,
        op: &mut Operation<'_>,
        memo: &mut HashMap<Ref, Ref>,
    ) -> Result<Ref> {
        op.step()?;
        if root < 2 {
            return Ok(root);
        }
        let regular = root & !1;
        if let Some(&out) = memo.get(&regular) {
            return Ok(out ^ (root & 1));
        }
        let out = match self.node(regular).view() {
            DiagramView::Constant(_) => unreachable!("nonconstant snapshot node"),
            DiagramView::Table {
                coordinates, words, ..
            } => op.import_table(coordinates, words)?,
            DiagramView::Split {
                coordinate,
                low,
                high,
            } => {
                let low = self.rebuild(low.root, op, memo)?;
                let high = self.rebuild(high.root, op, memo)?;
                let bit = op.variable(coordinate)?;
                op.ite(bit, high, low)?
            }
        };
        if memo.len() >= op.limits().memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        memo.try_reserve(1)?;
        memo.insert(regular, out);
        Ok(out ^ (root & 1))
    }
}

impl<'a> DiagramNode<'a> {
    /// Exact raw essential-coordinate mask. This is a property of the completed
    /// function in this representation, not a logical membership FD on support.
    #[must_use]
    pub fn coordinates(self) -> u64 {
        if self.root < 2 {
            0
        } else {
            self.graph.records[(self.root / 2 - 1) as usize].coordinates
        }
    }

    #[must_use]
    pub fn complement(self) -> Self {
        self.graph.node(self.root ^ 1)
    }

    /// Polarity-free node for memoization; `is_complemented` restores the sign.
    #[must_use]
    pub fn regular(self) -> Self {
        self.graph.node(self.root & !1)
    }

    #[must_use]
    pub fn is_complemented(self) -> bool {
        self.root & 1 != 0
    }

    #[must_use]
    pub fn view(self) -> DiagramView<'a> {
        if self.root < 2 {
            return DiagramView::Constant(self.root != 0);
        }
        let record = &self.graph.records[(self.root / 2 - 1) as usize];
        match record.kind {
            Kind::Table { start } => {
                let len = (1usize << record.coordinates.count_ones()).div_ceil(64);
                DiagramView::Table {
                    coordinates: record.coordinates,
                    words: &self.graph.words[start..start + len],
                    complemented: self.is_complemented(),
                }
            }
            Kind::Split {
                coordinate,
                low,
                high,
            } => DiagramView::Split {
                coordinate,
                low: self.graph.node(low ^ (self.root & 1)),
                high: self.graph.node(high ^ (self.root & 1)),
            },
        }
    }

    fn evaluate(self, world: u64) -> bool {
        match self.view() {
            DiagramView::Constant(value) => value,
            DiagramView::Table {
                coordinates,
                words,
                complemented,
            } => {
                let mut index = 0;
                let mut position = 0;
                for coordinate in 0..62 {
                    if coordinates & (1 << coordinate) != 0 {
                        index |= usize::from(world & (1 << coordinate) != 0) << position;
                        position += 1;
                    }
                }
                (words[index / 64] >> (index % 64) & 1 != 0) ^ complemented
            }
            DiagramView::Split {
                coordinate,
                low,
                high,
            } => {
                if world & (1 << coordinate) == 0 {
                    low.evaluate(world)
                } else {
                    high.evaluate(world)
                }
            }
        }
    }
}

impl std::fmt::Debug for DiagramNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiagramNode")
            .field("coordinates", &self.coordinates())
            .field("complemented", &self.is_complemented())
            .finish_non_exhaustive()
    }
}

impl PartialEq for DiagramNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.graph, other.graph) && self.root == other.root
    }
}
impl Eq for DiagramNode<'_> {}
impl Hash for DiagramNode<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.graph).hash(state);
        self.root.hash(state);
    }
}

struct Capture<'a> {
    op: Operation<'a>,
    graph: Graph,
    translated: HashMap<Ref, Ref>,
}

impl Capture<'_> {
    fn root(&mut self, root: Ref) -> Result<Ref> {
        self.op.step()?;
        if root < 2 {
            return Ok(root);
        }
        let regular = root & !1;
        if let Some(&out) = self.translated.get(&regular) {
            return Ok(out ^ (root & 1));
        }
        let limits = self.op.limits();
        let coordinates = self.op.arena.variables(regular);
        let kind = match self.op.arena.view(regular) {
            View::Constant(_) => unreachable!("nonconstant arena reference"),
            View::Table { words, .. } => {
                let start = self.graph.words.len();
                let end = start
                    .checked_add(words.len())
                    .ok_or(Error::Capacity(Capacity::TableWords))?;
                if end > limits.table_words {
                    return Err(Error::Capacity(Capacity::TableWords));
                }
                self.graph.words.try_reserve(words.len())?;
                self.graph.words.extend_from_slice(words);
                Kind::Table { start }
            }
            View::Split {
                variable,
                low,
                high,
            } => Kind::Split {
                coordinate: variable,
                low: self.root(low)?,
                high: self.root(high)?,
            },
        };
        if self.graph.records.len() >= limits.records {
            return Err(Error::Capacity(Capacity::Records));
        }
        if self.translated.len() >= limits.memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        let out = u32::try_from(self.graph.records.len() + 1)
            .ok()
            .and_then(|n| n.checked_mul(2))
            .ok_or(Error::Capacity(Capacity::Records))?;
        self.graph.records.try_reserve(1)?;
        self.translated.try_reserve(1)?;
        self.graph.records.push(Record { coordinates, kind });
        self.translated.insert(regular, out);
        Ok(out ^ (root & 1))
    }
}

pub(crate) fn capture(
    identity: SpaceId,
    support: Ref,
    root: Ref,
    measurement: Option<Arc<[u8]>>,
    arena: &mut Arena,
    control: &dyn Control,
) -> Result<Diagram> {
    let mut order = Vec::new();
    order.try_reserve_exact(arena.dimensions())?;
    order.extend_from_slice(arena.order());
    let dimensions =
        u8::try_from(arena.dimensions()).map_err(|_| Error::Capacity(Capacity::Coordinates))?;
    let mut capture = Capture {
        op: Operation::new(arena, control)?,
        graph: Graph {
            identity,
            measurement,
            dimensions,
            order,
            support: 0,
            root: 0,
            records: Vec::new(),
            words: Vec::new(),
        },
        translated: HashMap::new(),
    };
    capture.graph.support = capture.root(support)?;
    capture.graph.root = capture.root(root)?;
    control.checkpoint()?;
    Ok(Diagram(Arc::new(capture.graph)))
}
