//! The `crud` home-turf world — the OLTP regime where `SQLite` is
//! expected to be strong; we bench to lose honestly where we lose. This
//! module is the world's foundation: the schema, the sizes, and (in
//! [`corpus`]) the seeded rows and the durability-paired twin loader.
//! Lanes live elsewhere; everything here is REPORT-class infrastructure.
//!
//! The world's shape: `Doc` — a keyed row store (fresh id, a u64 `key`
//! under a scalar key statement, an i64 `val`, a 32-byte payload — the
//! points-world identity shape); `Counter` — a keyed accumulator (the
//! upsert lane's target). Both key statements render as UNIQUE indexes
//! on the mirror ([`crate::sqlmap::schema_ddl`]) — the `ON CONFLICT`
//! targets the write lanes need.
//!
//! Post-state verification is the shared comparator
//! ([`crate::poststate`]); the durability pairing is the closed lane sum
//! ([`crate::duralane::DurabilityLane`]) — both worlds' config from one
//! constructor, cross-matched pairs unrepresentable.

use crate::corpus_gen::Scale;

pub mod corpus;
#[cfg(test)]
mod tests;

bumbledb::schema! {
    pub CrudWorld;

    relation Doc {
        id: u64 as CrudDocId, fresh,
        key: u64,
        val: i64,
        payload: bytes<32>,
    }
    relation Counter {
        key: u64,
        val: i64,
    }

    Doc(key) -> Doc;
    Counter(key) -> Counter;
}

/// Relation ids by declaration order.
pub mod ids {
    use bumbledb::RelationId;

    pub const DOC: RelationId = RelationId(0);
    pub const COUNTER: RelationId = RelationId(1);
}

/// The validated crud schema, memoized for the mirror's DDL and the
/// comparator's field walks; the store is created from [`CrudWorld`]'s
/// descriptor ([`corpus::load_stores`]).
///
/// # Panics
///
/// Never in practice: the declared crud schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    use bumbledb::schema::ValidateDescriptor as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        CrudWorld
            .descriptor()
            .validate()
            .expect("the crud schema is valid")
    })
}

/// The crud corpus shape. `delete_pool` rows live at `Doc` ids/keys
/// `docs..docs+delete_pool` and exist to be deleted by the delete lane;
/// fresh minting after load therefore starts at `docs + delete_pool` on
/// BOTH engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrudSizes {
    /// The standing `Doc` mass — the rows the point lanes read and
    /// update.
    pub docs: u64,
    /// `Counter` rows — the upsert lane's key space.
    pub counters: u64,
    /// Extra `Doc` rows loaded above `docs`, reserved for the delete
    /// lane to consume (each is a pure function of `(seed, i)`, so the
    /// lane re-derives the full fact to hand `tx.delete`).
    pub delete_pool: u64,
}

impl CrudSizes {
    /// Two size points, the scratch-world precedent: `Tiny` for tests
    /// and the parity slice, one OLTP shape for every timed scale.
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        match scale {
            Scale::Tiny => Self {
                docs: 1_024,
                counters: 64,
                delete_pool: 256,
            },
            Scale::S | Scale::M | Scale::L => Self {
                docs: 200_000,
                counters: 4_096,
                delete_pool: 4_096,
            },
        }
    }
}
