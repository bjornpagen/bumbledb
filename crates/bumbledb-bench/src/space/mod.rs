//! Physical accounting census (chapter 40; SPACE-01 / PERF-002).
//!
//! File length is not resident RAM, allocated blocks, live pages, a virtual
//! map, or a per-namespace attribution. Each quantity is reported as itself.
//! Mixed namespaces share pages — page numbers are store-wide; namespace
//! bytes are key/value walks. Recalculated from the live layout in
//! `crates/bumbledb/src/storage/store/keys.rs` and `schema/compiled.rs`.

pub mod census;
pub mod sqlite_stat;
pub mod store_source;
#[cfg(test)]
mod tests;
pub mod variants;

/// Live persisted namespaces. Dictionary and reverse-edge entries are gone
/// from the successor store; they survive only as historical attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Namespace {
    /// Fact rows: `tag 1 + relation 4 + local row 8` key, canonical payload.
    Fact,
    /// Membership: `(relation, 16-byte fingerprint, row id) → ()`.
    Membership,
    /// Determinant: `(projection id, routing, optional interval tail, row) → ()`.
    Determinant,
    /// Host / meta database records (not multiplied by live fact count).
    HostMeta,
    /// Unexpected data-database tag — reported, never silently dropped.
    Unknown,
}

pub const NAMESPACES: [Namespace; 5] = [
    Namespace::Fact,
    Namespace::Membership,
    Namespace::Determinant,
    Namespace::HostMeta,
    Namespace::Unknown,
];

impl Namespace {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Membership => "membership",
            Self::Determinant => "determinant",
            Self::HostMeta => "host-meta",
            Self::Unknown => "unknown",
        }
    }

    /// Classify one `OwnedSnapshot::entry_census` record.
    #[must_use]
    pub const fn from_census_tag(is_meta: bool, tag: u8) -> Self {
        if is_meta {
            return Self::HostMeta;
        }
        match tag {
            0x01 => Self::Fact,
            0x02 => Self::Membership,
            0x03 => Self::Determinant,
            _ => Self::Unknown,
        }
    }
}

/// Live raw key/value model. Discriminator is interned [`ProjectionId`],
/// not a declaration-order statement number. Values for membership and
/// determinant entries are empty.
pub mod current_layout {
    /// Fact key: tag 1 + relation 4 + local row 8.
    pub const ROW_KEY: u64 = 13;
    /// Membership key: tag 1 + relation 4 + fingerprint 16 + local row 8.
    pub const MEMBERSHIP_KEY: u64 = 29;
    pub const MEMBERSHIP_VALUE: u64 = 0;
    pub const MEMBERSHIP_ENTRY: u64 = MEMBERSHIP_KEY + MEMBERSHIP_VALUE;
    /// Determinant overhead: tag 1 + projection id 2 + row surrogate 8.
    pub const DETERMINANT_OVERHEAD: u64 = 11;
    /// Exact u64 routing width (C1: scalar grouping ≤16 encoded bytes).
    pub const EXACT_U64_ROUTING: u64 = 8;
    /// Fingerprint routing width (BLAKE3 truncated, exact-checked).
    pub const FINGERPRINT_ROUTING: u64 = 16;
    /// Application Id128 stored width (not a physical row id).
    pub const ID128_WIDTH: u64 = 16;

    #[must_use]
    pub const fn fact_entry(payload: u64) -> u64 {
        ROW_KEY + payload
    }

    #[must_use]
    pub const fn determinant_entry(routing: u64, interval_tail: u64) -> u64 {
        DETERMINANT_OVERHEAD + routing + interval_tail
    }

    #[must_use]
    pub const fn determinant_exact_u64() -> u64 {
        determinant_entry(EXACT_U64_ROUTING, 0)
    }

    #[must_use]
    pub const fn determinant_fingerprint() -> u64 {
        determinant_entry(FINGERPRINT_ROUTING, 0)
    }

    /// Raw key bytes for one fact + membership + one fingerprint determinant
    /// (chapter 40: 13+29+27 = 69) — payload is extra.
    pub const KEY_BYTES_FACT_MEMBERSHIP_FP_DET: u64 = ROW_KEY + MEMBERSHIP_KEY + 27;

    #[must_use]
    pub const fn fact_plus_membership(payload: u64) -> u64 {
        fact_entry(payload) + MEMBERSHIP_ENTRY
    }

    /// One fact, membership, and one fingerprint determinant, including payload.
    #[must_use]
    pub const fn fact_membership_fp_det(payload: u64) -> u64 {
        fact_plus_membership(payload) + determinant_fingerprint()
    }
}

/// Historical 0.x bill (README 2026-08-22 attribution only). Not the
/// successor layout and not a prediction of current file size.
pub mod historical_layout {
    #[must_use]
    pub const fn fact_entry(fact_width: u64) -> u64 {
        13 + fact_width
    }

    /// 32-byte membership digest in the key + 8-byte row value.
    pub const MEMBERSHIP_ENTRY: u64 = 45;

    #[must_use]
    pub const fn determinant_entry(determinant_width: u64) -> u64 {
        15 + determinant_width
    }

    #[must_use]
    pub const fn reverse_edge_entry(determinant_width: u64, weighted: bool) -> u64 {
        15 + determinant_width + if weighted { 8 } else { 0 }
    }

    pub const DICT_FORWARD_ENTRY: u64 = 41;

    #[must_use]
    pub const fn dict_reverse_entry(text_len: u64) -> u64 {
        9 + text_len
    }

    #[must_use]
    pub const fn dict_total(text_len: u64) -> u64 {
        DICT_FORWARD_ENTRY + dict_reverse_entry(text_len)
    }

    #[must_use]
    pub const fn fact_plus_membership(fact_width: u64) -> u64 {
        fact_entry(fact_width) + MEMBERSHIP_ENTRY
    }
}

/// Compatibility names for existing SPACE tests. Prefer [`current_layout`].
pub mod audited_layout {
    pub use super::historical_layout::*;
}

pub mod successor_layout {
    pub const MEMBERSHIP_ENTRY: u64 = super::current_layout::MEMBERSHIP_ENTRY;
    pub const MEMBERSHIP_SAVING_PER_FACT: u64 =
        super::historical_layout::MEMBERSHIP_ENTRY - MEMBERSHIP_ENTRY;
}
