//! Seeded simulations of a schema change under faults: new-code writers
//! race freezes and migrations while old-code writers keep writing until
//! the log moves past their schema.

use bumbledb_log::{Input, Mode, Replica as _, Settled};

use crate::sim::{Ask, Faults, World, check, population, seeds};
use crate::support::{bundle, bundle2, migration_id, schema, schema2, tags};

#[derive(Default)]
struct Seen {
    migrations: usize,
    frozen: usize,
    thawed: usize,
}

/// Clients 0 and 1 ship the migration; 2 and 3 do not.
fn run(seed: u64, steps: usize) -> Seen {
    let bundles = vec![bundle2(), bundle2(), bundle(), bundle()];
    let mut world = World::with_bundles(seed, bundles, Faults::HOSTILE);
    for client in 0..4 {
        world.start(client);
    }
    let migrated_schema = bundle2().steps()[1].fingerprint;
    for _ in 0..steps {
        let client = usize::try_from(world.rng.below(4)).unwrap();
        let new_code = client < 2;
        match world.rng.below(100) {
            0..=19 if world.alive(client) => {
                let at_migrated = world
                    .machine(client)
                    .replica()
                    .head()
                    .is_some_and(|head| head.schema == migrated_schema);
                let schema = if at_migrated { schema2() } else { schema() };
                let (request, command) = world.random_command(client, &schema);
                world.submit(client, request, command);
            }
            20..=23 if new_code && world.alive(client) => {
                let ticket = world.ticket(client, Ask::Freeze);
                let lease = world.rng.below(300);
                world.input(client, Input::Freeze(ticket, lease));
            }
            24..=29 if new_code && world.alive(client) => {
                let Some(base) = world.machine(client).replica().head().map(|head| head.seq) else {
                    continue;
                };
                let label = world.rng.below(4);
                let rows = tags(&schema2(), &[(world.rng.below(3), label)]);
                let ticket = world.ticket(client, Ask::Migrate);
                let population = population(migration_id("0001_tags"), base, rows);
                world.input(client, Input::Migrate(ticket, population));
            }
            30..=33 if world.alive(client) => {
                let ticket = world.ticket(client, Ask::Sync);
                world.input(client, Input::Sync(ticket));
            }
            34 => {
                let wipe = world.rng.chance(30);
                world.kill(client, wipe);
            }
            35..=37 => world.start(client),
            38..=52 => world.deliver_one(),
            _ => world.execute_one(),
        }
    }
    world.quiesce();
    for client in 0..4 {
        let synced = world.sync(client);
        assert!(
            matches!(synced, Settled::Synced(_) | Settled::Refused(_)),
            "seed {seed}: client {client}: {synced:?}"
        );
    }
    let states = check(&world);
    let migrations = states
        .windows(2)
        .filter(|pair| pair[0].head.schema != pair[1].head.schema)
        .count();
    assert!(migrations <= 1, "seed {seed}: one schema change at most");
    let frozen = world
        .results
        .values()
        .filter(|result| matches!(result.settled, Settled::Frozen(_)))
        .count();
    let thawed = states
        .windows(2)
        .filter(|pair| {
            matches!(pair[0].head.mode, Mode::Frozen { .. })
                && pair[1].head.mode == Mode::Open
                && pair[0].head.schema == pair[1].head.schema
        })
        .count();
    Seen {
        migrations,
        frozen,
        thawed,
    }
}

#[test]
fn writers_of_two_code_versions_race_a_migration() {
    let mut total = Seen::default();
    for seed in seeds(300..340) {
        let seen = run(seed, 500);
        total.migrations += seen.migrations;
        total.frozen += seen.frozen;
        total.thawed += seen.thawed;
    }
    assert!(
        total.migrations > 20 && total.frozen > 0 && total.thawed > 0,
        "migrated {}, frozen {}, thawed {}",
        total.migrations,
        total.frozen,
        total.thawed
    );
}
