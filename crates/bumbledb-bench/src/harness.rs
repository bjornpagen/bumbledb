//! The measurement engine (docs/benchmarks/13): warmup → measured
//! samples → exact percentiles, with optional allocation windows and a
//! precisely defined cold protocol. Everything the report prints comes
//! from here — the harness owns time, never queries (runners pass
//! closures over their own prepared statements).

use std::time::Instant;

use bumbledb::obs::TraceEvent;
use bumbledb::Value;

/// The warmup/measure protocol. Warm reads use [`Protocol::WARM`]; writes
/// and cold runs use fewer (docs/benchmarks/15, [`Protocol::COLD`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protocol {
    pub warmups: u32,
    pub samples: u32,
}

impl Protocol {
    /// The warm-read default: 32 warmups, 256 measured samples.
    pub const WARM: Self = Self {
        warmups: 32,
        samples: 256,
    };
    /// The cold default: every sample pays the touch, so few are needed.
    pub const COLD: Self = Self {
        warmups: 2,
        samples: 16,
    };
}

/// Exact percentiles of one measured window, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
    pub mean_ns: u64,
}

/// Sorts the samples in place and takes **nearest-rank** percentiles:
/// `idx = ceil(p/100 × n) - 1` over the ascending sort (so p50 of
/// `[10, 20]` is 10, p99 of 100 samples is the 99th). `mean_ns` is the
/// integer mean. Reproducible by hand — no interpolation.
///
/// # Panics
///
/// On an empty sample vector (a programmer error — protocols demand at
/// least one sample).
#[must_use]
pub fn stats(samples: &mut [u64]) -> Stats {
    assert!(!samples.is_empty(), "stats over zero samples");
    samples.sort_unstable();
    let n = samples.len() as u64;
    let rank = |p: u64| {
        let idx = (p * n).div_ceil(100) - 1;
        samples[usize::try_from(idx).expect("index fits")]
    };
    Stats {
        min: samples[0],
        p50: rank(50),
        p90: rank(90),
        p95: rank(95),
        p99: rank(99),
        max: samples[samples.len() - 1],
        mean_ns: samples.iter().sum::<u64>() / n,
    }
}

/// One measured window: percentiles plus the summed per-sample work
/// counts (the anti-dead-code contract — every runner drains its rows
/// and returns the count, which the harness black-boxes and sums).
#[derive(Debug, Clone)]
pub struct Measurement {
    pub stats: Stats,
    pub work: u64,
    /// The allocation window over the measured samples, when
    /// [`Modes::alloc_window`] ran (needs the `obs` feature).
    #[cfg(feature = "obs")]
    pub alloc: Option<bumbledb::alloc_counter::AllocSnapshot>,
    /// One additional post-measurement traced sample, when
    /// [`Modes::trace`] ran — traces never contaminate the measured
    /// samples.
    pub trace: Option<(u64, Vec<TraceEvent>)>,
}

/// Optional harness modes — mutually exclusive (README rule: an
/// allocation window and a trace capture never share an invocation).
#[derive(Debug, Clone, Copy, Default)]
pub struct Modes {
    pub alloc_window: bool,
    pub trace: bool,
}

/// [`measure_with`] in plain timing mode.
///
/// # Errors
///
/// The closure's error, verbatim.
pub fn measure<F>(proto: Protocol, f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    measure_with(proto, Modes::default(), f)
}

/// The one measurement loop: `warmups` untimed calls, then `samples`
/// calls timed individually (`Instant::now` around exactly the call),
/// their work counts black-boxed and summed.
///
/// # Errors
///
/// The closure's error; a request for both modes at once; an
/// alloc-window request on a build without the `obs` feature.
pub fn measure_with<F>(proto: Protocol, modes: Modes, mut f: F) -> Result<Measurement, String>
where
    F: FnMut() -> Result<u64, String>,
{
    if modes.alloc_window && modes.trace {
        return Err("alloc-window and trace-capture are mutually exclusive modes".to_owned());
    }
    #[cfg(not(feature = "obs"))]
    if modes.alloc_window {
        return Err("the alloc window needs the obs feature (bumbledb/alloc-counter)".to_owned());
    }
    for _ in 0..proto.warmups {
        std::hint::black_box(f()?);
    }
    #[cfg(feature = "obs")]
    if modes.alloc_window {
        bumbledb::alloc_counter::reset();
    }
    let mut samples = Vec::with_capacity(proto.samples as usize);
    let mut work = 0u64;
    for _ in 0..proto.samples {
        let start = Instant::now();
        let count = f()?;
        samples.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        work += std::hint::black_box(count);
    }
    #[cfg(feature = "obs")]
    let alloc = modes.alloc_window.then(bumbledb::alloc_counter::snapshot);
    let trace = if modes.trace {
        Some(traced_sample(&mut f)?)
    } else {
        None
    };
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        #[cfg(feature = "obs")]
        alloc,
        trace,
    })
}

/// One traced sample: the closure runs inside `obs::start_capture` /
/// `finish_capture` (empty without the engine's `trace` feature).
///
/// # Errors
///
/// The closure's error (the capture is drained either way).
pub fn traced_sample<F>(f: &mut F) -> Result<(u64, Vec<TraceEvent>), String>
where
    F: FnMut() -> Result<u64, String>,
{
    bumbledb::obs::start_capture();
    let result = f();
    let events = bumbledb::obs::finish_capture();
    Ok((result?, events))
}

/// The cold protocol, defined exactly: per sample (warmups included),
/// `touch()` runs first — committing one fact, bumping the generation,
/// and evicting the image cache — then `f()` is timed once.
///
/// # Errors
///
/// Either closure's error, verbatim.
pub fn measure_cold<T, F>(proto: Protocol, mut touch: T, mut f: F) -> Result<Measurement, String>
where
    T: FnMut() -> Result<(), String>,
    F: FnMut() -> Result<u64, String>,
{
    let mut samples = Vec::with_capacity(proto.samples as usize);
    let mut work = 0u64;
    for round in 0..proto.warmups + proto.samples {
        touch()?;
        let start = Instant::now();
        let count = f()?;
        let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if round >= proto.warmups {
            samples.push(elapsed);
            work += std::hint::black_box(count);
        }
    }
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        #[cfg(feature = "obs")]
        alloc: None,
        trace: None,
    })
}

/// The canonical cold touch: commits one `Tag` fact whose label carries
/// the serial id under the `__touch_` prefix — unique forever (serials
/// never repeat) and disjoint from every corpus label (`tag-NNN`).
pub fn tag_touch<'d>(db: &'d bumbledb::Db<'_>) -> impl FnMut() -> Result<(), String> + 'd {
    move || {
        db.write(|tx| {
            let id: crate::schema::TagId = tx.alloc()?;
            tx.insert(&crate::schema::Tag {
                id: crate::schema::TagId(id.0),
                label: format!("__touch_{}", id.0),
            })
        })
        .map(|_| ())
        .map_err(|e| format!("cold touch: {e:?}"))
    }
}

/// Round-robin over a fixed param-set vector — the gate-style rotation
/// (misses included exactly where the family's policy says so).
#[derive(Debug, Clone)]
pub struct Rotation {
    sets: Vec<Vec<Value>>,
    cursor: usize,
}

impl Rotation {
    /// # Panics
    ///
    /// On an empty set vector (even param-less families carry one empty
    /// set).
    #[must_use]
    pub fn new(sets: Vec<Vec<Value>>) -> Self {
        assert!(!sets.is_empty(), "a rotation needs at least one set");
        Self { sets, cursor: 0 }
    }

    /// The next param set, wrapping around.
    pub fn next_set(&mut self) -> &[Value] {
        let set = &self.sets[self.cursor];
        self.cursor = (self.cursor + 1) % self.sets.len();
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_match_hand_computed_nearest_rank() {
        let mut one = vec![7];
        let s = stats(&mut one);
        assert_eq!((s.min, s.p50, s.p99, s.max, s.mean_ns), (7, 7, 7, 7, 7));

        let mut two = vec![20, 10];
        let s = stats(&mut two);
        // n = 2: p50 rank = ceil(1) - 1 = 0 -> 10; p90/p95/p99 -> 20.
        assert_eq!(s.min, 10);
        assert_eq!(s.p50, 10);
        assert_eq!(s.p90, 20);
        assert_eq!(s.p95, 20);
        assert_eq!(s.p99, 20);
        assert_eq!(s.max, 20);
        assert_eq!(s.mean_ns, 15);

        let mut hundred: Vec<u64> = (1..=100).rev().collect();
        let s = stats(&mut hundred);
        assert_eq!(s.min, 1);
        assert_eq!(s.p50, 50);
        assert_eq!(s.p90, 90);
        assert_eq!(s.p95, 95);
        assert_eq!(s.p99, 99);
        assert_eq!(s.max, 100);
        assert_eq!(s.mean_ns, 50, "integer mean of 50.5");
    }

    #[test]
    fn rotation_is_deterministic_round_robin() {
        let mut rotation = Rotation::new(vec![
            vec![Value::U64(0)],
            vec![Value::U64(1)],
            vec![Value::U64(2)],
        ]);
        let order: Vec<Value> = (0..7).map(|_| rotation.next_set()[0].clone()).collect();
        let expected: Vec<Value> = [0, 1, 2, 0, 1, 2, 0].map(Value::U64).into();
        assert_eq!(order, expected);
    }

    #[test]
    fn measure_calls_exactly_warmups_plus_samples_and_sums_work() {
        let proto = Protocol {
            warmups: 3,
            samples: 5,
        };
        let mut calls = 0u64;
        let m = measure(proto, || {
            calls += 1;
            Ok(2)
        })
        .expect("measures");
        assert_eq!(calls, 8, "3 warmups + 5 samples");
        assert_eq!(m.work, 10, "work sums the samples only");
        assert!(m.trace.is_none());
    }

    #[test]
    fn the_modes_are_mutually_exclusive() {
        let err = measure_with(
            Protocol::COLD,
            Modes {
                alloc_window: true,
                trace: true,
            },
            || Ok(0),
        )
        .expect_err("must refuse");
        assert!(err.contains("mutually exclusive"), "{err}");
    }

    #[test]
    fn trace_mode_adds_one_post_measurement_sample() {
        let proto = Protocol {
            warmups: 1,
            samples: 2,
        };
        let mut calls = 0u64;
        let m = measure_with(
            proto,
            Modes {
                alloc_window: false,
                trace: true,
            },
            || {
                calls += 1;
                Ok(1)
            },
        )
        .expect("measures");
        assert_eq!(calls, 4, "warmup + samples + the traced sample");
        assert_eq!(m.work, 2, "the traced sample's work is not summed");
        let (traced_work, _) = m.trace.expect("traced");
        assert_eq!(traced_work, 1);
    }

    #[cfg(feature = "obs")]
    #[test]
    fn the_alloc_window_returns_a_snapshot() {
        let proto = Protocol {
            warmups: 2,
            samples: 4,
        };
        let m = measure_with(
            proto,
            Modes {
                alloc_window: true,
                trace: false,
            },
            || Ok(std::hint::black_box(vec![1u8; 4096]).len() as u64),
        )
        .expect("measures");
        let alloc = m.alloc.expect("windowed");
        assert!(alloc.allocs >= 4, "each sample allocates");
        assert!(alloc.alloc_bytes >= 4 * 4096);
    }

    #[cfg(not(feature = "obs"))]
    #[test]
    fn the_alloc_window_refuses_without_the_feature() {
        let err = measure_with(
            Protocol::COLD,
            Modes {
                alloc_window: true,
                trace: false,
            },
            || Ok(0),
        )
        .expect_err("must refuse");
        assert!(err.contains("obs feature"), "{err}");
    }

    /// The touch runs before every sample (warmups included), and
    /// generations strictly increase across samples on a real store.
    #[test]
    fn cold_touches_before_every_sample_and_bumps_generations() {
        use std::cell::RefCell;
        let script = RefCell::new(String::new());
        let proto = Protocol {
            warmups: 2,
            samples: 3,
        };
        let m = measure_cold(
            proto,
            || {
                script.borrow_mut().push('t');
                Ok(())
            },
            || {
                script.borrow_mut().push('f');
                Ok(1)
            },
        )
        .expect("measures");
        assert_eq!(script.borrow().as_str(), "tftftftftf");
        assert_eq!(m.work, 3, "samples only");

        let dir = std::env::temp_dir().join("bumbledb-bench-harness-cold");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, crate::schema::schema()).expect("create");
        let generations = RefCell::new(Vec::new());
        measure_cold(proto, tag_touch(&db), || {
            let generation = db.generation().map_err(|e| format!("{e:?}"))?;
            generations.borrow_mut().push(generation);
            Ok(1)
        })
        .expect("measures");
        let generations = generations.into_inner();
        assert_eq!(generations.len(), 5);
        assert!(
            generations.windows(2).all(|w| w[0] < w[1]),
            "every touch bumps the generation: {generations:?}"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
