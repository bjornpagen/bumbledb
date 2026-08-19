use super::*;
use bumbledb::obs::{TraceArgs, TracePoint, names};

fn span(point: TracePoint, start_ns: u64, dur_ns: u64, a0: u64) -> TraceEvent {
    TraceEvent::Span {
        point,
        start_ns,
        dur_ns,
        args: if a0 == 0 {
            TraceArgs::None
        } else {
            TraceArgs::Count(a0)
        },
    }
}

fn point(point: TracePoint, start_ns: u64, a0: u64) -> TraceEvent {
    TraceEvent::Point {
        point,
        start_ns,
        args: TraceArgs::Count(a0),
    }
}

#[test]
fn the_chrome_writer_is_golden_and_structurally_valid() {
    let engine = vec![
        span(names::PREPARE, 1000, 2500, 0),
        span(names::EXECUTE, 4000, 10000, 7),
        span(names::JOIN, 5000, 8000, 0),
        point(names::CACHE_HIT, 6000, 3),
    ];
    let harness = vec![span(names::SAMPLE, 900, 15000, 0)];
    let mut out = Vec::new();
    write_chrome(&engine, &harness, &mut out).expect("writes");
    let text = String::from_utf8(out).expect("utf-8");
    let expected = "[\n\
        {\"name\":\"sample\",\"cat\":\"harness\",\"ph\":\"X\",\"ts\":0.900,\"dur\":15.000,\"pid\":1,\"tid\":2,\"args\":{\"a0\":0,\"a1\":0}},\n\
        {\"name\":\"prepare\",\"cat\":\"prepare\",\"ph\":\"X\",\"ts\":1.000,\"dur\":2.500,\"pid\":1,\"tid\":1,\"args\":{\"a0\":0,\"a1\":0}},\n\
        {\"name\":\"execute\",\"cat\":\"execute\",\"ph\":\"X\",\"ts\":4.000,\"dur\":10.000,\"pid\":1,\"tid\":1,\"args\":{\"a0\":7,\"a1\":0}},\n\
        {\"name\":\"join\",\"cat\":\"execute\",\"ph\":\"X\",\"ts\":5.000,\"dur\":8.000,\"pid\":1,\"tid\":1,\"args\":{\"a0\":0,\"a1\":0}},\n\
        {\"name\":\"cache_hit\",\"cat\":\"cache\",\"ph\":\"i\",\"ts\":6.000,\"s\":\"t\",\"pid\":1,\"tid\":1,\"args\":{\"a0\":3,\"a1\":0}}\n\
        ]\n";
    assert_eq!(text, expected);

    // Structural validity: balanced brackets, one object per event,
    // ts monotone nondecreasing in file order.
    assert_eq!(text.matches('{').count(), text.matches('}').count());
    assert_eq!(text.matches("\"name\":").count(), 5);
    let ts: Vec<f64> = text
        .lines()
        .filter_map(|line| {
            let start = line.find("\"ts\":")? + 5;
            let rest = &line[start..];
            let end = rest.find(',')?;
            rest[..end].parse().ok()
        })
        .collect();
    assert_eq!(ts.len(), 5);
    assert!(ts.windows(2).all(|w| w[0] <= w[1]), "{ts:?}");
}

#[test]
fn every_registered_name_is_escape_free_ascii() {
    // The writer relies on the registry discipline instead of
    // escaping machinery.
    assert!(FlameSummary::compute(&[]).rows.is_empty());
    let names = [
        bumbledb::obs::names::PREPARE,
        bumbledb::obs::names::EXECUTE,
        bumbledb::obs::names::JOIN,
        bumbledb::obs::names::VIEW_BUILD,
        bumbledb::obs::names::VIEW_MEMO_HIT,
        bumbledb::obs::names::SAMPLE,
        bumbledb::obs::names::TOUCH,
    ];
    for name in names {
        assert!(
            name.label().is_ascii() && !name.label().contains('"') && !name.label().contains('\\'),
            "{name}"
        );
    }
}

#[test]
fn the_flame_summary_computes_exact_self_time() {
    // Outer 100 us containing inner 60 us: outer self = 40 us.
    let events = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::JOIN, 10_000, 60_000, 0),
    ];
    let summary = FlameSummary::compute(&events);
    assert_eq!(summary.wall_ns, 100_000);
    assert_eq!(summary.rows.len(), 2);
    let inner = &summary.rows[0];
    assert_eq!(
        (inner.name, inner.total_ns, inner.self_ns),
        ("join", 60_000, 60_000),
        "inner leads by self time"
    );
    let outer = &summary.rows[1];
    assert_eq!(
        (outer.name, outer.total_ns, outer.self_ns),
        ("execute", 100_000, 40_000)
    );

    // Only DIRECT children are subtracted: grandchildren charge the
    // middle span, not the outer one.
    let nested = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::VIEWS, 10_000, 60_000, 0),
        span(names::FINALIZE, 20_000, 30_000, 0),
    ];
    let summary = FlameSummary::compute(&nested);
    let by_name = |name: &str| {
        summary
            .rows
            .iter()
            .find(|row| row.name == name)
            .expect("row")
            .self_ns
    };
    assert_eq!(by_name("execute"), 40_000);
    assert_eq!(by_name("views"), 30_000);
    assert_eq!(by_name("finalize"), 30_000);
}

#[test]
fn the_table_render_is_golden() {
    let events = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::JOIN, 10_000, 60_000, 0),
    ];
    let summary = FlameSummary::compute(&events);
    let expected = "span                       calls     total_us      self_us       p50_us       max_us\n\
                    join                           1       60.000       60.000       60.000       60.000\n\
                    execute                        1      100.000       40.000      100.000      100.000\n\
                    total wall 100.000 us\n";
    assert_eq!(summary.render(), expected);
}

#[test]
fn fold_stacks_charges_self_time_by_enclosure_path() {
    // outer(100) ⊃ middle(60) ⊃ leaf(30): each frame's self time is its
    // duration minus its direct child, and the stack path names the chain.
    let nested = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::VIEWS, 10_000, 60_000, 0),
        span(names::FINALIZE, 20_000, 30_000, 0),
    ];
    let folded = fold_stacks(&nested);
    // BTreeMap order: a prefix sorts before its extension.
    let expected = "execute 40000\n\
                    execute;views 30000\n\
                    execute;views;finalize 30000\n";
    assert_eq!(folded, expected);
}

#[test]
fn fold_stacks_collapses_identical_sibling_stacks() {
    // Two same-named children of one parent land on ONE folded line,
    // their self times summed (the collapsed-stack contract). Ordinary
    // point events carry no charge and never appear.
    let events = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::JOIN, 10_000, 20_000, 0),
        span(names::JOIN, 40_000, 30_000, 0),
        point(names::CACHE_HIT, 50_000, 3),
    ];
    let folded = fold_stacks(&events);
    let expected = "execute 50000\n\
                    execute;join 50000\n";
    assert_eq!(folded, expected);
    assert!(
        !folded.contains("cache_hit"),
        "point events carry no self time"
    );
}

#[test]
fn fold_stacks_charges_phase_accumulators_under_their_join() {
    // Phase accumulators flush AFTER the rule loop (their stamps sit
    // past the join span's end, inside execute), carrying the join
    // loop's interior time in a0. The fold hangs each under the nearest
    // preceding join and makes room in that join's self time — a
    // join-dominated capture stops rendering as one flat bar.
    let events = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        span(names::JOIN, 10_000, 50_000, 0),
        point(names::JOIN_PHASE[2][0], 70_000, 30_000),
        point(names::JOIN_PHASE[0][0], 70_001, 10_000),
    ];
    let expected = "execute 50000\n\
                    execute;join 10000\n\
                    execute;join;jp_iter_n0 10000\n\
                    execute;join;jp_probe_n0 30000\n";
    assert_eq!(fold_stacks(&events), expected);

    // No join anywhere: the accumulator charges the deepest span
    // containing its stamp — attribution, not identification.
    let joinless = vec![
        span(names::EXECUTE, 0, 100_000, 0),
        point(names::JOIN_PHASE[1][1], 5_000, 20_000),
    ];
    let expected = "execute 80000\n\
                    execute;jp_hash_n1 20000\n";
    assert_eq!(fold_stacks(&joinless), expected);

    // The flame summary still excludes them — the phase table is their
    // terminal rendering.
    let summary = FlameSummary::compute(&events);
    assert!(summary.rows.iter().all(|row| !row.name.starts_with("jp_")));
}

#[test]
fn fold_stacks_empty_capture_is_empty() {
    assert_eq!(fold_stacks(&[]), "");
}

#[test]
fn a_non_phase_event_never_suppresses_the_phase_table() {
    // A non-phase point sitting beside a registered accumulator must not
    // suppress the table — only JoinPhase rows render.
    let registered = names::JOIN_PHASE[0][0];
    let events = vec![
        point(registered, 0, 10_000),
        point(names::CACHE_HIT, 0, 5_000),
    ];
    let table = render_phase_table(&events).expect("the registered row still renders");
    assert!(table.contains(registered.label()), "{table}");
    assert!(!table.contains("cache_hit"), "{table}");

    // Nothing registered at all still means no table.
    let alien = vec![point(names::CACHE_HIT, 0, 5_000)];
    assert!(render_phase_table(&alien).is_none());
}

#[test]
fn equal_tick_nests_resolve_parenthood_by_drop_order() {
    // A sub-tick child inside a sub-tick parent shares BOTH endpoints on
    // the 41.67 ns counter. Spans record at drop, so the CHILD lands in
    // the buffer first — the sweep must order the parent first anyway
    // (a stable (start, -end) sort alone kept the buffer order and
    // inverted the pair, charging the parent to the child).
    let events = vec![
        span(names::JOIN, 1_000, 42, 0),
        span(names::EXECUTE, 1_000, 42, 0),
    ];

    let folded = fold_stacks(&events);
    assert_eq!(folded, "execute 0\nexecute;join 42\n");

    let summary = FlameSummary::compute(&events);
    let self_of = |name: &str| {
        summary
            .rows
            .iter()
            .find(|row| row.name == name)
            .expect("row")
            .self_ns
    };
    assert_eq!(
        self_of("execute"),
        0,
        "the child charge lands on the parent"
    );
    assert_eq!(self_of("join"), 42);

    // Distinct-tick nests are untouched: the (start, -end) key alone
    // already orders the parent first, record order notwithstanding.
    let distinct = vec![
        span(names::JOIN, 1_000, 42, 0),
        span(names::EXECUTE, 1_000, 84, 0),
    ];
    assert_eq!(fold_stacks(&distinct), "execute 42\nexecute;join 42\n");
}

/// A real captured S-scale `containment_walk` trace: the expected spans appear
/// and the summary wall tracks the execute span within 5%.
#[cfg(feature = "obs")]
#[test]
fn a_real_containment_walk_capture_summarizes_to_the_execute_span() {
    use crate::corpus_gen::{GenConfig, Scale};
    use crate::harness::Rotation;

    let dir = std::env::temp_dir().join("bumbledb-bench-trace-out");
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = GenConfig {
        seed: 1,
        scale: Scale::S,
    };
    let db = bumbledb::Db::create(&dir.join("db"), crate::schema::Ledger)
        .expect("create")
        .expect("accepted");
    crate::corpus::load_bumbledb(&db, cfg).expect("load");

    let family = crate::families::all()
        .iter()
        .find(|f| f.name == "containment_walk")
        .expect("registered");
    let mut prepared = db.prepare(&(family.query)()).expect("prepare");
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = bumbledb::Answers::new();
    let mut run = || {
        let args = crate::families::param_args(rotation.next_set());
        db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{e:?}"))?;
        Ok(buffer.len() as u64)
    };
    // Warm first — the traced sample is a warm one.
    for _ in 0..4 {
        run().expect("warm");
    }
    let (_, events) = crate::harness::traced_sample(&mut run).expect("traced");
    let (engine, harness) = split_harness(events);
    let names: std::collections::HashSet<&str> = engine.iter().map(|event| event.name()).collect();
    assert!(names.contains("execute"), "{names:?}");
    assert!(names.contains("join"), "{names:?}");
    assert!(
        names.contains("view_build") || names.contains("view_memo_hit"),
        "{names:?}"
    );
    assert_eq!(harness.len(), 1, "the sample span");

    let summary = FlameSummary::compute(&engine);
    let execute = summary
        .rows
        .iter()
        .find(|row| row.name == "execute")
        .expect("execute row");
    let wall = summary.wall_ns;
    assert!(
        wall.abs_diff(execute.total_ns) * 20 <= execute.total_ns,
        "wall {wall} vs execute {} exceeds 5%",
        execute.total_ns
    );

    // And it exports the pair: the Chrome json AND the collapsed fold.
    let path = write_trace_pair(
        &dir.join("trace"),
        "containment_walk.warm",
        &engine,
        &harness,
    )
    .expect("export");
    let text = std::fs::read_to_string(&path).expect("read back");
    assert!(text.starts_with("[\n") && text.ends_with("\n]\n"));
    let folded_path = path.with_extension("folded");
    let folded = std::fs::read_to_string(&folded_path).expect("folded beside json");
    // The root of every fold line is the execute span, and each line ends
    // in a nanosecond self-time count.
    assert!(!folded.is_empty(), "the fold is non-empty");
    for line in folded.lines() {
        assert!(line.starts_with("execute"), "root frame: {line}");
        let count = line.rsplit(' ').next().expect("a count");
        assert!(count.parse::<u64>().is_ok(), "self-ns tail: {line}");
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
