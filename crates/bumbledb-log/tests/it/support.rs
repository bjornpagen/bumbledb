//! Shared fixtures: a two-relation schema, change sets over it, and a
//! seeded generator.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{ChangeSet, RelationId, Schema, SchemaDescriptor, Theory as _, Value, WorkContext};
use bumbledb_log::{Bundle, MigrationHash, MigrationId};

mod v1 {
    bumbledb::schema! {
        pub Inventory;
        relation Item { id: u64, count: u64 }
        Item(id) -> Item;
    }
}

mod v2 {
    bumbledb::schema! {
        pub Inventory;
        relation Item { id: u64, count: u64 }
        relation Tag { item: u64, label: u64 }
        Item(id) -> Item;
        Tag(item) -> Tag;
    }
}

pub const ITEM: RelationId = RelationId(0);
pub const TAG: RelationId = RelationId(1);

pub fn descriptor() -> SchemaDescriptor {
    v1::Inventory.descriptor()
}

pub fn descriptor2() -> SchemaDescriptor {
    v2::Inventory.descriptor()
}

pub fn schema() -> Schema {
    descriptor().validate().expect("fixture schema")
}

pub fn schema2() -> Schema {
    descriptor2().validate().expect("fixture schema")
}

pub fn migration_id(name: &str) -> MigrationId {
    MigrationId {
        name: name.into(),
        hash: MigrationHash(*blake3::hash(name.as_bytes()).as_bytes()),
    }
}

/// The initial schema only.
pub fn bundle() -> Bundle {
    Bundle::new(vec![(migration_id("0000_init"), descriptor())]).expect("fixture bundle")
}

/// The initial schema, then one migration adding `Tag`.
pub fn bundle2() -> Bundle {
    Bundle::new(vec![
        (migration_id("0000_init"), descriptor()),
        (migration_id("0001_tags"), descriptor2()),
    ])
    .expect("fixture bundle")
}

/// One change set over `Item`: `adds` and `removes` as `(id, count)`.
pub fn items(schema: &Schema, adds: &[(u64, u64)], removes: &[(u64, u64)]) -> ChangeSet {
    let mut builder = ChangeSet::builder(schema, WorkContext::new());
    for &(id, count) in removes {
        builder
            .delete(ITEM, &[Value::U64(id), Value::U64(count)])
            .expect("remove");
    }
    for &(id, count) in adds {
        builder
            .insert(ITEM, &[Value::U64(id), Value::U64(count)])
            .expect("add");
    }
    builder.finish().expect("change set")
}

/// One change set adding `Tag` rows `(item, label)`.
pub fn tags(schema: &Schema, adds: &[(u64, u64)]) -> ChangeSet {
    let mut builder = ChangeSet::builder(schema, WorkContext::new());
    for &(item, label) in adds {
        builder
            .insert(TAG, &[Value::U64(item), Value::U64(label)])
            .expect("add");
    }
    builder.finish().expect("change set")
}

/// splitmix64: a seeded, platform-independent generator.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub fn bytes16(&mut self) -> [u8; 16] {
        let mut out = [0; 16];
        out[..8].copy_from_slice(&self.next().to_be_bytes());
        out[8..].copy_from_slice(&self.next().to_be_bytes());
        out
    }
}
