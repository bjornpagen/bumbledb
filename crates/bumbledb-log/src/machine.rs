//! The sans-IO protocol: [`Machine::step`] turns one [`Input`] into store
//! requests and settled tickets. It owns every invariant: one entry write in
//! flight, byte comparison of every unclear write, receipts before decisions,
//! freeze deadlines on store time, checkpoint cadence. It never sleeps or
//! reads a clock; time arrives only as the store's `Date` and `Last-Modified`.

use std::collections::{BTreeMap, VecDeque};

use crate::command::{Command, CommandRef, Precondition};
use crate::entry::{Body, Decided, Entry, Freeze, Genesis, Migration, Thaw, Verdict};
use crate::fold::{Folded, fold, migrated_head};
use crate::head::{Comparison, Mode, Rejection};
use crate::ids::{DatabaseId, Millis, Nonce, RequestId, Seq};
use crate::io::{
    Body as IoBody, Bucket, CHECKPOINT_PREFIX, CheckpointKey, IoId, IoRequest, IoResponse,
    IoResult, Op, Target, image_key, log_key,
};
use crate::receipt::Receipt;
use crate::replica::{CacheError, Image, Judgment, Migrated, Population, Replica, Update};

const NONCE_CONTEXT: &str = "bdb.entry.v1 nonce";
/// Encoded bytes of one decided command besides its changes.
const DECIDED_OVERHEAD: usize = 128;

#[derive(Debug, Clone)]
pub struct Config {
    /// Random per process: entry nonces derive from it.
    pub seed: [u8; 16],
    /// Create the database with this identity when its log is empty.
    pub create: Option<DatabaseId>,
    /// Most tail objects fetched at once while catching up.
    pub probe_window: u32,
    pub checkpoint: CheckpointPolicy,
    /// Largest entry the machine writes; a bigger command is refused.
    pub max_entry_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckpointPolicy {
    /// Write a checkpoint once the head is this many entries past the last.
    pub every: u64,
    /// Checkpoints kept when pruning older ones.
    pub keep: u32,
}

/// The caller's handle on one request; settled exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ticket(pub u64);

#[derive(Debug)]
pub enum Input {
    /// Hydrate and catch up; settles [`Settled::Opened`].
    Open(Ticket),
    Submit(Ticket, Command),
    /// Settle once the head reflects every entry that existed when this
    /// input arrived.
    Sync(Ticket),
    /// Sync, then report the request's receipt.
    Resolve(Ticket, RequestId),
    /// Freeze the database for the next pending migration, for `lease_millis`.
    Freeze(Ticket, u64),
    Migrate(Ticket, Population),
    Response(IoResponse),
    /// Settle everything pending; later inputs are refused.
    Close,
}

#[derive(Debug, Default)]
pub struct Step {
    pub io: Vec<IoRequest>,
    pub done: Vec<Done>,
}

#[derive(Debug)]
pub struct Done {
    pub ticket: Ticket,
    pub settled: Settled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// Open and caught up, with `pending` bundled migrations not yet applied.
    Opened {
        pending: usize,
    },
    Decided(Receipt),
    Synced(Seq),
    /// `None`: the request is not decided at the synced head.
    Resolved(Option<Receipt>),
    Frozen(Seq),
    Migrated(Seq),
    Refused(Refusal),
}

/// Why a request settled without the effect it asked for. Every refusal of
/// a submit except [`Refusal::Unknown`] proves the command is not in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NotOpen,
    Closed,
    /// The log is empty and the machine may not create the database.
    NotFound,
    /// A migration holds the database until `deadline` (store time).
    Frozen {
        deadline: Millis,
    },
    /// The log applied a migration this code does not bundle.
    SchemaAdvanced,
    /// Writes wait until the bundled migration at `next` is applied.
    MigrationPending {
        next: usize,
    },
    MigrationRejected(Rejection),
    /// The ledger and the bundle disagree at this bundle index.
    MigrationsDiverged {
        index: usize,
    },
    /// The migration is not the next pending one.
    NotPending,
    /// A population or freeze computed at an older head; recompute at `head`.
    Stale {
        head: Seq,
    },
    /// The request was decided for a different command.
    RequestReused(CommandRef),
    /// The command's changes belong to another schema than the head's.
    ForeignSchema,
    TooLarge,
    /// An entry carrying the command was written but its fate is unresolved;
    /// resolve the request later.
    Unknown,
    NotSubmitted,
    Cache(CacheError),
    /// The object at this position is not an entry this database can follow.
    Corrupt(Seq),
}

pub struct Machine<R> {
    replica: R,
    config: Config,
    life: Life,
    next_io: u64,
    nonces: u64,
    /// Newest store time seen.
    now: Option<Millis>,
    /// Bumped whenever someone needs evidence fresher than every GET issued so
    /// far; a GET remembers the epoch it was issued at.
    epoch: u64,
    io: BTreeMap<IoId, Purpose>,
    opens: Vec<Ticket>,
    waiters: Vec<Waiter>,
    tail: Tail,
    install: Option<Installing>,
    writer: Writer,
    queue: Vec<Submission>,
    controls: VecDeque<Control>,
    clock: Clock,
    checkpoints: Checkpoints,
    out: Step,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Life {
    Fresh,
    Opening,
    Open,
    /// Stopped before an entry that applies an unbundled migration.
    Advanced,
    /// Stopped by a local failure or a log it cannot follow.
    Broken(Refusal),
    Closed,
}

#[derive(Debug, Clone)]
enum Purpose {
    Slot { seq: Seq, issued: u64 },
    Put,
    Upload,
    List,
    Fetch,
    Checkpoint(CheckpointKey),
    Prune(CheckpointKey),
    Delete,
}

struct Waiter {
    need: u64,
    wait: Wait,
}

enum Wait {
    Open,
    Sync(Ticket),
    Resolve(Ticket, RequestId),
    Clock,
}

#[derive(Default)]
struct Tail {
    found: BTreeMap<Seq, (Vec<u8>, Millis)>,
    /// A slot seen empty, with the newest epoch a GET saw it empty at.
    missing: BTreeMap<Seq, u64>,
    /// Entries found since the tail was last seen empty; widens the window.
    streak: u32,
}

enum Installing {
    Checkpoint {
        io: IoId,
        key: CheckpointKey,
    },
    Migration {
        io: IoId,
        seq: Seq,
        entry: Box<Entry>,
        lost: Option<Box<Flight>>,
    },
}

enum Writer {
    Idle,
    InFlight(Box<Flight>),
}

struct Flight {
    slot: Seq,
    bytes: Vec<u8>,
    entry: Entry,
    cargo: Cargo,
    state: FlightState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlightState {
    /// The migration image is uploading; no entry PUT was issued yet.
    Uploading(IoId),
    Putting(IoId),
    /// A write came back unclear; any slot GET issued at or after `need`
    /// decides it.
    Verifying {
        need: u64,
    },
}

enum Cargo {
    Genesis,
    Commands(Vec<Submission>),
    Freeze(Ticket),
    Migration(Ticket, Image),
    Thaw(Option<Ticket>),
}

struct Submission {
    tickets: Vec<Ticket>,
    command: Command,
}

enum Control {
    Freeze(Ticket, u64),
    Migrate(Ticket, Population),
}

/// Whether queued commands facing a freeze have seen store time newer than
/// their arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    Unchecked,
    Waiting,
    Fresh,
}

#[derive(Default)]
struct Checkpoints {
    last: Option<Seq>,
    /// The opening LIST answered, so `last` reflects the store.
    known: bool,
    uploading: bool,
}

impl<R: Replica> Machine<R> {
    #[must_use]
    pub fn new(replica: R, config: Config) -> Self {
        Self {
            replica,
            config,
            life: Life::Fresh,
            next_io: 0,
            nonces: 0,
            now: None,
            epoch: 0,
            io: BTreeMap::new(),
            opens: Vec::new(),
            waiters: Vec::new(),
            tail: Tail::default(),
            install: None,
            writer: Writer::Idle,
            queue: Vec::new(),
            controls: VecDeque::new(),
            clock: Clock::Unchecked,
            checkpoints: Checkpoints::default(),
            out: Step::default(),
        }
    }

    #[must_use]
    pub fn replica(&self) -> &R {
        &self.replica
    }

    /// The replica back, for a later machine. Requests this machine issued
    /// must be abandoned first: their responses mean nothing to another.
    #[must_use]
    pub fn into_replica(self) -> R {
        self.replica
    }

    pub fn step(&mut self, input: Input) -> Step {
        match input {
            Input::Open(ticket) => self.open(ticket),
            Input::Submit(ticket, command) => self.submit(ticket, command),
            Input::Sync(ticket) => self.wait(ticket, Wait::Sync(ticket)),
            Input::Resolve(ticket, request) => self.wait(ticket, Wait::Resolve(ticket, request)),
            Input::Freeze(ticket, lease) => self.control(ticket, Control::Freeze(ticket, lease)),
            Input::Migrate(ticket, population) => {
                self.control(ticket, Control::Migrate(ticket, population));
            }
            Input::Response(response) => self.respond(response),
            Input::Close => self.halt(Life::Closed, &Refusal::Closed),
        }
        self.drive();
        std::mem::take(&mut self.out)
    }

    fn drive(&mut self) {
        self.advance();
        self.plan();
        self.refill();
    }

    fn refusal(&self) -> Option<Refusal> {
        match &self.life {
            Life::Fresh => Some(Refusal::NotOpen),
            Life::Opening | Life::Open => None,
            Life::Advanced => Some(Refusal::SchemaAdvanced),
            Life::Broken(refusal) => Some(refusal.clone()),
            Life::Closed => Some(Refusal::Closed),
        }
    }

    fn open(&mut self, ticket: Ticket) {
        match &self.life {
            Life::Fresh => {
                self.life = Life::Opening;
                self.opens.push(ticket);
                self.request(
                    Bucket::Checkpoints,
                    CHECKPOINT_PREFIX.to_owned(),
                    Op::List {
                        start_after: None,
                        max_keys: 1,
                    },
                    Purpose::List,
                );
                if self.replica.head().is_some() {
                    self.register(Wait::Open);
                }
            }
            Life::Opening | Life::Open => {
                self.opens.push(ticket);
                self.register(Wait::Open);
            }
            _ => {
                let refusal = self.refusal().expect("a halted machine refuses");
                self.settle(ticket, Settled::Refused(refusal));
            }
        }
    }

    fn submit(&mut self, ticket: Ticket, command: Command) {
        if let Some(refusal) = self.refusal() {
            return self.settle(ticket, Settled::Refused(refusal));
        }
        if command.changes().as_bytes().len() + DECIDED_OVERHEAD > self.config.max_entry_bytes {
            return self.settle(ticket, Settled::Refused(Refusal::TooLarge));
        }
        let reference = command.reference();
        let in_flight = match &mut self.writer {
            Writer::InFlight(flight) => match &mut flight.cargo {
                Cargo::Commands(batch) => Some(batch),
                _ => None,
            },
            Writer::Idle => None,
        };
        let pending = self
            .queue
            .iter_mut()
            .chain(in_flight.into_iter().flatten())
            .find(|pending| pending.command.request() == reference.request);
        match pending {
            Some(pending) if pending.command.reference() == reference => {
                pending.tickets.push(ticket);
            }
            Some(pending) => {
                let existing = pending.command.reference();
                self.settle(ticket, Settled::Refused(Refusal::RequestReused(existing)));
            }
            None => self.queue.push(Submission {
                tickets: vec![ticket],
                command,
            }),
        }
    }

    fn wait(&mut self, ticket: Ticket, wait: Wait) {
        match self.refusal() {
            Some(refusal) => self.settle(ticket, Settled::Refused(refusal)),
            None => self.register(wait),
        }
    }

    fn control(&mut self, ticket: Ticket, control: Control) {
        match self.refusal() {
            Some(refusal) => self.settle(ticket, Settled::Refused(refusal)),
            None => self.controls.push_back(control),
        }
    }

    fn register(&mut self, wait: Wait) {
        self.epoch += 1;
        self.waiters.push(Waiter {
            need: self.epoch,
            wait,
        });
    }

    fn respond(&mut self, response: IoResponse) {
        if let Some(date) = response.date {
            self.now = Some(self.now.map_or(date, |now| now.max(date)));
        }
        let Some(purpose) = self.io.remove(&response.id) else {
            return;
        };
        let id = response.id;
        let date = response.date;
        match (purpose, response.result) {
            (
                Purpose::Slot { seq, .. },
                IoResult::Body {
                    bytes,
                    last_modified,
                },
            ) => {
                if seq >= self.next_slot() {
                    self.tail.found.insert(seq, (bytes, last_modified));
                    self.tail.streak = self.tail.streak.saturating_add(1);
                }
            }
            (Purpose::Slot { seq, issued }, IoResult::Missing) => {
                let seen = self.tail.missing.entry(seq).or_insert(issued);
                *seen = (*seen).max(issued);
            }
            (Purpose::Put, result) => self.put_returned(id, result, date),
            (Purpose::Upload, result) => self.upload_returned(id, &result),
            (Purpose::List, IoResult::Keys(keys)) => self.listed(&keys),
            (Purpose::List, _) => self.listed(&[]),
            (Purpose::Fetch, result) => self.fetched(id, result),
            (Purpose::Checkpoint(key), IoResult::Created | IoResult::Occupied) => {
                self.checkpoints.uploading = false;
                self.checkpoints.last = self.checkpoints.last.max(Some(key.seq));
                self.request(
                    Bucket::Checkpoints,
                    CHECKPOINT_PREFIX.to_owned(),
                    Op::List {
                        start_after: Some(key.format()),
                        max_keys: 1000,
                    },
                    Purpose::Prune(key),
                );
            }
            (Purpose::Checkpoint(_), _) => self.checkpoints.uploading = false,
            (Purpose::Prune(newest), IoResult::Keys(keys)) => self.prune(newest, &keys),
            (Purpose::Slot { .. } | Purpose::Prune(_) | Purpose::Delete, _) => {}
        }
    }

    fn put_returned(&mut self, id: IoId, result: IoResult, date: Option<Millis>) {
        let Writer::InFlight(flight) = &mut self.writer else {
            return;
        };
        if flight.state != FlightState::Putting(id) {
            return;
        }
        if result == IoResult::Created {
            let at = date.or(self.now).unwrap_or(Millis(0));
            let bytes = flight.bytes.clone();
            let slot = flight.slot;
            self.land(slot, bytes, at);
        } else {
            self.epoch += 1;
            flight.state = FlightState::Verifying { need: self.epoch };
        }
    }

    fn upload_returned(&mut self, id: IoId, result: &IoResult) {
        let Writer::InFlight(flight) = &self.writer else {
            return;
        };
        if flight.state != FlightState::Uploading(id) {
            return;
        }
        let Cargo::Migration(_, image) = &flight.cargo else {
            return;
        };
        let image = image.clone();
        match result {
            IoResult::Created | IoResult::Occupied => self.put_entry(),
            _ => {
                let id = self.request(
                    Bucket::Checkpoints,
                    image_key(image.digest),
                    Op::PutIfAbsent(IoBody::File(image.path)),
                    Purpose::Upload,
                );
                if let Writer::InFlight(flight) = &mut self.writer {
                    flight.state = FlightState::Uploading(id);
                }
            }
        }
    }

    fn listed(&mut self, keys: &[String]) {
        let newest = keys.iter().find_map(|key| CheckpointKey::parse(key));
        if let Some(key) = newest {
            self.checkpoints.last = self.checkpoints.last.max(Some(key.seq));
        }
        self.checkpoints.known = true;
        if self.life != Life::Opening {
            return;
        }
        let head = self.replica.head().map(|head| head.seq);
        let every = self.config.checkpoint.every;
        let worth = newest.filter(|key| {
            self.replica.bundle().by_schema(key.schema).is_some()
                && head.is_none_or(|head| key.seq.get() >= head.get().saturating_add(every))
        });
        if let Some(key) = worth
            && self.install.is_none()
        {
            let io = self.request(
                Bucket::Checkpoints,
                key.format(),
                Op::Get(Target::File(self.replica.download_path())),
                Purpose::Fetch,
            );
            self.install = Some(Installing::Checkpoint { io, key });
        }
        if head.is_none() {
            self.register(Wait::Open);
        }
    }

    fn fetched(&mut self, id: IoId, result: IoResult) {
        match self.install.take() {
            Some(Installing::Checkpoint { io, key }) if io == id => {
                if matches!(result, IoResult::Saved { .. }) {
                    self.install_checkpoint(key);
                }
            }
            Some(Installing::Migration {
                io,
                seq,
                entry,
                lost,
            }) if io == id => match result {
                IoResult::Saved { .. } => self.install_migration(seq, &entry, lost),
                IoResult::Missing => self.break_down(Refusal::Corrupt(seq)),
                _ => {
                    let Body::Migration(migration) = &entry.body else {
                        unreachable!("a migration install holds a migration entry")
                    };
                    let io = self.request(
                        Bucket::Checkpoints,
                        image_key(migration.image),
                        Op::Get(Target::File(self.replica.download_path())),
                        Purpose::Fetch,
                    );
                    self.install = Some(Installing::Migration {
                        io,
                        seq,
                        entry,
                        lost,
                    });
                }
            },
            other => self.install = other,
        }
    }

    fn install_checkpoint(&mut self, key: CheckpointKey) {
        if self.replica.head().is_some_and(|head| head.seq >= key.seq) {
            return;
        }
        let database = self.replica.head().map(|head| head.database);
        let path = self.replica.download_path();
        match self.replica.install(&path, key.digest, key.schema) {
            Ok(()) => {
                let head = self.replica.head().expect("an installed image has a head");
                if head.seq != key.seq
                    || head.schema != key.schema
                    || database.is_some_and(|database| database != head.database)
                {
                    return self.break_down(Refusal::Corrupt(key.seq));
                }
                self.tail = Tail::default();
            }
            Err(CacheError::Digest | CacheError::UnknownSchema(_)) => {}
            Err(error) => self.break_down(Refusal::Cache(error)),
        }
    }

    fn install_migration(&mut self, seq: Seq, entry: &Entry, lost: Option<Box<Flight>>) {
        let Body::Migration(migration) = &entry.body else {
            unreachable!("a migration install holds a migration entry")
        };
        let head = self.replica.head().expect("a migration follows a head");
        let expected = migrated_head(head, seq, &migration.migration, migration.schema);
        let path = self.replica.download_path();
        match self
            .replica
            .install(&path, migration.image, migration.schema)
        {
            Ok(()) if self.replica.head() == Some(&expected) => {
                self.after_land();
                if let Some(flight) = lost {
                    self.lost(flight);
                }
            }
            Ok(()) | Err(CacheError::Digest) => self.break_down(Refusal::Corrupt(seq)),
            Err(error) => self.break_down(Refusal::Cache(error)),
        }
    }

    fn prune(&mut self, newest: CheckpointKey, keys: &[String]) {
        let older = keys
            .iter()
            .filter(|key| CheckpointKey::parse(key).is_some_and(|key| key.seq < newest.seq));
        let keep = usize::try_from(self.config.checkpoint.keep.saturating_sub(1)).unwrap_or(0);
        for key in older.skip(keep) {
            self.request(
                Bucket::Checkpoints,
                key.clone(),
                Op::Delete,
                Purpose::Delete,
            );
        }
    }

    fn next_slot(&self) -> Seq {
        self.replica
            .head()
            .map_or(Seq::GENESIS, |head| head.seq.next())
    }

    /// Apply every known entry at the next slot; act on a known-empty slot.
    fn advance(&mut self) {
        while self.install.is_none() && matches!(self.life, Life::Opening | Life::Open) {
            let next = self.next_slot();
            if let Some((bytes, at)) = self.tail.found.remove(&next) {
                self.land(next, bytes, at);
                continue;
            }
            if let Some(issued) = self.tail.missing.remove(&next) {
                self.tail.streak = 0;
                self.empty(issued);
            }
            break;
        }
        let next = self.next_slot();
        self.tail.found.retain(|seq, _| *seq >= next);
        self.tail.missing.retain(|seq, _| *seq >= next);
    }

    /// The next slot was empty for a GET issued at `issued`.
    fn empty(&mut self, issued: u64) {
        if let Writer::InFlight(flight) = &mut self.writer
            && let FlightState::Verifying { need } = flight.state
            && need <= issued
        {
            self.put_entry();
        }
        let (ready, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiters)
            .into_iter()
            .partition(|waiter| waiter.need <= issued);
        self.waiters = waiting;
        for waiter in ready {
            match waiter.wait {
                Wait::Open => self.opened(),
                Wait::Sync(ticket) => match self.replica.head() {
                    Some(head) => {
                        let seq = head.seq;
                        self.settle(ticket, Settled::Synced(seq));
                    }
                    None => self.settle(ticket, Settled::Refused(Refusal::NotFound)),
                },
                Wait::Resolve(ticket, _) if self.replica.head().is_none() => {
                    self.settle(ticket, Settled::Refused(Refusal::NotFound));
                }
                Wait::Resolve(ticket, request) => match self.replica.receipt(request) {
                    Ok(receipt) => self.settle(ticket, Settled::Resolved(receipt)),
                    Err(error) => self.settle(ticket, Settled::Refused(Refusal::Cache(error))),
                },
                Wait::Clock => self.clock = Clock::Fresh,
            }
        }
    }

    /// The content of the next slot is known: apply it.
    fn land(&mut self, seq: Seq, bytes: Vec<u8>, at: Millis) {
        debug_assert_eq!(seq, self.next_slot());
        let ours = matches!(&self.writer, Writer::InFlight(flight)
            if flight.slot == seq && flight.bytes == bytes);
        if ours {
            let Writer::InFlight(flight) = std::mem::replace(&mut self.writer, Writer::Idle) else {
                unreachable!()
            };
            return self.landed(flight, at);
        }
        let lost = match std::mem::replace(&mut self.writer, Writer::Idle) {
            Writer::InFlight(flight) if flight.slot == seq => Some(flight),
            writer => {
                self.writer = writer;
                None
            }
        };
        let bundle = self.replica.bundle();
        let schema = match self.replica.head() {
            None => Some(&bundle.initial().schema),
            Some(head) => bundle.by_schema(head.schema).map(|step| &step.schema),
        };
        let Some(entry) = schema.and_then(|schema| Entry::parse(schema, &bytes).ok()) else {
            return self.break_down(Refusal::Corrupt(seq));
        };
        if let Body::Migration(migration) = &entry.body {
            if self.replica.head().is_none() {
                return self.break_down(Refusal::Corrupt(seq));
            }
            if self.replica.bundle().by_schema(migration.schema).is_none() {
                if let Some(flight) = lost {
                    self.abandon(flight, &Refusal::SchemaAdvanced);
                }
                return self.halt(Life::Advanced, &Refusal::SchemaAdvanced);
            }
            let io = self.request(
                Bucket::Checkpoints,
                image_key(migration.image),
                Op::Get(Target::File(self.replica.download_path())),
                Purpose::Fetch,
            );
            self.install = Some(Installing::Migration {
                io,
                seq,
                entry: Box::new(entry),
                lost,
            });
            return;
        }
        if self.apply(seq, &entry, at).is_some() {
            self.after_land();
            if let Some(flight) = lost {
                self.lost(flight);
            }
        }
    }

    /// Fold and apply a non-migration entry; `None` when the machine broke.
    fn apply(&mut self, seq: Seq, entry: &Entry, at: Millis) -> Option<Vec<Receipt>> {
        let head = self.replica.head();
        let Ok(Folded {
            head,
            receipts,
            commits,
        }) = fold(head, seq, entry, at)
        else {
            self.break_down(Refusal::Corrupt(seq));
            return None;
        };
        let applied = if seq == Seq::GENESIS {
            self.replica.create(&head)
        } else {
            self.replica.apply(Update {
                head: &head,
                commits: &commits,
                receipts: &receipts,
            })
        };
        match applied {
            Ok(()) => Some(receipts),
            Err(error) => {
                self.break_down(Refusal::Cache(error));
                None
            }
        }
    }

    /// Our own entry landed at its slot.
    fn landed(&mut self, flight: Box<Flight>, at: Millis) {
        let seq = flight.slot;
        if let Cargo::Migration(ticket, image) = &flight.cargo {
            let Body::Migration(migration) = &flight.entry.body else {
                unreachable!("migration cargo rides a migration entry")
            };
            let head = self.replica.head().expect("a migration follows a head");
            let expected = migrated_head(head, seq, &migration.migration, migration.schema);
            let ticket = *ticket;
            self.settle(ticket, Settled::Migrated(seq));
            match self
                .replica
                .install(&image.path, migration.image, migration.schema)
            {
                Ok(()) if self.replica.head() == Some(&expected) => self.after_land(),
                Ok(()) => self.break_down(Refusal::Corrupt(seq)),
                Err(error) => self.break_down(Refusal::Cache(error)),
            }
            return;
        }
        let head_before = self.replica.head().cloned();
        let receipts = match fold(head_before.as_ref(), seq, &flight.entry, at) {
            Ok(folded) => folded.receipts,
            Err(_) => return self.break_down(Refusal::Corrupt(seq)),
        };
        match flight.cargo {
            Cargo::Genesis | Cargo::Migration(..) => {}
            Cargo::Commands(batch) => {
                for submission in batch {
                    let request = submission.command.request();
                    let receipt = receipts
                        .iter()
                        .find(|receipt| receipt.command.request == request)
                        .expect("every batched command is decided in its entry");
                    for ticket in submission.tickets {
                        self.settle(ticket, Settled::Decided(receipt.clone()));
                    }
                }
            }
            Cargo::Freeze(ticket) => self.settle(ticket, Settled::Frozen(seq)),
            Cargo::Thaw(ticket) => {
                if let (Some(ticket), Body::Thaw(Thaw::Rejected(rejection))) =
                    (ticket, &flight.entry.body)
                {
                    let refusal = Refusal::MigrationRejected(rejection.clone());
                    self.settle(ticket, Settled::Refused(refusal));
                }
            }
        }
        if self.apply(seq, &flight.entry, at).is_some() {
            self.after_land();
            if seq == Seq::GENESIS {
                self.opened();
            }
        }
    }

    /// A foreign entry took our slot; nothing of ours landed.
    fn lost(&mut self, flight: Box<Flight>) {
        let head = self.replica.head().map(|head| head.seq);
        let stale = Refusal::Stale {
            head: head.expect("a lost slot follows a head"),
        };
        match flight.cargo {
            Cargo::Genesis => self.opened(),
            Cargo::Commands(batch) => {
                let queued = std::mem::replace(&mut self.queue, batch);
                self.queue.extend(queued);
            }
            Cargo::Freeze(ticket) | Cargo::Migration(ticket, _) | Cargo::Thaw(Some(ticket)) => {
                self.settle(ticket, Settled::Refused(stale));
            }
            Cargo::Thaw(None) => {}
        }
    }

    fn after_land(&mut self) {
        if self.life == Life::Open {
            self.maybe_checkpoint();
        }
    }

    /// Ask for the slot GETs the waiters and the writer need.
    fn refill(&mut self) {
        if self.install.is_some() || !matches!(self.life, Life::Opening | Life::Open) {
            return;
        }
        let verifying = match &self.writer {
            Writer::InFlight(flight) => match flight.state {
                FlightState::Verifying { need } => Some(need),
                _ => None,
            },
            Writer::Idle => None,
        };
        let need = self
            .waiters
            .iter()
            .map(|waiter| waiter.need)
            .chain(verifying)
            .max();
        let Some(need) = need else {
            return;
        };
        let next = self.next_slot();
        let mut outstanding = BTreeMap::new();
        for purpose in self.io.values() {
            if let Purpose::Slot { seq, issued } = purpose {
                let newest = outstanding.entry(*seq).or_insert(*issued);
                *newest = (*newest).max(*issued);
            }
        }
        if outstanding.get(&next).is_none_or(|issued| *issued < need) {
            self.get_slot(next);
        }
        let window = self
            .config
            .probe_window
            .min(self.tail.streak.saturating_add(1));
        let mut seq = next;
        for _ in 1..window {
            seq = seq.next();
            if !outstanding.contains_key(&seq)
                && !self.tail.found.contains_key(&seq)
                && !self.tail.missing.contains_key(&seq)
            {
                self.get_slot(seq);
            }
        }
    }

    fn get_slot(&mut self, seq: Seq) {
        let issued = self.epoch;
        self.request(
            Bucket::Log,
            log_key(seq),
            Op::Get(Target::Memory),
            Purpose::Slot { seq, issued },
        );
    }

    /// Caught up during opening, or after a re-open.
    fn opened(&mut self) {
        if self.life == Life::Opening {
            if self.replica.head().is_none() {
                if let Some(database) = self.config.create {
                    if matches!(self.writer, Writer::Idle) {
                        let initial = self.replica.bundle().initial();
                        let body = Body::Genesis(Genesis {
                            database,
                            initial: initial.id.clone(),
                            schema: initial.fingerprint,
                        });
                        self.launch(body, Cargo::Genesis);
                    }
                } else {
                    self.halt(Life::Fresh, &Refusal::NotFound);
                }
                return;
            }
            self.life = Life::Open;
            self.maybe_checkpoint();
        }
        let settled = match self.gate() {
            Ok(()) => Settled::Opened { pending: 0 },
            Err(Refusal::MigrationPending { next }) => Settled::Opened {
                pending: self.replica.bundle().steps().len() - next,
            },
            Err(refusal) => Settled::Refused(refusal),
        };
        for ticket in std::mem::take(&mut self.opens) {
            self.settle(ticket, settled.clone());
        }
    }

    /// Whether this code may write commands at the head.
    fn gate(&self) -> Result<(), Refusal> {
        if self.life == Life::Advanced {
            return Err(Refusal::SchemaAdvanced);
        }
        let head = self.replica.head().ok_or(Refusal::NotFound)?;
        let bundled = self.replica.bundle().ids();
        if let Some(rejection) = head.ledger.rejection_of(&bundled) {
            return Err(Refusal::MigrationRejected(rejection.clone()));
        }
        match head.ledger.compare(&bundled) {
            Comparison::Equal => Ok(()),
            Comparison::Behind { next } => Err(Refusal::MigrationPending { next }),
            Comparison::Ahead => Err(Refusal::SchemaAdvanced),
            Comparison::Diverged { index } => Err(Refusal::MigrationsDiverged { index }),
        }
    }

    /// Start the next write when the writer is idle and the head is current.
    fn plan(&mut self) {
        while self.life == Life::Open
            && self.install.is_none()
            && matches!(self.writer, Writer::Idle)
        {
            if let Some(control) = self.controls.front() {
                // A migration image may still be uploading for a lost flight;
                // the next migration must not overwrite it meanwhile.
                let uploading = self
                    .io
                    .values()
                    .any(|purpose| matches!(purpose, Purpose::Upload));
                if uploading && matches!(control, Control::Migrate(..)) {
                    return;
                }
                let control = self.controls.pop_front().expect("a front control");
                self.start(control);
                continue;
            }
            if self.queue.is_empty() {
                return;
            }
            if let Err(refusal) = self.gate() {
                for submission in std::mem::take(&mut self.queue) {
                    for ticket in submission.tickets {
                        self.settle(ticket, Settled::Refused(refusal.clone()));
                    }
                }
                return;
            }
            let head = self.replica.head().expect("an open machine has a head");
            if let Some(deadline) = head.mode.deadline() {
                if self.now.is_some_and(|now| now >= deadline) {
                    self.launch(Body::Thaw(Thaw::Lifted), Cargo::Thaw(None));
                    return;
                }
                match self.clock {
                    Clock::Unchecked => {
                        self.clock = Clock::Waiting;
                        self.register(Wait::Clock);
                    }
                    Clock::Waiting => {}
                    Clock::Fresh => {
                        self.clock = Clock::Unchecked;
                        for submission in std::mem::take(&mut self.queue) {
                            for ticket in submission.tickets {
                                let refusal = Refusal::Frozen { deadline };
                                self.settle(ticket, Settled::Refused(refusal));
                            }
                        }
                    }
                }
                return;
            }
            self.clock = Clock::Unchecked;
            if let Err(error) = self.decide() {
                self.break_down(Refusal::Cache(error));
            }
        }
    }

    /// Decide the queue against the head and write it as one entry. Nothing
    /// is settled or taken from the queue unless every judgment succeeded.
    fn decide(&mut self) -> Result<(), CacheError> {
        enum Fate {
            Answered(Settled),
            Decided(Verdict),
        }
        let head = self.replica.head().expect("an open machine has a head");
        let (schema, mut revision) = (head.schema, head.revision);
        let mut accepted = Vec::new();
        let mut fates = Vec::new();
        let mut size = 0usize;
        for submission in &self.queue {
            let command = &submission.command;
            if let Some(receipt) = self.replica.receipt(command.request())? {
                fates.push(Fate::Answered(if receipt.command == command.reference() {
                    Settled::Decided(receipt)
                } else {
                    Settled::Refused(Refusal::RequestReused(receipt.command))
                }));
                continue;
            }
            if command.changes().schema() != schema {
                fates.push(Fate::Answered(Settled::Refused(Refusal::ForeignSchema)));
                continue;
            }
            let bytes = command.changes().as_bytes().len() + DECIDED_OVERHEAD;
            if size + bytes > self.config.max_entry_bytes {
                break;
            }
            size += bytes;
            let verdict = match command.precondition() {
                Precondition::ExactRevision(expected) if expected != revision => {
                    Verdict::PreconditionFailed {
                        expected,
                        observed: revision,
                    }
                }
                _ => match self.replica.judge(&accepted, command.changes())? {
                    Judgment::Changed(delta) => {
                        revision = revision.next();
                        accepted.push(command.changes().clone());
                        Verdict::Committed {
                            changes: command.changes().clone(),
                            delta,
                        }
                    }
                    Judgment::Unchanged => Verdict::NoChange,
                    Judgment::Rejected(evidence) => Verdict::InvariantRejected(evidence),
                },
            };
            fates.push(Fate::Decided(verdict));
        }
        let consumed: Vec<Submission> = self.queue.drain(..fates.len()).collect();
        let mut decided = Vec::new();
        let mut batch = Vec::new();
        for (submission, fate) in consumed.into_iter().zip(fates) {
            match fate {
                Fate::Answered(settled) => {
                    for ticket in submission.tickets {
                        self.settle(ticket, settled.clone());
                    }
                }
                Fate::Decided(verdict) => {
                    decided.push(Decided {
                        command: submission.command.reference(),
                        verdict,
                    });
                    batch.push(submission);
                }
            }
        }
        if !decided.is_empty() {
            self.launch(Body::Commands(decided.into()), Cargo::Commands(batch));
        }
        Ok(())
    }

    fn start(&mut self, control: Control) {
        let (ticket, step) = match &control {
            Control::Freeze(ticket, _) => (*ticket, None),
            Control::Migrate(ticket, population) => (*ticket, Some(&population.step)),
        };
        let next = match self.gate() {
            Err(Refusal::MigrationPending { next }) => next,
            Ok(()) => return self.settle(ticket, Settled::Refused(Refusal::NotPending)),
            Err(refusal) => return self.settle(ticket, Settled::Refused(refusal)),
        };
        let pending = self.replica.bundle().steps()[next].clone();
        if step.is_some_and(|step| *step != pending.id) {
            return self.settle(ticket, Settled::Refused(Refusal::NotPending));
        }
        let head = self
            .replica
            .head()
            .expect("an open machine has a head")
            .clone();
        let frozen_for_other = match &head.mode {
            Mode::Open => false,
            Mode::Frozen { freeze, .. } => {
                matches!(control, Control::Freeze(..)) || freeze.migration != pending.id
            }
        };
        if frozen_for_other {
            let deadline = head.mode.deadline().expect("a frozen head has a deadline");
            return self.settle(ticket, Settled::Refused(Refusal::Frozen { deadline }));
        }
        match control {
            Control::Freeze(ticket, lease_millis) => {
                let body = Body::Freeze(Freeze {
                    migration: pending.id,
                    lease_millis,
                });
                self.launch(body, Cargo::Freeze(ticket));
            }
            Control::Migrate(ticket, population) => {
                if population.base != head.seq {
                    let refusal = Refusal::Stale { head: head.seq };
                    return self.settle(ticket, Settled::Refused(refusal));
                }
                let target =
                    migrated_head(&head, head.seq.next(), &pending.id, pending.fingerprint);
                match self.replica.migrate(&population, &target) {
                    Ok(Migrated::Image(image)) => {
                        let body = Body::Migration(Migration {
                            migration: pending.id,
                            schema: pending.fingerprint,
                            image: image.digest,
                        });
                        self.launch(body, Cargo::Migration(ticket, image));
                    }
                    Ok(Migrated::Rejected(evidence)) => {
                        let rejection = Rejection {
                            migration: pending.id,
                            evidence,
                        };
                        self.launch(
                            Body::Thaw(Thaw::Rejected(rejection)),
                            Cargo::Thaw(Some(ticket)),
                        );
                    }
                    Err(error) => self.settle(ticket, Settled::Refused(Refusal::Cache(error))),
                }
            }
        }
    }

    fn launch(&mut self, body: Body, cargo: Cargo) {
        let entry = Entry {
            nonce: self.nonce(),
            body,
        };
        let bytes = entry.encode();
        let slot = self.next_slot();
        let state = match &cargo {
            Cargo::Migration(_, image) => FlightState::Uploading(self.request(
                Bucket::Checkpoints,
                image_key(image.digest),
                Op::PutIfAbsent(IoBody::File(image.path.clone())),
                Purpose::Upload,
            )),
            _ => FlightState::Putting(self.request(
                Bucket::Log,
                log_key(slot),
                Op::PutIfAbsent(IoBody::Bytes(bytes.clone())),
                Purpose::Put,
            )),
        };
        self.writer = Writer::InFlight(Box::new(Flight {
            slot,
            bytes,
            entry,
            cargo,
            state,
        }));
    }

    /// PUT the in-flight entry's exact bytes at its slot.
    fn put_entry(&mut self) {
        let Writer::InFlight(flight) = &self.writer else {
            return;
        };
        let (slot, bytes) = (flight.slot, flight.bytes.clone());
        let id = self.request(
            Bucket::Log,
            log_key(slot),
            Op::PutIfAbsent(IoBody::Bytes(bytes)),
            Purpose::Put,
        );
        if let Writer::InFlight(flight) = &mut self.writer {
            flight.state = FlightState::Putting(id);
        }
    }

    fn nonce(&mut self) -> Nonce {
        let mut hasher = blake3::Hasher::new_derive_key(NONCE_CONTEXT);
        hasher.update(&self.config.seed);
        hasher.update(&self.nonces.to_be_bytes());
        self.nonces += 1;
        let mut nonce = [0; 16];
        nonce.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
        Nonce(nonce)
    }

    fn maybe_checkpoint(&mut self) {
        let Some(head) = self.replica.head() else {
            return;
        };
        let since = head.seq.get() - self.checkpoints.last.map_or(0, Seq::get);
        if !self.checkpoints.known
            || self.checkpoints.uploading
            || self.install.is_some()
            || since < self.config.checkpoint.every
        {
            return;
        }
        let (seq, schema) = (head.seq, head.schema);
        match self.replica.image() {
            Ok(image) => {
                let key = CheckpointKey {
                    seq,
                    schema,
                    digest: image.digest,
                };
                self.checkpoints.uploading = true;
                self.request(
                    Bucket::Checkpoints,
                    key.format(),
                    Op::PutIfAbsent(IoBody::File(image.path)),
                    Purpose::Checkpoint(key),
                );
            }
            Err(error) => self.break_down(Refusal::Cache(error)),
        }
    }

    fn request(&mut self, bucket: Bucket, key: String, op: Op, purpose: Purpose) -> IoId {
        let id = IoId(self.next_io);
        self.next_io += 1;
        self.io.insert(id, purpose);
        self.out.io.push(IoRequest {
            id,
            bucket,
            key,
            op,
        });
        id
    }

    /// Settle every ticket a flight carries with `refusal`.
    fn abandon(&mut self, flight: Box<Flight>, refusal: &Refusal) {
        match flight.cargo {
            Cargo::Genesis | Cargo::Thaw(None) => {}
            Cargo::Commands(batch) => {
                for ticket in batch.into_iter().flat_map(|submission| submission.tickets) {
                    self.settle(ticket, Settled::Refused(refusal.clone()));
                }
            }
            Cargo::Freeze(ticket) | Cargo::Migration(ticket, _) | Cargo::Thaw(Some(ticket)) => {
                self.settle(ticket, Settled::Refused(refusal.clone()));
            }
        }
    }

    fn settle(&mut self, ticket: Ticket, settled: Settled) {
        self.out.done.push(Done { ticket, settled });
    }

    fn break_down(&mut self, refusal: Refusal) {
        self.halt(Life::Broken(refusal.clone()), &refusal);
    }

    /// Stop in `life`: settle everything pending with `refusal` (an unwritten
    /// command of a closed machine is `NotSubmitted`) and forget outstanding
    /// requests.
    fn halt(&mut self, life: Life, refusal: &Refusal) {
        self.life = life;
        self.io.clear();
        self.tail = Tail::default();
        self.install = None;
        self.clock = Clock::Unchecked;
        if let Writer::InFlight(flight) = std::mem::replace(&mut self.writer, Writer::Idle) {
            let unclear = match flight.state {
                FlightState::Uploading(_) => Refusal::NotSubmitted,
                FlightState::Putting(_) | FlightState::Verifying { .. } => Refusal::Unknown,
            };
            self.abandon(flight, &unclear);
        }
        let unwritten = match refusal {
            Refusal::Closed => Refusal::NotSubmitted,
            other => other.clone(),
        };
        for ticket in std::mem::take(&mut self.queue)
            .into_iter()
            .flat_map(|submission| submission.tickets)
        {
            self.settle(ticket, Settled::Refused(unwritten.clone()));
        }
        for control in std::mem::take(&mut self.controls) {
            let (Control::Freeze(ticket, _) | Control::Migrate(ticket, _)) = control;
            self.settle(ticket, Settled::Refused(unwritten.clone()));
        }
        for waiter in std::mem::take(&mut self.waiters) {
            match waiter.wait {
                Wait::Sync(ticket) | Wait::Resolve(ticket, _) => {
                    self.settle(ticket, Settled::Refused(refusal.clone()));
                }
                Wait::Open | Wait::Clock => {}
            }
        }
        for ticket in std::mem::take(&mut self.opens) {
            self.settle(ticket, Settled::Refused(refusal.clone()));
        }
    }
}
