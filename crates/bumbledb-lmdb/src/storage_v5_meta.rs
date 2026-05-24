use crate::storage_format::{STORAGE_FORMAT_VERSION, stats_key};
use crate::{Databases, Error, RawDatabase, ReadTxn, Result, WriteTxn};
use bumbledb_core::schema::SchemaDescriptor;

pub(crate) const META_STORAGE_FORMAT_VERSION: &[u8] = b"storage_format_version";
pub(crate) const META_SCHEMA_FINGERPRINT: &[u8] = b"schema_fingerprint";
pub(crate) const META_STORAGE_TX_ID: &[u8] = b"storage_tx_id";
pub(crate) const META_NEXT_DICT_ID: &[u8] = b"next_dict_id";
pub(crate) const DICT_FWD: u8 = b'F';
pub(crate) const DICT_REV: u8 = b'R';
pub(crate) const DICT_STRING: u8 = b's';
pub(crate) const DICT_BYTES: u8 = b'b';
const STAT_FACT_COUNT: &str = "fact_count";

pub(crate) fn init_metadata(
    dbs: Databases,
    txn: &mut heed::RwTxn<'_>,
    had_data_file: bool,
) -> Result<()> {
    match read_u32(&dbs.meta, txn, META_STORAGE_FORMAT_VERSION)? {
        Some(STORAGE_FORMAT_VERSION) => Ok(()),
        Some(found) => Err(Error::storage_format_mismatch(
            STORAGE_FORMAT_VERSION,
            found.to_string(),
        )),
        None if had_data_file => Err(Error::storage_format_mismatch(
            STORAGE_FORMAT_VERSION,
            "missing marker in LMDB metadata",
        )),
        None => write_u32(
            &dbs.meta,
            txn,
            META_STORAGE_FORMAT_VERSION,
            STORAGE_FORMAT_VERSION,
        ),
    }
}

pub(crate) fn verify_schema(
    dbs: Databases,
    txn: &mut heed::RwTxn<'_>,
    schema: &SchemaDescriptor,
) -> Result<()> {
    let expected = schema.fingerprint().0;
    match dbs.meta.get(txn, META_SCHEMA_FINGERPRINT)? {
        Some(found) if found == expected.as_slice() => Ok(()),
        Some(found) => Err(Error::schema_mismatch(hex(&expected), hex(found))),
        None => Ok(dbs
            .meta
            .put(txn, META_SCHEMA_FINGERPRINT, expected.as_slice())?),
    }
}

pub(crate) fn storage_format_version(txn: &ReadTxn<'_>) -> Result<u32> {
    read_u32(&txn.dbs.meta, &txn.txn, META_STORAGE_FORMAT_VERSION)?
        .ok_or_else(|| Error::storage_format_mismatch(STORAGE_FORMAT_VERSION, "missing marker"))
}

pub(crate) fn storage_tx_id(txn: &ReadTxn<'_>) -> Result<u64> {
    Ok(read_u64(&txn.dbs.meta, &txn.txn, META_STORAGE_TX_ID)?.unwrap_or(0))
}

pub(crate) fn dictionary_entry_count(txn: &ReadTxn<'_>) -> Result<usize> {
    let prefix = [DICT_FWD];
    let mut count = 0;
    for item in txn.dbs.dict.prefix_iter(&txn.txn, &prefix)? {
        let _ = item?;
        count += 1;
    }
    Ok(count)
}

pub(crate) fn relation_id(schema: &SchemaDescriptor, relation: &str) -> Result<u32> {
    schema
        .relations
        .iter()
        .position(|candidate| candidate.name == relation)
        .map(|id| id as u32)
        .ok_or_else(|| Error::invalid_fact(format!("unknown relation {relation}")))
}

pub(crate) fn read_relation_count(
    db: RawDatabase,
    txn: &heed::RoTxn<'_>,
    relation_id: u32,
) -> Result<u64> {
    Ok(read_u64(&db, txn, &stats_key(relation_id, STAT_FACT_COUNT))?.unwrap_or(0))
}

pub(crate) fn adjust_relation_count(
    txn: &mut WriteTxn<'_>,
    relation_id: u32,
    delta: i64,
) -> Result<()> {
    let current = read_relation_count(txn.dbs.data, &txn.txn, relation_id)?;
    let next = if delta >= 0 {
        current
            .checked_add(delta as u64)
            .ok_or_else(|| Error::corrupt("relation count overflow"))?
    } else {
        current
            .checked_sub(delta.unsigned_abs())
            .ok_or_else(|| Error::corrupt("relation count underflow"))?
    };
    write_u64(
        &txn.dbs.data,
        &mut txn.txn,
        &stats_key(relation_id, STAT_FACT_COUNT),
        next,
    )
}

pub(crate) fn advance_storage_tx_id(txn: &mut WriteTxn<'_>) -> Result<()> {
    let current = read_u64(&txn.dbs.meta, &txn.txn, META_STORAGE_TX_ID)?.unwrap_or(0);
    write_u64(&txn.dbs.meta, &mut txn.txn, META_STORAGE_TX_ID, current + 1)
}

pub(crate) fn read_u32(db: &RawDatabase, txn: &heed::RoTxn<'_>, key: &[u8]) -> Result<Option<u32>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| Error::corrupt("u32 value has wrong width"))?;
    Ok(Some(u32::from_be_bytes(bytes)))
}

pub(crate) fn write_u32(
    db: &RawDatabase,
    txn: &mut heed::RwTxn<'_>,
    key: &[u8],
    value: u32,
) -> Result<()> {
    Ok(db.put(txn, key, &value.to_be_bytes())?)
}

pub(crate) fn read_u64(db: &RawDatabase, txn: &heed::RoTxn<'_>, key: &[u8]) -> Result<Option<u64>> {
    let Some(bytes) = db.get(txn, key)? else {
        return Ok(None);
    };
    bytes_to_u64(bytes).map(Some)
}

pub(crate) fn write_u64(
    db: &RawDatabase,
    txn: &mut heed::RwTxn<'_>,
    key: &[u8],
    value: u64,
) -> Result<()> {
    Ok(db.put(txn, key, &value.to_be_bytes())?)
}

pub(crate) fn bytes_to_u64(bytes: &[u8]) -> Result<u64> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| Error::corrupt("u64 value has wrong width"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}
