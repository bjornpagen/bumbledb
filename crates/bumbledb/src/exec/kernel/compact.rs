//! Survivor compaction: keep `items[i]` where `mask[i]` is nonzero.

/// Compacts `items` in place, keeping `items[i]` where `mask[i] == 1`.
/// Producers write keep bytes as `u8::from(bool)` or an Allen keep bit, so
/// every byte is 0 or 1.
///
/// # Panics
/// If `mask` is shorter than `items`.
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    cursor_write(items, mask);
}

/// Writes every item at the cursor and advances it past kept items: no
/// branch on the keep byte.
#[expect(
    unsafe_code,
    reason = "unchecked cursor stores proven in bounds by write <= read < n"
)]
fn cursor_write(items: &mut Vec<u32>, mask: &[u8]) {
    let n = items.len();
    assert!(mask.len() >= n);
    let mask = &mask[..n];
    debug_assert!(mask.iter().all(|&keep| keep <= 1), "keep bytes are 0 or 1");
    let mut write = 0usize;
    // SAFETY: `write` starts at 0 and grows by at most 1 per iteration, so
    // `write <= read < n` at every store and both pointers stay inside the
    // initialized prefix; `set_len(write)` shrinks to a written prefix of a
    // `u32` buffer, which has no drop obligation.
    unsafe {
        let ptr = items.as_mut_ptr();
        for (read, &keep) in mask.iter().enumerate() {
            *ptr.add(write) = *ptr.add(read);
            write += usize::from(keep & 1);
        }
        items.set_len(write);
    }
}
