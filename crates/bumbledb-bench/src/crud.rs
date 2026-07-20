//! The `crud` home-turf world — the OLTP regime where `SQLite` is
//! expected to be strong; we bench to lose honestly where we lose. This
//! module is the world's foundation: the schema, the sizes, (in
//! [`corpus`]) the seeded rows and the durability-paired twin loader,
//! the precomputed op streams ([`ops`]), the family runners
//! ([`lanes`]), the family registry ([`families`]), the orchestration
//! fold ([`run`]), and the artifact renderers ([`render`]). Everything
//! here is REPORT-class infrastructure — no budget gate ever reads a
//! crud number.
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
use crate::harness::Protocol;

pub mod corpus;
pub mod lanes;
pub mod ops;
pub mod render;
pub mod run;
#[cfg(test)]
mod tests;

pub use run::{CrudRow, run, run_with};

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

/// One registered crud family: the name reports print, the honest
/// one-line description, and the registered protocol. The protocol is
/// DATA handed to the runners ([`lanes`]) at orchestration time, never
/// baked into a runner — tests run the same runners under tiny
/// protocols.
#[derive(Debug, Clone, Copy)]
pub struct CrudFamily {
    pub name: &'static str,
    pub about: &'static str,
    pub protocol: Protocol,
}

/// The eleven crud families in THE run order — reads before writes,
/// the bench-run law: `crud_read_point` measures the loaded corpus
/// before any write family mutates it, and the registry order IS the
/// run order (the orchestration iterates this slice, never reorders).
/// The delete pool (4 096 at every timed scale) covers the largest
/// registered write protocol (8 + 64 = 72 invocations) with room —
/// the pool-size ≥ warmups+samples invariant, re-asserted at runner
/// entry.
#[must_use]
pub fn families() -> &'static [CrudFamily] {
    &[
        CrudFamily {
            name: "crud_read_point",
            about: "keyed point read: (id, val) by key, 3 hits + 1 miss rotation",
            protocol: Protocol {
                warmups: 32,
                samples: 256,
            },
        },
        CrudFamily {
            name: "crud_insert",
            about: "one fresh Doc row per commit (fsync-bound single-writer floor)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_insert_10",
            about: "10 fresh Doc rows per commit",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_insert_100",
            about: "100 fresh Doc rows per commit",
            protocol: Protocol {
                warmups: 4,
                samples: 32,
            },
        },
        CrudFamily {
            name: "crud_insert_1k",
            about: "1000 fresh Doc rows per commit",
            protocol: Protocol {
                warmups: 2,
                samples: 16,
            },
        },
        CrudFamily {
            name: "crud_update",
            about: "one keyed Counter value replacement per commit",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_update_hot",
            about: "the same replacement pinned to one hot row (key 0 every sample)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_upsert",
            about: "keyed upsert over twice the Counter mass (~half miss)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_rmw",
            about: "read-modify-write round trip: point read, host +1, write back",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_delete",
            about: "one pool-row delete per commit (delete-bearing by contract)",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        CrudFamily {
            name: "crud_mixed_90_10",
            about: "9 point reads + 1 single-row insert commit per sample",
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
    ]
}
