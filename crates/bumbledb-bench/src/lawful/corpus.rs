//! The lawful corpus: every seeded row is a pure function of the sizes
//! alone (no RNG anywhere — the corpus is index arithmetic, so both
//! twins and the naive model derive the identical mass from
//! [`LawSizes`]). Every seeded row is legal under the full law roster:
//! task kinds cycle the closed `TaskKind` roster, attempts sit far
//! under the window's cap, steers alternate Observe/Repartition, and
//! scope rows ride only under Repartition steers (the ψ-selected
//! containment's selected targets).
//!
//! Task 0 is the `reject_window` lane's cap target: that lane's setup
//! (LAW-2) fills task 0 to the window's cap of 8 before sampling
//! refusals, so legal write streams round-robin tasks `1..tasks` and
//! never collide with the saturated parent.

use bumbledb::{RelationId, Value};

use super::{LawSizes, ids};

/// One relation's full seeded row stream, in field-declaration order.
/// `Verdict` seeds none: verdicts are what the judged write lanes mint,
/// and an empty relation keeps every seeded key and containment
/// trivially clean on both twins.
#[must_use]
pub fn relation_rows(sizes: LawSizes, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        // Task i: kind cycles the three-row TaskKind roster, subject is
        // the index (the identity key (kind, subject) stays unique).
        ids::TASK => Box::new(
            (0..sizes.tasks).map(|i| vec![Value::U64(i), Value::U64(i % 3), Value::U64(i)]),
        ),
        // Attempt i: `attempts_per_task` per task, n counting up from 0
        // (2 per task at every scale, n ∈ {0, 1} — far under the cap).
        ids::ATTEMPT => {
            let per = sizes.attempts_per_task;
            Box::new(
                (0..sizes.tasks * per)
                    .map(move |i| vec![Value::U64(i), Value::U64(i / per), Value::U64(i % per)]),
            )
        }
        ids::VERDICT => Box::new(std::iter::empty()),
        // Steer j: even = Observe (0), odd = Repartition (1); the task
        // reference strides the task space coprime to it.
        ids::STEER => {
            let tasks = sizes.tasks;
            Box::new((0..sizes.steers).map(move |j| {
                vec![
                    Value::U64(j),
                    Value::U64(j % 2),
                    Value::U64((j * 7) % tasks),
                ]
            }))
        }
        // One scope row per ODD (Repartition) steer — the ψ-selected
        // containment holds by construction.
        ids::STEER_SCOPE => Box::new(
            (0..sizes.steers)
                .filter(|j| !j.is_multiple_of(2))
                .map(|j| vec![Value::U64(j), Value::U64(j)]),
        ),
        _ => unreachable!("five ordinary lawful relations"),
    }
}
