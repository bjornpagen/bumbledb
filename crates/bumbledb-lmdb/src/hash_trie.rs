use std::collections::HashMap;
use std::ops::Range;

use smallvec::SmallVec;

use crate::{
    EncodedOwned, EncodedRef, FieldId, IndexSpec, RelationId, RelationImage, Result, RowId,
    RowRange, RowSetRef,
};

type NodeStack<'a> = SmallVec<[&'a HashNode; 8]>;
type KeyStack = SmallVec<[EncodedOwned; 8]>;

/// In-memory hash trie index over one relation image.
#[derive(Clone, Debug)]
pub struct HashTrieIndex {
    /// Relation this index belongs to.
    pub relation: RelationId,
    /// Index name.
    pub name: String,
    /// Field order for trie levels.
    pub fields: Vec<FieldId>,
    /// Root hash node.
    pub root: HashNode,
    /// Build and shape statistics.
    pub stats: HashTrieStats,
}

impl HashTrieIndex {
    /// Builds a hash trie retaining row IDs in leaves.
    pub fn build(relation: &RelationImage, spec: IndexSpec) -> Result<Self> {
        Self::build_with_mode(relation, spec, LeafMode::Rows)
    }

    /// Builds a hash trie with a specified leaf mode.
    pub fn build_with_mode(
        relation: &RelationImage,
        spec: IndexSpec,
        leaf_mode: LeafMode,
    ) -> Result<Self> {
        let _span = tracing::debug_span!(
            "bumbledb.hash_trie.build",
            relation = relation.id.0,
            rows = relation.row_count,
            fields = spec.fields.len()
        )
        .entered();
        let mut root = HashNode::Inner(HashMap::new());
        for row in 0..relation.row_count {
            let row = RowId(row as u32);
            let keys = spec
                .fields
                .iter()
                .map(|field| {
                    relation
                        .encoded(row, *field)
                        .map(EncodedOwned::from_ref)
                        .ok_or_else(|| crate::Error::internal("missing hash trie field value"))
                })
                .collect::<Result<KeyStack>>()?;
            insert_row(&mut root, &keys, row, leaf_mode);
        }
        let stats = HashTrieStats::from_root(&root, spec.fields.len());
        Ok(Self {
            relation: relation.id,
            name: spec.name,
            fields: spec.fields,
            root,
            stats,
        })
    }
}

/// Leaf storage mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafMode {
    /// Store matching row IDs.
    Rows,
    /// Store only matching row counts.
    CountOnly,
}

/// Hash trie node.
#[derive(Clone, Debug)]
pub enum HashNode {
    /// Internal hash map from encoded key to next node.
    Inner(HashMap<EncodedOwned, HashNode>),
    /// Row set leaf.
    Leaf(RowSet),
    /// Count-only leaf for existence-only relations.
    CountOnly(u32),
}

/// Owned row-id set used by hash trie leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowSet {
    /// No rows.
    Empty,
    /// One row.
    One(RowId),
    /// Small row set.
    Small(Vec<RowId>),
    /// Larger row set.
    Many(Vec<RowId>),
    /// Contiguous row range.
    Range(RowRange),
}

impl RowSet {
    fn push(&mut self, row: RowId) {
        match self {
            RowSet::Empty => *self = RowSet::One(row),
            RowSet::One(existing) => *self = RowSet::Small(vec![*existing, row]),
            RowSet::Small(rows) if rows.len() < 4 => rows.push(row),
            RowSet::Small(rows) => {
                let mut many = std::mem::take(rows);
                many.push(row);
                *self = RowSet::Many(many);
            }
            RowSet::Many(rows) => rows.push(row),
            RowSet::Range(range) => {
                let mut rows = (range.start.0..range.end.0).map(RowId).collect::<Vec<_>>();
                rows.push(row);
                *self = RowSet::Many(rows);
            }
        }
    }

    /// Number of rows represented by this row set.
    pub fn len(&self) -> usize {
        match self {
            RowSet::Empty => 0,
            RowSet::One(_) => 1,
            RowSet::Small(rows) | RowSet::Many(rows) => rows.len(),
            RowSet::Range(range) => range.end.0.saturating_sub(range.start.0) as usize,
        }
    }

    /// True when this set has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn as_ref(&self) -> RowSetRef<'_> {
        match self {
            RowSet::Empty => RowSetRef::Empty,
            RowSet::One(row) => RowSetRef::One(*row),
            RowSet::Small(rows) | RowSet::Many(rows) => RowSetRef::Slice(rows),
            RowSet::Range(range) => RowSetRef::Range(*range),
        }
    }
}

/// Hash trie shape statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HashTrieStats {
    /// Number of trie levels.
    pub depth: usize,
    /// Number of stored rows or count-only entries.
    pub rows: usize,
    /// Number of internal hash nodes.
    pub inner_nodes: usize,
    /// Number of leaves.
    pub leaves: usize,
}

impl HashTrieStats {
    fn from_root(root: &HashNode, depth: usize) -> Self {
        let mut stats = Self {
            depth,
            ..Self::default()
        };
        accumulate_stats(root, &mut stats);
        stats
    }
}

/// Prefix probe interface over hash trie indexes.
pub trait PrefixProbe {
    /// True when the prefix exists.
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> bool;
    /// Number of rows under the prefix.
    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize;
    /// Row IDs under the prefix if this trie stores row IDs.
    fn rows<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> RowSetRef<'a>;
}

/// Streaming row iterator interface over hash trie prefixes.
pub trait PrefixRows {
    /// Borrowed row iterator type tied to the borrowed index.
    type Rows<'a>: Iterator<Item = RowId> + 'a
    where
        Self: 'a;

    /// Returns row IDs under a prefix without materializing a row vector.
    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Rows<'a>;
}

impl HashTrieIndex {
    /// Visits row IDs under any prefix depth for row-retaining tries.
    pub fn for_each_row(&self, prefix: &[EncodedRef<'_>], mut visit: impl FnMut(RowId) -> bool) {
        let Some(node) = find_node(&self.root, prefix) else {
            return;
        };
        visit_rows(node, &mut visit);
    }
}

impl PrefixProbe for HashTrieIndex {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> bool {
        find_node(&self.root, prefix).is_some_and(|node| match node {
            HashNode::Inner(map) => !map.is_empty(),
            HashNode::Leaf(rows) => !rows.is_empty(),
            HashNode::CountOnly(count) => *count > 0,
        })
    }

    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize {
        find_node(&self.root, prefix).map_or(0, count_node)
    }

    fn rows<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> RowSetRef<'a> {
        match find_node(&self.root, prefix) {
            Some(HashNode::Leaf(rows)) => rows.as_ref(),
            _ => RowSetRef::Empty,
        }
    }
}

impl PrefixRows for HashTrieIndex {
    type Rows<'a> = PrefixRowIter<'a>;

    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Rows<'a> {
        PrefixRowIter::new(find_node(&self.root, prefix))
    }
}

/// Concrete streaming row iterator for hash prefix traversal.
pub struct PrefixRowIter<'a> {
    stack: NodeStack<'a>,
    current: RowSetIter<'a>,
}

impl<'a> PrefixRowIter<'a> {
    fn new(node: Option<&'a HashNode>) -> Self {
        let mut stack = SmallVec::new();
        if let Some(node) = node {
            stack.push(node);
        }
        Self {
            stack,
            current: RowSetIter::Empty,
        }
    }
}

impl Iterator for PrefixRowIter<'_> {
    type Item = RowId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.current.next() {
                return Some(row);
            }
            let node = self.stack.pop()?;
            match node {
                HashNode::Inner(map) => self.stack.extend(map.values()),
                HashNode::Leaf(rows) => self.current = RowSetIter::from(rows),
                HashNode::CountOnly(_) => {}
            }
        }
    }
}

enum RowSetIter<'a> {
    Empty,
    One(Option<RowId>),
    Slice(std::slice::Iter<'a, RowId>),
    Range(Range<u32>),
}

impl<'a> From<&'a RowSet> for RowSetIter<'a> {
    fn from(rows: &'a RowSet) -> Self {
        match rows {
            RowSet::Empty => RowSetIter::Empty,
            RowSet::One(row) => RowSetIter::One(Some(*row)),
            RowSet::Small(rows) | RowSet::Many(rows) => RowSetIter::Slice(rows.iter()),
            RowSet::Range(range) => RowSetIter::Range(range.start.0..range.end.0),
        }
    }
}

impl Iterator for RowSetIter<'_> {
    type Item = RowId;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RowSetIter::Empty => None,
            RowSetIter::One(row) => row.take(),
            RowSetIter::Slice(rows) => rows.next().copied(),
            RowSetIter::Range(rows) => rows.next().map(RowId),
        }
    }
}

fn insert_row(node: &mut HashNode, keys: &[EncodedOwned], row: RowId, leaf_mode: LeafMode) {
    if keys.is_empty() {
        match leaf_mode {
            LeafMode::Rows => match node {
                HashNode::Leaf(rows) => rows.push(row),
                _ => *node = HashNode::Leaf(RowSet::One(row)),
            },
            LeafMode::CountOnly => match node {
                HashNode::CountOnly(count) => *count += 1,
                _ => *node = HashNode::CountOnly(1),
            },
        }
        return;
    }
    match node {
        HashNode::Inner(map) => {
            let child = map
                .entry(keys[0].clone())
                .or_insert_with(|| HashNode::Inner(HashMap::new()));
            insert_row(child, &keys[1..], row, leaf_mode);
        }
        HashNode::Leaf(_) | HashNode::CountOnly(_) => {
            *node = HashNode::Inner(HashMap::new());
            insert_row(node, keys, row, leaf_mode);
        }
    }
}

fn find_node<'a>(node: &'a HashNode, prefix: &[EncodedRef<'_>]) -> Option<&'a HashNode> {
    if prefix.is_empty() {
        return Some(node);
    }
    let HashNode::Inner(map) = node else {
        return None;
    };
    let key = EncodedOwned::from_ref(prefix[0]);
    find_node(map.get(&key)?, &prefix[1..])
}

fn count_node(node: &HashNode) -> usize {
    match node {
        HashNode::Inner(map) => map.values().map(count_node).sum(),
        HashNode::Leaf(rows) => rows.len(),
        HashNode::CountOnly(count) => *count as usize,
    }
}

fn visit_rows(node: &HashNode, visit: &mut impl FnMut(RowId) -> bool) -> bool {
    match node {
        HashNode::Inner(map) => {
            for child in map.values() {
                if !visit_rows(child, visit) {
                    return false;
                }
            }
            true
        }
        HashNode::Leaf(rows) => match rows {
            RowSet::Empty => true,
            RowSet::One(row) => visit(*row),
            RowSet::Small(rows) | RowSet::Many(rows) => {
                for row in rows {
                    if !visit(*row) {
                        return false;
                    }
                }
                true
            }
            RowSet::Range(range) => {
                for row in (range.start.0..range.end.0).map(RowId) {
                    if !visit(row) {
                        return false;
                    }
                }
                true
            }
        },
        HashNode::CountOnly(_) => true,
    }
}

fn accumulate_stats(node: &HashNode, stats: &mut HashTrieStats) {
    match node {
        HashNode::Inner(map) => {
            stats.inner_nodes += 1;
            for child in map.values() {
                accumulate_stats(child, stats);
            }
        }
        HashNode::Leaf(rows) => {
            stats.leaves += 1;
            stats.rows += rows.len();
        }
        HashNode::CountOnly(count) => {
            stats.leaves += 1;
            stats.rows += *count as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use bumbledb_core::schema::{
        FieldDescriptor, IdentityAllocation, RelationDescriptor, SchemaDescriptor, ValueType,
    };

    use super::*;
    use crate::{Environment, IdentityValue, Row, StorageSchema, Value};

    #[test]
    fn builds_hash_trie_over_primary_key() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build(account, IndexSpec::new("covering", [FieldId(0)]))?;
        let key = EncodedOwned::Eight(2u64.to_be_bytes());

        assert!(index.exists(&[key.as_ref()]));
        assert_eq!(index.count(&[key.as_ref()]), 1);
        assert_eq!(index.rows(&[key.as_ref()]), RowSetRef::One(RowId(1)));
        Ok(())
    }

    #[test]
    fn builds_hash_trie_over_non_unique_field() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build(account, IndexSpec::new("by_currency", [FieldId(1)]))?;
        let key = EncodedOwned::One([1]);

        assert!(index.exists(&[key.as_ref()]));
        assert_eq!(index.count(&[key.as_ref()]), 2);
        assert_eq!(
            index.rows(&[key.as_ref()]),
            RowSetRef::Slice(&[RowId(0), RowId(2)])
        );
        Ok(())
    }

    #[test]
    fn count_only_hash_trie_stores_counts_without_rows() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build_with_mode(
            account,
            IndexSpec::new("exists_currency", [FieldId(1)]),
            LeafMode::CountOnly,
        )?;
        let key = EncodedOwned::One([1]);

        assert!(index.exists(&[key.as_ref()]));
        assert_eq!(index.count(&[key.as_ref()]), 2);
        assert_eq!(index.rows(&[key.as_ref()]), RowSetRef::Empty);
        Ok(())
    }

    #[test]
    fn two_level_hash_trie_probes_prefixes() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build(
            account,
            IndexSpec::new("currency_active", [FieldId(1), FieldId(2)]),
        )?;
        let currency = EncodedOwned::One([1]);
        let active = EncodedOwned::One([1]);

        assert_eq!(index.count(&[currency.as_ref()]), 2);
        assert_eq!(index.count(&[currency.as_ref(), active.as_ref()]), 2);
        assert_eq!(
            index.rows(&[currency.as_ref(), active.as_ref()]),
            RowSetRef::Slice(&[RowId(0), RowId(2)])
        );
        Ok(())
    }

    #[test]
    fn prefix_rows_streams_empty_one_slice_and_nested_prefixes() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let covering = HashTrieIndex::build(account, IndexSpec::new("covering", [FieldId(0)]))?;
        let missing = EncodedOwned::Eight(99u64.to_be_bytes());
        assert_eq!(
            covering
                .rows_for_prefix(&[missing.as_ref()])
                .collect::<Vec<_>>(),
            []
        );

        let one = EncodedOwned::Eight(2u64.to_be_bytes());
        assert_eq!(
            covering
                .rows_for_prefix(&[one.as_ref()])
                .collect::<Vec<_>>(),
            [RowId(1)]
        );

        let by_currency =
            HashTrieIndex::build(account, IndexSpec::new("by_currency", [FieldId(1)]))?;
        let currency = EncodedOwned::One([1]);
        assert_eq!(
            by_currency
                .rows_for_prefix(&[currency.as_ref()])
                .collect::<Vec<_>>(),
            [RowId(0), RowId(2)]
        );

        let nested = HashTrieIndex::build(
            account,
            IndexSpec::new("currency_active", [FieldId(1), FieldId(2)]),
        )?;
        let active = EncodedOwned::One([1]);
        assert_eq!(
            nested
                .rows_for_prefix(&[currency.as_ref()])
                .collect::<Vec<_>>(),
            [RowId(0), RowId(2)]
        );
        assert_eq!(
            nested
                .rows_for_prefix(&[currency.as_ref(), active.as_ref()])
                .collect::<Vec<_>>(),
            [RowId(0), RowId(2)]
        );
        Ok(())
    }

    fn account_relation(image: &crate::QueryImage) -> Result<&crate::RelationImage> {
        image
            .relation("Account")
            .ok_or_else(|| crate::Error::internal("missing Account relation"))
    }

    fn account_image() -> Result<crate::QueryImage> {
        let dir = tempfile::tempdir().map_err(|error| crate::Error::io("tempdir", error))?;
        let path = dir.keep();
        let env = Environment::open(path)?;
        let schema = StorageSchema::new(account_schema(), env.max_key_size())?;
        env.write(|txn| {
            for row in account_rows() {
                txn.insert(&schema, row)?;
            }
            Ok::<_, crate::Error>(())
        })?;
        Ok(env.query_image(&schema)?.as_ref().clone())
    }

    fn account_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "Accounts",
            vec![
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Identity {
                                type_name: "AccountId".to_owned(),
                                owning_relation: "Account".to_owned(),
                                allocation: IdentityAllocation::Serial,
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("active", ValueType::Bool),
                    ],
                )
                .with_covering_unique("id", ["id"]),
            ],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [1, 2],
        ))
    }

    fn account_rows() -> Vec<Row> {
        vec![
            Row::new(
                "Account",
                [
                    ("id", Value::Identity(IdentityValue::Serial(1))),
                    ("currency", Value::Enum(1)),
                    ("active", Value::Bool(true)),
                ],
            ),
            Row::new(
                "Account",
                [
                    ("id", Value::Identity(IdentityValue::Serial(2))),
                    ("currency", Value::Enum(2)),
                    ("active", Value::Bool(false)),
                ],
            ),
            Row::new(
                "Account",
                [
                    ("id", Value::Identity(IdentityValue::Serial(3))),
                    ("currency", Value::Enum(1)),
                    ("active", Value::Bool(true)),
                ],
            ),
        ]
    }
}
