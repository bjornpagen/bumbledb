use crate::query::free_join::{FjCoverCandidate, FjNode, FjSubatom};
use crate::query::model::AtomOccurrenceId;

const MAX_PLAN_NODES: usize = 16;
const MAX_NODE_SUBATOMS: usize = 16;
const MAX_ID_LIST: usize = 4;

#[derive(Clone, Copy, Debug)]
pub(crate) struct IdList {
    len: u8,
    values: [usize; MAX_ID_LIST],
}

impl PartialEq for IdList {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for IdList {}

impl IdList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn push(&mut self, value: usize) {
        if self.len() < self.values.len() {
            self.values[self.len()] = value;
            self.len += 1;
        }
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, usize> {
        self.as_slice().iter()
    }

    pub(crate) fn as_slice(&self) -> &[usize] {
        &self.values[..self.len()]
    }
}

impl Default for IdList {
    fn default() -> Self {
        Self {
            len: 0,
            values: [0; MAX_ID_LIST],
        }
    }
}

impl FromIterator<usize> for IdList {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let mut out = Self::new();
        for value in iter {
            out.push(value);
        }
        out
    }
}

impl<'a> IntoIterator for &'a IdList {
    type Item = &'a usize;
    type IntoIter = std::slice::Iter<'a, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SubatomList {
    len: u8,
    values: [FjSubatom; MAX_NODE_SUBATOMS],
}

impl PartialEq for SubatomList {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for SubatomList {}

impl SubatomList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn push(&mut self, value: FjSubatom) {
        if self.len() < self.values.len() {
            self.values[self.len()] = value;
            self.len += 1;
        }
    }

    pub(crate) fn pop(&mut self) -> Option<FjSubatom> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.values[self.len()])
    }

    pub(crate) fn remove(&mut self, index: usize) -> FjSubatom {
        let removed = self.values[index];
        for cursor in index..self.len() - 1 {
            self.values[cursor] = self.values[cursor + 1];
        }
        self.len -= 1;
        removed
    }

    pub(crate) fn insert(&mut self, index: usize, value: FjSubatom) {
        if self.len() >= self.values.len() {
            return;
        }
        for cursor in (index..self.len()).rev() {
            self.values[cursor + 1] = self.values[cursor];
        }
        self.values[index] = value;
        self.len += 1;
    }

    pub(crate) fn first(&self) -> Option<&FjSubatom> {
        self.as_slice().first()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, FjSubatom> {
        self.as_slice().iter()
    }

    pub(crate) fn as_slice(&self) -> &[FjSubatom] {
        &self.values[..self.len()]
    }
}

impl Default for SubatomList {
    fn default() -> Self {
        Self {
            len: 0,
            values: [FjSubatom::default(); MAX_NODE_SUBATOMS],
        }
    }
}

impl FromIterator<FjSubatom> for SubatomList {
    fn from_iter<T: IntoIterator<Item = FjSubatom>>(iter: T) -> Self {
        let mut out = Self::new();
        for value in iter {
            out.push(value);
        }
        out
    }
}

impl std::ops::Index<usize> for SubatomList {
    type Output = FjSubatom;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a> IntoIterator for &'a SubatomList {
    type Item = &'a FjSubatom;
    type IntoIter = std::slice::Iter<'a, FjSubatom>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeList {
    len: u8,
    values: [FjNode; MAX_PLAN_NODES],
}

impl PartialEq for NodeList {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for NodeList {}

impl NodeList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    pub(crate) fn push(&mut self, value: FjNode) {
        if self.len() < self.values.len() {
            self.values[self.len()] = value;
            self.len += 1;
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&FjNode> {
        self.as_slice().get(index)
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, FjNode> {
        self.as_slice().iter()
    }

    pub(crate) fn as_slice(&self) -> &[FjNode] {
        &self.values[..self.len()]
    }
}

impl Default for NodeList {
    #[allow(clippy::large_stack_arrays)]
    fn default() -> Self {
        Self {
            len: 0,
            values: [FjNode::default(); MAX_PLAN_NODES],
        }
    }
}

impl FromIterator<FjNode> for NodeList {
    fn from_iter<T: IntoIterator<Item = FjNode>>(iter: T) -> Self {
        let mut out = Self::new();
        for value in iter {
            out.push(value);
        }
        out
    }
}

impl std::ops::Index<usize> for NodeList {
    type Output = FjNode;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl std::ops::IndexMut<usize> for NodeList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}

impl Default for FjSubatom {
    fn default() -> Self {
        Self {
            atom: AtomOccurrenceId(0),
            vars: IdList::new(),
            field_ids: IdList::new(),
        }
    }
}

impl Default for FjNode {
    fn default() -> Self {
        Self {
            id: 0,
            subatoms: SubatomList::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IdRange {
    pub(super) start: usize,
    pub(super) len: usize,
}

impl IdRange {
    pub(crate) fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AtomPartitionCount(pub(super) usize);

impl AtomPartitionCount {
    pub(crate) fn len(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CoverList {
    len: u8,
    values: [FjCoverCandidate; MAX_NODE_SUBATOMS],
}

impl CoverList {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn push(&mut self, value: FjCoverCandidate) {
        if self.len() < self.values.len() {
            self.values[self.len()] = value;
            self.len += 1;
        }
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, FjCoverCandidate> {
        self.as_slice().iter()
    }

    pub(crate) fn as_slice(&self) -> &[FjCoverCandidate] {
        &self.values[..self.len()]
    }
}

impl Default for CoverList {
    fn default() -> Self {
        Self {
            len: 0,
            values: [FjCoverCandidate::default(); MAX_NODE_SUBATOMS],
        }
    }
}
