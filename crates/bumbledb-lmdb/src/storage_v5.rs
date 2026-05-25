use crate::Value;
#[cfg(test)]
use crate::storage_format::FactHandle;
use crate::storage_format::{
    RowId, accelerator_key, canonical_fact_key, column_key, fact_handle_key, live_row_key,
    physical_row_key, reverse_fk_guard_key, reverse_fk_guard_prefix, unique_guard_key,
};
use crate::{
    Databases, DeleteOutcome, Error, Fact, FactView, InsertOutcome, ReadTxn, Result, WriteTxn,
};
use bumbledb_core::schema::{ConstraintDescriptor, SchemaDescriptor, ValueType};

#[path = "storage_v5_codec.rs"]
mod codec;
#[path = "storage_v5_meta.rs"]
mod meta;

#[cfg(test)]
pub(crate) use meta::META_STORAGE_FORMAT_VERSION;

#[cfg(test)]
use codec::decode_fact;
use codec::{
    EncodeDelete, EncodedFact, encode_delete_fact, encode_insert_fact, encoded_key_from_fields,
};
use meta::{
    adjust_relation_count, advance_storage_tx_id, allocate_row_id, bytes_to_u64,
    read_relation_count, relation_id,
};

pub(crate) fn init_metadata(
    dbs: Databases,
    txn: &mut heed::RwTxn<'_>,
    had_data_file: bool,
) -> Result<()> {
    meta::init_metadata(dbs, txn, had_data_file)
}

pub(crate) fn verify_schema(
    dbs: Databases,
    txn: &mut heed::RwTxn<'_>,
    schema: &SchemaDescriptor,
) -> Result<()> {
    meta::verify_schema(dbs, txn, schema)
}

pub(crate) fn storage_format_version(txn: &ReadTxn<'_>) -> Result<u32> {
    meta::storage_format_version(txn)
}

pub(crate) fn storage_tx_id(txn: &ReadTxn<'_>) -> Result<u64> {
    meta::storage_tx_id(txn)
}

pub(crate) fn dictionary_entry_count(txn: &ReadTxn<'_>) -> Result<usize> {
    meta::dictionary_entry_count(txn)
}

pub(crate) fn decode_value(
    txn: &ReadTxn<'_>,
    value_type: &ValueType,
    bytes: &[u8],
) -> Result<Value> {
    codec::decode_value(txn, value_type, bytes)
}

pub(crate) fn encode_existing_value(
    txn: &ReadTxn<'_>,
    schema: &SchemaDescriptor,
    value_type: &ValueType,
    value: &Value,
) -> Result<Option<crate::colt::KeyOwned>> {
    codec::encode_existing_value(txn, schema, value_type, value)
}

pub(crate) fn relation_fact_count(
    txn: &ReadTxn<'_>,
    schema: &crate::StorageSchema,
    relation: &str,
) -> Result<u64> {
    let relation_id = relation_id(schema.descriptor(), relation)?;
    read_relation_count(txn.dbs.data, &txn.txn, relation_id)
}

pub(crate) fn insert<F: FactView>(
    txn: &mut WriteTxn<'_>,
    schema: &crate::StorageSchema,
    fact: &F,
) -> Result<InsertOutcome> {
    let encoded = encode_insert_fact(txn, schema.descriptor(), fact)?;
    let canonical_key = canonical_fact_key(encoded.relation_id, &encoded.bytes);
    if txn.dbs.data.get(&txn.txn, &canonical_key)?.is_some() {
        return Ok(InsertOutcome::AlreadyPresent);
    }

    check_foreign_keys(txn, schema.descriptor(), &encoded)?;
    check_unique_constraints(txn, &encoded)?;
    let row_id = allocate_row_id(txn, encoded.relation_id)?;
    write_fact(txn, schema.descriptor(), &encoded, row_id)?;
    advance_storage_tx_id(txn)?;
    Ok(InsertOutcome::Inserted)
}

pub(crate) fn delete<F: FactView>(
    txn: &mut WriteTxn<'_>,
    schema: &crate::StorageSchema,
    fact: &F,
) -> Result<DeleteOutcome> {
    let encoded = match encode_delete_fact(txn, schema.descriptor(), fact)? {
        EncodeDelete::Encoded(encoded) => encoded,
        EncodeDelete::MissingDictionary => return Ok(DeleteOutcome::Absent),
    };
    let canonical_key = canonical_fact_key(encoded.relation_id, &encoded.bytes);
    if txn.dbs.data.get(&txn.txn, &canonical_key)?.is_none() {
        return Ok(DeleteOutcome::Absent);
    }

    check_restrict_delete(txn, &encoded)?;
    delete_fact(txn, schema.descriptor(), &encoded)?;
    advance_storage_tx_id(txn)?;
    Ok(DeleteOutcome::Deleted)
}

pub(crate) fn bulk_load(
    txn: &mut WriteTxn<'_>,
    schema: &crate::StorageSchema,
    facts: impl IntoIterator<Item = Fact>,
) -> Result<usize> {
    let mut inserted = 0;
    for fact in facts {
        if insert(txn, schema, &fact)? == InsertOutcome::Inserted {
            inserted += 1;
        }
    }
    Ok(inserted)
}

#[cfg(test)]
pub(crate) fn debug_relation_facts(
    txn: &ReadTxn<'_>,
    schema: &crate::StorageSchema,
    relation: &str,
) -> Result<Vec<Fact>> {
    let descriptor = schema.descriptor();
    let relation_id = relation_id(descriptor, relation)?;
    let relation = descriptor
        .relations
        .get(relation_id as usize)
        .ok_or_else(|| Error::corrupt("relation id missing"))?;
    let prefix = fact_handle_key(relation_id, FactHandle([0; 16]));
    let prefix = &prefix[..5];
    let mut facts = Vec::new();
    for item in txn.dbs.data.prefix_iter(&txn.txn, prefix)? {
        let (_key, bytes) = item?;
        facts.push(decode_fact(txn, relation, bytes)?);
    }
    facts.sort();
    Ok(facts)
}

fn write_fact(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &EncodedFact<'_>,
    row_id: RowId,
) -> Result<()> {
    let data = txn.dbs.data;
    data.put(
        &mut txn.txn,
        &canonical_fact_key(fact.relation_id, &fact.bytes),
        &fact.handle.0,
    )?;
    data.put(
        &mut txn.txn,
        &fact_handle_key(fact.relation_id, fact.handle),
        &fact.bytes,
    )?;
    data.put(
        &mut txn.txn,
        &live_row_key(fact.relation_id, row_id),
        &fact.handle.0,
    )?;
    data.put(
        &mut txn.txn,
        &physical_row_key(fact.relation_id, fact.handle),
        &row_id.0.to_be_bytes(),
    )?;
    for field_id in 0..fact.fields.len() {
        data.put(
            &mut txn.txn,
            &column_key(fact.relation_id, field_id as u32, row_id),
            fact.field_bytes(field_id),
        )?;
        data.put(
            &mut txn.txn,
            &accelerator_key(
                fact.relation_id,
                field_id as u32,
                fact.field_bytes(field_id),
                row_id,
            ),
            &[],
        )?;
    }
    write_unique_guards(txn, fact)?;
    write_reverse_fk_guards(txn, schema, fact)?;
    adjust_relation_count(txn, fact.relation_id, 1)
}

fn delete_fact(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &EncodedFact<'_>,
) -> Result<()> {
    delete_reverse_fk_guards(txn, schema, fact)?;
    delete_unique_guards(txn, fact)?;
    let data = txn.dbs.data;
    let row_id = row_id_for_fact(txn, fact.relation_id, fact.handle)?;
    for field_id in 0..fact.fields.len() {
        data.delete(
            &mut txn.txn,
            &column_key(fact.relation_id, field_id as u32, row_id),
        )?;
        data.delete(
            &mut txn.txn,
            &accelerator_key(
                fact.relation_id,
                field_id as u32,
                fact.field_bytes(field_id),
                row_id,
            ),
        )?;
    }
    data.delete(&mut txn.txn, &live_row_key(fact.relation_id, row_id))?;
    data.delete(
        &mut txn.txn,
        &physical_row_key(fact.relation_id, fact.handle),
    )?;
    data.delete(
        &mut txn.txn,
        &fact_handle_key(fact.relation_id, fact.handle),
    )?;
    data.delete(
        &mut txn.txn,
        &canonical_fact_key(fact.relation_id, &fact.bytes),
    )?;
    adjust_relation_count(txn, fact.relation_id, -1)
}

fn row_id_for_fact(
    txn: &WriteTxn<'_>,
    relation_id: u32,
    handle: crate::storage_format::FactHandle,
) -> Result<RowId> {
    let bytes = txn
        .dbs
        .data
        .get(&txn.txn, &physical_row_key(relation_id, handle))?
        .ok_or_else(|| Error::corrupt("physical row lookup missing"))?;
    Ok(RowId(bytes_to_u64(bytes)?))
}

fn check_unique_constraints(txn: &WriteTxn<'_>, fact: &EncodedFact<'_>) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::Unique { name, fields } = constraint {
            let key_bytes = encoded_key_from_fields(fact.relation, fact, fields)?;
            if txn
                .dbs
                .data
                .get(
                    &txn.txn,
                    &unique_guard_key(fact.relation_id, name, &key_bytes),
                )?
                .is_some()
            {
                return Err(Error::unique_violation(&fact.relation.name, name));
            }
        }
    }
    Ok(())
}

fn write_unique_guards(txn: &mut WriteTxn<'_>, fact: &EncodedFact<'_>) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::Unique { name, fields } = constraint {
            let key_bytes = encoded_key_from_fields(fact.relation, fact, fields)?;
            txn.dbs.data.put(
                &mut txn.txn,
                &unique_guard_key(fact.relation_id, name, &key_bytes),
                &fact.handle.0,
            )?;
        }
    }
    Ok(())
}

fn delete_unique_guards(txn: &mut WriteTxn<'_>, fact: &EncodedFact<'_>) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::Unique { name, fields } = constraint {
            let key_bytes = encoded_key_from_fields(fact.relation, fact, fields)?;
            txn.dbs.data.delete(
                &mut txn.txn,
                &unique_guard_key(fact.relation_id, name, &key_bytes),
            )?;
        }
    }
    Ok(())
}

fn check_foreign_keys(
    txn: &WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &EncodedFact<'_>,
) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::ForeignKey {
            name,
            fields,
            target_relation,
            target_constraint,
            ..
        } = constraint
        {
            let target_relation_id = relation_id(schema, target_relation)?;
            let source_key = encoded_key_from_fields(fact.relation, fact, fields)?;
            let target_key = unique_guard_key(target_relation_id, target_constraint, &source_key);
            if txn.dbs.data.get(&txn.txn, &target_key)?.is_none() {
                return Err(Error::foreign_key_violation(&fact.relation.name, name));
            }
        }
    }
    Ok(())
}

fn write_reverse_fk_guards(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &EncodedFact<'_>,
) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::ForeignKey {
            name,
            fields,
            target_relation,
            target_constraint,
            ..
        } = constraint
        {
            let target_relation_id = relation_id(schema, target_relation)?;
            let source_key = encoded_key_from_fields(fact.relation, fact, fields)?;
            txn.dbs.data.put(
                &mut txn.txn,
                &reverse_fk_guard_key(
                    target_relation_id,
                    target_constraint,
                    &source_key,
                    fact.relation_id,
                    name,
                    fact.handle,
                ),
                &[],
            )?;
        }
    }
    Ok(())
}

fn delete_reverse_fk_guards(
    txn: &mut WriteTxn<'_>,
    schema: &SchemaDescriptor,
    fact: &EncodedFact<'_>,
) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::ForeignKey {
            name,
            fields,
            target_relation,
            target_constraint,
            ..
        } = constraint
        {
            let target_relation_id = relation_id(schema, target_relation)?;
            let source_key = encoded_key_from_fields(fact.relation, fact, fields)?;
            txn.dbs.data.delete(
                &mut txn.txn,
                &reverse_fk_guard_key(
                    target_relation_id,
                    target_constraint,
                    &source_key,
                    fact.relation_id,
                    name,
                    fact.handle,
                ),
            )?;
        }
    }
    Ok(())
}

fn check_restrict_delete(txn: &WriteTxn<'_>, fact: &EncodedFact<'_>) -> Result<()> {
    for constraint in &fact.relation.constraints {
        if let ConstraintDescriptor::Unique { name, fields } = constraint {
            let key_bytes = encoded_key_from_fields(fact.relation, fact, fields)?;
            let prefix = reverse_fk_guard_prefix(fact.relation_id, name, &key_bytes);
            let mut iter = txn.dbs.data.prefix_iter(&txn.txn, &prefix)?;
            if let Some(item) = iter.next() {
                let _ = item?;
                return Err(Error::restrict_violation(&fact.relation.name, name));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "storage_v5_tests.rs"]
mod tests;
