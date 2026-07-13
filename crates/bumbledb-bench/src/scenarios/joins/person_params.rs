use bumbledb::Value;

use super::{HOT_PEOPLE, PEOPLE, mix};
use crate::corpus_gen::Rng;

pub(super) fn person_params(seed: u64, salt: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 900, salt));
    vec![
        vec![Value::U64(rng.range(HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(HOT_PEOPLE + rng.range(PEOPLE - HOT_PEOPLE))],
        vec![Value::U64(PEOPLE + 1_000_000)],
    ]
}
