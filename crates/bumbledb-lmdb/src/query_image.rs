use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use bumbledb_core::schema::{RelationDescriptor, SchemaFingerprint, ValueType};

use crate::{Error, ReadTxn, Result, StorageSchema};

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
                hasher.update(&(field.encoded_width as u64).to_be_bytes());
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
}

impl RelationImage {
    /// Returns the encoded value for `row` and `field`.
    pub fn encoded(&self, row: RowId, field: FieldId) -> Option<&[u8]> {
        self.columns.get(field.0 as usize)?.encoded(row)
    }

    /// Encoded column byte footprint.
    pub fn encoded_column_bytes(&self) -> usize {
        self.columns.iter().map(ColumnImage::byte_len).sum()
    }
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
    pub encoded_width: usize,
}

/// Encoded fixed-width column image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnImage {
    /// One-byte fixed-width column.
    Fixed1 {
        field: FieldId,
        values: Vec<[u8; 1]>,
    },
    /// Eight-byte fixed-width column.
    Fixed8 {
        field: FieldId,
        values: Vec<[u8; 8]>,
    },
    /// Sixteen-byte fixed-width column.
    Fixed16 {
        field: FieldId,
        values: Vec<[u8; 16]>,
    },
}

impl ColumnImage {
    fn from_bytes(field: FieldId, width: usize, values: Vec<Vec<u8>>) -> Result<Self> {
        Ok(match width {
            1 => ColumnImage::Fixed1 {
                field,
                values: values
                    .into_iter()
                    .map(|bytes| exact_array::<1>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            },
            8 => ColumnImage::Fixed8 {
                field,
                values: values
                    .into_iter()
                    .map(|bytes| exact_array::<8>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            },
            16 => ColumnImage::Fixed16 {
                field,
                values: values
                    .into_iter()
                    .map(|bytes| exact_array::<16>(&bytes))
                    .collect::<Result<Vec<_>>>()?,
            },
            _ => return Err(Error::internal(format!("unsupported column width {width}"))),
        })
    }

    fn encoded(&self, row: RowId) -> Option<&[u8]> {
        let row = row.0 as usize;
        match self {
            ColumnImage::Fixed1 { values, .. } => values.get(row).map(|bytes| bytes.as_slice()),
            ColumnImage::Fixed8 { values, .. } => values.get(row).map(|bytes| bytes.as_slice()),
            ColumnImage::Fixed16 { values, .. } => values.get(row).map(|bytes| bytes.as_slice()),
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            ColumnImage::Fixed1 { values, .. } => values.len(),
            ColumnImage::Fixed8 { values, .. } => values.len() * 8,
            ColumnImage::Fixed16 { values, .. } => values.len() * 16,
        }
    }

    #[cfg(test)]
    fn hash_into(&self, hasher: &mut blake3::Hasher) {
        match self {
            ColumnImage::Fixed1 { values, .. } => {
                for value in values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed8 { values, .. } => {
                for value in values {
                    hasher.update(value);
                }
            }
            ColumnImage::Fixed16 { values, .. } => {
                for value in values {
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
        for (relation_id, relation) in self.schema.descriptor().relations.iter().enumerate() {
            relations.push(
                RelationImageBuilder::new(
                    self.txn,
                    self.schema,
                    RelationId(relation_id as u16),
                    relation,
                )
                .build()?,
            );
        }
        Ok(QueryImage::new(
            self.schema,
            tx_id,
            relations,
            start.elapsed().as_micros(),
        ))
    }
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

    fn build(self) -> Result<RelationImage> {
        let fields = self
            .relation
            .fields
            .iter()
            .enumerate()
            .map(|(field_id, field)| FieldImage {
                id: FieldId(field_id as u16),
                name: field.name.clone(),
                value_type: field.value_type.clone(),
                encoded_width: field.value_type.encoded_width(),
            })
            .collect::<Vec<_>>();
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
                    field.encoded_width,
                    raw_columns[field.id.0 as usize].clone(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(RelationImage {
            id: self.relation_id,
            name: self.relation.name.clone(),
            row_count,
            fields,
            columns,
        })
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
    use crate::{Environment, Row, Value};

    #[test]
    fn builds_query_image_from_snapshot_and_matches_diagnostics() {
        let (env, schema) = seeded_env();

        let image = env.query_image(&schema).unwrap();
        let diagnostics = env.storage_diagnostics(&schema).unwrap();

        assert_eq!(image.stats().relation_count, 1);
        assert_eq!(image.stats().row_count, 2);
        assert_eq!(image.stats().sorted_trie_bytes, 0);
        assert_eq!(image.stats().hash_trie_bytes, 0);
        assert_eq!(diagnostics.relations[0].row_count, 2);

        let account = image.relation("Account").unwrap();
        assert_eq!(account.row_count, 2);
        assert_eq!(account.fields.len(), 3);
        assert_eq!(account.encoded_column_bytes(), 2 * (8 + 8 + 8));
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
                Row::new(
                    "Account",
                    [
                        ("id", Value::Id(1)),
                        ("currency", Value::Symbol(840)),
                        ("name", Value::String("Cash USD".to_owned())),
                    ],
                ),
            )?;
            txn.insert(
                &schema,
                Row::new(
                    "Account",
                    [
                        ("id", Value::Id(2)),
                        ("currency", Value::Symbol(978)),
                        ("name", Value::String("Cash EUR".to_owned())),
                    ],
                ),
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
                        txn.decode_query_value(&field.value_type, bytes)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            rows.push(Row::new(relation.name.clone(), values));
        }
        Ok(rows)
    }
}
