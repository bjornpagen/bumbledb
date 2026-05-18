use std::time::Instant;

use crate::{EncodedRef, FieldId, RelationId, RelationImage, Result, RowId, RowRange};

/// Owned fixed-width encoded value used in trie levels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedOwned {
    /// One-byte encoded value.
    One([u8; 1]),
    /// Eight-byte encoded value.
    Eight([u8; 8]),
    /// Sixteen-byte encoded value.
    Sixteen([u8; 16]),
}

impl EncodedOwned {
    /// Copies an encoded reference into an owned value.
    pub fn from_ref(value: EncodedRef<'_>) -> Self {
        match value {
            EncodedRef::One(bytes) => EncodedOwned::One(*bytes),
            EncodedRef::Eight(bytes) => EncodedOwned::Eight(*bytes),
            EncodedRef::Sixteen(bytes) => EncodedOwned::Sixteen(*bytes),
        }
    }

    /// Returns this value as encoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            EncodedOwned::One(bytes) => &bytes[..],
            EncodedOwned::Eight(bytes) => &bytes[..],
            EncodedOwned::Sixteen(bytes) => &bytes[..],
        }
    }

    fn as_ref(&self) -> EncodedRef<'_> {
        match self {
            EncodedOwned::One(bytes) => EncodedRef::One(bytes),
            EncodedOwned::Eight(bytes) => EncodedRef::Eight(bytes),
            EncodedOwned::Sixteen(bytes) => EncodedRef::Sixteen(bytes),
        }
    }
}

/// Sorted trie index build specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSpec {
    /// Stable index name.
    pub name: String,
    /// Field order for trie levels.
    pub fields: Vec<FieldId>,
}

impl IndexSpec {
    /// Creates a sorted trie index build specification.
    pub fn new(name: impl Into<String>, fields: impl IntoIterator<Item = FieldId>) -> Self {
        Self {
            name: name.into(),
            fields: fields.into_iter().collect(),
        }
    }
}

/// In-memory sorted trie index over one relation image.
#[derive(Clone, Debug)]
pub struct SortedTrieIndex {
    /// Relation this index belongs to.
    pub relation: RelationId,
    /// Index name.
    pub name: String,
    /// Field order for trie levels.
    pub fields: Vec<FieldId>,
    /// Row IDs sorted by this index's field order.
    pub order: Vec<RowId>,
    /// Distinct-value levels.
    pub levels: Vec<TrieLevel>,
    /// Build and shape statistics.
    pub stats: TrieStats,
}

impl SortedTrieIndex {
    /// Builds a sorted trie index from an immutable relation image.
    pub fn build(relation: &RelationImage, spec: IndexSpec) -> Result<Self> {
        let start = Instant::now();
        let mut order = (0..relation.row_count)
            .map(|row| RowId(row as u32))
            .collect::<Vec<_>>();

        order.sort_by(|left, right| {
            for field in &spec.fields {
                let left = relation.encoded_bytes(*left, *field).unwrap_or(&[]);
                let right = relation.encoded_bytes(*right, *field).unwrap_or(&[]);
                match left.cmp(right) {
                    std::cmp::Ordering::Equal => continue,
                    ordering => return ordering,
                }
            }
            left.cmp(right)
        });

        let levels = build_levels(relation, &order, &spec.fields)?;
        let stats = TrieStats::from_levels(order.len(), &levels, start.elapsed().as_micros());
        Ok(Self {
            relation: relation.id,
            name: spec.name,
            fields: spec.fields,
            order,
            levels,
            stats,
        })
    }

    /// Creates an iterator positioned before the root.
    pub fn iter(&self) -> SortedTrieIter<'_> {
        SortedTrieIter {
            index: self,
            stack: Vec::with_capacity(self.fields.len()),
        }
    }

    fn child_bounds(&self, depth: usize, parent: u32) -> (usize, usize) {
        let parents = &self.levels[depth].parent;
        let begin = parents.partition_point(|value| *value < parent);
        let end = parents.partition_point(|value| *value <= parent);
        (begin, end)
    }
}

/// One trie level of distinct encoded keys and child row ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrieLevel {
    /// Field represented by this level.
    pub field: FieldId,
    /// Distinct encoded keys at this level.
    pub keys: Vec<EncodedOwned>,
    /// Half-open ranges into `SortedTrieIndex::order`.
    pub ranges: Vec<RowRange>,
    /// Parent entry index in previous level, or `u32::MAX` for root level.
    pub parent: Vec<u32>,
}

/// Sorted trie shape and build statistics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrieStats {
    /// Number of rows in the indexed relation.
    pub row_count: usize,
    /// Distinct key count by depth.
    pub distinct_by_depth: Vec<usize>,
    /// Average fanout by depth.
    pub avg_fanout_by_depth: Vec<f64>,
    /// Maximum fanout by depth.
    pub max_fanout_by_depth: Vec<usize>,
    /// Build elapsed time in microseconds.
    pub build_micros: u128,
}

impl TrieStats {
    fn from_levels(row_count: usize, levels: &[TrieLevel], build_micros: u128) -> Self {
        let distinct_by_depth = levels
            .iter()
            .map(|level| level.keys.len())
            .collect::<Vec<_>>();
        let mut avg_fanout_by_depth = Vec::new();
        let mut max_fanout_by_depth = Vec::new();
        for level in levels {
            let mut groups = Vec::new();
            let mut current_parent = None;
            let mut current_count = 0usize;
            for parent in &level.parent {
                if current_parent == Some(*parent) {
                    current_count += 1;
                } else {
                    if current_parent.is_some() {
                        groups.push(current_count);
                    }
                    current_parent = Some(*parent);
                    current_count = 1;
                }
            }
            if current_parent.is_some() {
                groups.push(current_count);
            }
            let max = groups.iter().copied().max().unwrap_or(0);
            let avg = if groups.is_empty() {
                0.0
            } else {
                groups.iter().sum::<usize>() as f64 / groups.len() as f64
            };
            avg_fanout_by_depth.push(avg);
            max_fanout_by_depth.push(max);
        }
        Self {
            row_count,
            distinct_by_depth,
            avg_fanout_by_depth,
            max_fanout_by_depth,
            build_micros,
        }
    }
}

/// Linear iterator interface used by leapfrog join.
pub trait LinearIter {
    /// Current encoded key.
    fn key(&self) -> EncodedRef<'_>;
    /// Advances to the next key.
    fn next(&mut self);
    /// Seeks to the least key greater than or equal to `target`.
    fn seek(&mut self, target: EncodedRef<'_>);
    /// True when the iterator is exhausted.
    fn at_end(&self) -> bool;
}

/// Trie iterator interface used by LFTJ.
pub trait TrieIter: LinearIter {
    /// Descends to the next trie depth.
    fn open(&mut self);
    /// Returns to the parent trie depth.
    fn up(&mut self);
    /// Current trie depth.
    fn depth(&self) -> usize;
    /// Current row range into sorted row order.
    fn current_range(&self) -> RowRange;
    /// Number of rows under the current key/range.
    fn count(&self) -> usize;
}

/// Concrete sorted trie iterator.
pub struct SortedTrieIter<'a> {
    index: &'a SortedTrieIndex,
    stack: Vec<TrieFrame>,
}

/// Cursor frame for one trie depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrieFrame {
    /// Trie depth.
    pub depth: usize,
    /// Inclusive begin entry in this level.
    pub begin: usize,
    /// Exclusive end entry in this level.
    pub end: usize,
    /// Current entry in this level.
    pub pos: usize,
}

impl<'a> SortedTrieIter<'a> {
    /// Returns row IDs under the current key.
    pub fn current_rows(&self) -> &'a [RowId] {
        let range = self.current_range();
        &self.index.order[range.start.0 as usize..range.end.0 as usize]
    }

    fn current_frame(&self) -> Option<&TrieFrame> {
        self.stack.last()
    }

    fn current_frame_mut(&mut self) -> Option<&mut TrieFrame> {
        self.stack.last_mut()
    }

    fn current_entry_index(&self) -> Option<usize> {
        let frame = self.current_frame()?;
        if frame.pos < frame.end {
            Some(frame.pos)
        } else {
            None
        }
    }
}

impl LinearIter for SortedTrieIter<'_> {
    fn key(&self) -> EncodedRef<'_> {
        let frame = self
            .current_frame()
            .expect("sorted trie key requested before open");
        self.index.levels[frame.depth].keys[frame.pos].as_ref()
    }

    fn next(&mut self) {
        if let Some(frame) = self.current_frame_mut()
            && frame.pos < frame.end
        {
            frame.pos += 1;
        }
    }

    fn seek(&mut self, target: EncodedRef<'_>) {
        let Some(frame) = self.current_frame().copied() else {
            return;
        };
        let keys = &self.index.levels[frame.depth].keys;
        let mut low = frame.pos;
        let mut high = frame.end;
        while low < high {
            let mid = low + (high - low) / 2;
            if keys[mid].as_bytes() < target.as_bytes() {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if let Some(frame) = self.current_frame_mut() {
            frame.pos = low;
        }
    }

    fn at_end(&self) -> bool {
        self.current_frame()
            .is_none_or(|frame| frame.pos >= frame.end)
    }
}

impl TrieIter for SortedTrieIter<'_> {
    fn open(&mut self) {
        let depth = self.stack.len();
        if depth >= self.index.levels.len() {
            self.stack.push(TrieFrame {
                depth,
                begin: 0,
                end: 0,
                pos: 0,
            });
            return;
        }
        let (begin, end) = if depth == 0 {
            (0, self.index.levels[0].keys.len())
        } else if let Some(parent_entry) = self.current_entry_index() {
            self.index.child_bounds(depth, parent_entry as u32)
        } else {
            (0, 0)
        };
        self.stack.push(TrieFrame {
            depth,
            begin,
            end,
            pos: begin,
        });
    }

    fn up(&mut self) {
        self.stack.pop();
    }

    fn depth(&self) -> usize {
        self.current_frame().map_or(0, |frame| frame.depth)
    }

    fn current_range(&self) -> RowRange {
        let Some(frame) = self.current_frame() else {
            return RowRange {
                start: RowId(0),
                end: RowId(0),
            };
        };
        if frame.pos >= frame.end {
            return RowRange {
                start: RowId(0),
                end: RowId(0),
            };
        }
        self.index.levels[frame.depth].ranges[frame.pos]
    }

    fn count(&self) -> usize {
        let range = self.current_range();
        range.end.0.saturating_sub(range.start.0) as usize
    }
}

fn build_levels(
    relation: &RelationImage,
    order: &[RowId],
    fields: &[FieldId],
) -> Result<Vec<TrieLevel>> {
    let mut levels = Vec::new();
    let mut parents = vec![(0usize, order.len(), u32::MAX)];
    for field in fields {
        let mut level = TrieLevel {
            field: *field,
            keys: Vec::new(),
            ranges: Vec::new(),
            parent: Vec::new(),
        };
        let mut next_parents = Vec::new();
        for (parent_start, parent_end, parent_index) in parents {
            let mut start = parent_start;
            while start < parent_end {
                let key = EncodedOwned::from_ref(
                    relation
                        .encoded(order[start], *field)
                        .ok_or_else(|| crate::Error::internal("missing trie field value"))?,
                );
                let mut end = start + 1;
                while end < parent_end {
                    let next_key = relation
                        .encoded(order[end], *field)
                        .ok_or_else(|| crate::Error::internal("missing trie field value"))?;
                    if next_key.as_bytes() != key.as_bytes() {
                        break;
                    }
                    end += 1;
                }
                let entry_index = level.keys.len() as u32;
                level.keys.push(key);
                level.ranges.push(RowRange {
                    start: RowId(start as u32),
                    end: RowId(end as u32),
                });
                level.parent.push(parent_index);
                next_parents.push((start, end, entry_index));
                start = end;
            }
        }
        parents = next_parents;
        levels.push(level);
    }
    Ok(levels)
}

#[cfg(test)]
mod tests {
    use bumbledb_core::schema::{
        FieldDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind, SchemaDescriptor,
        ValueType,
    };

    use super::*;
    use crate::{Environment, Row, StorageSchema, Value};

    #[test]
    fn builds_one_level_trie_and_collapses_duplicate_keys() {
        let image = account_image();
        let account = image.relation("Account").unwrap();
        let index =
            SortedTrieIndex::build(account, IndexSpec::new("by_currency", [FieldId(1)])).unwrap();

        assert_eq!(index.order, vec![RowId(0), RowId(2), RowId(1)]);
        assert_eq!(index.stats.row_count, 3);
        assert_eq!(index.stats.distinct_by_depth, vec![2]);
        assert_eq!(index.stats.max_fanout_by_depth, vec![2]);

        let mut iter = index.iter();
        iter.open();
        assert_eq!(iter.key().as_bytes(), 840u64.to_be_bytes().as_slice());
        assert_eq!(iter.count(), 2);
        assert_eq!(iter.current_rows(), &[RowId(0), RowId(2)]);
        iter.next();
        assert_eq!(iter.key().as_bytes(), 978u64.to_be_bytes().as_slice());
        assert_eq!(iter.count(), 1);
        assert_eq!(iter.current_rows(), &[RowId(1)]);
        iter.next();
        assert!(iter.at_end());
    }

    #[test]
    fn seek_lands_on_least_upper_bound() {
        let image = account_image();
        let account = image.relation("Account").unwrap();
        let index =
            SortedTrieIndex::build(account, IndexSpec::new("by_currency", [FieldId(1)])).unwrap();
        let target = EncodedOwned::Eight(900u64.to_be_bytes());

        let mut iter = index.iter();
        iter.open();
        iter.seek(target.as_ref());
        assert_eq!(iter.key().as_bytes(), 978u64.to_be_bytes().as_slice());
        iter.seek(
            999u64
                .to_be_bytes()
                .as_slice()
                .try_into()
                .map(EncodedRef::Eight)
                .unwrap(),
        );
        assert!(iter.at_end());
    }

    #[test]
    fn open_and_up_preserve_parent_cursor_state() {
        let image = account_image();
        let account = image.relation("Account").unwrap();
        let index = SortedTrieIndex::build(
            account,
            IndexSpec::new("currency_active_id", [FieldId(1), FieldId(2), FieldId(0)]),
        )
        .unwrap();

        let mut iter = index.iter();
        iter.open();
        assert_eq!(iter.key().as_bytes(), 840u64.to_be_bytes().as_slice());
        iter.open();
        assert_eq!(iter.depth(), 1);
        assert_eq!(iter.key().as_bytes(), &[1]);
        assert_eq!(iter.count(), 2);
        iter.open();
        assert_eq!(iter.depth(), 2);
        assert_eq!(iter.key().as_bytes(), 1u64.to_be_bytes().as_slice());
        iter.next();
        assert_eq!(iter.key().as_bytes(), 3u64.to_be_bytes().as_slice());
        iter.up();
        assert_eq!(iter.depth(), 1);
        assert_eq!(iter.key().as_bytes(), &[1]);
        iter.up();
        assert_eq!(iter.depth(), 0);
        assert_eq!(iter.key().as_bytes(), 840u64.to_be_bytes().as_slice());
        iter.next();
        assert_eq!(iter.key().as_bytes(), 978u64.to_be_bytes().as_slice());
    }

    #[test]
    fn three_level_ranges_map_to_expected_rows() {
        let image = account_image();
        let account = image.relation("Account").unwrap();
        let index = SortedTrieIndex::build(
            account,
            IndexSpec::new("currency_active_id", [FieldId(1), FieldId(2), FieldId(0)]),
        )
        .unwrap();

        let mut iter = index.iter();
        iter.open();
        iter.open();
        iter.open();
        assert_eq!(
            iter.current_range(),
            RowRange {
                start: RowId(0),
                end: RowId(1)
            }
        );
        assert_eq!(iter.current_rows(), &[RowId(0)]);
        iter.next();
        assert_eq!(
            iter.current_range(),
            RowRange {
                start: RowId(1),
                end: RowId(2)
            }
        );
        assert_eq!(iter.current_rows(), &[RowId(2)]);
    }

    fn account_image() -> crate::QueryImage {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(path).unwrap();
        let schema = StorageSchema::new(account_schema(), env.max_key_size()).unwrap();
        env.write(|txn| {
            for row in account_rows() {
                txn.insert(&schema, row)?;
            }
            Ok::<_, crate::Error>(())
        })
        .unwrap();
        env.query_image(&schema).unwrap().as_ref().clone()
    }

    fn account_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "Accounts",
            vec![RelationDescriptor::new(
                "Account",
                RelationKind::Entity,
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Id {
                            name: "AccountId".to_owned(),
                            relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Symbol {
                            name: "Currency".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("active", ValueType::Bool),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )],
        )
    }

    fn account_rows() -> Vec<Row> {
        vec![
            Row::new(
                "Account",
                [
                    ("id", Value::Id(1)),
                    ("currency", Value::Symbol(840)),
                    ("active", Value::Bool(true)),
                ],
            ),
            Row::new(
                "Account",
                [
                    ("id", Value::Id(2)),
                    ("currency", Value::Symbol(978)),
                    ("active", Value::Bool(false)),
                ],
            ),
            Row::new(
                "Account",
                [
                    ("id", Value::Id(3)),
                    ("currency", Value::Symbol(840)),
                    ("active", Value::Bool(true)),
                ],
            ),
        ]
    }
}
