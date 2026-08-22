#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
#[inline]
#[must_use]
pub fn ticks() -> u64 {
    let t: u64;
    // SAFETY: cntvct_el0 is user-readable on aarch64 (Apple Silicon

    unsafe {
        core::arch::asm!("mrs {t}, cntvct_el0", t = out(reg) t, options(nomem, nostack));
    }
    t
}

#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
#[inline]
#[must_use]
pub fn ticks_ss() -> u64 {
    if !has_ecv() {
        return ticks();
    }
    let t: u64;
    // SAFETY: the runtime feature check above establishes FEAT_ECV;

    unsafe {
        core::arch::asm!("mrs {t}, s3_3_c14_c0_6", t = out(reg) t, options(nomem, nostack));
    }
    t
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[expect(
    unsafe_code,
    reason = "one read-only sysctl query discovers whether the ECV register is legal"
)]
fn has_ecv() -> bool {
    use core::ffi::{c_char, c_int, c_void};
    use std::sync::OnceLock;

    unsafe extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> c_int;
    }

    static HAS_ECV: OnceLock<bool> = OnceLock::new();
    *HAS_ECV.get_or_init(|| {
        let mut value: c_int = 0;
        let mut len = size_of::<c_int>();
        // SAFETY: the name is NUL-terminated; `value` and `len` point to

        let status = unsafe {
            sysctlbyname(
                c"hw.optional.arm.FEAT_ECV".as_ptr(),
                (&raw mut value).cast(),
                &raw mut len,
                core::ptr::null_mut(),
                0,
            )
        };
        status == 0 && len == size_of::<c_int>() && value != 0
    })
}

#[cfg(all(target_arch = "aarch64", not(target_os = "macos")))]
fn has_ecv() -> bool {
    false
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
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
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
/// # Panics
/// Never in practice: process uptime in nanoseconds overflows `u64`
/// after ~584 years.
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
/// # Panics
/// total exceeding u64 nanoseconds (~584 years).
/// Only on a programmer-invariant violation: an accumulated phase
#[must_use]
pub fn ticks_to_ns(ticks: u64) -> u64 {
    u64::try_from(u128::from(ticks) * 1_000_000_000 / u128::from(frequency()))
        .expect("accumulated phase time fits u64 ns")
}
