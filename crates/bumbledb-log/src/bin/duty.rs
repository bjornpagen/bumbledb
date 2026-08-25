//! The duty binary: `--once` is the Lambda arm; the default is the
//! resident sleep loop; `inspect <key>` renders a protocol document to
//! text. Argv is a parsed grammar; the exit code is a total function
//! of the outcome.

use std::env;
use std::fmt;
use std::fmt::Write as _;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use bumbledb::SchemaDescriptor;
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Compact, Ran};
use bumbledb_log::gc::Gc;
use bumbledb_log::inspect::{self, InspectError, Kind};
use bumbledb_log::manifest::{Published, hex32};
use bumbledb_log::replica::{Fault, OpenRefusal};
use bumbledb_log::schema_file::{self, TheoryFile};
use bumbledb_log::sidecar::CHAIN_FILE;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store};
use bumbledb_log::store::{ObjectStore, StoreError, StoreKey};

/// Resident sleep default. Consumer: this binary's default loop; the
/// scheduled cloud invoke is the peer, not a second cadence.
const SLEEP_MS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    Once,
    Inspect,
    Dir,
    Prefix,
    Theory,
    Writer,
    SleepMs,
    Store,
    Root,
    Bucket,
    Region,
    Endpoint,
    S3Prefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreKind {
    Fs,
    S3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Once,
    Resident { sleep: Duration },
    Inspect { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug)]
struct Config {
    mode: Mode,
    dir: PathBuf,
    prefix: String,
    theory: PathBuf,
    writer_id: u64,
    backend: Backend,
}

#[derive(Debug, PartialEq, Eq)]
enum ConfigError {
    Unknown(String),
    Missing(&'static str),
    MissingValue(&'static str),
    BadStore(String),
    BadInt(&'static str),
    OnceSleep,
    InspectDuty,
    Cross { flag: Flag, backend: StoreKind },
}

enum Error {
    Config(ConfigError),
    Theory(TheoryFile),
    Store(StoreError),
    Fault(Fault),
    Refused(OpenRefusal),
    Credentials,
    Inspect(InspectError),
}

enum Outcome {
    Duty(Ran),
    Inspected(String),
}

enum Atom {
    Flag(Flag),
    Bound { name: Flag, value: String },
    Bare(String),
}

#[derive(Clone)]
enum ModeDraft {
    Default,
    Once,
    Resident { sleep_ms: u64 },
    Inspect { key: String },
}

enum Draft {
    Empty,
    FsHint {
        root: Option<PathBuf>,
    },
    S3Hint {
        bucket: Option<String>,
        region: String,
        endpoint: Option<String>,
        key_prefix: String,
    },
    Fs {
        root: Option<PathBuf>,
    },
    S3 {
        bucket: Option<String>,
        region: String,
        endpoint: Option<String>,
        key_prefix: String,
    },
}

impl Flag {
    fn parse(name: &str) -> Result<Self, ConfigError> {
        Ok(match name {
            "once" => Self::Once,
            "inspect" => Self::Inspect,
            "dir" => Self::Dir,
            "prefix" => Self::Prefix,
            "theory" => Self::Theory,
            "writer" => Self::Writer,
            "sleep-ms" => Self::SleepMs,
            "store" => Self::Store,
            "root" => Self::Root,
            "bucket" => Self::Bucket,
            "region" => Self::Region,
            "endpoint" => Self::Endpoint,
            "s3-prefix" => Self::S3Prefix,
            other => return Err(ConfigError::Unknown(format!("--{other}"))),
        })
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Inspect => "inspect",
            Self::Dir => "dir",
            Self::Prefix => "prefix",
            Self::Theory => "theory",
            Self::Writer => "writer",
            Self::SleepMs => "sleep-ms",
            Self::Store => "store",
            Self::Root => "root",
            Self::Bucket => "bucket",
            Self::Region => "region",
            Self::Endpoint => "endpoint",
            Self::S3Prefix => "s3-prefix",
        }
    }
}

impl StoreKind {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "fs" => Ok(Self::Fs),
            "s3" => Ok(Self::S3),
            got => Err(ConfigError::BadStore(got.to_string())),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Fs => "fs",
            Self::S3 => "s3",
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(flag) => write!(f, "unknown flag {flag}"),
            Self::Missing(name) => write!(f, "missing --{name}"),
            Self::MissingValue(name) => write!(f, "--{name} needs a value"),
            Self::BadStore(got) => write!(f, "store is fs or s3, not {got}"),
            Self::BadInt(name) => write!(f, "--{name} is not an integer"),
            Self::OnceSleep => write!(f, "--once does not take --sleep-ms"),
            Self::InspectDuty => write!(f, "inspect does not take --once or --sleep-ms"),
            Self::Cross { flag, backend } => {
                write!(
                    f,
                    "--{} does not apply to store {}",
                    flag.name(),
                    backend.name()
                )
            }
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
            Self::Refused(refusal) => write!(f, "duty refused: {refusal:?}"),
            Self::Credentials => write!(f, "missing AWS_ACCESS_KEY_ID or AWS_SECRET_ACCESS_KEY"),
            Self::Inspect(error) => write!(f, "{error}"),
        }
    }
}

fn main() -> ExitCode {
    match start(env::args().skip(1)) {
        Ok(Outcome::Duty(ran)) => {
            eprint!("{}", scream(&ran));
            ExitCode::from(code(&ran))
        }
        Ok(Outcome::Inspected(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: impl Iterator<Item = String>) -> Result<Outcome, Error> {
    let config = Config::parse(args).map_err(Error::Config)?;
    let theory = schema_file::load(&config.theory).map_err(Error::Theory)?;
    match &config.mode {
        Mode::Inspect { key } => match &config.backend {
            Backend::Fs { root } => {
                inspect(&FsStore::new(root.clone()), &config, &theory, key).map(Outcome::Inspected)
            }
            Backend::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            } => inspect(
                &open_s3(bucket, region, endpoint.as_deref(), key_prefix)?,
                &config,
                &theory,
                key,
            )
            .map(Outcome::Inspected),
        },
        Mode::Once | Mode::Resident { .. } => match &config.backend {
            Backend::Fs { root } => {
                cycle(FsStore::new(root.clone()), &config, theory).map(Outcome::Duty)
            }
            Backend::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            } => cycle(
                open_s3(bucket, region, endpoint.as_deref(), key_prefix)?,
                &config,
                theory,
            )
            .map(Outcome::Duty),
        },
    }
}

/// Fetch the object and render it through the crate parsers.
fn inspect<S: ObjectStore>(
    store: &S,
    config: &Config,
    theory: &SchemaDescriptor,
    key: &str,
) -> Result<String, Error> {
    let kind = inspect::kind(key).map_err(Error::Inspect)?;
    let bytes = match kind {
        Kind::Sidecar => match std::fs::read(config.dir.join(CHAIN_FILE)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(Error::Inspect(InspectError::Missing));
            }
            Err(error) => {
                return Err(Error::Inspect(InspectError::Io(error.to_string())));
            }
        },
        Kind::Manifest | Kind::Checkpoint | Kind::Batch => {
            let store_key = object_key(&config.prefix, key)?;
            store
                .get(&store_key)
                .map_err(Error::Store)?
                .ok_or(Error::Inspect(InspectError::Missing))?
                .bytes
        }
    };
    inspect::render(kind, &bytes, theory).map_err(Error::Inspect)
}

fn object_key(prefix: &str, rest: &str) -> Result<StoreKey, Error> {
    let raw = if prefix.is_empty() {
        rest.to_string()
    } else {
        format!("{prefix}/{rest}")
    };
    StoreKey::parse(&raw).map_err(|error| Error::Inspect(InspectError::Key(error.key)))
}

fn cycle<S: ObjectStore>(
    store: S,
    config: &Config,
    theory: SchemaDescriptor,
) -> Result<Ran, Error> {
    let mut duty =
        match Checkpointer::open(store, &config.prefix, &config.dir, theory, config.writer_id)
            .map_err(Error::Fault)?
        {
            CheckpointerOpened::Ready(duty) => duty,
            CheckpointerOpened::Refused(refusal) => return Err(Error::Refused(refusal)),
        };
    loop {
        let ran = duty.run().map_err(Error::Fault)?;
        match config.mode {
            Mode::Once => return Ok(ran),
            Mode::Resident { sleep: _ } if !ready(&ran) => return Ok(ran),
            Mode::Resident { sleep } => thread::sleep(sleep),
            Mode::Inspect { .. } => unreachable!("inspect does not cycle"),
        }
    }
}

fn code(ran: &Ran) -> u8 {
    match ran {
        Ran::Ready { compact, gc } => match (compact, gc) {
            (
                Compact::Quiet | Compact::Published(Published::Replaced),
                Gc::Swept(_) | Gc::NothingEligible,
            ) => 0,
            (Compact::Published(Published::Kept { .. } | Published::Refused(_)), _)
            | (_, Gc::Refused(_)) => 1,
        },
        Ran::RefreshRefused(_) => 1,
    }
}

fn ready(ran: &Ran) -> bool {
    code(ran) == 0
}

fn scream(ran: &Ran) -> String {
    let mut out = String::new();
    match ran {
        Ran::Ready { compact, gc } => {
            match compact {
                Compact::Quiet | Compact::Published(Published::Replaced) => {}
                Compact::Published(Published::Kept { incumbent }) => {
                    writeln!(out, "duty kept: incumbent {}", hex32(incumbent)).expect("scream");
                }
                Compact::Published(Published::Refused(refusal)) => {
                    writeln!(out, "duty refused: publish {refusal:?}").expect("scream");
                }
            }
            if let Gc::Refused(refusal) = gc {
                writeln!(out, "duty refused: gc {refusal:?}").expect("scream");
            }
        }
        Ran::RefreshRefused(refusal) => {
            writeln!(out, "duty refused: {refusal:?}").expect("scream");
        }
    }
    out
}

fn open_s3(
    bucket: &str,
    region: &str,
    endpoint: Option<&str>,
    key_prefix: &str,
) -> Result<S3Store, Error> {
    let access_key_id = env::var("AWS_ACCESS_KEY_ID").map_err(|_| Error::Credentials)?;
    let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| Error::Credentials)?;
    if access_key_id.is_empty() || secret_access_key.is_empty() {
        return Err(Error::Credentials);
    }
    S3Store::new(&S3Config {
        endpoint: endpoint.map(str::to_string),
        region: region.to_string(),
        bucket: bucket.to_string(),
        credentials: S3Credentials::Static {
            access_key_id,
            secret_access_key,
            session_token: env::var("AWS_SESSION_TOKEN").ok().filter(|t| !t.is_empty()),
        },
        prefix: key_prefix.to_string(),
    })
    .map_err(Error::Store)
}

fn atom(raw: String) -> Result<Atom, ConfigError> {
    match raw.strip_prefix("--") {
        Some(rest) if !rest.is_empty() => match rest.split_once('=') {
            Some((name, value)) if !name.is_empty() => Ok(Atom::Bound {
                name: Flag::parse(name)?,
                value: value.to_string(),
            }),
            _ => Ok(Atom::Flag(Flag::parse(rest)?)),
        },
        _ => Ok(Atom::Bare(raw)),
    }
}

fn take_value(
    flag: Flag,
    args: &mut std::iter::Peekable<impl Iterator<Item = Atom>>,
) -> Result<String, ConfigError> {
    match args.peek() {
        Some(Atom::Bare(_)) => match args.next() {
            Some(Atom::Bare(value)) => Ok(value),
            _ => unreachable!("peeked a bare atom"),
        },
        Some(Atom::Flag(_) | Atom::Bound { .. }) | None => {
            Err(ConfigError::MissingValue(flag.name()))
        }
    }
}

fn s3_defaults() -> (Option<String>, String, Option<String>, String) {
    (None, "us-east-1".to_string(), None, String::new())
}

impl Draft {
    fn store(self, kind: StoreKind) -> Result<Self, ConfigError> {
        match (self, kind) {
            (Self::Empty, StoreKind::Fs) => Ok(Self::Fs { root: None }),
            (Self::FsHint { root } | Self::Fs { root }, StoreKind::Fs) => Ok(Self::Fs { root }),
            (Self::Empty, StoreKind::S3) => {
                let (bucket, region, endpoint, key_prefix) = s3_defaults();
                Ok(Self::S3 {
                    bucket,
                    region,
                    endpoint,
                    key_prefix,
                })
            }
            (
                Self::S3Hint {
                    bucket,
                    region,
                    endpoint,
                    key_prefix,
                }
                | Self::S3 {
                    bucket,
                    region,
                    endpoint,
                    key_prefix,
                },
                StoreKind::S3,
            ) => Ok(Self::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            }),
            (Self::FsHint { .. } | Self::Fs { .. }, StoreKind::S3) => Err(ConfigError::Cross {
                flag: Flag::Store,
                backend: StoreKind::Fs,
            }),
            (Self::S3Hint { .. } | Self::S3 { .. }, StoreKind::Fs) => Err(ConfigError::Cross {
                flag: Flag::Store,
                backend: StoreKind::S3,
            }),
        }
    }

    fn root(self, root: PathBuf) -> Result<Self, ConfigError> {
        match self {
            Self::Empty | Self::FsHint { .. } => Ok(Self::FsHint { root: Some(root) }),
            Self::Fs { .. } => Ok(Self::Fs { root: Some(root) }),
            Self::S3Hint { .. } | Self::S3 { .. } => Err(ConfigError::Cross {
                flag: Flag::Root,
                backend: StoreKind::S3,
            }),
        }
    }

    fn s3_field(self, flag: Flag, value: String) -> Result<Self, ConfigError> {
        let (mut bucket, mut region, mut endpoint, mut key_prefix, named) = match self {
            Self::Empty => {
                let (bucket, region, endpoint, key_prefix) = s3_defaults();
                (bucket, region, endpoint, key_prefix, false)
            }
            Self::S3Hint {
                bucket,
                region,
                endpoint,
                key_prefix,
            } => (bucket, region, endpoint, key_prefix, false),
            Self::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            } => (bucket, region, endpoint, key_prefix, true),
            Self::FsHint { .. } | Self::Fs { .. } => {
                return Err(ConfigError::Cross {
                    flag,
                    backend: StoreKind::Fs,
                });
            }
        };
        match flag {
            Flag::Bucket => bucket = Some(value),
            Flag::Region => region = value,
            Flag::Endpoint => endpoint = Some(value),
            Flag::S3Prefix => key_prefix = value,
            Flag::Once
            | Flag::Inspect
            | Flag::Dir
            | Flag::Prefix
            | Flag::Theory
            | Flag::Writer
            | Flag::SleepMs
            | Flag::Store
            | Flag::Root => unreachable!("s3_field is the s3-only flags"),
        }
        Ok(if named {
            Self::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            }
        } else {
            Self::S3Hint {
                bucket,
                region,
                endpoint,
                key_prefix,
            }
        })
    }

    fn finish(self) -> Result<Backend, ConfigError> {
        match self {
            Self::Empty | Self::FsHint { .. } | Self::S3Hint { .. } => {
                Err(ConfigError::Missing("store"))
            }
            Self::Fs { root } => Ok(Backend::Fs {
                root: root.ok_or(ConfigError::Missing("root"))?,
            }),
            Self::S3 {
                bucket,
                region,
                endpoint,
                key_prefix,
            } => Ok(Backend::S3 {
                bucket: bucket.ok_or(ConfigError::Missing("bucket"))?,
                region,
                endpoint,
                key_prefix,
            }),
        }
    }
}

impl ModeDraft {
    fn once(self) -> Result<Self, ConfigError> {
        match self {
            Self::Default | Self::Once => Ok(Self::Once),
            Self::Resident { .. } => Err(ConfigError::OnceSleep),
            Self::Inspect { .. } => Err(ConfigError::InspectDuty),
        }
    }

    fn inspect(self, key: String) -> Result<Self, ConfigError> {
        match self {
            Self::Default | Self::Inspect { .. } => Ok(Self::Inspect { key }),
            Self::Once | Self::Resident { .. } => Err(ConfigError::InspectDuty),
        }
    }

    fn sleep(self, sleep_ms: u64) -> Result<Self, ConfigError> {
        match self {
            Self::Default | Self::Resident { .. } => Ok(Self::Resident { sleep_ms }),
            Self::Once => Err(ConfigError::OnceSleep),
            Self::Inspect { .. } => Err(ConfigError::InspectDuty),
        }
    }

    fn finish(self) -> Mode {
        match self {
            Self::Once => Mode::Once,
            Self::Default => Mode::Resident {
                sleep: Duration::from_millis(SLEEP_MS),
            },
            Self::Resident { sleep_ms } => Mode::Resident {
                sleep: Duration::from_millis(sleep_ms),
            },
            Self::Inspect { key } => Mode::Inspect { key },
        }
    }
}

struct Parse {
    mode: ModeDraft,
    dir: Option<PathBuf>,
    prefix: String,
    theory: Option<PathBuf>,
    writer_id: u64,
    draft: Draft,
}

impl Parse {
    fn bind(&mut self, flag: Flag, value: String) -> Result<(), ConfigError> {
        match flag {
            Flag::Once => unreachable!("once is the valueless arm"),
            Flag::Inspect => {
                self.mode = std::mem::replace(&mut self.mode, ModeDraft::Default).inspect(value)?;
            }
            Flag::Dir => self.dir = Some(PathBuf::from(value)),
            Flag::Prefix => self.prefix = value,
            Flag::Theory => self.theory = Some(PathBuf::from(value)),
            Flag::Writer => {
                self.writer_id = value.parse().map_err(|_| ConfigError::BadInt("writer"))?;
            }
            Flag::SleepMs => {
                let sleep_ms = value.parse().map_err(|_| ConfigError::BadInt("sleep-ms"))?;
                self.mode =
                    std::mem::replace(&mut self.mode, ModeDraft::Default).sleep(sleep_ms)?;
            }
            Flag::Store => {
                self.draft = std::mem::replace(&mut self.draft, Draft::Empty)
                    .store(StoreKind::parse(&value)?)?;
            }
            Flag::Root => {
                self.draft =
                    std::mem::replace(&mut self.draft, Draft::Empty).root(PathBuf::from(value))?;
            }
            Flag::Bucket | Flag::Region | Flag::Endpoint | Flag::S3Prefix => {
                self.draft =
                    std::mem::replace(&mut self.draft, Draft::Empty).s3_field(flag, value)?;
            }
        }
        Ok(())
    }
}

impl Config {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, ConfigError> {
        let mut parse = Parse {
            mode: ModeDraft::Default,
            dir: None,
            prefix: String::new(),
            theory: None,
            writer_id: 0,
            draft: Draft::Empty,
        };
        let tokens: Vec<Atom> = args.map(atom).collect::<Result<_, _>>()?;
        let mut args = tokens.into_iter().peekable();
        while let Some(next) = args.next() {
            match next {
                Atom::Flag(Flag::Once) => {
                    parse.mode = parse.mode.once()?;
                }
                Atom::Bound {
                    name: Flag::Once, ..
                } => return Err(ConfigError::Unknown("--once".into())),
                Atom::Flag(flag) => {
                    let value = take_value(flag, &mut args)?;
                    parse.bind(flag, value)?;
                }
                Atom::Bound { name, value } => {
                    if value.is_empty() {
                        return Err(ConfigError::MissingValue(name.name()));
                    }
                    parse.bind(name, value)?;
                }
                Atom::Bare(raw) if raw == "inspect" => {
                    let value = take_value(Flag::Inspect, &mut args)?;
                    parse.mode = parse.mode.inspect(value)?;
                }
                Atom::Bare(raw) => return Err(ConfigError::Unknown(raw)),
            }
        }
        Ok(Self {
            mode: parse.mode.finish(),
            dir: parse.dir.ok_or(ConfigError::Missing("dir"))?,
            prefix: parse.prefix,
            theory: parse.theory.ok_or(ConfigError::Missing("theory"))?,
            writer_id: parse.writer_id,
            backend: parse.draft.finish()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Atom, Backend, Config, ConfigError, Flag, Mode, StoreKind, atom, code, ready, scream,
    };
    use bumbledb_log::checkpointer::{Compact, Ran};
    use bumbledb_log::gc::{Gc, GcRefusal};
    use bumbledb_log::manifest::{PublishRefusal, Published, hex32};
    use bumbledb_log::replica::OpenRefusal;
    use std::time::Duration;

    fn argv(args: &[&str]) -> impl Iterator<Item = String> {
        args.iter().map(|s| (*s).to_string())
    }

    fn parse_err(args: &[&str]) -> ConfigError {
        Config::parse(argv(args)).expect_err("refused")
    }

    #[test]
    fn a_flag_is_not_another_flag_s_value() {
        assert_eq!(
            parse_err(&["--dir", "--theory", "/tmp/x"]),
            ConfigError::MissingValue("dir")
        );
    }

    #[test]
    fn store_then_once_is_missing_the_store_value() {
        assert_eq!(
            parse_err(&["--store", "--once"]),
            ConfigError::MissingValue("store")
        );
    }

    #[test]
    fn equals_binds_the_value() {
        let cfg = Config::parse(argv(&[
            "--once",
            "--store=fs",
            "--root=/tmp/r",
            "--dir=/tmp/d",
            "--theory=/tmp/t",
        ]))
        .expect("parse");
        assert_eq!(cfg.mode, Mode::Once);
        assert!(matches!(cfg.backend, Backend::Fs { .. }));
    }

    #[test]
    fn resident_carries_the_sleep() {
        let cfg = Config::parse(argv(&[
            "--sleep-ms=100",
            "--store=fs",
            "--root=/tmp/r",
            "--dir=/tmp/d",
            "--theory=/tmp/t",
        ]))
        .expect("parse");
        assert_eq!(
            cfg.mode,
            Mode::Resident {
                sleep: Duration::from_millis(100)
            }
        );
    }

    #[test]
    fn once_and_sleep_are_unrepresentable() {
        assert_eq!(
            parse_err(&[
                "--once",
                "--sleep-ms=100",
                "--store=fs",
                "--root=/tmp/r",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::OnceSleep
        );
        assert_eq!(
            parse_err(&[
                "--sleep-ms=100",
                "--once",
                "--store=fs",
                "--root=/tmp/r",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::OnceSleep
        );
    }

    #[test]
    fn a_cross_backend_flag_is_refused() {
        assert_eq!(
            parse_err(&[
                "--store=fs",
                "--root=/tmp/r",
                "--bucket=b",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::Cross {
                flag: Flag::Bucket,
                backend: StoreKind::Fs,
            }
        );
        assert_eq!(
            parse_err(&[
                "--store=s3",
                "--bucket=b",
                "--root=/tmp/r",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::Cross {
                flag: Flag::Root,
                backend: StoreKind::S3,
            }
        );
    }

    #[test]
    fn inspect_is_a_mode_arm() {
        let cfg = Config::parse(argv(&[
            "inspect",
            "manifest",
            "--store=fs",
            "--root=/tmp/r",
            "--dir=/tmp/d",
            "--theory=/tmp/t",
        ]))
        .expect("parse");
        assert_eq!(
            cfg.mode,
            Mode::Inspect {
                key: "manifest".into()
            }
        );
        let cfg = Config::parse(argv(&[
            "--inspect=ckpt/aa",
            "--store=fs",
            "--root=/tmp/r",
            "--dir=/tmp/d",
            "--theory=/tmp/t",
        ]))
        .expect("parse");
        assert_eq!(
            cfg.mode,
            Mode::Inspect {
                key: "ckpt/aa".into()
            }
        );
    }

    #[test]
    fn inspect_excludes_once_and_sleep() {
        assert_eq!(
            parse_err(&[
                "inspect",
                "manifest",
                "--once",
                "--store=fs",
                "--root=/tmp/r",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::InspectDuty
        );
        assert_eq!(
            parse_err(&[
                "--sleep-ms=100",
                "inspect",
                "chain",
                "--store=fs",
                "--root=/tmp/r",
                "--dir=/tmp/d",
                "--theory=/tmp/t",
            ]),
            ConfigError::InspectDuty
        );
    }

    #[test]
    fn inspect_needs_a_key() {
        assert_eq!(
            parse_err(&["inspect", "--store=fs"]),
            ConfigError::MissingValue("inspect")
        );
    }

    #[test]
    fn an_unknown_atom_is_refused() {
        assert_eq!(
            parse_err(&["--once", "--nope"]),
            ConfigError::Unknown("--nope".into())
        );
        assert!(matches!(
            atom("--nope".into()),
            Err(ConfigError::Unknown(flag)) if flag == "--nope"
        ));
        assert!(matches!(
            atom("--store=fs".into()),
            Ok(Atom::Bound {
                name: Flag::Store,
                value
            }) if value == "fs"
        ));
    }

    #[test]
    fn ready_quiet_exits_zero() {
        let ran = Ran::Ready {
            compact: Compact::Quiet,
            gc: Gc::NothingEligible,
        };
        assert!(ready(&ran));
        assert_eq!(code(&ran), 0);
        assert_eq!(scream(&ran), "");
    }

    #[test]
    fn kept_and_refused_exit_one() {
        let incumbent = [0u8; 32];
        let kept = Ran::Ready {
            compact: Compact::Published(Published::Kept { incumbent }),
            gc: Gc::NothingEligible,
        };
        let publish = Ran::Ready {
            compact: Compact::Published(Published::Refused(PublishRefusal::ManifestMissing)),
            gc: Gc::NothingEligible,
        };
        let gc = Ran::Ready {
            compact: Compact::Quiet,
            gc: Gc::Refused(GcRefusal::ManifestMissing),
        };
        let refresh = Ran::RefreshRefused(OpenRefusal::ManifestMissing);
        for ran in [&kept, &publish, &gc, &refresh] {
            assert!(!ready(ran));
            assert_eq!(code(ran), 1);
        }
        assert_eq!(
            scream(&kept),
            format!("duty kept: incumbent {}\n", hex32(&incumbent))
        );
        assert_eq!(scream(&publish), "duty refused: publish ManifestMissing\n");
        assert_eq!(scream(&gc), "duty refused: gc ManifestMissing\n");
        assert_eq!(scream(&refresh), "duty refused: ManifestMissing\n");
    }
}
