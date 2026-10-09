//! A deterministic world for the machine: a fake two-bucket object store
//! with injected faults (failed and dropped writes, reordered responses,
//! killed processes whose requests still land), driven by a seeded
//! generator. `check` replays the final log through the reference model and
//! verifies every settled ticket against it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb::{ChangeSet, Schema};
use bumbledb_log::{
    Body, Bucket, CheckpointKey, CheckpointPolicy, Command, Config, DatabaseId, Entry, Input,
    IoBody, IoRequest, IoResponse, IoResult, Machine, Millis, Op, Outcome, Population,
    Precondition, Receipt, Refusal, Replica as _, RequestId, Revision, Seq, Settled, Target,
    Ticket, Verdict, image_key,
};

use crate::model::{Model, State};
use crate::support::{Rng, items};

/// A scratch directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "bumbledb-log-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Faults {
    /// A PUT fails before reaching the store.
    pub put_lost: u64,
    /// A PUT lands but its response is lost.
    pub put_unanswered: u64,
    /// A response is held back and delivered later, out of order.
    pub delayed: u64,
    /// A GET or LIST fails.
    pub read_failed: u64,
}

impl Faults {
    pub const NONE: Self = Self {
        put_lost: 0,
        put_unanswered: 0,
        delayed: 0,
        read_failed: 0,
    };

    pub const HOSTILE: Self = Self {
        put_lost: 8,
        put_unanswered: 8,
        delayed: 15,
        read_failed: 6,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    Answered,
    /// Never reaches the store; the response is `Failed`.
    Lost,
    /// Reaches the store; the response is `Failed`.
    Unanswered,
    /// Reaches the store; the response arrives later.
    Delayed,
}

#[derive(Debug, Clone)]
pub enum Ask {
    Open,
    Submit(RequestId),
    Sync,
    Resolve(RequestId),
    Freeze,
    Migrate,
}

#[derive(Debug, Clone)]
pub struct Result {
    pub settled: Settled,
    /// The settling client's head right after the step.
    pub head: Option<Seq>,
}

pub struct Client {
    pub machine: Option<Machine<Model>>,
    /// The cache of a killed process, kept unless the kill wiped it.
    pub parked: Option<Model>,
    pub incarnation: u32,
    pub config: Config,
}

struct Issued {
    client: usize,
    incarnation: u32,
    request: IoRequest,
    /// A PUT body read when the request was issued.
    body: Option<Vec<u8>>,
}

struct Delivery {
    client: usize,
    incarnation: u32,
    response: IoResponse,
}

pub struct World {
    pub rng: Rng,
    pub faults: Faults,
    pub now: u64,
    pub log: BTreeMap<String, (Vec<u8>, u64)>,
    pub checkpoints: BTreeMap<String, (Vec<u8>, u64)>,
    pub clients: Vec<Client>,
    pub asks: BTreeMap<Ticket, (usize, Ask)>,
    pub results: BTreeMap<Ticket, Result>,
    /// Tickets of killed incarnations that never settled.
    pub abandoned: BTreeSet<Ticket>,
    pub commands: BTreeMap<RequestId, Command>,
    /// Commands submitted under an already-used request id.
    pub reused: BTreeMap<Ticket, Command>,
    _dir: TempDir,
    /// The longest bundle among the clients: the replay's schemas.
    bundle: bumbledb_log::Bundle,
    issued: Vec<Issued>,
    delayed: Vec<Delivery>,
    next_ticket: u64,
    operations: u64,
    pub tally: Tally,
}

/// What a run exercised, so a passing simulation is not a vacuous one.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    pub occupied: u64,
    pub unanswered: u64,
    pub lost: u64,
    pub delayed: u64,
    pub kills: u64,
    pub installs: u64,
}

impl std::ops::AddAssign for Tally {
    fn add_assign(&mut self, other: Self) {
        self.occupied += other.occupied;
        self.unanswered += other.unanswered;
        self.lost += other.lost;
        self.delayed += other.delayed;
        self.kills += other.kills;
        self.installs += other.installs;
    }
}

impl World {
    pub fn new(seed: u64, clients: usize, bundle: &bumbledb_log::Bundle, faults: Faults) -> Self {
        Self::with_bundles(seed, vec![bundle.clone(); clients], faults)
    }

    /// One client per bundle.
    pub fn with_bundles(seed: u64, bundles: Vec<bumbledb_log::Bundle>, faults: Faults) -> Self {
        let mut rng = Rng::new(seed);
        let dir = TempDir::new("sim");
        let newest = bundles
            .iter()
            .max_by_key(|bundle| bundle.steps().len())
            .expect("a client")
            .clone();
        let clients = bundles
            .into_iter()
            .enumerate()
            .map(|(index, bundle)| {
                let config = Config {
                    seed: rng.bytes16(),
                    create: Some(DatabaseId(rng.bytes16())),
                    probe_window: 1 + u32::try_from(rng.below(6)).unwrap(),
                    checkpoint: CheckpointPolicy {
                        every: 4 + rng.below(8),
                        keep: 2,
                    },
                    max_entry_bytes: 4096,
                };
                let model = Model::new(bundle, &dir.path().join(format!("client{index}")));
                Client {
                    machine: None,
                    parked: Some(model),
                    incarnation: 0,
                    config,
                }
            })
            .collect();
        Self {
            rng,
            faults,
            now: 1_000_000,
            log: BTreeMap::new(),
            checkpoints: BTreeMap::new(),
            clients,
            asks: BTreeMap::new(),
            results: BTreeMap::new(),
            abandoned: BTreeSet::new(),
            commands: BTreeMap::new(),
            reused: BTreeMap::new(),
            _dir: dir,
            bundle: newest,
            issued: Vec::new(),
            delayed: Vec::new(),
            next_ticket: 0,
            operations: 0,
            tally: Tally::default(),
        }
    }

    pub fn ticket(&mut self, client: usize, ask: Ask) -> Ticket {
        self.next_ticket += 1;
        let ticket = Ticket(self.next_ticket);
        self.asks.insert(ticket, (client, ask));
        ticket
    }

    /// Start (or restart) a client's machine and open it.
    pub fn start(&mut self, client: usize) {
        let state = &mut self.clients[client];
        let Some(model) = state.parked.take() else {
            return;
        };
        state.incarnation += 1;
        state.machine = Some(Machine::new(model, state.config.clone()));
        let ticket = self.ticket(client, Ask::Open);
        self.input(client, Input::Open(ticket));
    }

    /// Kill a client's process: its requests may still land, its responses
    /// are lost, and its cache survives unless `wipe`.
    pub fn kill(&mut self, client: usize, wipe: bool) {
        let state = &mut self.clients[client];
        let Some(machine) = state.machine.take() else {
            return;
        };
        self.tally.kills += 1;
        let mut model = machine.into_replica();
        if wipe {
            model.wipe();
        }
        state.parked = Some(model);
        for (ticket, (owner, _)) in &self.asks {
            if *owner == client && !self.results.contains_key(ticket) {
                self.abandoned.insert(*ticket);
            }
        }
    }

    pub fn alive(&self, client: usize) -> bool {
        self.clients[client].machine.is_some()
    }

    pub fn machine(&self, client: usize) -> &Machine<Model> {
        self.clients[client]
            .machine
            .as_ref()
            .expect("a live client")
    }

    pub fn input(&mut self, client: usize, input: Input) {
        let state = &mut self.clients[client];
        let incarnation = state.incarnation;
        let machine = state.machine.as_mut().expect("a live client");
        let step = machine.step(input);
        let head = machine.replica().head().map(|head| head.seq);
        for request in step.io {
            let body = match &request.op {
                Op::PutIfAbsent(IoBody::Bytes(bytes)) => Some(bytes.clone()),
                Op::PutIfAbsent(IoBody::File(path)) => {
                    Some(std::fs::read(path).expect("an issued PUT's file exists"))
                }
                _ => None,
            };
            assert!(
                request.bucket == Bucket::Checkpoints
                    || matches!(request.op, Op::Get(_) | Op::PutIfAbsent(_)),
                "the log bucket is only read and created: {request:?}"
            );
            self.issued.push(Issued {
                client,
                incarnation,
                request,
                body,
            });
        }
        for done in step.done {
            assert!(
                !self.abandoned.contains(&done.ticket),
                "an abandoned ticket settled"
            );
            let previous = self.results.insert(
                done.ticket,
                Result {
                    settled: done.settled,
                    head,
                },
            );
            assert!(previous.is_none(), "ticket {:?} settled twice", done.ticket);
        }
    }

    /// Submit the command for `request`, generating it on first use.
    pub fn submit(&mut self, client: usize, request: RequestId, command: Command) -> Ticket {
        let ticket = self.ticket(client, Ask::Submit(request));
        match self.commands.get(&request) {
            Some(existing) if existing.reference() != command.reference() => {
                self.reused.insert(ticket, command.clone());
            }
            Some(_) => {}
            None => {
                self.commands.insert(request, command.clone());
            }
        }
        self.input(client, Input::Submit(ticket, command));
        ticket
    }

    pub fn random_command(&mut self, client: usize, schema: &Schema) -> (RequestId, Command) {
        let reuse = !self.commands.is_empty() && self.rng.chance(20);
        let existing = if reuse {
            let pick = usize::try_from(self.rng.below(self.commands.len() as u64)).unwrap();
            let (request, command) = self.commands.iter().nth(pick).expect("picked");
            if self.rng.chance(75) {
                return (*request, command.clone());
            }
            Some(*request)
        } else {
            None
        };
        let request = existing.unwrap_or_else(|| RequestId(self.rng.bytes16()));
        let mut adds = Vec::new();
        let mut removes = Vec::new();
        for _ in 0..=self.rng.below(3) {
            let row = (self.rng.below(6), self.rng.below(3));
            if self.rng.chance(70) {
                adds.push(row);
            } else {
                removes.push(row);
            }
        }
        removes.retain(|row| !adds.contains(row));
        let changes = items(schema, &adds, &removes);
        let precondition = if self.rng.chance(25) {
            let revision = self.clients[client]
                .machine
                .as_ref()
                .and_then(|machine| machine.replica().head().map(|head| head.revision.0))
                .unwrap_or(0);
            Precondition::ExactRevision(Revision(revision + self.rng.below(2)))
        } else {
            Precondition::None
        };
        (request, Command::seal(request, precondition, changes))
    }

    /// Execute one issued request with a fate drawn from the faults.
    pub fn execute(&mut self, index: usize) {
        let is_put = matches!(self.issued[index].request.op, Op::PutIfAbsent(_));
        let lose = if is_put {
            self.faults.put_lost
        } else {
            self.faults.read_failed
        };
        let fate = if self.rng.chance(lose) {
            Fate::Lost
        } else if is_put && self.rng.chance(self.faults.put_unanswered) {
            Fate::Unanswered
        } else if self.rng.chance(self.faults.delayed) {
            Fate::Delayed
        } else {
            Fate::Answered
        };
        self.execute_as(index, fate);
    }

    /// Execute one issued request against the store with the given fate.
    pub fn execute_as(&mut self, index: usize, fate: Fate) {
        self.operations += 1;
        assert!(self.operations < 2_000_000, "the world does not quiesce");
        let Issued {
            client,
            incarnation,
            request,
            body,
        } = self.issued.remove(index);
        self.now += 1 + self.rng.below(3);
        let result = match fate {
            Fate::Lost => {
                self.tally.lost += 1;
                IoResult::Failed
            }
            Fate::Unanswered => {
                self.tally.unanswered += 1;
                self.apply(client, incarnation, &request, body);
                IoResult::Failed
            }
            Fate::Answered | Fate::Delayed => self.apply(client, incarnation, &request, body),
        };
        if result == IoResult::Occupied {
            self.tally.occupied += 1;
        }
        if fate == Fate::Delayed {
            self.tally.delayed += 1;
        }
        if matches!(&request.op, Op::Get(Target::File(_))) && result != IoResult::Missing {
            self.tally.installs += 1;
        }
        let delivery = Delivery {
            client,
            incarnation,
            response: IoResponse {
                id: request.id,
                date: Some(Millis(self.now)),
                result,
            },
        };
        if fate == Fate::Delayed {
            self.delayed.push(delivery);
        } else {
            self.deliver(delivery);
        }
    }

    /// The index of the first issued request of `client` matching `pick`.
    pub fn find(&self, client: usize, pick: impl Fn(&IoRequest) -> bool) -> Option<usize> {
        self.issued
            .iter()
            .position(|issued| issued.client == client && pick(&issued.request))
    }

    /// Issued requests not yet executed, with their clients.
    pub fn issued(&self) -> impl Iterator<Item = (usize, &IoRequest)> {
        self.issued
            .iter()
            .map(|issued| (issued.client, &issued.request))
    }

    /// Execute and deliver everything, fault-free, without restarting anyone.
    pub fn drain(&mut self) {
        while !self.issued.is_empty() || !self.delayed.is_empty() {
            if self.issued.is_empty() {
                self.deliver_one();
            } else {
                self.execute_as(0, Fate::Answered);
            }
        }
    }

    fn apply(
        &mut self,
        client: usize,
        incarnation: u32,
        request: &IoRequest,
        body: Option<Vec<u8>>,
    ) -> IoResult {
        let now = self.now;
        let bucket = match request.bucket {
            Bucket::Log => &mut self.log,
            Bucket::Checkpoints => &mut self.checkpoints,
        };
        match &request.op {
            Op::Get(target) => match bucket.get(&request.key) {
                None => IoResult::Missing,
                Some((bytes, created)) => match target {
                    Target::Memory => IoResult::Body {
                        bytes: bytes.clone(),
                        last_modified: Millis(*created),
                    },
                    Target::File(path) => {
                        if self.clients[client].incarnation == incarnation {
                            std::fs::write(path, bytes).expect("download");
                        }
                        IoResult::Saved {
                            last_modified: Millis(*created),
                        }
                    }
                },
            },
            Op::PutIfAbsent(_) => {
                if bucket.contains_key(&request.key) {
                    IoResult::Occupied
                } else {
                    bucket.insert(request.key.clone(), (body.expect("a PUT body"), now));
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
        }
    }

    fn deliver(&mut self, delivery: Delivery) {
        let Delivery {
            client,
            incarnation,
            response,
        } = delivery;
        if self.alive(client) && self.clients[client].incarnation == incarnation {
            self.input(client, Input::Response(response));
        }
    }

    pub fn deliver_one(&mut self) {
        if !self.delayed.is_empty() {
            let index = usize::try_from(self.rng.below(self.delayed.len() as u64)).unwrap();
            let delivery = self.delayed.remove(index);
            self.deliver(delivery);
        }
    }

    pub fn execute_one(&mut self) {
        if !self.issued.is_empty() {
            let index = usize::try_from(self.rng.below(self.issued.len() as u64)).unwrap();
            self.execute(index);
        }
    }

    /// Fault-free: restart every client, then run until no request is left.
    pub fn quiesce(&mut self) {
        self.faults = Faults::NONE;
        for client in 0..self.clients.len() {
            if !self.alive(client) {
                self.start(client);
            }
        }
        self.drain();
    }

    /// Run `input` on a client and drive the world until `ticket` settles.
    pub fn await_ticket(&mut self, client: usize, ticket: Ticket, input: Input) -> Settled {
        self.input(client, input);
        while !self.results.contains_key(&ticket) {
            assert!(
                !self.issued.is_empty() || !self.delayed.is_empty(),
                "ticket {ticket:?} waits on nothing"
            );
            if self.issued.is_empty() {
                self.deliver_one();
            } else {
                self.execute_as(0, Fate::Answered);
            }
        }
        self.results[&ticket].settled.clone()
    }

    pub fn sync(&mut self, client: usize) -> Settled {
        let ticket = self.ticket(client, Ask::Sync);
        self.await_ticket(client, ticket, Input::Sync(ticket))
    }

    pub fn bundle(&self) -> &bumbledb_log::Bundle {
        &self.bundle
    }

    pub fn log_entries(&self) -> Vec<Vec<u8>> {
        let mut entries = Vec::new();
        for (index, (key, (bytes, _))) in self.log.iter().enumerate() {
            let seq = Seq::new(index as u64 + 1).expect("nonzero");
            assert_eq!(
                *key,
                bumbledb_log::log_key(seq),
                "the log is a contiguous prefix"
            );
            entries.push(bytes.clone());
        }
        entries
    }

    pub fn log_modified(&self, seq: Seq) -> Millis {
        Millis(self.log[&bumbledb_log::log_key(seq)].1)
    }
}

/// The world's log replayed through a fresh reference model: the state after
/// every entry (index 0 is after Genesis). Each decision is re-derived from
/// the original commands against the state before it.
pub fn replay(world: &World) -> Vec<State> {
    let dir = TempDir::new("replay");
    let mut model = Model::new(world.bundle().clone(), dir.path());
    let mut states = Vec::new();
    for (index, bytes) in world.log_entries().iter().enumerate() {
        let seq = Seq::new(index as u64 + 1).expect("nonzero");
        let bundle = world.bundle();
        let schema = match model.head() {
            None => &bundle.initial().schema,
            Some(head) => &bundle.by_schema(head.schema).expect("bundled").schema,
        };
        let entry = Entry::parse(schema, bytes).expect("every log object is an entry");
        if let Body::Commands(decided) = &entry.body {
            redecide(world, &model, decided);
        }
        if let Body::Migration(migration) = &entry.body {
            let (image, _) = &world.checkpoints[&image_key(migration.image)];
            let path = dir.path().join("migration.img");
            std::fs::write(&path, image).expect("image");
            let expected =
                bumbledb_log::migrated_head(model.head().expect("a head"), seq, migration);
            model
                .install(&path, migration.image, migration.schema)
                .expect("a migration image installs");
            assert_eq!(
                model.head(),
                Some(&expected),
                "the image head is the folded head"
            );
        } else {
            let folded = bumbledb_log::fold(model.head(), seq, &entry, world.log_modified(seq))
                .expect("every entry follows its head");
            if seq == Seq::GENESIS {
                model.create(&folded.head).expect("create");
            } else {
                model
                    .apply(bumbledb_log::Update {
                        head: &folded.head,
                        commits: &folded.commits,
                        receipts: &folded.receipts,
                    })
                    .expect("a decided entry applies with its recorded deltas");
            }
        }
        states.push(model.state().expect("a state").clone());
    }
    states
}

/// Each decided command's verdict is what deciding the original command
/// against the state before it gives.
fn redecide(world: &World, model: &Model, decided: &[bumbledb_log::Decided]) {
    let head = model.head().expect("commands follow a head");
    let mut revision = head.revision;
    let mut accepted: Vec<ChangeSet> = Vec::new();
    for decided in decided {
        let request = decided.command.request;
        assert!(
            model.receipt(request).expect("receipt").is_none(),
            "request {request:?} decided twice"
        );
        let command = world
            .commands
            .get(&request)
            .filter(|command| command.reference() == decided.command)
            .or_else(|| {
                world
                    .reused
                    .values()
                    .find(|command| command.reference() == decided.command)
            })
            .expect("a decided command was submitted");
        let expected = match command.precondition() {
            Precondition::ExactRevision(expected) if expected != revision => {
                Outcome::PreconditionFailed {
                    expected,
                    observed: revision,
                }
            }
            _ => match model.judge(&accepted, command.changes()).expect("judge") {
                bumbledb_log::Judgment::Changed(delta) => {
                    revision = revision.next();
                    accepted.push(command.changes().clone());
                    Outcome::Committed(delta)
                }
                bumbledb_log::Judgment::Unchanged => Outcome::NoChange,
                bumbledb_log::Judgment::Rejected(evidence) => Outcome::InvariantRejected(evidence),
            },
        };
        assert_eq!(
            decided.verdict.outcome(),
            expected,
            "decision for {request:?}"
        );
        if let Verdict::Committed { changes, .. } = &decided.verdict {
            assert_eq!(changes.as_bytes(), command.changes().as_bytes());
        }
    }
}

/// The receipt of `request` in the final state, if any.
fn final_receipt(states: &[State], request: RequestId) -> Option<&Receipt> {
    states.last().and_then(|state| state.receipts.get(&request))
}

/// Every image object holds exactly the bytes its key names.
fn check_images(world: &World) {
    for (key, (bytes, _)) in &world.checkpoints {
        let digest = *blake3::hash(bytes).as_bytes();
        if let Some(checkpoint) = CheckpointKey::parse(key) {
            assert_eq!(checkpoint.digest.0, digest, "checkpoint {key}");
        } else {
            assert_eq!(
                *key,
                image_key(bumbledb_log::ImageDigest(digest)),
                "image {key}"
            );
        }
    }
}

/// Verify every settled ticket and every live replica against the replay.
pub fn check(world: &World) -> Vec<State> {
    check_images(world);
    let states = replay(world);
    let mut decided_or_unclear: BTreeSet<RequestId> = BTreeSet::new();
    for (ticket, (_, ask)) in &world.asks {
        let Some(result) = world.results.get(ticket) else {
            continue;
        };
        match (ask, &result.settled) {
            (Ask::Submit(request), Settled::Decided(receipt)) => {
                decided_or_unclear.insert(*request);
                assert_eq!(
                    final_receipt(&states, *request),
                    Some(receipt),
                    "a decided receipt is the log's"
                );
            }
            (Ask::Submit(request), Settled::Refused(Refusal::Unknown)) => {
                decided_or_unclear.insert(*request);
            }
            (Ask::Submit(request), Settled::Refused(Refusal::RequestReused(existing))) => {
                let submitted = std::iter::once(&world.commands[request])
                    .chain(world.reused.values())
                    .any(|command| command.reference() == *existing);
                assert!(submitted, "reuse names a submitted command for the request");
            }
            (Ask::Resolve(request), Settled::Resolved(found)) => {
                let head = result.head.expect("a resolved client has a head");
                let state = &states[usize::try_from(head.get()).unwrap() - 1];
                assert_eq!(
                    found.as_ref(),
                    state.receipts.get(request),
                    "resolve agrees"
                );
            }
            (Ask::Sync, Settled::Synced(seq)) => {
                assert!(usize::try_from(seq.get()).unwrap() <= states.len());
            }
            _ => {}
        }
    }
    for (ticket, (_, ask)) in &world.asks {
        let unsettled = world.abandoned.contains(ticket) || !world.results.contains_key(ticket);
        if let (true, Ask::Submit(request)) = (unsettled, ask) {
            decided_or_unclear.insert(*request);
        }
    }
    for (ticket, command) in &world.reused {
        if world.results.get(ticket).is_none_or(|result| {
            matches!(
                result.settled,
                Settled::Decided(_) | Settled::Refused(Refusal::Unknown)
            )
        }) {
            decided_or_unclear.insert(command.request());
        }
    }
    if let Some(last) = states.last() {
        for request in last.receipts.keys() {
            assert!(
                decided_or_unclear.contains(request),
                "{request:?} is in the log though every submission was refused"
            );
        }
    }
    for client in 0..world.clients.len() {
        if !world.alive(client) {
            continue;
        }
        let Some(state) = world.machine(client).replica().state() else {
            continue;
        };
        let index = usize::try_from(state.head.seq.get()).unwrap() - 1;
        assert_eq!(
            *state, states[index],
            "client {client} equals the replay at its head"
        );
    }
    states
}

/// Checkpoint keys in the store, newest first.
pub fn checkpoint_keys(world: &World) -> Vec<CheckpointKey> {
    world
        .checkpoints
        .keys()
        .filter_map(|key| CheckpointKey::parse(key))
        .collect()
}

pub fn population(step: bumbledb_log::MigrationId, base: Seq, rows: ChangeSet) -> Population {
    Population {
        step,
        base,
        copy: vec![(crate::support::ITEM, crate::support::ITEM)].into(),
        rows,
    }
}

pub fn receipt_of(settled: &Settled) -> &Receipt {
    match settled {
        Settled::Decided(receipt) => receipt,
        other => panic!("expected a decision, got {other:?}"),
    }
}
