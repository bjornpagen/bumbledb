fn init_tracing(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "bumbledb_lmdb=debug".to_owned());
    match config.trace_format {
        TraceFormat::Fmt => {
            if let Some(path) = &config.trace_output {
                let writer = SharedTraceWriter::create(path)?;
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_writer(writer)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            } else {
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            }
        }
        TraceFormat::Json => {
            if let Some(path) = &config.trace_output {
                let writer = SharedTraceWriter::create(path)?;
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_writer(writer)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            } else {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            }
        }
        TraceFormat::Chrome | TraceFormat::Flame => {
            return Err(bench_error(
                "trace format requires an optional profiler dependency that is not enabled",
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
struct SharedTraceWriter {
    file: Arc<Mutex<File>>,
}

impl SharedTraceWriter {
    fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            file: Arc::new(Mutex::new(File::create(path)?)),
        })
    }
}

struct SharedTraceWriterGuard<'a> {
    file: MutexGuard<'a, File>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedTraceWriter {
    type Writer = SharedTraceWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        SharedTraceWriterGuard {
            file: self
                .file
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        }
    }
}

impl IoWrite for SharedTraceWriterGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn next_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| bench_error(format!("missing value for {flag}")))
}

pub(crate) fn bench_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.into(),
    ))
}

pub(crate) struct Dataset {
    name: &'static str,
    schema: SchemaDescriptor,
    facts: Vec<Fact>,
    fact_source: Option<open::FactSource>,
    sqlite_schema: &'static str,
    sqlite_insert: SqliteInsert,
    queries: Vec<BenchQuery>,
}

pub(crate) type SqliteInsert = fn(&Connection, &[Fact]) -> Result<(), Box<dyn std::error::Error>>;

pub(crate) struct BenchQuery {
    name: &'static str,
    build: fn(&SchemaDescriptor) -> QueryBuildResult<TypedQuery>,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CorrectnessMode {
    ResultSet,
}

impl CorrectnessMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResultSet => "result-set",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SqlValue {
    Integer(i64),
    Text(String),
    Blob(Vec<u8>),
}
