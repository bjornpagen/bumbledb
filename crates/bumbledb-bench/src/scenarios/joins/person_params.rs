use bumbledb::Value;

use super::{mix, HOT_PEOPLE, PEOPLE};
use crate::gen::Rng;

pub(super) fn person_params(seed: u64, salt: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, salt));
    vec![
        vec![Value::U64(rng.range(HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(PEOPLE + 1_000_000)],
    ]
}
