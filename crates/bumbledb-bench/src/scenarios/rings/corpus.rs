use bumbledb::{Interval, Value};

use super::ids;
use crate::corpus_gen::Rng;
use crate::scenarios::mix;

pub struct Sizes {
    pub parties: u64,
    pub transfers: u64,
    pub bomb1_m: u64,
    pub bomb2_m: u64,
}

pub const FULL: Sizes = Sizes {
    parties: 20_000,
    transfers: 60_000,
    bomb1_m: 48,
    bomb2_m: 384,
};

#[cfg(test)]
pub const SMOKE: Sizes = Sizes {
    parties: 64,
    transfers: 400,
    bomb1_m: 6,
    bomb2_m: 8,
};

const fn hubs(parties: u64) -> u64 {
    let h = parties / 1000;
    if h < 2 { 2 } else { h }
}

pub const RG_BASE: i64 = 1_700_000_000;
pub const RG_HORIZON: i64 = 30_000_000;

fn party_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::PARTY.0, i));
    vec![Value::U64(i), Value::U64(rng.range(4))]
}

fn endpoint(rng: &mut Rng, parties: u64, h: u64) -> u64 {
    if rng.chance(3, 20) {
        rng.range(h)
    } else {
        h + rng.range(parties - h)
    }
}

fn transfers(seed: u64, z: &Sizes) -> Vec<Vec<Value>> {
    let h = hubs(z.parties);
    let horizon = u64::try_from(RG_HORIZON - 200_000).expect("positive horizon");
    let mut out: Vec<Vec<Value>> = Vec::new();
    let row = |out: &mut Vec<Vec<Value>>, src: u64, dst: u64, amount: i64, span: Value| {
        let id = u64::try_from(out.len()).expect("fits");
        out.push(vec![
            Value::U64(id),
            Value::U64(src),
            Value::U64(dst),
            Value::I64(amount),
            span,
        ]);
    };
    for i in 0..z.transfers {
        let mut rng = Rng::new(mix(seed, ids::TRANSFER.0, i));
        let src = endpoint(&mut rng, z.parties, h);
        let dst = endpoint(&mut rng, z.parties, h);
        let amount = i64::try_from(rng.range(10_000)).expect("small");
        let s = i64::try_from(rng.range(horizon)).expect("fits");
        let w = 1 + i64::try_from(rng.range(172_800)).expect("small");
        let span = Value::IntervalI64(
            Interval::<i64>::new(RG_BASE + s, RG_BASE + s + w).expect("positive width"),
        );
        row(&mut out, src, dst, amount, span.clone());
        if rng.chance(1, 8) {
            row(&mut out, dst, src, amount, span);
        }
    }
    let ring_span = Value::IntervalI64(
        Interval::<i64>::new(RG_BASE + 1_000, RG_BASE + 2_000).expect("nonempty"),
    );
    for (src, dst) in [(0u64, 1u64), (1, 2), (2, 0)] {
        row(&mut out, src, dst, 9_999, ring_span.clone());
    }
    out
}

fn bomb(m: u64) -> Vec<Vec<Value>> {
    let mut out = Vec::new();
    for a in 0..m {
        for b in m..(2 * m) {
            out.push(vec![Value::U64(a), Value::U64(b)]);
            out.push(vec![Value::U64(b), Value::U64(a)]);
        }
    }
    for (src, dst) in [
        (2 * m, 2 * m + 1),
        (2 * m + 1, 2 * m + 2),
        (2 * m + 2, 2 * m),
    ] {
        out.push(vec![Value::U64(src), Value::U64(dst)]);
    }
    out
}

fn rows(seed: u64, z: &Sizes) -> super::Rows {
    vec![
        (
            ids::PARTY,
            Box::new((0..z.parties).map(move |i| party_row(seed, i)))
                as Box<dyn Iterator<Item = Vec<Value>>>,
        ),
        (ids::TRANSFER, Box::new(transfers(seed, z).into_iter())),
        (ids::BOMB1, Box::new(bomb(z.bomb1_m).into_iter())),
        (ids::BOMB2, Box::new(bomb(z.bomb2_m).into_iter())),
    ]
}

pub(super) fn rows_full(seed: u64) -> super::Rows {
    rows(seed, &FULL)
}

#[cfg(test)]
pub(super) fn rows_smoke(seed: u64) -> super::Rows {
    rows(seed, &SMOKE)
}
