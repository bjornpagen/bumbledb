/// Best-effort read prefetch into L1 (`prfm pldl1keep`); a no-op off
/// aarch64. Purely a scheduling hint — no architectural effect, no
/// safety obligations on the pointer beyond being a valid address to
/// hint about (a stale hint is harmless).
#[inline]
// `unsafe` exists only in the aarch64 body; the portable body is safe, so an
// expectation would be unfulfilled when this same item is built off aarch64.
#[allow(unsafe_code)]
pub fn prefetch_read<T>(ptr: *const T) {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: prfm is a hint; it cannot fault and has no memory effects.
    unsafe {
        core::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) ptr, options(readonly, nostack));
    }
    #[cfg(not(target_arch = "aarch64"))]
    let _ = ptr;
}
