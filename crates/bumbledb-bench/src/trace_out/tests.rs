use super::*;

fn span(name: &'static str, cat: Category, start_ns: u64, dur_ns: u64, a0: u64) -> TraceEvent {
    TraceEvent {
        name,
        cat,
        start_ns,
        dur_ns,
        a0,
        a1: 0,
    }
}

#[test]
fn the_chrome_writer_is_golden_and_structurally_valid() {
    let engine = vec![
        span("prepare", Category::Prepare, 1000, 2500, 0),
        span("execute", Category::Execute, 4000, 10000, 7),
        span("join", Category::Execute, 5000, 8000, 0),
        span("cache_hit", Category::Cache, 6000, 0, 3),
    ];
    let harness = vec![span("sample", Category::Harness, 900, 15000, 0)];
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
            name.is_ascii() && !name.contains('"') && !name.contains('\\'),
            "{name}"
        );
    }
}

#[test]
fn the_flame_summary_computes_exact_self_time() {
    // Outer 100 us containing inner 60 us: outer self = 40 us.
    let events = vec![
        span("outer", Category::Execute, 0, 100_000, 0),
        span("inner", Category::Execute, 10_000, 60_000, 0),
    ];
    let summary = FlameSummary::compute(&events);
    assert_eq!(summary.wall_ns, 100_000);
    assert_eq!(summary.rows.len(), 2);
    let inner = &summary.rows[0];
    assert_eq!(
        (inner.name, inner.total_ns, inner.self_ns),
        ("inner", 60_000, 60_000),
        "inner leads by self time"
    );
    let outer = &summary.rows[1];
    assert_eq!(
        (outer.name, outer.total_ns, outer.self_ns),
        ("outer", 100_000, 40_000)
    );

    // Only DIRECT children are subtracted: grandchildren charge the
    // middle span, not the outer one.
    let nested = vec![
        span("outer", Category::Execute, 0, 100_000, 0),
        span("middle", Category::Execute, 10_000, 60_000, 0),
        span("leaf", Category::Execute, 20_000, 30_000, 0),
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
    assert_eq!(by_name("outer"), 40_000);
    assert_eq!(by_name("middle"), 30_000);
    assert_eq!(by_name("leaf"), 30_000);
}

#[test]
fn the_table_render_is_golden() {
    let events = vec![
        span("outer", Category::Execute, 0, 100_000, 0),
        span("inner", Category::Execute, 10_000, 60_000, 0),
    ];
    let summary = FlameSummary::compute(&events);
    let expected = "span                       calls     total_us      self_us       p50_us       max_us\n\
                    inner                          1       60.000       60.000       60.000       60.000\n\
                    outer                          1      100.000       40.000      100.000      100.000\n\
                    total wall 100.000 us\n";
    assert_eq!(summary.render(), expected);
}

/// A real captured S-scale `fk_walk` trace: the expected spans appear
/// and the summary wall tracks the execute span within 5%.
#[cfg(feature = "obs")]
#[test]
fn a_real_fk_walk_capture_summarizes_to_the_execute_span() {
    use crate::gen::{GenConfig, Scale};
    use crate::harness::Rotation;

    let dir = std::env::temp_dir().join("bumbledb-bench-trace-out");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let cfg = GenConfig {
        seed: 1,
        scale: Scale::S,
    };
    let db = bumbledb::Db::create(&dir.join("db"), crate::schema::schema()).expect("create");
    crate::corpus::load_bumbledb(&db, cfg).expect("load");

    let family = crate::families::all()
        .iter()
        .find(|f| f.name == "fk_walk")
        .expect("registered");
    let mut prepared = db.prepare(&(family.query)()).expect("prepare");
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = bumbledb::ResultBuffer::new();
    let mut run = || {
        let params = rotation.next_set().to_vec();
        db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
            .map_err(|e| format!("{e:?}"))?;
        Ok(buffer.len() as u64)
    };
    // Warm first — the traced sample is a warm one.
    for _ in 0..4 {
        run().expect("warm");
    }
    let (_, events) = crate::harness::traced_sample(&mut run).expect("traced");
    let (engine, harness) = split_harness(events);
    let names: std::collections::HashSet<&str> =
        engine.iter().map(|event| event.name).collect();
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

    // And it exports.
    let path = write_trace_file(&dir.join("trace"), "fk_walk.warm", &engine, &harness)
        .expect("export");
    let text = std::fs::read_to_string(path).expect("read back");
    assert!(text.starts_with("[\n") && text.ends_with("\n]\n"));
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
