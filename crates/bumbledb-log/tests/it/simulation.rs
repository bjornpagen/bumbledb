//! Seeded simulations of several writers on one log under injected faults.
//! Every run replays the final log through the reference model and checks
//! every settled ticket and every replica against it.

use bumbledb_log::{Input, Refusal, Settled};

use crate::sim::{Ask, Faults, Tally, World, check};
use crate::support::{bundle, schema};

/// What the run settled, beside what the world tallied.
#[derive(Debug, Default)]
struct Outcomes {
    tally: Tally,
    decided: u64,
    unknown: u64,
    reused: u64,
    batched: u64,
}

fn run(seed: u64, clients: usize, faults: Faults, steps: usize) -> Outcomes {
    let schema = schema();
    let mut world = World::new(seed, clients, &bundle(), faults);
    for client in 0..clients {
        world.start(client);
    }
    for _ in 0..steps {
        let client = usize::try_from(world.rng.below(clients as u64)).unwrap();
        match world.rng.below(100) {
            0..=24 if world.alive(client) => {
                let (request, command) = world.random_command(client, &schema);
                world.submit(client, request, command);
            }
            25..=29 if world.alive(client) => {
                let ticket = world.ticket(client, Ask::Sync);
                world.input(client, Input::Sync(ticket));
            }
            30..=33 if world.alive(client) && !world.commands.is_empty() => {
                let pick = usize::try_from(world.rng.below(world.commands.len() as u64)).unwrap();
                let request = *world.commands.keys().nth(pick).expect("picked");
                let ticket = world.ticket(client, Ask::Resolve(request));
                world.input(client, Input::Resolve(ticket, request));
            }
            34..=35 => {
                if world.alive(client) && world.rng.chance(50) {
                    world.input(client, Input::Close);
                }
                let wipe = world.rng.chance(30);
                world.kill(client, wipe);
            }
            36..=39 => world.start(client),
            40..=54 => world.deliver_one(),
            _ => world.execute_one(),
        }
    }
    world.quiesce();
    for client in 0..clients {
        match world.sync(client) {
            Settled::Synced(_) => {}
            other => panic!("seed {seed}: client {client} failed to sync: {other:?}"),
        }
    }
    check(&world);
    let heads: Vec<_> = (0..clients)
        .map(|client| {
            world
                .machine(client)
                .replica()
                .state()
                .map(|state| state.head.seq)
        })
        .collect();
    assert!(
        heads.windows(2).all(|pair| pair[0] == pair[1]),
        "seed {seed}: synced clients share the tip: {heads:?}"
    );
    for client in 0..clients {
        world.input(client, Input::Close);
    }
    for (ticket, (owner, _)) in &world.asks {
        assert!(
            world.results.contains_key(ticket) || world.abandoned.contains(ticket),
            "seed {seed}: ticket {ticket:?} of client {owner} never settled"
        );
    }
    let count = |pick: fn(&Settled) -> bool| {
        world
            .results
            .values()
            .filter(|result| pick(&result.settled))
            .count() as u64
    };
    let batched = world
        .log_entries()
        .iter()
        .filter_map(|bytes| bumbledb_log::Entry::parse(&schema, bytes).ok())
        .filter(|entry| matches!(&entry.body, bumbledb_log::Body::Commands(decided) if decided.len() > 1))
        .count() as u64;
    Outcomes {
        tally: world.tally,
        decided: count(|settled| matches!(settled, Settled::Decided(_))),
        unknown: count(|settled| *settled == Settled::Refused(Refusal::Unknown)),
        reused: count(|settled| matches!(settled, Settled::Refused(Refusal::RequestReused(_)))),
        batched,
    }
}

fn sweep(seeds: std::ops::Range<u64>, clients: usize, faults: Faults, steps: usize) -> Outcomes {
    let mut total = Outcomes::default();
    for seed in seeds {
        let outcomes = run(seed, clients, faults, steps);
        total.tally += outcomes.tally;
        total.decided += outcomes.decided;
        total.unknown += outcomes.unknown;
        total.reused += outcomes.reused;
        total.batched += outcomes.batched;
    }
    total
}

#[test]
fn hostile_store_with_three_writers() {
    let total = sweep(0..48, 3, Faults::HOSTILE, 500);
    let tally = total.tally;
    assert!(total.decided > 1_000, "{total:?}");
    assert!(
        tally.occupied > 0 && tally.unanswered > 0 && tally.lost > 0,
        "{total:?}"
    );
    assert!(
        tally.delayed > 0 && tally.kills > 0 && tally.installs > 0,
        "{total:?}"
    );
    assert!(
        total.unknown > 0 && total.reused > 0 && total.batched > 0,
        "{total:?}"
    );
}

#[test]
fn honest_store_with_five_writers() {
    let total = sweep(100..116, 5, Faults::NONE, 400);
    assert!(total.tally.occupied > 0 && total.batched > 0, "{total:?}");
}

#[test]
fn lone_writer_through_hostile_store() {
    let total = sweep(200..232, 1, Faults::HOSTILE, 400);
    assert!(
        total.decided > 500 && total.tally.unanswered > 0,
        "{total:?}"
    );
}
