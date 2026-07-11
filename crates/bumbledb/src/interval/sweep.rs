//! The segment sweep: one covered-frontier walk, two continuation shapes
//! (`docs/architecture/30-dependencies.md` § enforcement,
//! `50-storage.md` § the commit pipeline).
//!
//! "Walk start-ordered segments, tracking the covered frontier" is one
//! algorithm with two owners: the containment judgment's coverage walk —
//! is the window `[s, e)` jointly covered, or where is the gap? — and
//! `Pack`'s coalescing fold (`20-query-ir.md`) — what are the maximal
//! segments of a claim set? The anti-probe precedent, replayed before a
//! copy can drift: the walk lives here once, and a caller is its
//! [`Continuation`] — a small trait, monomorphized, never `dyn`, the
//! sink/counter discipline.
//!
//! The adjacency law lives here and nowhere else: segments are half-open
//! point sets (the point-domain law, `10-data-model.md`), so
//! `start == frontier` continues a run and only `start > frontier`
//! breaks one. Overlapping inputs are legal (`Pack` folds arbitrary
//! claim sets) and impossible-by-key for the checker; the walk needs no
//! mode for the difference, because tracking the *maximum* frontier
//! subsumes disjoint chaining. Rays need no case either: ∞ is the
//! largest end word (`MAX`), so a ray is simply the segment whose
//! frontier no later input exceeds.
//!
//! Words are anything ordered and copyable: the checker walks the 8-byte
//! order-preserving halves of guard keys, `Pack` walks element words —
//! on both sides of the seam, word compare is point-order compare.

/// A caller of the sweep: what happens at the two places the walk can
/// speak — a consumed segment, and a maximal covered run. Implemented
/// per call site and monomorphized into [`sweep`]; never `dyn`.
pub(crate) trait Continuation<W, P> {
    type Error;

    /// A consumed segment, in input order, before it extends the
    /// frontier — the checker's per-segment σ re-check hangs here;
    /// `Pack` needs nothing and answers `Ok`.
    fn segment(&mut self, payload: P) -> Result<(), Self::Error>;

    /// A maximal covered run `[start, frontier)`: the next segment
    /// starts strictly past the frontier, or the input ended. Under a
    /// window this fires only while the window is uncovered — the
    /// checker's gap verdict; without one it is `Pack`'s emission.
    fn maximal(&mut self, start: W, frontier: W) -> Result<(), Self::Error>;
}

/// The covered-frontier walk over start-ordered `(start, end, payload)`
/// segments. With a window `[s, e)` the run opens at `s`, the walk
/// answers `Ok` the moment the frontier reaches `e` (later input is
/// moot), and any earlier stall — a gap, or exhaustion — is one
/// [`Continuation::maximal`] call and the walk ends: a stalled window
/// frontier can never recover, so the verdict is the continuation's to
/// raise. Without a window every maximal run is reported, gap-separated,
/// the last at exhaustion.
///
/// The input iterator owns its own trust boundary — a yielded error
/// aborts the walk untouched by the continuation. Windows are valid
/// intervals by every caller's construction; a degenerate one is
/// vacuously covered.
pub(crate) fn sweep<W, P, I, C>(
    segments: I,
    window: Option<(W, W)>,
    continuation: &mut C,
) -> Result<(), C::Error>
where
    W: Copy + Ord,
    I: IntoIterator<Item = Result<(W, W, P), C::Error>>,
    C: Continuation<W, P>,
{
    let mut segments = segments.into_iter();
    // The run: `[start, frontier)` is jointly covered by the consumed
    // input. A window demands coverage from `s`, so it opens the run
    // there; with no window the first segment opens it.
    let mut run: Option<(W, W)> = window.map(|(s, _)| (s, s));
    loop {
        if let (Some((_, frontier)), Some((_, e))) = (run, window) {
            if frontier >= e {
                // Window covered: success, the rest of the input is moot.
                return Ok(());
            }
        }
        let Some(item) = segments.next() else {
            // Exhaustion: the open run is maximal. Under a window this
            // is the gap-at-frontier verdict (the covered case returned
            // above); under none it is the final emission.
            if let Some((start, frontier)) = run {
                continuation.maximal(start, frontier)?;
            }
            return Ok(());
        };
        let (start, end, payload) = item?;
        if let Some((run_start, frontier)) = run {
            if start > frontier {
                // The gap — the one adjacency law: half-open segments
                // make `start == frontier` a continuation, so only
                // strictly-past breaks the run.
                continuation.maximal(run_start, frontier)?;
                if window.is_some() {
                    return Ok(());
                }
                continuation.segment(payload)?;
                run = Some((start, end));
            } else {
                continuation.segment(payload)?;
                run = Some((run_start, frontier.max(end)));
            }
        } else {
            continuation.segment(payload)?;
            run = Some((start, end));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{sweep, Continuation};

    /// A deterministic LCG (Knuth's MMIX line) so the property sweeps
    /// are reproducible; draws discard the weak low half.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// The `Pack` shape: collect every maximal run.
    struct Collect(Vec<(u64, u64)>);

    impl Continuation<u64, ()> for Collect {
        type Error = ();

        fn segment(&mut self, (): ()) -> Result<(), ()> {
            Ok(())
        }

        fn maximal(&mut self, start: u64, frontier: u64) -> Result<(), ()> {
            self.0.push((start, frontier));
            Ok(())
        }
    }

    /// The checker shape: any maximal run short of the window convicts.
    struct GapVerdict;

    impl Continuation<u64, ()> for GapVerdict {
        type Error = ();

        fn segment(&mut self, (): ()) -> Result<(), ()> {
            Ok(())
        }

        fn maximal(&mut self, _start: u64, _frontier: u64) -> Result<(), ()> {
            Err(())
        }
    }

    fn feed(segments: &[(u64, u64)]) -> impl Iterator<Item = Result<(u64, u64, ()), ()>> + '_ {
        segments.iter().map(|&(start, end)| Ok((start, end, ())))
    }

    fn pack(segments: &[(u64, u64)]) -> Vec<(u64, u64)> {
        let mut collect = Collect(Vec::new());
        sweep(feed(segments), None, &mut collect).expect("collect never fails");
        collect.0
    }

    fn covered(segments: &[(u64, u64)], window: (u64, u64)) -> bool {
        sweep(feed(segments), Some(window), &mut GapVerdict).is_ok()
    }

    // ---------- the naive point-set reference ----------
    //
    // The generated domain: bounded segments start below 26 and end at
    // or below 29; a ray ends at `u64::MAX`. The reference enumerates
    // literal points `0..=LAST_POINT` — two points past the largest
    // bounded end, so a run touching `LAST_POINT` can only be a ray's,
    // and a ray covers every point past the horizon too.
    const LAST_POINT: u64 = 30;

    fn covers_point(segments: &[(u64, u64)], p: u64) -> bool {
        segments.iter().any(|&(start, end)| start <= p && p < end)
    }

    /// Union-then-maximal-segments, as literal points.
    fn naive_pack(segments: &[(u64, u64)]) -> Vec<(u64, u64)> {
        let mut packed = Vec::new();
        let mut run: Option<(u64, u64)> = None; // inclusive point run
        for p in 0..=LAST_POINT {
            if covers_point(segments, p) {
                run = Some(run.map_or((p, p), |(a, _)| (a, p)));
            } else if let Some((a, b)) = run.take() {
                packed.push((a, b + 1));
            }
        }
        if let Some((a, b)) = run {
            // Only a ray reaches the horizon, and a ray runs to ∞.
            packed.push((a, if b == LAST_POINT { u64::MAX } else { b + 1 }));
        }
        packed
    }

    /// Random claim sets with the boundary cases forced: overlaps and
    /// containments arise freely from the dense domain, every fourth
    /// bounded draw is a ray, and every third segment sits exactly on
    /// its predecessor's adjacency boundary (`start == prev.end`) or one
    /// point past it — the minimal gap.
    fn random_segments(rng: &mut Lcg) -> Vec<(u64, u64)> {
        let count = rng.next() % 13;
        let mut segments: Vec<(u64, u64)> = Vec::new();
        for ordinal in 0..count {
            let derived = segments
                .last()
                .filter(|&&(_, end)| ordinal % 3 == 2 && end <= 24)
                .map(|&(_, end)| {
                    let start = end + rng.next() % 2;
                    (start, start + 1 + rng.next() % 4)
                });
            let segment = derived.unwrap_or_else(|| {
                let start = rng.next() % 24;
                let end = if rng.next().is_multiple_of(4) {
                    u64::MAX // the ray: ∞ is the largest end word
                } else {
                    start + 1 + rng.next() % 6
                };
                (start, end)
            });
            segments.push(segment);
        }
        segments.sort_unstable();
        segments
    }

    #[test]
    fn packed_output_matches_the_naive_point_set() {
        let mut rng = Lcg(0x5EED_0011);
        for _ in 0..2_000 {
            let segments = random_segments(&mut rng);
            assert_eq!(
                pack(&segments),
                naive_pack(&segments),
                "packed output diverges from the point-set union for {segments:?}"
            );
        }
    }

    #[test]
    fn coverage_verdict_matches_the_naive_subset_check() {
        let mut rng = Lcg(0x5EED_0022);
        for _ in 0..2_000 {
            let segments = random_segments(&mut rng);
            let s = rng.next() % 28;
            let e = if rng.next().is_multiple_of(3) {
                u64::MAX // a source ray: coverage demanded to ∞
            } else {
                s + 1 + rng.next() % (LAST_POINT - s)
            };
            // Pointwise truth over the sampled universe: past the
            // horizon only rays cover, and a ray covering `LAST_POINT`
            // covers everything beyond it too.
            let last = if e == u64::MAX { LAST_POINT } else { e - 1 };
            let reference = (s..=last).all(|p| covers_point(&segments, p));
            assert_eq!(
                covered(&segments, (s, e)),
                reference,
                "verdict diverges from the point-set subset check for \
                 {segments:?} over [{s}, {e})"
            );
        }
    }

    #[test]
    fn adjacency_continues_and_the_minimal_gap_breaks() {
        // Half-open law: `end == next.start` shares no point yet leaves
        // no hole — one maximal run; one point further is a gap.
        assert_eq!(pack(&[(2, 5), (5, 9)]), vec![(2, 9)]);
        assert!(covered(&[(2, 5), (5, 9)], (2, 9)));
        assert!(!covered(&[(2, 5), (5, 9)], (2, 10)));
        assert_eq!(pack(&[(2, 5), (6, 9)]), vec![(2, 5), (6, 9)]);
        assert!(!covered(&[(2, 5), (6, 9)], (2, 9)));
    }

    #[test]
    fn overlap_and_containment_fold_into_one_run() {
        // Arbitrary claim sets: a contained segment must not shrink the
        // frontier, an overlapping one extends it.
        assert_eq!(pack(&[(2, 10), (3, 4), (9, 12)]), vec![(2, 12)]);
        assert!(covered(&[(2, 10), (3, 4), (9, 12)], (2, 12)));
    }

    #[test]
    fn rays_are_ordinary_largest_end_words() {
        // A target ray covers any bounded window above its start; a
        // window to ∞ is satisfied only by a chain reaching a ray.
        assert!(covered(&[(5, u64::MAX)], (7, u64::MAX)));
        assert!(covered(&[(0, 3), (3, u64::MAX)], (0, u64::MAX)));
        assert!(!covered(&[(0, 3), (3, 9)], (0, u64::MAX)));
        // A bounded segment after a ray never exceeds its frontier.
        assert_eq!(pack(&[(5, u64::MAX), (6, 9)]), vec![(5, u64::MAX)]);
    }

    #[test]
    fn a_window_ignores_gaps_outside_itself() {
        // Input before `s` and holes before `s` are irrelevant: the run
        // opens at `s`, and max-frontier tracking rides over them.
        assert!(covered(&[(0, 1), (4, 9)], (5, 8)));
        assert!(covered(&[(1, 4)], (1, 4)));
        assert!(!covered(&[(1, 4)], (1, 5)));
    }

    #[test]
    fn empty_input_packs_nothing_and_covers_nothing() {
        assert_eq!(pack(&[]), Vec::<(u64, u64)>::new());
        assert!(!covered(&[], (0, 1)));
    }

    /// Pins the callback order the checker's σ re-check stands on.
    struct Trace(Vec<u64>);

    impl Continuation<u64, u64> for Trace {
        type Error = ();

        fn segment(&mut self, payload: u64) -> Result<(), ()> {
            self.0.push(payload);
            Ok(())
        }

        fn maximal(&mut self, _start: u64, _frontier: u64) -> Result<(), ()> {
            Err(())
        }
    }

    #[test]
    fn consumed_segments_are_handed_over_in_order_and_gaps_convict_first() {
        // The segment past a gap is never handed over under a window —
        // the verdict fires before its σ re-check would.
        let mut trace = Trace(Vec::new());
        let input = [Ok((1, 4, 0u64)), Ok((6, 9, 1u64))];
        assert_eq!(sweep(input, Some((1, 9)), &mut trace), Err(()));
        assert_eq!(trace.0, vec![0]);
        // Early success: input past a covered window is never consumed.
        let mut trace = Trace(Vec::new());
        let input = [Ok((1, 9, 0u64)), Ok((9, 12, 1u64))];
        assert_eq!(sweep(input, Some((2, 5)), &mut trace), Ok(()));
        assert_eq!(trace.0, vec![0]);
    }
}
