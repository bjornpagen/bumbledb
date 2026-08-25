//! The duty body over `FsStore`: one cadence check, compact under the
//! checkpoint order, the retention sweep, and the same body through
//! the `duty` binary's `--once` arm.

mod lane_e_support;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use bumbledb::SchemaDescriptor;
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor as Desc,
    StatementDescriptor, ValueType,
};
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Compact, Ran};
use bumbledb_log::codec::Codec;
use bumbledb_log::gc::Gc;
use bumbledb_log::manifest::{
    Checkpoint, Head, Manifest, ckpt_json_key, hex32, manifest_key, publish_checkpoint,
};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{CHECKPOINT_EVERY_BYTES, Commit, Options, Writer, WriterOpened};
use lane_e_support::{NOTE, note_row, temp_dir, theory};

fn open_writer(root: &std::path::Path, dir: &std::path::Path) -> Writer<SchemaDescriptor, FsStore> {
    match Writer::open(
        FsStore::new(root.to_path_buf()),
        "",
        dir,
        theory(),
        Options::new(17),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_duty(
    root: &std::path::Path,
    dir: &std::path::Path,
) -> Checkpointer<SchemaDescriptor, FsStore> {
    match Checkpointer::open(FsStore::new(root.to_path_buf()), "", dir, theory(), 17)
        .expect("open checkpointer")
    {
        CheckpointerOpened::Ready(duty) => *duty,
        CheckpointerOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn commit_notes(writer: &Writer<SchemaDescriptor, FsStore>, count: u64) {
    for id in 0..count {
        assert!(matches!(
            writer
                .commit(|batch| {
                    batch.insert(NOTE, [note_row(id, "duty")]);
                    Ok(())
                })
                .expect("commit"),
            Commit::Accepted { .. }
        ));
    }
    writer.quiesce();
}

fn manifest_checkpoint(root: &std::path::Path) -> Option<[u8; 32]> {
    let store = FsStore::new(root.to_path_buf());
    let fetched = store
        .get(&manifest_key(""))
        .expect("get")
        .expect("manifest");
    Manifest::parse(&fetched.bytes)
        .expect("manifest parses")
        .checkpoint
}

#[test]
fn under_cadence_the_body_is_quiet() {
    let root = temp_dir("duty_quiet");
    let writer = open_writer(&root, &root.join("w"));
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    commit_notes(&writer, 1);
    drop(writer);

    let mut duty = open_duty(&root, &root.join("d"));
    duty.set_checkpoint_cadence(u64::MAX, u64::MAX);
    match duty.run().expect("run") {
        Ran::Ready {
            compact: Compact::Quiet,
            gc: Gc::NothingEligible,
        } => {}
        other => panic!("quiet expected, got {other:?}"),
    }
    assert!(manifest_checkpoint(&root).is_none());
}

#[test]
fn crossing_the_sum_cadence_publishes() {
    let root = temp_dir("duty_sum");
    let writer = open_writer(&root, &root.join("w"));
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    commit_notes(&writer, 2);
    drop(writer);

    let mut duty = open_duty(&root, &root.join("d"));
    duty.set_checkpoint_cadence(2, u64::MAX);
    match duty.run().expect("run") {
        Ran::Ready {
            compact: Compact::Published(_),
            gc,
        } => {
            assert!(matches!(gc, Gc::Swept(_) | Gc::NothingEligible));
        }
        other => panic!("publish expected, got {other:?}"),
    }
    assert!(manifest_checkpoint(&root).is_some());
}

#[test]
fn a_second_run_is_quiet_and_keeps_the_incumbent() {
    let root = temp_dir("duty_idem");
    let writer = open_writer(&root, &root.join("w"));
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    commit_notes(&writer, 2);
    drop(writer);

    let mut duty = open_duty(&root, &root.join("d"));
    duty.set_checkpoint_cadence(2, u64::MAX);
    duty.run().expect("first");
    let first = manifest_checkpoint(&root).expect("published");
    match duty.run().expect("second") {
        Ran::Ready {
            compact: Compact::Quiet,
            ..
        } => {}
        other => panic!("second run is quiet, got {other:?}"),
    }
    assert_eq!(manifest_checkpoint(&root), Some(first));
}

const NOTE_THEORY: &str = r#"{"relations":[{"name":"note","fields":[{"name":"id","type":"u64"},{"name":"body","type":"string"}]}],"statements":[{"functionality":{"relation":0,"projection":[0]}}]}"#;

fn note_only() -> Desc {
    Desc {
        relations: vec![RelationDescriptor {
            name: "note".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "body".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
            ],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
}

fn write_theory(root: &Path) -> std::path::PathBuf {
    let path = root.join("theory.json");
    std::fs::write(&path, NOTE_THEORY).expect("write theory");
    path
}

fn note_writer(root: &Path) -> Writer<Desc, FsStore> {
    match Writer::open(
        FsStore::new(root.to_path_buf()),
        "",
        &root.join("w"),
        note_only(),
        Options::new(17),
    )
    .expect("open")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("refused: {refusal:?}"),
    }
}

fn cross_the_byte_cadence(writer: &Writer<Desc, FsStore>) {
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    let body = "p".repeat(CHECKPOINT_EVERY_BYTES as usize);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(1, &body)]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
}

fn duty_cmd(root: &Path, theory_path: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_duty"));
    cmd.args([
        "--store",
        "fs",
        "--root",
        root.to_str().expect("utf8"),
        "--dir",
        root.join("d").to_str().expect("utf8"),
        "--theory",
        theory_path.to_str().expect("utf8"),
    ]);
    cmd
}

fn wait_exists(path: &Path, timeout: Duration) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < timeout,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn finish(mut child: std::process::Child, timeout: Duration) -> Output {
    let start = Instant::now();
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => return child.wait_with_output().expect("output"),
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                panic!("duty did not exit");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn note_codec() -> Codec {
    let descriptor = note_only();
    let schema = descriptor.clone().validate().expect("validates");
    Codec::new(&descriptor, schema_fingerprint(&schema).0)
}

fn scratch_dir(root: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.duty-ckpt", root.join("d").display()))
}

fn plant_high_incumbent(root: &Path) -> [u8; 32] {
    let codec = note_codec();
    let braids: std::collections::BTreeMap<_, _> = codec
        .braids()
        .components()
        .keys()
        .map(|braid| {
            (
                *braid,
                Head {
                    g: 10_000,
                    hash: [0x11; 32],
                    ts: 1,
                },
            )
        })
        .collect();
    let doc = Checkpoint {
        braids,
        catalog: [0x22; 32],
        writer: 99,
        prev: None,
    };
    let store = FsStore::new(root.to_path_buf());
    match publish_checkpoint(&store, "", codec.braids(), &doc).expect("plant") {
        bumbledb_log::manifest::Published::Replaced => {}
        other => panic!("plant must replace a null incumbent, got {other:?}"),
    }
    doc.digest()
}

fn overwrite_manifest(root: &Path, bytes: &[u8]) {
    std::fs::write(root.join("manifest.json"), bytes).expect("overwrite manifest");
}

fn spawn_once(root: &Path, theory_path: &Path) -> std::process::Child {
    duty_cmd(root, theory_path)
        .arg("--once")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn duty")
}

#[test]
fn the_once_binary_publishes_and_exits_zero() {
    let root = temp_dir("duty_bin_pub");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);

    let out = duty_cmd(&root, &theory_path)
        .arg("--once")
        .output()
        .expect("spawn duty");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "publish must exit 0: {stderr}");
    assert!(
        stderr.is_empty(),
        "Replaced + successful sweep is silent, got {stderr}"
    );
    assert!(
        manifest_checkpoint(&root).is_some(),
        "duty --once must publish a checkpoint"
    );
}

#[test]
fn the_once_binary_is_quiet_under_cadence() {
    let root = temp_dir("duty_bin_quiet");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(1, "bin")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
    drop(writer);

    let out = duty_cmd(&root, &theory_path)
        .arg("--once")
        .output()
        .expect("spawn duty");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "quiet must exit 0: {stderr}");
    assert!(stderr.is_empty(), "Quiet is silent, got {stderr}");
    assert!(
        manifest_checkpoint(&root).is_none(),
        "under-cadence --once must not publish"
    );
}

#[test]
fn the_once_binary_screams_kept() {
    let root = temp_dir("duty_bin_kept");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);

    let child = spawn_once(&root, &theory_path);
    wait_exists(&root.join("d"), Duration::from_secs(10));
    let incumbent = plant_high_incumbent(&root);
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "Kept is 1: {stderr}");
    assert!(
        stderr.contains(&format!("duty kept: incumbent {}", hex32(&incumbent))),
        "{stderr}"
    );
}

#[test]
fn the_once_binary_screams_publish_manifest_missing() {
    let root = temp_dir("duty_bin_pub_miss");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);

    let child = spawn_once(&root, &theory_path);
    wait_exists(&scratch_dir(&root), Duration::from_secs(30));
    FsStore::new(root.clone())
        .delete(&manifest_key(""))
        .expect("delete manifest");
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: publish ManifestMissing"),
        "{stderr}"
    );
}

#[test]
fn the_once_binary_screams_publish_manifest_parse() {
    let root = temp_dir("duty_bin_pub_mal");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);

    let child = spawn_once(&root, &theory_path);
    wait_exists(&scratch_dir(&root), Duration::from_secs(30));
    overwrite_manifest(&root, b"not-a-manifest");
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: publish Manifest("),
        "{stderr}"
    );
}

#[test]
fn the_once_binary_screams_publish_checkpoint_doc_missing() {
    let root = temp_dir("duty_bin_pub_ckpt_miss");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);
    let digest = [0xabu8; 32];
    let fingerprint = *note_codec().fingerprint();
    let child = spawn_once(&root, &theory_path);
    wait_exists(&scratch_dir(&root), Duration::from_secs(30));
    overwrite_manifest(
        &root,
        &Manifest {
            fingerprint,
            checkpoint: Some(digest),
        }
        .render(),
    );
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: publish CheckpointDocMissing"),
        "{stderr}"
    );
}

#[test]
fn the_once_binary_screams_publish_checkpoint_parse() {
    let root = temp_dir("duty_bin_pub_ckpt_mal");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);
    let digest = [0xcdu8; 32];
    let fingerprint = *note_codec().fingerprint();
    let child = spawn_once(&root, &theory_path);
    wait_exists(&scratch_dir(&root), Duration::from_secs(30));
    std::fs::create_dir_all(root.join("ckpt")).expect("ckpt dir");
    std::fs::write(
        root.join(format!("ckpt/{}.json", hex32(&digest))),
        b"not-a-checkpoint",
    )
    .expect("plant garbage json");
    overwrite_manifest(
        &root,
        &Manifest {
            fingerprint,
            checkpoint: Some(digest),
        }
        .render(),
    );
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: publish Checkpoint("),
        "{stderr}"
    );
}

#[test]
fn the_once_binary_screams_refresh_refused() {
    let root = temp_dir("duty_bin_refresh");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    cross_the_byte_cadence(&writer);
    drop(writer);

    let child = spawn_once(&root, &theory_path);
    wait_exists(&scratch_dir(&root), Duration::from_secs(30));
    overwrite_manifest(
        &root,
        &Manifest {
            fingerprint: [0x11; 32],
            checkpoint: None,
        }
        .render(),
    );
    let out = finish(child, Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: FingerprintMismatch"),
        "{stderr}"
    );
}

fn seed_published_store(root: &Path) -> [u8; 32] {
    let writer = note_writer(root);
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(1, "seed")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(2, "seed")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
    drop(writer);
    let mut duty = match Checkpointer::open(
        FsStore::new(root.to_path_buf()),
        "",
        &root.join("d"),
        note_only(),
        17,
    )
    .expect("open")
    {
        CheckpointerOpened::Ready(duty) => *duty,
        CheckpointerOpened::Refused(refusal) => panic!("refused: {refusal:?}"),
    };
    duty.set_checkpoint_cadence(2, u64::MAX);
    match duty.run().expect("seed publish") {
        Ran::Ready {
            compact: Compact::Published(_),
            ..
        } => {}
        other => panic!("seed publish, got {other:?}"),
    }
    manifest_checkpoint(root).expect("seeded checkpoint")
}

fn prove_once_quiet(root: &Path, theory_path: &Path) {
    let out = duty_cmd(root, theory_path)
        .arg("--once")
        .output()
        .expect("prove --once");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "seeded store must open through the binary: {stderr}"
    );
    assert!(
        stderr.is_empty(),
        "prove --once must be silent, got {stderr}"
    );
}

fn resident_then(root: &Path, theory_path: &Path, after_first: impl FnOnce()) -> Output {
    prove_once_quiet(root, theory_path);
    let child = duty_cmd(root, theory_path)
        .args(["--sleep-ms", "250"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn resident");
    std::thread::sleep(Duration::from_millis(400));
    after_first();
    finish(child, Duration::from_secs(10))
}

#[test]
fn the_resident_binary_screams_gc_manifest_missing() {
    let root = temp_dir("duty_bin_gc_miss");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(1, "gc")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
    drop(writer);

    let out = resident_then(&root, &theory_path, || {
        FsStore::new(root.clone())
            .delete(&manifest_key(""))
            .expect("delete");
    });
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: gc ManifestMissing"),
        "{stderr}"
    );
}

#[test]
fn the_resident_binary_screams_gc_manifest_parse() {
    let root = temp_dir("duty_bin_gc_mal");
    let theory_path = write_theory(&root);
    let writer = note_writer(&root);
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(RelationId(0), [note_row(1, "gc")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { .. }
    ));
    writer.quiesce();
    drop(writer);

    let out = resident_then(&root, &theory_path, || {
        overwrite_manifest(&root, b"not-a-manifest");
    });
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("duty refused: gc Manifest("), "{stderr}");
}

#[test]
fn the_resident_binary_screams_gc_checkpoint_doc_missing() {
    let root = temp_dir("duty_bin_gc_ckpt_miss");
    let theory_path = write_theory(&root);
    let digest = seed_published_store(&root);
    let out = resident_then(&root, &theory_path, || {
        FsStore::new(root.clone())
            .delete(&ckpt_json_key("", &digest))
            .expect("delete ckpt json");
    });
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("duty refused: gc CheckpointDocMissing"),
        "{stderr}"
    );
}

#[test]
fn the_resident_binary_screams_gc_checkpoint_parse() {
    let root = temp_dir("duty_bin_gc_ckpt_mal");
    let theory_path = write_theory(&root);
    let digest = seed_published_store(&root);
    let out = resident_then(&root, &theory_path, || {
        std::fs::write(
            root.join(format!("ckpt/{}.json", hex32(&digest))),
            b"not-a-checkpoint",
        )
        .expect("corrupt ckpt json");
    });
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("duty refused: gc Checkpoint {"), "{stderr}");
}

#[test]
fn argv_refuses_an_unknown_flag() {
    let out = Command::new(env!("CARGO_BIN_EXE_duty"))
        .args(["--once", "--nope"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown flag"), "{err}");
}

#[test]
fn argv_refuses_a_flag_as_a_value() {
    let out = Command::new(env!("CARGO_BIN_EXE_duty"))
        .args(["--once", "--dir", "--theory", "/tmp/x"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("needs a value"), "{err}");
}
