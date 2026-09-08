//! Monotonic transaction-admission stamps for age diagnostics.
//! They observe elapsed time; they never expire an operation.
use std::time::Duration;
#[cfg(any(not(target_os = "macos"), test))]
use std::time::Instant;

/// An actual transaction admission sample, not an operation's earlier
/// queue-admission deadline origin. Retain raw Apple ticks; age diagnostics
/// alone pay conversion. The two APIs share one native clock and timebase.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AdmissionStamp(u64);

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AdmissionStamp(Instant);

impl AdmissionStamp {
    pub(crate) fn now() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self(native_now())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self(Instant::now())
        }
    }

    pub(crate) fn elapsed(self) -> Duration {
        #[cfg(target_os = "macos")]
        {
            let now = native_now();
            timebase().elapsed(self.0, now)
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.0.elapsed()
        }
    }

    /// Synthetic-age fixture only: return a stamp at least `duration`
    /// earlier, rounding backwards to a whole tick. This is deliberately
    /// not exposed as a new production timestamp-arithmetic surface.
    #[cfg(test)]
    pub(crate) fn earlier_by(self, duration: Duration) -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            let scale = timebase();
            // Duration is <=94 bits and denom <=32: product fits u128.
            let ticks =
                (duration.as_nanos() * u128::from(scale.denom)).div_ceil(u128::from(scale.numer));
            Some(Self(self.0.checked_sub(u64::try_from(ticks).ok()?)?))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Some(Self(self.0.checked_sub(duration)?))
        }
    }
}

#[cfg(any(target_os = "macos", test))]
struct TickScale {
    numer: u32,
    denom: u32,
}

#[cfg(any(target_os = "macos", test))]
impl TickScale {
    fn nanos(&self, ticks: u64) -> u128 {
        // 64-bit ticks times the documented 32-bit numerator fit u128.
        u128::from(ticks) * u128::from(self.numer) / u128::from(self.denom)
    }

    fn elapsed(&self, started: u64, now: u64) -> Duration {
        // Instant observes floor-converted ABSOLUTE samples. Converting
        // their tick difference instead can lose a nanosecond. A reversed
        // sample saturates to zero, as Instant::elapsed does.
        let nanos = self
            .nanos(now)
            .saturating_sub(self.nanos(started))
            .min(Duration::MAX.as_nanos());
        // Real Instant-compatible samples fit Duration. The clamp keeps
        // extreme synthetic timebases total too, without overflow/panic.
        Duration::new(
            u64::try_from(nanos / 1_000_000_000).expect("clamped seconds fit"),
            u32::try_from(nanos % 1_000_000_000).expect("subsecond remainder fits"),
        )
    }
}

#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "macOS's pointer-free monotonic clock ABI; no Rust memory or aliasing preconditions"
)]
fn native_now() -> u64 {
    unsafe extern "C" {
        fn mach_absolute_time() -> u64;
    }
    // SAFETY: this documented, process-wide clock has no arguments and
    // may be read from any thread. Do not substitute approximate/continuous
    // clocks: the former changes precision, the latter includes sleep.
    unsafe { mach_absolute_time() }
}

#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "the documented C timebase ABI writes one initialized repr(C) output record"
)]
fn timebase() -> &'static TickScale {
    static SCALE: std::sync::OnceLock<TickScale> = std::sync::OnceLock::new();
    SCALE.get_or_init(|| {
        #[repr(C)]
        struct TimebaseInfo {
            numer: u32,
            denom: u32,
        }
        unsafe extern "C" {
            fn mach_timebase_info(info: *mut TimebaseInfo) -> i32;
        }
        let mut info = TimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: exclusive writable storage of the documented C size and
        // alignment lives through the call; the function retains no pointer.
        let result = unsafe { mach_timebase_info(&raw mut info) };
        assert_eq!(result, 0, "mach timebase unavailable");
        assert!(info.numer != 0 && info.denom != 0, "invalid mach timebase");
        TickScale {
            numer: info.numer,
            denom: info.denom,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_age_matches_difference_of_rounded_absolute_samples() {
        for (numer, denom) in [
            (1, 1),
            (125, 3),
            (3, 125),
            (1, u32::MAX),
            (u32::MAX, 1),
            (u32::MAX, u32::MAX - 1),
        ] {
            let scale = TickScale { numer, denom };
            for start in [0, 1, 2, 41, 42, u64::MAX - 1, u64::MAX] {
                for now in [0, 1, 2, 41, 42, u64::MAX - 1, u64::MAX] {
                    let expected = nanos_at(u128::from(now), &scale)
                        .saturating_sub(nanos_at(u128::from(start), &scale))
                        .min(Duration::MAX.as_nanos());
                    assert_eq!(scale.elapsed(start, now).as_nanos(), expected);
                }
            }
        }
        assert_eq!(
            TickScale {
                numer: 125,
                denom: 3
            }
            .elapsed(1, 2),
            Duration::from_nanos(42),
            "floor of the tick delta would report 41ns"
        );
        assert_eq!(
            TickScale {
                numer: 3,
                denom: 125
            }
            .elapsed(41, 42),
            Duration::from_nanos(1),
            "sub-nanosecond ticks still preserve the absolute rounding boundary"
        );
        assert_eq!(
            TickScale {
                numer: u32::MAX,
                denom: 1
            }
            .elapsed(0, u64::MAX),
            Duration::MAX
        );
    }

    #[test]
    fn admission_age_stays_inside_standard_clock_brackets() {
        let before = Instant::now();
        let admission = AdmissionStamp::now();
        let after = Instant::now();
        let lower = after.elapsed();
        let age = admission.elapsed();
        let upper = before.elapsed();
        assert!(
            lower <= age && age <= upper,
            "same precise sleep-excluding monotonic domain"
        );
        assert!(admission.earlier_by(Duration::MAX).is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn admission_stamp_retains_only_one_hardware_word() {
        assert_eq!(
            std::mem::size_of::<AdmissionStamp>(),
            std::mem::size_of::<u64>()
        );
        assert_eq!(
            std::mem::align_of::<AdmissionStamp>(),
            std::mem::align_of::<u64>()
        );
    }

    // Independent forward conversion at the exact rounding boundary. Its
    // split product also handles thresholds wider than the hardware clock.
    fn nanos_at(ticks: u128, scale: &TickScale) -> u128 {
        let numer = u128::from(scale.numer);
        let denom = u128::from(scale.denom);
        (ticks / denom) * numer + (ticks % denom) * numer / denom
    }
}
