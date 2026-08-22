use super::*;

#[test]
fn nested_spans_record_containment_in_drop_order() {
    start_capture();
    {
        let mut outer = span(names::EXECUTE);
        std::hint::black_box(1 + 1);
        {
            let _inner = span_args(names::JOIN, TraceArgs::Pair(7, 9));
            std::hint::black_box(2 + 2);
        }
        outer.set_count(42);
    }
    let events = finish_capture();
    assert_eq!(events.len(), 2);
    // Drop order: inner lands first.
    let (inner, outer) = (&events[0], &events[1]);
    assert_eq!(inner.point(), names::JOIN);
    assert_eq!(outer.point(), names::EXECUTE);
    assert_eq!((inner.a0(), inner.a1()), (7, 9));
    assert_eq!(outer.a0(), 42, "set_count landed");
    assert!(outer.start_ns() <= inner.start_ns());
    assert!(inner.start_ns() + inner.dur_ns() <= outer.start_ns() + outer.dur_ns());
}

#[test]
fn point_events_record_as_points_with_args() {
    start_capture();
    event(names::CACHE_HIT, TraceArgs::Pair(3, 4));
    let events = finish_capture();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(
            events[0],
            TraceEvent::Point {
                point: names::CACHE_HIT,
                args: TraceArgs::Pair(3, 4),
                ..
            }
        ),
        "{:?}",
        events[0]
    );
}

#[test]
fn nothing_records_outside_capture() {
    {
        let _span = span(names::EXECUTE);
        event(names::CACHE_HIT, TraceArgs::None);
    }
    assert!(!capturing());
    start_capture();
    let events = finish_capture();
    assert!(events.is_empty());
}

#[test]
#[ignore = "stamp-cost pin gate; timing-sensitive, run manually"]
fn stamp_costs_match_the_measured_model() {
    use super::fastclock;
    let n = 1_000_000u64;

    let mut acc = 0u64;
    let start = fastclock::ticks();
    for _ in 0..n {
        acc = acc.wrapping_add(fastclock::ticks());
    }
    let raw_ticks = fastclock::ticks().wrapping_sub(start);
    std::hint::black_box(acc);
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let raw_ns = fastclock::ticks_to_ns(raw_ticks) as f64 / n as f64;

    let mut acc = 0u64;
    let start = fastclock::ticks();
    for _ in 0..n {
        acc = acc.wrapping_add(fastclock::ticks_ss());
    }
    let ss_ticks = fastclock::ticks().wrapping_sub(start);
    std::hint::black_box(acc);
    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let ss_ns = fastclock::ticks_to_ns(ss_ticks) as f64 / n as f64;

    assert!(
        raw_ns <= 0.6,
        "raw cntvct read: {raw_ns:.3} ns (model 0.30)"
    );
    assert!(ss_ns <= 7.0, "CNTVCTSS read: {ss_ns:.3} ns (model 4.6)");

    assert!(raw_ns < ss_ns, "raw {raw_ns:.3} vs ss {ss_ns:.3}");
}

#[test]
fn nested_start_capture_extends_instead_of_discarding() {
    start_capture();
    event(names::SAMPLE, TraceArgs::Count(1));
    start_capture(); 
    event(names::TOUCH, TraceArgs::Count(2));
    let events = finish_capture();
    assert_eq!(
        events.len(),
        2,
        "no event was destroyed by the nested start"
    );
    assert_eq!(events[0].point(), names::SAMPLE);
    assert_eq!(events[1].point(), names::TOUCH);
    assert!(!capturing(), "one finish drains the whole capture");
}

#[test]
fn sequential_captures_are_independent() {
    start_capture();
    event(names::SAMPLE, TraceArgs::None);
    let a = finish_capture();
    start_capture();
    event(names::TOUCH, TraceArgs::None);
    let b = finish_capture();
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_eq!(a[0].point(), names::SAMPLE);
    assert_eq!(b[0].point(), names::TOUCH);
}
