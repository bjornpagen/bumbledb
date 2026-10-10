//! Scripted protocol paths: each test drives the machine through one case
//! with chosen request fates, then checks the world against the replay.

use bumbledb::Schema;
use bumbledb_log::{
    Body, Bucket, Bundle, Command, Entry, Input, IoBody, IoRequest, Mode, Op, Outcome,
    Precondition, Refusal, Replica as _, RequestId, Revision, Seq, Settled, Target, Ticket,
};

use crate::sim::{Ask, Fate, Faults, World, check, receipt_of, standings};
use crate::support::{bundle, bundle2, items, migration_id, schema};

fn opened(seed: u64, clients: usize) -> World {
    opened_with(seed, vec![bundle(); clients])
}

/// One client per bundle, each started and caught up in turn.
fn opened_with(seed: u64, bundles: Vec<Bundle>) -> World {
    let clients = bundles.len();
    let mut world = World::with_bundles(seed, bundles, Faults::NONE);
    for client in 0..clients {
        world.start(client);
        world.drain();
    }
    world
}

fn command(schema: &Schema, request: u8, adds: &[(u64, u64)]) -> Command {
    Command::seal(
        RequestId([request; 16]),
        Precondition::None,
        items(schema, adds, &[]),
    )
}

fn submit(world: &mut World, client: usize, command: Command) -> Ticket {
    world.submit(client, command.request(), command)
}

fn settled(world: &World, ticket: Ticket) -> &Settled {
    &world.results[&ticket].settled
}

fn is_put(request: &IoRequest) -> bool {
    request.bucket == Bucket::Log && matches!(request.op, Op::PutIfAbsent(_))
}

fn put_at(seq: u64) -> impl Fn(&IoRequest) -> bool {
    move |request| is_put(request) && request.key == bumbledb_log::log_key(Seq::new(seq).unwrap())
}

fn get_at(seq: u64) -> impl Fn(&IoRequest) -> bool {
    move |request| {
        request.bucket == Bucket::Log
            && matches!(request.op, Op::Get(_))
            && request.key == bumbledb_log::log_key(Seq::new(seq).unwrap())
    }
}

/// The body of `client`'s issued PUT at `seq`.
fn body_at(world: &World, client: usize, seq: u64) -> Vec<u8> {
    let index = world.find(client, put_at(seq)).expect("a PUT at the slot");
    let (_, request) = world.issued().nth(index).expect("issued");
    let Op::PutIfAbsent(IoBody::Bytes(bytes)) = &request.op else {
        panic!("an entry PUT carries bytes");
    };
    bytes.clone()
}

fn head(world: &World, client: usize) -> Seq {
    world.machine(client).replica().head().expect("a head").seq
}

fn put_body(world: &World, client: usize) -> Vec<u8> {
    world
        .issued()
        .find_map(|(owner, request)| match &request.op {
            Op::PutIfAbsent(IoBody::Bytes(bytes)) if owner == client => Some(bytes.clone()),
            _ => None,
        })
        .expect("an entry PUT is issued")
}

#[test]
fn a_lone_writer_records_every_kind_of_decision() {
    let schema = schema();
    let mut world = opened(1, 1);
    let opens: Vec<_> = world
        .results
        .values()
        .map(|result| result.settled.clone())
        .collect();
    assert_eq!(opens, [Settled::Opened { pending: 0 }]);

    let first = submit(&mut world, 0, command(&schema, 1, &[(1, 10)]));
    world.drain();
    let receipt = receipt_of(settled(&world, first)).clone();
    assert_eq!(receipt.seq, Seq::new(2).unwrap());
    assert_eq!(receipt.revision, Revision(1));
    assert!(matches!(receipt.outcome, Outcome::Committed(delta) if delta.added() == 1));

    let same = submit(&mut world, 0, command(&schema, 2, &[(1, 10)]));
    let clash = submit(&mut world, 0, command(&schema, 3, &[(1, 11)]));
    let stale = world.submit(
        0,
        RequestId([4; 16]),
        Command::seal(
            RequestId([4; 16]),
            Precondition::ExactRevision(Revision(0)),
            items(&schema, &[(2, 1)], &[]),
        ),
    );
    world.drain();
    assert_eq!(receipt_of(settled(&world, same)).outcome, Outcome::NoChange);
    assert!(matches!(
        receipt_of(settled(&world, clash)).outcome,
        Outcome::InvariantRejected(_)
    ));
    assert_eq!(
        receipt_of(settled(&world, stale)).outcome,
        Outcome::PreconditionFailed {
            expected: Revision(0),
            observed: Revision(1)
        }
    );

    // A decided request answers from its receipt without touching the store.
    let again = submit(&mut world, 0, command(&schema, 1, &[(1, 10)]));
    assert_eq!(world.issued().count(), 0);
    assert_eq!(settled(&world, again), &Settled::Decided(receipt));

    let reused = submit(&mut world, 0, command(&schema, 1, &[(9, 9)]));
    assert!(matches!(
        settled(&world, reused),
        Settled::Refused(Refusal::RequestReused(_))
    ));
    check(&world);
}

#[test]
fn commands_arriving_during_a_write_share_the_next_entry() {
    let schema = schema();
    let mut world = opened(2, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    assert_eq!(
        world
            .issued()
            .filter(|(_, request)| is_put(request))
            .count(),
        1
    );
    let b = submit(&mut world, 0, command(&schema, 2, &[(2, 2)]));
    let c = submit(&mut world, 0, command(&schema, 3, &[(3, 3)]));
    assert_eq!(world.issued().count(), 1, "one write in flight, no timer");
    world.execute_as(0, Fate::Answered);
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    world.drain();
    let (b, c) = (
        receipt_of(settled(&world, b)),
        receipt_of(settled(&world, c)),
    );
    assert_eq!(b.seq, Seq::new(3).unwrap());
    assert_eq!(c.seq, b.seq);
    assert_eq!((b.revision, c.revision), (Revision(2), Revision(3)));
    check(&world);
}

#[test]
fn a_refused_batch_is_written_again_at_the_next_slot_in_the_same_step() {
    let schema = schema();
    let mut world = opened(3, 2);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let b = submit(&mut world, 1, command(&schema, 2, &[(2, 2)]));
    let refused = body_at(&world, 0, 2);
    world.execute_as(world.find(1, put_at(2)).unwrap(), Fate::Answered);
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Answered);
    assert_eq!(
        body_at(&world, 0, 3),
        refused,
        "the same bytes go to the next slot"
    );
    assert!(
        world.find(0, get_at(2)).is_some(),
        "the refused slot is read"
    );
    assert_eq!(head(&world, 0), Seq::GENESIS, "nothing was read yet");
    world.drain();
    let (a, b) = (
        receipt_of(settled(&world, a)),
        receipt_of(settled(&world, b)),
    );
    assert_eq!((b.seq, b.revision), (Seq::new(2).unwrap(), Revision(1)));
    assert_eq!((a.seq, a.revision), (Seq::new(3).unwrap(), Revision(2)));
    assert!(matches!(a.outcome, Outcome::Committed(_)));
    assert_eq!(world.log.len(), 3);
    let states = check(&world);
    assert_eq!(standings(&world, &states).rebased, 1);
}

fn requests_at(world: &World, client: usize, seq: u64, schema: &Schema) -> Vec<RequestId> {
    let Body::Commands(batch) = Entry::parse(&body_at(world, client, seq)).unwrap().body else {
        panic!("a batch at {seq}");
    };
    batch
        .proposals(schema)
        .unwrap()
        .iter()
        .map(|proposal| proposal.command.request())
        .collect()
}

#[test]
fn a_refused_batch_takes_the_commands_queued_behind_it() {
    let schema = schema();
    let mut world = opened(20, 2);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let b = submit(&mut world, 1, command(&schema, 2, &[(2, 2)]));
    let c = submit(&mut world, 0, command(&schema, 3, &[(3, 3)]));
    world.execute_as(world.find(1, put_at(2)).unwrap(), Fate::Answered);
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Answered);
    assert_eq!(
        requests_at(&world, 0, 3, &schema),
        [RequestId([1; 16]), RequestId([3; 16])],
        "the queued command joins the refused batch"
    );
    assert_eq!(head(&world, 0), Seq::GENESIS, "nothing was read yet");
    world.drain();
    for (ticket, seq) in [(b, 2), (a, 3), (c, 3)] {
        assert_eq!(
            receipt_of(settled(&world, ticket)).seq,
            Seq::new(seq).unwrap()
        );
    }
    assert_eq!(world.log.len(), 3);
    let states = check(&world);
    assert_eq!(standings(&world, &states).rebased, 1);
}

#[test]
fn a_grown_batch_over_a_landed_copy_decides_each_request_once() {
    let schema = schema();
    let mut world = opened(21, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let c = submit(&mut world, 0, command(&schema, 3, &[(3, 3)]));
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Retried);
    assert_eq!(
        requests_at(&world, 0, 3, &schema),
        [RequestId([1; 16]), RequestId([3; 16])]
    );
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    assert_eq!(receipt_of(settled(&world, c)).seq, Seq::new(3).unwrap());
    let states = check(&world);
    assert_eq!(
        states[2].receipts[&RequestId([1; 16])].seq,
        Seq::new(2).unwrap(),
        "the grown copy decides only what the first left undecided"
    );
    assert_eq!(states[2].head.revision, Revision(2));
}

#[test]
fn a_rebased_batch_is_judged_where_it_lands() {
    let schema = schema();
    let mut world = opened(14, 2);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let b = submit(&mut world, 1, command(&schema, 2, &[(1, 2)]));
    world.execute_as(world.find(1, put_at(2)).unwrap(), Fate::Answered);
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Answered);
    world.drain();
    let (a, b) = (
        receipt_of(settled(&world, a)),
        receipt_of(settled(&world, b)),
    );
    assert!(matches!(b.outcome, Outcome::Committed(_)));
    assert_eq!(a.seq, Seq::new(3).unwrap());
    assert!(
        matches!(a.outcome, Outcome::InvariantRejected(_)),
        "judged against the winner's state, not as recorded"
    );
    let Body::Commands(batch) = Entry::parse(&world.log_entries()[2]).unwrap().body else {
        panic!("a batch at 3");
    };
    assert_eq!(batch.base, Seq::GENESIS);
    assert!(matches!(
        batch.proposals(&schema).unwrap()[0].outcome,
        Outcome::Committed(_)
    ));
    let states = check(&world);
    assert_eq!(
        states[2].rows, states[1].rows,
        "the rejection changed nothing"
    );
}

#[test]
fn a_writer_that_reads_the_winner_first_decides_again_at_the_new_head() {
    let schema = schema();
    let mut world = opened(15, 2);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    submit(&mut world, 1, command(&schema, 2, &[(1, 2)]));
    world.execute_as(world.find(1, put_at(2)).unwrap(), Fate::Answered);
    let refused = body_at(&world, 0, 2);
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Delayed);
    let sync = world.ticket(0, Ask::Sync);
    world.input(0, Input::Sync(sync));
    world.execute_as(world.find(0, get_at(2)).unwrap(), Fate::Answered);
    assert_ne!(
        body_at(&world, 0, 3),
        refused,
        "a batch decided again at the head it read"
    );
    world.drain();
    let a = receipt_of(settled(&world, a));
    assert_eq!(a.seq, Seq::new(3).unwrap());
    assert!(matches!(a.outcome, Outcome::InvariantRejected(_)));
    let states = check(&world);
    assert_eq!(standings(&world, &states).rebased, 0);
}

#[test]
fn a_batch_landing_on_a_frozen_head_decides_nothing_and_its_commands_meet_the_freeze() {
    let schema = schema();
    let mut world = opened_with(16, vec![bundle2(), bundle(), bundle2()]);
    let a = submit(&mut world, 1, command(&schema, 1, &[(1, 1)]));
    let freeze = world.ticket(0, Ask::Freeze);
    world.input(0, Input::Freeze(freeze, 60_000));
    let late = world.ticket(2, Ask::Freeze);
    world.input(2, Input::Freeze(late, 60_000));
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Answered);
    assert_eq!(
        settled(&world, freeze),
        &Settled::Frozen(Seq::new(2).unwrap())
    );
    world.execute_as(world.find(1, put_at(2)).unwrap(), Fate::Answered);
    world.execute_as(world.find(1, put_at(3)).unwrap(), Fate::Answered);
    world.execute_as(world.find(2, put_at(2)).unwrap(), Fate::Answered);
    world.execute_as(world.find(2, put_at(3)).unwrap(), Fate::Answered);
    world.execute_as(world.find(2, put_at(4)).unwrap(), Fate::Answered);
    world.drain();
    assert!(matches!(
        settled(&world, a),
        Settled::Refused(Refusal::Frozen { .. })
    ));
    assert_eq!(
        settled(&world, late),
        &Settled::Refused(Refusal::Stale {
            head: Seq::new(4).unwrap()
        })
    );
    assert_eq!(world.log.len(), 4);
    let states = check(&world);
    assert_eq!(standings(&world, &states).void, 1);
    assert!(
        states[3].receipts.is_empty(),
        "a void batch records nothing"
    );
    assert!(
        matches!(states[3].head.mode, Mode::Frozen { since, .. } if since == world.log_modified(Seq::new(2).unwrap())),
        "the late freeze is void"
    );
}

#[test]
fn a_batch_landing_twice_decides_each_request_once() {
    let schema = schema();
    let mut world = opened(17, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    world.execute_as(world.find(0, put_at(2)).unwrap(), Fate::Retried);
    world.execute_as(world.find(0, put_at(3)).unwrap(), Fate::Answered);
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    let entries = world.log_entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1], entries[2], "one batch at two slots");
    let late = submit(&mut world, 0, command(&schema, 2, &[(2, 2)]));
    world.drain();
    assert_eq!(receipt_of(settled(&world, late)).seq, Seq::new(4).unwrap());
    let states = check(&world);
    assert_eq!(states[2].receipts, states[1].receipts);
    assert_eq!(states[2].head.revision, Revision(1));
}

#[test]
fn an_unanswered_write_is_recognized_by_reading_it_back() {
    let schema = schema();
    let mut world = opened(4, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    world.execute_as(0, Fate::Unanswered);
    assert!(!world.results.contains_key(&a));
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    assert_eq!(world.log.len(), 2, "no second copy of the entry");
    check(&world);
}

#[test]
fn a_failed_write_is_put_again_with_identical_bytes() {
    let schema = schema();
    let mut world = opened(5, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let first = put_body(&world, 0);
    world.execute_as(0, Fate::Lost);
    world.execute_as(0, Fate::Answered);
    assert_eq!(put_body(&world, 0), first, "the same nonce and bytes");
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    check(&world);
}

#[test]
fn a_stale_writer_catches_up_through_its_conflicts() {
    let schema = schema();
    let mut world = opened(6, 2);
    for request in 1..=5 {
        let ticket = submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request), 0)]),
        );
        world.drain();
        assert!(matches!(settled(&world, ticket), Settled::Decided(_)));
    }
    let late = submit(&mut world, 1, command(&schema, 9, &[(9, 9)]));
    world.drain();
    assert_eq!(receipt_of(settled(&world, late)).seq, Seq::new(7).unwrap());
    check(&world);
}

#[test]
fn closing_mid_write_leaves_the_outcome_to_resolve() {
    let schema = schema();
    let mut world = opened(7, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let queued = submit(&mut world, 0, command(&schema, 2, &[(2, 2)]));
    world.execute_as(0, Fate::Unanswered);
    world.input(0, Input::Close);
    assert_eq!(settled(&world, a), &Settled::Refused(Refusal::Unknown));
    assert_eq!(
        settled(&world, queued),
        &Settled::Refused(Refusal::NotSubmitted)
    );
    world.drain();
    world.kill(0, false);
    world.start(0);
    world.drain();
    for (request, decided) in [(1u8, true), (2, false)] {
        let request = RequestId([request; 16]);
        let ticket = world.ticket(0, Ask::Resolve(request));
        let resolved = world.await_ticket(0, ticket, Input::Resolve(ticket, request));
        assert!(matches!(resolved, Settled::Resolved(found) if found.is_some() == decided));
    }
    check(&world);
}

#[test]
fn a_sync_sees_every_entry_written_before_it() {
    let schema = schema();
    let mut world = opened(8, 2);
    for request in 1..=3 {
        submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request), 1)]),
        );
        world.drain();
    }
    assert_eq!(world.sync(1), Settled::Synced(Seq::new(4).unwrap()));
    check(&world);
}

#[test]
fn a_cold_replica_reads_the_tail_while_the_newest_checkpoint_downloads() {
    let schema = schema();
    let mut world = World::new(9, 2, &bundle(), Faults::NONE);
    world.clients[0].config.checkpoint.every = 4;
    world.clients[1].config.probe_window = 4;
    world.start(0);
    world.drain();
    for request in 1..=13 {
        submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request % 5), 0)]),
        );
        world.drain();
    }
    let checkpoints = crate::sim::checkpoint_keys(&world);
    let seqs: Vec<u64> = checkpoints.iter().map(|key| key.seq.get()).collect();
    assert_eq!(seqs, [12, 8], "every 4 entries, the newest 2 kept");
    world.start(1);
    let is_list = |request: &IoRequest| matches!(request.op, Op::List { .. });
    assert!(
        world.find(1, is_list).is_some() && world.find(1, get_at(1)).is_some(),
        "the LIST and the first slot go out together"
    );
    world.execute_as(world.find(1, is_list).unwrap(), Fate::Answered);
    let is_image = |request: &IoRequest| matches!(request.op, Op::Get(Target::File(_)));
    assert!(
        world.find(1, is_image).is_some(),
        "the newest checkpoint downloads"
    );
    let mut probed = Vec::new();
    while let Some(index) = world.find(1, |request| request.bucket == Bucket::Log) {
        let (_, request) = world.issued().nth(index).unwrap();
        probed.push(request.key.clone());
        world.execute_as(index, Fate::Answered);
    }
    assert!(
        world.find(1, is_image).is_some() && world.machine(1).replica().head().is_none(),
        "the tail was read before the image arrived"
    );
    let tail: Vec<String> = probed
        .iter()
        .filter(|key| *key != &bumbledb_log::log_key(Seq::GENESIS))
        .cloned()
        .collect();
    assert!(
        tail.iter()
            .all(|key| key.as_str() > bumbledb_log::log_key(Seq::new(12).unwrap()).as_str()),
        "nothing else at or below the checkpoint is fetched: {probed:?}"
    );
    assert!(tail.contains(&bumbledb_log::log_key(Seq::new(14).unwrap())));
    world.drain();
    assert_eq!(head(&world, 1), Seq::new(14).unwrap());
    let opens: Vec<_> = world
        .asks
        .iter()
        .filter(|(_, (client, ask))| *client == 1 && matches!(ask, Ask::Open))
        .map(|(ticket, _)| world.results[ticket].settled.clone())
        .collect();
    assert_eq!(opens, [Settled::Opened { pending: 0 }]);
    check(&world);
}

/// Run `client`'s requests alone until it has none left.
fn run_alone(world: &mut World, client: usize) {
    while let Some(index) = world.find(client, |_| true) {
        world.execute_as(index, Fate::Answered);
    }
}

#[test]
fn a_checkpoint_at_the_tip_is_awaited_without_probing_again() {
    let schema = schema();
    let mut world = World::new(18, 2, &bundle(), Faults::NONE);
    world.clients[0].config.checkpoint.every = 4;
    world.start(0);
    world.drain();
    for request in 1..=11 {
        submit(&mut world, 0, command(&schema, request, &[(1, 0)]));
        world.drain();
    }
    assert_eq!(head(&world, 0), Seq::new(12).unwrap());
    world.start(1);
    let is_list = |request: &IoRequest| matches!(request.op, Op::List { .. });
    world.execute_as(world.find(1, is_list).unwrap(), Fate::Answered);
    let is_image = |request: &IoRequest| matches!(request.op, Op::Get(Target::File(_)));
    let mut gets = 0;
    while let Some(index) = world.find(1, |request| request.bucket == Bucket::Log) {
        gets += 1;
        assert!(gets < 10, "a slot seen empty is not probed again");
        world.execute_as(index, Fate::Answered);
    }
    assert!(world.find(1, is_image).is_some());
    world.drain();
    assert_eq!(head(&world, 1), Seq::new(12).unwrap());
    check(&world);
}

#[test]
fn a_checkpoint_listed_after_the_open_settles_moves_no_head() {
    let schema = schema();
    let mut world = World::new(19, 2, &bundle(), Faults::NONE);
    world.clients[0].config.checkpoint.every = 4;
    world.start(0);
    world.drain();
    world.start(1);
    let is_list = |request: &IoRequest| matches!(request.op, Op::List { .. });
    while let Some(index) = world.find(1, |request| !matches!(request.op, Op::List { .. })) {
        world.execute_as(index, Fate::Answered);
    }
    let opened = world
        .asks
        .iter()
        .find(|(_, (client, ask))| *client == 1 && matches!(ask, Ask::Open))
        .map(|(ticket, _)| *ticket)
        .unwrap();
    assert_eq!(settled(&world, opened), &Settled::Opened { pending: 0 });
    for request in 1..=7 {
        submit(&mut world, 0, command(&schema, request, &[(1, 0)]));
        run_alone(&mut world, 0);
    }
    assert_eq!(
        crate::sim::checkpoint_keys(&world)[0].seq,
        Seq::new(8).unwrap()
    );
    world.execute_as(world.find(1, is_list).unwrap(), Fate::Answered);
    assert_eq!(head(&world, 1), Seq::GENESIS);
    assert_eq!(world.sync(1), Settled::Synced(Seq::new(8).unwrap()));
    check(&world);
}

#[test]
fn a_slow_slot_at_the_head_does_not_idle_catch_up() {
    let schema = schema();
    let mut world = World::new(11, 2, &bundle(), Faults::NONE);
    world.clients[0].config.checkpoint.every = 1_000;
    world.clients[1].config.probe_window = 4;
    world.start(0);
    world.drain();
    for request in 1..=20 {
        submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request), 0)]),
        );
        world.drain();
    }
    world.start(1);
    let applied = |world: &World| {
        world
            .machine(1)
            .replica()
            .state()
            .map_or(0, |state| state.head.seq.get())
    };
    while applied(&world) < 4 {
        world.execute_as(0, Fate::Answered);
    }
    let held = bumbledb_log::log_key(Seq::new(applied(&world) + 1).unwrap());
    let mut probed = Vec::new();
    while let Some(index) = world.find(1, |request| request.key != held) {
        probed.extend(
            world
                .issued()
                .filter(|(client, request)| *client == 1 && request.bucket == Bucket::Log)
                .map(|(_, request)| request.key.clone()),
        );
        world.execute_as(index, Fate::Answered);
    }
    probed.sort();
    probed.dedup();
    let beyond = probed.iter().filter(|key| **key > held).count();
    assert!(
        beyond >= 8,
        "{beyond} slots past the held head were fetched; a window of 4 must keep going"
    );
    world.drain();
    assert_eq!(applied(&world), 21);
    check(&world);
}

#[test]
fn every_log_object_parses_and_carries_a_fresh_nonce() {
    let schema = schema();
    let mut world = opened(10, 2);
    for request in 1..=6 {
        submit(
            &mut world,
            usize::from(request % 2),
            command(&schema, request, &[(1, 1)]),
        );
        world.drain();
    }
    let mut nonces = std::collections::BTreeSet::new();
    for bytes in world.log_entries() {
        let entry = Entry::parse(&bytes).unwrap();
        assert!(nonces.insert(entry.nonce), "nonces are unique");
        if let Body::Commands(batch) = &entry.body {
            assert!(!batch.proposals(&schema).unwrap().is_empty());
        }
    }
    check(&world);
}

#[test]
fn a_database_that_does_not_exist_is_not_created_without_an_identity() {
    let mut world = World::new(11, 1, &bundle(), Faults::NONE);
    world.clients[0].config.create = None;
    world.start(0);
    world.drain();
    let opens: Vec<_> = world
        .results
        .values()
        .map(|result| result.settled.clone())
        .collect();
    assert_eq!(opens, [Settled::Refused(Refusal::NotFound)]);
    assert!(world.log.is_empty());
}

#[test]
fn two_creators_agree_on_one_genesis() {
    let schema = schema();
    let mut world = World::new(12, 2, &bundle(), Faults::NONE);
    world.start(0);
    world.start(1);
    world.drain();
    assert_eq!(world.log.len(), 1);
    let databases: Vec<_> = (0..2)
        .map(|client| {
            world
                .machine(client)
                .replica()
                .state()
                .unwrap()
                .head
                .database
        })
        .collect();
    assert_eq!(databases[0], databases[1]);
    submit(&mut world, 1, command(&schema, 1, &[(1, 1)]));
    world.drain();
    check(&world);
}

#[test]
fn bundles_must_be_nonempty() {
    assert!(matches!(
        Bundle::new(Vec::new()),
        Err(bumbledb_log::BundleError::Empty)
    ));
}

#[test]
fn a_command_for_another_schema_is_refused_and_the_writer_goes_on() {
    let schema = schema();
    let mut world = opened(13, 1);
    let foreign = Command::seal(
        RequestId([1; 16]),
        Precondition::None,
        crate::support::tags(&crate::support::schema2(), &[(1, 1)]),
    );
    let refused = submit(&mut world, 0, foreign);
    world.drain();
    assert_eq!(
        settled(&world, refused),
        &Settled::Refused(Refusal::ForeignSchema)
    );
    let fine = submit(&mut world, 0, command(&schema, 2, &[(1, 1)]));
    world.drain();
    assert!(matches!(settled(&world, fine), Settled::Decided(_)));
    check(&world);
}

#[test]
fn an_entry_stands_by_the_head_it_lands_on() {
    use bumbledb_log::{
        Batch, DatabaseId, Evidence, Freeze, Head, Ledger, Millis, Misplaced, Proposal, Rejection,
        Standing, standing,
    };
    let schema = schema();
    let open = Head {
        database: DatabaseId([1; 16]),
        seq: Seq::new(5).unwrap(),
        revision: Revision(3),
        schema: bundle().initial().fingerprint,
        ledger: Ledger {
            applied: vec![migration_id("0000_init")],
            rejected: vec![Rejection {
                migration: migration_id("0002_bad"),
                evidence: Evidence::new([1].into()).unwrap(),
            }],
        },
        mode: Mode::Open,
    };
    let freeze = |name: &str| Freeze {
        migration: migration_id(name),
        lease_millis: 10,
    };
    let frozen = Head {
        mode: Mode::Frozen {
            freeze: freeze("0001_tags"),
            since: Millis(1),
        },
        ..open.clone()
    };
    let batch = |base: u64, schema_of: &Head| {
        let proposal = Proposal {
            command: command(&schema, 1, &[(1, 1)]),
            outcome: Outcome::NoChange,
        };
        Body::Commands(Batch::new(Seq::new(base).unwrap(), schema_of.schema, &[proposal]).unwrap())
    };
    let other = Head {
        schema: bundle2().steps()[1].fingerprint,
        ..open.clone()
    };
    let at = Seq::new(6).unwrap();
    let stands = |head: &Head, body: &Body| standing(Some(head), at, body);
    assert_eq!(stands(&open, &batch(5, &open)), Ok(Standing::Fresh));
    assert_eq!(stands(&open, &batch(2, &open)), Ok(Standing::Rebased));
    assert_eq!(stands(&open, &batch(2, &other)), Ok(Standing::Void));
    assert_eq!(stands(&frozen, &batch(5, &open)), Ok(Standing::Void));
    assert_eq!(stands(&open, &batch(6, &open)), Err(Misplaced));
    assert_eq!(
        standing(Some(&open), Seq::new(7).unwrap(), &batch(5, &open)),
        Err(Misplaced)
    );
    let freezing = |name: &str| Body::Freeze(freeze(name));
    assert_eq!(stands(&open, &freezing("0001_tags")), Ok(Standing::Fresh));
    assert_eq!(stands(&frozen, &freezing("0001_tags")), Ok(Standing::Void));
    assert_eq!(stands(&open, &freezing("0000_init")), Ok(Standing::Void));
    assert_eq!(stands(&open, &freezing("0002_bad")), Ok(Standing::Void));
}
