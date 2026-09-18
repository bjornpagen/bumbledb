//! Matched physical stores for the same essential-coordinate normal form.
//! Slab fingerprints choose collision chains; full content establishes identity.
use super::Ref;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) enum Body {
    Constant,
    Table(Vec<u64>),
    Branch { variable: u32, low: Ref, high: Ref },
}
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct Node {
    pub variables: u64,
    pub body: Body,
}
pub(super) enum BorrowedBody<'a> {
    Constant,
    Table(&'a [u64]),
    Branch { variable: u32, low: Ref, high: Ref },
}
pub(super) struct BorrowedNode<'a> {
    pub variables: u64,
    pub body: BorrowedBody<'a>,
}

const TABLE: u64 = 1 << 63;
const CHILD_MASK: u64 = (1 << 28) - 1;
const END: Ref = Ref::MAX;

#[derive(Clone, Copy)]
#[repr(C)]
struct Record {
    variables: u64,
    payload: u64,
}
const _: () = assert!(std::mem::size_of::<Record>() == 16);

#[derive(Clone)]
pub(super) enum Store {
    Enum {
        nodes: Vec<Node>,
        unique: FxHashMap<Node, Ref>,
    },
    Slab(Slab),
}
#[derive(Clone)]
pub(super) struct Slab {
    records: Vec<Record>,
    words: Vec<u64>,
    links: Vec<Ref>,
    index: FxHashMap<u64, Ref>,
    // All bits in normal execution. A deliberately colliding test can narrow it.
    fingerprint_mask: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub record_bytes: usize,
    pub table_bytes: usize,
    pub interner_bytes: usize,
    pub records: usize,
    pub tables: usize,
    pub logical_words: usize,
}
impl Statistics {
    pub fn bytes(self) -> usize {
        self.record_bytes + self.table_bytes + self.interner_bytes
    }
}

impl Store {
    pub fn new(layout: &str) -> Self {
        match layout {
            "enum" => Self::Enum {
                nodes: vec![Node {
                    variables: 0,
                    body: Body::Constant,
                }],
                unique: FxHashMap::default(),
            },
            "slab" => Self::Slab(Slab {
                records: vec![Record {
                    variables: 0,
                    payload: 0,
                }],
                words: vec![],
                links: vec![END],
                index: FxHashMap::default(),
                fingerprint_mask: !0,
            }),
            _ => panic!("unknown essential layout"),
        }
    }
    pub fn layout(&self) -> &'static str {
        match self {
            Self::Enum { .. } => "enum",
            Self::Slab(_) => "slab",
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Enum { nodes, .. } => nodes.len(),
            Self::Slab(s) => s.records.len(),
        }
    }
    pub fn variables(&self, r: Ref) -> u64 {
        match self {
            Self::Enum { nodes, .. } => nodes[(r / 2) as usize].variables,
            Self::Slab(s) => s.records[(r / 2) as usize].variables,
        }
    }
    pub fn is_table(&self, r: Ref) -> bool {
        match self {
            Self::Enum { nodes, .. } => matches!(nodes[(r / 2) as usize].body, Body::Table(_)),
            Self::Slab(s) => s.records[(r / 2) as usize].payload & TABLE != 0,
        }
    }
    pub fn node(&self, r: Ref) -> BorrowedNode<'_> {
        match self {
            Self::Enum { nodes, .. } => {
                let n = &nodes[(r / 2) as usize];
                BorrowedNode {
                    variables: n.variables,
                    body: match &n.body {
                        Body::Constant => BorrowedBody::Constant,
                        Body::Table(words) => BorrowedBody::Table(words),
                        Body::Branch {
                            variable,
                            low,
                            high,
                        } => BorrowedBody::Branch {
                            variable: *variable,
                            low: *low,
                            high: *high,
                        },
                    },
                }
            }
            Self::Slab(s) => {
                let n = s.records[(r / 2) as usize];
                let body = if r < 2 {
                    BorrowedBody::Constant
                } else if n.payload & TABLE != 0 {
                    let offset = (n.payload & !TABLE) as usize;
                    let len = (1usize << n.variables.count_ones()).div_ceil(64);
                    BorrowedBody::Table(&s.words[offset..offset + len])
                } else {
                    BorrowedBody::Branch {
                        variable: ((n.payload >> 56) & 63) as u32,
                        low: (n.payload & CHILD_MASK) as Ref,
                        high: ((n.payload >> 28) & CHILD_MASK) as Ref,
                    }
                };
                BorrowedNode {
                    variables: n.variables,
                    body,
                }
            }
        }
    }
    pub fn owned(&self, r: Ref) -> Node {
        let n = self.node(r);
        Node {
            variables: n.variables,
            body: match n.body {
                BorrowedBody::Constant => Body::Constant,
                BorrowedBody::Table(words) => Body::Table(words.to_vec()),
                BorrowedBody::Branch {
                    variable,
                    low,
                    high,
                } => Body::Branch {
                    variable,
                    low,
                    high,
                },
            },
        }
    }
    fn equal(&self, r: Ref, node: &Node) -> bool {
        let old = self.node(r);
        old.variables == node.variables
            && match (old.body, &node.body) {
                (BorrowedBody::Constant, Body::Constant) => true,
                (BorrowedBody::Table(a), Body::Table(b)) => a == b,
                (
                    BorrowedBody::Branch {
                        variable: av,
                        low: al,
                        high: ah,
                    },
                    Body::Branch {
                        variable: bv,
                        low: bl,
                        high: bh,
                    },
                ) => av == *bv && al == *bl && ah == *bh,
                _ => false,
            }
    }
    fn fingerprint(node: &Node, mask: u64) -> u64 {
        let mut hasher = FxHasher::default();
        node.hash(&mut hasher);
        hasher.finish() & mask
    }
    pub fn find(&self, node: &Node) -> Option<Ref> {
        match self {
            Self::Enum { unique, .. } => unique.get(node).copied(),
            Self::Slab(s) => {
                let hash = Self::fingerprint(node, s.fingerprint_mask);
                let mut at = *s.index.get(&hash)?;
                while at != END {
                    if self.equal(at, node) {
                        return Some(at);
                    }
                    at = s.links[(at / 2) as usize];
                }
                None
            }
        }
    }
    // Caller has established absence and appends the corresponding count once.
    pub fn insert(&mut self, node: Node) -> Ref {
        assert!(self.len() < 2_000_000, "RESOURCE_CAP: essential raw nodes");
        let r = self.len() as Ref * 2;
        match self {
            Self::Enum { nodes, unique } => {
                nodes.push(node.clone());
                unique.insert(node, r);
            }
            Self::Slab(s) => {
                let hash = Self::fingerprint(&node, s.fingerprint_mask);
                let payload = match &node.body {
                    Body::Constant => panic!("constant is the distinguished record"),
                    Body::Table(data) => {
                        assert!((1..=12).contains(&node.variables.count_ones()));
                        assert_eq!(
                            data.len(),
                            (1usize << node.variables.count_ones()).div_ceil(64)
                        );
                        let offset = s.words.len();
                        assert!(
                            offset
                                .checked_add(data.len())
                                .is_some_and(|n| n <= u32::MAX as usize),
                            "RESOURCE_CAP: essential table slab"
                        );
                        s.words.extend_from_slice(data);
                        TABLE | offset as u64
                    }
                    Body::Branch {
                        variable,
                        low,
                        high,
                    } => {
                        assert!(
                            *variable < 63
                                && (*low as u64) <= CHILD_MASK
                                && (*high as u64) <= CHILD_MASK
                        );
                        assert!(*low / 2 < r / 2 && *high / 2 < r / 2);
                        (*variable as u64) << 56 | (*high as u64) << 28 | *low as u64
                    }
                };
                let prior = s.index.insert(hash, r).unwrap_or(END);
                s.records.push(Record {
                    variables: node.variables,
                    payload,
                });
                s.links.push(prior);
            }
        }
        r
    }
    pub fn statistics(&self) -> Statistics {
        let mut out = Statistics {
            records: self.len(),
            ..Statistics::default()
        };
        for i in 0..self.len() {
            if let BorrowedBody::Table(words) = self.node(i as Ref * 2).body {
                out.tables += 1;
                out.logical_words += words.len();
            }
        }
        match self {
            Self::Enum { nodes, unique } => {
                out.record_bytes = nodes.capacity() * std::mem::size_of::<Node>();
                out.interner_bytes = unique.capacity() * (std::mem::size_of::<(Node, Ref)>() + 1);
                out.table_bytes = nodes
                    .iter()
                    .chain(unique.keys())
                    .map(|n| match &n.body {
                        Body::Table(words) => words.capacity() * 8,
                        _ => 0,
                    })
                    .sum();
            }
            Self::Slab(s) => {
                out.record_bytes = s.records.capacity() * std::mem::size_of::<Record>();
                out.table_bytes = s.words.capacity() * 8;
                out.interner_bytes = s.links.capacity() * std::mem::size_of::<Ref>()
                    + s.index.capacity() * (std::mem::size_of::<(u64, Ref)>() + 1);
            }
        }
        out
    }
    #[cfg(test)]
    pub fn collide_all(&mut self) {
        let Self::Slab(s) = self else {
            panic!("collision test requires slab store")
        };
        assert_eq!(s.records.len(), 1);
        s.fingerprint_mask = 0;
    }
}
