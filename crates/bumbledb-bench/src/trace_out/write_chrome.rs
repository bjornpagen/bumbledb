use std::io::Write;
use std::path::{Path, PathBuf};

use bumbledb::obs::TraceEvent;

fn write_event(out: &mut impl Write, event: &TraceEvent, tid: u32) -> std::io::Result<()> {
    use std::fmt::Write as _;

    let mut line = String::new();
    line.push_str("{\"name\":");
    crate::json::push_str_lit(&mut line, event.name());
    line.push_str(",\"cat\":");
    crate::json::push_str_lit(&mut line, event.cat().label());
    match event {
        TraceEvent::Point { start_ns, .. } => {
            line.push_str(",\"ph\":\"i\",\"ts\":");
            crate::json::push_us(&mut line, *start_ns);
            line.push_str(",\"s\":\"t\"");
        }
        TraceEvent::Span {
            start_ns, dur_ns, ..
        } => {
            line.push_str(",\"ph\":\"X\",\"ts\":");
            crate::json::push_us(&mut line, *start_ns);
            line.push_str(",\"dur\":");
            crate::json::push_us(&mut line, *dur_ns);
        }
    }
    let _ = write!(
        line,
        ",\"pid\":1,\"tid\":{tid},\"args\":{{\"a0\":{},\"a1\":{}}}}}",
        event.a0(),
        event.a1()
    );
    out.write_all(line.as_bytes())
}

/// order is start-time order (spans record at drop, so the capture
/// # Errors
pub fn write_chrome(
    events: &[TraceEvent],
    harness: &[TraceEvent],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let mut all: Vec<(&TraceEvent, u32)> = events
        .iter()
        .map(|e| (e, 1))
        .chain(harness.iter().map(|e| (e, 2)))
        .collect();
    all.sort_by_key(|(e, _)| e.start_ns());
    out.write_all(b"[\n")?;
    for (index, (event, tid)) in all.iter().enumerate() {
        if index > 0 {
            out.write_all(b",\n")?;
        }
        write_event(out, event, *tid)?;
    }
    out.write_all(b"\n]\n")
}

/// # Errors
pub fn write_trace_file(
    trace_dir: &Path,
    stem: &str,
    events: &[TraceEvent],
    harness: &[TraceEvent],
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(trace_dir)?;
    let path = trace_dir.join(format!("{stem}.json"));
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path)?);
    write_chrome(events, harness, &mut file)?;
    file.flush()?;
    Ok(path)
}

/// # Errors
pub fn write_trace_pair(
    trace_dir: &Path,
    stem: &str,
    events: &[TraceEvent],
    harness: &[TraceEvent],
) -> std::io::Result<PathBuf> {
    let json = write_trace_file(trace_dir, stem, events, harness)?;
    std::fs::write(
        trace_dir.join(format!("{stem}.folded")),
        super::fold_stacks(events),
    )?;
    Ok(json)
}
