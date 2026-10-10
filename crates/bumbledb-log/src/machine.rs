//! The sans-IO protocol: [`Machine::step`] turns one [`Input`] into store
//! requests and settled tickets. It owns every invariant: one entry write in
//! flight, a refused batch or freeze written again at the next slot, byte
//! comparison of every unclear write, receipts before decisions, checkpoint
//! cadence, freeze deadlines on store time. It never sleeps or reads a clock.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::command::{Command, CommandRef};
use crate::entry::{Batch, Body, Entry, Freeze, Genesis, Migration, Proposal, Thaw};
use crate::fold::{Standing, fold, migrated_head, standing};
use crate::head::{Comparison, Mode, Rejection};
use crate::ids::{DatabaseId, Millis, Nonce, RequestId, Seq};
use crate::io::{
    Body as IoBody, Bucket, CHECKPOINT_PREFIX, CheckpointKey, IoId, IoRequest, IoResponse,
    IoResult, Op, Target, image_key, log_key,
};
use crate::receipt::Receipt;
use crate::replica::{CacheError, Image, Migrated, Population, Replica, Update};

const NONCE_CONTEXT: &str = "bdb.entry.v1 nonce";
/// Encoded bytes of one proposal besides its changes.
const PROPOSAL_OVERHEAD: usize = 128;
/// How far past the head catch-up probes, in probe windows.
const LOOKAHEAD_WINDOWS: u32 = 4;

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
/// a submit except [`Refusal::Unknown`] proves the log does not decide the
/// command.
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
    /// Every slot after the head and before this one is occupied: the flight
    /// moved past each on a refused PUT.
    slot: Seq,
    bytes: Vec<u8>,
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
                self.register(Wait::Open);
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
        if command.changes().as_bytes().len() + PROPOSAL_OVERHEAD > self.config.max_entry_bytes {
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
                // Below the flight's slot the answer is stale: the slot is taken.
                let taken = matches!(&self.writer, Writer::InFlight(flight) if seq < flight.slot);
                if !taken {
                    let seen = self.tail.missing.entry(seq).or_insert(issued);
                    *seen = (*seen).max(issued);
                }
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

    /// A PUT of the flight answered. Created: the entry waits in the tail for
    /// its predecessors. Refused: a batch or a freeze, which stands wherever
    /// it lands, goes to the next slot at once; the catch-up reads the
    /// refused one. Anything else is unclear until the slot is read.
    fn put_returned(&mut self, id: IoId, result: IoResult, date: Option<Millis>) {
        let Writer::InFlight(flight) = &mut self.writer else {
            return;
        };
        if flight.state != FlightState::Putting(id) {
            return;
        }
        match result {
            IoResult::Created => {
                let at = date.or(self.now).unwrap_or(Millis(0));
                self.tail
                    .found
                    .insert(flight.slot, (flight.bytes.clone(), at));
            }
            IoResult::Occupied if matches!(flight.cargo, Cargo::Commands(_) | Cargo::Freeze(_)) => {
                self.tail.missing.remove(&flight.slot);
                flight.slot = flight.slot.next();
                let grows = matches!(flight.cargo, Cargo::Commands(_)) && !self.queue.is_empty();
                if grows && self.takes_commands() {
                    self.regrow();
                } else {
                    self.put_entry();
                }
            }
            _ => {
                self.epoch += 1;
                flight.state = FlightState::Verifying { need: self.epoch };
            }
        }
    }

    /// Decide the refused batch again at the head with the queue behind it,
    /// and write the result at the flight's next slot. Its commands may have
    /// landed in an earlier copy, so they are `Unknown` if judging fails.
    fn regrow(&mut self) {
        let Writer::InFlight(flight) = std::mem::replace(&mut self.writer, Writer::Idle) else {
            return;
        };
        let Cargo::Commands(carried) = flight.cargo else {
            unreachable!("only a batch grows")
        };
        let count = carried.len();
        self.requeue(carried);
        if let Err(error) = self.decide(flight.slot) {
            let carried: Vec<_> = self.queue.drain(..count).collect();
            for ticket in carried
                .into_iter()
                .flat_map(|submission| submission.tickets)
            {
                self.settle(ticket, Settled::Refused(Refusal::Unknown));
            }
            self.break_down(Refusal::Cache(error));
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
        if self.life != Life::Opening || !matches!(self.writer, Writer::Idle) {
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

    /// Install a downloaded checkpoint; the tail fetched past it meanwhile
    /// stays. One that cannot be installed (a damaged download, an image this
    /// platform or bundle cannot open) only costs replaying the log from the
    /// current head instead.
    fn install_checkpoint(&mut self, key: CheckpointKey) {
        if self.replica.head().is_some_and(|head| head.seq >= key.seq) {
            return;
        }
        let database = self.replica.head().map(|head| head.database);
        let path = self.replica.download_path();
        if self.replica.install(&path, key.digest, key.schema).is_err() {
            return;
        }
        let head = self.replica.head().expect("an installed image has a head");
        if head.seq != key.seq
            || head.schema != key.schema
            || database.is_some_and(|database| database != head.database)
        {
            self.break_down(Refusal::Corrupt(key.seq));
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
                    self.lost(*flight);
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

    /// Where catch-up reads from: past a downloading checkpoint, else the
    /// next slot.
    fn fetch_from(&self) -> Seq {
        match &self.install {
            Some(Installing::Checkpoint { key, .. }) => key.seq.next(),
            _ => self.next_slot(),
        }
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

    /// The content of the next slot is known: apply it, and answer every
    /// pending command it decides, whoever wrote it. The flight ends when its
    /// entry decides here, or when its own slot resolves without that; a copy
    /// below its slot leaves the PUT at the slot outstanding.
    fn land(&mut self, seq: Seq, bytes: Vec<u8>, at: Millis) {
        debug_assert_eq!(seq, self.next_slot());
        let Ok(entry) = Entry::parse(&bytes) else {
            return self.break_down(Refusal::Corrupt(seq));
        };
        if let Body::Migration(migration) = &entry.body {
            match std::mem::replace(&mut self.writer, Writer::Idle) {
                Writer::InFlight(flight) if flight.bytes == bytes => {
                    return self.migrated(&flight, seq, migration);
                }
                writer => self.writer = writer,
            }
            return self.follow_migration(seq, entry);
        }
        let Some((standing, receipts)) = self.apply(seq, &entry, at) else {
            return;
        };
        self.after_land();
        self.answer_decided(&receipts);
        match std::mem::replace(&mut self.writer, Writer::Idle) {
            Writer::InFlight(flight) if flight.bytes == bytes && standing != Standing::Void => {
                self.landed(*flight, seq, &entry);
            }
            Writer::InFlight(flight) if flight.slot == seq => self.lost(*flight),
            writer => self.writer = writer,
        }
    }

    /// Our own migration landed: install the image it was built from.
    fn migrated(&mut self, flight: &Flight, seq: Seq, migration: &Migration) {
        let Cargo::Migration(ticket, image) = &flight.cargo else {
            unreachable!("migration bytes ride migration cargo")
        };
        let head = self.replica.head().expect("a migration follows a head");
        let expected = migrated_head(head, seq, &migration.migration, migration.schema);
        self.settle(*ticket, Settled::Migrated(seq));
        match self
            .replica
            .install(&image.path, migration.image, migration.schema)
        {
            Ok(()) if self.replica.head() == Some(&expected) => self.after_land(),
            Ok(()) => self.break_down(Refusal::Corrupt(seq)),
            Err(error) => self.break_down(Refusal::Cache(error)),
        }
    }

    /// Another writer's migration landed: download its image, or stop before
    /// a schema this code does not ship.
    fn follow_migration(&mut self, seq: Seq, entry: Entry) {
        let Body::Migration(migration) = &entry.body else {
            unreachable!("a migration install holds a migration entry")
        };
        if self.replica.head().is_none() {
            return self.break_down(Refusal::Corrupt(seq));
        }
        if self.replica.bundle().by_schema(migration.schema).is_none() {
            // The flight can only land after this migration, at a schema it
            // was not judged at.
            if let Writer::InFlight(flight) = std::mem::replace(&mut self.writer, Writer::Idle) {
                self.abandon(*flight, &Refusal::SchemaAdvanced);
            }
            return self.halt(Life::Advanced, &Refusal::SchemaAdvanced);
        }
        let lost = match std::mem::replace(&mut self.writer, Writer::Idle) {
            Writer::InFlight(flight) if flight.slot == seq => Some(flight),
            writer => {
                self.writer = writer;
                None
            }
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
            entry: Box::new(entry),
            lost,
        });
    }

    /// Fold and apply a non-migration entry, judging a rebased batch here;
    /// its standing and receipts, or `None` when the machine broke.
    fn apply(&mut self, seq: Seq, entry: &Entry, at: Millis) -> Option<(Standing, Vec<Receipt>)> {
        let head = self.replica.head();
        let Ok(standing) = standing(head, seq, &entry.body) else {
            self.break_down(Refusal::Corrupt(seq));
            return None;
        };
        let decided = match (&entry.body, standing) {
            (Body::Commands(batch), Standing::Fresh | Standing::Rebased) => {
                match self.decided(seq, batch, standing) {
                    Ok(decided) => decided,
                    Err(refusal) => {
                        self.break_down(refusal);
                        return None;
                    }
                }
            }
            _ => Vec::new(),
        };
        let Ok(folded) = fold(head, seq, entry, at, &decided) else {
            self.break_down(Refusal::Corrupt(seq));
            return None;
        };
        let applied = if seq == Seq::GENESIS {
            self.replica.create(&folded.head)
        } else {
            self.replica.apply(Update {
                head: &folded.head,
                commits: &folded.commits,
                receipts: &folded.receipts,
            })
        };
        match applied {
            Ok(()) => Some((standing, folded.receipts)),
            Err(error) => {
                self.break_down(Refusal::Cache(error));
                None
            }
        }
    }

    /// The commands a batch that is not void at `seq` decides there, with
    /// the outcomes that hold there. Fresh: as recorded. Rebased: a request
    /// decided before, in the log or earlier in the batch, is skipped; the
    /// rest are judged here.
    fn decided(
        &self,
        seq: Seq,
        batch: &Batch,
        standing: Standing,
    ) -> Result<Vec<Proposal>, Refusal> {
        let head = self.replica.head().expect("a batch follows a head");
        let step = self
            .replica
            .bundle()
            .by_schema(head.schema)
            .ok_or(Refusal::Cache(CacheError::UnknownSchema(head.schema)))?;
        let proposals = batch
            .proposals(&step.schema)
            .map_err(|_| Refusal::Corrupt(seq))?;
        if standing == Standing::Fresh {
            return Ok(proposals.into_vec());
        }
        let mut seen = BTreeSet::new();
        let mut undecided = Vec::new();
        for proposal in &proposals {
            let request = proposal.command.request();
            if seen.insert(request)
                && self
                    .replica
                    .receipt(request)
                    .map_err(Refusal::Cache)?
                    .is_none()
            {
                undecided.push(&proposal.command);
            }
        }
        let outcomes = self.replica.judge(&undecided).map_err(Refusal::Cache)?;
        Ok(undecided
            .into_iter()
            .zip(outcomes)
            .map(|(command, outcome)| Proposal {
                command: command.clone(),
                outcome,
            })
            .collect())
    }

    /// Our own entry decided at `seq`. A batch's commands were answered as
    /// the receipts deciding them landed, here or in an earlier copy.
    fn landed(&mut self, flight: Flight, seq: Seq, entry: &Entry) {
        match flight.cargo {
            Cargo::Genesis => self.opened(),
            Cargo::Commands(batch) => {
                debug_assert!(
                    batch.is_empty(),
                    "a landed batch leaves no command unanswered"
                );
                self.requeue(batch);
            }
            Cargo::Freeze(ticket) => self.settle(ticket, Settled::Frozen(seq)),
            Cargo::Thaw(ticket) => {
                if let (Some(ticket), Body::Thaw(Thaw::Rejected(rejection))) = (ticket, &entry.body)
                {
                    let refusal = Refusal::MigrationRejected(rejection.clone());
                    self.settle(ticket, Settled::Refused(refusal));
                }
            }
            Cargo::Migration(..) => unreachable!("a migration lands through its image"),
        }
    }

    /// Answer every pending command whose request `receipts` decide.
    fn answer_decided(&mut self, receipts: &[Receipt]) {
        if receipts.is_empty() {
            return;
        }
        let decided: BTreeMap<RequestId, &Receipt> = receipts
            .iter()
            .map(|receipt| (receipt.command.request, receipt))
            .collect();
        let is_decided =
            |submission: &mut Submission| decided.contains_key(&submission.command.request());
        let mut answered: Vec<Submission> = self.queue.extract_if(.., is_decided).collect();
        if let Writer::InFlight(flight) = &mut self.writer
            && let Cargo::Commands(batch) = &mut flight.cargo
        {
            answered.extend(batch.extract_if(.., is_decided));
        }
        for submission in answered {
            let receipt = decided[&submission.command.request()].clone();
            let settled = answer(receipt, &submission.command);
            for ticket in submission.tickets {
                self.settle(ticket, settled.clone());
            }
        }
    }

    /// Put commands back at the front of the queue.
    fn requeue(&mut self, batch: Vec<Submission>) {
        let queued = std::mem::replace(&mut self.queue, batch);
        self.queue.extend(queued);
    }

    /// The flight's slot resolved without its entry deciding anything there.
    fn lost(&mut self, flight: Flight) {
        let head = self.replica.head().map(|head| head.seq);
        let stale = Refusal::Stale {
            head: head.expect("a lost slot follows a head"),
        };
        match flight.cargo {
            Cargo::Genesis => self.opened(),
            Cargo::Commands(batch) => self.requeue(batch),
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

    /// Ask for the slot GETs the flight and the waiters need.
    fn refill(&mut self) {
        if matches!(self.install, Some(Installing::Migration { .. }))
            || !matches!(self.life, Life::Opening | Life::Open)
        {
            return;
        }
        let next = self.fetch_from();
        let mut outstanding = BTreeMap::new();
        for purpose in self.io.values() {
            if let Purpose::Slot { seq, issued } = purpose {
                let newest = outstanding.entry(*seq).or_insert(*issued);
                *newest = (*newest).max(*issued);
            }
        }
        let (first, verifying) = match &self.writer {
            Writer::InFlight(flight) => match flight.state {
                FlightState::Verifying { need } => (flight.slot, Some(need)),
                _ => (flight.slot, None),
            },
            Writer::Idle => (next, None),
        };
        // The slots below the flight's exist: fetch them all, unprompted.
        let mut seq = next;
        while seq < first {
            if !outstanding.contains_key(&seq) && !self.tail.found.contains_key(&seq) {
                self.get_slot(seq);
                outstanding.insert(seq, self.epoch);
            }
            seq = seq.next();
        }
        let need = self
            .waiters
            .iter()
            .map(|waiter| waiter.need)
            .chain(verifying)
            .max();
        let Some(need) = need else {
            return;
        };
        // A slot seen empty for a GET at least as fresh needs no other.
        let probed = outstanding
            .get(&first)
            .into_iter()
            .chain(self.tail.missing.get(&first))
            .max();
        if !self.tail.found.contains_key(&first) && probed.is_none_or(|issued| *issued < need) {
            self.get_slot(first);
            outstanding.insert(first, self.epoch);
        }
        // `window` GETs in flight, not `window` slots: one slow GET must not
        // idle the rest. The lookahead bounds buffered entries, and nothing
        // is probed past a slot already seen empty.
        let window = self
            .config
            .probe_window
            .min(self.tail.streak.saturating_add(1));
        let mut in_flight = outstanding.range(first..).count();
        let mut seq = first;
        for _ in 1..window.saturating_mul(LOOKAHEAD_WINDOWS) {
            if in_flight >= window as usize {
                break;
            }
            seq = seq.next();
            if self.tail.missing.contains_key(&seq) {
                break;
            }
            if !outstanding.contains_key(&seq) && !self.tail.found.contains_key(&seq) {
                self.get_slot(seq);
                in_flight += 1;
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
                        self.launch(self.next_slot(), body, Cargo::Genesis);
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
                    self.launch(
                        self.next_slot(),
                        Body::Thaw(Thaw::Lifted),
                        Cargo::Thaw(None),
                    );
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
            if let Err(error) = self.decide(self.next_slot()) {
                self.break_down(Refusal::Cache(error));
            }
        }
    }

    /// Whether a batch may be decided at the head now.
    fn takes_commands(&self) -> bool {
        self.life == Life::Open
            && self.install.is_none()
            && self.gate().is_ok()
            && self
                .replica
                .head()
                .is_some_and(|head| head.mode == Mode::Open)
    }

    /// Judge the queue against the head and write it as one batch at `slot`.
    /// Nothing is settled or taken from the queue unless the judgment
    /// succeeded.
    fn decide(&mut self, slot: Seq) -> Result<(), CacheError> {
        enum Fate {
            Answered(Settled),
            Proposed,
        }
        let head = self.replica.head().expect("an open machine has a head");
        let (base, schema) = (head.seq, head.schema);
        let mut fates = Vec::new();
        let mut proposed = Vec::new();
        let mut size = 0usize;
        for submission in &self.queue {
            let command = &submission.command;
            if let Some(receipt) = self.replica.receipt(command.request())? {
                fates.push(Fate::Answered(answer(receipt, command)));
                continue;
            }
            if command.changes().schema() != schema {
                fates.push(Fate::Answered(Settled::Refused(Refusal::ForeignSchema)));
                continue;
            }
            let bytes = command.changes().as_bytes().len() + PROPOSAL_OVERHEAD;
            if size + bytes > self.config.max_entry_bytes {
                break;
            }
            size += bytes;
            fates.push(Fate::Proposed);
            proposed.push(command);
        }
        let outcomes = self.replica.judge(&proposed)?;
        let proposals: Vec<Proposal> = proposed
            .into_iter()
            .zip(outcomes)
            .map(|(command, outcome)| Proposal {
                command: command.clone(),
                outcome,
            })
            .collect();
        let consumed: Vec<Submission> = self.queue.drain(..fates.len()).collect();
        let mut batch = Vec::new();
        for (submission, fate) in consumed.into_iter().zip(fates) {
            match fate {
                Fate::Answered(settled) => {
                    for ticket in submission.tickets {
                        self.settle(ticket, settled.clone());
                    }
                }
                Fate::Proposed => batch.push(submission),
            }
        }
        if let Some(proposals) = Batch::new(base, schema, &proposals) {
            self.launch(slot, Body::Commands(proposals), Cargo::Commands(batch));
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
                self.launch(self.next_slot(), body, Cargo::Freeze(ticket));
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
                        self.launch(self.next_slot(), body, Cargo::Migration(ticket, image));
                    }
                    Ok(Migrated::Rejected(evidence)) => {
                        let rejection = Rejection {
                            migration: pending.id,
                            evidence,
                        };
                        self.launch(
                            self.next_slot(),
                            Body::Thaw(Thaw::Rejected(rejection)),
                            Cargo::Thaw(Some(ticket)),
                        );
                    }
                    Err(error) => self.settle(ticket, Settled::Refused(Refusal::Cache(error))),
                }
            }
        }
    }

    fn launch(&mut self, slot: Seq, body: Body, cargo: Cargo) {
        let bytes = Entry {
            nonce: self.nonce(),
            body,
        }
        .encode();
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
        let since = head
            .seq
            .get()
            .saturating_sub(self.checkpoints.last.map_or(0, Seq::get));
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
    fn abandon(&mut self, flight: Flight, refusal: &Refusal) {
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
            self.abandon(*flight, &unclear);
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

/// How the log answers `command` once its request has `receipt`.
fn answer(receipt: Receipt, command: &Command) -> Settled {
    if receipt.command == command.reference() {
        Settled::Decided(receipt)
    } else {
        Settled::Refused(Refusal::RequestReused(receipt.command))
    }
}
