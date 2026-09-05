//! Real LMDB atomic adjunct tests, independent of the log receipt grammar.

use super::*;
use crate::storage::env::host::{
    AttachmentChange, HostChanges, HostRecordChange, HostSealError, MAX_HOST_KEY,
};
use crate::storage::env::{GenerationId, ReadTxn};
use crate::testutil::TempDir;
use crate::work::{ExecutionPolicy, Resource, WorkContext, WorkError};

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 4096,
        working_bytes: 4096,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: 100,
        timeout: std::time::Duration::from_secs(30),
    }
    .start()
    .unwrap()
}

fn insertion<'a>(schema: &'a Schema, env: &Environment, id: u64) -> WriteDelta<'a> {
    let view = env.read_txn().unwrap();
    let mut delta = WriteDelta::new(schema);
    delta
        .insert(&view, TARGET, &target_fact(schema, id))
        .unwrap();
    delta
}

fn has_target(view: &ReadTxn<'_>, env: &Environment, id: u64) -> bool {
    env.data()
        .get(view.raw(), &keys::fact_key(TARGET, id))
        .unwrap()
        .is_some()
}

fn commit_host(prepared: PreparedCommit<'_>, changes: HostChanges<'_>) -> CommitReport {
    prepared.seal(changes, &work()).unwrap().commit().unwrap()
}

#[test]
fn prepared_facts_and_sealed_host_rows_stay_private_until_one_commit() {
    let dir = TempDir::new("prepared-host-atomic");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let old = env.read_txn().unwrap();
    let delta = insertion(&schema, &env, 7);
    let prepared = prepare(&delta, &env).unwrap().expect("admitted");
    assert!(prepared.report().changed());
    assert_eq!(
        prepared.application_changes(),
        ApplicationChanges {
            added: 1,
            removed: 0
        }
    );
    assert!(!has_target(&old, &env, 7));
    assert_eq!(old.host_record(b"receipt/1").unwrap(), None);
    let records = [HostRecordChange::Put {
        key: b"receipt/1",
        value: b"terminal",
    }];
    let sealed = prepared
        .seal(
            HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"position/1"),
            },
            &work(),
        )
        .unwrap();
    let before_commit = env.read_txn().unwrap();
    assert!(!has_target(&before_commit, &env, 7));
    assert_eq!(before_commit.host_attachment().unwrap(), None);
    assert_eq!(before_commit.host_record(b"receipt/1").unwrap(), None);
    let report = sealed.commit().unwrap();
    assert_eq!(report.generation().value(), 1);
    let published = env.read_txn().unwrap();
    assert!(has_target(&published, &env, 7));
    assert_eq!(
        published.host_record(b"receipt/1").unwrap(),
        Some(b"terminal".as_slice())
    );
    assert_eq!(
        published.host_attachment().unwrap(),
        Some(b"position/1".as_slice())
    );
    assert_eq!(published.generation().unwrap().value(), 1);
    for snapshot in [&old, &before_commit] {
        assert!(!has_target(snapshot, &env, 7));
        assert_eq!(snapshot.host_record(b"receipt/1").unwrap(), None);
        assert_eq!(snapshot.host_attachment().unwrap(), None);
        assert_eq!(snapshot.generation().unwrap().value(), 0);
    }
}

#[test]
fn dropping_prepared_or_sealed_aborts_facts_metadata_and_generation() {
    let dir = TempDir::new("prepared-host-abort");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let before = committed_data(&env);
    let delta = insertion(&schema, &env, 9);
    drop(prepare(&delta, &env).unwrap().expect("admitted"));
    assert_eq!(committed_data(&env), before);
    let records = [HostRecordChange::Put {
        key: b"receipt",
        value: b"never-published",
    }];
    let sealed = prepare(&delta, &env)
        .unwrap()
        .expect("admitted")
        .seal(
            HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"never-visible"),
            },
            &work(),
        )
        .unwrap();
    drop(sealed);
    assert_eq!(committed_data(&env), before);
    let view = env.read_txn().unwrap();
    assert_eq!(view.host_record(b"receipt").unwrap(), None);
    assert_eq!(view.host_attachment().unwrap(), None);
    assert_eq!(view.generation().unwrap().value(), 0);
}

#[test]
fn metadata_only_commit_retirement_and_reopen_share_the_same_snapshot() {
    let dir = TempDir::new("prepared-host-reopen");
    let schema = schema();
    {
        let env = Environment::create(dir.path(), &schema).unwrap();
        let empty = WriteDelta::new(&schema);
        let records = [
            HostRecordChange::Put {
                key: b"one",
                value: b"old-receipt",
            },
            HostRecordChange::Put {
                key: b"two",
                value: b"retained-receipt",
            },
        ];
        let report = prepare(&empty, &env)
            .unwrap()
            .expect("admitted")
            .seal(
                HostChanges {
                    records: &records,
                    attachment: AttachmentChange::Put(b"epoch/1"),
                },
                &work(),
            )
            .unwrap()
            .commit()
            .unwrap();
        assert!(report.changed());
        assert_eq!(report.generation().value(), 1);
        let old = env.read_txn().unwrap();
        let records = [HostRecordChange::Delete { key: b"one" }];
        prepare(&empty, &env)
            .unwrap()
            .expect("admitted")
            .seal(
                HostChanges {
                    records: &records,
                    attachment: AttachmentChange::Put(b"epoch/2"),
                },
                &work(),
            )
            .unwrap()
            .commit()
            .unwrap();
        assert_eq!(
            old.host_record(b"one").unwrap(),
            Some(b"old-receipt".as_slice())
        );
        assert_eq!(old.host_attachment().unwrap(), Some(b"epoch/1".as_slice()));
    }
    let env = Environment::open(dir.path(), &schema).unwrap();
    let reopened = env.read_txn().unwrap();
    assert_eq!(reopened.host_record(b"one").unwrap(), None);
    assert_eq!(
        reopened.host_record(b"two").unwrap(),
        Some(b"retained-receipt".as_slice())
    );
    assert_eq!(
        reopened.host_attachment().unwrap(),
        Some(b"epoch/2".as_slice())
    );
    assert_eq!(reopened.generation().unwrap().value(), 2);
    let empty = WriteDelta::new(&schema);
    prepare(&empty, &env)
        .unwrap()
        .expect("admitted")
        .seal(
            HostChanges {
                records: &[],
                attachment: AttachmentChange::Clear,
            },
            &work(),
        )
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(env.read_txn().unwrap().host_attachment().unwrap(), None);
    assert_eq!(
        reopened.host_attachment().unwrap(),
        Some(b"epoch/2".as_slice())
    );
}

#[test]
fn injected_storage_failure_after_host_prefix_aborts_the_whole_candidate() {
    let dir = TempDir::new("prepared-host-map-full");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let before = committed_data(&env);
    let delta = insertion(&schema, &env, 10);
    let records = [
        HostRecordChange::Put {
            key: b"a",
            value: b"first-written",
        },
        HostRecordChange::Put {
            key: b"b",
            value: b"fails",
        },
    ];
    env.fail_host_seal_after(Some(1));
    let sealed = prepare(&delta, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &records,
            attachment: AttachmentChange::Put(b"not-published"),
        },
        &work(),
    );
    assert!(matches!(
        sealed,
        Err(HostSealError::Storage(Error::Lmdb(
            crate::error::LmdbFailure::Mdb(heed::MdbError::MapFull)
        )))
    ));
    assert_eq!(committed_data(&env), before);
    let view = env.read_txn().unwrap();
    assert_eq!(view.host_record(b"a").unwrap(), None);
    assert_eq!(view.host_record(b"b").unwrap(), None);
    assert_eq!(view.host_attachment().unwrap(), None);
    assert_eq!(view.generation().unwrap().value(), 0);
    env.fail_host_seal_after(None);
    prepare(&delta, &env)
        .unwrap()
        .expect("admitted")
        .seal(
            HostChanges {
                records: &records,
                attachment: AttachmentChange::Keep,
            },
            &work(),
        )
        .unwrap()
        .commit()
        .unwrap();
    assert!(has_target(&env.read_txn().unwrap(), &env, 10));
}

#[test]
fn host_caps_cancellation_and_bad_keys_refuse_without_any_prefix() {
    let dir = TempDir::new("prepared-host-bounds");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let empty = WriteDelta::new(&schema);
    let too_long = [0; MAX_HOST_KEY + 1];
    let records = [HostRecordChange::Put {
        key: &too_long,
        value: &[],
    }];
    let result = prepare(&empty, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &records,
            attachment: AttachmentChange::Keep,
        },
        &work(),
    );
    assert!(matches!(result, Err(HostSealError::KeyTooLong { .. })));
    let records = [
        HostRecordChange::Put {
            key: b"a",
            value: b"one",
        },
        HostRecordChange::Delete { key: b"a" },
    ];
    let result = prepare(&empty, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &records,
            attachment: AttachmentChange::Keep,
        },
        &work(),
    );
    assert!(matches!(result, Err(HostSealError::KeysNotStrictlyOrdered)));
    let stopped = work();
    stopped.cancel();
    let result = prepare(&empty, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &[],
            attachment: AttachmentChange::Clear,
        },
        &stopped,
    );
    assert!(matches!(
        result,
        Err(HostSealError::Work(WorkError::Cancelled))
    ));
    let huge = [0; 4097];
    let result = prepare(&empty, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &[],
            attachment: AttachmentChange::Put(&huge),
        },
        &work(),
    );
    assert!(matches!(
        result,
        Err(HostSealError::Work(WorkError::Exhausted {
            resource: Resource::InputBytes,
            ..
        }))
    ));
    assert_eq!(env.read_txn().unwrap().host_record(b"a").unwrap(), None);
    assert_eq!(env.read_txn().unwrap().host_attachment().unwrap(), None);
}

fn byte_work(work_units: u64) -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 4096,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units,
        timeout: std::time::Duration::from_secs(30),
    }
    .start()
    .unwrap()
}

#[test]
fn large_host_copy_stops_between_chunks_and_aborts_facts_and_record_prefix() {
    for attachment in [false, true] {
        let dir = TempDir::new("prepared-host-copy-budget");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).unwrap();
        let before = committed_data(&env);
        let delta = insertion(&schema, &env, 17);
        let large = vec![0x5a; 3 * 4096 + 17];
        let records = [
            HostRecordChange::Put {
                key: b"a",
                value: b"private-prefix",
            },
            HostRecordChange::Put {
                key: b"z",
                value: &large,
            },
        ];
        let context = byte_work(4500);
        let result = prepare(&delta, &env).unwrap().expect("admitted").seal(
            HostChanges {
                records: &records[..if attachment { 1 } else { 2 }],
                attachment: if attachment {
                    AttachmentChange::Put(&large)
                } else {
                    AttachmentChange::Keep
                },
            },
            &context,
        );
        assert!(matches!(
            result,
            Err(HostSealError::Work(WorkError::Exhausted {
                resource: Resource::WorkUnits,
                ..
            }))
        ));
        assert!(context.used(Resource::WorkUnits) >= 4096);
        assert_eq!(context.used(Resource::WorkingBytes), 0);
        assert_eq!(committed_data(&env), before);
        let snapshot = env.read_txn().unwrap();
        assert_eq!(snapshot.host_record(b"a").unwrap(), None);
        assert_eq!(snapshot.host_record(b"z").unwrap(), None);
        assert_eq!(snapshot.host_attachment().unwrap(), None);
        assert_eq!(snapshot.generation().unwrap().value(), 0);
    }
}

#[test]
fn large_host_equality_checks_consume_work_even_when_no_value_changes() {
    for attachment in [false, true] {
        let dir = TempDir::new("prepared-host-equality-budget");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).unwrap();
        let empty = WriteDelta::new(&schema);
        let large = vec![0x5a; 3 * 4096 + 17];
        let records = [HostRecordChange::Put {
            key: b"z",
            value: &large,
        }];
        let initial = HostChanges {
            records: if attachment { &[] } else { &records },
            attachment: if attachment {
                AttachmentChange::Put(&large)
            } else {
                AttachmentChange::Keep
            },
        };
        prepare(&empty, &env)
            .unwrap()
            .expect("admitted")
            .seal(initial, &byte_work(100_000))
            .unwrap()
            .commit()
            .unwrap();
        let before = committed_data(&env);
        let delta = insertion(&schema, &env, 18);
        let context = byte_work(4500);
        let result = prepare(&delta, &env)
            .unwrap()
            .expect("admitted")
            .seal(initial, &context);
        assert!(matches!(
            result,
            Err(HostSealError::Work(WorkError::Exhausted {
                resource: Resource::WorkUnits,
                ..
            }))
        ));
        assert!(context.used(Resource::WorkUnits) >= 4096);
        assert_eq!(context.used(Resource::WorkingBytes), 0);
        assert_eq!(committed_data(&env), before);
        let snapshot = env.read_txn().unwrap();
        if attachment {
            assert_eq!(snapshot.host_attachment().unwrap(), Some(large.as_slice()));
        } else {
            assert_eq!(snapshot.host_record(b"z").unwrap(), Some(large.as_slice()));
        }
        assert_eq!(snapshot.generation().unwrap().value(), 1);
    }
}

#[test]
fn rejected_candidate_can_be_replaced_by_empty_receipt_transaction() {
    let dir = TempDir::new("prepared-host-rejection");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let before = committed_data(&env);
    let view = env.read_txn().unwrap();
    let mut invalid = WriteDelta::new(&schema);
    invalid
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 10))
        .unwrap();
    invalid
        .insert(&view, KEYED, &keyed_fact(&schema, 1, 20))
        .unwrap();
    drop(view);
    let rejected = match prepare(&invalid, &env).unwrap() {
        Admission::Rejected(violations) => violations,
        Admission::Accepted(_) => panic!("must reject"),
    };
    assert_eq!(rejected.len(), 1);
    assert!(
        !rejected.cited_facts(0).is_empty(),
        "no best-effort missing evidence"
    );
    let empty = WriteDelta::new(&schema);
    let records = [HostRecordChange::Put {
        key: b"reject/1",
        value: b"opaque-rejection-record",
    }];
    prepare(&empty, &env)
        .unwrap()
        .expect("admitted")
        .seal(
            HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"decision/1-state/0"),
            },
            &work(),
        )
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(committed_data(&env), before);
    let snapshot = env.read_txn().unwrap();
    assert_eq!(snapshot.generation().unwrap().value(), 1);
    assert_eq!(
        snapshot.host_record(b"reject/1").unwrap(),
        Some(b"opaque-rejection-record".as_slice())
    );
    // This single-caller test does not prove writer-session exclusion between
    // the rejected candidate and its replacement; that facade remains required.
}

#[test]
fn prepared_generation_exhaustion_is_typed_and_actual_noop_still_works() {
    let dir = TempDir::new("prepared-host-generation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let mut txn = env.write_txn().unwrap();
    txn.put_generation(GenerationId::from_storage(u64::MAX))
        .unwrap();
    txn.commit().unwrap();
    let before = committed_data(&env);
    let delta = insertion(&schema, &env, 11);
    assert!(matches!(
        prepare(&delta, &env),
        Err(HostSealError::GenerationExhausted)
    ));
    assert_eq!(committed_data(&env), before);
    let empty = WriteDelta::new(&schema);
    let exhausted = prepare(&empty, &env).unwrap().expect("admitted").seal(
        HostChanges {
            records: &[],
            attachment: AttachmentChange::Put(b"must-not-publish"),
        },
        &work(),
    );
    assert!(matches!(exhausted, Err(HostSealError::GenerationExhausted)));
    assert_eq!(env.read_txn().unwrap().host_attachment().unwrap(), None);
    let report = prepare(&empty, &env)
        .unwrap()
        .expect("admitted")
        .seal(
            HostChanges {
                records: &[],
                attachment: AttachmentChange::Keep,
            },
            &work(),
        )
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(report.generation().value(), u64::MAX);
}

/// Separate process entry; ordinary harness invocation does no filesystem work.
#[test]
fn host_atomic_crash_child() {
    use std::io::Write as _;

    let Ok(directory) = std::env::var("BUMBLE_HOST_CRASH_DIRECTORY") else {
        return;
    };
    let phase = std::env::var("BUMBLE_HOST_CRASH_PHASE").unwrap();
    let schema = schema();
    let env = Environment::open(std::path::Path::new(&directory), &schema).unwrap();
    let delta = insertion(&schema, &env, 7);
    let prepared = prepare(&delta, &env).unwrap().expect("admitted");
    if phase == "prepared" {
        println!("HOST_CRASH_READY");
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
    let records = [HostRecordChange::Put {
        key: b"receipt/7",
        value: b"terminal/7",
    }];
    let sealed = prepared
        .seal(
            HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(b"position/2"),
            },
            &work(),
        )
        .unwrap();
    if phase == "committed" {
        sealed.commit().unwrap();
    } else {
        assert_eq!(phase, "sealed");
        println!("HOST_CRASH_READY");
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
    println!("HOST_CRASH_READY");
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::park();
    }
}

struct CrashChild {
    child: std::process::Child,
    output_reader: Option<std::thread::JoinHandle<()>>,
}

#[test]
fn idempotent_host_changes_do_not_advance_generation_and_fact_counts_are_net() {
    let dir = TempDir::new("prepared-host-net");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let first = insertion(&schema, &env, 1);
    let records = [HostRecordChange::Put {
        key: b"key",
        value: b"value",
    }];
    let changes = HostChanges {
        records: &records,
        attachment: AttachmentChange::Put(b"same"),
    };
    let first = prepare(&first, &env).unwrap().expect("admitted");
    assert_eq!(
        first.application_changes(),
        ApplicationChanges {
            added: 1,
            removed: 0
        }
    );
    assert_eq!(commit_host(first, changes).generation().value(), 1);
    let empty = WriteDelta::new(&schema);
    let duplicate = prepare(&empty, &env).unwrap().expect("admitted");
    assert_eq!(
        duplicate.application_changes(),
        ApplicationChanges {
            added: 0,
            removed: 0
        }
    );
    let report = commit_host(duplicate, changes);
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);
    let absent = [HostRecordChange::Delete {
        key: b"not-present",
    }];
    assert!(
        !prepare(&empty, &env)
            .unwrap()
            .expect("admitted")
            .seal(
                HostChanges {
                    records: &absent,
                    attachment: AttachmentChange::Keep
                },
                &work(),
            )
            .unwrap()
            .commit()
            .unwrap()
            .changed()
    );
    let view = env.read_txn().unwrap();
    let mut replacement = WriteDelta::new(&schema);
    replacement
        .insert(&view, TARGET, &target_fact(&schema, 1))
        .unwrap(); // existing, no net add
    replacement
        .insert(&view, TARGET, &target_fact(&schema, 2))
        .unwrap();
    replacement
        .insert(&view, TARGET, &target_fact(&schema, 2))
        .unwrap(); // duplicate spelling
    replacement
        .delete(&view, TARGET, &target_fact(&schema, 1))
        .unwrap();
    replacement
        .delete(&view, TARGET, &target_fact(&schema, 99))
        .unwrap(); // absent, no net delete
    drop(view);
    let prepared = prepare(&replacement, &env).unwrap().expect("admitted");
    assert_eq!(
        prepared.application_changes(),
        ApplicationChanges {
            added: 1,
            removed: 1
        }
    );
    let different = HostChanges {
        records: &[],
        attachment: AttachmentChange::Put(b"different"),
    };
    assert_eq!(
        commit_host(prepared, different).generation().value(),
        2,
        "facts plus metadata increment generation only once"
    );
}

#[test]
fn reserved_empty_host_values_are_present_idempotent_and_distinct_from_deletion() {
    let dir = TempDir::new("prepared-host-empty-value");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).unwrap();
    let empty = WriteDelta::new(&schema);
    let records = [HostRecordChange::Put {
        key: b"",
        value: b"",
    }];
    let changes = HostChanges {
        records: &records,
        attachment: AttachmentChange::Put(b""),
    };
    let report = commit_host(prepare(&empty, &env).unwrap().expect("admitted"), changes);
    assert!(report.changed());
    assert_eq!(report.generation().value(), 1);
    let populated = env.read_txn().unwrap();
    assert_eq!(populated.host_record(b"").unwrap(), Some(b"".as_slice()));
    assert_eq!(populated.host_attachment().unwrap(), Some(b"".as_slice()));

    let report = commit_host(prepare(&empty, &env).unwrap().expect("admitted"), changes);
    assert!(!report.changed());
    assert_eq!(report.generation().value(), 1);
    let records = [HostRecordChange::Delete { key: b"" }];
    let removed = commit_host(
        prepare(&empty, &env).unwrap().expect("admitted"),
        HostChanges {
            records: &records,
            attachment: AttachmentChange::Clear,
        },
    );
    assert!(removed.changed());
    assert_eq!(removed.generation().value(), 2);
    let current = env.read_txn().unwrap();
    assert_eq!(current.host_record(b"").unwrap(), None);
    assert_eq!(current.host_attachment().unwrap(), None);
    assert_eq!(populated.host_record(b"").unwrap(), Some(b"".as_slice()));
    assert_eq!(populated.host_attachment().unwrap(), Some(b"".as_slice()));
}

impl Drop for CrashChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.output_reader.take() {
            let _ = reader.join();
        }
    }
}

#[test]
fn process_kill_before_and_after_commit_never_reopens_a_mixed_state() {
    use std::io::{BufRead as _, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    for phase in ["prepared", "sealed", "committed"] {
        let dir = TempDir::new("host-crash-atomic");
        let schema = schema();
        {
            let env = Environment::create(dir.path(), &schema).unwrap();
            let delta = insertion(&schema, &env, 1);
            prepare(&delta, &env)
                .unwrap()
                .expect("admitted")
                .seal(
                    HostChanges {
                        records: &[],
                        attachment: AttachmentChange::Put(b"position/1"),
                    },
                    &work(),
                )
                .unwrap()
                .commit()
                .unwrap();
        }
        let mut process = Command::new(std::env::current_exe().unwrap());
        process
            .args([
                "--exact",
                "storage::commit::tests::host::host_atomic_crash_child",
                "--nocapture",
            ])
            .env("BUMBLE_HOST_CRASH_DIRECTORY", dir.path())
            .env("BUMBLE_HOST_CRASH_PHASE", phase)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = process.spawn().unwrap();
        let output = child.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        let output_reader = std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                match line {
                    Ok(line) if line == "HOST_CRASH_READY" => {
                        let _ = send.send(());
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
        let mut owner = CrashChild {
            child,
            output_reader: Some(output_reader),
        };
        receive
            .recv_timeout(Duration::from_secs(15))
            .expect("child reached explicit crash point");
        owner.child.kill().expect("kill exact owned child");
        let status = owner.child.wait().expect("reap child");
        assert!(!status.success());
        drop(owner); // Joins capture reader even when the test later panics.
        let env = Environment::open(dir.path(), &schema).expect("kernel lock released on death");
        let view = env.read_txn().unwrap();
        let published = phase == "committed";
        assert!(has_target(&view, &env, 1));
        assert_eq!(has_target(&view, &env, 7), published, "phase {phase}");
        assert_eq!(
            view.host_record(b"receipt/7").unwrap(),
            if published {
                Some(b"terminal/7".as_slice())
            } else {
                None
            }
        );
        assert_eq!(
            view.host_attachment().unwrap(),
            Some(if published {
                b"position/2".as_slice()
            } else {
                b"position/1".as_slice()
            })
        );
        assert_eq!(
            view.generation().unwrap().value(),
            if published { 2 } else { 1 }
        );
    }
}
