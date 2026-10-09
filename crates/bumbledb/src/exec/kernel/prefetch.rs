/// Hints the CPU to pull `ptr`'s cache line into L1 ahead of a read. A no-op
/// off aarch64 and under Miri.
#[inline]
#[allow(
    unsafe_code,
    reason = "only the aarch64 body is unsafe, so an expect would go unfulfilled elsewhere"
)]
pub fn prefetch_read<T>(ptr: *const T) {
    #[cfg(all(target_arch = "aarch64", not(miri)))]
    // SAFETY: prfm is a hint; it cannot fault and has no memory effects.
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) ptr, options(readonly, nostack));
    }
    #[cfg(not(all(target_arch = "aarch64", not(miri))))]
    let _ = ptr;
}
