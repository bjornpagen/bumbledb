use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::query_image::FactId;
use crate::{
    AccessId, EncodedOwned, Error, FieldId, RelationId, RelationImage, Result, StorageSchema,
};

const FIELD_STATS_SAMPLE_ROWS: usize = 4096;

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
    field_stats_built: AtomicU64,
    index_stats_built: AtomicU64,
    stats_from_segments: AtomicU64,
    stats_exact_scans: AtomicU64,
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
        self.inner
            .field_stats_built
            .fetch_add(built.fields.len() as u64, Ordering::Relaxed);
        self.inner
            .index_stats_built
            .fetch_add(built.indexes.len() as u64, Ordering::Relaxed);
        self.inner
            .stats_from_segments
            .fetch_add(built.indexes.len() as u64, Ordering::Relaxed);
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
            field_stats_built: self.inner.field_stats_built.load(Ordering::Relaxed),
            index_stats_built: self.inner.index_stats_built.load(Ordering::Relaxed),
            stats_from_segments: self.inner.stats_from_segments.load(Ordering::Relaxed),
            stats_exact_scans: self.inner.stats_exact_scans.load(Ordering::Relaxed),
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
    /// Field-stat descriptors built without exact scans.
    pub field_stats_built: u64,
    /// Access-path/index-stat descriptors built without exact trie construction.
    pub index_stats_built: u64,
    /// Access-path stats derived from relation/access metadata.
    pub stats_from_segments: u64,
    /// Exact field/index scans performed during planning.
    pub stats_exact_scans: u64,
}

/// Optimizer relation statistics derived from one relation image.
#[derive(Clone, Debug)]
pub(crate) struct OptimizerRelationStats {
    /// Relation fact count.
    pub facts: usize,
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
                OptimizerFieldStats::sample(relation, field.id)?,
            );
        }

        let mut indexes = BTreeMap::new();
        for path in schema.access_paths(&relation.name)? {
            let layout = schema
                .layout(&relation.name, &path.index_name)
                .ok_or_else(|| Error::unknown_index(&relation.name, &path.index_name))?;
            indexes.insert(
                path.index_name,
                OptimizerIndexStats::cheap(
                    AccessId(layout.index_id),
                    relation.fact_count,
                    &path.leading_fields,
                    &fields,
                ),
            );
        }

        Ok(Self {
            facts: relation.fact_count,
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
    fn sample(relation: &RelationImage, field: FieldId) -> Result<Self> {
        let sample_facts = relation.fact_count.min(FIELD_STATS_SAMPLE_ROWS);
        let mut frequencies = BTreeMap::<EncodedOwned, usize>::new();
        for fact in 0..sample_facts {
            let value = relation
                .encoded(FactId(fact as u32), field)
                .map(EncodedOwned::from_ref)
                .ok_or_else(|| Error::internal("missing optimizer sample field value"))?;
            *frequencies.entry(value).or_insert(0) += 1;
        }
        let sample_distinct = frequencies.len().max(1);
        let distinct =
            if sample_facts == relation.fact_count || sample_distinct <= sample_facts / 16 {
                sample_distinct
            } else {
                sample_distinct
                    .saturating_mul(relation.fact_count.max(1))
                    .div_ceil(sample_facts.max(1))
                    .min(relation.fact_count.max(1))
            };
        let min = frequencies.keys().next().cloned();
        let max = frequencies.keys().next_back().cloned();
        let heavy_hitter_floor = (sample_facts / 10).max(2);
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
    /// Indexed fact count.
    pub facts: usize,
    /// Distinct count by trie depth.
    pub distinct_by_depth: Vec<usize>,
    /// Average fanout by trie depth.
    pub avg_fanout_by_depth: Vec<f64>,
    /// Maximum fanout by trie depth.
    pub max_fanout_by_depth: Vec<usize>,
}

impl OptimizerIndexStats {
    fn cheap(
        index: AccessId,
        facts: usize,
        leading_fields: &[String],
        fields: &BTreeMap<String, OptimizerFieldStats>,
    ) -> Self {
        let facts = facts.max(1);
        let depth = leading_fields.len().max(1);
        let mut distinct_by_depth = Vec::with_capacity(depth);
        let mut avg_fanout_by_depth = Vec::with_capacity(depth);
        let mut max_fanout_by_depth = Vec::with_capacity(depth);
        for level in 0..depth {
            let distinct = leading_fields
                .get(level)
                .and_then(|field| fields.get(field))
                .map_or(facts, |stats| stats.distinct)
                .max(1)
                .min(facts);
            let depth_distinct = if level + 1 == depth { facts } else { distinct };
            distinct_by_depth.push(depth_distinct);
            let parent_distinct = if level == 0 {
                1
            } else {
                distinct_by_depth[level - 1].max(1)
            };
            let fanout = depth_distinct as f64 / parent_distinct as f64;
            avg_fanout_by_depth.push(fanout.max(1.0));
            max_fanout_by_depth.push(fanout.ceil().max(1.0) as usize);
        }
        Self {
            index,
            facts,
            distinct_by_depth,
            avg_fanout_by_depth,
            max_fanout_by_depth,
        }
    }

    pub(crate) fn estimated_facts_for_prefix(&self, prefix_len: usize) -> u64 {
        if prefix_len == 0 {
            return self.facts.max(1) as u64;
        }
        let distinct = self
            .distinct_by_depth
            .get(prefix_len - 1)
            .copied()
            .unwrap_or(1)
            .max(1);
        divide_ceil(self.facts.max(1) as u64, distinct as u64).max(1)
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
