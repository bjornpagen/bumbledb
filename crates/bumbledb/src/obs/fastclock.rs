/// An opaque monotonic tick count.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)] // one mrs read; sanctioned for trace-only builds
#[inline]
#[must_use]
pub fn ticks() -> u64 {
    let t: u64;
    // SAFETY: cntvct_el0 is user-readable on aarch64 (Apple Silicon
    // and Linux both expose it); the read has no memory effects.
    unsafe {
        core::arch::asm!("mrs {t}, cntvct_el0", t = out(reg) t, options(nomem, nostack));
    }
    t
}

/// The self-synchronized tick count (`CNTVCTSS_EL0`): cannot read
/// early across in-flight work — the closing stamp for single-shot
/// spans. Spelled by encoding (`s3_3_c14_c0_6`) so the assembler
/// accepts it without a `FEAT_ECV` target attribute; the machine
/// tailoring rulings pin the reference host at M2+, where ECV is
/// architectural.
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)] // one mrs read; sanctioned for trace-only builds
#[inline]
#[must_use]
pub fn ticks_ss() -> u64 {
    let t: u64;
    // SAFETY: CNTVCTSS_EL0 is user-readable wherever cntvct_el0 is
    // (FEAT_ECV); the read has no memory effects.
    unsafe {
        core::arch::asm!("mrs {t}, s3_3_c14_c0_6", t = out(reg) t, options(nomem, nostack));
    }
    t
}

/// Portable fallback: the ordinary tick (no reorder-slide semantics
/// to preserve off aarch64).
#[cfg(not(target_arch = "aarch64"))]
#[must_use]
pub fn ticks_ss() -> u64 {
    ticks()
}

/// Tick frequency in Hz (`cntfrq_el0`).
#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)]
#[must_use]
pub fn frequency() -> u64 {
    // SAFETY: cntfrq_el0 is a user-readable constant register.
    let f: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) f, options(nomem, nostack));
    }
    f
}

/// Portable fallback: nanoseconds from a process anchor.
#[cfg(not(target_arch = "aarch64"))]
#[must_use]
pub fn ticks() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static ANCHOR: OnceLock<Instant> = OnceLock::new();
    u64::try_from(ANCHOR.get_or_init(Instant::now).elapsed().as_nanos())
        .expect("process uptime fits u64 ns")
}

/// Portable fallback frequency: the tick already is a nanosecond.
#[cfg(not(target_arch = "aarch64"))]
#[must_use]
pub fn frequency() -> u64 {
    1_000_000_000
}

/// Converts accumulated ticks to nanoseconds (u128 interim: no
/// overflow below ~584 years of ticks).
///
/// # Panics
///
/// Only on a programmer-invariant violation: an accumulated phase
/// total exceeding u64 nanoseconds (~584 years).
#[must_use]
pub fn ticks_to_ns(ticks: u64) -> u64 {
    u64::try_from(u128::from(ticks) * 1_000_000_000 / u128::from(frequency()))
        .expect("accumulated phase time fits u64 ns")
}
