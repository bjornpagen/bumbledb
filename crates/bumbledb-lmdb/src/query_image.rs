use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use bumbledb_core::schema::{RelationDescriptor, SchemaFingerprint, ValueType};

use crate::planner_stats::{PlannerStatsCache, PlannerStatsCacheDiagnostics};
use crate::query::ExecutionPlan;
use crate::storage_schema::FACT_SET_ACCESS_NAME;
use crate::{
    AccessId, EncodedOwned, Error, IndexSpec, ReadTxn, Result, SortedTrieIndex, StorageSchema,
};

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

/// Dense fact ID inside a relation image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId(pub u32);

/// Half-open fact-id range inside a relation image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactRange {
    /// Inclusive start fact id.
    pub start: FactId,
    /// Exclusive end fact id.
    pub end: FactId,
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

/// Immutable snapshot-local image used by the query runtime.
#[derive(Clone, Debug)]
pub struct QueryImage {
    #[cfg_attr(not(test), expect(dead_code, reason = "query image key is diagnostic"))]
    key: QueryImageKey,
    relations: BTreeMap<RelationId, RelationImage>,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "name lookup is used by tests/diagnostics")
    )]
    relation_by_name: BTreeMap<String, RelationId>,
    stats: QueryImageStats,
    planner_stats: PlannerStatsCache,
    prepared_plans: PreparedPlanCache,
    sorted_trie_cache: Arc<RwLock<BTreeMap<LftjAtomKey, Arc<SortedTrieIndex>>>>,
}

impl QueryImage {
    fn new(
        schema: &StorageSchema,
        tx_id: u64,
        scope: QueryImageScope,
        relations: BTreeMap<RelationId, RelationImage>,
        build_micros: u128,
    ) -> Self {
        let relation_by_name = relations
            .values()
            .map(|relation| (relation.name.clone(), relation.id))
            .collect::<BTreeMap<_, _>>();
        let relation_count = relations.len();
        let fact_count = relations.values().map(|relation| relation.fact_count).sum();
        let encoded_column_bytes = relations
            .values()
            .map(RelationImage::encoded_column_bytes)
            .sum();
        let access_key_bytes = relations
            .values()
            .map(RelationImage::access_key_bytes)
            .sum();
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
                fact_count,
                encoded_column_bytes,
                access_key_bytes,
                sorted_trie_bytes: 0,
                build_micros,
            },
            planner_stats: PlannerStatsCache::default(),
            prepared_plans: PreparedPlanCache::default(),
            sorted_trie_cache: Arc::default(),
        }
    }

    /// Returns this image's cache key.
    #[cfg(test)]
    pub fn key(&self) -> QueryImageKey {
        self.key.clone()
    }

    /// Looks up a loaded relation image by ID.
    pub fn relation_by_id(&self, id: RelationId) -> Option<&RelationImage> {
        self.relations.get(&id)
    }

    /// Looks up a relation image by name.
    #[cfg(test)]
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
                source_facts_scanned: 0,
                facts_retained: 0,
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
                source_facts_scanned: 0,
                facts_retained: 0,
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
            source_facts_scanned: built.source_facts_scanned,
            facts_retained: built.facts_retained,
            bytes_copied: built.bytes_copied,
            scan_micros: built.scan_micros,
            column_micros: built.column_micros,
            sort_micros: built.sort_micros,
        })
    }

    #[cfg(test)]
    fn content_fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.key.schema.0);
        for relation in self.relations.values() {
            hasher.update(relation.name.as_bytes());
            hasher.update(&(relation.fact_count as u64).to_be_bytes());
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
    pub source_facts_scanned: u64,
    pub facts_retained: u64,
    pub bytes_copied: u64,
    pub scan_micros: u64,
    pub column_micros: u64,
    pub sort_micros: u64,
}

pub(crate) struct SortedTrieBuild {
    pub index: SortedTrieIndex,
    pub source_facts_scanned: u64,
    pub facts_retained: u64,
    pub bytes_copied: u64,
    pub scan_micros: u64,
    pub column_micros: u64,
    pub sort_micros: u64,
}

pub(crate) fn build_sorted_trie_index(
    relation: &RelationImage,
    spec: IndexSpec,
) -> Result<SortedTrieIndex> {
    SortedTrieIndex::build(relation, spec)
}

/// Query image build/cache statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryImageStats {
    /// Number of relation images.
    pub relation_count: usize,
    /// Total fact count across all relations.
    pub fact_count: usize,
    /// Encoded column bytes stored in relation images.
    pub encoded_column_bytes: usize,
    /// Encoded access-key bytes stored in relation images.
    pub access_key_bytes: usize,
    /// Bytes used by cached sorted trie indexes.
    pub sorted_trie_bytes: usize,
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
    /// Number of facts in this image.
    pub fact_count: usize,
    /// Field metadata in declaration order.
    pub fields: Vec<FieldImage>,
    /// Encoded columns in declaration order.
    pub columns: Vec<ColumnImage>,
    /// Durable sorted index images in access-path order when available.
    pub indexes: Vec<RelationIndexImage>,
    /// Relation image statistics.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "relation stats are retained for diagnostics")
    )]
    pub stats: RelationStats,
}

/// Immutable durable sorted index bytes for one relation image.
#[derive(Clone, Debug)]
pub struct RelationIndexImage {
    /// Dense storage access ID.
    pub access: AccessId,
    /// Leading fields in access-path order.
    pub fields: Vec<FieldId>,
    /// Encoded key components in access order.
    pub components: Vec<RelationAccessComponent>,
    /// Bytes per encoded index entry.
    pub encoded_len: usize,
    /// Namespace/relation/access prefix bytes before components.
    pub prefix_len: usize,
    /// Concatenated encoded index entries.
    pub bytes: Vec<u8>,
}

/// One field component inside a durable relation index image.
#[derive(Clone, Debug)]
pub struct RelationAccessComponent {
    /// Field ID in relation declaration order.
    pub field: FieldId,
    /// Offset of this component inside an encoded index entry.
    pub offset: usize,
    /// Encoded component width.
    pub width: usize,
}

impl RelationIndexImage {
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
    #[cfg(test)]
    pub fn prefix_count(&self, prefix: &[u8]) -> usize {
        debug_assert!(prefix.len() <= self.encoded_len.saturating_sub(self.prefix_len));
        let range = self.prefix_range(prefix);
        range.end.saturating_sub(range.start)
    }

    /// Returns true when any encoded index entry matches a leading component prefix.
    #[cfg(test)]
    pub fn prefix_exists(&self, prefix: &[u8]) -> bool {
        let range = self.prefix_range(prefix);
        range.start < range.end
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
    /// Returns the encoded value for `fact` and `field`.
    pub(crate) fn encoded(&self, fact: FactId, field: FieldId) -> Option<EncodedRef<'_>> {
        self.columns.get(field.0 as usize)?.encoded(fact)
    }

    /// Returns the encoded bytes for `fact` and `field`.
    pub(crate) fn encoded_bytes(&self, fact: FactId, field: FieldId) -> Option<&[u8]> {
        self.encoded(fact, field).map(EncodedRef::as_bytes)
    }

    /// Returns field metadata by ID.
    #[cfg(test)]
    pub fn field(&self, field: FieldId) -> Option<&FieldImage> {
        self.fields.get(field.0 as usize)
    }

    /// Returns durable sorted index images for this relation.
    pub fn indexes(&self) -> &[RelationIndexImage] {
        &self.indexes
    }

    /// Encoded column byte footprint.
    pub fn encoded_column_bytes(&self) -> usize {
        self.columns.iter().map(ColumnImage::byte_len).sum()
    }

    /// Number of facts in this relation image.
    #[cfg(test)]
    pub fn relation_cardinality(&self) -> usize {
        self.fact_count
    }

    /// Looks up an access image by ID.
    #[cfg(test)]
    pub fn access(&self, access: AccessId) -> Option<&RelationIndexImage> {
        self.indexes.iter().find(|index| index.access == access)
    }

    /// Returns true if an access prefix exists.
    #[cfg(test)]
    pub fn access_prefix_exists(&self, access: AccessId, prefix: &[u8]) -> bool {
        self.access(access)
            .is_some_and(|index| index.prefix_exists(prefix))
    }

    /// Returns the fact cardinality under an access prefix.
    #[cfg(test)]
    pub fn access_prefix_cardinality(&self, access: AccessId, prefix: &[u8]) -> usize {
        self.access(access)
            .map_or(0, |index| index.prefix_count(prefix))
    }

    /// Encoded access-key byte footprint.
    pub fn access_key_bytes(&self) -> usize {
        self.indexes.iter().map(|index| index.bytes.len()).sum()
    }
}

/// Relation-level image statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationStats {
    /// Number of facts in the relation image.
    pub fact_count: usize,
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
    #[cfg(test)]
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
    #[cfg(test)]
    pub fn field(&self) -> FieldId {
        self.field
    }

    /// Number of encoded values in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl<T: Copy> FixedColumn<T> {
    /// Returns a copied value by fact ID.
    #[cfg(test)]
    #[inline]
    pub fn get(&self, fact: FactId) -> Option<T> {
        self.values.get(fact.0 as usize).copied()
    }

    /// Returns a borrowed value by fact ID.
    #[inline]
    pub fn get_ref(&self, fact: FactId) -> Option<&T> {
        self.values.get(fact.0 as usize)
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

    pub(crate) fn append_encoded_owned(&mut self, value: &EncodedOwned) -> Result<()> {
        self.append_bytes(value.as_bytes())
    }

    #[cfg(test)]
    pub(crate) fn extend_flat_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let width = self.width();
        if width == 0 || !bytes.len().is_multiple_of(width) {
            return Err(Error::corrupt("column byte width mismatch"));
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
    #[cfg(test)]
    pub(crate) fn from_flat_bytes(field: FieldId, width: usize, bytes: &[u8]) -> Result<Self> {
        let mut builder = EncodedColumnBuilder::with_capacity(field, width, bytes.len() / width)?;
        builder.extend_flat_bytes(bytes)?;
        Ok(builder.finish())
    }

    fn encoded(&self, fact: FactId) -> Option<EncodedRef<'_>> {
        match self {
            ColumnImage::Bool(column) => column.get_ref(fact).map(EncodedRef::One),
            ColumnImage::Fixed8(column) => column.get_ref(fact).map(EncodedRef::Eight),
            ColumnImage::Fixed16(column) => column.get_ref(fact).map(EncodedRef::Sixteen),
        }
    }

    /// Field ID stored by this column.
    #[cfg(test)]
    pub fn field(&self) -> FieldId {
        match self {
            ColumnImage::Bool(column) => column.field(),
            ColumnImage::Fixed8(column) => column.field(),
            ColumnImage::Fixed16(column) => column.field(),
        }
    }

    /// Number of values in this column.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        match self {
            ColumnImage::Bool(column) => column.len(),
            ColumnImage::Fixed8(column) => column.len(),
            ColumnImage::Fixed16(column) => column.len(),
        }
    }

    /// True when this column has no values.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fixed encoded width of values in this column.
    #[cfg(test)]
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
            relations.insert(relation_id, built.relation);
        }
        Ok(QueryImage::new(
            self.schema,
            tx_id,
            self.scope,
            relations,
            start.elapsed().as_micros(),
        ))
    }
}

struct BuiltRelationImage {
    relation: RelationImage,
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
        self.build_from_current_access()
    }

    fn build_from_current_access(self) -> Result<BuiltRelationImage> {
        let fields = self.field_images();
        let mut builders = encoded_column_builders(&fields, 0)?;
        let layout = self
            .schema
            .fact_set_layout(&self.relation.name)
            .ok_or_else(|| Error::unknown_index(&self.relation.name, FACT_SET_ACCESS_NAME))?;
        let component_by_field = layout
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (component.field_name.as_str(), index))
            .collect::<BTreeMap<_, _>>();

        let fact_set_access = self
            .schema
            .fact_set_index_name(&self.relation.name)
            .ok_or_else(|| Error::unknown_index(&self.relation.name, FACT_SET_ACCESS_NAME))?;
        let scan = self.txn.scan_encoded_access_prefix(
            self.schema,
            &self.relation.name,
            fact_set_access,
            &[],
        )?;
        for item in scan {
            let item = item?;
            for (field_id, field) in self.relation.fields.iter().enumerate() {
                let component_index = *component_by_field
                    .get(field.name.as_str())
                    .ok_or_else(|| Error::corrupt("query image missing access component"))?;
                let bytes = item
                    .component(&layout.components, component_index)
                    .ok_or_else(|| Error::corrupt("query image access component missing"))?;
                builders[field_id].append_bytes(bytes)?;
            }
        }

        let fact_count = builders.first().map_or(0, EncodedColumnBuilder::len);
        let columns = finish_column_builders(builders);
        let encoded_column_bytes = columns.iter().map(ColumnImage::byte_len).sum();
        let indexes = self
            .schema
            .layouts_for_relation(self.relation_id.0)
            .filter(|layout| {
                self.scope.include_all_indexes
                    || self.scope.indexes.contains(&AccessId(layout.index_id))
            })
            .map(|layout| {
                let mut bytes = Vec::new();
                let scan = self.txn.scan_encoded_access_prefix(
                    self.schema,
                    &self.relation.name,
                    &layout.index_name,
                    &[],
                )?;
                for item in scan {
                    bytes.extend_from_slice(item?.key());
                }
                let prefix_len = 1 + 2 + 2;
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
                        let image_component = RelationAccessComponent {
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
                Ok(Some(RelationIndexImage {
                    access: AccessId(layout.index_id),
                    fields: layout
                        .leading_fields
                        .iter()
                        .filter_map(|field| {
                            self.relation
                                .fields
                                .iter()
                                .position(|relation_field| relation_field.name == *field)
                                .map(|field_id| FieldId(field_id as u16))
                        })
                        .collect(),
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
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                fact_count,
                fields,
                columns,
                indexes,
                stats: RelationStats {
                    fact_count,
                    field_count: self.relation.fields.len(),
                    encoded_column_bytes,
                },
            },
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
#[path = "query_image_tests.rs"]
mod tests;
