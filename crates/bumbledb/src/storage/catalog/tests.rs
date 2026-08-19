use super::{
    Bounds, CatalogMap, CatalogRead, CatalogWrite, FactCursor, OrderedRead, OrderedWrite,
    PutOutcome, ReadCursor, SortedGets, WriteCursor,
};
use crate::encoding::InternId;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::dict;
use crate::storage::env::Environment;
use crate::storage::keys;
use crate::testutil::TempDir;
use bumbledb_theory::schema::{FieldId, RelationId, SchemaDescriptor};

fn empty_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn env(dir: &TempDir) -> Environment {
    Environment::create(dir.path(), &empty_schema()).expect("create")
}

#[test]
fn ordered_put_get_len_and_neighbors() {
    let dir = TempDir::new("catalog-ordered-neighbors");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    {
        let mut catalog = wtxn.catalog();
        catalog.put(CatalogMap::Data, b"a", b"1").expect("put a");
        catalog.put(CatalogMap::Data, b"c", b"3").expect("put c");
        catalog.put(CatalogMap::Data, b"e", b"5").expect("put e");
        assert_eq!(catalog.len(CatalogMap::Data).expect("len"), 3);
        assert_eq!(
            catalog.get(CatalogMap::Data, b"c").expect("get"),
            Some(&b"3"[..])
        );
        let lower = catalog
            .lower(CatalogMap::Data, b"c")
            .expect("lower")
            .unwrap();
        assert_eq!(lower.key, b"a");
        assert_eq!(lower.value, b"1");
        let greater = catalog.greater(CatalogMap::Data, b"c").expect("greater");
        assert_eq!(greater.unwrap().key, b"e");
        let ge = catalog
            .greater_or_equal(CatalogMap::Data, b"c")
            .expect("ge");
        assert_eq!(ge.unwrap().key, b"c");
    }
    wtxn.commit().expect("commit");
}

#[test]
fn range_walks_in_key_order() {
    let dir = TempDir::new("catalog-range");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    {
        let mut catalog = wtxn.catalog();
        catalog.put(CatalogMap::Data, b"b", b"2").expect("put");
        catalog.put(CatalogMap::Data, b"a", b"1").expect("put");
        catalog.put(CatalogMap::Data, b"c", b"3").expect("put");
        let mut range = catalog
            .range(CatalogMap::Data, Bounds::all())
            .expect("range");
        let mut keys = Vec::new();
        while let Some(entry) = ReadCursor::next(&mut range).expect("next") {
            keys.push(entry.key.to_vec());
        }
        assert_eq!(keys, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }
    wtxn.commit().expect("commit");
}

#[test]
fn sorted_gets_is_reusable_after_reset() {
    let dir = TempDir::new("catalog-sorted-gets");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    {
        let mut catalog = wtxn.catalog();
        catalog.put(CatalogMap::Data, b"a", b"1").expect("put");
        catalog.put(CatalogMap::Data, b"c", b"3").expect("put");
        catalog.put(CatalogMap::Data, b"e", b"5").expect("put");
        let mut gets = catalog.sorted_gets(CatalogMap::Data).expect("sorted_gets");
        assert_eq!(gets.get(b"a").expect("a"), Some(&b"1"[..]));
        assert_eq!(gets.get(b"c").expect("c"), Some(&b"3"[..]));
        assert_eq!(gets.get(b"d").expect("d"), None);
        assert_eq!(gets.get(b"e").expect("e"), Some(&b"5"[..]));
        gets.reset();
        assert_eq!(gets.get(b"a").expect("reset a"), Some(&b"1"[..]));
    }
    wtxn.commit().expect("commit");
}

#[test]
fn put_no_overwrite_is_a_named_sum() {
    let dir = TempDir::new("catalog-no-overwrite");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    {
        let mut catalog = wtxn.catalog();
        assert_eq!(
            catalog
                .put_no_overwrite(CatalogMap::Data, b"k", b"v")
                .expect("insert"),
            PutOutcome::Inserted
        );
        assert_eq!(
            catalog
                .put_no_overwrite(CatalogMap::Data, b"k", b"other")
                .expect("occupied"),
            PutOutcome::Occupied
        );
        assert_eq!(
            catalog.get(CatalogMap::Data, b"k").expect("get"),
            Some(&b"v"[..])
        );
        assert!(catalog.delete(CatalogMap::Data, b"k").expect("delete"));
        assert!(!catalog.delete(CatalogMap::Data, b"k").expect("missing"));
    }
    wtxn.commit().expect("commit");
}

#[test]
fn del_current_removes_the_yielded_entry() {
    let dir = TempDir::new("catalog-del-current");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    {
        let mut catalog = wtxn.catalog();
        catalog.put(CatalogMap::Data, b"a", b"1").expect("put");
        catalog.put(CatalogMap::Data, b"b", b"2").expect("put");
        {
            let mut writes = catalog
                .range_mut(CatalogMap::Data, Bounds::all())
                .expect("range_mut");
            assert_eq!(
                ReadCursor::next(&mut writes).expect("next").unwrap().key,
                b"a"
            );
            WriteCursor::del_current(&mut writes).expect("del");
            assert_eq!(
                ReadCursor::next(&mut writes).expect("next").unwrap().key,
                b"b"
            );
        }
        assert_eq!(catalog.get(CatalogMap::Data, b"a").expect("gone"), None);
        assert_eq!(
            catalog.get(CatalogMap::Data, b"b").expect("kept"),
            Some(&b"2"[..])
        );
    }
    wtxn.commit().expect("commit");
}

#[test]
fn dict_ids_are_intern_ids() {
    let dir = TempDir::new("catalog-dict-intern-id");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    dict::put_pending(&mut wtxn, b"hello", InternId::from_raw(0)).expect("pending");
    wtxn.catalog()
        .set_dict_next_id(InternId::from_raw(1))
        .expect("set next");
    wtxn.commit().expect("commit");

    let rtxn = env.read_txn().expect("read");
    let catalog = rtxn.catalog();
    assert_eq!(
        catalog.dict_lookup(b"hello").expect("lookup"),
        Some(InternId::from_raw(0))
    );
    assert_eq!(
        catalog
            .dict_resolve(InternId::from_raw(0))
            .expect("resolve"),
        b"hello"
    );
    assert_eq!(catalog.dict_next_id().expect("next"), InternId::from_raw(1));
    assert!(
        catalog
            .fetch_fact(RelationId(0), 0)
            .expect("fetch")
            .is_none()
    );
    let mut facts = catalog.scan_facts(RelationId(0)).expect("scan");
    assert!(FactCursor::next(&mut facts).expect("next").is_none());
}

#[test]
fn catalog_counters_membership_and_facts() {
    let dir = TempDir::new("catalog-counters");
    let env = env(&dir);
    let mut wtxn = env.write_txn().expect("txn");
    let rel = RelationId(0);
    let field = FieldId(0);
    {
        let mut catalog = wtxn.catalog();
        catalog.set_row_count(rel, 4).expect("row count");
        catalog.set_row_id_high_water(rel, 9).expect("high water");
        catalog.set_fresh_next(rel, field, 11).expect("fresh");
        assert_eq!(catalog.row_count(rel).expect("read count"), 4);
        assert_eq!(catalog.row_id_high_water(rel).expect("read hw"), 9);
        assert_eq!(catalog.fresh_next(rel, field).expect("read q"), 11);

        let hash = [7u8; 32];
        catalog
            .put(
                CatalogMap::Data,
                &keys::membership_key(rel, &hash),
                &3u64.to_le_bytes(),
            )
            .expect("M");
        assert_eq!(catalog.membership_row(rel, &hash).expect("M get"), Some(3));

        let mut buf = [0u8; keys::MAX_KEY];
        let u_key = keys::determinant_key(
            &mut buf,
            rel,
            bumbledb_theory::schema::StatementId(0),
            &[1, 2, 3, 4, 5, 6, 7, 8],
        )
        .to_vec();
        catalog
            .put(CatalogMap::Data, &keys::fact_key(rel, 1), b"row-bytes")
            .expect("F");
        catalog
            .put(CatalogMap::Data, &u_key, &1u64.to_le_bytes())
            .expect("U");
        assert_eq!(catalog.determinant_row(&u_key).expect("U"), Some(1));
        assert_eq!(
            catalog.fetch_fact(rel, 1).expect("F get"),
            Some(&b"row-bytes"[..])
        );
        let mut scan = catalog.scan_facts(rel).expect("scan");
        let entry = FactCursor::next(&mut scan).expect("next").expect("row");
        assert_eq!(entry.row, 1);
        assert_eq!(entry.bytes, b"row-bytes");
    }
    wtxn.commit().expect("commit");
}
