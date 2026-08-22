/// Compacts `items` in place, keeping `items[i]` where `mask[i] == 1` —
/// the survivor-compaction kernel (scalar cursor-write on every target;
/// see the module docs).
/// Mask bytes are **0/1 by contract**: every producer writes
/// `u8::from(bool)` (the probe/residual/anti-probe masks) or an Allen
/// keep bit — `(mask >> code) & 1` in the scalar and `std::simd`
/// forms, a 0/1 table byte through the NEON `tbl` — and the debug
/// build asserts it. The contract buys the triad diet: `mask[read]
/// The cursor store is unchecked under the module's unsafe law (safe
/// most 1 per iteration — but the invariant is invisible to LLVM, so
/// # Panics
/// Only on a programmer-invariant violation: `mask` shorter than `items`.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    let n = items.len();
    assert!(mask.len() >= n);
    let mask = &mask[..n];
    debug_assert!(
        mask.iter().all(|&keep| keep <= 1),
        "keep bytes are 0/1 by contract"
    );
    let mut write = 0usize;
    // SAFETY: `write <= read < n` at every store — both cursors start
    // at 0 and `write` advances by at most 1 after each store — so

    // carries no drop obligation).
    unsafe {
        let ptr = items.as_mut_ptr();
        for (read, &keep) in mask.iter().enumerate() {
            *ptr.add(write) = *ptr.add(read);
            write += usize::from(keep & 1);
        }
        items.set_len(write);
    }
}
