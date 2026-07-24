use super::*;

#[test]
fn nested_spans_record_containment_in_drop_order() {
    start_capture();
    {
        let mut outer = span("outer", Category::Execute);
        std::hint::black_box(1 + 1);
        {
            let _inner = span_args("inner", Category::Execute, 7, 9);
            std::hint::black_box(2 + 2);
        }
        outer.set_args(42, 0);
    }
    let events = finish_capture();
    assert_eq!(events.len(), 2);
    // Drop order: inner lands first.
    let (inner, outer) = (&events[0], &events[1]);
    assert_eq!(inner.name, "inner");
    assert_eq!(outer.name, "outer");
    assert_eq!((inner.a0, inner.a1), (7, 9));
    assert_eq!(outer.a0, 42, "set_args landed");
    assert!(outer.start_ns <= inner.start_ns);
    assert!(inner.start_ns + inner.dur_ns <= outer.start_ns + outer.dur_ns);
}

#[test]
fn point_events_record_zero_duration_and_args() {
    start_capture();
    event("tick", Category::Cache, 3, 4);
    let events = finish_capture();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].dur_ns, 0);
    assert_eq!((events[0].a0, events[0].a1), (3, 4));
}

#[test]
fn nothing_records_outside_capture() {
    {
        let _span = span("ghost", Category::Execute);
        event("ghost-event", Category::Execute, 0, 0);
    }
    assert!(!capturing());
    start_capture();
    let events = finish_capture();
    assert!(events.is_empty());
}

/// The stamp-cost pin (measured): raw
/// `cntvct` reads are ~0.30 ns (1/cycle) and `CNTVCTSS` back-to-back
/// ~4.6 ns on the reference host; the gates leave headroom for load.
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
    // The ordering that justifies the policy: ss costs more than raw,
    // and both are far under the old ~2 ns budget assumption.
    assert!(raw_ns < ss_ns, "raw {raw_ns:.3} vs ss {ss_ns:.3}");
}

#[test]
fn nested_start_capture_extends_instead_of_discarding() {
    start_capture();
    event("before", Category::Harness, 1, 0);
    start_capture(); // idempotent: the live buffer survives
    event("after", Category::Harness, 2, 0);
    let events = finish_capture();
    assert_eq!(
        events.len(),
        2,
        "no event was destroyed by the nested start"
    );
    assert_eq!(events[0].name, "before");
    assert_eq!(events[1].name, "after");
    assert!(!capturing(), "one finish drains the whole capture");
}

#[test]
fn sequential_captures_are_independent() {
    start_capture();
    event("first", Category::Harness, 0, 0);
    let a = finish_capture();
    start_capture();
    event("second", Category::Harness, 0, 0);
    let b = finish_capture();
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_eq!(a[0].name, "first");
    assert_eq!(b[0].name, "second");
}
