use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::channel;
use std::time::Duration;

use bumbledb::{RelationId, StatementId, Value};
use bumbledb_log::{DatabaseId, IoResponse, IoResult, Millis};

use super::*;
use crate::marshal::ValueOut;
use crate::runtime::owners::DirectoryOwner;
use crate::runtime::session::SnapshotSession;
use crate::runtime::{CloseReport, Options, Runtime};

const SPEC: &str = r#"{"relations":[{"name":"Item","fields":[
    {"name":"id","valueType":{"kind":"U64"}},
    {"name":"label","valueType":{"kind":"String"}}]}],
  "statements":[{"kind":"Fd","relation":"Item","projection":["id"]}]}"#;

fn options() -> Options {
    Options {
        workers: 2,
        queue_capacity: 16,
        cleanup_capacity: 8,
        owner_capacity: 4,
        native_handle_capacity: 16,
        cleanup_timeout: Duration::from_secs(5),
    }
}

/// An object store in memory: create-only puts, lexicographic lists, and a
/// clock that ticks once per request.
#[derive(Default)]
struct Store {
    log: BTreeMap<String, (Vec<u8>, u64)>,
    checkpoints: BTreeMap<String, (Vec<u8>, u64)>,
    now: u64,
}

impl Store {
    fn serve(&mut self, request: &IoRequestOut) -> IoResponse {
        self.now += 1;
        let bucket = match request.bucket {
            BucketOut::Log => &mut self.log,
            BucketOut::Checkpoints => &mut self.checkpoints,
        };
        let mut put = |bytes: Vec<u8>, now| {
            if bucket.contains_key(&request.key) {
                IoResult::Occupied
            } else {
                bucket.insert(request.key.clone(), (bytes, now));
                IoResult::Created
            }
        };
        let result = match &request.op {
            OpOut::GetMemory => {
                bucket
                    .get(&request.key)
                    .map_or(IoResult::Missing, |found| IoResult::Body {
                        bytes: found.0.clone(),
                        last_modified: Millis(found.1),
                    })
            }
            OpOut::GetFile { path } => {
                bucket.get(&request.key).map_or(IoResult::Missing, |found| {
                    std::fs::write(path, &found.0).unwrap();
                    IoResult::Saved {
                        last_modified: Millis(found.1),
                    }
                })
            }
            OpOut::PutBytes { bytes } => put(bytes.bytes.clone(), self.now),
            OpOut::PutFile { path } => put(std::fs::read(path).unwrap(), self.now),
            OpOut::List {
                start_after,
                max_keys,
            } => IoResult::Keys(
                bucket
                    .keys()
                    .filter(|key| key.starts_with(&request.key))
                    .filter(|key| start_after.as_ref().is_none_or(|after| *key > after))
                    .take(*max_keys as usize)
                    .cloned()
                    .collect(),
            ),
            OpOut::Delete => {
                bucket.remove(&request.key);
                IoResult::Deleted
            }
        };
        IoResponse {
            id: IoId(request.id),
            date: Some(Millis(self.now)),
            result,
        }
    }
}

fn wait(
    runtime: &Runtime,
    start: impl FnOnce(crate::runtime::Notify) -> Arc<crate::runtime::Operation>,
) -> Output {
    let (sender, receiver) = channel();
    let operation = start(Box::new(move || sender.send(()).unwrap()));
    receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    runtime.take(&operation).expect("operation succeeds")
}

/// Run one input and every store request it leads to; the settled tickets.
fn drive(
    runtime: &Runtime,
    hosted: &RegistryAdmission,
    store: &mut Store,
    first: Output,
) -> Vec<DoneOut> {
    let Output::HostedStep(step) = first else {
        panic!("expected a machine step")
    };
    let mut done = step.done;
    let mut pending: VecDeque<_> = step.io.into();
    while let Some(request) = pending.pop_front() {
        let response = store.serve(&request);
        let Output::HostedStep(step) = wait(runtime, |notify| {
            super::step(hosted, notify, Input::Response(response)).unwrap()
        }) else {
            panic!("expected a machine step")
        };
        pending.extend(step.io);
        done.extend(step.done);
    }
    done
}

fn acquire(runtime: &Arc<Runtime>, path: &std::path::Path) -> DirectoryOwner {
    match wait(runtime, |notify| {
        runtime
            .acquire_directory(
                path.to_string_lossy().into_owned(),
                WorkContext::new(),
                notify,
            )
            .unwrap()
    }) {
        Output::Directory(owner) => owner,
        _ => panic!("expected a directory owner"),
    }
}

fn changes(schema: &SchemaHandle, rows: &[(u64, &str)]) -> Payload {
    let mut builder = bumbledb::ChangeSet::builder(&schema.schema, WorkContext::new());
    for (id, label) in rows {
        builder
            .insert(
                RelationId(0),
                &[Value::U64(*id), Value::String((*label).into())],
            )
            .unwrap();
    }
    let changes = builder.finish().unwrap();
    Payload::Changes {
        fingerprint: crate::schema::hex(&changes.schema().0),
        changes,
        schema: Arc::clone(&schema.schema),
    }
}

fn read_label(runtime: &Arc<Runtime>, session: &SnapshotSession, id: u64) -> Option<String> {
    let Output::Row(row) = wait(runtime, |notify| {
        session
            .submit(WorkContext::new(), notify, move |_| {
                Ok(crate::db_wire::snapshot_get_work(
                    RelationId(0),
                    StatementId(0),
                    vec![Value::U64(id)],
                ))
            })
            .unwrap()
    }) else {
        panic!("expected a point read")
    };
    row.map(|row| match &row.values[1] {
        ValueOut::Text(label) => label.clone(),
        _ => panic!("label is text"),
    })
}

fn open_config() -> Config {
    Config {
        seed: [3; 16],
        create: Some(DatabaseId([9; 16])),
        probe_window: 4,
        checkpoint: CheckpointPolicy { every: 2, keep: 2 },
        max_entry_bytes: 1 << 20,
    }
}

fn bundle() -> Vec<BundleStepIn> {
    vec![
        crate::input::decode(&format!(
            r#"{{"name":"0000_init","hash":"{}","schema":{SPEC}}}"#,
            "ab".repeat(32)
        ))
        .unwrap(),
    ]
}

#[test]
fn a_hosted_database_creates_decides_and_reads_through_the_bridge() {
    let runtime = Runtime::start(options()).unwrap();
    let base = std::env::temp_dir().join(format!("bumbledb-node-hosted-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let owner = acquire(&runtime, &base.join("cache"));
    let reference = owner.reference();
    let root = reference.child_path("db").unwrap();
    let opened = open_hosted(
        reference.clone(),
        &root,
        bundle(),
        open_config(),
        reference.lease().unwrap(),
    )
    .unwrap();
    assert_eq!(opened.unchanged.len(), 1);
    let schema = Arc::clone(&opened.hosted.schemas[0]);
    let reads = Arc::clone(&opened.hosted.reads);
    let hosted = RegistryAdmission::admit(
        Arc::clone(&runtime),
        NativeKind::Hosted,
        Payload::Hosted(opened.hosted),
    )
    .unwrap();
    reference.adopt(hosted.cap()).unwrap();
    let mut store = Store::default();

    let open = wait(&runtime, |notify| {
        step(&hosted, notify, Input::Open(Ticket(1))).unwrap()
    });
    let done = drive(&runtime, &hosted, &mut store, open);
    assert!(matches!(
        done.as_slice(),
        [DoneOut {
            ticket: 1,
            settled: SettledOut::Opened { pending: 0 }
        }]
    ));
    assert!(store.log.contains_key("log/00000000000000000001"));

    let staged = RegistryAdmission::admit(
        Arc::clone(&runtime),
        NativeKind::Changes,
        changes(&schema, &[(1, "one"), (2, "two")]),
    )
    .unwrap();
    let changes_handle = crate::db_wire::ChangesHandle::from_admission(staged);
    let submitted = wait(&runtime, |notify| {
        step_with_changes(&hosted, &changes_handle, notify, |changes, _| {
            Ok(Input::Submit(
                Ticket(2),
                Command::seal(RequestId([5; 16]), Precondition::None, changes),
            ))
        })
        .unwrap()
    });
    let done = drive(&runtime, &hosted, &mut store, submitted);
    let [
        DoneOut {
            ticket: 2,
            settled: SettledOut::Decided { receipt },
        },
    ] = done.as_slice()
    else {
        panic!("the command is decided: {}", done.len())
    };
    assert!(matches!(
        receipt.outcome,
        OutcomeOut::Committed {
            added: 2,
            removed: 0
        }
    ));
    assert_eq!(receipt.request, "05".repeat(16));

    let Output::Session(opened) = wait(&runtime, |notify| snapshot(&reads, notify).unwrap()) else {
        panic!("expected a snapshot")
    };
    assert_eq!(opened.schema.fingerprint, schema.fingerprint);
    assert_eq!(
        read_label(&runtime, &opened.session, 2).as_deref(),
        Some("two")
    );
    assert_eq!(read_label(&runtime, &opened.session, 3), None);

    let closing = wait(&runtime, |notify| {
        step(&hosted, notify, Input::Close).unwrap()
    });
    assert!(drive(&runtime, &hosted, &mut store, closing).is_empty());
    drop(opened);
    drop(reads);
    drop(changes_handle);
    drop(hosted);
    let (sender, receiver) = channel();
    owner.drain(Box::new(move |report| sender.send(report).unwrap()));
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
        CloseReport::Closed
    );
    let (sender, receiver) = channel();
    runtime.drain(None, Box::new(move |report| sender.send(report).unwrap()));
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
        CloseReport::Closed
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn malformed_open_documents_cite_their_path() {
    let refused = crate::input::decode::<HostedOpenIn>(
        r#"{"bundle":[],"seed":"00","probeWindow":1,"checkpointEvery":"1","checkpointKeep":1,"maxEntryBytes":1,"extra":true}"#,
    )
    .unwrap_err();
    assert!(refused.message.contains("extra"), "{refused:?}");
}
