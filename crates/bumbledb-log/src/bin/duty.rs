//! The maintenance duty CLI: schema-free administrative operations over one
//! hosted backend — status, GC, revision fencing, receipt-epoch rotation,
//! named roots, independent backup/verification and erasure.
//!
//! Operations that require the application schema (checkpoint capture,
//! restore, migration) are NOT here: they run through the TypeScript log
//! SDK's native runtime, which owns the schema. This binary is an adapter
//! over the same library implementation, not a second machine. It parses an
//! explicit finite grammar, refuses unknown arguments, and prints bounded
//! redacted reports (no credentials, no fact payloads).
//!
//! The 0.x duty (braid checkpoint cadence/compaction/retention sweeps over
//! the five-verb store) is deleted with those representations.

use std::process::ExitCode;
use std::time::Duration;

use bumbledb::{ExecutionPolicy, Id128, WorkContext};

use bumbledb_log::backup::{backup_root, read_backup_manifest, verify_backup};
use bumbledb_log::checkpointer::read_live_head;
use bumbledb_log::erase::{erase_hosted, residual_report};
use bumbledb_log::gc::{GcPolicy, run_collection};
use bumbledb_log::history::command::Limits;
use bumbledb_log::history::{OperationId, ReceiptEpoch};
use bumbledb_log::inspect::{render, status_hosted};
use bumbledb_log::manifest::{RootKind, RootPolicy};
use bumbledb_log::store::fs::{FsError, FsStore};
use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Error, S3Store};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore, HeadVersion, ListPage, ObservedError, PutOutcome,
    ReceivedBody, ReceivedHead, ReceivingStore, TransportContext, TransportObservation,
};
use bumbledb_log::certainty::AdminCertainty;
use bumbledb_log::{admin, codec};

const HEAD_CAP: usize = 1024 * 1024;

const LIMITS: Limits = Limits {
    envelope_bytes: 16 * 1024 * 1024,
    change_bytes: 8 * 1024 * 1024,
    evidence_bytes: 64 * 1024,
    result_bytes: 64 * 1024,
};

fn work() -> Result<WorkContext, String> {
    ExecutionPolicy {
        input_bytes: 1 << 32,
        working_bytes: 1 << 32,
        scratch_bytes: 1 << 34,
        result_bytes: 1 << 32,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: Duration::from_hours(24),
    }
    .start()
    .map_err(|error| format!("work budget: {error:?}"))
}

/// One backend value over the two supported drivers. Production reads
/// are only [`ReceivingStore::receive_object`] / [`ReceivingStore::receive_head`]
/// (`ReceivedBody` / `ReceivedHead`). Deleted `get_object` / `read_head`
/// and the uncharged three-argument `get_verified` are not re-introduced.
/// Library verbs take this type — no driver-unwrap macros, no second store API.
enum AnyStore {
    Fs(FsStore),
    S3(Box<S3Store>),
}

/// Unified adapter fault so [`AnyStore`] is a [`ReceivingStore`].
#[derive(Debug)]
enum DutyFault {
    Fs(FsError),
    S3(S3Error),
}

impl ObservedError for DutyFault {
    fn observation(&self) -> TransportObservation {
        match self {
            Self::Fs(error) => error.observation(),
            Self::S3(error) => error.observation(),
        }
    }
}

impl std::fmt::Display for DutyFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fs(error) => error.fmt(f),
            Self::S3(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DutyFault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => std::error::Error::source(error),
            Self::S3(error) => std::error::Error::source(error),
        }
    }
}

impl ConditionalStore for AnyStore {
    type Error = DutyFault;

    fn create_head(&self, head_key: &str, body: &[u8]) -> Result<ConditionalOutcome, DutyFault> {
        match self {
            Self::Fs(store) => store.create_head(head_key, body).map_err(DutyFault::Fs),
            Self::S3(store) => store.create_head(head_key, body).map_err(DutyFault::S3),
        }
    }

    fn replace_head(
        &self,
        head_key: &str,
        expected: &HeadVersion,
        body: &[u8],
    ) -> Result<ConditionalOutcome, DutyFault> {
        match self {
            Self::Fs(store) => store
                .replace_head(head_key, expected, body)
                .map_err(DutyFault::Fs),
            Self::S3(store) => store
                .replace_head(head_key, expected, body)
                .map_err(DutyFault::S3),
        }
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<PutOutcome, DutyFault> {
        match self {
            Self::Fs(store) => store.put_object(key, body).map_err(DutyFault::Fs),
            Self::S3(store) => store.put_object(key, body).map_err(DutyFault::S3),
        }
    }

    fn list_objects(&self, prefix: &str, after: Option<&[u8]>) -> Result<ListPage, DutyFault> {
        match self {
            Self::Fs(store) => store.list_objects(prefix, after).map_err(DutyFault::Fs),
            Self::S3(store) => store.list_objects(prefix, after).map_err(DutyFault::S3),
        }
    }

    fn delete_object(&self, key: &str) -> Result<(), DutyFault> {
        match self {
            Self::Fs(store) => store.delete_object(key).map_err(DutyFault::Fs),
            Self::S3(store) => store.delete_object(key).map_err(DutyFault::S3),
        }
    }
}

impl ReceivingStore for AnyStore {
    fn receive_object(
        &self,
        key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedBody, DutyFault> {
        match self {
            Self::Fs(store) => store.receive_object(key, ctx).map_err(DutyFault::Fs),
            Self::S3(store) => store.receive_object(key, ctx).map_err(DutyFault::S3),
        }
    }

    fn receive_head(
        &self,
        head_key: &str,
        ctx: TransportContext<'_>,
    ) -> Result<ReceivedHead, DutyFault> {
        match self {
            Self::Fs(store) => store.receive_head(head_key, ctx).map_err(DutyFault::Fs),
            Self::S3(store) => store.receive_head(head_key, ctx).map_err(DutyFault::S3),
        }
    }
}

fn duty_admin<T>(certainty: AdminCertainty<T>, op: &str) -> Result<T, String> {
    match certainty {
        AdminCertainty::Completed { value } => Ok(value),
        AdminCertainty::NotStarted { error } => Err(format!("{op}: not started: {error:?}")),
        AdminCertainty::OutcomeUnknown { error } => {
            Err(format!("{op}: outcome unknown: {error:?}"))
        }
    }
}

/// Explicit parsed arguments: `--name value` pairs only, no positional
/// arguments after the command, unknown names refuse.
struct Args {
    command: String,
    values: Vec<(String, String)>,
    flags: Vec<String>,
}

const VALUE_NAMES: [&str; 12] = [
    "--fs-root",
    "--s3-bucket",
    "--s3-region",
    "--s3-endpoint",
    "--prefix",
    "--op",
    "--root-id",
    "--label",
    "--epoch",
    "--dest-fs-root",
    "--dest-prefix",
    "--release-root",
];
const FLAG_NAMES: [&str; 1] = ["--hold"];

fn parse_args(mut argv: std::env::Args) -> Result<Args, String> {
    let _program = argv.next();
    let command = argv.next().ok_or_else(usage)?;
    let mut values = Vec::new();
    let mut flags = Vec::new();
    while let Some(name) = argv.next() {
        if FLAG_NAMES.contains(&name.as_str()) {
            flags.push(name);
            continue;
        }
        if !VALUE_NAMES.contains(&name.as_str()) {
            return Err(format!("unknown argument `{name}`\n{}", usage()));
        }
        let value = argv
            .next()
            .ok_or_else(|| format!("`{name}` requires a value"))?;
        values.push((name, value));
    }
    Ok(Args {
        command,
        values,
        flags,
    })
}

impl Args {
    fn get(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| value.as_str())
    }

    fn require(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("`{name}` is required"))
    }

    fn all(&self, name: &str) -> Vec<&str> {
        self.values
            .iter()
            .filter(|(held, _)| held == name)
            .map(|(_, value)| value.as_str())
            .collect()
    }
}

fn usage() -> String {
    "usage: duty <status|gc|fence|rotate-receipts|root-add|root-release|backup|verify-backup|erase|residual>\n\
     backend: --fs-root PATH | --s3-bucket B --s3-region R [--s3-endpoint URL] (credentials from AWS_* env)\n\
     common: --prefix P\n\
     gc/erase/backup: --op HEX32; rotate-receipts: --epoch N\n\
     root-add: --root-id HEX32 --label TEXT [--hold] --op HEX32; root-release: --root-id HEX32\n\
     backup/verify-backup: --dest-fs-root PATH --dest-prefix P\n\
     erase: repeatable --release-root HEX32"
        .to_string()
}

fn parse_id128(hex: &str) -> Result<Id128, String> {
    if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("`{hex}` is not 32 hex characters"));
    }
    let mut bytes = [0u8; 16];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk).expect("hex chunk");
        bytes[index] = u8::from_str_radix(text, 16).map_err(|error| error.to_string())?;
    }
    Ok(Id128::from_bytes(bytes))
}

fn operation(args: &Args, name: &str) -> Result<OperationId, String> {
    Ok(OperationId::from_core(parse_id128(args.require(name)?)?))
}

fn backend_of(args: &Args, root_name: &str, standard: bool) -> Result<AnyStore, String> {
    if let Some(root) = args.get(root_name) {
        return Ok(AnyStore::Fs(FsStore::new(root)));
    }
    if !standard {
        return Err(format!("`{root_name}` is required"));
    }
    let bucket = args.require("--s3-bucket")?;
    let region = args.require("--s3-region")?;
    let access_key_id =
        std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| "AWS_ACCESS_KEY_ID is not set")?;
    let secret_access_key =
        std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| "AWS_SECRET_ACCESS_KEY is not set")?;
    let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
    let store = S3Store::new(&S3Config {
        endpoint: args.get("--s3-endpoint").map(str::to_string),
        region: region.to_string(),
        bucket: bucket.to_string(),
        credentials: S3Credentials::Static {
            access_key_id,
            secret_access_key,
            session_token,
        },
    })
    .map_err(|error| error.to_string())?;
    Ok(AnyStore::S3(Box::new(store)))
}

#[expect(
    clippy::too_many_lines,
    reason = "one flat argv dispatch over the closed duty verb roster"
)]
fn run() -> Result<(), String> {
    let args = parse_args(std::env::args())?;
    let work = work()?;
    let prefix = args.require("--prefix")?.to_string();
    let gc_policy = GcPolicy::DEFAULT;
    match args.command.as_str() {
        "status" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let status = status_hosted(&backend, &prefix, None, HEAD_CAP, &work);
            print!("{}", render(&status));
            Ok(())
        }
        "gc" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let op = operation(&args, "--op")?;
            let report = run_collection(&backend, &prefix, op, LIMITS, &gc_policy, &work)
                .map_err(|error| format!("gc: {error:?}"))?;
            println!(
                "gc: deleted {}, retained {} marked / {} newer / {} unparsed, {} pages, finished {}",
                report.deleted,
                report.retained_marked,
                report.retained_newer,
                report.retained_unparsed,
                report.pages,
                report.finished
            );
            Ok(())
        }
        "fence" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let revision = duty_admin(
                admin::fence_revision_hosted(&backend, &prefix, HEAD_CAP, &work),
                "fence",
            )?;
            println!("fenced: head revision {}", revision.0);
            Ok(())
        }
        "rotate-receipts" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let raw: u64 = args
                .require("--epoch")?
                .parse()
                .map_err(|_| "`--epoch` must be a positive integer")?;
            let next = ReceiptEpoch::new(raw).ok_or("`--epoch` must be positive")?;
            let revision = duty_admin(
                admin::rotate_receipts_hosted(&backend, &prefix, next, HEAD_CAP, &work),
                "rotate",
            )?;
            println!("rotated: open epoch {raw}, head revision {}", revision.0);
            Ok(())
        }
        "root-add" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let root_id = operation(&args, "--root-id")?;
            let op = operation(&args, "--op")?;
            let label = args.require("--label")?;
            let kind = if args.flags.iter().any(|flag| flag == "--hold") {
                RootKind::HydrationHold
            } else {
                RootKind::RestorePoint
            };
            let root = duty_admin(
                admin::add_named_root_hosted(
                    &backend,
                    &prefix,
                    root_id,
                    kind,
                    label,
                    op,
                    &RootPolicy::DEFAULT,
                    HEAD_CAP,
                    &work,
                ),
                "root-add",
            )?;
            println!(
                "root added: base seq {}, tip seq {}, label {:?}",
                root.recovery.base.seq, root.recovery.tip.seq, root.label
            );
            Ok(())
        }
        "root-release" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let root_id = operation(&args, "--root-id")?;
            let released = duty_admin(
                admin::release_named_root_hosted(
                    &backend, &prefix, root_id, false, HEAD_CAP, &work,
                ),
                "root-release",
            )?;
            match released {
                Some(root) => println!(
                    "released root {:?}: recovery capability base {}..tip {} is no longer pinned",
                    root.label, root.recovery.base.seq, root.recovery.tip.seq
                ),
                None => println!("released: already absent"),
            }
            Ok(())
        }
        "backup" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let destination = backend_of(&args, "--dest-fs-root", false)?;
            let dest_prefix = args.require("--dest-prefix")?;
            let op = operation(&args, "--op")?;
            let (head, _) = read_live_head(&backend, &prefix, HEAD_CAP, &work)
                .map_err(|error| format!("backup: head: {error:?}"))?;
            let live = head
                .control
                .live()
                .map_err(|_| "backup: the authority is a tombstone")?;
            let recovery = head
                .recovery
                .ok_or("backup: live head without recovery root")?;
            let report = backup_root(
                &backend,
                &prefix,
                &destination,
                dest_prefix,
                head.control.identity,
                live.state,
                &recovery,
                head.object_epoch,
                op,
                LIMITS,
                codec::StreamLimits::DEFAULT,
                &work,
            )
            .map_err(|error| format!("backup: {error:?}"))?;
            println!(
                "backup {}: {} objects, {} bytes, manifest digest {}",
                if report.installed {
                    "complete"
                } else {
                    "already complete"
                },
                report.objects_copied,
                report.bytes_copied,
                bumbledb_log::store::hex32(&report.manifest_digest)
            );
            Ok(())
        }
        "verify-backup" => {
            let destination = backend_of(&args, "--dest-fs-root", false)?;
            let dest_prefix = args.require("--dest-prefix")?;
            let op = operation(&args, "--op")?;
            let (_, digest) = read_backup_manifest(&destination, dest_prefix, op, &work)
                .map_err(|error| format!("verify-backup: {error:?}"))?;
            let report = verify_backup(
                &destination,
                dest_prefix,
                op,
                LIMITS,
                codec::StreamLimits::DEFAULT,
                &work,
            )
            .map_err(|error| format!("verify-backup: {error:?}"))?;
            println!(
                "backup verified: {} objects, {} bytes, manifest digest {}",
                report.objects_verified,
                report.bytes_verified,
                bumbledb_log::store::hex32(&digest)
            );
            Ok(())
        }
        "erase" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let op = operation(&args, "--op")?;
            let mut release = Vec::new();
            for hex in args.all("--release-root") {
                release.push(OperationId::from_core(parse_id128(hex)?));
            }
            let report = erase_hosted(&backend, &prefix, op, &release, LIMITS, &gc_policy, &work)
                .map_err(|error| format!("erase: {error:?}"))?;
            println!(
                "erased: tombstone retained {}, {} objects deleted, {} objects remain, {} retained roots; backups/exports/blobs/keys untouched",
                report.residual.head_tombstone_retained,
                report.sweep.deleted,
                report.residual.remaining_objects,
                report.residual.retained_roots.len()
            );
            Ok(())
        }
        "residual" => {
            let backend = backend_of(&args, "--fs-root", true)?;
            let report = residual_report(&backend, &prefix, &gc_policy, &work)
                .map_err(|error| format!("residual: {error:?}"))?;
            println!(
                "residual: tombstone {}, {} objects remain, {} retained roots; backups/exports/blobs/keys are separate policy",
                report.head_tombstone_retained,
                report.remaining_objects,
                report.retained_roots.len()
            );
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n{}", usage())),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::work::Resource;
    use bumbledb_log::store::{ObjectKind, ReceiveLimits, get_verified, put_verified};

    fn scratch(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "bdb-duty-charged-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch");
        path
    }

    fn modest_work() -> WorkContext {
        ExecutionPolicy {
            input_bytes: 0,
            working_bytes: 1 << 20,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 0,
            work_units: 1_024,
            timeout: Duration::from_secs(5),
        }
        .start()
        .expect("work")
    }

    #[test]
    fn any_store_receive_object_and_receive_head_keep_charged_bytes_not_a_vec() {
        let root = scratch("receive");
        let store = AnyStore::Fs(FsStore::new(&root));
        let ctx = modest_work();
        let reference =
            put_verified(&store, "t", 1, ObjectKind::Chunk, b"duty-payload").expect("put");
        store
            .create_head("t/HEAD", b"duty-head")
            .expect("create_head");
        let transport = TransportContext::new(&ctx, ReceiveLimits::capped(1 << 20));
        let baseline = ctx.used(Resource::WorkingBytes);
        let body = get_verified(&store, "t", &reference, transport).expect("verified");
        assert!(
            ctx.used(Resource::WorkingBytes) > baseline,
            "duty AnyStore must not hand out an uncharged Vec"
        );
        assert_eq!(body.as_bytes(), b"duty-payload");
        let received = store
            .receive_object(&reference.key("t"), transport)
            .expect("receive_object");
        assert!(
            received.into_charged().is_some(),
            "receive_object keeps ChargedBytes when work is present"
        );
        match store
            .receive_head("t/HEAD", transport)
            .expect("receive_head")
        {
            ReceivedHead::Present { body: head, .. } => {
                assert_eq!(head.as_bytes(), b"duty-head");
                assert!(
                    head.into_charged().is_some(),
                    "receive_head keeps ChargedBytes when work is present"
                );
            }
            ReceivedHead::Absent => panic!("duty AnyStore receive_head must see the created head"),
        }
        drop(body);
        let _ = std::fs::remove_dir_all(&root);
    }
}
