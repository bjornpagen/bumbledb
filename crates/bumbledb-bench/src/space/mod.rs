//! The physical accounting census (chapter 41; gates SPACE-01..SPACE-02).
//!
//! The existing [`crate::lanes::storage`] lane measures file lengths under a
//! matched-durability protocol; keep it. This module adds what file length
//! cannot answer:
//!
//! - per-namespace live key/value byte accounting (F/M/U/R/dictionary/
//!   metadata) so the recorded 2.3–2.45× indexed-SQLite gap gets attributed
//!   instead of labeled a Free Join tax ([`census`]),
//! - the raw-bytes arithmetic model of the current and successor entry
//!   layouts, cross-checked against chapter 41's tables (this file),
//! - LMDB page statistics, OS-allocated blocks and the resident/disk split
//!   ([`census`]),
//! - SQLite page/freelist/index-roster accounting so "indexed" is an actual
//!   roster, not a label ([`sqlite_stat`]),
//! - the SPACE-02 before/after layout-variant matrix ([`variants`]).
//!
//! Execution happens only in F3 (SPACE lanes need the integrated successor
//! store); the arithmetic and report shapes are complete and tested now.
//! File length is not resident RAM, allocated blocks, live pages or a
//! per-namespace attribution; each is reported as itself. Mixed namespaces
//! share pages, so page-level numbers are store-wide with namespace bytes
//! reported at the key/value level — no invented per-namespace page split.

pub mod census;
pub mod sqlite_stat;
#[cfg(test)]
mod tests;
pub mod variants;

/// The persisted entry namespaces of the audited layout (chapter 41's bill).
/// The successor may delete some (dictionary) — the census reports zeros
/// rather than dropping the row, so a deletion shows up as evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Namespace {
    /// Fact rows: `tag 1 + relation 4 + local row 8` key, encoded fact value.
    Fact,
    /// Membership: fingerprint-keyed row references.
    Membership,
    /// Key determinant entries.
    Determinant,
    /// Reverse containment/capacity edges.
    ReverseEdge,
    /// Text dictionary, forward (digest → intern id). Selected for deletion.
    DictionaryForward,
    /// Text dictionary, reverse (intern id → text). Selected for deletion.
    DictionaryReverse,
    /// Counters/metadata — per relation/field, never multiplied by row count.
    Metadata,
}

pub const NAMESPACES: [Namespace; 7] = [
    Namespace::Fact,
    Namespace::Membership,
    Namespace::Determinant,
    Namespace::ReverseEdge,
    Namespace::DictionaryForward,
    Namespace::DictionaryReverse,
    Namespace::Metadata,
];

impl Namespace {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Membership => "membership",
            Self::Determinant => "determinant",
            Self::ReverseEdge => "reverse-edge",
            Self::DictionaryForward => "dictionary-forward",
            Self::DictionaryReverse => "dictionary-reverse",
            Self::Metadata => "metadata",
        }
    }
}

/// Raw per-entry byte model of the **audited** layout (chapter 41's table).
/// `W` = encoded fact width, `d` = projected key width, text = UTF-8 length.
/// These are raw key+value bytes before LMDB node/slot/page overhead.
pub mod audited_layout {
    /// Fact entry: key 13 (tag 1 + relation 4 + local row 8) + value `W`.
    #[must_use]
    pub const fn fact_entry(fact_width: u64) -> u64 {
        13 + fact_width
    }

    /// Membership entry: key 37 (tag 1 + relation 4 + digest 32) + value 8.
    pub const MEMBERSHIP_ENTRY: u64 = 45;

    /// Determinant entry: key `7 + d` (tag 1 + relation 4 + statement 2 + d)
    /// + value 8.
    #[must_use]
    pub const fn determinant_entry(determinant_width: u64) -> u64 {
        15 + determinant_width
    }

    /// Reverse edge: key `15 + d`, value 0 (unweighted) or 8 (weighted).
    #[must_use]
    pub const fn reverse_edge_entry(determinant_width: u64, weighted: bool) -> u64 {
        15 + determinant_width + if weighted { 8 } else { 0 }
    }

    /// Dictionary forward: key 33 (tag 1 + digest 32) + value 8.
    pub const DICT_FORWARD_ENTRY: u64 = 41;

    /// Dictionary reverse: key 9 (tag 1 + intern 8) + UTF-8 text value.
    #[must_use]
    pub const fn dict_reverse_entry(text_len: u64) -> u64 {
        9 + text_len
    }

    /// Both dictionary sides for one distinct historical string:
    /// `50 + text length`, plus an 8-byte intern reference per occurrence.
    #[must_use]
    pub const fn dict_total(text_len: u64) -> u64 {
        DICT_FORWARD_ENTRY + dict_reverse_entry(text_len)
    }

    /// Every ordinary fact costs `W + 58` raw bytes for F+M before
    /// determinants, edges, dictionary and page overhead.
    #[must_use]
    pub const fn fact_plus_membership(fact_width: u64) -> u64 {
        fact_entry(fact_width) + MEMBERSHIP_ENTRY
    }
}

/// Raw per-entry model of the **selected successor** membership layout:
/// `(relation, 16-byte fingerprint, local-row-id) → empty` — the row ID moves
/// from the value into the key and is not duplicated (chapter 41 item 2).
pub mod successor_layout {
    /// Membership entry: key 29 (tag 1 + relation 4 + fingerprint 16 +
    /// local row 8) + empty value.
    pub const MEMBERSHIP_ENTRY: u64 = 29;

    /// Raw bytes saved per fact against the audited 45-byte membership entry:
    /// exactly the 16 truncated digest bytes.
    pub const MEMBERSHIP_SAVING_PER_FACT: u64 =
        super::audited_layout::MEMBERSHIP_ENTRY - MEMBERSHIP_ENTRY;
}
