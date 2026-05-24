#![allow(dead_code)]

use std::path::Path;

use crate::{Error, Result};

/// Current breaking storage format version.
pub(crate) const STORAGE_FORMAT_VERSION: u32 = 5;

const FORMAT_MARKER_FILE: &str = "FORMAT";

/// Durable key namespace byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Namespace {
    /// Canonical fact membership by full encoded fact.
    CanonicalFact = b'T' as isize,
    /// Fact handle to full encoded fact lookup.
    FactHandle = b'H' as isize,
    /// Live row membership by relation and fact handle.
    LiveRow = b'L' as isize,
    /// Durable column value by relation, field, and fact handle.
    Column = b'C' as isize,
    /// Serial sequence metadata.
    SerialSequence = b'Q' as isize,
    /// Unique constraint guard.
    UniqueGuard = b'U' as isize,
    /// Reverse foreign-key guard.
    ReverseForeignKeyGuard = b'R' as isize,
    /// Optional physical accelerator.
    Accelerator = b'A' as isize,
    /// Statistics.
    Stats = b'S' as isize,
}

impl Namespace {
    fn byte(self) -> u8 {
        self as u8
    }
}

/// Content-derived fact handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FactHandle(pub(crate) [u8; 16]);

/// Computes a relation-scoped content-derived fact handle.
pub(crate) fn fact_handle(relation_id: u32, fact_bytes: &[u8]) -> FactHandle {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&relation_id.to_be_bytes());
    hasher.update(fact_bytes);
    let hash = hasher.finalize();
    let mut handle = [0; 16];
    handle.copy_from_slice(&hash.as_bytes()[..16]);
    FactHandle(handle)
}

/// Opens an existing format marker or initializes a new empty environment.
pub(crate) fn open_or_init_format(path: &Path) -> Result<()> {
    let marker = path.join(FORMAT_MARKER_FILE);
    if marker.exists() {
        let version = read_format_version(path)?;
        if version == STORAGE_FORMAT_VERSION {
            Ok(())
        } else {
            Err(Error::storage_format_mismatch(
                STORAGE_FORMAT_VERSION,
                version.to_string(),
            ))
        }
    } else if is_empty_directory(path)? {
        std::fs::write(marker, STORAGE_FORMAT_VERSION.to_string())?;
        Ok(())
    } else {
        Err(Error::storage_format_mismatch(
            STORAGE_FORMAT_VERSION,
            "missing marker in non-empty directory",
        ))
    }
}

/// Reads the storage format marker.
pub(crate) fn read_format_version(path: &Path) -> Result<u32> {
    let marker = path.join(FORMAT_MARKER_FILE);
    let raw = std::fs::read_to_string(&marker).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::storage_format_mismatch(STORAGE_FORMAT_VERSION, "missing marker")
        } else {
            Error::Io(error)
        }
    })?;
    raw.trim()
        .parse::<u32>()
        .map_err(|_| Error::storage_format_mismatch(STORAGE_FORMAT_VERSION, raw.trim().to_owned()))
}

fn is_empty_directory(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn key(namespace: Namespace, parts: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![namespace.byte()];
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

fn u32_bytes(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

fn handle_bytes(handle: FactHandle) -> [u8; 16] {
    handle.0
}

/// `T | relation_id | fact_bytes -> fact_handle`.
pub(crate) fn canonical_fact_key(relation_id: u32, fact_bytes: &[u8]) -> Vec<u8> {
    key(
        Namespace::CanonicalFact,
        &[&u32_bytes(relation_id), fact_bytes],
    )
}

/// `H | relation_id | fact_handle -> fact_bytes`.
pub(crate) fn fact_handle_key(relation_id: u32, handle: FactHandle) -> Vec<u8> {
    key(
        Namespace::FactHandle,
        &[&u32_bytes(relation_id), &handle_bytes(handle)],
    )
}

/// `L | relation_id | fact_handle -> empty`.
pub(crate) fn live_row_key(relation_id: u32, handle: FactHandle) -> Vec<u8> {
    key(
        Namespace::LiveRow,
        &[&u32_bytes(relation_id), &handle_bytes(handle)],
    )
}

/// `C | relation_id | field_id | fact_handle -> encoded_field_bytes`.
pub(crate) fn column_key(relation_id: u32, field_id: u32, handle: FactHandle) -> Vec<u8> {
    key(
        Namespace::Column,
        &[
            &u32_bytes(relation_id),
            &u32_bytes(field_id),
            &handle_bytes(handle),
        ],
    )
}

/// `Q | relation_id | field_id -> next_u64`.
pub(crate) fn serial_sequence_key(relation_id: u32, field_id: u32) -> Vec<u8> {
    key(
        Namespace::SerialSequence,
        &[&u32_bytes(relation_id), &u32_bytes(field_id)],
    )
}

/// `U | relation_id | constraint_name | unique_key_bytes -> fact_handle`.
pub(crate) fn unique_guard_key(
    relation_id: u32,
    constraint_name: &str,
    unique_key_bytes: &[u8],
) -> Vec<u8> {
    key(
        Namespace::UniqueGuard,
        &[
            &u32_bytes(relation_id),
            constraint_name.as_bytes(),
            b"\0",
            unique_key_bytes,
        ],
    )
}

/// `R | target_relation | target_constraint | target_key | source_relation | source_constraint | source_handle`.
pub(crate) fn reverse_fk_guard_key(
    target_relation_id: u32,
    target_constraint: &str,
    target_key_bytes: &[u8],
    source_relation_id: u32,
    source_constraint: &str,
    source_handle: FactHandle,
) -> Vec<u8> {
    key(
        Namespace::ReverseForeignKeyGuard,
        &[
            &u32_bytes(target_relation_id),
            target_constraint.as_bytes(),
            b"\0",
            target_key_bytes,
            &u32_bytes(source_relation_id),
            source_constraint.as_bytes(),
            b"\0",
            &handle_bytes(source_handle),
        ],
    )
}

/// `A | relation_id | accelerator_id | tuple_key | fact_handle -> empty`.
pub(crate) fn accelerator_key(
    relation_id: u32,
    accelerator_id: u32,
    tuple_key: &[u8],
    handle: FactHandle,
) -> Vec<u8> {
    key(
        Namespace::Accelerator,
        &[
            &u32_bytes(relation_id),
            &u32_bytes(accelerator_id),
            tuple_key,
            &handle_bytes(handle),
        ],
    )
}

/// `S | relation_id | stat_name -> encoded_stat`.
pub(crate) fn stats_key(relation_id: u32, stat_name: &str) -> Vec<u8> {
    key(
        Namespace::Stats,
        &[&u32_bytes(relation_id), stat_name.as_bytes()],
    )
}

#[cfg(test)]
#[path = "storage_format_tests.rs"]
mod tests;
