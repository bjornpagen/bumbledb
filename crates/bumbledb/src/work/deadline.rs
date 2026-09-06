//! Operation deadlines and diagnostic stamps in one monotonic clock domain.
//!
//! Rust's pinned Apple Instant implementation uses `CLOCK_UPTIME_RAW`: the
//! converted value of `mach_absolute_time`, excluding system sleep. Convert
//! the deadline once, not every checkpoint. Other platforms retain Instant.
//! This changes neither polling cadence nor cancellation/refusal priority.
use std::time::{Duration, Instant};

use super::WorkError;

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

#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct Deadline {
    low: u64,
    high: u64,
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
pub(super) struct Deadline(Instant);

impl Deadline {
    pub(super) fn start(timeout: Duration) -> Result<Self, WorkError> {
        // The native origin precedes the validation sample, so conversion
        // cannot grant extra queue time. No Instant representation is read
        // or reconstructed; std still decides the accepted timeout range.
        #[cfg(target_os = "macos")]
        let started = native_now();
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(WorkError::InvalidTimeout)?;
        #[cfg(target_os = "macos")]
        {
            let _ = deadline;
            Ok(Self::from_ticks(timebase().deadline(started, timeout)))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self(deadline))
        }
    }

    #[cfg(target_os = "macos")]
    fn from_ticks(ticks: u128) -> Self {
        // Preserve Instant's 8-byte alignment inside Ledger. A native u128
        // field would raise alignment to 16 and can enlarge every context.
        // The representation remains lossless; no packed or unaligned reads.
        Self {
            low: u64::try_from(ticks & u128::from(u64::MAX)).expect("masked low word"),
            high: u64::try_from(ticks >> 64).expect("upper word fits u64"),
        }
    }

    #[cfg(target_os = "macos")]
    fn expired_at(&self, now: u64) -> bool {
        self.high == 0 && now >= self.low
    }

    pub(super) fn expired(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            // Keep one native read at every existing deadline checkpoint,
            // including thresholds beyond the hardware counter's range.
            self.expired_at(native_now())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Instant::now() >= self.0
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

    fn deadline(&self, started: u64, timeout: Duration) -> u128 {
        let numer = u128::from(self.numer);
        let denom = u128::from(self.denom);
        // Match the clock's floor-to-nanoseconds admission observation.
        // start + ceil(duration in ticks) can add a whole extra tick.
        let nanos = self.nanos(started) + timeout.as_nanos();
        // The product fits: floor(start*numer/denom)*denom <= start*numer
        // needs at most 96 bits; Duration nanos (<2^94) times denom (<2^32)
        // needs at most 126 bits. Their sum is below 2^127. Keep the full
        // threshold, even when it exceeds the hardware counter's u64 range.
        // One final division also preserves zero-timeout clock plateaus.
        (nanos * denom).div_ceil(numer)
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

    #[test]
    #[cfg(target_os = "macos")]
    fn split_tick_deadline_preserves_instant_layout_and_wide_comparison() {
        assert_eq!(
            std::mem::size_of::<Deadline>(),
            std::mem::size_of::<Instant>()
        );
        assert_eq!(
            std::mem::align_of::<Deadline>(),
            std::mem::align_of::<Instant>()
        );
        for ticks in [
            0,
            1,
            u128::from(u64::MAX),
            u128::from(u64::MAX) + 1,
            u128::MAX,
        ] {
            let deadline = Deadline::from_ticks(ticks);
            assert_eq!(
                (u128::from(deadline.high) << 64) | u128::from(deadline.low),
                ticks
            );
            for now in [0, 1, u64::MAX - 1, u64::MAX] {
                assert_eq!(deadline.expired_at(now), u128::from(now) >= ticks);
            }
        }
    }

    // Independent forward conversion at the exact rounding boundary. Its
    // split product also handles thresholds wider than the hardware clock.
    fn nanos_at(ticks: u128, scale: &TickScale) -> u128 {
        let numer = u128::from(scale.numer);
        let denom = u128::from(scale.denom);
        (ticks / denom) * numer + (ticks % denom) * numer / denom
    }

    #[test]
    fn converted_deadline_is_the_first_tick_whose_nanoseconds_reach_due() {
        for (numer, denom) in [
            (1, 1),
            (125, 3),
            (3, 125),
            (1, u32::MAX),
            (u32::MAX, 1),
            (u32::MAX, u32::MAX - 1),
        ] {
            let scale = TickScale { numer, denom };
            for start in [0, 1, 2, 3, u64::MAX - 1, u64::MAX] {
                for timeout in [
                    Duration::ZERO,
                    Duration::from_nanos(1),
                    Duration::from_nanos(42),
                    Duration::from_secs(1),
                    Duration::MAX,
                ] {
                    let due = nanos_at(u128::from(start), &scale) + timeout.as_nanos();
                    let tick = scale.deadline(start, timeout);
                    assert!(nanos_at(tick, &scale) >= due);
                    if tick > 0 {
                        assert!(nanos_at(tick - 1, &scale) < due);
                    }
                    if timeout.is_zero() {
                        assert!(u128::from(start) >= tick);
                    }
                }
            }
        }
        let scale = TickScale {
            numer: 125,
            denom: 3,
        };
        assert_eq!(
            scale.deadline(1, Duration::from_nanos(42)),
            2,
            "rounding duration separately would incorrectly wait until tick 3"
        );
    }

    #[test]
    fn direct_deadline_product_matches_split_reference_across_full_width_inputs() {
        let mut seed = 0x79b9_7f4a_7c15_9e37u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            seed
        };
        for _ in 0..1024 {
            let numer = u32::try_from(next() >> 32).expect("high word").max(1);
            let denom = u32::try_from(next() >> 32).expect("high word").max(1);
            let start = next();
            let timeout = Duration::new(next(), u32::try_from(next() % 1_000_000_000).unwrap());
            let scale = TickScale { numer, denom };
            assert_direct_product(&scale, start, timeout);
        }
        for (numer, denom) in [
            (1, 1),
            (125, 3),
            (3, 125),
            (1, u32::MAX),
            (u32::MAX, 1),
            (u32::MAX, u32::MAX),
            (u32::MAX - 1, u32::MAX),
            (u32::MAX, u32::MAX - 1),
        ] {
            let scale = TickScale { numer, denom };
            for start in [0, 1, 41, 42, u64::MAX - 1, u64::MAX] {
                for timeout in [Duration::ZERO, Duration::from_nanos(1), Duration::MAX] {
                    assert_direct_product(&scale, start, timeout);
                }
            }
        }
    }

    fn assert_direct_product(scale: &TickScale, start: u64, timeout: Duration) {
        let numer = u128::from(scale.numer);
        let denom = u128::from(scale.denom);
        let due = nanos_at(u128::from(start), scale) + timeout.as_nanos();
        let product = due
            .checked_mul(denom)
            .expect("correlated product fits u128");
        assert!(product < 1u128 << 127);
        let reference = (due / numer) * denom + ((due % numer) * denom).div_ceil(numer);
        let tick = scale.deadline(start, timeout);
        assert_eq!(tick, reference);
        assert!(nanos_at(tick, scale) >= due);
        if tick != 0 {
            assert!(nanos_at(tick - 1, scale) < due);
        }
    }

    #[test]
    fn std_still_rejects_unrepresentable_timeout_and_zero_expires() {
        assert!(matches!(
            Deadline::start(Duration::MAX),
            Err(WorkError::InvalidTimeout)
        ));
        assert!(Deadline::start(Duration::ZERO).unwrap().expired());
        let large = Duration::from_hours(100 * 365 * 24);
        assert!(Instant::now().checked_add(large).is_some());
        assert!(!Deadline::start(large).unwrap().expired());
    }
}
