use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::{
    AccessId, EncodedOwned, Error, FieldId, IndexSpec, RelationId, RelationImage, Result, RowId,
    SortedTrieIndex, StorageSchema,
};

/// Snapshot-scoped cache for optimizer/planner relation statistics.
#[derive(Clone, Default)]
pub(crate) struct PlannerStatsCache {
    inner: Arc<PlannerStatsCacheInner>,
}

#[derive(Default)]
struct PlannerStatsCacheInner {
    relations: RwLock<BTreeMap<RelationId, Arc<OptimizerRelationStats>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    build_micros: AtomicU64,
}

impl fmt::Debug for PlannerStatsCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlannerStatsCache")
            .field("diagnostics", &self.diagnostics())
            .finish()
    }
}

impl PlannerStatsCache {
    /// Returns cached relation stats or builds them for this immutable image.
    pub(crate) fn get_or_build(
        &self,
        schema: &StorageSchema,
        relation: &RelationImage,
    ) -> Result<Arc<OptimizerRelationStats>> {
        if let Some(stats) = self
            .inner
            .relations
            .read()
            .map_err(|_| Error::internal("planner stats cache read lock poisoned"))?
            .get(&relation.id)
            .cloned()
        {
            self.inner.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(stats);
        }

        self.inner.misses.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();
        let built = Arc::new(OptimizerRelationStats::build(schema, relation)?);
        let elapsed = start.elapsed().as_micros() as u64;

        let mut relations = self
            .inner
            .relations
            .write()
            .map_err(|_| Error::internal("planner stats cache write lock poisoned"))?;
        if let Some(existing) = relations.get(&relation.id).cloned() {
            self.inner.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(existing);
        }
        relations.insert(relation.id, built.clone());
        self.inner.builds.fetch_add(1, Ordering::Relaxed);
        self.inner
            .build_micros
            .fetch_add(elapsed, Ordering::Relaxed);
        Ok(built)
    }

    /// Returns current cache diagnostics.
    pub(crate) fn diagnostics(&self) -> PlannerStatsCacheDiagnostics {
        PlannerStatsCacheDiagnostics {
            cached_relations: self
                .inner
                .relations
                .read()
                .map_or(0, |relations| relations.len()),
            hits: self.inner.hits.load(Ordering::Relaxed),
            misses: self.inner.misses.load(Ordering::Relaxed),
            builds: self.inner.builds.load(Ordering::Relaxed),
            build_micros: self.inner.build_micros.load(Ordering::Relaxed),
        }
    }
}

/// Planner stats cache diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlannerStatsCacheDiagnostics {
    /// Cached relation stats count.
    pub cached_relations: usize,
    /// Relation stats cache hits.
    pub hits: u64,
    /// Relation stats cache misses.
    pub misses: u64,
    /// Relation stats builds inserted into the cache.
    pub builds: u64,
    /// Total relation stats build time in microseconds.
    pub build_micros: u64,
}

/// Optimizer relation statistics derived from one relation image.
#[derive(Clone, Debug)]
pub(crate) struct OptimizerRelationStats {
    /// Relation row count.
    pub rows: usize,
    /// Field stats keyed by field name.
    pub fields: BTreeMap<String, OptimizerFieldStats>,
    /// Index stats keyed by index/access-path name.
    pub indexes: BTreeMap<String, OptimizerIndexStats>,
}

impl OptimizerRelationStats {
    fn build(schema: &StorageSchema, relation: &RelationImage) -> Result<Self> {
        let mut fields = BTreeMap::new();
        for field in &relation.fields {
            fields.insert(
                field.name.clone(),
                OptimizerFieldStats::build(relation, field.id)?,
            );
        }

        let mut indexes = BTreeMap::new();
        for path in schema.access_paths(&relation.name)? {
            let field_ids = path
                .leading_fields
                .iter()
                .map(|field_name| {
                    relation
                        .fields
                        .iter()
                        .find(|field| &field.name == field_name)
                        .map(|field| field.id)
                        .ok_or_else(|| Error::unknown_field(&relation.name, field_name))
                })
                .collect::<Result<Vec<_>>>()?;
            let layout = schema
                .layout(&relation.name, &path.index_name)
                .ok_or_else(|| Error::unknown_index(&relation.name, &path.index_name))?;
            let trie = SortedTrieIndex::build(
                relation,
                IndexSpec::new(format!("{}_stats", path.index_name), field_ids),
            )?;
            indexes.insert(
                path.index_name,
                OptimizerIndexStats {
                    index: AccessId(layout.index_id),
                    rows: relation.row_count,
                    distinct_by_depth: trie.stats.distinct_by_depth,
                    avg_fanout_by_depth: trie.stats.avg_fanout_by_depth,
                    max_fanout_by_depth: trie.stats.max_fanout_by_depth,
                },
            );
        }

        Ok(Self {
            rows: relation.row_count,
            fields,
            indexes,
        })
    }
}

/// Optimizer field statistics.
#[derive(Clone, Debug)]
pub(crate) struct OptimizerFieldStats {
    /// Exact distinct count.
    pub distinct: usize,
    /// Minimum encoded value.
    pub min: Option<EncodedOwned>,
    /// Maximum encoded value.
    pub max: Option<EncodedOwned>,
    /// Top high-frequency encoded values.
    pub heavy_hitters: Vec<(EncodedOwned, usize)>,
}

impl OptimizerFieldStats {
    fn build(relation: &RelationImage, field: FieldId) -> Result<Self> {
        let mut frequencies = BTreeMap::<EncodedOwned, usize>::new();
        for row in 0..relation.row_count {
            let value = relation
                .encoded(RowId(row as u32), field)
                .map(EncodedOwned::from_ref)
                .ok_or_else(|| Error::internal("missing optimizer field value"))?;
            *frequencies.entry(value).or_insert(0) += 1;
        }
        let distinct = frequencies.len();
        let min = frequencies.keys().next().cloned();
        let max = frequencies.keys().next_back().cloned();
        let heavy_hitter_floor = (relation.row_count / 10).max(2);
        let mut heavy_hitters = frequencies
            .iter()
            .filter(|(_, count)| **count >= heavy_hitter_floor)
            .map(|(value, count)| (value.clone(), *count))
            .collect::<Vec<_>>();
        heavy_hitters.sort_by(|(left_value, left_count), (right_value, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_value.cmp(right_value))
        });
        heavy_hitters.truncate(4);
        Ok(Self {
            distinct,
            min,
            max,
            heavy_hitters,
        })
    }
}

/// Optimizer index/access-path statistics.
#[derive(Clone, Debug)]
pub(crate) struct OptimizerIndexStats {
    /// Dense storage access ID.
    pub index: AccessId,
    /// Indexed row count.
    pub rows: usize,
    /// Distinct count by trie depth.
    pub distinct_by_depth: Vec<usize>,
    /// Average fanout by trie depth.
    pub avg_fanout_by_depth: Vec<f64>,
    /// Maximum fanout by trie depth.
    pub max_fanout_by_depth: Vec<usize>,
}

impl OptimizerIndexStats {
    pub(crate) fn estimated_rows_for_prefix(&self, prefix_len: usize) -> u64 {
        if prefix_len == 0 {
            return self.rows.max(1) as u64;
        }
        let distinct = self
            .distinct_by_depth
            .get(prefix_len - 1)
            .copied()
            .unwrap_or(1)
            .max(1);
        divide_ceil(self.rows.max(1) as u64, distinct as u64).max(1)
    }

    pub(crate) fn fanout_after_prefix(&self, prefix_len: usize) -> u64 {
        self.avg_fanout_by_depth
            .get(prefix_len)
            .copied()
            .unwrap_or(1.0)
            .ceil()
            .max(1.0) as u64
    }

    pub(crate) fn max_fanout_after_prefix(&self, prefix_len: usize) -> usize {
        self.max_fanout_by_depth
            .get(prefix_len)
            .copied()
            .unwrap_or(1)
            .max(1)
    }
}

fn divide_ceil(value: u64, divisor: u64) -> u64 {
    if divisor == 0 {
        value
    } else {
        value.div_ceil(divisor)
    }
}
