/// Compacts `items` in place, keeping `items[i]` where `mask[i] == 1` —
/// the survivor-compaction kernel (scalar cursor-write on every target;
/// see the module docs).
///
/// Mask bytes are **0/1 by contract**: every producer writes
/// `u8::from(bool)` (the probe/residual/anti-probe masks) or an Allen
/// keep bit — `(mask >> code) & 1` in the scalar and `std::simd`
/// forms, a 0/1 table byte through the NEON `tbl` — and the debug
/// build asserts it. The contract buys the triad diet: `mask[read]
/// != 0` compiles to `cmp`+`cinc`, two µops confined to the 3-port
/// flag triad (`m2max.core.flag-port-asymmetry`), where `& 1` is
/// `and`+`add` on any of the 6 ALUs.
///
/// The cursor store is unchecked under the module's unsafe law (safe
/// reference twin + bit-identity property test): `write <= read` holds
/// by induction — both cursors start at 0 and `write` advances by at
/// most 1 per iteration — but the invariant is invisible to LLVM, so
/// the safe form carries an unelidable `items[write]` bounds check
/// (`cmp`+`b.hs`, the same triad) in the hottest per-survivor loop the
/// executor owns.
///
/// # Panics
///
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
    // every read and every write lands inside the vector's initialized
    // prefix, and `set_len(write)` only shrinks (`write <= n`; `u32`
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
