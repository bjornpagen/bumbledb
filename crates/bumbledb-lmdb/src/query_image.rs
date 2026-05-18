use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use bumbledb_core::schema::{RelationDescriptor, SchemaFingerprint, ValueType};

use crate::{Error, ReadTxn, Result, SegmentDescriptor, StorageSchema};

/// Cache key for an immutable query image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueryImageKey {
    /// Schema fingerprint for the image.
    pub schema: SchemaFingerprint,
    /// Last committed storage transaction ID visible to the image.
    pub tx_id: u64,
}

impl PartialOrd for QueryImageKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueryImageKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.schema.0, self.tx_id).cmp(&(other.schema.0, other.tx_id))
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
    relations: Vec<RelationImage>,
    relation_by_name: BTreeMap<String, RelationId>,
    stats: QueryImageStats,
}

impl QueryImage {
    fn new(
        schema: &StorageSchema,
        tx_id: u64,
        relations: Vec<RelationImage>,
        build_micros: u128,
        segment_count: usize,
        segment_bytes: usize,
        built_from_segments: bool,
    ) -> Self {
        let relation_by_name = relations
            .iter()
            .map(|relation| (relation.name.clone(), relation.id))
            .collect::<BTreeMap<_, _>>();
        let row_count = relations.iter().map(|relation| relation.row_count).sum();
        let encoded_column_bytes = relations
            .iter()
            .map(RelationImage::encoded_column_bytes)
            .sum();
        Self {
            key: QueryImageKey {
                schema: schema.descriptor().fingerprint(),
                tx_id,
            },
            relations,
            relation_by_name,
            stats: QueryImageStats {
                relation_count: schema.descriptor().relations.len(),
                row_count,
                encoded_column_bytes,
                sorted_trie_bytes: 0,
                hash_trie_bytes: 0,
                segment_count,
                segment_bytes,
                built_from_segments,
                build_micros,
            },
        }
    }

    /// Returns this image's cache key.
    pub fn key(&self) -> QueryImageKey {
        self.key
    }

    /// Returns all relation images in schema declaration order.
    pub fn relations(&self) -> &[RelationImage] {
        &self.relations
    }

    /// Looks up a relation image by name.
    pub fn relation(&self, name: &str) -> Option<&RelationImage> {
        let id = self.relation_by_name.get(name)?;
        self.relations.get(id.0 as usize)
    }

    /// Returns memory/build statistics for this image.
    pub fn stats(&self) -> &QueryImageStats {
        &self.stats
    }

    #[cfg(test)]
    fn content_fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.key.schema.0);
        for relation in &self.relations {
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
    /// Placeholder count for sorted indexes built in PRD 03.
    pub sorted_index_count: usize,
    /// Placeholder count for hash indexes built in PRD 06.
    pub hash_index_count: usize,
    /// Relation image statistics.
    pub stats: RelationStats,
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
    fn from_bytes(field: FieldId, width: usize, values: Vec<Vec<u8>>) -> Result<Self> {
        Self::from_query_image_bytes(field, width, values)
    }

    pub(crate) fn from_query_image_bytes(
        field: FieldId,
        width: usize,
        values: Vec<Vec<u8>>,
    ) -> Result<Self> {
        Ok(match width {
            1 => ColumnImage::Bool(FixedColumn::new(
                field,
                values
                    .into_iter()
                    .map(|bytes| exact_array::<1>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            )),
            8 => ColumnImage::Fixed8(FixedColumn::new(
                field,
                values
                    .into_iter()
                    .map(|bytes| exact_array::<8>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            )),
            16 => ColumnImage::Fixed16(FixedColumn::new(
                field,
                values
                    .into_iter()
                    .map(|bytes| exact_array::<16>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            )),
            _ => return Err(Error::internal(format!("unsupported column width {width}"))),
        })
    }

    pub(crate) fn from_segment_bytes(field: FieldId, width: usize, bytes: Vec<u8>) -> Result<Self> {
        if width == 0 || !bytes.len().is_multiple_of(width) {
            return Err(Error::corrupt("segment column byte width mismatch"));
        }
        let values = bytes
            .chunks_exact(width)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>();
        Self::from_query_image_bytes(field, width, values)
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
}

impl QueryImageCache {
    /// Returns an existing image for the read snapshot, or builds and caches one.
    pub fn get_or_build(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<Arc<QueryImage>> {
        let key = QueryImageKey {
            schema: schema.descriptor().fingerprint(),
            tx_id: txn.last_committed_tx_id()?,
        };
        if let Some(image) = self
            .images
            .read()
            .map_err(|_| Error::internal("query image cache read lock poisoned"))?
            .get(&key)
            .cloned()
        {
            return Ok(image);
        }
        let image = Arc::new(QueryImageBuilder::new(txn, schema).build()?);
        self.images
            .write()
            .map_err(|_| Error::internal("query image cache write lock poisoned"))?
            .insert(key, image.clone());
        Ok(image)
    }
}

/// Builder for immutable query images.
pub struct QueryImageBuilder<'a, 'env> {
    txn: &'a ReadTxn<'env>,
    schema: &'a StorageSchema,
}

impl<'a, 'env> QueryImageBuilder<'a, 'env> {
    /// Creates a builder over one read snapshot.
    pub fn new(txn: &'a ReadTxn<'env>, schema: &'a StorageSchema) -> Self {
        Self { txn, schema }
    }

    /// Builds the query image.
    pub fn build(self) -> Result<QueryImage> {
        let start = Instant::now();
        let tx_id = self.txn.last_committed_tx_id()?;
        let mut relations = Vec::new();
        let mut segment_count = 0usize;
        let mut segment_bytes = 0usize;
        let mut built_from_segments = true;
        for (relation_id, relation) in self.schema.descriptor().relations.iter().enumerate() {
            let built = RelationImageBuilder::new(
                self.txn,
                self.schema,
                RelationId(relation_id as u16),
                relation,
            )
            .build()?;
            segment_count += usize::from(built.from_segment);
            segment_bytes += built.segment_bytes;
            built_from_segments &= built.from_segment;
            relations.push(built.relation);
        }
        Ok(QueryImage::new(
            self.schema,
            tx_id,
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
}

impl<'a, 'env, 'schema> RelationImageBuilder<'a, 'env, 'schema> {
    fn new(
        txn: &'a ReadTxn<'env>,
        schema: &'schema StorageSchema,
        relation_id: RelationId,
        relation: &'schema RelationDescriptor,
    ) -> Self {
        Self {
            txn,
            schema,
            relation_id,
            relation,
        }
    }

    fn build(self) -> Result<BuiltRelationImage> {
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
        for index in &segment.indexes {
            segment_bytes += index.byte_len;
        }
        let encoded_column_bytes = columns.iter().map(ColumnImage::byte_len).sum();
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                row_count: segment.row_count,
                fields,
                columns,
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
        let mut raw_columns = vec![Vec::<Vec<u8>>::new(); fields.len()];
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
                raw_columns[field_id].push(bytes.to_vec());
            }
        }

        let row_count = raw_columns.first().map_or(0, Vec::len);
        let columns = fields
            .iter()
            .map(|field| {
                ColumnImage::from_bytes(
                    field.id,
                    field.width,
                    raw_columns[field.id.0 as usize].clone(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let encoded_column_bytes = columns.iter().map(ColumnImage::byte_len).sum();
        Ok(BuiltRelationImage {
            relation: RelationImage {
                id: self.relation_id,
                name: self.relation.name.clone(),
                row_count,
                fields,
                columns,
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
    use crate::{Environment, KeyValues, Row, Value};

    #[test]
    fn builds_query_image_from_snapshot_and_matches_diagnostics() {
        let (env, schema) = seeded_env();

        let image = env.query_image(&schema).unwrap();
        let diagnostics = env.storage_diagnostics(&schema).unwrap();

        assert_eq!(image.stats().relation_count, 1);
        assert_eq!(image.stats().row_count, 2);
        assert_eq!(image.stats().sorted_trie_bytes, 0);
        assert_eq!(image.stats().hash_trie_bytes, 0);
        assert_eq!(image.stats().segment_count, 1);
        assert!(image.stats().segment_bytes > 0);
        assert!(image.stats().built_from_segments);
        assert_eq!(diagnostics.relations[0].row_count, 2);

        let segments = env.read(|txn| txn.visible_segments(&schema)).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].row_count, 2);
        assert_eq!(segments[0].columns.len(), 5);
        assert_eq!(segments[0].columns[0].byte_len, 16);
        assert!(!segments[0].indexes.is_empty());

        let account = image.relation("Account").unwrap();
        assert_eq!(account.row_count, 2);
        assert_eq!(account.fields.len(), 5);
        assert_eq!(account.encoded_column_bytes(), 2 * (8 + 8 + 1 + 8 + 8));
        assert_eq!(account.stats.row_count, account.row_count);
        assert_eq!(account.stats.field_count, account.fields.len());
        assert_eq!(
            account.stats.encoded_column_bytes,
            account.encoded_column_bytes()
        );
    }

    #[test]
    fn relation_image_columns_expose_widths_and_stable_row_ids() {
        let (env, schema) = seeded_env();
        let image = env.query_image(&schema).unwrap();
        let account = image.relation("Account").unwrap();

        assert_eq!(
            account.all_rows(),
            RowRange {
                start: RowId(0),
                end: RowId(2)
            }
        );
        assert_eq!(account.field(FieldId(0)).unwrap().encoded_width(), 8);
        assert_eq!(account.field(FieldId(2)).unwrap().encoded_width(), 1);
        assert_eq!(account.column(FieldId(0)).unwrap().len(), 2);
        assert_eq!(account.column(FieldId(0)).unwrap().field(), FieldId(0));
        assert_eq!(account.column(FieldId(2)).unwrap().width(), 1);
        assert!(matches!(
            account.column(FieldId(2)).unwrap(),
            ColumnImage::Bool(_)
        ));

        assert_eq!(
            account.encoded_bytes(RowId(0), FieldId(0)).unwrap(),
            1u64.to_be_bytes().as_slice()
        );
        assert_eq!(
            account.encoded_bytes(RowId(1), FieldId(0)).unwrap(),
            2u64.to_be_bytes().as_slice()
        );
        assert!(matches!(
            account.encoded(RowId(0), FieldId(2)).unwrap(),
            EncodedRef::One(_)
        ));
    }

    #[test]
    fn string_and_bytes_columns_store_intern_ids_not_raw_values() {
        let (env, schema) = seeded_env();
        let image = env.query_image(&schema).unwrap();
        let account = image.relation("Account").unwrap();

        let payload = account.encoded_bytes(RowId(0), FieldId(3)).unwrap();
        let name = account.encoded_bytes(RowId(0), FieldId(4)).unwrap();

        assert_eq!(payload.len(), 8);
        assert_eq!(name.len(), 8);
        assert_ne!(payload, &[1, 2, 3][..]);
        assert_ne!(name, b"Cash USD".as_slice());

        env.read(|txn| {
            assert_eq!(
                txn.decode_query_value(&account.field(FieldId(3)).unwrap().value_type, payload)?,
                Value::Bytes(vec![1, 2, 3])
            );
            assert_eq!(
                txn.decode_query_value(&account.field(FieldId(4)).unwrap().value_type, name)?,
                Value::String("Cash USD".to_owned())
            );
            Ok::<_, crate::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn query_image_encoded_columns_decode_to_public_scan_rows() {
        let (env, schema) = seeded_env();
        let image = env.query_image(&schema).unwrap();

        env.read(|txn| {
            let mut scanned = txn
                .scan_relation(&schema, "Account")?
                .map(|item| item.map(|item| item.row))
                .collect::<Result<Vec<_>>>()?;
            let account = image.relation("Account").unwrap();
            let mut imaged = decode_relation_rows(txn, account)?;
            scanned.sort();
            imaged.sort();
            assert_eq!(imaged, scanned);
            Ok::<_, crate::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn query_image_build_is_deterministic_for_same_snapshot() {
        let (env, schema) = seeded_env();

        env.read(|txn| {
            let left = QueryImageBuilder::new(txn, &schema).build()?;
            let right = QueryImageBuilder::new(txn, &schema).build()?;
            assert_eq!(left.content_fingerprint(), right.content_fingerprint());
            Ok::<_, crate::Error>(())
        })
        .unwrap();
    }

    #[test]
    fn query_image_cache_hits_until_transaction_id_changes() {
        let (env, schema) = seeded_env();

        let first = env.query_image(&schema).unwrap();
        let second = env.query_image(&schema).unwrap();
        assert!(Arc::ptr_eq(&first, &second));

        env.write(|txn| {
            txn.insert(
                &schema,
                Row::new(
                    "Account",
                    [
                        ("id", Value::Id(3)),
                        ("currency", Value::Symbol(826)),
                        ("active", Value::Bool(true)),
                        ("payload", Value::Bytes(vec![7, 8, 9])),
                        ("name", Value::String("Cash GBP".to_owned())),
                    ],
                ),
            )?;
            Ok::<_, crate::Error>(())
        })
        .unwrap();

        let third = env.query_image(&schema).unwrap();
        assert!(!Arc::ptr_eq(&first, &third));
        assert!(third.key().tx_id > first.key().tx_id);
        assert_eq!(third.relation("Account").unwrap().row_count, 3);
    }

    #[test]
    fn reopened_query_image_uses_durable_segments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(&path).unwrap();
        let schema = StorageSchema::new(account_schema(true), env.max_key_size()).unwrap();
        env.bulk_load(
            &schema,
            [
                account_row(1, 840, true, vec![1, 2, 3], "Cash USD"),
                account_row(2, 978, false, vec![4, 5, 6], "Cash EUR"),
            ],
        )
        .unwrap();
        drop(env);

        let reopened = Environment::open(&path).unwrap();
        let image = reopened.query_image(&schema).unwrap();

        assert!(image.stats().built_from_segments);
        assert_eq!(image.stats().segment_count, 1);
        assert_eq!(image.relation("Account").unwrap().row_count, 2);
    }

    #[test]
    fn read_snapshot_sees_stable_visible_segments() {
        let (env, schema) = seeded_env();

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
            let image = QueryImageBuilder::new(read, &schema).build()?;
            assert_eq!(image.relation("Account").unwrap().row_count, 2);
            Ok::<_, crate::Error>(())
        })
        .unwrap();

        let after = env.read(|read| read.visible_segments(&schema)).unwrap();
        assert_eq!(after[0].row_count, 3);
    }

    #[test]
    fn replace_and_delete_publish_visible_segments() {
        let (env, schema) = seeded_env();

        env.write(|txn| {
            txn.replace(
                &schema,
                account_row(2, 826, true, vec![9, 9, 9], "Cash GBP"),
            )?;
            txn.delete(&schema, KeyValues::new("Account", [("id", Value::Id(1))]))?;
            Ok::<_, crate::Error>(())
        })
        .unwrap();

        let image = env.query_image(&schema).unwrap();
        let account = image.relation("Account").unwrap();
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
        })
        .unwrap();
    }

    #[test]
    fn query_image_cache_does_not_reuse_mismatched_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(&path).unwrap();
        let schema_a = StorageSchema::new(account_schema(false), env.max_key_size()).unwrap();
        let schema_b = StorageSchema::new(account_schema(true), env.max_key_size()).unwrap();

        let image_a = env.query_image(&schema_a).unwrap();
        let image_b = env.query_image(&schema_b).unwrap();

        assert_ne!(image_a.key().schema, image_b.key().schema);
        assert!(!Arc::ptr_eq(&image_a, &image_b));
    }

    fn seeded_env() -> (Environment, StorageSchema) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let env = Environment::open(&path).unwrap();
        let schema = StorageSchema::new(account_schema(true), env.max_key_size()).unwrap();
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
        })
        .unwrap();
        (env, schema)
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
                ValueType::Symbol {
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
    }

    fn account_row(id: u64, currency: u64, active: bool, payload: Vec<u8>, name: &str) -> Row {
        Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("currency", Value::Symbol(currency)),
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
