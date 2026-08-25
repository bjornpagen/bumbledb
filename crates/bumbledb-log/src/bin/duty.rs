//! The duty binary: one body, two modes. `--once` is the Lambda arm;
//! the default is the resident sleep loop. Argv is a parsed grammar;
//! the exit code is a total function of [`Ran`].

use std::env;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use bumbledb::SchemaDescriptor;
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Compact, Ran};
use bumbledb_log::gc::Gc;
use bumbledb_log::manifest::{hex32, Published};
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

/// One argv atom. A value is bound in the same token (`--name=value`)
/// or is the next bare token — never another flag.
enum Atom {
    Flag(String),
    Bound { name: String, value: String },
    Bare(String),
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
            Self::Refused(refusal) => write!(f, "duty refused: {refusal:?}"),
            Self::Credentials => write!(f, "missing AWS_ACCESS_KEY_ID or AWS_SECRET_ACCESS_KEY"),
        }
    }
}

fn main() -> ExitCode {
    match start(env::args().skip(1)) {
        Ok(ran) => {
            scream(&ran);
            ExitCode::from(code(&ran))
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: impl Iterator<Item = String>) -> Result<Ran, Error> {
    let config = Config::parse(args).map_err(Error::Config)?;
    let theory = schema_file::load(&config.theory).map_err(Error::Theory)?;
    match &config.backend {
        Backend::Fs { root } => cycle(FsStore::new(root.clone()), &config, theory),
        Backend::S3 { .. } => cycle(open_s3(&config)?, &config, theory),
    }
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
        if config.once || !ready(&ran) {
            return Ok(ran);
        }
        thread::sleep(config.sleep);
    }
}

/// Exit status of one body. Total on [`Ran`]: 0 only for a successful
/// compact (`Quiet` / `Replaced`) and a successful sweep; `Kept` and
/// every `Refused` arm are 1.
fn code(ran: &Ran) -> u8 {
    match ran {
        Ran::Ready { compact, gc } => match (compact, gc) {
            (Compact::Quiet, Gc::Swept(_) | Gc::NothingEligible)
            | (Compact::Published(Published::Replaced), Gc::Swept(_) | Gc::NothingEligible) => 0,
            (Compact::Published(Published::Kept { .. }), _)
            | (Compact::Published(Published::Refused(_)), _)
            | (_, Gc::Refused(_)) => 1,
        },
        Ran::RefreshRefused(_) => 1,
    }
}

fn ready(ran: &Ran) -> bool {
    code(ran) == 0
}

fn scream(ran: &Ran) {
    match ran {
        Ran::Ready { compact, gc } => {
            match compact {
                Compact::Quiet | Compact::Published(Published::Replaced) => {}
                Compact::Published(Published::Kept { incumbent }) => {
                    eprintln!("duty kept: incumbent {}", hex32(incumbent));
                }
                Compact::Published(Published::Refused(refusal)) => {
                    eprintln!("duty refused: publish {refusal:?}");
                }
            }
            if let Gc::Refused(refusal) = gc {
                eprintln!("duty refused: gc {refusal:?}");
            }
        }
        Ran::RefreshRefused(refusal) => eprintln!("duty refused: {refusal:?}"),
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

fn atom(raw: String) -> Atom {
    match raw.strip_prefix("--") {
        Some(rest) if !rest.is_empty() => match rest.split_once('=') {
            Some((name, value)) if !name.is_empty() => Atom::Bound {
                name: name.to_string(),
                value: value.to_string(),
            },
            _ => Atom::Flag(rest.to_string()),
        },
        _ => Atom::Bare(raw),
    }
}

fn valued(name: &str) -> Result<&'static str, ConfigError> {
    Ok(match name {
        "dir" => "dir",
        "prefix" => "prefix",
        "theory" => "theory",
        "writer" => "writer",
        "sleep-ms" => "sleep-ms",
        "store" => "store",
        "root" => "root",
        "bucket" => "bucket",
        "region" => "region",
        "endpoint" => "endpoint",
        "s3-prefix" => "s3-prefix",
        other => return Err(ConfigError::Unknown(format!("--{other}"))),
    })
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
        let mut args = args.map(atom).peekable();
        while let Some(next) = args.next() {
            let (name, value) = match next {
                Atom::Flag(name) if name == "once" => {
                    once = true;
                    continue;
                }
                Atom::Bound { name, .. } if name == "once" => {
                    return Err(ConfigError::Unknown("--once".into()));
                }
                Atom::Flag(name) => {
                    let name = valued(&name)?;
                    let value = match args.peek() {
                        Some(Atom::Bare(_)) => match args.next() {
                            Some(Atom::Bare(value)) => value,
                            _ => unreachable!("peeked a bare atom"),
                        },
                        Some(Atom::Flag(_) | Atom::Bound { .. }) | None => {
                            return Err(ConfigError::MissingValue(name));
                        }
                    };
                    (name, value)
                }
                Atom::Bound { name, value } => {
                    let name = valued(&name)?;
                    if value.is_empty() {
                        return Err(ConfigError::MissingValue(name));
                    }
                    (name, value)
                }
                Atom::Bare(raw) => return Err(ConfigError::Unknown(raw)),
            };
            match name {
                "dir" => dir = Some(PathBuf::from(value)),
                "prefix" => prefix = value,
                "theory" => theory = Some(PathBuf::from(value)),
                "writer" => {
                    writer_id = value.parse().map_err(|_| ConfigError::BadInt("writer"))?;
                }
                "sleep-ms" => {
                    sleep_ms = value.parse().map_err(|_| ConfigError::BadInt("sleep-ms"))?;
                }
                "store" => store = Some(value),
                "root" => root = Some(PathBuf::from(value)),
                "bucket" => bucket = Some(value),
                "region" => region = value,
                "endpoint" => endpoint = Some(value),
                "s3-prefix" => key_prefix = value,
                _ => unreachable!("valued() only yields the arms above"),
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

#[cfg(test)]
mod tests {
    use super::{atom, code, ready, Atom, Config, ConfigError};
    use bumbledb_log::checkpointer::{Compact, Ran};
    use bumbledb_log::gc::{Gc, GcRefusal};
    use bumbledb_log::manifest::{PublishRefusal, Published};
    use bumbledb_log::replica::OpenRefusal;

    fn argv(args: &[&str]) -> impl Iterator<Item = String> {
        args.iter().map(|s| (*s).to_string())
    }

    fn parse_err(args: &[&str]) -> String {
        Config::parse(argv(args))
            .err()
            .expect("refused")
            .to_string()
    }

    #[test]
    fn a_flag_is_not_another_flag_s_value() {
        let err = parse_err(&["--dir", "--theory", "/tmp/x"]);
        assert!(err.contains("needs a value"), "{err}");
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
        assert!(cfg.once);
        assert!(matches!(cfg.backend, super::Backend::Fs { .. }));
    }

    #[test]
    fn an_unknown_atom_is_refused() {
        let err = parse_err(&["--once", "--nope"]);
        assert!(err.contains("unknown flag"), "{err}");
        assert!(matches!(atom("--nope".into()), Atom::Flag(name) if name == "nope"));
        assert!(matches!(
            Config::parse(argv(&["--once", "--nope"])).expect_err("unknown"),
            ConfigError::Unknown(_)
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
    }

    #[test]
    fn kept_and_refused_exit_one() {
        let kept = Ran::Ready {
            compact: Compact::Published(Published::Kept { incumbent: [0; 32] }),
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
    }
}
