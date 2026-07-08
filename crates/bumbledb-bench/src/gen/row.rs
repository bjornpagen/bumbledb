use bumbledb::{RelationId, Value};

use crate::gen::{
    account_tag_pair, GenConfig, Rng, Sizes, AT_BASE, AT_STEP, HOT_SHARE_PCT, MEMO_VOCAB,
    UNIQUE_MEMO_DEN,
};
use crate::schema::ids;

/// The `SQLite` mapping axiom (docs/architecture/50-validation.md): every `u64` stays below
/// 2⁶³ so INTEGER columns compare correctly.
fn checked_id(id: u64) -> u64 {
    assert!(id < 1 << 63, "the SQLite mapping axiom: u64 < 2^63");
    id
}

fn mix(seed: u64, rel: RelationId, row: u64) -> u64 {
    // splitmix-style avalanche over the triple.
    let mut z = seed ^ (u64::from(rel.0) << 56) ^ row;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn memo(rng: &mut Rng, row: u64) -> Value {
    let text = if rng.chance(1, UNIQUE_MEMO_DEN) {
        format!("uniq-{row}")
    } else {
        format!("m{}", rng.range(MEMO_VOCAB))
    };
    Value::String(text.into_bytes().into())
}

/// One relation's row, by index — the pure function everything streams
/// from.
///
/// # Panics
///
/// Only on programmer-invariant violations: an unknown relation id, or
/// derived values exceeding their documented ranges (the size table and
/// the `SQLite` mapping axiom bound everything).
#[must_use]
pub fn row(cfg: &GenConfig, sizes: &Sizes, rel: RelationId, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(cfg.seed, rel, i));
    match rel {
        ids::CURRENCY => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("CUR{i:02}").into_bytes().into()),
        ],
        ids::HOLDER => vec![
            Value::U64(checked_id(i)),
            Value::String(
                format!("holder-{}", rng.range(MEMO_VOCAB))
                    .into_bytes()
                    .into(),
            ),
            Value::Enum(u8::try_from(rng.range(4)).expect("4 regions")),
        ],
        ids::INSTRUMENT => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("SYM{i:04}").into_bytes().into()),
            Value::U64(rng.range(sizes.currencies)),
            Value::Enum(u8::try_from(rng.range(4)).expect("4 kinds")),
        ],
        ids::ACCOUNT => vec![
            Value::U64(checked_id(i)),
            Value::U64(rng.range(sizes.holders)),
            Value::U64(rng.range(sizes.currencies)),
            // status: 90% Open.
            Value::Enum(if rng.chance(9, 10) {
                0
            } else {
                u8::try_from(1 + rng.range(2)).expect("3 statuses")
            }),
            Value::I64(AT_BASE - i64::try_from(rng.range(1 << 30)).expect("fits")),
        ],
        ids::TRANSFER => {
            let mut extref = Vec::with_capacity(16);
            for _ in 0..2 {
                extref.extend_from_slice(&rng.u64().to_le_bytes());
            }
            vec![
                Value::U64(checked_id(i)),
                Value::I64(AT_BASE + i64::try_from(i).expect("fits") * AT_STEP * 2),
                Value::Bytes(extref.into()),
            ]
        }
        ids::POSTING => {
            let account = if rng.chance(HOT_SHARE_PCT, 100) {
                rng.range(sizes.hot_accounts())
            } else {
                rng.range(sizes.accounts)
            };
            let amount = {
                let magnitude = 1 + rng.range(5_000_000);
                let signed = i64::try_from(magnitude).expect("fits");
                if rng.chance(1, 2) {
                    signed
                } else {
                    -signed
                }
            };
            let at = AT_BASE
                + i64::try_from(i).expect("fits") * AT_STEP
                + i64::try_from(rng.range(u64::try_from(AT_STEP).expect("positive")))
                    .expect("fits");
            vec![
                Value::U64(checked_id(i)),
                Value::U64(rng.range(sizes.transfers)),
                Value::U64(account),
                Value::U64(rng.range(sizes.instruments)),
                Value::I64(amount),
                Value::I64(at),
                memo(&mut rng, i),
                Value::Bool(rng.chance(3, 4)),
            ]
        }
        ids::TAG => vec![
            Value::U64(checked_id(i)),
            Value::String(format!("tag-{i:03}").into_bytes().into()),
        ],
        ids::ACCOUNT_TAG => {
            let (account, tag) = account_tag_pair(sizes, i);
            vec![Value::U64(account), Value::U64(tag)]
        }
        ids::TAG_NOTE => {
            // Every 4th AccountTag pair carries a note — a subset by
            // construction, so the compound FK always resolves.
            let (account, tag) = account_tag_pair(sizes, i * 4);
            vec![
                Value::U64(account),
                Value::U64(tag),
                Value::String(
                    format!("note-{}", rng.range(MEMO_VOCAB))
                        .into_bytes()
                        .into(),
                ),
            ]
        }
        _ => unreachable!("nine ledger relations"),
    }
}

/// One relation's full row stream — O(1) memory, deterministically
/// restartable (regenerate, never store).
pub fn relation_rows(cfg: GenConfig, rel: RelationId) -> impl Iterator<Item = Vec<Value>> + Clone {
    let sizes = Sizes::of(cfg.scale);
    (0..sizes.rows(rel)).map(move |i| row(&cfg, &sizes, rel, i))
}
