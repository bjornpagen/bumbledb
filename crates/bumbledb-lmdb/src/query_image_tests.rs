use std::sync::Arc;

use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::*;
use crate::{AccessId, Environment, Fact, Value};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn encoded_column_builder_appends_width_1() -> TestResult {
    let mut builder = EncodedColumnBuilder::with_capacity(FieldId(2), 1, 0)?;
    builder.append_bytes(&[1])?;
    builder.append_bytes(&[0])?;

    assert_eq!(builder.len(), 2);
    assert_eq!(builder.len() * builder.width(), 2);
    match builder.finish() {
        ColumnImage::Bool(column) => {
            assert_eq!(column.field(), FieldId(2));
            assert_eq!(column.get(FactId(0)), Some([1]));
            assert_eq!(column.get(FactId(1)), Some([0]));
        }
        other => return Err(format!("expected bool column, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn encoded_column_builder_appends_width_8() -> TestResult {
    let mut builder = EncodedColumnBuilder::with_capacity(FieldId(1), 8, 2)?;
    builder.append_encoded_owned(&EncodedOwned::Eight(7u64.to_be_bytes()))?;
    builder.append_bytes(&9u64.to_be_bytes())?;

    assert_eq!(builder.len(), 2);
    assert_eq!(builder.len() * builder.width(), 16);
    match builder.finish() {
        ColumnImage::Fixed8(column) => {
            assert_eq!(column.field(), FieldId(1));
            assert_eq!(column.get(FactId(0)), Some(7u64.to_be_bytes()));
            assert_eq!(column.get(FactId(1)), Some(9u64.to_be_bytes()));
        }
        other => return Err(format!("expected fixed8 column, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn encoded_column_builder_appends_width_16() -> TestResult {
    let mut builder = EncodedColumnBuilder::with_capacity(FieldId(3), 16, 0)?;
    builder.append_bytes(&1u128.to_be_bytes())?;
    builder.append_bytes(&2u128.to_be_bytes())?;

    assert_eq!(builder.len(), 2);
    assert_eq!(builder.len() * builder.width(), 32);
    match builder.finish() {
        ColumnImage::Fixed16(column) => {
            assert_eq!(column.field(), FieldId(3));
            assert_eq!(column.get(FactId(0)), Some(1u128.to_be_bytes()));
            assert_eq!(column.get(FactId(1)), Some(2u128.to_be_bytes()));
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
            assert_eq!(column.get(FactId(0)), Some(3u64.to_be_bytes()));
            assert_eq!(column.get(FactId(1)), Some(4u64.to_be_bytes()));
        }
        other => return Err(format!("expected fixed8 column, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn encoded_column_builder_rejects_bad_width() {
    assert!(EncodedColumnBuilder::with_capacity(FieldId(0), 4, 0).is_err());
}

#[test]
fn encoded_column_builder_rejects_bad_flat_length() -> TestResult {
    let mut builder = EncodedColumnBuilder::with_capacity(FieldId(0), 8, 0)?;

    assert!(builder.extend_flat_bytes(&[1, 2, 3]).is_err());
    assert!(builder.append_bytes(&[1, 2, 3]).is_err());
    Ok(())
}

#[test]
fn column_image_rejects_bad_flat_length() {
    assert!(ColumnImage::from_flat_bytes(FieldId(0), 8, &[1, 2, 3]).is_err());
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
        components: vec![RelationAccessComponent {
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
    assert_eq!(image.stats().fact_count, 2);
    assert_eq!(diagnostics.relations[0].fact_count, 2);

    let account = account_relation(&image)?;
    assert_eq!(account.fact_count, 2);
    assert_eq!(account.fields.len(), 5);
    assert_eq!(account.encoded_column_bytes(), 2 * (8 + 1 + 1 + 8 + 8));
    assert_eq!(account.stats.fact_count, account.fact_count);
    assert_eq!(account.stats.field_count, account.fields.len());
    assert_eq!(
        account.stats.encoded_column_bytes,
        account.encoded_column_bytes()
    );
    Ok(())
}

#[test]
fn relation_image_columns_expose_widths_and_stable_fact_ids() -> TestResult {
    let (env, schema) = seeded_env()?;
    let image = env.query_image(&schema)?;
    let account = account_relation(&image)?;

    assert_eq!(account.relation_cardinality(), 2);
    assert_eq!(field(account, FieldId(0))?.encoded_width(), 8);
    assert_eq!(field(account, FieldId(1))?.encoded_width(), 1);
    assert_eq!(field(account, FieldId(2))?.encoded_width(), 1);
    assert_eq!(column(account, FieldId(0))?.len(), 2);
    assert_eq!(column(account, FieldId(0))?.field(), FieldId(0));
    assert_eq!(column(account, FieldId(2))?.width(), 1);
    assert!(matches!(column(account, FieldId(2))?, ColumnImage::Bool(_)));

    assert_eq!(
        encoded_bytes(account, FactId(0), FieldId(0))?,
        1u64.to_be_bytes().as_slice()
    );
    assert_eq!(
        encoded_bytes(account, FactId(1), FieldId(0))?,
        2u64.to_be_bytes().as_slice()
    );
    assert!(matches!(
        encoded(account, FactId(0), FieldId(2))?,
        EncodedRef::One(_)
    ));
    Ok(())
}

#[test]
fn relation_image_exposes_access_prefix_cardinality() -> TestResult {
    let (env, schema) = seeded_env()?;
    let image = env.query_image(&schema)?;
    let account = account_relation(&image)?;
    let access = AccessId(
        schema
            .layout("Account", "fact_set")
            .ok_or_else(|| crate::Error::internal("missing fact_set access"))?
            .index_id,
    );
    let id_one = 1u64.to_be_bytes();
    let id_three = 3u64.to_be_bytes();

    assert_eq!(account.relation_cardinality(), 2);
    assert!(account.access_prefix_exists(access, &id_one));
    assert_eq!(account.access_prefix_cardinality(access, &id_one), 1);
    assert!(!account.access_prefix_exists(access, &id_three));
    assert_eq!(account.access_prefix_cardinality(access, &id_three), 0);
    assert!(image.stats().access_key_bytes > 0);
    Ok(())
}

#[test]
fn string_and_bytes_columns_store_intern_ids_not_raw_values() -> TestResult {
    let (env, schema) = seeded_env()?;
    let image = env.query_image(&schema)?;
    let account = account_relation(&image)?;

    let payload = encoded_bytes(account, FactId(0), FieldId(3))?;
    let name = encoded_bytes(account, FactId(0), FieldId(4))?;

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
fn query_image_encoded_columns_decode_to_public_scan_facts() -> TestResult {
    let (env, schema) = seeded_env()?;
    let image = env.query_image(&schema)?;

    env.read(|txn| {
        let mut scanned = txn
            .scan_relation(&schema, "Account")?
            .map(|item| item.map(|item| item.fact))
            .collect::<Result<Vec<_>>>()?;
        let account = account_relation(&image)?;
        let mut imaged = decode_relation_facts(txn, account)?;
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
        let left = QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
        let right = QueryImageBuilder::new(txn, &schema, QueryImageScope::full(&schema)).build()?;
        assert_eq!(left.content_fingerprint(), right.content_fingerprint());
        Ok::<_, crate::Error>(())
    })?;
    Ok(())
}

#[test]
fn bulk_loaded_query_image_exposes_current_access_images() -> TestResult {
    let dir = tempfile::tempdir().map_err(|error| crate::Error::io("tempdir", error))?;
    let path = dir.keep();
    let env = Environment::open(&path)?;
    let schema = StorageSchema::new(account_schema(true), env.max_key_size())?;
    env.bulk_load(
        &schema,
        vec![
            account_fact(1, 1, true, vec![1, 2, 3], "Cash USD"),
            account_fact(2, 2, false, vec![4, 5, 6], "Cash EUR"),
        ],
    )?;

    let image = env.query_image(&schema)?;
    let account = account_relation(&image)?;

    assert!(!account.indexes().is_empty());
    let fact_set_access = account
        .indexes()
        .iter()
        .find(|index| index.fields == vec![FieldId(0)])
        .ok_or_else(|| crate::Error::internal("missing fact-set index image"))?;
    assert_eq!(
        fact_set_access.bytes.len(),
        fact_set_access.encoded_len * account.fact_count
    );
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
            Fact::new(
                "Account",
                [
                    ("id", Value::Serial(3)),
                    ("currency", Value::Enum(3)),
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
    assert_eq!(account_relation(&third)?.fact_count, 3);
    Ok(())
}

#[test]
fn reopened_query_image_uses_current_access_fallback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.keep();
    let env = Environment::open(&path)?;
    let schema = StorageSchema::new(account_schema(true), env.max_key_size())?;
    env.bulk_load(
        &schema,
        [
            account_fact(1, 1, true, vec![1, 2, 3], "Cash USD"),
            account_fact(2, 2, false, vec![4, 5, 6], "Cash EUR"),
        ],
    )?;
    drop(env);

    let reopened = Environment::open(&path)?;
    let image = reopened.query_image(&schema)?;

    assert_eq!(account_relation(&image)?.fact_count, 2);
    Ok(())
}

#[test]
fn read_snapshot_sees_stable_current_access_image() -> TestResult {
    let (env, schema) = seeded_env()?;

    env.read(|read| {
        env.write(|write| {
            write.insert(&schema, account_fact(3, 3, true, vec![7, 8, 9], "Cash GBP"))?;
            Ok::<_, crate::Error>(())
        })?;

        let image =
            QueryImageBuilder::new(read, &schema, QueryImageScope::full(&schema)).build()?;
        assert_eq!(account_relation(&image)?.fact_count, 2);
        Ok::<_, crate::Error>(())
    })?;

    let after = env.query_image(&schema)?;
    assert_eq!(account_relation(&after)?.fact_count, 3);
    Ok(())
}

#[test]
fn exact_delete_and_insert_update_current_access_image() -> TestResult {
    let (env, schema) = seeded_env()?;

    env.write(|txn| {
        txn.delete(
            &schema,
            account_fact(2, 2, false, vec![4, 5, 6], "Cash EUR"),
        )?;
        txn.insert(&schema, account_fact(2, 3, true, vec![9, 9, 9], "Cash GBP"))?;
        txn.delete(&schema, account_fact(1, 1, true, vec![1, 2, 3], "Cash USD"))?;
        Ok::<_, crate::Error>(())
    })?;

    let image = env.query_image(&schema)?;
    let account = account_relation(&image)?;
    assert_eq!(account.fact_count, 1);

    env.read(|txn| {
        let facts = decode_relation_facts(txn, account)?;
        assert_eq!(
            facts,
            vec![account_fact(2, 3, true, vec![9, 9, 9], "Cash GBP")]
        );
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
        txn.insert(&schema, account_fact(1, 1, true, vec![1, 2, 3], "Cash USD"))?;
        txn.insert(
            &schema,
            account_fact(2, 2, false, vec![4, 5, 6], "Cash EUR"),
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
        .columns
        .get(&field)
        .ok_or_else(|| crate::Error::internal(format!("missing column {}", field.0)))
}

fn encoded<'a>(
    relation: &'a RelationImage,
    fact: FactId,
    field: FieldId,
) -> Result<EncodedRef<'a>> {
    relation.encoded(fact, field).ok_or_else(|| {
        crate::Error::internal(format!(
            "missing encoded value fact={} field={}",
            fact.0, field.0
        ))
    })
}

fn encoded_bytes(relation: &RelationImage, fact: FactId, field: FieldId) -> Result<&[u8]> {
    relation.encoded_bytes(fact, field).ok_or_else(|| {
        crate::Error::internal(format!(
            "missing encoded bytes fact={} field={}",
            fact.0, field.0
        ))
    })
}

fn account_schema(with_name: bool) -> SchemaDescriptor {
    let mut fields = vec![
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
        FieldDescriptor::new("payload", ValueType::Bytes),
    ];
    if with_name {
        fields.push(FieldDescriptor::new("name", ValueType::String));
    }
    SchemaDescriptor::new(
        "Accounts",
        vec![RelationDescriptor::new("Account", fields).with_unique("id", ["id"])],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1, 2, 3],
    ))
}

fn two_relation_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "ScopedAccounts",
        vec![
            RelationDescriptor::new("Account", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_unique("id", ["id"]),
            RelationDescriptor::new("Audit", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_unique("id", ["id"]),
        ],
    )
}

fn account_fact(id: u64, currency: u8, active: bool, payload: Vec<u8>, name: &str) -> Fact {
    Fact::new(
        "Account",
        [
            ("id", Value::Serial(id)),
            ("currency", Value::Enum(currency)),
            ("active", Value::Bool(active)),
            ("payload", Value::Bytes(payload)),
            ("name", Value::String(name.to_owned())),
        ],
    )
}

fn decode_relation_facts(txn: &ReadTxn<'_>, relation: &RelationImage) -> Result<Vec<Fact>> {
    let mut facts = Vec::new();
    for fact in 0..relation.fact_count {
        let fact = FactId(fact as u32);
        let values = relation
            .fields
            .iter()
            .map(|field| {
                let bytes = relation
                    .encoded(fact, field.id)
                    .ok_or_else(|| Error::internal("missing query image field"))?;
                Ok((
                    field.name.clone(),
                    txn.decode_query_value(&field.value_type, bytes.as_bytes())?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        facts.push(Fact::new(relation.name.clone(), values));
    }
    Ok(facts)
}
