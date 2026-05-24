#![allow(dead_code)]

/// Current breaking storage format version.
pub(crate) const STORAGE_FORMAT_VERSION: u32 = 5;

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

const STORAGE_KEY_INLINE: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StorageKey {
    len: usize,
    bytes: [u8; STORAGE_KEY_INLINE],
}

impl StorageKey {
    fn new(namespace: Namespace) -> Self {
        let mut key = Self {
            len: 0,
            bytes: [0; STORAGE_KEY_INLINE],
        };
        key.push_byte(namespace.byte());
        key
    }

    fn push_byte(&mut self, byte: u8) {
        self.bytes[self.len] = byte;
        self.len += 1;
    }

    fn extend_from_slice(&mut self, bytes: &[u8]) {
        let end = self.len + bytes.len();
        self.bytes[self.len..end].copy_from_slice(bytes);
        self.len = end;
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl std::ops::Deref for StorageKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

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

fn key(namespace: Namespace, parts: &[&[u8]]) -> StorageKey {
    let mut out = StorageKey::new(namespace);
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
pub(crate) fn canonical_fact_key(relation_id: u32, fact_bytes: &[u8]) -> StorageKey {
    key(
        Namespace::CanonicalFact,
        &[&u32_bytes(relation_id), fact_bytes],
    )
}

/// `H | relation_id | fact_handle -> fact_bytes`.
pub(crate) fn fact_handle_key(relation_id: u32, handle: FactHandle) -> StorageKey {
    key(
        Namespace::FactHandle,
        &[&u32_bytes(relation_id), &handle_bytes(handle)],
    )
}

/// `L | relation_id | fact_handle -> empty`.
pub(crate) fn live_row_key(relation_id: u32, handle: FactHandle) -> StorageKey {
    key(
        Namespace::LiveRow,
        &[&u32_bytes(relation_id), &handle_bytes(handle)],
    )
}

/// `C | relation_id | field_id | fact_handle -> encoded_field_bytes`.
pub(crate) fn column_key(relation_id: u32, field_id: u32, handle: FactHandle) -> StorageKey {
    key(
        Namespace::Column,
        &[
            &u32_bytes(relation_id),
            &u32_bytes(field_id),
            &handle_bytes(handle),
        ],
    )
}

/// `C | relation_id | field_id` prefix for sequential column scans.
pub(crate) fn column_prefix_key(relation_id: u32, field_id: u32) -> StorageKey {
    key(
        Namespace::Column,
        &[&u32_bytes(relation_id), &u32_bytes(field_id)],
    )
}

/// Decodes the fact handle suffix from a full column key.
pub(crate) fn decode_column_key_handle(key: &[u8]) -> Option<FactHandle> {
    const COLUMN_PREFIX_LEN: usize = 1 + 4 + 4;
    const COLUMN_KEY_LEN: usize = COLUMN_PREFIX_LEN + 16;
    if key.len() != COLUMN_KEY_LEN {
        return None;
    }
    let handle = key
        .get(COLUMN_PREFIX_LEN..COLUMN_KEY_LEN)?
        .try_into()
        .ok()?;
    Some(FactHandle(handle))
}

/// `Q | relation_id | field_id -> next_u64`.
pub(crate) fn serial_sequence_key(relation_id: u32, field_id: u32) -> StorageKey {
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
) -> StorageKey {
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
pub(crate) fn reverse_fk_guard_prefix(
    target_relation_id: u32,
    target_constraint: &str,
    target_key_bytes: &[u8],
) -> StorageKey {
    key(
        Namespace::ReverseForeignKeyGuard,
        &[
            &u32_bytes(target_relation_id),
            target_constraint.as_bytes(),
            b"\0",
            target_key_bytes,
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
) -> StorageKey {
    let mut out = reverse_fk_guard_prefix(target_relation_id, target_constraint, target_key_bytes);
    out.extend_from_slice(&u32_bytes(source_relation_id));
    out.extend_from_slice(source_constraint.as_bytes());
    out.extend_from_slice(b"\0");
    out.extend_from_slice(&handle_bytes(source_handle));
    out
}

/// `A | relation_id | accelerator_id | tuple_key | fact_handle -> empty`.
pub(crate) fn accelerator_key(
    relation_id: u32,
    accelerator_id: u32,
    tuple_key: &[u8],
    handle: FactHandle,
) -> StorageKey {
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
pub(crate) fn stats_key(relation_id: u32, stat_name: &str) -> StorageKey {
    key(
        Namespace::Stats,
        &[&u32_bytes(relation_id), stat_name.as_bytes()],
    )
}

#[cfg(test)]
#[path = "storage_format_tests.rs"]
mod tests;
