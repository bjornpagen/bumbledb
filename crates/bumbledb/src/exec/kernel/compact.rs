/// Compacts `items` in place, keeping `items[i]` where `mask[i] != 0` —
/// the survivor-compaction kernel (scalar cursor-write on every target;
/// see the module docs).
///
/// # Panics
///
/// Only on a programmer-invariant violation: `mask` shorter than `items`.
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    assert!(mask.len() >= items.len());
    let mut write = 0usize;
    for read in 0..items.len() {
        items[write] = items[read];
        write += usize::from(mask[read] != 0);
    }
    items.truncate(write);
}
