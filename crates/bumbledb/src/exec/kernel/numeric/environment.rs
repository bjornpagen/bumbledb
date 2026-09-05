//! The only floating-control-register island. The guard is thread-bound and
//! restores the complete relevant control/status registers even on unwind.
//! Engine code must not call host callbacks or suspend while a guard is live.
//! A signal handler/foreign native code modifying these registers during an
//! operation is outside the supported embedding contract.

#![expect(
    unsafe_code,
    reason = "audited control-register and single-operation assembly boundary"
)]

use super::UnsupportedNumericalPlatform;
use bumbledb_theory::F64;

pub(super) struct NumericalGuard {
    saved: Environment,
    // A guard may neither migrate to another thread nor be used concurrently.
    _thread: core::marker::PhantomData<std::rc::Rc<()>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Environment {
    #[cfg(target_arch = "aarch64")]
    pub(super) control: u64,
    #[cfg(target_arch = "aarch64")]
    pub(super) status: u64,
    #[cfg(target_arch = "x86_64")]
    pub(super) mxcsr: u32,
}

impl NumericalGuard {
    #[cfg_attr(
        any(target_arch = "aarch64", target_arch = "x86_64"),
        expect(
            clippy::unnecessary_wraps,
            reason = "unsupported targets return a typed refusal with the identical API"
        )
    )]
    pub(super) fn enter() -> Result<Self, UnsupportedNumericalPlatform> {
        #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
        {
            // SAFETY: only this thread's architected numerical registers are
            // read/written. The RAII owner restores the saved state on exit.
            let saved = unsafe { Environment::read() };
            unsafe {
                Environment::canonical().install();
            }
            Ok(Self {
                saved,
                _thread: core::marker::PhantomData,
            })
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        Err(UnsupportedNumericalPlatform)
    }
}

impl Drop for NumericalGuard {
    fn drop(&mut self) {
        // SAFETY: !Send/!Sync keeps this guard on the originating thread, and
        // saved is an actual prior register image, not caller-provided bits.
        unsafe {
            self.saved.install();
        }
    }
}

impl Environment {
    /// All traps masked, nearest-even, gradual underflow, zero status flags.
    pub(super) const fn canonical() -> Self {
        Self {
            #[cfg(target_arch = "aarch64")]
            control: 0,
            #[cfg(target_arch = "aarch64")]
            status: 0,
            #[cfg(target_arch = "x86_64")]
            mxcsr: 0x1f80,
        }
    }

    pub(super) unsafe fn read() -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            let control: u64;
            let status: u64;
            // SAFETY: FPCR/FPSR are unprivileged registers. No memory or vector
            // value is accessed; default asm barriers preserve operation order.
            unsafe {
                core::arch::asm!("mrs {control}, fpcr", "mrs {status}, fpsr",
                    control = out(reg) control, status = out(reg) status, options(nostack));
            }
            Self { control, status }
        }
        #[cfg(target_arch = "x86_64")]
        {
            let mut mxcsr = 0_u32;
            // SAFETY: stmxcsr stores exactly four bytes into this valid local.
            unsafe {
                core::arch::asm!("stmxcsr [{ptr}]", ptr = in(reg) &mut mxcsr, options(nostack));
            }
            Self { mxcsr }
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        Self {}
    }

    pub(super) unsafe fn install(self) {
        #[cfg(target_arch = "aarch64")]
        // SAFETY: caller supplies an architected/saved FPCR/FPSR image. FPCR's
        // ARM rounding/flush/trap layout is not inferred from x86 MXCSR.
        unsafe {
            core::arch::asm!("msr fpcr, {control}", "msr fpsr, {status}",
                control = in(reg) self.control, status = in(reg) self.status, options(nostack));
        }
        #[cfg(target_arch = "x86_64")]
        // SAFETY: caller supplies a saved or known-valid MXCSR image; reserved
        // bits remain clear. SSE2 is baseline on x86_64. We execute no x87 code,
        // so x87 state is unrelated and remains untouched.
        unsafe {
            core::arch::asm!("ldmxcsr [{ptr}]", ptr = in(reg) &self.mxcsr, options(nostack));
        }
    }
}

// Fixed instruction spellings make the expression-node boundary immune to
// LLVM constant folding, reassociation and implicit multiply-add contraction.
// No native f64 arithmetic occurs outside this guarded instruction island.
macro_rules! binary {
    ($name:ident, $arm:literal, $x86:literal) => {
        impl NumericalGuard {
            pub(super) fn $name(&self, left: F64, right: F64) -> F64 {
                #[cfg(target_arch = "aarch64")]
                {
                    let bits: u64;
                    // SAFETY: the live guard establishes IEEE controls. The
                    // declared scalar/vector clobbers cover every register.
                    unsafe {
                        core::arch::asm!("fmov d0, {left}", "fmov d1, {right}",
                            $arm, "fmov {result}, d0",
                            left = in(reg) left.to_bits(), right = in(reg) right.to_bits(),
                            result = out(reg) bits, out("v0") _, out("v1") _, options(nostack));
                    }
                    F64::from_bits(bits)
                }
                #[cfg(target_arch = "x86_64")]
                {
                    let bits: u64;
                    // SAFETY: SSE2 is baseline; guard/clobber contract as above.
                    unsafe {
                        core::arch::asm!("movq xmm0, {left}", "movq xmm1, {right}",
                            $x86, "movq {result}, xmm0",
                            left = in(reg) left.to_bits(), right = in(reg) right.to_bits(),
                            result = out(reg) bits, out("xmm0") _, out("xmm1") _, options(nostack));
                    }
                    F64::from_bits(bits)
                }
                #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
                {
                    let _ = (left, right);
                    // No NumericalGuard can be constructed on this target.
                    unreachable!("unsupported targets refuse NumericalGuard::enter")
                }
            }
        }
    };
}

binary!(add, "fadd d0, d0, d1", "addsd xmm0, xmm1");
binary!(subtract, "fsub d0, d0, d1", "subsd xmm0, xmm1");
binary!(multiply, "fmul d0, d0, d1", "mulsd xmm0, xmm1");
binary!(divide, "fdiv d0, d0, d1", "divsd xmm0, xmm1");
