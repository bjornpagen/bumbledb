//! Common-field discriminator, compiled unchanged against Q1 and Q2.
use super::WordMap;

#[test]
#[ignore = "explicit representation regression; fails on the Q1 baseline"]
fn packed_payload_lengths_discriminator() {
    for arity in [0, 1, 2, 8, 9] {
        let mut map = WordMap::<u64>::new(arity);
        for i in 0..129_u64 {
            map.get_or_insert_with(&vec![i; arity], || i + 10);
        }
        assert_eq!(map.keys.len(), map.len() * arity, "keys must be packed");
        assert_eq!(map.values.len(), map.len(), "values must be packed");
        let pointers = (map.keys.as_ptr(), map.values.as_ptr());
        let capacity = (map.keys.capacity(), map.values.capacity());
        map.grow();
        assert_eq!((map.keys.as_ptr(), map.values.as_ptr()), pointers);
        assert_eq!((map.keys.capacity(), map.values.capacity()), capacity);
        map.clear();
        assert!(map.keys.is_empty() && map.values.is_empty());
    }
}
