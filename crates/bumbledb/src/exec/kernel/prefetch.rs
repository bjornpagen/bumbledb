/// Best-effort read prefetch into L1 (`prfm pldl1keep`); a no-op off
/// aarch64 and under Miri (the interpreter has no cache to hint, and
/// `asm!` is an unsupported operation there — the store-free colt
/// fixture walks through this on the Miri lane). Purely a scheduling
/// hint — no architectural effect, no safety obligations on the
/// pointer beyond being a valid address to hint about (a stale hint is
/// harmless).
#[inline]
// `unsafe` exists only in the aarch64 body; the portable body is safe, so an
// expectation would be unfulfilled when this same item is built off aarch64.
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
