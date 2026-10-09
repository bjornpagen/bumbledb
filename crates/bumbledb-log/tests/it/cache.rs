//! The LMDB cache under the machine: its decisions match the reference
//! model's, applying a decided entry reproduces the recorded deltas, it
//! replays hostile logs to the reference fold, and its images, reopenings
//! and migrations install whole states.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bumbledb::{RelationId, WorkContext};
use bumbledb_log::{
    Body, Bucket, Bundle, Cache, CheckpointPolicy, Command, Config, DatabaseId, Done, Entry, Input,
    IoBody, IoResponse, IoResult, Machine, Millis, Op, Outcome, Precondition, Receipt, Refusal,
    Replica as _, RequestId, Revision, Seq, Settled, Target, Ticket,
};

use crate::model::{Model, State};
use crate::sim::{Faults, TempDir, World, population, replay};
use crate::support::{Rng, bundle, bundle2, items, migration_id, schema, schema2, tags};

type Bucketed = BTreeMap<String, (Vec<u8>, u64)>;

/// One machine over a real cache, with a fault-free store run to completion
/// after every input.
struct Solo {
    machine: Machine<Cache>,
    tickets: u64,
}

#[derive(Default)]
struct Store {
    log: Bucketed,
    checkpoints: Bucketed,
    now: u64,
}

impl Store {
    fn run(&mut self, solo: &mut Solo, input: Input) -> Vec<Done> {
        let step = solo.machine.step(input);
        let mut done = step.done;
        done.extend(self.drive(solo, step.io));
        done
    }

    /// Execute `io` and everything it leads to, in order.
    fn drive(&mut self, solo: &mut Solo, io: Vec<bumbledb_log::IoRequest>) -> Vec<Done> {
        let mut done = Vec::new();
        let mut pending: std::collections::VecDeque<_> = io.into();
        while let Some(request) = pending.pop_front() {
            self.now += 1;
            let bucket = match request.bucket {
                Bucket::Log => &mut self.log,
                Bucket::Checkpoints => &mut self.checkpoints,
            };
            let result = match &request.op {
                Op::Get(target) => match (bucket.get(&request.key), target) {
                    (None, _) => IoResult::Missing,
                    (Some((bytes, at)), Target::Memory) => IoResult::Body {
                        bytes: bytes.clone(),
                        last_modified: Millis(*at),
                    },
                    (Some((bytes, at)), Target::File(path)) => {
                        std::fs::write(path, bytes).unwrap();
                        IoResult::Saved {
                            last_modified: Millis(*at),
                        }
                    }
                },
                Op::PutIfAbsent(body) => {
                    if bucket.contains_key(&request.key) {
                        IoResult::Occupied
                    } else {
                        let bytes = match body {
                            IoBody::Bytes(bytes) => bytes.clone(),
                            IoBody::File(path) => std::fs::read(path).unwrap(),
                        };
                        bucket.insert(request.key.clone(), (bytes, self.now));
                        IoResult::Created
                    }
                }
                Op::List {
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
                Op::Delete => {
                    bucket.remove(&request.key);
                    IoResult::Deleted
                }
            };
            let step = solo.machine.step(Input::Response(IoResponse {
                id: request.id,
                date: Some(Millis(self.now)),
                result,
            }));
            pending.extend(step.io);
            done.extend(step.done);
        }
        done
    }
}

fn config(seed: u8, every: u64) -> Config {
    Config {
        seed: [seed; 16],
        create: Some(DatabaseId([seed; 16])),
        probe_window: 8,
        checkpoint: CheckpointPolicy { every, keep: 2 },
        max_entry_bytes: 1 << 20,
    }
}

impl Solo {
    fn open(store: &mut Store, root: &Path, bundle: Bundle, config: Config) -> (Self, Settled) {
        let cache = Cache::open(root, bundle).unwrap();
        let mut solo = Self {
            machine: Machine::new(cache, config),
            tickets: 0,
        };
        let ticket = solo.ticket();
        let done = store.run(&mut solo, Input::Open(ticket));
        (solo, settled_one(&done, ticket))
    }

    fn ticket(&mut self) -> Ticket {
        self.tickets += 1;
        Ticket(self.tickets)
    }
}

fn settled_one(done: &[Done], ticket: Ticket) -> Settled {
    done.iter()
        .find(|done| done.ticket == ticket)
        .map(|done| done.settled.clone())
        .expect("the ticket settled")
}

/// The cache's whole state as the reference model represents it.
fn cache_state(cache: &Cache) -> State {
    let db = cache.db().expect("a live cache");
    let work = WorkContext::new();
    let read = db.snapshot(&work).unwrap();
    let mut rows = BTreeSet::new();
    for relation in 0..db.schema().relations().len() {
        let relation = u32::try_from(relation).unwrap();
        for row in read.snapshot().rows(RelationId(relation)).unwrap() {
            rows.insert((relation, row.unwrap().1.to_vec()));
        }
    }
    let mut receipts = BTreeMap::new();
    db.read(work, |frame| {
        frame
            .integration_host_scan(b"r", &mut |_, value| {
                let receipt = Receipt::decode(value).expect("a receipt record");
                receipts.insert(receipt.command.request, receipt);
                Ok(())
            })
            .expect("scan");
        Ok(())
    })
    .unwrap();
    State {
        head: cache.head().expect("a head").clone(),
        rows,
        receipts,
    }
}

/// Equal up to the bytes of rejection evidence, which the engine and the
/// reference model spell differently.
fn assert_same(cache: &State, model: &State) {
    assert_eq!(cache.head, model.head);
    assert_eq!(cache.rows, model.rows);
    let kinds = |state: &State| {
        state
            .receipts
            .values()
            .map(|receipt| {
                let outcome = match &receipt.outcome {
                    Outcome::InvariantRejected(_) => None,
                    other => Some(other.clone()),
                };
                (receipt.command, receipt.seq, receipt.revision, outcome)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(cache), kinds(model));
}

/// Fold `log` (commands only) through a fresh reference model.
fn reference(log: &Bucketed, bundle: &Bundle) -> State {
    let dir = TempDir::new("reference");
    let mut model = Model::new(bundle.clone(), dir.path());
    for (index, (_, (bytes, at))) in log.iter().enumerate() {
        let seq = Seq::new(index as u64 + 1).unwrap();
        let entry = Entry::parse(&bundle.initial().schema, bytes).unwrap();
        let folded = bumbledb_log::fold(model.head(), seq, &entry, Millis(*at)).unwrap();
        if seq == Seq::GENESIS {
            model.create(&folded.head).unwrap();
        } else {
            model
                .apply(bumbledb_log::Update {
                    head: &folded.head,
                    commits: &folded.commits,
                    receipts: &folded.receipts,
                })
                .unwrap();
        }
    }
    model.state().unwrap().clone()
}

#[test]
fn the_cache_decides_and_applies_like_the_reference() {
    let schema = schema();
    let dir = TempDir::new("cache-decide");
    let mut store = Store::default();
    let (mut solo, opened) = Solo::open(&mut store, dir.path(), bundle(), config(1, 1_000));
    assert_eq!(opened, Settled::Opened { pending: 0 });
    let mut rng = Rng::new(7);
    let mut receipts = Vec::new();
    for round in 0..12u8 {
        // Several commands per round: the first is written alone, the rest
        // wait for it and are decided together in the next entry.
        let mut tickets = Vec::new();
        let mut steps = Vec::new();
        for index in 0..=rng.below(4) {
            let request = RequestId([
                round,
                u8::try_from(index).unwrap(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
            ]);
            let id = rng.below(5);
            let changes = if rng.chance(70) {
                items(&schema, &[(id, rng.below(2))], &[])
            } else {
                items(&schema, &[], &[(id, rng.below(2))])
            };
            let precondition = if rng.chance(20) {
                Precondition::ExactRevision(Revision(rng.below(8)))
            } else {
                Precondition::None
            };
            let ticket = solo.ticket();
            tickets.push(ticket);
            steps.push(solo.machine.step(Input::Submit(
                ticket,
                Command::seal(request, precondition, changes),
            )));
        }
        let mut done: Vec<Done> = Vec::new();
        let mut io = Vec::new();
        for step in steps {
            done.extend(step.done);
            io.extend(step.io);
        }
        assert_eq!(io.len(), 1, "one write in flight");
        done.extend(store.drive(&mut solo, io));
        for ticket in tickets {
            let Settled::Decided(receipt) = settled_one(&done, ticket) else {
                panic!("every command is decided");
            };
            receipts.push(receipt);
        }
    }
    let model = reference(&store.log, &bundle());
    assert_same(&cache_state(solo.machine.replica()), &model);
    for receipt in receipts {
        let recorded = &model.receipts[&receipt.command.request];
        assert_eq!(
            (recorded.seq, recorded.revision),
            (receipt.seq, receipt.revision)
        );
    }
    assert!(
        store.log.values().any(|(bytes, _)| matches!(
            Entry::parse(&schema, bytes).unwrap().body,
            Body::Commands(ref decided) if decided.len() > 1
        )),
        "some entry decided several commands"
    );
}

#[test]
fn a_cold_cache_replays_a_hostile_log_to_the_reference_fold() {
    for seed in [3, 4] {
        let schema = schema();
        let mut world = World::new(seed, 3, &bundle(), Faults::HOSTILE);
        for client in 0..3 {
            world.start(client);
        }
        for _ in 0..300 {
            let client = usize::try_from(world.rng.below(3)).unwrap();
            match world.rng.below(10) {
                0..=2 if world.alive(client) => {
                    let (request, command) = world.random_command(client, &schema);
                    world.submit(client, request, command);
                }
                3 => world.deliver_one(),
                _ => world.execute_one(),
            }
        }
        world.quiesce();
        let states = replay(&world);
        let dir = TempDir::new("cache-replay");
        let mut store = Store {
            log: world.log.clone(),
            checkpoints: Bucketed::new(),
            now: world.now,
        };
        let mut config = config(9, 1_000);
        config.create = None;
        let (solo, opened) = Solo::open(&mut store, dir.path(), bundle(), config);
        assert_eq!(opened, Settled::Opened { pending: 0 });
        assert_same(&cache_state(solo.machine.replica()), states.last().unwrap());
    }
}

#[test]
fn a_cold_cache_installs_a_checkpoint_and_refuses_a_corrupt_one() {
    let schema = schema();
    let writer_dir = TempDir::new("cache-writer");
    let mut store = Store::default();
    let (mut writer, _) = Solo::open(&mut store, writer_dir.path(), bundle(), config(2, 3));
    for request in 1..=8u8 {
        let ticket = writer.ticket();
        let command = Command::seal(
            RequestId([request; 16]),
            Precondition::None,
            items(&schema, &[(u64::from(request), 1)], &[]),
        );
        store.run(&mut writer, Input::Submit(ticket, command));
    }
    let checkpoints: Vec<_> = store
        .checkpoints
        .keys()
        .filter_map(|key| bumbledb_log::CheckpointKey::parse(key))
        .map(|key| key.seq.get())
        .collect();
    assert_eq!(checkpoints, [9, 6], "every 3 entries, 2 kept");
    let expected = cache_state(writer.machine.replica());

    let cold = TempDir::new("cache-cold");
    let mut config = config(3, 1_000);
    config.create = None;
    let (reader, _) = Solo::open(&mut store, cold.path(), bundle(), config.clone());
    assert_same(&cache_state(reader.machine.replica()), &expected);

    let newest = store.checkpoints.keys().next().unwrap().clone();
    store.checkpoints.get_mut(&newest).unwrap().0[100] ^= 0xff;
    let corrupt = TempDir::new("cache-corrupt");
    let (reader, opened) = Solo::open(&mut store, corrupt.path(), bundle(), config.clone());
    assert_eq!(opened, Settled::Opened { pending: 0 });
    assert_same(&cache_state(reader.machine.replica()), &expected);

    // An image whose digest matches but which LMDB cannot open.
    let garbage = b"not an LMDB environment".to_vec();
    let key = bumbledb_log::CheckpointKey {
        seq: Seq::new(9).unwrap(),
        schema: bundle().initial().fingerprint,
        digest: bumbledb_log::ImageDigest(*blake3::hash(&garbage).as_bytes()),
    };
    store.checkpoints.clear();
    store.checkpoints.insert(key.format(), (garbage, 1));
    let unopenable = TempDir::new("cache-unopenable");
    let (reader, opened) = Solo::open(&mut store, unopenable.path(), bundle(), config);
    assert_eq!(opened, Settled::Opened { pending: 0 });
    assert_same(&cache_state(reader.machine.replica()), &expected);
}

#[test]
fn a_reopened_cache_resumes_and_a_broken_one_is_rebuilt() {
    let schema = schema();
    let dir = TempDir::new("cache-reopen");
    let mut store = Store::default();
    let (mut solo, _) = Solo::open(&mut store, dir.path(), bundle(), config(4, 1_000));
    for request in 1..=3u8 {
        let ticket = solo.ticket();
        let command = Command::seal(
            RequestId([request; 16]),
            Precondition::None,
            items(&schema, &[(u64::from(request), 1)], &[]),
        );
        store.run(&mut solo, Input::Submit(ticket, command));
    }
    let expected = cache_state(solo.machine.replica());
    drop(solo.machine.into_replica());

    let reopened = Cache::open(dir.path(), bundle()).unwrap();
    assert_same(&cache_state(&reopened), &expected);
    drop(reopened);

    let live = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "bdb"))
        .expect("a live generation");
    std::fs::write(live.join("data.mdb"), b"not a database").unwrap();
    let rebuilt = Cache::open(dir.path(), bundle()).unwrap();
    assert!(rebuilt.head().is_none(), "an unreadable cache is discarded");
    let mut config = config(5, 1_000);
    config.create = None;
    let (solo, opened) = Solo::open(&mut store, dir.path(), bundle(), config);
    assert_eq!(opened, Settled::Opened { pending: 0 });
    assert_same(&cache_state(solo.machine.replica()), &expected);
}

#[test]
fn the_cache_migrates_and_records_a_rejected_migration() {
    let dir = TempDir::new("cache-migrate");
    let mut store = Store::default();
    let (mut solo, opened) = Solo::open(&mut store, dir.path(), bundle2(), config(6, 1_000));
    assert_eq!(opened, Settled::Opened { pending: 1 });
    let ticket = solo.ticket();
    let done = store.run(
        &mut solo,
        Input::Submit(
            ticket,
            Command::seal(
                RequestId([1; 16]),
                Precondition::None,
                items(&schema(), &[(1, 1)], &[]),
            ),
        ),
    );
    assert_eq!(
        settled_one(&done, ticket),
        Settled::Refused(Refusal::MigrationPending { next: 1 })
    );

    let rejected = population(
        migration_id("0001_tags"),
        Seq::GENESIS,
        tags(&schema2(), &[(1, 1), (1, 2)]),
    );
    let ticket = solo.ticket();
    let done = store.run(&mut solo, Input::Migrate(ticket, rejected));
    assert!(matches!(
        settled_one(&done, ticket),
        Settled::Refused(Refusal::MigrationRejected(_))
    ));
    assert_eq!(
        solo.machine.replica().head().unwrap().ledger.rejected.len(),
        1
    );

    let fixed = Bundle::new(vec![
        (migration_id("0000_init"), crate::support::descriptor()),
        (
            migration_id("0001_tags_fixed"),
            crate::support::descriptor2(),
        ),
    ])
    .unwrap();
    let fixed_dir = TempDir::new("cache-fixed");
    let mut config = config(7, 1_000);
    config.create = None;
    let (mut fixed_solo, opened) =
        Solo::open(&mut store, fixed_dir.path(), fixed.clone(), config.clone());
    assert_eq!(opened, Settled::Opened { pending: 1 });
    let ticket = fixed_solo.ticket();
    let base = fixed_solo.machine.replica().head().unwrap().seq;
    let mut migration = population(
        migration_id("0001_tags_fixed"),
        base,
        tags(&schema2(), &[(1, 5), (2, 6)]),
    );
    migration.step = migration_id("0001_tags_fixed");
    let done = store.run(&mut fixed_solo, Input::Migrate(ticket, migration));
    assert_eq!(
        settled_one(&done, ticket),
        Settled::Migrated(Seq::new(3).unwrap())
    );
    let ticket = fixed_solo.ticket();
    let done = store.run(
        &mut fixed_solo,
        Input::Submit(
            ticket,
            Command::seal(
                RequestId([2; 16]),
                Precondition::None,
                tags(&schema2(), &[(3, 3)]),
            ),
        ),
    );
    assert!(matches!(settled_one(&done, ticket), Settled::Decided(_)));
    let migrated = cache_state(fixed_solo.machine.replica());
    assert_eq!(migrated.head.schema, fixed.steps()[1].fingerprint);
    assert_eq!(
        migrated
            .rows
            .iter()
            .filter(|(relation, _)| *relation == 1)
            .count(),
        3
    );

    let cold = TempDir::new("cache-cold-migrated");
    let (reader, opened) = Solo::open(&mut store, cold.path(), fixed, config);
    assert_eq!(opened, Settled::Opened { pending: 0 });
    assert_same(&cache_state(reader.machine.replica()), &migrated);
}

#[test]
fn catch_up_applies_what_the_log_decided_without_judging_it_again() {
    let schema = schema();
    let bundle = bundle();
    let step = &bundle.steps()[0];
    let genesis = Entry {
        nonce: bumbledb_log::Nonce([1; 16]),
        body: Body::Genesis(bumbledb_log::Genesis {
            database: DatabaseId([1; 16]),
            initial: step.id.clone(),
            schema: step.fingerprint,
        }),
    };
    // Two rows sharing a key: today's judgment would reject this commit.
    let decided = Entry {
        nonce: bumbledb_log::Nonce([2; 16]),
        body: Body::Commands(
            vec![bumbledb_log::Decided {
                command: bumbledb_log::CommandRef {
                    request: RequestId([3; 16]),
                    digest: bumbledb_log::CommandDigest([4; 32]),
                },
                verdict: bumbledb_log::Verdict::Committed {
                    changes: items(&schema, &[(1, 1), (1, 2)], &[]),
                    delta: bumbledb_log::Delta::new(2, 0).unwrap(),
                },
            }]
            .into(),
        ),
    };
    let mut store = Store::default();
    for (seq, entry) in [genesis, decided].iter().enumerate() {
        let key = bumbledb_log::log_key(Seq::new(seq as u64 + 1).unwrap());
        store.log.insert(key, (entry.encode(), 1));
    }
    let dir = TempDir::new("cache-decided");
    let mut config = config(8, 1_000);
    config.create = None;
    let (solo, opened) = Solo::open(&mut store, dir.path(), bundle.clone(), config);
    assert_eq!(opened, Settled::Opened { pending: 0 });
    let state = cache_state(solo.machine.replica());
    assert_eq!(state.head.seq, Seq::new(2).unwrap());
    assert_eq!(state.rows.len(), 2);
}
