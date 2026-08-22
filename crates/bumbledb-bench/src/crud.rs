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

pub mod ids {
    use bumbledb::RelationId;

    pub const DOC: RelationId = RelationId(0);
    pub const COUNTER: RelationId = RelationId(1);
}

/// # Panics
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

/// fresh minting after load therefore starts at `docs + delete_pool` on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrudSizes {
    pub docs: u64,

    pub counters: u64,

    pub delete_pool: u64,
}

impl CrudSizes {
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

/// one-line description, and the registered protocol. The protocol is
#[derive(Debug, Clone, Copy)]
pub struct CrudFamily {
    pub name: &'static str,
    pub about: &'static str,
    pub protocol: Protocol,
}

/// The eleven crud families in THE run order — reads before writes, before any
/// write family mutates it, and the registry order IS the registered write
/// protocol (8 + 64 = 72 invocations) with room — the pool-size ≥
/// warmups+samples invariant, re-asserted at runner
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
