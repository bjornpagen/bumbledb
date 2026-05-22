use std::collections::HashMap;
use std::ops::Range;

use smallvec::SmallVec;

use crate::query_image::{FactId, FactRange, FactSetRef};
use crate::{EncodedOwned, EncodedRef, FieldId, IndexSpec, RelationId, RelationImage, Result};

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
    /// Builds a hash trie retaining fact IDs in leaves.
    pub fn build(relation: &RelationImage, spec: IndexSpec) -> Result<Self> {
        Self::build_with_mode(relation, spec, LeafMode::Facts)
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
            facts = relation.fact_count,
            fields = spec.fields.len()
        )
        .entered();
        let mut root = HashNode::Inner(HashMap::new());
        for fact in 0..relation.fact_count {
            let fact = FactId(fact as u32);
            let keys = spec
                .fields
                .iter()
                .map(|field| {
                    relation
                        .encoded(fact, *field)
                        .map(EncodedOwned::from_ref)
                        .ok_or_else(|| crate::Error::internal("missing hash trie field value"))
                })
                .collect::<Result<KeyStack>>()?;
            insert_row(&mut root, &keys, fact, leaf_mode);
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
    /// Store matching fact IDs.
    Facts,
    /// Store only matching fact counts.
    CardinalityOnly,
}

/// Hash trie node.
#[derive(Clone, Debug)]
pub enum HashNode {
    /// Internal hash map from encoded key to next node.
    Inner(HashMap<EncodedOwned, HashNode>),
    /// Fact set leaf.
    Leaf(FactSet),
    /// Count-only leaf for existence-only relations.
    CardinalityOnly(u32),
}

/// Owned fact-id set used by hash trie leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactSet {
    /// No facts.
    Empty,
    /// One fact.
    One(FactId),
    /// Small fact set.
    Small(Vec<FactId>),
    /// Larger fact set.
    Many(Vec<FactId>),
    /// Contiguous fact range.
    Range(FactRange),
}

impl FactSet {
    fn push(&mut self, fact: FactId) {
        match self {
            FactSet::Empty => *self = FactSet::One(fact),
            FactSet::One(existing) => *self = FactSet::Small(vec![*existing, fact]),
            FactSet::Small(facts) if facts.len() < 4 => facts.push(fact),
            FactSet::Small(facts) => {
                let mut many = std::mem::take(facts);
                many.push(fact);
                *self = FactSet::Many(many);
            }
            FactSet::Many(facts) => facts.push(fact),
            FactSet::Range(range) => {
                let mut facts = (range.start.0..range.end.0).map(FactId).collect::<Vec<_>>();
                facts.push(fact);
                *self = FactSet::Many(facts);
            }
        }
    }

    /// Number of facts represented by this fact set.
    pub fn len(&self) -> usize {
        match self {
            FactSet::Empty => 0,
            FactSet::One(_) => 1,
            FactSet::Small(facts) | FactSet::Many(facts) => facts.len(),
            FactSet::Range(range) => range.end.0.saturating_sub(range.start.0) as usize,
        }
    }

    /// True when this set has no facts.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn as_ref(&self) -> FactSetRef<'_> {
        match self {
            FactSet::Empty => FactSetRef::Empty,
            FactSet::One(fact) => FactSetRef::One(*fact),
            FactSet::Small(facts) | FactSet::Many(facts) => FactSetRef::Slice(facts),
            FactSet::Range(range) => FactSetRef::Range(*range),
        }
    }
}

/// Hash trie shape statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HashTrieStats {
    /// Number of trie levels.
    pub depth: usize,
    /// Number of stored facts or cardinality-only entries.
    pub facts: usize,
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
    /// Number of facts under the prefix.
    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize;
    /// Fact IDs under the prefix if this trie stores fact IDs.
    fn facts<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> FactSetRef<'a>;
}

/// Streaming fact iterator interface over hash trie prefixes.
pub trait PrefixRows {
    /// Borrowed fact iterator type tied to the borrowed index.
    type Facts<'a>: Iterator<Item = FactId> + 'a
    where
        Self: 'a;

    /// Returns fact IDs under a prefix without materializing a fact vector.
    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Facts<'a>;
}

impl HashTrieIndex {
    /// Visits fact IDs under any prefix depth for fact-retaining tries.
    pub fn for_each_row(&self, prefix: &[EncodedRef<'_>], mut visit: impl FnMut(FactId) -> bool) {
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
            HashNode::Leaf(facts) => !facts.is_empty(),
            HashNode::CardinalityOnly(count) => *count > 0,
        })
    }

    fn count(&self, prefix: &[EncodedRef<'_>]) -> usize {
        find_node(&self.root, prefix).map_or(0, count_node)
    }

    fn facts<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> FactSetRef<'a> {
        match find_node(&self.root, prefix) {
            Some(HashNode::Leaf(facts)) => facts.as_ref(),
            _ => FactSetRef::Empty,
        }
    }
}

impl PrefixRows for HashTrieIndex {
    type Facts<'a> = PrefixRowIter<'a>;

    fn rows_for_prefix<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Self::Facts<'a> {
        PrefixRowIter::new(find_node(&self.root, prefix))
    }
}

/// Concrete streaming fact iterator for hash prefix traversal.
pub struct PrefixRowIter<'a> {
    stack: NodeStack<'a>,
    current: FactSetIter<'a>,
}

impl<'a> PrefixRowIter<'a> {
    fn new(node: Option<&'a HashNode>) -> Self {
        let mut stack = SmallVec::new();
        if let Some(node) = node {
            stack.push(node);
        }
        Self {
            stack,
            current: FactSetIter::Empty,
        }
    }
}

impl Iterator for PrefixRowIter<'_> {
    type Item = FactId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(fact) = self.current.next() {
                return Some(fact);
            }
            let node = self.stack.pop()?;
            match node {
                HashNode::Inner(map) => self.stack.extend(map.values()),
                HashNode::Leaf(facts) => self.current = FactSetIter::from(facts),
                HashNode::CardinalityOnly(_) => {}
            }
        }
    }
}

enum FactSetIter<'a> {
    Empty,
    One(Option<FactId>),
    Slice(std::slice::Iter<'a, FactId>),
    Range(Range<u32>),
}

impl<'a> From<&'a FactSet> for FactSetIter<'a> {
    fn from(facts: &'a FactSet) -> Self {
        match facts {
            FactSet::Empty => FactSetIter::Empty,
            FactSet::One(fact) => FactSetIter::One(Some(*fact)),
            FactSet::Small(facts) | FactSet::Many(facts) => FactSetIter::Slice(facts.iter()),
            FactSet::Range(range) => FactSetIter::Range(range.start.0..range.end.0),
        }
    }
}

impl Iterator for FactSetIter<'_> {
    type Item = FactId;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FactSetIter::Empty => None,
            FactSetIter::One(fact) => fact.take(),
            FactSetIter::Slice(facts) => facts.next().copied(),
            FactSetIter::Range(facts) => facts.next().map(FactId),
        }
    }
}

fn insert_row(node: &mut HashNode, keys: &[EncodedOwned], fact: FactId, leaf_mode: LeafMode) {
    if keys.is_empty() {
        match leaf_mode {
            LeafMode::Facts => match node {
                HashNode::Leaf(facts) => facts.push(fact),
                _ => *node = HashNode::Leaf(FactSet::One(fact)),
            },
            LeafMode::CardinalityOnly => match node {
                HashNode::CardinalityOnly(count) => *count += 1,
                _ => *node = HashNode::CardinalityOnly(1),
            },
        }
        return;
    }
    match node {
        HashNode::Inner(map) => {
            let child = map
                .entry(keys[0].clone())
                .or_insert_with(|| HashNode::Inner(HashMap::new()));
            insert_row(child, &keys[1..], fact, leaf_mode);
        }
        HashNode::Leaf(_) | HashNode::CardinalityOnly(_) => {
            *node = HashNode::Inner(HashMap::new());
            insert_row(node, keys, fact, leaf_mode);
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
        HashNode::Leaf(facts) => facts.len(),
        HashNode::CardinalityOnly(count) => *count as usize,
    }
}

fn visit_rows(node: &HashNode, visit: &mut impl FnMut(FactId) -> bool) -> bool {
    match node {
        HashNode::Inner(map) => {
            for child in map.values() {
                if !visit_rows(child, visit) {
                    return false;
                }
            }
            true
        }
        HashNode::Leaf(facts) => match facts {
            FactSet::Empty => true,
            FactSet::One(fact) => visit(*fact),
            FactSet::Small(facts) | FactSet::Many(facts) => {
                for fact in facts {
                    if !visit(*fact) {
                        return false;
                    }
                }
                true
            }
            FactSet::Range(range) => {
                for fact in (range.start.0..range.end.0).map(FactId) {
                    if !visit(fact) {
                        return false;
                    }
                }
                true
            }
        },
        HashNode::CardinalityOnly(_) => true,
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
        HashNode::Leaf(facts) => {
            stats.leaves += 1;
            stats.facts += facts.len();
        }
        HashNode::CardinalityOnly(count) => {
            stats.leaves += 1;
            stats.facts += *count as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

    use super::*;
    use crate::{Environment, Fact, StorageSchema, Value};

    #[test]
    fn builds_hash_trie_over_primary_key() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build(account, IndexSpec::new("covering", [FieldId(0)]))?;
        let key = EncodedOwned::Eight(2u64.to_be_bytes());

        assert!(index.exists(&[key.as_ref()]));
        assert_eq!(index.count(&[key.as_ref()]), 1);
        assert_eq!(index.facts(&[key.as_ref()]), FactSetRef::One(FactId(1)));
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
            index.facts(&[key.as_ref()]),
            FactSetRef::Slice(&[FactId(0), FactId(2)])
        );
        Ok(())
    }

    #[test]
    fn cardinality_only_hash_trie_stores_counts_without_rows() -> Result<()> {
        let image = account_image()?;
        let account = account_relation(&image)?;
        let index = HashTrieIndex::build_with_mode(
            account,
            IndexSpec::new("exists_currency", [FieldId(1)]),
            LeafMode::CardinalityOnly,
        )?;
        let key = EncodedOwned::One([1]);

        assert!(index.exists(&[key.as_ref()]));
        assert_eq!(index.count(&[key.as_ref()]), 2);
        assert_eq!(index.facts(&[key.as_ref()]), FactSetRef::Empty);
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
            index.facts(&[currency.as_ref(), active.as_ref()]),
            FactSetRef::Slice(&[FactId(0), FactId(2)])
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
            [FactId(1)]
        );

        let by_currency =
            HashTrieIndex::build(account, IndexSpec::new("by_currency", [FieldId(1)]))?;
        let currency = EncodedOwned::One([1]);
        assert_eq!(
            by_currency
                .rows_for_prefix(&[currency.as_ref()])
                .collect::<Vec<_>>(),
            [FactId(0), FactId(2)]
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
            [FactId(0), FactId(2)]
        );
        assert_eq!(
            nested
                .rows_for_prefix(&[currency.as_ref(), active.as_ref()])
                .collect::<Vec<_>>(),
            [FactId(0), FactId(2)]
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
            for fact in account_rows() {
                txn.insert(&schema, fact)?;
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
                            ValueType::Serial {
                                type_name: "AccountId".to_owned(),
                                owning_relation: "Account".to_owned(),
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
                .with_unique("id", ["id"]),
            ],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [1, 2],
        ))
    }

    fn account_rows() -> Vec<Fact> {
        vec![
            Fact::new(
                "Account",
                [
                    ("id", Value::Serial(1)),
                    ("currency", Value::Enum(1)),
                    ("active", Value::Bool(true)),
                ],
            ),
            Fact::new(
                "Account",
                [
                    ("id", Value::Serial(2)),
                    ("currency", Value::Enum(2)),
                    ("active", Value::Bool(false)),
                ],
            ),
            Fact::new(
                "Account",
                [
                    ("id", Value::Serial(3)),
                    ("currency", Value::Enum(1)),
                    ("active", Value::Bool(true)),
                ],
            ),
        ]
    }
}
