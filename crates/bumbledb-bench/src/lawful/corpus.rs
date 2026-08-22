//! Every seeded row is legal under the full law roster: task kinds cycle the
//! closed `TaskKind` roster, attempts sit far under the window's cap, steers
//! alternate Observe/Repartition, and scope rows ride only under Repartition
//! steers (the ψ-selected (LAW-2) fills task 0 to the window's cap of 8 before
//! sampling

use bumbledb::{RelationId, Value};

use super::{LawSizes, ids};

pub fn relation_rows(sizes: LawSizes, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {

        ids::TASK => Box::new(
            (0..sizes.tasks).map(|i| vec![Value::U64(i), Value::U64(i % 3), Value::U64(i)]),
        ),

        ids::ATTEMPT => {
            let per = sizes.attempts_per_task;
            Box::new(
                (0..sizes.tasks * per)
                    .map(move |i| vec![Value::U64(i), Value::U64(i / per), Value::U64(i % per)]),
            )
        }
        ids::VERDICT => Box::new(std::iter::empty()),

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

        ids::STEER_SCOPE => Box::new(
            (0..sizes.steers)
                .filter(|j| !j.is_multiple_of(2))
                .map(|j| vec![Value::U64(j), Value::U64(j)]),
        ),
        _ => unreachable!("five ordinary lawful relations"),
    }
}
