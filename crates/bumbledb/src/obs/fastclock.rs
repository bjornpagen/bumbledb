/// An opaque monotonic tick count.
#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] // one mrs read; sanctioned for trace-only builds
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
/// accepts it without a `FEAT_ECV` target attribute. ECV is detected at
/// runtime: the reference M2+ path uses the self-synchronizing counter,
/// while older aarch64 machines use the ordinary counter instead of
/// executing an unsupported system-register read.
#[cfg(target_arch = "aarch64")]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] // one mrs read; sanctioned for trace-only builds
#[inline]
#[must_use]
pub fn ticks_ss() -> u64 {
    if !has_ecv() {
        return ticks();
    }
    let t: u64;
    // SAFETY: the runtime feature check above establishes FEAT_ECV;
    // CNTVCTSS_EL0 is then user-readable wherever cntvct_el0 is, and
    // the read has no memory effects.
    unsafe {
        core::arch::asm!("mrs {t}, s3_3_c14_c0_6", t = out(reg) t, options(nomem, nostack));
    }
    t
}

/// Apple exposes architectural feature bits through `sysctl`; cache the
/// answer because this predicate sits on the trace-event hot path. Other
/// aarch64 hosts conservatively use the ordinary counter: losing the
/// self-synchronizing close stamp is preferable to guessing at ECV and
/// trapping on an unsupported register.
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
        // writable objects of the declared sizes; both new-value
        // arguments are null/zero, making this a read-only query.
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
