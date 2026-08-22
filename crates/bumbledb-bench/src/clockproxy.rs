//! P-cores 2.4–3.5 GHz and manufactured fake 2× findings before this

use std::time::{Duration, Instant};

pub const CONTAMINATION_GHZ: f64 = 3.2;

const CHAIN_MULS: u64 = 8;

const MUL_LATENCY_CYCLES: u64 = 3;

const PROXY_ITERS: u64 = 30_000;

#[cfg(target_arch = "aarch64")]
#[inline]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] 

fn chain(seed: u64, iters: u64) -> u64 {

    let mut x = seed | 1;
    for _ in 0..iters {
        // SAFETY: register-only integer multiplies; no memory access.
        unsafe {
            core::arch::asm!(
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                "mul {x}, {x}, {y}",
                x = inout(reg) x,
                y = in(reg) 0x9E37_79B9_7F4A_7C15_u64,
                options(nomem, nostack),
            );
        }
    }
    x
}

#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn chain(seed: u64, iters: u64) -> u64 {
    let mut x = seed | 1;
    for _ in 0..iters {
        for _ in 0..CHAIN_MULS {
            x = std::hint::black_box(x).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    x
}

#[must_use]
pub fn effective_ghz() -> f64 {
    let start = Instant::now();
    std::hint::black_box(chain(0x1234_5678, PROXY_ITERS));
    let ns = start.elapsed().as_nanos().max(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let cycles = (PROXY_ITERS * CHAIN_MULS * MUL_LATENCY_CYCLES) as f64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let ns = ns as f64;
    cycles / ns
}

pub fn warm_up(min: Duration) {
    let start = Instant::now();
    while start.elapsed() < min {
        std::hint::black_box(chain(1, 4_096));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhzStamp {
    pub pre: f64,
    pub post: f64,

    pub retried: bool,

    pub threshold: f64,
}

impl GhzStamp {

    #[must_use]
    pub fn min(&self) -> f64 {
        self.pre.min(self.post)
    }

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            pre: self.pre.min(other.pre),
            post: self.post.min(other.post),
            retried: self.retried || other.retried,
            threshold: self.threshold,
        }
    }

    /// NaN readings (no reference-host law off aarch64) never mark.
    #[must_use]
    pub fn contaminated(&self) -> bool {
        self.min() < self.threshold
    }
}

/// # Errors
pub fn frequency_checked<T, F>(f: F) -> Result<(T, GhzStamp), String>
where
    F: FnMut() -> Result<T, String>,
{
    frequency_checked_at(CONTAMINATION_GHZ, f)
}

/// # Errors
pub fn stamped<T, F>(mut f: F) -> Result<(T, GhzStamp), String>
where
    F: FnMut() -> Result<T, String>,
{
    let pre = effective_ghz();
    let value = f()?;
    let post = effective_ghz();
    Ok((
        value,
        GhzStamp {
            pre,
            post,
            retried: false,
            threshold: CONTAMINATION_GHZ,
        },
    ))
}

/// # Errors
pub fn frequency_checked_at<T, F>(threshold: f64, mut f: F) -> Result<(T, GhzStamp), String>
where
    F: FnMut() -> Result<T, String>,
{
    let pre = effective_ghz();
    let value = f()?;
    let post = effective_ghz();
    if pre.min(post) >= threshold {
        return Ok((
            value,
            GhzStamp {
                pre,
                post,
                retried: false,
                threshold,
            },
        ));
    }
    let pre = effective_ghz();
    let value = f()?;
    let post = effective_ghz();
    Ok((
        value,
        GhzStamp {
            pre,
            post,
            retried: true,
            threshold,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plausibility band transcribes aarch64 physics: only there is

    /// per mul, so off aarch64 the estimate is indicative, not a claim —

    #[test]
    #[cfg_attr(
        not(target_arch = "aarch64"),
        ignore = "host-pinned falsifier: the plausibility band transcribes the aarch64 asm chain's by-construction cycle count — the portable fallback is indicative only"
    )]
    fn the_estimate_is_a_plausible_core_frequency() {
        warm_up(Duration::from_millis(20));
        let ghz = effective_ghz();
        assert!(
            (0.5..=6.0).contains(&ghz),
            "effective GHz out of any plausible band: {ghz}"
        );
    }

    #[test]
    fn a_clean_bracket_never_retries() {
        let mut calls = 0u32;
        let ((), stamp) = frequency_checked_at(0.0, || {
            calls += 1;
            Ok(())
        })
        .expect("runs");
        assert_eq!(calls, 1);
        assert!(!stamp.retried);
        assert!(!stamp.contaminated(), "threshold 0 can never mark");
    }

    #[test]
    fn a_dirty_bracket_retries_exactly_once_and_reports_honestly() {

        let mut calls = 0u32;
        let (out, stamp) = frequency_checked_at(1e9, || {
            calls += 1;
            Ok(calls)
        })
        .expect("runs");
        assert_eq!(calls, 2, "exactly one bounded retry");
        assert_eq!(out, 2, "the retried block's value wins");
        assert!(stamp.retried);
        assert!(stamp.contaminated(), "still dirty after the retry");
    }

    #[test]
    #[ignore = "spin-load detector demonstration; run manually"]
    fn the_detector_fires_under_spin_load() {
        warm_up(Duration::from_millis(200));
        let quiet = (0..20).map(|_| effective_ghz()).fold(f64::MAX, f64::min);

        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let threads: Vec<_> = (0..24u64)
            .map(|i| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    let mut x = i + 1;
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        x = std::hint::black_box(chain(x, 4_096));
                    }
                    x
                })
            })
            .collect();

        let threshold = quiet - 0.15;
        let mut fired = false;
        let mut worst = f64::MAX;
        for _ in 0..200 {
            let ((), stamp) = frequency_checked_at(threshold, || Ok(())).expect("runs");
            worst = worst.min(stamp.min());
            if stamp.retried || stamp.contaminated() {
                fired = true;
                break;
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        for t in threads {
            let _ = t.join();
        }
        assert!(
            fired,
            "detector never fired under spin load: quiet floor {quiet:.2} GHz, \
             loaded worst {worst:.2} GHz"
        );
    }
}
