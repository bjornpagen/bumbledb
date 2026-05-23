use super::*;

pub(super) const NS_CANONICAL_FACT: u8 = 0x10;
pub(super) const NS_FACT_ID: u8 = 0x12;
pub(super) const NS_ACCESS_ENTRY: u8 = 0x11;
pub(super) const NS_UNIQUE_ENTRY: u8 = 0x13;
pub(super) const NS_REVERSE_FK_ENTRY: u8 = 0x14;
pub(super) const FACT_ID_BYTES: usize = 16;
pub(super) const DICT_FWD: u8 = 0x01;
pub(super) const DICT_REV: u8 = 0x02;
pub(super) const DICT_STRING: u8 = 0x01;
pub(super) const DICT_BYTES: u8 = 0x02;

pub(super) const NEXT_TX_ID_KEY: &[u8] = b"next_tx_id";

pub(super) fn encoded_key_from_fields<'a>(
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<u8>> {
    let mut prefix = Vec::new();
    for field in fields {
        prefix.extend_from_slice(fact.field(relation, field)?);
    }
    Ok(prefix)
}

pub(super) fn unique_entry_key_from_fact(
    relation_id: u16,
    constraint: &str,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: &[String],
) -> Result<Vec<u8>> {
    let encoded_key = encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
    Ok(unique_entry_key(relation_id, constraint, &encoded_key))
}

pub(super) fn unique_entry_key_from_source(
    relation_id: u16,
    constraint: &str,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
    fields: &[String],
) -> Result<Vec<u8>> {
    let encoded_key = encoded_key_from_fields(relation, fact, fields.iter().map(String::as_str))?;
    Ok(unique_entry_key(relation_id, constraint, &encoded_key))
}

pub(super) fn unique_entry_key(relation_id: u16, constraint: &str, encoded_key: &[u8]) -> Vec<u8> {
    let mut key = vec![NS_UNIQUE_ENTRY];
    push_u16(&mut key, relation_id);
    push_name(&mut key, constraint);
    key.extend_from_slice(encoded_key);
    key
}

pub(super) fn reverse_fk_prefix(relation_id: u16, constraint: &str, encoded_key: &[u8]) -> Vec<u8> {
    let mut key = vec![NS_REVERSE_FK_ENTRY];
    push_u16(&mut key, relation_id);
    push_name(&mut key, constraint);
    key.extend_from_slice(encoded_key);
    key
}

pub(super) fn reverse_fk_entry_key(
    relation_id: u16,
    constraint: &str,
    encoded_key: &[u8],
    source_relation_id: u16,
    source_constraint: &str,
    source_fact_id: &[u8; FACT_ID_BYTES],
) -> Vec<u8> {
    let mut key = reverse_fk_prefix(relation_id, constraint, encoded_key);
    push_u16(&mut key, source_relation_id);
    push_name(&mut key, source_constraint);
    key.extend_from_slice(source_fact_id);
    key
}

pub(super) fn access_prefix(relation_id: u16, index_id: u16) -> Vec<u8> {
    let mut key = vec![NS_ACCESS_ENTRY];
    push_u16(&mut key, relation_id);
    push_u16(&mut key, index_id);
    key
}

pub(super) fn canonical_fact_key(relation_id: u16, fact: &EncodedFact) -> Vec<u8> {
    let mut key = canonical_fact_prefix(relation_id);
    key.extend_from_slice(fact.bytes());
    key
}

pub(super) fn canonical_fact_prefix(relation_id: u16) -> Vec<u8> {
    let mut key = vec![NS_CANONICAL_FACT];
    push_u16(&mut key, relation_id);
    key
}

pub(super) fn fact_id(fact: &EncodedFact) -> [u8; FACT_ID_BYTES] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&fact.relation.0.to_be_bytes());
    hasher.update(fact.bytes());
    let hash = hasher.finalize();
    let mut out = [0; FACT_ID_BYTES];
    out.copy_from_slice(&hash.as_bytes()[..FACT_ID_BYTES]);
    out
}

pub(super) fn fact_id_prefix(relation_id: u16) -> Vec<u8> {
    let mut key = vec![NS_FACT_ID];
    push_u16(&mut key, relation_id);
    key
}

pub(super) fn fact_id_key(relation_id: u16, fact: &EncodedFact) -> Vec<u8> {
    let mut key = fact_id_prefix(relation_id);
    key.extend_from_slice(&fact_id(fact));
    key
}

#[cfg(test)]
pub(super) fn lookup_fact_by_id(
    db: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation_id: u16,
    id: &[u8],
) -> Result<EncodedFact> {
    if id.len() != FACT_ID_BYTES {
        return Err(Error::corrupt("fact id width invalid"));
    }
    let mut key = fact_id_prefix(relation_id);
    key.extend_from_slice(id);
    let bytes = db
        .get(txn, key.as_slice())?
        .ok_or_else(|| Error::corrupt("fact id target missing"))?
        .to_vec();
    Ok(EncodedFact {
        relation: RelationId(relation_id),
        bytes,
    })
}

pub(super) fn access_key(
    layout: &AccessLayout,
    relation: &RelationDescriptor,
    fact: &EncodedFact,
) -> Result<Vec<u8>> {
    if fact.relation.0 != layout.relation_id {
        return Err(Error::corrupt(
            "encoded fact relation does not match index layout",
        ));
    }
    let mut key = access_prefix(layout.relation_id, layout.index_id);
    for component in &layout.components {
        key.extend_from_slice(fact.field(relation, &component.field_name)?);
    }
    key.extend_from_slice(&fact_id(fact));
    Ok(key)
}

pub(super) fn read_u64_meta(txn: &WriteTxn<'_>, key: &[u8]) -> Result<Option<u64>> {
    read_u64(&txn.dbs.meta, &txn.txn, key)
}

pub(super) fn write_u64_meta(txn: &mut WriteTxn<'_>, key: &[u8], value: u64) -> Result<()> {
    write_u64(&txn.dbs.meta, &mut txn.txn, key, value)
}

pub(super) fn read_u64(
    db: &crate::RawDatabase,
    txn: &heed::RoTxn,
    key: &[u8],
) -> Result<Option<u64>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| Error::corrupt("u64 metadata must be eight bytes"))?;
    Ok(Some(u64::from_be_bytes(bytes)))
}

pub(super) fn write_u64(
    db: &crate::RawDatabase,
    txn: &mut heed::RwTxn,
    key: &[u8],
    value: u64,
) -> Result<()> {
    let bytes = value.to_be_bytes();
    Ok(db.put(txn, key, &bytes[..])?)
}

pub(super) fn adjust_relation_fact_count(
    txn: &mut WriteTxn<'_>,
    relation_id: u16,
    delta: i64,
) -> Result<()> {
    adjust_u64_meta(txn, &relation_fact_count_key(relation_id), delta)
}

pub(super) fn adjust_access_entry_count(
    txn: &mut WriteTxn<'_>,
    relation_id: u16,
    index_id: u16,
    delta: i64,
) -> Result<()> {
    adjust_u64_meta(txn, &access_entry_count_key(relation_id, index_id), delta)
}

pub(super) fn adjust_u64_meta(txn: &mut WriteTxn<'_>, key: &[u8], delta: i64) -> Result<()> {
    crate::failpoints::check(crate::failpoints::Failpoint::BeforeStatsUpdate)?;
    let current = read_u64_meta(txn, key)?.unwrap_or(0);
    let next = if delta >= 0 {
        current
            .checked_add(delta as u64)
            .ok_or_else(|| Error::internal("metadata counter overflow"))?
    } else {
        current
            .checked_sub(delta.unsigned_abs())
            .ok_or_else(|| Error::internal("metadata counter underflow"))?
    };
    write_u64_meta(txn, key, next)?;
    crate::failpoints::check(crate::failpoints::Failpoint::AfterStatsUpdate)?;
    Ok(())
}

pub(super) fn relation_fact_count_key(relation_id: u16) -> Vec<u8> {
    let mut key = b"stats:facts:".to_vec();
    push_u16(&mut key, relation_id);
    key
}

pub(super) fn access_entry_count_key(relation_id: u16, index_id: u16) -> Vec<u8> {
    let mut key = b"stats:index:".to_vec();
    push_u16(&mut key, relation_id);
    push_u16(&mut key, index_id);
    key
}

pub(super) fn next_dict_id_key(kind: u8) -> Vec<u8> {
    vec![
        b'd', b'i', b'c', b't', b':', b'n', b'e', b'x', b't', b':', kind,
    ]
}

pub(super) fn dict_fwd_key(kind: u8, raw: &[u8]) -> Vec<u8> {
    let mut key = vec![DICT_FWD, kind];
    key.extend_from_slice(blake3::hash(raw).as_bytes());
    key
}

pub(super) fn dict_rev_key(kind: u8, id: u64) -> Vec<u8> {
    let mut key = vec![DICT_REV, kind];
    push_u64(&mut key, id);
    key
}

pub(super) fn lookup_intern_value(
    db: &crate::RawDatabase,
    txn: &heed::RoTxn,
    kind: u8,
    raw: &[u8],
) -> Result<Option<u64>> {
    let Some(value) = db.get(txn, dict_fwd_key(kind, raw).as_slice())? else {
        return Ok(None);
    };
    if value.len() < 8 {
        return Err(Error::corrupt("dictionary forward value too short"));
    }
    let id = u64::from_be_bytes(
        value[..8]
            .try_into()
            .map_err(|_| Error::corrupt("dictionary ID width invalid"))?,
    );
    if &value[8..] != raw {
        return Err(Error::hash_collision(dict_kind_name(kind)));
    }
    Ok(Some(id))
}

pub(super) fn lookup_intern_raw_by_id(
    db: crate::RawDatabase,
    txn: &heed::RoTxn,
    kind: u8,
    id: u64,
) -> Result<Vec<u8>> {
    db.get(txn, dict_rev_key(kind, id).as_slice())?
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::dictionary_value_not_found(dict_kind_name(kind)))
}

pub(super) fn dict_kind_name(kind: u8) -> &'static str {
    match kind {
        DICT_STRING => "string",
        DICT_BYTES => "bytes",
        _ => "unknown",
    }
}

pub(super) fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn push_name(out: &mut Vec<u8>, value: &str) {
    push_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}
