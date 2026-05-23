fn read_job_csv(
    dir: &Path,
    file: &str,
    accepted_limit: Option<usize>,
    mut f: impl FnMut(StringRecord) -> Result<bool, Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    eprintln!(
        "[bench:job] reading {} limit={}",
        file,
        accepted_limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "full".to_owned())
    );
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;
    let mut accepted = 0;
    let mut read = 0usize;
    for record in reader.records() {
        if reached_limit(accepted, accepted_limit) {
            break;
        }
        read += 1;
        if f(record?)? {
            accepted += 1;
            if accepted % 100_000 == 0 {
                eprintln!("[bench:job] {} accepted={} read={}", file, accepted, read);
            }
        }
    }
    eprintln!(
        "[bench:job] finished {} accepted={} read={}",
        file, accepted, read
    );
    Ok(())
}

fn job_text(value: &str) -> String {
    if value.is_empty() || value == r"\N" {
        String::new()
    } else {
        value.to_owned()
    }
}

fn read_csv(
    dir: &Path,
    file: &str,
    limit: Option<usize>,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        f(&headers, &record?)?;
    }
    Ok(())
}

fn read_pipe(
    dir: &Path,
    file: &str,
    limit: Option<usize>,
    mut f: impl FnMut(StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = require_file(dir, file)?;
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        f(record?)?;
    }
    Ok(())
}

fn read_pipe_path(
    path: &Path,
    limit: Option<usize>,
    mut f: impl FnMut(&StringRecord, &StringRecord) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b'|')
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    for (read, record) in reader.records().enumerate() {
        if reached_limit(read, limit) {
            break;
        }
        f(&headers, &record?)?;
    }
    Ok(())
}

fn tsv_reader(path: &Path) -> Result<csv::Reader<std::fs::File>, Box<dyn std::error::Error>> {
    Ok(ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .from_path(path)?)
}

fn require_file(dir: &Path, file: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(file);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("missing required dataset file {}", path.display()).into())
    }
}

fn find_prefixed(dir: &Path, prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".csv")
            && (name == format!("{prefix}.csv") || name.starts_with(&format!("{prefix}_")))
        {
            candidates.push(path);
        }
    }
    candidates.sort();
    if let Some(path) = candidates.into_iter().next() {
        return Ok(path);
    }
    Err(format!(
        "missing LDBC file with prefix {prefix} in {}",
        dir.display()
    )
    .into())
}

fn get(record: &StringRecord, index: usize) -> &str {
    record.get(index).unwrap_or("")
}

fn col<'a>(headers: &StringRecord, record: &'a StringRecord, names: &[&str]) -> &'a str {
    col_n(headers, record, names, 0)
}

fn col_n<'a>(
    headers: &StringRecord,
    record: &'a StringRecord,
    names: &[&str],
    occurrence: usize,
) -> &'a str {
    for name in names {
        let mut seen = 0;
        for (index, header) in headers.iter().enumerate() {
            if header == *name {
                if seen == occurrence {
                    return record.get(index).unwrap_or("");
                }
                seen += 1;
            }
        }
    }
    ""
}

