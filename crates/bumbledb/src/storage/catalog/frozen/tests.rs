use super::FrozenMap;
use crate::storage::catalog::{
    Bounds, CatalogMap, CatalogRead, Entry, OrderedRead, ReadCursor, SortedGets,
};

fn packed(pairs: &[(&[u8], &[u8])]) -> FrozenMap {
    FrozenMap::pack(pairs.iter().copied())
}

#[test]
fn get_lower_greater_and_len() {
    let map = packed(&[(b"a", b"1"), (b"c", b"3"), (b"e", b"5")]);
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(b"c"), Some(&b"3"[..]));
    assert_eq!(map.get(b"b"), None);
    let lower = map.lower(b"c").unwrap();
    assert_eq!(lower.key, b"a");
    assert_eq!(lower.value, b"1");
    assert_eq!(map.greater(b"c").unwrap().key, b"e");
    assert_eq!(map.greater_or_equal(b"c").unwrap().key, b"c");
    assert_eq!(map.greater_or_equal(b"b").unwrap().key, b"c");
    assert!(map.lower(b"a").is_none());
    assert!(map.greater(b"e").is_none());
}

#[test]
fn range_walks_in_key_order() {
    let map = packed(&[(b"a", b"1"), (b"b", b"2"), (b"c", b"3")]);
    let mut range = map.range_bounds(&Bounds::all());
    let mut keys = Vec::new();
    while let Some(Entry { key, .. }) = ReadCursor::next(&mut range).expect("next") {
        keys.push(key.to_vec());
    }
    assert_eq!(keys, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
}

#[test]
fn sorted_gets_is_reusable_after_reset() {
    let map = packed(&[(b"a", b"1"), (b"c", b"3"), (b"e", b"5")]);
    let mut gets = super::FrozenGets::new(&map);
    assert_eq!(gets.get(b"a").expect("a"), Some(&b"1"[..]));
    assert_eq!(gets.get(b"c").expect("c"), Some(&b"3"[..]));
    assert_eq!(gets.get(b"d").expect("d"), None);
    assert_eq!(gets.get(b"e").expect("e"), Some(&b"5"[..]));
    gets.reset();
    assert_eq!(gets.get(b"a").expect("reset a"), Some(&b"1"[..]));
}

#[test]
fn empty_map_neighbors_are_none() {
    let map = packed(&[]);
    assert!(map.is_empty());
    assert_eq!(map.get(b"x"), None);
    assert!(map.lower(b"x").is_none());
    assert!(map.greater(b"x").is_none());
}

#[test]
fn catalog_read_dispatches_data_and_dict() {
    use crate::encoding::InternId;
    use crate::storage::dict;

    let data = packed(&[(b"k", b"v")]);
    let raw = b"hello";
    let forward = dict::forward_key(raw);
    let reverse = dict::reverse_key(InternId::from_raw(0));
    let dict = packed(&[
        (forward.as_slice(), 0u64.to_be_bytes().as_slice()),
        (reverse.as_slice(), raw.as_slice()),
    ]);
    let catalog = super::FrozenCatalog {
        data,
        dict,
        dict_next: InternId::from_raw(1),
    };
    assert_eq!(
        OrderedRead::get(&catalog, CatalogMap::Data, b"k").expect("get"),
        Some(&b"v"[..])
    );
    assert_eq!(
        catalog.dict_lookup(raw).expect("lookup"),
        Some(InternId::from_raw(0))
    );
    assert_eq!(
        catalog.dict_resolve(InternId::from_raw(0)).expect("rev"),
        raw
    );
    assert_eq!(catalog.dict_next_id().expect("next"), InternId::from_raw(1));
}
