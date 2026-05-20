use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use bumbledb_core::schema::{RelationDescriptor, SchemaFingerprint, ValueType};

use crate::planner_stats::{PlannerStatsCache, PlannerStatsCacheDiagnostics};
use crate::query::ExecutionPlan;
use crate::{
    AccessId, EncodedOwned, Error, ReadTxn, Result, SegmentDescriptor, SortedTrieIndex,
    StorageSchema,
};
use crate::{HashTrieIndex, IndexSpec, LeafMode};

/// Cache key for an immutable query image.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QueryImageKey {
    /// Schema fingerprint for the image.
    pub schema: SchemaFingerprint,
    /// Last committed storage transaction ID visible to the image.
    pub tx_id: u64,
    /// Relation/index/column scope loaded into this image.
    pub scope: QueryImageScopeKey,
}

/// Stable cache key for a query-image scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryImageScopeKey(pub [u8; 32]);

/// Explicit relation scope for a query image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryImageScope {
    relations: BTreeMap<RelationId, RelationScope>,
}

/// Explicit field/index scope for one relation image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationScope {
    columns: BTreeSet<FieldId>,
    indexes: BTreeSet<AccessId>,
    include_all_columns: bool,
    include_all_indexes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct QueryShapeKey(pub(crate) [u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LftjAtomKey(pub(crate) [u8; 32]);

impl PartialOrd for QueryImageKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueryImageKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.schema.0, self.tx_id, self.scope).cmp(&(other.schema.0, other.tx_id, other.scope))
    }
}

impl QueryImageScope {
    /// Full-schema image scope.
    pub fn full(schema: &StorageSchema) -> Self {
        let relations = schema
            .descriptor()
            .relations
            .iter()
            .enumerate()
            .map(|(id, relation)| {
                let relation_id = RelationId(id as u16);
                let indexes = schema
                    .layouts_for_relation(relation_id.0)
                    .map(|layout| AccessId(layout.index_id))
                    .collect();
                (
                    relation_id,
                    RelationScope {
                        columns: (0..relation.fields.len())
                            .map(|field| FieldId(field as u16))
                            .collect(),
                        indexes,
                        include_all_columns: true,
                        include_all_indexes: true,
                    },
                )
            })
            .collect();
        Self { relations }
    }

    /// Scope containing all fields and indexes for selected relations.
    pub fn relations_all(
        schema: &StorageSchema,
        relation_ids: impl IntoIterator<Item = RelationId>,
    ) -> Self {
        let mut relations = BTreeMap::new();
        for relation_id in relation_ids {
            let Some(relation) = schema.descriptor().relations.get(relation_id.0 as usize) else {
                continue;
            };
            let indexes = schema
                .layouts_for_relation(relation_id.0)
                .map(|layout| AccessId(layout.index_id))
                .collect();
            relations.insert(
                relation_id,
                RelationScope {
                    columns: (0..relation.fields.len())
                        .map(|field| FieldId(field as u16))
                        .collect(),
                    indexes,
                    include_all_columns: true,
                    include_all_indexes: true,
                },
            );
        }
        Self { relations }
    }

    /// Returns a stable structural cache key for this scope.
    pub fn key(&self) -> QueryImageScopeKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bumbledb.query_image_scope.v1");
        for (relation_id, scope) in &self.relations {
            hasher.update(&relation_id.0.to_be_bytes());
            hasher.update(&[u8::from(scope.include_all_columns)]);
            hasher.update(&[u8::from(scope.include_all_indexes)]);
            hasher.update(&(scope.columns.len() as u64).to_be_bytes());
            for field in &scope.columns {
                hasher.update(&field.0.to_be_bytes());
            }
            hasher.update(&(scope.indexes.len() as u64).to_be_bytes());
            for index in &scope.indexes {
                hasher.update(&index.0.to_be_bytes());
            }
        }
        QueryImageScopeKey(*hasher.finalize().as_bytes())
    }

    fn relation_scope(&self, relation: RelationId) -> Option<&RelationScope> {
        self.relations.get(&relation)
    }

    fn relation_ids(&self) -> impl Iterator<Item = RelationId> + '_ {
        self.relations.keys().copied()
    }
}

/// Dense relation ID in schema declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u16);

/// Dense field ID in relation declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub u16);

/// Dense row ID inside a relation image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowId(pub u32);

/// Half-open row-id range inside a relation image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowRange {
    /// Inclusive start row id.
    pub start: RowId,
    /// Exclusive end row id.
    pub end: RowId,
}

/// Borrowed row-id set reference used by future indexes and plan nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowSetRef<'a> {
    /// Empty row set.
    Empty,
    /// Single row id.
    One(RowId),
    /// Contiguous row-id range.
    Range(RowRange),
    /// Borrowed row-id slice.
    Slice(&'a [RowId]),
}

/// Borrowed fixed-width encoded value reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedRef<'a> {
    /// One-byte encoded value.
    One(&'a [u8; 1]),
    /// Eight-byte encoded value.
    Eight(&'a [u8; 8]),
    /// Sixteen-byte encoded value.
    Sixteen(&'a [u8; 16]),
}

impl<'a> EncodedRef<'a> {
    /// Returns the encoded bytes for this value.
    #[inline]
    pub fn as_bytes(self) -> &'a [u8] {
        match self {
            EncodedRef::One(bytes) => &bytes[..],
            EncodedRef::Eight(bytes) => &bytes[..],
            EncodedRef::Sixteen(bytes) => &bytes[..],
        }
    }
}

/// Immutable snapshot-local image used by the future query runtime.
#[derive(Clone, Debug)]
pub struct QueryImage {
    key: QueryImageKey,
    relations: BTreeMap<RelationId, RelationImage>,
    relation_by_name: BTreeMap<String, RelationId>,
    stats: QueryImageStats,
    planner_stats: PlannerStatsCache,
    prepared_plans: PreparedPlanCache,
    static_empty_queries: Arc<RwLock<BTreeSet<QueryShapeKey>>>,
    sorted_trie_cache: Arc<RwLock<BTreeMap<LftjAtomKey, Arc<SortedTrieIndex>>>>,
    hash_trie_cache: Arc<RwLock<BTreeMap<String, Arc<HashTrieIndex>>>>,
}

impl QueryImage {
    #[expect(
        clippy::too_many_arguments,
        reason = "query image construction records scope and build diagnostics"
    )]
    fn new(
        schema: &StorageSchema,
        tx_id: u64,
        scope: QueryImageScope,
        relations: BTreeMap<RelationId, RelationImage>,
        build_micros: u128,
        segment_count: usize,
        segment_bytes: usize,
        built_from_segments: bool,
    ) -> Self {
        let relation_by_name = relations
            .values()
            .map(|relation| (relation.name.clone(), relation.id))
            .collect::<BTreeMap<_, _>>();
        let relation_count = relation_by_name.len();
        let row_count = relations.values().map(|relation| relation.row_count).sum();
        let encoded_column_bytes = relations
            .values()
            .map(RelationImage::encoded_column_bytes)
            .sum();
        let sorted_trie_bytes = segment_bytes.saturating_sub(encoded_column_bytes);
        Self {
            key: QueryImageKey {
                schema: schema.descriptor().fingerprint(),
                tx_id,
                scope: scope.key(),
            },
            relations,
            relation_by_name,
            stats: QueryImageStats {
                relation_count,
                row_count,
                encoded_column_bytes,
                sorted_trie_bytes,
                hash_trie_bytes: 0,
                segment_count,
                segment_bytes,
                built_from_segments,
                build_micros,
            },
            planner_stats: PlannerStatsCache::default(),
            prepared_plans: PreparedPlanCache::default(),
            static_empty_queries: Arc::default(),
            sorted_trie_cache: Arc::default(),
            hash_trie_cache: Arc::default(),
        }
    }

    /// Returns this image's cache key.
    pub fn key(&self) -> QueryImageKey {
        self.key.clone()
    }

    /// Returns all loaded relation images in relation-id order.
    pub fn relations(&self) -> impl Iterator<Item = &RelationImage> {
        self.relations.values()
    }

    /// Looks up a loaded relation image by ID.
    pub fn relation_by_id(&self, id: RelationId) -> Option<&RelationImage> {
        self.relations.get(&id)
    }

    /// Looks up a relation image by name.
    pub fn relation(&self, name: &str) -> Option<&RelationImage> {
        let id = self.relation_by_name.get(name)?;
        self.relations.get(id)
    }

    /// Returns memory/build statistics for this image.
    pub fn stats(&self) -> &QueryImageStats {
        &self.stats
    }

    /// Returns current planner statistics cache diagnostics for this image.
    pub fn planner_stats_diagnostics(&self) -> PlannerStatsCacheDiagnostics {
        self.planner_stats.diagnostics()
    }

    pub(crate) fn planner_relation_stats(
        &self,
        schema: &StorageSchema,
        relation: &RelationImage,
    ) -> Result<std::sync::Arc<crate::planner_stats::OptimizerRelationStats>> {
        self.planner_stats.get_or_build(schema, relation)
    }

    pub(crate) fn prepared_plan_diagnostics(&self) -> PreparedPlanCacheDiagnostics {
        self.prepared_plans.diagnostics()
    }

    pub(crate) fn cached_prepared_plan(
        &self,
        key: QueryShapeKey,
    ) -> Result<Option<Arc<ExecutionPlan>>> {
        self.prepared_plans.get(key)
    }

    pub(crate) fn insert_prepared_plan(
        &self,
        key: QueryShapeKey,
        plan: ExecutionPlan,
        build_micros: u64,
    ) -> Result<Arc<ExecutionPlan>> {
        self.prepared_plans.insert(key, plan, build_micros)
    }

    pub(crate) fn static_empty_cached(&self, key: QueryShapeKey) -> Result<bool> {
        Ok(self
            .static_empty_queries
            .read()
            .map_err(|_| Error::internal("static-empty cache read lock poisoned"))?
            .contains(&key))
    }

    pub(crate) fn insert_static_empty(&self, key: QueryShapeKey) -> Result<()> {
        self.static_empty_queries
            .write()
            .map_err(|_| Error::internal("static-empty cache write lock poisoned"))?
            .insert(key);
        Ok(())
    }

    pub(crate) fn cached_sorted_trie(
        &self,
        key: LftjAtomKey,
        build: impl FnOnce() -> Result<SortedTrieBuild>,
    ) -> Result<CachedSortedTrie> {
        if let Some(index) = self
            .sorted_trie_cache
            .read()
            .map_err(|_| Error::internal("sorted trie cache read lock poisoned"))?
            .get(&key)
            .cloned()
        {
            return Ok(CachedSortedTrie {
                index,
                hit: true,
                build_micros: 0,
                source_rows_scanned: 0,
                rows_retained: 0,
                bytes_copied: 0,
                scan_micros: 0,
                column_micros: 0,
                sort_micros: 0,
            });
        }

        let start = Instant::now();
        let built = build()?;
        let index = Arc::new(built.index);
        let build_micros = start.elapsed().as_micros();
        let mut cache = self
            .sorted_trie_cache
            .write()
            .map_err(|_| Error::internal("sorted trie cache write lock poisoned"))?;
        if let Some(existing) = cache.get(&key).cloned() {
            return Ok(CachedSortedTrie {
                index: existing,
                hit: true,
                build_micros: 0,
                source_rows_scanned: 0,
                rows_retained: 0,
                bytes_copied: 0,
                scan_micros: 0,
                column_micros: 0,
                sort_micros: 0,
            });
        }
        cache.insert(key, index.clone());
        Ok(CachedSortedTrie {
            index,
            hit: false,
            build_micros,
            source_rows_scanned: built.source_rows_scanned,
            rows_retained: built.rows_retained,
            bytes_copied: built.bytes_copied,
            scan_micros: built.scan_micros,
            column_micros: built.column_micros,
            sort_micros: built.sort_micros,
        })
    }

    pub(crate) fn cached_hash_trie(
        &self,
        key: impl AsRef<str>,
        build: impl FnOnce() -> Result<HashTrieIndex>,
    ) -> Result<CachedHashTrie> {
        let key = key.as_ref();
        if let Some(index) = self
            .hash_trie_cache
            .read()
            .map_err(|_| Error::internal("hash trie cache read lock poisoned"))?
            .get(key)
            .cloned()
        {
            return Ok(CachedHashTrie { index, hit: true });
        }

        let index = Arc::new(build()?);
        let mut cache = self
            .hash_trie_cache
            .write()
            .map_err(|_| Error::internal("hash trie cache write lock poisoned"))?;
        if let Some(existing) = cache.get(key).cloned() {
            return Ok(CachedHashTrie {
                index: existing,
                hit: true,
            });
        }
        cache.insert(key.to_owned(), index.clone());
        Ok(CachedHashTrie { index, hit: false })
    }

    #[cfg(test)]
    fn content_fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.key.schema.0);
        for relation in self.relations.values() {
            hasher.update(relation.name.as_bytes());
            hasher.update(&(relation.row_count as u64).to_be_bytes());
            for field in &relation.fields {
                hasher.update(field.name.as_bytes());
                hasher.update(&(field.width as u64).to_be_bytes());
            }
            for column in &relation.columns {
                column.hash_into(&mut hasher);
            }
        }
        *hasher.finalize().as_bytes()
    }
}

#[derive(Clone, Default)]
struct PreparedPlanCache {
    inner: Arc<PreparedPlanCacheInner>,
}

#[derive(Default)]
struct PreparedPlanCacheInner {
    plans: RwLock<BTreeMap<QueryShapeKey, Arc<ExecutionPlan>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    build_micros: AtomicU64,
}

impl std::fmt::Debug for PreparedPlanCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPlanCache")
            .field("diagnostics", &self.diagnostics())
            .finish()
    }
}

impl PreparedPlanCache {
    fn get(&self, key: QueryShapeKey) -> Result<Option<Arc<ExecutionPlan>>> {
        let plan = self
            .inner
            .plans
            .read()
            .map_err(|_| Error::internal("prepared plan cache read lock poisoned"))?
            .get(&key)
            .cloned();
        if plan.is_some() {
            self.inner.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.inner.misses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(plan)
    }

    fn insert(
        &self,
        key: QueryShapeKey,
        plan: ExecutionPlan,
        build_micros: u64,
    ) -> Result<Arc<ExecutionPlan>> {
        let mut plans = self
            .inner
            .plans
            .write()
            .map_err(|_| Error::internal("prepared plan cache write lock poisoned"))?;
        if let Some(existing) = plans.get(&key).cloned() {
            self.inner.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(existing);
        }
        let plan = Arc::new(plan);
        plans.insert(key, plan.clone());
        self.inner.builds.fetch_add(1, Ordering::Relaxed);
        self.inner
            .build_micros
            .fetch_add(build_micros, Ordering::Relaxed);
        Ok(plan)
    }

    fn diagnostics(&self) -> PreparedPlanCacheDiagnostics {
        PreparedPlanCacheDiagnostics {
            cached_plans: self.inner.plans.read().map_or(0, |plans| plans.len()),
            hits: self.inner.hits.load(Ordering::Relaxed),
            misses: self.inner.misses.load(Ordering::Relaxed),
            builds: self.inner.builds.load(Ordering::Relaxed),
            build_micros: self.inner.build_micros.load(Ordering::Relaxed),
        }
    }
}

/// Prepared physical plan cache diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreparedPlanCacheDiagnostics {
    /// Cached prepared physical plans.
    pub cached_plans: usize,
    /// Prepared plan cache hits.
    pub hits: u64,
    /// Prepared plan cache misses.
    pub misses: u64,
    /// Prepared physical plan builds inserted into the cache.
    pub builds: u64,
    /// Total cold physical planning time inserted into the cache.
    pub build_micros: u64,
}

pub(crate) struct CachedSortedTrie {
    pub index: Arc<SortedTrieIndex>,
    pub hit: bool,
    pub build_micros: u128,
    pub source_rows_scanned: u64,
    pub rows_retained: u64,
    pub bytes_copied: u64,
    pub scan_micros: u64,
    pub column_micros: u64,
    pub sort_micros: u64,
}

pub(crate) struct SortedTrieBuild {
    pub index: SortedTrieIndex,
    pub source_rows_scanned: u64,
    pub rows_retained: u64,
    pub bytes_copied: u64,
    pub scan_micros: u64,
    pub column_micros: u64,
    pub sort_micros: u64,
}

pub(crate) struct CachedHashTrie {
    pub index: Arc<HashTrieIndex>,
    pub hit: bool,
}

pub(crate) fn build_sorted_trie_index(
    relation: &RelationImage,
    spec: IndexSpec,
) -> Result<SortedTrieIndex> {
    SortedTrieIndex::build(relation, spec)
}

pub(crate) fn build_hash_trie_index(
    relation: &RelationImage,
    spec: IndexSpec,
) -> Result<HashTrieIndex> {
    HashTrieIndex::build_with_mode(relation, spec, LeafMode::Rows)
}

/// Query image build/cache statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryImageStats {
    /// Number of relation images.
    pub relation_count: usize,
    /// Total row count across all relations.
    pub row_count: usize,
    /// Encoded column bytes stored in relation images.
    pub encoded_column_bytes: usize,
    /// Bytes used by sorted trie indexes. Zero until the sorted-trie PRD lands.
    pub sorted_trie_bytes: usize,
    /// Bytes used by hash trie indexes. Zero until the hash-trie PRD lands.
    pub hash_trie_bytes: usize,
    /// Number of durable relation segments used by this image.
    pub segment_count: usize,
    /// Bytes read from durable column/index segments for this image.
    pub segment_bytes: usize,
    /// True when every relation image was built from visible segment metadata.
    pub built_from_segments: bool,
    /// Build elapsed time in microseconds.
    pub build_micros: u128,
}

/// Immutable image of one relation.
#[derive(Clone, Debug)]
pub struct RelationImage {
    /// Relation ID in schema declaration order.
    pub id: RelationId,
    /// Relation name.
    pub name: String,
    /// Number of rows in this image.
    pub row_count: usize,
    /// Field metadata in declaration order.
    pub fields: Vec<FieldImage>,
    /// Encoded columns in declaration order.
    pub columns: Vec<ColumnImage>,
    /// Durable sorted index images in access-path order when available.
    pub indexes: Vec<RelationIndexImage>,
    /// Placeholder count for sorted indexes built in PRD 03.
    pub sorted_index_count: usize,
    /// Placeholder count for hash indexes built in PRD 06.
    pub hash_index_count: usize,
    /// Relation image statistics.
    pub stats: RelationStats,
}

/// Immutable durable sorted index bytes for one relation image.
#[derive(Clone, Debug)]
pub struct RelationIndexImage {
    /// Dense storage access ID.
    pub access: AccessId,
    /// Leading fields in access-path order.
    pub fields: Vec<FieldId>,
    /// Full covering components in encoded key order.
    pub components: Vec<RelationIndexComponent>,
    /// Bytes per encoded index entry.
    pub encoded_len: usize,
    /// Namespace/relation/access prefix bytes before components.
    pub prefix_len: usize,
    /// Concatenated encoded index entries.
    pub bytes: Vec<u8>,
}

/// One field component inside a durable relation index image.
#[derive(Clone, Debug)]
pub struct RelationIndexComponent {
    /// Field ID in relation declaration order.
    pub field: FieldId,
    /// Offset of this component inside an encoded index entry.
    pub offset: usize,
    /// Encoded component width.
    pub width: usize,
}

impl RelationIndexImage {
    /// Number of encoded components in one index entry.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns true when this encoded index entry layout contains `field`.
    pub fn contains_field(&self, field: FieldId) -> bool {
        self.components
            .iter()
            .any(|component| component.field == field)
    }

    /// Returns the encoded field bytes from one encoded index entry.
    pub fn component_bytes<'a>(&self, entry: &'a [u8], field: FieldId) -> Option<&'a [u8]> {
        let component = self
            .components
            .iter()
            .find(|component| component.field == field)?;
        entry.get(component.offset..component.offset + component.width)
    }

    /// Returns encoded entries matching a leading component prefix.
    pub fn entries_with_prefix<'a>(&'a self, prefix: &'a [u8]) -> RelationIndexPrefixIter<'a> {
        let range = self.prefix_range(prefix);
        RelationIndexPrefixIter {
            index: self,
            prefix,
            position: range.start,
            end: range.end,
        }
    }

    /// Returns the half-open entry-position range matching a leading component prefix.
    pub fn prefix_range(&self, prefix: &[u8]) -> Range<usize> {
        debug_assert!(prefix.len() <= self.encoded_len.saturating_sub(self.prefix_len));
        let start = self.lower_bound_prefix(prefix);
        let end = self.upper_bound_prefix(prefix);
        start..end
    }

    /// Returns the number of encoded index entries matching a leading component prefix.
    pub fn prefix_count(&self, prefix: &[u8]) -> usize {
        debug_assert!(prefix.len() <= self.encoded_len.saturating_sub(self.prefix_len));
        let entry_count = self.bytes.len() / self.encoded_len;
        let mut position = self.lower_bound_prefix(prefix);
        let start = position;
        while position < entry_count {
            let Some(entry) = self.entry(position) else {
                break;
            };
            let Some(key) = self.entry_prefix(entry, prefix.len()) else {
                break;
            };
            if key != prefix {
                break;
            }
            position += 1;
        }
        position.saturating_sub(start)
    }

    /// Returns true when any encoded index entry matches a leading component prefix.
    pub fn prefix_exists(&self, prefix: &[u8]) -> bool {
        self.prefix_count(prefix) != 0
    }

    /// Returns an encoded entry by entry position.
    pub fn entry_at(&self, position: usize) -> Option<&[u8]> {
        self.entry(position)
    }

    fn lower_bound_prefix(&self, prefix: &[u8]) -> usize {
        let entry_count = self.bytes.len() / self.encoded_len;
        let mut low = 0usize;
        let mut high = entry_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = self.entry(mid).unwrap_or(&[]);
            let key = self.entry_prefix(entry, prefix.len()).unwrap_or(&[]);
            if key < prefix {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn upper_bound_prefix(&self, prefix: &[u8]) -> usize {
        let entry_count = self.bytes.len() / self.encoded_len;
        let mut low = 0usize;
        let mut high = entry_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = self.entry(mid).unwrap_or(&[]);
            let key = self.entry_prefix(entry, prefix.len()).unwrap_or(&[]);
            if key <= prefix {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn entry(&self, position: usize) -> Option<&[u8]> {
        let start = position.checked_mul(self.encoded_len)?;
        self.bytes.get(start..start + self.encoded_len)
    }

    fn entry_prefix<'a>(&self, entry: &'a [u8], len: usize) -> Option<&'a [u8]> {
        entry.get(self.prefix_len..self.prefix_len + len)
    }
}

/// Iterator over durable index entries matching an encoded prefix.
pub struct RelationIndexPrefixIter<'a> {
    index: &'a RelationIndexImage,
    prefix: &'a [u8],
    position: usize,
    end: usize,
}

impl<'a> Iterator for RelationIndexPrefixIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.end {
            return None;
        }
        let entry = self.index.entry(self.position)?;
        let key = self.index.entry_prefix(entry, self.prefix.len())?;
        if key != self.prefix {
            self.position = self.end;
            return None;
        }
        self.position += 1;
        Some(entry)
    }
}

impl RelationImage {
    /// Returns the encoded value for `row` and `field`.
    pub fn encoded(&self, row: RowId, field: FieldId) -> Option<EncodedRef<'_>> {
        self.columns.get(field.0 as usize)?.encoded(row)
    }

    /// Returns the encoded bytes for `row` and `field`.
    pub fn encoded_bytes(&self, row: RowId, field: FieldId) -> Option<&[u8]> {
        self.encoded(row, field).map(EncodedRef::as_bytes)
    }

    /// Returns field metadata by ID.
    pub fn field(&self, field: FieldId) -> Option<&FieldImage> {
        self.fields.get(field.0 as usize)
    }

    /// Returns column metadata/data by field ID.
    pub fn column(&self, field: FieldId) -> Option<&ColumnImage> {
        self.columns.get(field.0 as usize)
    }

    /// Returns durable sorted index images for this relation.
    pub fn indexes(&self) -> &[RelationIndexImage] {
        &self.indexes
    }

    /// Returns all row IDs in this relation image.
    pub fn all_rows(&self) -> RowRange {
        RowRange {
            start: RowId(0),
            end: RowId(self.row_count as u32),
        }
    }

    /// Encoded column byte footprint.
    pub fn encoded_column_bytes(&self) -> usize {
        self.columns.iter().map(ColumnImage::byte_len).sum()
    }
}

/// Relation-level image statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationStats {
    /// Number of rows in the relation image.
    pub row_count: usize,
    /// Number of fields/columns.
    pub field_count: usize,
    /// Encoded column bytes.
    pub encoded_column_bytes: usize,
}

/// Field metadata inside a relation image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldImage {
    /// Field ID in relation declaration order.
    pub id: FieldId,
    /// Field name.
    pub name: String,
    /// Logical value type.
    pub value_type: ValueType,
    /// Fixed encoded width.
    pub width: usize,
}

impl FieldImage {
    /// Fixed encoded width for this field.
    pub fn encoded_width(&self) -> usize {
        self.width
    }
}

/// Typed fixed-width column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedColumn<T> {
    field: FieldId,
    values: Vec<T>,
}

impl<T> FixedColumn<T> {
    fn new(field: FieldId, values: Vec<T>) -> Self {
        Self { field, values }
    }

    /// Field ID stored by this column.
    pub fn field(&self) -> FieldId {
        self.field
    }

    /// Number of encoded values in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// True when this column has no values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<T: Copy> FixedColumn<T> {
    /// Returns a copied value by row ID.
    #[inline]
    pub fn get(&self, row: RowId) -> Option<T> {
        self.values.get(row.0 as usize).copied()
    }

    /// Returns a borrowed value by row ID.
    #[inline]
    pub fn get_ref(&self, row: RowId) -> Option<&T> {
        self.values.get(row.0 as usize)
    }
}

/// Builder for fixed-width encoded column images.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EncodedColumnBuilder {
    Bool {
        field: FieldId,
        values: Vec<[u8; 1]>,
    },
    Fixed8 {
        field: FieldId,
        values: Vec<[u8; 8]>,
    },
    Fixed16 {
        field: FieldId,
        values: Vec<[u8; 16]>,
    },
}

impl EncodedColumnBuilder {
    #[allow(dead_code)]
    pub(crate) fn new(field: FieldId, width: usize) -> Result<Self> {
        Self::with_capacity(field, width, 0)
    }

    pub(crate) fn with_capacity(field: FieldId, width: usize, capacity: usize) -> Result<Self> {
        Ok(match width {
            1 => Self::Bool {
                field,
                values: Vec::with_capacity(capacity),
            },
            8 => Self::Fixed8 {
                field,
                values: Vec::with_capacity(capacity),
            },
            16 => Self::Fixed16 {
                field,
                values: Vec::with_capacity(capacity),
            },
            _ => return Err(Error::internal(format!("unsupported column width {width}"))),
        })
    }

    pub(crate) fn append_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        match self {
            Self::Bool { values, .. } => values.push(exact_array::<1>(bytes)?),
            Self::Fixed8 { values, .. } => values.push(exact_array::<8>(bytes)?),
            Self::Fixed16 { values, .. } => values.push(exact_array::<16>(bytes)?),
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn append_encoded_owned(&mut self, value: &EncodedOwned) -> Result<()> {
        self.append_bytes(value.as_bytes())
    }

    #[allow(dead_code)]
    pub(crate) fn append_encoded_ref(&mut self, value: EncodedRef<'_>) -> Result<()> {
        self.append_bytes(value.as_bytes())
    }

    pub(crate) fn extend_flat_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let width = self.width();
        if width == 0 || !bytes.len().is_multiple_of(width) {
            return Err(Error::corrupt("segment column byte width mismatch"));
        }
        for chunk in bytes.chunks_exact(width) {
            self.append_bytes(chunk)?;
        }
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Bool { values, .. } => values.len(),
            Self::Fixed8 { values, .. } => values.len(),
            Self::Fixed16 { values, .. } => values.len(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)]
    pub(crate) fn byte_len(&self) -> usize {
        self.len() * self.width()
    }

    pub(crate) fn width(&self) -> usize {
        match self {
            Self::Bool { .. } => 1,
            Self::Fixed8 { .. } => 8,
            Self::Fixed16 { .. } => 16,
        }
    }

    pub(crate) fn finish(self) -> ColumnImage {
        match self {
            Self::Bool { field, values } => ColumnImage::Bool(FixedColumn::new(field, values)),
            Self::Fixed8 { field, values } => ColumnImage::Fixed8(FixedColumn::new(field, values)),
            Self::Fixed16 { field, values } => {
                ColumnImage::Fixed16(FixedColumn::new(field, values))
            }
        }
    }
}

pub(crate) fn encoded_column_builders(
    fields: &[FieldImage],
    capacity: usize,
) -> Result<Vec<EncodedColumnBuilder>> {
    fields
        .iter()
        .map(|field| EncodedColumnBuilder::with_capacity(field.id, field.width, capacity))
        .collect()
}

pub(crate) fn finish_column_builders(builders: Vec<EncodedColumnBuilder>) -> Vec<ColumnImage> {
    builders
        .into_iter()
        .map(EncodedColumnBuilder::finish)
        .collect()
}

/// Encoded fixed-width column image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnImage {
    /// Boolean/one-byte fixed-width column.
    Bool(FixedColumn<[u8; 1]>),
    /// Eight-byte fixed-width column.
    Fixed8(FixedColumn<[u8; 8]>),
    /// Sixteen-byte fixed-width column.
    Fixed16(FixedColumn<[u8; 16]>),
}

impl ColumnImage {
    pub(crate) fn from_flat_bytes(field: FieldId, width: usize, bytes: &[u8]) -> Result<Self> {
        let mut builder = EncodedColumnBuilder::with_capacity(field, width, bytes.len() / width)?;
        builder.extend_flat_bytes(bytes)?;
        Ok(builder.finish())
    }

    pub(crate) fn from_segment_bytes(field: FieldId, width: usize, bytes: Vec<u8>) -> Result<Self> {
        if width == 0 || !bytes.len().is_multiple_of(width) {
            return Err(Error::corrupt("segment column byte width mismatch"));
        }
        Self::from_flat_bytes(field, width, &bytes)
    }

    fn encoded(&self, row: RowId) -> Option<EncodedRef<'_>> {
        match self {
            ColumnImage::Bool(column) => column.get_ref(row).map(EncodedRef::One),
            ColumnImage::Fixed8(column) => column.get_ref(row).map(EncodedRef::Eight),
            ColumnImage::Fixed16(column) => column.get_ref(row).map(EncodedRef::Sixteen),
        }
    }

    /// Field ID stored by this column.
    pub fn field(&self) -> FieldId {
        match self {
            ColumnImage::Bool(column) => column.field(),
            ColumnImage::Fixed8(column) => column.field(),
            ColumnImage::Fixed16(column) => column.field(),
        }
    }

    /// Number of values in this column.
    pub fn len(&self) -> usize {
        match self {
            ColumnImage::Bool(column) => column.len(),
            ColumnImage::Fixed8(column) => column.len(),
            ColumnImage::Fixed16(column) => column.len(),
        }
    }

    /// True when this column has no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fixed encoded width of values in this column.
    pub fn width(&self) -> usize {
        match self {
            ColumnImage::Bool(_) => 1,
            ColumnImage::Fixed8(_) => 8,
            ColumnImage::Fixed16(_) => 16,
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            ColumnImage::Bool(column) => column.len(),
            ColumnImage::Fixed8(column) => column.len() * 8,
            ColumnImage::Fixed16(column) => column.len() * 16,
        }
    }

    #[cfg(test)]
    fn hash_into(&self, hasher: &mut blake3::Hasher) {
        match self {
            ColumnImage::Bool(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed8(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed16(column) => {
                for value in &column.values {
                    hasher.update(value);
                }
            }
        }
    }
}

fn exact_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::corrupt("query image column width mismatch"))
}

/// Cache of immutable query images by schema fingerprint and storage tx id.
#[derive(Default)]
pub struct QueryImageCache {
    images: RwLock<BTreeMap<QueryImageKey, Arc<QueryImage>>>,
    static_empty_fast: RwLock<BTreeSet<QueryShapeKey>>,
    hits: AtomicU64,
    misses: AtomicU64,
    builds: AtomicU64,
    build_micros: AtomicU64,
}

/// Query image cache diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryImageCacheDiagnostics {
    /// Number of cached image entries.
    pub cached_images: usize,
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Images built and inserted.
    pub builds: u64,
    /// Total image build time in microseconds.
    pub build_micros: u64,
}

impl QueryImageCache {
    /// Returns an existing image for the read snapshot, or builds and caches one.
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<Arc<QueryImage>> {
        self.get_or_build_scoped(txn, schema, QueryImageScope::full(schema))
    }

    /// Returns an existing scoped image for the read snapshot, or builds and caches one.
    pub fn get_or_build_scoped(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
        scope: QueryImageScope,
    ) -> Result<Arc<QueryImage>> {
        let key = QueryImageKey {
            schema: schema.descriptor().fingerprint(),
            tx_id: txn.last_committed_tx_id()?,
            scope: scope.key(),
        };
        if let Some(image) = self
            .images
            .read()
            .map_err(|_| Error::internal("query image cache read lock poisoned"))?
            .get(&key)
            .cloned()
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(image);
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();
        let image = Arc::new(QueryImageBuilder::new(txn, schema, scope).build()?);
        let elapsed = start.elapsed().as_micros() as u64;
        let mut images = self
            .images
            .write()
            .map_err(|_| Error::internal("query image cache write lock poisoned"))?;
        if let Some(existing) = images.get(&key).cloned() {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(existing);
        }
        images.insert(key, image.clone());
        self.builds.fetch_add(1, Ordering::Relaxed);
        self.build_micros.fetch_add(elapsed, Ordering::Relaxed);
        Ok(image)
    }

    /// Returns current query image cache diagnostics.
    pub fn diagnostics(&self) -> QueryImageCacheDiagnostics {
        QueryImageCacheDiagnostics {
            cached_images: self.images.read().map_or(0, |images| images.len()),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            builds: self.builds.load(Ordering::Relaxed),
            build_micros: self.build_micros.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn static_empty_fast_cached(&self, key: QueryShapeKey) -> Result<bool> {
        Ok(self
            .static_empty_fast
            .read()
            .map_err(|_| Error::internal("static-empty fast cache read lock poisoned"))?
            .contains(&key))
    }

    pub(crate) fn insert_static_empty_fast(&self, key: QueryShapeKey) -> Result<()> {
        self.static_empty_fast
            .write()
            .map_err(|_| Error::internal("static-empty fast cache write lock poisoned"))?
            .insert(key);
        Ok(())
    }
}

/// Builder for immutable query images.
pub struct QueryImageBuilder<'a, 'env> {
    txn: &'a ReadTxn<'env>,
    schema: &'a StorageSchema,
    scope: QueryImageScope,
}

impl<'a, 'env> QueryImageBuilder<'a, 'env> {
    /// Creates a builder over one read snapshot.
    pub fn new(txn: &'a ReadTxn<'env>, schema: &'a StorageSchema, scope: QueryImageScope) -> Self {
        Self { txn, schema, scope }
    }

    /// Builds the query image.
    pub fn build(self) -> Result<QueryImage> {
        let _span = tracing::debug_span!("bumbledb.query_image.build").entered();
        let start = Instant::now();
        let tx_id = self.txn.last_committed_tx_id()?;
        let mut relations = BTreeMap::new();
        let mut segment_count = 0usize;
        let mut segment_bytes = 0usize;
        let mut built_from_segments = true;
        for relation_id in self.scope.relation_ids() {
            let relation = self
                .schema
                .descriptor()
                .relations
                .get(relation_id.0 as usize)
                .ok_or_else(|| Error::internal("query image scope relation out of bounds"))?;
            let relation_scope = self
                .scope
                .relation_scope(relation_id)
                .ok_or_else(|| Error::internal("query image relation scope missing"))?;
            let built = RelationImageBuilder::new(
                self.txn,
                self.schema,
                relation_id,
                relation,
                relation_scope.clone(),
            )
            .build()?;
            segment_count += usize::from(built.from_segment);
            segment_bytes += built.segment_bytes;
            built_from_segments &= built.from_segment;
            relations.insert(relation_id, built.relation);
        }
        Ok(QueryImage::new(
            self.schema,
            tx_id,
            self.scope,
            relations,
            start.elapsed().as_micros(),
            segment_count,
            segment_bytes,
            built_from_segments,
        ))
    }
}

struct BuiltRelationImage {
    relation: RelationImage,
    from_segment: bool,
    segment_bytes: usize,
}

struct RelationImageBuilder<'a, 'env, 'schema> {
    txn: &'a ReadTxn<'env>,
    schema: &'schema StorageSchema,
    relation_id: RelationId,
    relation: &'schema RelationDescriptor,
    scope: RelationScope,
}

impl<'a, 'env, 'schema> RelationImageBuilder<'a, 'env, 'schema> {
    fn new(
        txn: &'a ReadTxn<'env>,
        schema: &'schema StorageSchema,
        relation_id: RelationId,
        relation: &'schema RelationDescriptor,
        scope: RelationScope,
    ) -> Self {
        Self {
            txn,
            schema,
            relation_id,
            relation,
            scope,
        }
    }

    fn build(self) -> Result<BuiltRelationImage> {
        let _span = tracing::trace_span!(
            "bumbledb.query_image.relation",
            relation = %self.relation.name,
        )
        .entered();
        if let Some(segment) =
            self.txn
                .visible_relation_segment(self.schema, self.relation_id, self.relation)?
        {
            return self.build_from_segment(&segment);
        }

        self.build_from_current_index()
    }

    fn build_from_segment(self, segment: &SegmentDescriptor) -> Result<BuiltRelationImage> {
        let fields = self.field_images();
        let mut segment_bytes = 0usize;
        let columns = fields
            .iter()
            .map(|field| {
                let descriptor = segment
                    .columns
                    .iter()
                    .find(|column| column.field == field.id)
                    .ok_or_else(|| Error::corrupt("segment column descriptor missing"))?;
                let bytes = self.txn.segment_bytes(&descriptor.lmdb_key)?;
                segment_bytes += bytes.len();
                ColumnImage::from_segment_bytes(field.id, field.width, bytes)
            })
            .collect::<Result<Vec<_>>>()?;
        let indexes = segment
            .indexes
            .iter()
            .filter(|index| {
                self.scope.include_all_indexes || self.scope.indexes.contains(&index.access)
            })
            .map(|index| {
                let bytes = self.txn.segment_bytes(&index.lmdb_key)?;
                let layout = self
                    .schema
                    .layouts()
                    .iter()
                    .find(|layout| {
                        layout.relation_id == self.relation_id.0
                            && layout.index_id == index.access.0
                    })
                    .ok_or_else(|| Error::unknown_index(&self.relation.name, "segment"))?;
                let prefix_len = layout.encoded_len
                    - layout
                        .components
                        .iter()
                        .map(|component| component.encoded_width)
                        .sum::<usize>();
                let mut offset = prefix_len;
                let components = layout
                    .components
                    .iter()
                    .map(|component| {
                        let Some(field) = self
                            .relation
                            .fields
                            .iter()
                            .position(|field| field.name == component.field_name)
                            .map(|field| FieldId(field as u16))
                        else {
                            return Ok(None);
                        };
                        let image_component = RelationIndexComponent {
                            field,
                            offset,
                            width: component.encoded_width,
                        };
                        offset += component.encoded_width;
                        Ok(Some(image_component))
                    })
                    .collect::<Result<Option<Vec<_>>>>()?;
                let Some(components) = components else {
                    return Ok(None);
                };
                segment_bytes += bytes.len();
                Ok(Some(RelationIndexImage {
                    access: index.access,
                    fields: index.fields.clone(),
                    components,
                    encoded_len: layout.encoded_len,
                    prefix_len,
                    bytes,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let encoded_column_bytes = columns.iter().map(ColumnImage::byte_len).sum();
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                row_count: segment.row_count,
                fields,
                columns,
                indexes,
                sorted_index_count: segment.indexes.len(),
                hash_index_count: 0,
                stats: RelationStats {
                    row_count: segment.row_count,
                    field_count: self.relation.fields.len(),
                    encoded_column_bytes,
                },
            },
            from_segment: true,
            segment_bytes,
        })
    }

    fn build_from_current_index(self) -> Result<BuiltRelationImage> {
        let fields = self.field_images();
        let mut builders = encoded_column_builders(&fields, 0)?;
        let layout = self
            .schema
            .layout(&self.relation.name, "primary")
            .ok_or_else(|| Error::unknown_index(&self.relation.name, "primary"))?;
        let component_by_field = layout
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (component.field_name.as_str(), index))
            .collect::<BTreeMap<_, _>>();

        let scan =
            self.txn
                .scan_encoded_index_prefix(self.schema, &self.relation.name, "primary", &[])?;
        for item in scan {
            let item = item?;
            for (field_id, field) in self.relation.fields.iter().enumerate() {
                let component_index = *component_by_field
                    .get(field.name.as_str())
                    .ok_or_else(|| Error::corrupt("query image missing primary index component"))?;
                let bytes = item
                    .component(&layout.components, component_index)
                    .ok_or_else(|| Error::corrupt("query image primary index component missing"))?;
                builders[field_id].append_bytes(bytes)?;
            }
        }

        let row_count = builders.first().map_or(0, EncodedColumnBuilder::len);
        let columns = finish_column_builders(builders);
        let encoded_column_bytes = columns.iter().map(ColumnImage::byte_len).sum();
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                row_count,
                fields,
                columns,
                indexes: Vec::new(),
                sorted_index_count: 0,
                hash_index_count: 0,
                stats: RelationStats {
                    row_count,
                    field_count: self.relation.fields.len(),
                    encoded_column_bytes,
                },
            },
            from_segment: false,
            segment_bytes: 0,
        })
    }

    fn field_images(&self) -> Vec<FieldImage> {
        self.relation
            .fields
            .iter()
            .enumerate()
            .filter(|(field_id, _)| {
                self.scope.include_all_columns
                    || self.scope.columns.contains(&FieldId(*field_id as u16))
            })
            .map(|(field_id, field)| FieldImage {
                id: FieldId(field_id as u16),
                name: field.name.clone(),
                value_type: field.value_type.clone(),
                width: field.value_type.encoded_width(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bumbledb_core::schema::{
        FieldDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind, SchemaDescriptor,
        ValueType,
    };

    use super::*;
    use crate::{AccessId, Environment, KeyValues, Row, Value};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn encoded_column_builder_appends_width_1() -> TestResult {
        let mut builder = EncodedColumnBuilder::new(FieldId(2), 1)?;
        builder.append_bytes(&[1])?;
        builder.append_bytes(&[0])?;

        assert_eq!(builder.len(), 2);
        assert!(!builder.is_empty());
        assert_eq!(builder.byte_len(), 2);
        match builder.finish() {
            ColumnImage::Bool(column) => {
                assert_eq!(column.field(), FieldId(2));
                assert_eq!(column.get(RowId(0)), Some([1]));
                assert_eq!(column.get(RowId(1)), Some([0]));
            }
            other => return Err(format!("expected bool column, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn encoded_column_builder_appends_width_8() -> TestResult {
        let mut builder = EncodedColumnBuilder::with_capacity(FieldId(1), 8, 2)?;
        builder.append_encoded_owned(&EncodedOwned::Eight(7u64.to_be_bytes()))?;
        builder.append_encoded_ref(EncodedRef::Eight(&9u64.to_be_bytes()))?;

        assert_eq!(builder.len(), 2);
        assert_eq!(builder.byte_len(), 16);
        match builder.finish() {
            ColumnImage::Fixed8(column) => {
                assert_eq!(column.field(), FieldId(1));
                assert_eq!(column.get(RowId(0)), Some(7u64.to_be_bytes()));
                assert_eq!(column.get(RowId(1)), Some(9u64.to_be_bytes()));
            }
            other => return Err(format!("expected fixed8 column, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn encoded_column_builder_appends_width_16() -> TestResult {
        let mut builder = EncodedColumnBuilder::new(FieldId(3), 16)?;
        builder.append_bytes(&1u128.to_be_bytes())?;
        builder.append_bytes(&2u128.to_be_bytes())?;

        assert_eq!(builder.len(), 2);
        assert_eq!(builder.byte_len(), 32);
        match builder.finish() {
            ColumnImage::Fixed16(column) => {
                assert_eq!(column.field(), FieldId(3));
                assert_eq!(column.get(RowId(0)), Some(1u128.to_be_bytes()));
                assert_eq!(column.get(RowId(1)), Some(2u128.to_be_bytes()));
            }
            other => return Err(format!("expected fixed16 column, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn encoded_column_builder_extends_flat_bytes() -> TestResult {
        let bytes = [3u64.to_be_bytes(), 4u64.to_be_bytes()].concat();
        let column = ColumnImage::from_flat_bytes(FieldId(0), 8, &bytes)?;

        match column {
            ColumnImage::Fixed8(column) => {
                assert_eq!(column.len(), 2);
                assert_eq!(column.get(RowId(0)), Some(3u64.to_be_bytes()));
                assert_eq!(column.get(RowId(1)), Some(4u64.to_be_bytes()));
            }
            other => return Err(format!("expected fixed8 column, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn encoded_column_builder_rejects_bad_width() {
        assert!(EncodedColumnBuilder::new(FieldId(0), 4).is_err());
    }

    #[test]
    fn encoded_column_builder_rejects_bad_flat_length() -> TestResult {
        let mut builder = EncodedColumnBuilder::new(FieldId(0), 8)?;

        assert!(builder.extend_flat_bytes(&[1, 2, 3]).is_err());
        assert!(builder.append_bytes(&[1, 2, 3]).is_err());
        Ok(())
    }

    #[test]
    fn column_image_rejects_segment_length_mismatch() {
        assert!(ColumnImage::from_segment_bytes(FieldId(0), 8, vec![1, 2, 3]).is_err());
    }

    #[test]
    fn column_image_accepts_empty_flat_bytes() -> TestResult {
        let column = ColumnImage::from_flat_bytes(FieldId(0), 8, &[])?;

        assert!(column.is_empty());
        assert_eq!(column.len(), 0);
        Ok(())
    }

    #[test]
    fn relation_index_prefix_count_matches_iterator() {
        let index = prefix_count_test_index([1, 1, 2, 4]);
        let one = 1u64.to_be_bytes();
        let two = 2u64.to_be_bytes();
        let three = 3u64.to_be_bytes();
        let zero = 0u64.to_be_bytes();
        let five = 5u64.to_be_bytes();

        assert_eq!(index.prefix_range(&one), 0..2);
        assert_eq!(index.prefix_count(&one), 2);
        assert!(index.prefix_exists(&one));
        assert_eq!(index.entries_with_prefix(&one).count(), 2);

        assert_eq!(index.prefix_range(&two), 2..3);
        assert_eq!(index.prefix_count(&two), 1);
        assert_eq!(index.entries_with_prefix(&two).count(), 1);

        assert_eq!(index.prefix_count(&three), 0);
        assert_eq!(index.entries_with_prefix(&three).count(), 0);
        assert_eq!(index.prefix_count(&zero), 0);
        assert_eq!(index.prefix_count(&five), 0);
        assert_eq!(index.prefix_count(&[]), 4);
        assert_eq!(index.entries_with_prefix(&[]).count(), 4);
        assert_eq!(index.entry_at(2), Some(two.as_slice()));
    }

    #[test]
    fn scoped_query_image_key_and_relations_are_explicit() -> TestResult {
        let dir = tempfile::tempdir().map_err(|error| crate::Error::io("tempdir", error))?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(two_relation_schema(), env.max_key_size())?;

        let scoped = env.read(|txn| {
            env.query_images.get_or_build_scoped(
                txn,
                &schema,
                QueryImageScope::relations_all(&schema, [RelationId(0)]),
            )
        })?;
        let full = env.query_image(&schema)?;

        assert_ne!(scoped.key().scope, full.key().scope);
        assert_eq!(scoped.stats().relation_count, 1);
        assert!(scoped.relation("Account").is_some());
        assert!(scoped.relation("Audit").is_none());
        assert_eq!(full.stats().relation_count, 2);
        assert!(full.relation("Audit").is_some());
        Ok(())
    }

    fn prefix_count_test_index(values: impl IntoIterator<Item = u64>) -> RelationIndexImage {
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        RelationIndexImage {
            access: AccessId(0),
            fields: vec![FieldId(0)],
            components: vec![RelationIndexComponent {
                field: FieldId(0),
                offset: 0,
                width: 8,
            }],
            encoded_len: 8,
            prefix_len: 0,
            bytes,
        }
    }

    #[test]
    fn builds_query_image_from_snapshot_and_matches_diagnostics() -> TestResult {
        let (env, schema) = seeded_env()?;

        let image = env.query_image(&schema)?;
        let diagnostics = env.storage_diagnostics(&schema)?;

        assert_eq!(image.stats().relation_count, 1);
        assert_eq!(image.stats().row_count, 2);
        assert!(image.stats().sorted_trie_bytes > 0);
        assert_eq!(image.stats().hash_trie_bytes, 0);
        assert_eq!(image.stats().segment_count, 1);
        assert!(image.stats().segment_bytes > 0);
        assert!(image.stats().built_from_segments);
        assert_eq!(diagnostics.relations[0].row_count, 2);

        let segments = env.read(|txn| txn.visible_segments(&schema))?;
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].row_count, 2);
        assert_eq!(segments[0].columns.len(), 5);
        assert_eq!(segments[0].columns[0].byte_len, 16);
        assert!(!segments[0].indexes.is_empty());

        let account = account_relation(&image)?;
        assert_eq!(account.row_count, 2);
        assert_eq!(account.fields.len(), 5);
        assert_eq!(account.encoded_column_bytes(), 2 * (8 + 8 + 1 + 8 + 8));
        assert_eq!(account.stats.row_count, account.row_count);
        assert_eq!(account.stats.field_count, account.fields.len());
        assert_eq!(
            account.stats.encoded_column_bytes,
            account.encoded_column_bytes()
        );
        Ok(())
    }

    #[test]
    fn relation_image_columns_expose_widths_and_stable_row_ids() -> TestResult {
        let (env, schema) = seeded_env()?;
        let image = env.query_image(&schema)?;
        let account = account_relation(&image)?;

        assert_eq!(
            account.all_rows(),
            RowRange {
                start: RowId(0),
                end: RowId(2)
            }
        );
        assert_eq!(field(account, FieldId(0))?.encoded_width(), 8);
        assert_eq!(field(account, FieldId(2))?.encoded_width(), 1);
        assert_eq!(column(account, FieldId(0))?.len(), 2);
        assert_eq!(column(account, FieldId(0))?.field(), FieldId(0));
        assert_eq!(column(account, FieldId(2))?.width(), 1);
        assert!(matches!(column(account, FieldId(2))?, ColumnImage::Bool(_)));

        assert_eq!(
            encoded_bytes(account, RowId(0), FieldId(0))?,
            1u64.to_be_bytes().as_slice()
        );
        assert_eq!(
            encoded_bytes(account, RowId(1), FieldId(0))?,
            2u64.to_be_bytes().as_slice()
        );
        assert!(matches!(
            encoded(account, RowId(0), FieldId(2))?,
            EncodedRef::One(_)
        ));
        Ok(())
    }

    #[test]
    fn string_and_bytes_columns_store_intern_ids_not_raw_values() -> TestResult {
        let (env, schema) = seeded_env()?;
        let image = env.query_image(&schema)?;
        let account = account_relation(&image)?;

        let payload = encoded_bytes(account, RowId(0), FieldId(3))?;
        let name = encoded_bytes(account, RowId(0), FieldId(4))?;

        assert_eq!(payload.len(), 8);
        assert_eq!(name.len(), 8);
        assert_ne!(payload, &[1, 2, 3][..]);
        assert_ne!(name, b"Cash USD".as_slice());

        env.read(|txn| {
            assert_eq!(
                txn.decode_query_value(&field(account, FieldId(3))?.value_type, payload)?,
                Value::Bytes(vec![1, 2, 3])
            );
            assert_eq!(
                txn.decode_query_value(&field(account, FieldId(4))?.value_type, name)?,
                Value::String("Cash USD".to_owned())
            );
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn query_image_encoded_columns_decode_to_public_scan_rows() -> TestResult {
        let (env, schema) = seeded_env()?;
        let image = env.query_image(&schema)?;

        env.read(|txn| {
            let mut scanned = txn
                .scan_relation(&schema, "Account")?
                .map(|item| item.map(|item| item.row))
                .collect::<Result<Vec<_>>>()?;
            let account = account_relation(&image)?;
            let mut imaged = decode_relation_rows(txn, account)?;
            scanned.sort();
            imaged.sort();
            assert_eq!(imaged, scanned);
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn query_image_build_is_deterministic_for_same_snapshot() -> TestResult {
        let (env, schema) = seeded_env()?;

        env.read(|txn| {
            let left =
                QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
            let right =
                QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
            assert_eq!(left.content_fingerprint(), right.content_fingerprint());
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn bulk_loaded_query_image_exposes_segment_index_images() -> TestResult {
        let dir = tempfile::tempdir().map_err(|error| crate::Error::io("tempdir", error))?;
        let path = dir.keep();
        let env = Environment::open(&path)?;
        let schema = StorageSchema::new(account_schema(true), env.max_key_size())?;
        env.bulk_load(
            &schema,
            vec![
                account_row(1, 840, true, vec![1, 2, 3], "Cash USD"),
                account_row(2, 978, false, vec![4, 5, 6], "Cash EUR"),
            ],
        )?;

        let image = env.query_image(&schema)?;
        let account = account_relation(&image)?;

        assert!(!account.indexes().is_empty());
        let primary = account
            .indexes()
            .iter()
            .find(|index| index.fields == vec![FieldId(0)])
            .ok_or_else(|| crate::Error::internal("missing primary segment index image"))?;
        assert_eq!(primary.bytes.len(), primary.encoded_len * account.row_count);
        Ok(())
    }

    #[test]
    fn query_image_cache_hits_until_transaction_id_changes() -> TestResult {
        let (env, schema) = seeded_env()?;

        let first = env.query_image(&schema)?;
        let second = env.query_image(&schema)?;
        assert!(Arc::ptr_eq(&first, &second));

        env.write(|txn| {
            txn.insert(
                &schema,
                Row::new(
                    "Account",
                    [
                        ("id", Value::Id(3)),
                        ("currency", Value::Enum(826)),
                        ("active", Value::Bool(true)),
                        ("payload", Value::Bytes(vec![7, 8, 9])),
                        ("name", Value::String("Cash GBP".to_owned())),
                    ],
                ),
            )?;
            Ok::<_, crate::Error>(())
        })?;

        let third = env.query_image(&schema)?;
        assert!(!Arc::ptr_eq(&first, &third));
        assert!(third.key().tx_id > first.key().tx_id);
        assert_eq!(account_relation(&third)?.row_count, 3);
        Ok(())
    }

    #[test]
    fn reopened_query_image_uses_durable_segments() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.keep();
        let env = Environment::open(&path)?;
        let schema = StorageSchema::new(account_schema(true), env.max_key_size())?;
        env.bulk_load(
            &schema,
            [
                account_row(1, 840, true, vec![1, 2, 3], "Cash USD"),
                account_row(2, 978, false, vec![4, 5, 6], "Cash EUR"),
            ],
        )?;
        drop(env);

        let reopened = Environment::open(&path)?;
        let image = reopened.query_image(&schema)?;

        assert!(image.stats().built_from_segments);
        assert_eq!(image.stats().segment_count, 1);
        assert_eq!(account_relation(&image)?.row_count, 2);
        Ok(())
    }

    #[test]
    fn read_snapshot_sees_stable_visible_segments() -> TestResult {
        let (env, schema) = seeded_env()?;

        env.read(|read| {
            let before = read.visible_segments(&schema)?;
            assert_eq!(before[0].row_count, 2);

            env.write(|write| {
                write.insert(
                    &schema,
                    account_row(3, 826, true, vec![7, 8, 9], "Cash GBP"),
                )?;
                Ok::<_, crate::Error>(())
            })?;

            let still_before = read.visible_segments(&schema)?;
            assert_eq!(still_before[0].row_count, 2);
            let image =
                QueryImageBuilder::new(read, &schema, QueryImageScope::full(&schema)).build()?;
            assert_eq!(account_relation(&image)?.row_count, 2);
            Ok::<_, crate::Error>(())
        })?;

        let after = env.read(|read| read.visible_segments(&schema))?;
        assert_eq!(after[0].row_count, 3);
        Ok(())
    }

    #[test]
    fn replace_and_delete_publish_visible_segments() -> TestResult {
        let (env, schema) = seeded_env()?;

        env.write(|txn| {
            txn.replace(
                &schema,
                account_row(2, 826, true, vec![9, 9, 9], "Cash GBP"),
            )?;
            txn.delete(&schema, KeyValues::new("Account", [("id", Value::Id(1))]))?;
            Ok::<_, crate::Error>(())
        })?;

        let image = env.query_image(&schema)?;
        let account = account_relation(&image)?;
        assert!(image.stats().built_from_segments);
        assert_eq!(account.row_count, 1);

        env.read(|txn| {
            let rows = decode_relation_rows(txn, account)?;
            assert_eq!(
                rows,
                vec![account_row(2, 826, true, vec![9, 9, 9], "Cash GBP")]
            );
            let segments = txn.visible_segments(&schema)?;
            assert_eq!(segments[0].row_count, 1);
            assert!(segments[0].tx_end.is_none());
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }

    #[test]
    fn query_image_cache_does_not_reuse_mismatched_schema() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.keep();
        let env = Environment::open(&path)?;
        let schema_a = StorageSchema::new(account_schema(false), env.max_key_size())?;
        let schema_b = StorageSchema::new(account_schema(true), env.max_key_size())?;

        let image_a = env.query_image(&schema_a)?;
        let image_b = env.query_image(&schema_b)?;

        assert_ne!(image_a.key().schema, image_b.key().schema);
        assert!(!Arc::ptr_eq(&image_a, &image_b));
        Ok(())
    }

    fn seeded_env() -> Result<(Environment, StorageSchema)> {
        let dir = tempfile::tempdir().map_err(|error| crate::Error::io("tempdir", error))?;
        let path = dir.keep();
        let env = Environment::open(&path)?;
        let schema = StorageSchema::new(account_schema(true), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(
                &schema,
                account_row(1, 840, true, vec![1, 2, 3], "Cash USD"),
            )?;
            txn.insert(
                &schema,
                account_row(2, 978, false, vec![4, 5, 6], "Cash EUR"),
            )?;
            Ok::<_, crate::Error>(())
        })?;
        Ok((env, schema))
    }

    fn account_relation(image: &QueryImage) -> Result<&RelationImage> {
        image
            .relation("Account")
            .ok_or_else(|| crate::Error::internal("missing Account relation"))
    }

    fn field(relation: &RelationImage, field: FieldId) -> Result<&FieldImage> {
        relation
            .field(field)
            .ok_or_else(|| crate::Error::internal(format!("missing field {}", field.0)))
    }

    fn column(relation: &RelationImage, field: FieldId) -> Result<&ColumnImage> {
        relation
            .column(field)
            .ok_or_else(|| crate::Error::internal(format!("missing column {}", field.0)))
    }

    fn encoded<'a>(
        relation: &'a RelationImage,
        row: RowId,
        field: FieldId,
    ) -> Result<EncodedRef<'a>> {
        relation.encoded(row, field).ok_or_else(|| {
            crate::Error::internal(format!(
                "missing encoded value row={} field={}",
                row.0, field.0
            ))
        })
    }

    fn encoded_bytes(relation: &RelationImage, row: RowId, field: FieldId) -> Result<&[u8]> {
        relation.encoded_bytes(row, field).ok_or_else(|| {
            crate::Error::internal(format!(
                "missing encoded bytes row={} field={}",
                row.0, field.0
            ))
        })
    }

    fn account_schema(with_name: bool) -> SchemaDescriptor {
        let mut fields = vec![
            FieldDescriptor::new(
                "id",
                ValueType::Id {
                    name: "AccountId".to_owned(),
                    relation: "Account".to_owned(),
                },
            ),
            FieldDescriptor::new(
                "currency",
                ValueType::Enum {
                    name: "Currency".to_owned(),
                },
            ),
            FieldDescriptor::new("active", ValueType::Bool),
            FieldDescriptor::new("payload", ValueType::Bytes),
        ];
        if with_name {
            fields.push(FieldDescriptor::new("name", ValueType::String));
        }
        SchemaDescriptor::new(
            "Accounts",
            vec![RelationDescriptor::new(
                "Account",
                RelationKind::Entity,
                fields,
                PrimaryKeyDescriptor::new(["id"]),
            )],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [826, 840, 978],
        ))
    }

    fn two_relation_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "ScopedAccounts",
            vec![
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![FieldDescriptor::new("id", ValueType::U64)],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Audit",
                    RelationKind::Event,
                    vec![FieldDescriptor::new("id", ValueType::U64)],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
            ],
        )
    }

    fn account_row(id: u64, currency: u64, active: bool, payload: Vec<u8>, name: &str) -> Row {
        Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("currency", Value::Enum(currency)),
                ("active", Value::Bool(active)),
                ("payload", Value::Bytes(payload)),
                ("name", Value::String(name.to_owned())),
            ],
        )
    }

    fn decode_relation_rows(txn: &ReadTxn<'_>, relation: &RelationImage) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        for row in 0..relation.row_count {
            let row = RowId(row as u32);
            let values = relation
                .fields
                .iter()
                .map(|field| {
                    let bytes = relation
                        .encoded(row, field.id)
                        .ok_or_else(|| Error::internal("missing query image field"))?;
                    Ok((
                        field.name.clone(),
                        txn.decode_query_value(&field.value_type, bytes.as_bytes())?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            rows.push(Row::new(relation.name.clone(), values));
        }
        Ok(rows)
    }
}
