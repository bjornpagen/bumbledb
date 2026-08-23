//! The replication equality oracle: blake3 over the raw ordered
//! enumeration of every data entry then every dictionary entry — for each
//! entry in key order, key length (u64 LE), key bytes, value length
//! (u64 LE), value bytes. Equal digests imply identical catalog content
//! regardless of LMDB page layout. Harness-tier (the
//! [`Db::verify_store`] class): one sequential pass, off any hot path.
//! `_meta` stays out on purpose — the generation and the schema
//! fingerprint are carried and verified by their own consumers, and the
//! digest answers exactly one question: is the judged content the same?
use super::{Db, OwnedInstance};
use crate::error::Result;
use crate::storage::catalog::{Bounds, CatalogMap, OrderedRead, ReadCursor};

impl<S> Db<S> {
    /// # Errors
    #[doc(hidden)]
    pub fn catalog_digest(&self) -> Result<[u8; 32]> {
        let txn = self.env().read_txn()?;
        digest_catalog(&txn.catalog())
    }
}

impl<S> OwnedInstance<S> {
    /// # Errors
    #[doc(hidden)]
    pub fn catalog_digest(&self) -> Result<[u8; 32]> {
        digest_catalog(self.catalog())
    }
}

/// One implementation for both backends: the digest is a fold over the
/// [`OrderedRead`] contract, so the LMDB and packed-heap answers can only
/// disagree where the catalogs themselves disagree.
fn digest_catalog<C: OrderedRead>(catalog: &C) -> Result<[u8; 32]> {
    let mut digest = crate::digest::Digest::new();
    for map in [CatalogMap::Data, CatalogMap::Dictionary] {
        let mut range = catalog.range(map, Bounds::all())?;
        while let Some(entry) = ReadCursor::next(&mut range)? {
            digest.update(&length_word(entry.key));
            digest.update(entry.key);
            digest.update(&length_word(entry.value));
            digest.update(entry.value);
        }
    }
    Ok(digest.finalize())
}

fn length_word(bytes: &[u8]) -> [u8; 8] {
    u64::try_from(bytes.len())
        .expect("entry length fits u64")
        .to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::super::InstanceBuilder;
    use super::*;
    use crate::ir::Value;
    use crate::testutil::TempDir;
    use bumbledb_theory::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
    };

    fn digest_schema() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Named".into(),
                    fields: vec![FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    }],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Num".into(),
                    fields: vec![FieldDescriptor {
                        name: "n".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    }],
                },
            ],
            statements: vec![],
        }
    }

    const NAMED: RelationId = RelationId(0);
    const NUM: RelationId = RelationId(1);

    fn named(value: &str) -> Vec<Value> {
        vec![Value::String(value.into())]
    }

    fn num(value: u64) -> Vec<Value> {
        vec![Value::U64(value)]
    }

    fn create(dir: &TempDir) -> Db<SchemaDescriptor> {
        Db::create(dir.path(), digest_schema())
            .expect("create")
            .expect("accepted")
    }

    fn seed(db: &Db<SchemaDescriptor>) {
        db.write(|tx| {
            tx.insert_dyn(NAMED, [&named("x"), &named("y")])?;
            tx.insert_dyn(NUM, [&num(1)])?;
            Ok(())
        })
        .expect("seed")
        .unwrap();
    }

    #[test]
    fn equal_content_digests_equal_and_repeat_calls_agree() {
        let dir_a = TempDir::new("digest-equal-a");
        let dir_b = TempDir::new("digest-equal-b");
        let a = create(&dir_a);
        let b = create(&dir_b);
        assert_eq!(
            a.catalog_digest().expect("digest"),
            b.catalog_digest().expect("digest"),
            "two empty stores of one schema hold one catalog content"
        );
        seed(&a);
        seed(&b);
        let first = a.catalog_digest().expect("digest");
        assert_eq!(first, a.catalog_digest().expect("digest"));
        assert_eq!(first, b.catalog_digest().expect("digest"));
    }

    #[test]
    fn different_content_digests_differ() {
        let dir_a = TempDir::new("digest-differ-a");
        let dir_b = TempDir::new("digest-differ-b");
        let a = create(&dir_a);
        let b = create(&dir_b);
        seed(&a);
        let empty = b.catalog_digest().expect("digest");
        assert_ne!(a.catalog_digest().expect("digest"), empty);
        seed(&b);
        b.write(|tx| tx.insert_dyn(NUM, [&num(2)]).map(|_| ()))
            .expect("extra")
            .unwrap();
        assert_ne!(
            a.catalog_digest().expect("digest"),
            b.catalog_digest().expect("digest")
        );
    }

    #[test]
    fn db_and_owned_instance_twins_agree_on_equal_catalogs() {
        let dir = TempDir::new("digest-twin");
        let db = create(&dir);
        seed(&db);

        let mut builder = InstanceBuilder::new(digest_schema()).expect("builder");
        builder
            .load_dyn(NAMED, [&named("x"), &named("y")])
            .expect("load");
        builder.load_dyn(NUM, [&num(1)]).expect("load");
        let instance = builder.admit().expect("admit").expect("accepted");

        assert_eq!(
            db.catalog_digest().expect("digest"),
            instance.catalog_digest().expect("digest"),
            "one fact set, two backends, one catalog content"
        );
    }
}
