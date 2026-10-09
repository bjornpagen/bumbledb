//! The float environment check. Rust and LLVM compile `f64` arithmetic for
//! the IEEE default environment: round to nearest even, subnormals kept, all
//! traps masked. A host that changed the control register would silently
//! change results, so float arithmetic first proves the register is default.
//! The check only reads the register; nothing here ever writes it.

use bumbledb_theory::F64;

/// The thread's float control register is not the IEEE default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonDefaultFloatEnvironment {
    control: u64,
}

impl NonDefaultFloatEnvironment {
    /// The control register image that failed the check (FPCR on aarch64,
    /// MXCSR on x86-64).
    #[must_use]
    pub const fn control(self) -> u64 {
        self.control
    }
}

impl core::fmt::Display for NonDefaultFloatEnvironment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "the float environment is not the IEEE default (control register {:#x})",
            self.control
        )
    }
}

impl std::error::Error for NonDefaultFloatEnvironment {}

/// Proof that the executing thread's float environment was the IEEE default
/// when checked. Holders check again for each execution, since the register
/// is per thread.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DefaultFloatEnvironment(());

impl DefaultFloatEnvironment {
    /// Reads the control register once.
    pub(crate) fn check() -> Result<Self, NonDefaultFloatEnvironment> {
        Self::from_control(read_control())
    }

    fn from_control(control: u64) -> Result<Self, NonDefaultFloatEnvironment> {
        if control & CONTROL_MASK == CONTROL_DEFAULT {
            Ok(Self(()))
        } else {
            Err(NonDefaultFloatEnvironment { control })
        }
    }
}

#[expect(
    clippy::unused_self,
    reason = "the receiver is the proof that the environment was checked"
)]
impl DefaultFloatEnvironment {
    pub(crate) fn add(self, left: F64, right: F64) -> F64 {
        F64::from(left.to_f64() + right.to_f64())
    }

    pub(crate) fn subtract(self, left: F64, right: F64) -> F64 {
        F64::from(left.to_f64() - right.to_f64())
    }

    pub(crate) fn multiply(self, left: F64, right: F64) -> F64 {
        F64::from(left.to_f64() * right.to_f64())
    }

    pub(crate) fn divide(self, left: F64, right: F64) -> F64 {
        F64::from(left.to_f64() / right.to_f64())
    }
}

/// FPCR bits that change `f64` results or trap: FIZ and AH (0, 1), the
/// trap enables (8..=12, 15), the rounding mode (22, 23) and FZ (24). DN
/// (25) only changes NaN payloads, which canonicalization erases.
#[cfg(all(target_arch = "aarch64", not(miri)))]
const CONTROL_MASK: u64 = 0b11 | 0x9f00 | (0b111 << 22);
#[cfg(all(target_arch = "aarch64", not(miri)))]
const CONTROL_DEFAULT: u64 = 0;

/// MXCSR bits that change `f64` results or trap: DAZ (6), the exception
/// masks (7..=12), the rounding control (13, 14) and FTZ (15). The sticky
/// status flags (0..=5) are ignored.
#[cfg(all(target_arch = "x86_64", not(miri)))]
const CONTROL_MASK: u64 = 0xffc0;
#[cfg(all(target_arch = "x86_64", not(miri)))]
const CONTROL_DEFAULT: u64 = 0x1f80;

/// Other targets and Miri have no register to read; Rust's own default
/// environment assumption stands.
#[cfg(any(miri, not(any(target_arch = "aarch64", target_arch = "x86_64"))))]
const CONTROL_MASK: u64 = 0;
#[cfg(any(miri, not(any(target_arch = "aarch64", target_arch = "x86_64"))))]
const CONTROL_DEFAULT: u64 = 0;

#[cfg(all(target_arch = "aarch64", not(miri)))]
#[expect(unsafe_code, reason = "one read-only FPCR read")]
fn read_control() -> u64 {
    let control: u64;
    // SAFETY: FPCR is readable at EL0; `mrs` writes only the output register
    // and touches no memory, flags or float state.
    unsafe {
        core::arch::asm!("mrs {control}, fpcr", control = out(reg) control,
            options(nomem, nostack, preserves_flags));
    }
    control
}

#[cfg(all(target_arch = "x86_64", not(miri)))]
#[expect(unsafe_code, reason = "one read-only MXCSR read")]
fn read_control() -> u64 {
    let mut mxcsr = 0_u32;
    // SAFETY: SSE is baseline on x86-64; `stmxcsr` stores exactly four bytes
    // into this local and changes no register state.
    unsafe {
        core::arch::asm!("stmxcsr [{ptr}]", ptr = in(reg) &raw mut mxcsr,
            options(nostack, preserves_flags));
    }
    u64::from(mxcsr)
}

#[cfg(any(miri, not(any(target_arch = "aarch64", target_arch = "x86_64"))))]
fn read_control() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_test_thread_runs_the_default_environment() {
        assert!(DefaultFloatEnvironment::check().is_ok());
    }

    #[cfg(all(target_arch = "aarch64", not(miri)))]
    #[test]
    fn fpcr_images_that_change_results_or_trap_are_refused() {
        for bit in [0, 1, 8, 9, 10, 11, 12, 15, 22, 23, 24] {
            let control = 1u64 << bit;
            assert_eq!(
                DefaultFloatEnvironment::from_control(control).map(|_| ()),
                Err(NonDefaultFloatEnvironment { control }),
                "bit {bit}"
            );
        }
        for harmless in [1u64 << 25, 1 << 19, 1 << 2] {
            assert!(DefaultFloatEnvironment::from_control(harmless).is_ok());
        }
    }

    #[cfg(all(target_arch = "x86_64", not(miri)))]
    #[test]
    fn mxcsr_images_that_change_results_or_trap_are_refused() {
        for flip in [6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
            let control = 0x1f80 ^ (1u64 << flip);
            assert!(
                DefaultFloatEnvironment::from_control(control).is_err(),
                "bit {flip}"
            );
        }
        assert!(DefaultFloatEnvironment::from_control(0x1f80 | 0x3f).is_ok());
    }
}
