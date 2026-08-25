//! The duty binary: one body, two modes. `--once` is the Lambda arm;
//! the default is the resident sleep loop. Argv is parsed once.

use std::env;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use bumbledb::SchemaDescriptor;
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Ran};
use bumbledb_log::replica::{Fault, OpenRefusal};
use bumbledb_log::schema_file::{self, TheoryFile};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store};
use bumbledb_log::store::{ObjectStore, StoreError};

/// Resident sleep default. Consumer: this binary's default loop; the
/// scheduled cloud invoke is the peer, not a second cadence.
const SLEEP_MS: u64 = 5 * 60 * 1000;

enum Backend {
    Fs {
        root: PathBuf,
    },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        key_prefix: String,
    },
}

struct Config {
    once: bool,
    dir: PathBuf,
    prefix: String,
    theory: PathBuf,
    writer_id: u64,
    sleep: Duration,
    backend: Backend,
}

enum ConfigError {
    Unknown(String),
    Missing(&'static str),
    MissingValue(&'static str),
    BadStore(String),
    BadInt(&'static str),
}

enum Error {
    Config(ConfigError),
    Theory(TheoryFile),
    Store(StoreError),
    Fault(Fault),
    Refused(OpenRefusal),
    Credentials,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(flag) => write!(f, "unknown flag {flag}"),
            Self::Missing(name) => write!(f, "missing --{name}"),
            Self::MissingValue(name) => write!(f, "--{name} needs a value"),
            Self::BadStore(got) => write!(f, "store is fs or s3, not {got}"),
            Self::BadInt(name) => write!(f, "--{name} is not an integer"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "{error}"),
            Self::Theory(error) => write!(f, "{error}"),
            Self::Store(error) => write!(f, "{error}"),
            Self::Fault(error) => write!(f, "{error}"),
            Self::Refused(refusal) => write!(f, "open refused: {refusal:?}"),
            Self::Credentials => write!(f, "missing AWS_ACCESS_KEY_ID or AWS_SECRET_ACCESS_KEY"),
        }
    }
}

fn main() -> ExitCode {
    match start(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: impl Iterator<Item = String>) -> Result<(), Error> {
    let config = Config::parse(args).map_err(Error::Config)?;
    let theory = schema_file::load(&config.theory).map_err(Error::Theory)?;
    match &config.backend {
        Backend::Fs { root } => cycle(FsStore::new(root.clone()), &config, theory),
        Backend::S3 { .. } => cycle(open_s3(&config)?, &config, theory),
    }
}

fn cycle<S: ObjectStore>(store: S, config: &Config, theory: SchemaDescriptor) -> Result<(), Error> {
    let mut duty =
        match Checkpointer::open(store, &config.prefix, &config.dir, theory, config.writer_id)
            .map_err(Error::Fault)?
        {
            CheckpointerOpened::Ready(duty) => duty,
            CheckpointerOpened::Refused(refusal) => return Err(Error::Refused(refusal)),
        };
    loop {
        match duty.run().map_err(Error::Fault)? {
            Ran::Ready { .. } => {}
            Ran::RefreshRefused(refusal) => return Err(Error::Refused(refusal)),
        }
        if config.once {
            return Ok(());
        }
        thread::sleep(config.sleep);
    }
}

fn open_s3(config: &Config) -> Result<S3Store, Error> {
    let Backend::S3 {
        bucket,
        region,
        endpoint,
        key_prefix,
    } = &config.backend
    else {
        unreachable!("open_s3 is the s3 arm");
    };
    let access_key_id = env::var("AWS_ACCESS_KEY_ID").map_err(|_| Error::Credentials)?;
    let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| Error::Credentials)?;
    if access_key_id.is_empty() || secret_access_key.is_empty() {
        return Err(Error::Credentials);
    }
    S3Store::new(&S3Config {
        endpoint: endpoint.clone(),
        region: region.clone(),
        bucket: bucket.clone(),
        credentials: S3Credentials::Static {
            access_key_id,
            secret_access_key,
            session_token: env::var("AWS_SESSION_TOKEN").ok().filter(|t| !t.is_empty()),
        },
        prefix: key_prefix.clone(),
    })
    .map_err(Error::Store)
}

fn take_value(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &'static str,
) -> Result<String, ConfigError> {
    match args.peek() {
        Some(next) if next.starts_with("--") => Err(ConfigError::MissingValue(name)),
        None => Err(ConfigError::MissingValue(name)),
        Some(_) => args.next().ok_or(ConfigError::MissingValue(name)),
    }
}

impl Config {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, ConfigError> {
        let mut once = false;
        let mut dir = None;
        let mut prefix = String::new();
        let mut theory = None;
        let mut writer_id = 0;
        let mut sleep_ms = SLEEP_MS;
        let mut store = None;
        let mut root = None;
        let mut bucket = None;
        let mut region = "us-east-1".to_string();
        let mut endpoint = None;
        let mut key_prefix = String::new();
        let mut args = args.peekable();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--once" => once = true,
                "--dir" => dir = Some(PathBuf::from(take_value(&mut args, "dir")?)),
                "--prefix" => prefix = take_value(&mut args, "prefix")?,
                "--theory" => theory = Some(PathBuf::from(take_value(&mut args, "theory")?)),
                "--writer" => {
                    writer_id = take_value(&mut args, "writer")?
                        .parse()
                        .map_err(|_| ConfigError::BadInt("writer"))?;
                }
                "--sleep-ms" => {
                    sleep_ms = take_value(&mut args, "sleep-ms")?
                        .parse()
                        .map_err(|_| ConfigError::BadInt("sleep-ms"))?;
                }
                "--store" => store = Some(take_value(&mut args, "store")?),
                "--root" => root = Some(PathBuf::from(take_value(&mut args, "root")?)),
                "--bucket" => bucket = Some(take_value(&mut args, "bucket")?),
                "--region" => region = take_value(&mut args, "region")?,
                "--endpoint" => endpoint = Some(take_value(&mut args, "endpoint")?),
                "--s3-prefix" => key_prefix = take_value(&mut args, "s3-prefix")?,
                other => return Err(ConfigError::Unknown(other.to_string())),
            }
        }
        let backend = match store.as_deref() {
            Some("fs") => Backend::Fs {
                root: root.ok_or(ConfigError::Missing("root"))?,
            },
            Some("s3") => Backend::S3 {
                bucket: bucket.ok_or(ConfigError::Missing("bucket"))?,
                region,
                endpoint,
                key_prefix,
            },
            Some(got) => return Err(ConfigError::BadStore(got.to_string())),
            None => return Err(ConfigError::Missing("store")),
        };
        Ok(Self {
            once,
            dir: dir.ok_or(ConfigError::Missing("dir"))?,
            prefix,
            theory: theory.ok_or(ConfigError::Missing("theory"))?,
            writer_id,
            sleep: Duration::from_millis(sleep_ms),
            backend,
        })
    }
}
