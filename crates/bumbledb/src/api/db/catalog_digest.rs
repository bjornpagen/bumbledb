//! The replication equality oracle: blake3 over the canonical
//! enumeration of the catalog — every data entry rendered with allocator
//! row ids quotiented to fact identity (the `M` hash), sorted, then every
//! dictionary entry in key order; for each entry, key length (u64 LE),
//! key bytes, value length (u64 LE), value bytes. Row ids are per-relation
//! commit-order counters (`StatKind::RowIdHighWater`), so raw `F`/`R` keys
//! and `M`/`U` values differ across the apply-order swaps L8 proves
//! state-equal; the quotient makes the digest a function of the judged
//! content alone, which is the one question it answers. Equal digests
//! imply identical judged content regardless of LMDB page layout or
//! allocation history. Harness-tier (the [`Db::verify_store`] class): a
//! bounded number of sequential passes, off any hot path. `_meta` stays
//! out on purpose — the generation and the schema fingerprint are carried
//! and verified by their own consumers.
use std::collections::BTreeMap;

use super::{Db, OwnedInstance};
use crate::error::{CorruptionError, Error, Result};
use crate::storage::catalog::{Bounds, CatalogMap, OrderedRead, ReadCursor};
use crate::storage::{keys, stored_u64};
use bumbledb_theory::schema::RelationId;

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
    let identities = row_identities(catalog)?;
    let mut entries = Vec::new();
    {
        let mut range = catalog.range(CatalogMap::Data, Bounds::all())?;
        while let Some(entry) = ReadCursor::next(&mut range)? {
            entries.push(canonical_entry(entry.key, entry.value, &identities)?);
        }
    }
    entries.sort_unstable();

    let mut digest = crate::digest::Digest::new();
    for (key, value) in &entries {
        digest.update(&length_word(key));
        digest.update(key);
        digest.update(&length_word(value));
        digest.update(value);
    }
    let mut range = catalog.range(CatalogMap::Dictionary, Bounds::all())?;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        digest.update(&length_word(entry.key));
        digest.update(entry.key);
        digest.update(&length_word(entry.value));
        digest.update(entry.value);
    }
    Ok(digest.finalize())
}

/// Every stored row's identity, read off the `M` namespace: the fact hash
/// is state-independent where the row id is allocation history.
fn row_identities<C: OrderedRead>(catalog: &C) -> Result<BTreeMap<(u32, u64), [u8; 32]>> {
    let mut identities = BTreeMap::new();
    let mut range = catalog.range(CatalogMap::Data, Bounds::all())?;
    while let Some(entry) = ReadCursor::next(&mut range)? {
        if let Some((relation, hash)) = keys::parse_membership_key(entry.key) {
            let row_id = stored_u64(entry.value, "M row id")?;
            identities.insert((relation.0, row_id), *hash);
        }
    }
    Ok(identities)
}

/// One data entry's canonical rendering. Row ids appear in exactly four
/// places — `F` key tails, `M` values, `U` values, `R` key source rows —
/// and each is replaced by the row's fact hash (`M` values by nothing:
/// the key already carries the hash). Everything else digests raw,
/// unparseable keys included, so a corrupt store still digests
/// deterministically.
fn canonical_entry(
    key: &[u8],
    value: &[u8],
    identities: &BTreeMap<(u32, u64), [u8; 32]>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    if let Some((relation, row_id)) = keys::parse_fact_key(key) {
        let hash = identity(identities, relation, row_id)?;
        let mut canonical = Vec::with_capacity(1 + 4 + hash.len());
        canonical.push(keys::Namespace::Fact.tag());
        canonical.extend_from_slice(&relation.0.to_be_bytes());
        canonical.extend_from_slice(hash);
        return Ok((canonical, value.to_vec()));
    }
    if keys::parse_membership_key(key).is_some() {
        return Ok((key.to_vec(), Vec::new()));
    }
    if let Some((relation, _, _)) = keys::parse_determinant_key(key) {
        let row_id = stored_u64(value, "U row id")?;
        let hash = identity(identities, relation, row_id)?;
        return Ok((key.to_vec(), hash.to_vec()));
    }
    if let Some((statement, key_bytes, source_relation, source_row)) = keys::parse_reverse_key(key)
    {
        let hash = identity(identities, source_relation, source_row)?;
        let mut canonical = Vec::with_capacity(1 + 2 + key_bytes.len() + 4 + hash.len());
        canonical.push(keys::Namespace::Reverse.tag());
        canonical.extend_from_slice(&statement.0.to_be_bytes());
        canonical.extend_from_slice(key_bytes);
        canonical.extend_from_slice(&source_relation.0.to_be_bytes());
        canonical.extend_from_slice(hash);
        return Ok((canonical, value.to_vec()));
    }
    Ok((key.to_vec(), value.to_vec()))
}

fn identity(
    identities: &BTreeMap<(u32, u64), [u8; 32]>,
    relation: RelationId,
    row_id: u64,
) -> Result<&[u8; 32]> {
    identities
        .get(&(relation.0, row_id))
        .ok_or(Error::Corruption(CorruptionError::MembershipDesync {
            relation,
            row_id,
        }))
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
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
        Side, StatementDescriptor, ValueType,
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

    /// The host-order pin: with the batch's intern first-use sequence
    /// held fixed (every string committed beforehand), the same ops in
    /// two orders land byte-identical catalogs — the canonical
    /// `(relation, fact_hash)` plan sort owns apply order, so op order
    /// inside a batch cannot reach stored bytes.
    #[test]
    fn op_order_inside_a_batch_cannot_influence_stored_bytes() {
        let dir_a = TempDir::new("digest-order-a");
        let dir_b = TempDir::new("digest-order-b");
        let a = create(&dir_a);
        let b = create(&dir_b);
        seed(&a);
        seed(&b);
        let before = a.catalog_digest().expect("digest");

        a.write(|tx| {
            tx.insert_dyn(NUM, [&num(2)])?;
            tx.insert_dyn(NUM, [&num(3)])?;
            tx.delete_dyn(NAMED, [&named("x")])?;
            Ok(())
        })
        .expect("forward order")
        .unwrap();
        b.write(|tx| {
            tx.delete_dyn(NAMED, [&named("x")])?;
            tx.insert_dyn(NUM, [&num(3)])?;
            tx.insert_dyn(NUM, [&num(2)])?;
            Ok(())
        })
        .expect("reversed order")
        .unwrap();

        let after_a = a.catalog_digest().expect("digest");
        assert_ne!(before, after_a, "the batch changed content");
        assert_eq!(after_a, b.catalog_digest().expect("digest"));
    }

    /// A schema whose statements exercise every row-id carrier: an FD on
    /// the child relation (`U` values), a containment from child to
    /// parent (`R` source rows), and a capacity over weighted children
    /// (`R` values beside them).
    fn statement_schema() -> SchemaDescriptor {
        let field = |name: &str| FieldDescriptor {
            name: name.into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        };
        let side = |relation: RelationId, fields: &[u16]| Side {
            relation,
            projection: fields.iter().map(|f| FieldId(*f)).collect(),
            selection: Box::from([]),
        };
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "parent".into(),
                    fields: vec![field("id")],
                },
                RelationDescriptor {
                    extension: None,
                    name: "child".into(),
                    fields: vec![field("parent"), field("slot"), field("units")],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: RelationId(0),
                    projection: Box::from([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: RelationId(1),
                    projection: Box::from([FieldId(1)]),
                },
                StatementDescriptor::Containment {
                    source: side(RelationId(1), &[0]),
                    target: side(RelationId(0), &[0]),
                },
                StatementDescriptor::Capacity {
                    target: side(RelationId(0), &[0]),
                    weight: bumbledb_theory::schema::Weight::Field(FieldId(2)),
                    lo: 0,
                    hi: Some(bumbledb_theory::schema::Bound::Lit(100)),
                    source: side(RelationId(1), &[0]),
                },
            ],
        }
    }

    /// The cross-batch half of L8's representation pin: per-relation row
    /// ids are commit-order counters, so swapping two disjoint commits
    /// swaps the ids under every `F` key, `M`/`U` value, and `R` source
    /// row — and the canonical rendering must erase exactly that,
    /// landing one digest for one judged content.
    #[test]
    fn commit_order_across_batches_cannot_influence_the_digest() {
        let parent = |id: u64| vec![Value::U64(id)];
        let child = |group: u64, slot: u64, units: u64| {
            vec![Value::U64(group), Value::U64(slot), Value::U64(units)]
        };
        let create = |dir: &TempDir| {
            Db::create(dir.path(), statement_schema())
                .expect("create")
                .expect("accepted")
        };
        let seed = |db: &Db<SchemaDescriptor>| {
            db.write(|tx| {
                tx.insert_dyn(RelationId(0), [&parent(1), &parent(2)])?;
                Ok(())
            })
            .expect("seed")
            .unwrap();
        };
        let commit = |db: &Db<SchemaDescriptor>, row: &Vec<Value>| {
            db.write(|tx| tx.insert_dyn(RelationId(1), [row]).map(|_| ()))
                .expect("commit")
                .unwrap();
        };

        let dir_a = TempDir::new("digest-swap-a");
        let dir_b = TempDir::new("digest-swap-b");
        let a = create(&dir_a);
        let b = create(&dir_b);
        seed(&a);
        seed(&b);

        let first = child(1, 10, 3);
        let second = child(2, 20, 4);
        commit(&a, &first);
        commit(&a, &second);
        commit(&b, &second);
        commit(&b, &first);

        assert_eq!(
            a.catalog_digest().expect("digest"),
            b.catalog_digest().expect("digest"),
            "one judged content, two commit orders, one digest"
        );

        a.write(|tx| tx.delete_dyn(RelationId(1), [&first]).map(|_| ()))
            .expect("delete")
            .unwrap();
        assert_ne!(
            a.catalog_digest().expect("digest"),
            b.catalog_digest().expect("digest"),
            "the quotient erases allocation history, never judged content"
        );
    }
}
