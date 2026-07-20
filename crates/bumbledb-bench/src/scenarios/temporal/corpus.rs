//! The temporal corpus: a fixed-horizon interval world whose stress
//! cases are construction laws, never query-side filters. Row counts
//! live in [`Sizes`]; every param policy upstairs depends only on the
//! fixed value horizon constants ([`TP_BASE`], [`TP_HORIZON`]), never
//! on `Sizes` — so the smoke gate runs the SAME queries and params as
//! the night run, only smaller. Key 0 is the deterministic Zipf head
//! (a 1-in-50 redirect) at BOTH scales.

use bumbledb::{Interval, Value};

use super::ids;
use crate::corpus_gen::Rng;
use crate::scenarios::mix;

/// The corpus row counts — the one axis the smoke twin varies.
pub struct Sizes {
    pub keys: u64,
    pub spans: u64,
}

/// The night-run corpus.
pub const FULL: Sizes = Sizes {
    keys: 2_000,
    spans: 150_000,
};

/// The smoke corpus: same generators, tiny counts — the tier-0 oracle
/// gate's world.
#[cfg(test)]
pub const SMOKE: Sizes = Sizes {
    keys: 8,
    spans: 240,
};

/// The value horizon — size-independent: the param policies fix their
/// instants against these constants, never against [`Sizes`].
pub const TP_BASE: i64 = 1_700_000_000;
pub const TP_HORIZON: i64 = 30_000_000;

/// The planted rows' fixed weight — in the random rows' `0..1_000`
/// range; corpus texture only, no t1–t4 family binds the field.
const PLANTED_WEIGHT: i64 = 500;

/// The span rows. THE CORPUS LAW (t4 depends on it): every bounded
/// span ends strictly inside `[TP_BASE, TP_BASE + TP_HORIZON)` — starts
/// are drawn below `TP_HORIZON − 200_000` and widths cap at `172_800`
/// (two days), so `end < TP_BASE + TP_HORIZON` always. Rays are
/// `end == i64::MAX` interval values (`Interval::<i64>::ray` — the
/// engine's own ray representation), ~2% of rows, starting below
/// `TP_HORIZON − 1_000_000`; past the horizon only rays cover an
/// instant, so "the family whose answers are exactly the rays" is the
/// stabbing query at a post-horizon coordinate, no ray predicate
/// anywhere. After the random loop, deterministic witnesses land on
/// the low keys that exist at every scale: per key `k in 0..8` one
/// exact-abutment MEETS pair and one strict-containment DURING pair,
/// then two planted rays on keys 0 and 1 (so ≥ 2 rays exist even at
/// SMOKE). Ids are the row position, so every row is distinct and both
/// engines load identical sets.
pub(super) fn spans(seed: u64, z: &Sizes) -> Vec<Vec<Value>> {
    let ray_room = u64::try_from(TP_HORIZON - 1_000_000).expect("positive ray room");
    let start_room = u64::try_from(TP_HORIZON - 200_000).expect("positive start room");
    let mut out: Vec<Vec<Value>> = Vec::new();
    let row = |out: &mut Vec<Vec<Value>>, key: u64, span: Value, weight: i64| {
        let id = u64::try_from(out.len()).expect("fits");
        out.push(vec![
            Value::U64(id),
            Value::U64(key),
            span,
            Value::I64(weight),
        ]);
    };
    for i in 0..z.spans {
        let mut rng = Rng::new(mix(seed, ids::SPAN.0, i));
        let key = if rng.chance(1, 50) {
            // The deterministic Zipf head — key 0 is the heavy key at
            // both scales.
            0
        } else {
            rng.range(z.keys)
        };
        let span = if rng.chance(1, 50) {
            let s = i64::try_from(rng.range(ray_room)).expect("fits");
            Value::IntervalI64(
                Interval::<i64>::ray(TP_BASE + s).expect("ray start below the ceiling"),
            )
        } else {
            let s = i64::try_from(rng.range(start_room)).expect("fits");
            let w = 1 + i64::try_from(rng.range(172_800)).expect("small");
            Value::IntervalI64(
                Interval::<i64>::new(TP_BASE + s, TP_BASE + s + w).expect("positive width"),
            )
        };
        let weight = i64::try_from(rng.range(1_000)).expect("small");
        row(&mut out, key, span, weight);
    }
    // The planted MEETS witnesses: exact abutment — left.end ==
    // right.start, one pair per low key.
    for k in 0i64..8 {
        let key = u64::try_from(k).expect("small");
        let left_start = TP_BASE + 3_000_000 + k * 20_000;
        let left_end = left_start + 3_600;
        row(
            &mut out,
            key,
            Value::IntervalI64(Interval::<i64>::new(left_start, left_end).expect("positive width")),
            PLANTED_WEIGHT,
        );
        row(
            &mut out,
            key,
            Value::IntervalI64(
                Interval::<i64>::new(left_end, left_end + 3_600).expect("positive width"),
            ),
            PLANTED_WEIGHT,
        );
    }
    // The planted DURING witnesses: the inner strictly inside the
    // outer, one pair per low key.
    for k in 0i64..8 {
        let key = u64::try_from(k).expect("small");
        let outer_start = TP_BASE + 6_000_000 + k * 20_000;
        row(
            &mut out,
            key,
            Value::IntervalI64(
                Interval::<i64>::new(outer_start, outer_start + 100_000).expect("positive width"),
            ),
            PLANTED_WEIGHT,
        );
        row(
            &mut out,
            key,
            Value::IntervalI64(
                Interval::<i64>::new(outer_start + 10_000, outer_start + 11_000)
                    .expect("positive width"),
            ),
            PLANTED_WEIGHT,
        );
    }
    // The planted rays: keys 0 and 1 — ≥ 2 rays exist even at SMOKE.
    for key in [0u64, 1] {
        row(
            &mut out,
            key,
            Value::IntervalI64(
                Interval::<i64>::ray(TP_BASE + 123_456).expect("ray start below the ceiling"),
            ),
            PLANTED_WEIGHT,
        );
    }
    out
}

/// Every writable relation's rows, in containment order.
fn rows(seed: u64, z: &Sizes) -> super::Rows {
    vec![
        (
            ids::KEY,
            Box::new((0..z.keys).map(|i| vec![Value::U64(i)]))
                as Box<dyn Iterator<Item = Vec<Value>>>,
        ),
        (ids::SPAN, Box::new(spans(seed, z).into_iter())),
    ]
}

pub(super) fn rows_full(seed: u64) -> super::Rows {
    rows(seed, &FULL)
}

#[cfg(test)]
pub(super) fn rows_smoke(seed: u64) -> super::Rows {
    rows(seed, &SMOKE)
}
