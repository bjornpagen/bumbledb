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
    /// Fact rows: tag + schema-fixed relation ordinal + selected exact home
    /// (zero bytes for unclustered relations) + eight-byte row ordinal.
    Fact,
    /// Fingerprint membership for relations without an eligible exact
    /// scalar-key index: `(relation, 16-byte fingerprint, row id) → ()`.
    Membership,
    /// Secondary determinant: `(projection id, routing, optional interval tail,
    /// row ordinal) → home bytes`. The selected primary has no separate entry.
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
        use bumbledb::store::PhysicalKeyKind;
        match PhysicalKeyKind::from_census_tag(is_meta, tag) {
            PhysicalKeyKind::Row => Self::Fact,
            PhysicalKeyKind::Membership => Self::Membership,
            PhysicalKeyKind::Determinant => Self::Determinant,
            PhysicalKeyKind::Metadata => Self::HostMeta,
            PhysicalKeyKind::Unknown => Self::Unknown,
        }
    }
}

/// Live raw key/value model. Discriminator is interned [`ProjectionId`],
/// not a declaration-order statement number. Membership values are empty;
/// secondary determinant values carry the relation's selected home bytes.
pub mod current_layout {
    pub use bumbledb::store::PhysicalKeyWidths;
    /// Raw bytes for one row, membership entry and fingerprint determinant.
    /// A hypothetical shape, not a per-row bill for all relations: eligible
    /// scalar-key relations cluster their bodies by that key, omit membership
    /// and the selected determinant, and put home bytes in secondary values.
    #[must_use]
    pub fn fact_membership_fp_det(widths: PhysicalKeyWidths, payload: u64) -> u64 {
        (widths.row + widths.membership + widths.determinant_overhead + bumbledb::store::FP_LEN)
            as u64
            + payload
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
    #[must_use]
    pub fn membership_saving_per_fact(widths: bumbledb::store::PhysicalKeyWidths) -> u64 {
        super::historical_layout::MEMBERSHIP_ENTRY - widths.membership as u64
    }
}
