//! Survivor compaction: keep `items[i]` where `mask[i]` is nonzero. x86 with
//! AVX2 compresses eight lanes per step; elsewhere a scalar cursor write runs
//! at one item per cycle, and NEON has no compress instruction to beat it.
use fearless_simd::Level;

/// Compacts `items` in place, keeping `items[i]` where `mask[i] == 1`.
/// Producers write keep bytes as `u8::from(bool)` or an Allen keep bit, so
/// every byte is 0 or 1.
///
/// # Panics
/// If `mask` is shorter than `items`.
pub fn compact_u32_by_mask(items: &mut Vec<u32>, mask: &[u8]) {
    compact(super::level(), items, mask);
}

pub(super) fn compact(level: Level, items: &mut Vec<u32>, mask: &[u8]) {
    assert!(mask.len() >= items.len(), "one keep byte per item");
    debug_assert!(
        mask[..items.len()].iter().all(|&keep| keep <= 1),
        "keep bytes are 0 or 1"
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if let Some(avx2) = level.as_avx2() {
        use fearless_simd::Simd;
        avx2.vectorize(
            #[inline(always)]
            || compress(avx2, items, mask),
        );
        return;
    }
    let _ = level;
    cursor_write(items, mask);
}

/// Bit `i` is the low bit of `keep[i]`.
#[cfg(any(test, target_arch = "x86", target_arch = "x86_64"))]
fn keep_bits(keep: [u8; 8]) -> u64 {
    // Byte i's low bit lands at bit 56 + i; no other partial product reaches
    // bits 56..64.
    (u64::from_le_bytes(keep) & 0x0101_0101_0101_0101).wrapping_mul(0x0102_0408_1020_4080) >> 56
}

/// Loads eight items, compresses the kept ones to the front, stores all eight
/// lanes at the cursor (`write + 8 <= read + 8 <= n`, so the store never
/// passes unread items) and advances the cursor by the kept count.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
fn compress<S: fearless_simd::Simd>(simd: S, items: &mut Vec<u32>, mask: &[u8]) {
    use fearless_simd::{SimdBase, SimdMask, mask32x8, u32x8};
    let n = items.len();
    let (keep_chunks, keep_tail) = mask[..n].as_chunks::<8>();
    let mut write = 0usize;
    for (chunk, keep) in keep_chunks.iter().enumerate() {
        let read = chunk * 8;
        let bits = keep_bits(*keep);
        let lanes = u32x8::from_slice(simd, &items[read..read + 8]);
        let kept = lanes.compress(mask32x8::from_bitmask(simd, bits));
        kept.store_slice(&mut items[write..write + 8]);
        write += bits.count_ones() as usize;
    }
    for (read, &keep) in (n - keep_tail.len()..n).zip(keep_tail) {
        items[write] = items[read];
        write += usize::from(keep & 1);
    }
    items.truncate(write);
}

/// Writes every item at the cursor and advances it past kept items: no
/// branch on the keep byte.
#[expect(
    unsafe_code,
    reason = "unchecked cursor stores proven in bounds by write <= read < n"
)]
fn cursor_write(items: &mut Vec<u32>, mask: &[u8]) {
    let mask = &mask[..items.len()];
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

#[cfg(test)]
mod tests {
    #[test]
    fn keep_bits_packs_every_byte_pattern() {
        for bits in 0..=255u64 {
            let keep = std::array::from_fn(|i| u8::from((bits >> i) & 1 != 0));
            assert_eq!(super::keep_bits(keep), bits);
        }
    }
}
