//! The duty body over `FsStore`: one cadence check, compact under the
//! checkpoint order, the retention sweep, and the same body through
//! the `duty` binary's `--once` arm.

mod lane_e_support;

use std::process::Command;

use bumbledb::SchemaDescriptor;
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor as Desc,
    StatementDescriptor, ValueType,
};
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened, Compact, Ran};
use bumbledb_log::gc::Gc;
use bumbledb_log::manifest::{Manifest, manifest_key};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Options, Writer, WriterOpened};
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

#[test]
fn the_once_binary_exits_zero_after_one_body() {
    let root = temp_dir("duty_bin");
    let theory_path = root.join("theory.json");
    std::fs::write(
        &theory_path,
        r#"{"relations":[{"name":"note","fields":[{"name":"id","type":"u64"},{"name":"body","type":"string"}]}],"statements":[{"functionality":{"relation":0,"projection":[0]}}]}"#,
    )
    .expect("write theory");

    let writer = match Writer::open(
        FsStore::new(root.clone()),
        "",
        &root.join("w"),
        note_only(),
        Options::new(17),
    )
    .expect("open")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("refused: {refusal:?}"),
    };
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

    let out = Command::new(env!("CARGO_BIN_EXE_duty"))
        .args([
            "--once",
            "--store",
            "fs",
            "--root",
            root.to_str().expect("utf8"),
            "--dir",
            root.join("d").to_str().expect("utf8"),
            "--theory",
            theory_path.to_str().expect("utf8"),
        ])
        .output()
        .expect("spawn duty");
    assert!(
        out.status.success(),
        "duty --once failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
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
