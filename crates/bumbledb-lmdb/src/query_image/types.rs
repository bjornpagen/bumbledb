use std::collections::BTreeMap;

use bumbledb_core::schema::ValueType;

use crate::planner_stats::{PlannerStatsCache, PlannerStatsCacheDiagnostics};
use crate::query_image::columns::ColumnImage;
use crate::query_image::{QueryImageKey, QueryImageScope, RelationIndexImage};
use crate::{Result, StorageSchema};

/// Dense relation ID in schema declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u16);

/// Dense field ID in relation declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub u16);

/// Dense fact ID inside a relation image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId(pub u32);

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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "query image stats are retained for tests")
    )]
    stats: QueryImageStats,
    planner_stats: PlannerStatsCache,
}

impl QueryImage {
    pub(super) fn new(
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
                build_micros,
            },
            planner_stats: PlannerStatsCache::default(),
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
    #[cfg(test)]
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
    ) -> Result<std::sync::Arc<crate::planner_stats::PlannerRelationStats>> {
        self.planner_stats.get_or_build(schema, relation)
    }

    #[cfg(test)]
    pub(super) fn content_fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.key.schema.0);
        for relation in self.relations.values() {
            hasher.update(relation.name.as_bytes());
            hasher.update(&(relation.fact_count as u64).to_be_bytes());
            for field in &relation.fields {
                hasher.update(field.name.as_bytes());
                hasher.update(&(field.width as u64).to_be_bytes());
            }
            for column in relation.columns.values() {
                column.hash_into(&mut hasher);
            }
        }
        *hasher.finalize().as_bytes()
    }
}

/// Query image build/cache statistics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueryImageStats {
    /// Number of relation images.
    pub relation_count: usize,
    /// Total fact count across all relations.
    pub fact_count: usize,
    /// Encoded column bytes stored in relation images.
    pub encoded_column_bytes: usize,
    /// Encoded access-key bytes stored in relation images.
    pub access_key_bytes: usize,
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
    pub columns: BTreeMap<FieldId, ColumnImage>,
    /// Durable sorted index images in access-path order when available.
    pub indexes: Vec<RelationIndexImage>,
    /// Relation image statistics.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "relation stats are retained for diagnostics")
    )]
    pub stats: RelationStats,
}

impl RelationImage {
    /// Returns the encoded value for `fact` and `field`.
    pub(crate) fn encoded(&self, fact: FactId, field: FieldId) -> Option<EncodedRef<'_>> {
        self.columns.get(&field)?.encoded(fact)
    }

    /// Returns the encoded bytes for `fact` and `field`.
    #[cfg(test)]
    pub(crate) fn encoded_bytes(&self, fact: FactId, field: FieldId) -> Option<&[u8]> {
        self.encoded(fact, field).map(EncodedRef::as_bytes)
    }

    /// Returns field metadata by ID.
    #[cfg(test)]
    pub fn field(&self, field: FieldId) -> Option<&FieldImage> {
        self.fields.iter().find(|candidate| candidate.id == field)
    }

    /// Returns durable sorted index images for this relation.
    pub fn indexes(&self) -> &[RelationIndexImage] {
        &self.indexes
    }

    /// Encoded column byte footprint.
    pub fn encoded_column_bytes(&self) -> usize {
        self.columns.values().map(ColumnImage::byte_len).sum()
    }

    /// Number of facts in this relation image.
    #[cfg(test)]
    pub fn relation_cardinality(&self) -> usize {
        self.fact_count
    }

    /// Looks up an access image by ID.
    #[cfg(test)]
    pub fn access(&self, access: crate::AccessId) -> Option<&RelationIndexImage> {
        self.indexes.iter().find(|index| index.access == access)
    }

    /// Returns true if an access prefix exists.
    #[cfg(test)]
    pub fn access_prefix_exists(&self, access: crate::AccessId, prefix: &[u8]) -> bool {
        self.access(access)
            .is_some_and(|index| index.prefix_exists(prefix))
    }

    /// Returns the fact cardinality under an access prefix.
    #[cfg(test)]
    pub fn access_prefix_cardinality(&self, access: crate::AccessId, prefix: &[u8]) -> usize {
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
