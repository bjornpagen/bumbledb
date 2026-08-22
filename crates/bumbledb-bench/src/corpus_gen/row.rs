use bumbledb::{RelationId, Value};

use crate::corpus_gen::{
    AT_BASE, AT_STEP, GenConfig, HOT_SHARE_PCT, HOT_TAG_PCT, MANDATE_SEGMENTS, Rng, Sizes,
    TAG_VARIANTS, mandate_segments, mix,
};
use crate::schema::ids;

fn checked_id(id: u64) -> u64 {
    assert!(id < 1 << 63, "the SQLite mapping axiom: u64 < 2^63");
    id
}

fn tag_pair(seed: u64, pair: u64) -> (u64, u64) {
    let mut rng = Rng::new(mix(seed, ids::POSTING_TAG, pair));
    let first = if rng.chance(HOT_TAG_PCT, 100) {
        0
    } else {
        1 + rng.range(TAG_VARIANTS - 1)
    };
    let second = (first + 1 + rng.range(TAG_VARIANTS - 1)) % TAG_VARIANTS;
    (first, second)
}

/// # Panics
/// Only on programmer-invariant violations: an unknown relation id, or
#[must_use]
pub fn row(cfg: &GenConfig, sizes: &Sizes, rel: RelationId, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(cfg.seed, rel, i));
    match rel {
        ids::HOLDER => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("holder-{i:05}").into()),
        ],
        ids::ACCOUNT => vec![
            Value::U64(checked_id(i)),
            Value::U64(rng.range(sizes.holders)),
            Value::U64(rng.range(3)),
        ],
        ids::INSTRUMENT => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("SYM{i:04}").into()),
        ],
        ids::JOURNAL_ENTRY => vec![
            Value::U64(checked_id(i)),
            Value::U64(rng.range(3)),
            Value::I64(AT_BASE + i64::try_from(i).expect("fits") * AT_STEP * 2),
        ],
        ids::POSTING => {
            let account = if rng.chance(HOT_SHARE_PCT, 100) {
                rng.range(sizes.hot_accounts())
            } else {
                rng.range(sizes.accounts)
            };
            let amount = {
                let magnitude = 1 + rng.range(5_000_000);
                let signed = i64::try_from(magnitude).expect("fits");
                if rng.chance(1, 2) { signed } else { -signed }
            };
            let at = AT_BASE
                + i64::try_from(i).expect("fits") * AT_STEP
                + i64::try_from(rng.range(u64::try_from(AT_STEP).expect("positive")))
                    .expect("fits");
            vec![
                Value::U64(checked_id(i)),
                Value::U64(rng.range(sizes.entries)),
                Value::U64(account),
                Value::U64(rng.range(sizes.instruments)),
                Value::I64(amount),
                Value::I64(at),
            ]
        }
        ids::POSTING_TAG => {
            let posting = (i / 2) * 2;
            let (first, second) = tag_pair(cfg.seed, i / 2);
            vec![
                Value::U64(posting),
                Value::U64(if i.is_multiple_of(2) { first } else { second }),
            ]
        }
        ids::ORG => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("org-{i:02}").into()),
        ],
        ids::ORG_PARENT => {
            let child = i + 1;
            vec![Value::U64(child), Value::U64(child / 2)]
        }
        ids::MANDATE => {
            let account = i / MANDATE_SEGMENTS;
            let k = usize::try_from(i % MANDATE_SEGMENTS).expect("small");
            let segment = mandate_segments(cfg.seed, sizes, account)[k];
            vec![
                Value::U64(checked_id(account)),
                Value::U64(segment.org),
                Value::IntervalI64(
                    bumbledb::Interval::<i64>::new(segment.start, segment.end)
                        .expect("nonempty interval"),
                ),
            ]
        }
        _ => unreachable!("nine ledger relations"),
    }
}

pub fn relation_rows(cfg: GenConfig, rel: RelationId) -> impl Iterator<Item = Vec<Value>> + Clone {
    let sizes = Sizes::of(cfg.scale);
    (0..sizes.rows(rel)).map(move |i| row(&cfg, &sizes, rel, i))
}
