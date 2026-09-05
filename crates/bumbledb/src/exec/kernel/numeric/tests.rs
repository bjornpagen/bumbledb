#![expect(
    unsafe_code,
    reason = "tests deliberately perturb only their own thread's floating registers"
)]

use super::*;
use environment::Environment;

struct RestoreEnvironment(Environment);
impl Drop for RestoreEnvironment {
    fn drop(&mut self) {
        // SAFETY: saved on the same thread immediately before this test.
        unsafe {
            self.0.install();
        }
    }
}

fn hostile_modes() -> Vec<Environment> {
    let mut result = Vec::new();
    for rounding in 0..4 {
        for flush in 0..4 {
            let mut mode = Environment::canonical();
            #[cfg(target_arch = "aarch64")]
            {
                mode.control = (rounding << 22) | ((flush & 1) << 24) | ((flush >> 1) << 19);
                mode.status = 0x9f;
            }
            #[cfg(target_arch = "x86_64")]
            {
                mode.mxcsr |= (rounding << 13) | ((flush & 1) << 15) | ((flush >> 1) << 6) | 0x3f;
            }
            result.push(mode);
            // A host may unmask arithmetic traps. No hardware float operation
            // is executed until our guard has remasked them.
            #[cfg(target_arch = "aarch64")]
            {
                mode.control |= 0x9f00;
            }
            #[cfg(target_arch = "x86_64")]
            {
                mode.mxcsr &= !0x1f80;
            }
            result.push(mode);
        }
    }
    result
}

#[test]
fn every_host_rounding_and_flush_mode_is_overridden_and_exactly_restored() {
    // SAFETY: test runs synchronously; the RAII owner restores on assertion failure.
    let original = RestoreEnvironment(unsafe { Environment::read() });
    for mode in hostile_modes() {
        unsafe {
            mode.install();
        }
        let actual_mode = unsafe { Environment::read() }; // read back implemented bits
        let subnormal = F64Math::multiply(F64::from_bits(1), F64::from_bits(0x3ff0_0000_0000_0000));
        let tiny = F64Math::divide(F64::from_bits(3), F64::from_bits(0x4000_0000_0000_0000));
        let tie = F64Math::add(
            F64::from_bits(0x3ff0_0000_0000_0000),
            F64::from_bits(0x3ca0_0000_0000_0000),
        );
        let invalid = F64Math::divide(F64::ZERO, F64::ZERO);
        let after = unsafe { Environment::read() };
        unsafe {
            original.0.install();
        }
        assert_eq!(after, actual_mode);
        assert_eq!(subnormal.unwrap().to_bits(), 1);
        assert_eq!(tiny.unwrap().to_bits(), 2);
        assert_eq!(tie.unwrap().to_bits(), 0x3ff0_0000_0000_0000);
        assert_eq!(invalid.unwrap(), F64::NAN);
    }
}

#[test]
fn guard_restores_on_error_unwind_and_nested_execution() {
    let original = RestoreEnvironment(unsafe { Environment::read() });
    for mode in hostile_modes() {
        unsafe {
            mode.install();
        }
        let installed = unsafe { Environment::read() };
        let error: Result<(), &str> = {
            let _guard = NumericalGuard::enter().unwrap();
            Err("injected cancellation/error exit")
        };
        let after_error = unsafe { Environment::read() };
        let unwind = std::panic::catch_unwind(|| {
            let _guard = NumericalGuard::enter().unwrap();
            panic!("injected numerical unwind");
        });
        let after_unwind = unsafe { Environment::read() };
        {
            let guard = NumericalGuard::enter().unwrap();
            let inner = F64Math::divide(F64::from_bits(0x3ff0_0000_0000_0000), F64::ZERO).unwrap();
            assert_eq!(inner, F64::INFINITY);
            assert_eq!(guard.add(F64::ZERO, F64::ZERO), F64::ZERO);
        }
        let after_nested = unsafe { Environment::read() };
        unsafe {
            original.0.install();
        }
        assert!(error.is_err());
        assert!(unwind.is_err());
        assert_eq!(after_error, installed);
        assert_eq!(after_unwind, installed);
        assert_eq!(after_nested, installed);
    }
}

#[test]
fn zero_canonicalization_and_forbidden_rewrites_have_distinguishing_examples() {
    let one = F64::from_bits(0x3ff0_0000_0000_0000);
    assert_eq!(
        F64Math::divide(one, F64::ZERO.negated()).unwrap(),
        F64::INFINITY
    );
    assert_eq!(
        F64Math::subtract(F64::INFINITY, F64::INFINITY).unwrap(),
        F64::NAN
    );
    assert_eq!(F64Math::divide(F64::ZERO, F64::ZERO).unwrap(), F64::NAN);
    assert_eq!(
        F64Math::multiply(F64::INFINITY, F64::ZERO).unwrap(),
        F64::NAN
    );
    let large = F64::from_bits(0x4341_c379_37e0_8000);
    let sequential = F64Math::add(F64Math::add(large, one).unwrap(), large.negated()).unwrap();
    let reassociated = F64Math::add(large, F64Math::add(one, large.negated()).unwrap()).unwrap();
    assert_eq!(sequential, F64::ZERO);
    assert_eq!(reassociated, F64::ZERO);
    assert_eq!(
        F64Math::sum([large, one, large.negated()]).unwrap(),
        Some(one)
    );
    // (large + -large) + one differs from large + (-large + one).
    assert_ne!(
        F64Math::add(F64Math::add(large, large.negated()).unwrap(), one).unwrap(),
        reassociated
    );
    // (1+2^-52)*(1-2^-52)-1: product rounds to 1; fused result is -2^-104.
    let product = F64Math::multiply(
        F64::from_bits(0x3ff0_0000_0000_0001),
        F64::from_bits(0x3fef_ffff_ffff_fffe),
    )
    .unwrap();
    assert_eq!(F64Math::subtract(product, one).unwrap(), F64::ZERO);
    assert_ne!(F64::ZERO, F64::from_bits(0xb970_0000_0000_0000));
}
