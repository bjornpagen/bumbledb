//! The duty maintenance CLI over a real `FsStore` layout: status, GC, roots,
//! backup, verify-backup, erase and the explicit finite argument grammar
//! (OPS-TEST-01 shape at the CLI boundary). The binary is an adapter over
//! the same library implementation, not a second machine. Verification:
//! `NotRun` (F1 authors, does not execute).

mod lane_support;

use std::path::Path;
use std::process::Command;

use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::store::fs::FsStore;
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, temp_dir, work};

fn duty(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_duty"))
        .args(args)
        .env_remove("AWS_ACCESS_KEY_ID")
        .env_remove("AWS_SECRET_ACCESS_KEY")
        .output()
        .expect("duty runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn fs_args<'a>(root: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut args = rest.to_vec();
    args.extend_from_slice(&["--fs-root", root, "--prefix", "t"]);
    args
}

/// A real FsStore-hosted tenant with two decisions and a checkpoint.
fn fixture(tag: &str) -> (std::path::PathBuf, FsStore) {
    let root = temp_dir(tag);
    let store = FsStore::new(&root);
    let mut mirror = Mirror::create(tag, &store, "t");
    let identity = mirror.identity;
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    mirror.submit(&insert_user(mirror.db(), identity, 2, 20));
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &CheckpointPolicy {
            chunk_bytes: 4_096,
            head_cap: HEAD_CAP,
            ..CheckpointPolicy::DEFAULT
        },
        &work(),
    )
    .expect("checkpoint");
    (root, store)
}

fn root_str(root: &Path) -> &str {
    root.to_str().expect("utf-8 test path")
}

#[test]
fn unknown_commands_and_arguments_refuse_with_usage_and_do_nothing() {
    let root = temp_dir("duty-grammar");
    let (ok, _, err) = duty(&["frobnicate", "--fs-root", root_str(&root), "--prefix", "t"]);
    assert!(!ok);
    assert!(err.contains("usage:"), "{err}");
    let (ok, _, err) = duty(&fs_args(root_str(&root), &["status", "--bogus-flag"]));
    assert!(!ok);
    assert!(err.contains("unknown argument"), "{err}");
    let (ok, _, err) = duty(&["gc", "--fs-root", root_str(&root), "--prefix", "t"]);
    assert!(!ok, "gc without --op refuses");
    assert!(err.contains("--op"), "{err}");
    // A malformed operation ID refuses before any backend work.
    let (ok, _, err) = duty(&fs_args(root_str(&root), &["gc", "--op", "not-hex"]));
    assert!(!ok);
    assert!(err.contains("32 hex characters"), "{err}");
}

#[test]
fn status_renders_the_bounded_redacted_report() {
    let (root, _store) = fixture("duty-status");
    let (ok, out, err) = duty(&fs_args(root_str(&root), &["status"]));
    assert!(ok, "{err}");
    assert!(out.contains("condition:"), "{out}");
    assert!(out.contains("head-revision:"), "{out}");
    assert!(out.contains("roots: 0 held"), "{out}");
    // Redaction: no credentials vocabulary, no fact payload spellings.
    for banned in ["AKIA", "secret_access", "10", "20"] {
        assert!(
            !out.lines()
                .any(|line| line.split(':').nth(1).is_some_and(|v| v.trim() == banned)),
            "{banned} must not appear as a value: {out}"
        );
    }
    // A missing database is reported, never created.
    let empty = temp_dir("duty-status-empty");
    let (ok, out, _) = duty(&fs_args(root_str(&empty), &["status"]));
    assert!(ok);
    assert!(
        out.contains("Missing"),
        "a missing head is definite absence: {out}"
    );
}

#[test]
fn gc_roots_backup_verify_and_erase_arms_run_the_real_operations() {
    let (root, _store) = fixture("duty-ops");
    let root_text = root_str(&root).to_string();
    let op_hex = "000000000000000000000000000000aa";
    let root_id = "000000000000000000000000000000bb";
    // root-add / root-release.
    let (ok, out, err) = duty(&fs_args(
        &root_text,
        &[
            "root-add",
            "--root-id",
            root_id,
            "--op",
            op_hex,
            "--label",
            "pin",
        ],
    ));
    assert!(ok, "{err}");
    assert!(out.contains("root added"), "{out}");
    // gc runs a full pass with the pin held.
    let (ok, out, err) = duty(&fs_args(&root_text, &["gc", "--op", op_hex]));
    assert!(ok, "{err}");
    assert!(out.contains("finished true"), "{out}");
    // backup into a separate destination; verify from the destination only.
    let vault = temp_dir("duty-vault");
    let vault_text = root_str(&vault).to_string();
    let (ok, out, err) = duty(&fs_args(
        &root_text,
        &[
            "backup",
            "--op",
            op_hex,
            "--dest-fs-root",
            &vault_text,
            "--dest-prefix",
            "vault",
        ],
    ));
    assert!(ok, "{err}");
    assert!(out.contains("backup complete"), "{out}");
    let (ok, out, err) = duty(&fs_args(
        &root_text,
        &[
            "verify-backup",
            "--op",
            op_hex,
            "--dest-fs-root",
            &vault_text,
            "--dest-prefix",
            "vault",
        ],
    ));
    assert!(ok, "{err}");
    assert!(out.contains("backup verified"), "{out}");
    // The backup retry is idempotent evidence.
    let (ok, out, err) = duty(&fs_args(
        &root_text,
        &[
            "backup",
            "--op",
            op_hex,
            "--dest-fs-root",
            &vault_text,
            "--dest-prefix",
            "vault",
        ],
    ));
    assert!(ok, "{err}");
    assert!(out.contains("already complete"), "{out}");
    // root-release, then erase; the residual report is honest.
    let (ok, out, err) = duty(&fs_args(
        &root_text,
        &["root-release", "--root-id", root_id],
    ));
    assert!(ok, "{err}");
    assert!(out.contains("released root"), "{out}");
    let erase_op = "000000000000000000000000000000cc";
    let (ok, out, err) = duty(&fs_args(&root_text, &["erase", "--op", erase_op]));
    assert!(ok, "{err}");
    assert!(out.contains("tombstone retained true"), "{out}");
    assert!(
        out.contains("backups/exports/blobs/keys untouched"),
        "{out}"
    );
    // After erasure, status reports the tombstone; the backup still verifies
    // from its independent destination.
    let (ok, out, err) = duty(&fs_args(&root_text, &["status"]));
    assert!(ok, "{err}");
    assert!(out.contains("Deleted"), "{out}");
    let (ok, _, err) = duty(&fs_args(
        &root_text,
        &[
            "verify-backup",
            "--op",
            op_hex,
            "--dest-fs-root",
            &vault_text,
            "--dest-prefix",
            "vault",
        ],
    ));
    assert!(ok, "erasure never owns the backup namespace: {err}");
}
