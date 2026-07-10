//! The serial-ALU clock proxy: a dependent `mul` chain whose cycle
//! count is known by construction
//! (8 muls × latency 3 = 24 cycles per iteration on the reference core),
//! timed over a loop-amortized window. Wall time over known cycles is an
//! effective-frequency estimate — the discriminator that separates "the
//! code got slower" from "the clock got slower" (co-tenant builds swing
//! P-cores 2.4–3.5 GHz and manufactured fake 2× findings before this
//! control existed — measured).
//!
//! Contamination is any wall-time inflation of the fixed-cycle chain:
//! DVFS, E-core scheduling, and preemption all read as a low GHz
//! estimate, which is exactly the set of conditions that also poison a
//! measurement taken in the same window.

use std::time::{Duration, Instant};

/// Below this effective frequency a measurement block is presumed
/// contaminated (reference host P-cores sit at 3.3–3.5 GHz warm; the
/// observed contamination band starts at 2.4).
pub const CONTAMINATION_GHZ: f64 = 3.2;

/// Dependent multiplies per chain iteration.
const CHAIN_MULS: u64 = 8;
/// `mul` latency on the reference core (dougallj, confirmed on M2).
const MUL_LATENCY_CYCLES: u64 = 3;
/// Iterations per proxy read: 24 cycles × 30k ≈ 200 µs at 3.5 GHz —
/// thousands of 41.67 ns timer quanta, so quantization is noise.
const PROXY_ITERS: u64 = 30_000;

/// One serial chain of `iters × CHAIN_MULS` dependent multiplies. The
/// Rust loop around the asm block costs ~2 instructions per 24-cycle
/// iteration and runs on ports the chain never saturates — it does not
/// extend the dependent chain.
#[cfg(target_arch = "aarch64")]
#[inline]
#[allow(unsafe_code)] // 00-product policy: register-only asm, no memory —
                      // the proxy's cycle count must be known by construction, which no
                      // compiler-emitted loop guarantees across rustc versions.
fn chain(seed: u64, iters: u64) -> u64 {
    // An odd multiplier keeps the chain value from collapsing to zero.
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

/// Portable fallback: the same dependent chain in plain Rust. The
/// 3-cycle-mul assumption is reference-host law; off aarch64 the
/// estimate is only indicative (and the threshold conservative).
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

/// One effective-frequency reading in GHz (known cycles / measured ns).
#[must_use]
pub fn effective_ghz() -> f64 {
    let start = Instant::now();
    std::hint::black_box(chain(0x1234_5678, PROXY_ITERS));
    let ns = start.elapsed().as_nanos().max(1);
    #[allow(clippy::cast_precision_loss)]
    let cycles = (PROXY_ITERS * CHAIN_MULS * MUL_LATENCY_CYCLES) as f64;
    #[allow(clippy::cast_precision_loss)]
    let ns = ns as f64;
    cycles / ns
}

/// Spins the chain until `min` wall time has passed — the DVFS ramp
/// eater (the findings measured opening calibrations at 3.06 GHz vs
/// 3.49 steady-state; ≥ 200 ms of warm work closes that gap).
pub fn warm_up(min: Duration) {
    let start = Instant::now();
    while start.elapsed() < min {
        std::hint::black_box(chain(1, 4_096));
    }
}

/// The proxy bracket around one measurement block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhzStamp {
    pub pre: f64,
    pub post: f64,
    /// The block was re-measured once because its first bracket read
    /// contaminated.
    pub retried: bool,
    /// The threshold this stamp was judged against.
    pub threshold: f64,
}

impl GhzStamp {
    /// The worse of the two bracket readings.
    #[must_use]
    pub fn min(&self) -> f64 {
        self.pre.min(self.post)
    }

    /// The worst of two brackets guarded around one family's pair of
    /// measurement blocks — contamination of either engine's block
    /// dirties the ratio, so the merged stamp reads component-wise
    /// worst.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            pre: self.pre.min(other.pre),
            post: self.post.min(other.post),
            retried: self.retried || other.retried,
            threshold: self.threshold,
        }
    }

    /// Whether the FINAL bracket (post-retry) still reads contaminated.
    /// NaN readings (no reference-host law off aarch64) never mark.
    #[must_use]
    pub fn contaminated(&self) -> bool {
        self.min() < self.threshold
    }
}

/// Runs `f` bracketed by proxy readings; on a contaminated bracket the
/// block is re-measured exactly once (bounded retry — beyond that the
/// dirt is reported, never hidden by a retry loop). The closure must be
/// idempotent — read-family measurement blocks are; write blocks use
/// [`stamped`] instead (their contamination is fsync-DVFS physics, and
/// re-running a block that creates stores is not a retry, it's a crash).
///
/// # Errors
///
/// The closure's error, verbatim.
pub fn guarded<T, F>(f: F) -> Result<(T, GhzStamp), String>
where
    F: FnMut() -> Result<T, String>,
{
    guarded_at(CONTAMINATION_GHZ, f)
}

/// Brackets `f` with proxy readings WITHOUT retry — the non-idempotent
/// (write-family) form: the stamp annotates, never re-runs.
///
/// # Errors
///
/// The closure's error, verbatim.
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

/// [`guarded`] with an injectable threshold (the detector's test seam).
///
/// # Errors
///
/// The closure's error, verbatim.
pub fn guarded_at<T, F>(threshold: f64, mut f: F) -> Result<(T, GhzStamp), String>
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

    #[test]
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
        let ((), stamp) = guarded_at(0.0, || {
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
        // A threshold above any real frequency forces the dirty path
        // deterministically: one retry, then honest contamination.
        let mut calls = 0u32;
        let (out, stamp) = guarded_at(1e9, || {
            calls += 1;
            Ok(calls)
        })
        .expect("runs");
        assert_eq!(calls, 2, "exactly one bounded retry");
        assert_eq!(out, 2, "the retried block's value wins");
        assert!(stamp.retried);
        assert!(stamp.contaminated(), "still dirty after the retry");
    }

    /// The detector fires under real co-tenant load: oversubscribe the
    /// machine with spin threads, and the measuring thread's wall time
    /// inflates (preemption, E-core placement, DVFS) — the proxy reads
    /// below its own quiet floor. Ignored: needs a quiet-then-loaded
    /// machine and several seconds; run manually.
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

        // Under 24-way oversubscription some bracket must read below the
        // quiet floor; 200 attempts gives the scheduler every chance.
        let threshold = quiet - 0.15;
        let mut fired = false;
        let mut worst = f64::MAX;
        for _ in 0..200 {
            let ((), stamp) = guarded_at(threshold, || Ok(())).expect("runs");
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
