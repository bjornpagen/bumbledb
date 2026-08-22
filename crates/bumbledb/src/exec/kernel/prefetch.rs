#[inline]
// `unsafe` exists only in the aarch64 body; the portable body is safe, so an
#[allow(unsafe_code)]
pub fn prefetch_read<T>(ptr: *const T) {
    #[cfg(all(target_arch = "aarch64", not(miri)))]
    // SAFETY: prfm is a hint; it cannot fault and has no memory effects.
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) ptr, options(readonly, nostack));
    }
    #[cfg(not(all(target_arch = "aarch64", not(miri))))]
    let _ = ptr;
}
