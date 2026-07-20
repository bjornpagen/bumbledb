//! The rings corpus: a power-law hub transfer graph with a planted
//! wash ring, and two bipartite-bomb relations whose exact triangle
//! answer is a construction theorem. Row counts live in [`Sizes`];
//! every param policy upstairs depends only on the fixed value horizon
//! constants, never on `Sizes` — so the smoke gate runs the SAME
//! queries and params as the night run, only smaller.

use bumbledb::{Interval, Value};

use super::ids;
use crate::corpus_gen::Rng;
use crate::scenarios::mix;

/// The corpus row counts — the one axis the smoke twin varies.
pub struct Sizes {
    pub parties: u64,
    pub transfers: u64,
    pub bomb1_m: u64,
    pub bomb2_m: u64,
}

/// The night-run corpus. Tier magnitudes are exponent arithmetic, never
/// timed at authoring: tier 1 m=48 → m³ ≈ 1.1e5 closing probes (sized
/// to finish within the cap); tier 2 m=384 → m³ ≈ 5.7e7, ≥ two decades
/// past tier 1 (the exponent evidence).
pub const FULL: Sizes = Sizes {
    parties: 20_000,
    transfers: 60_000,
    bomb1_m: 48,
    bomb2_m: 384,
};

/// The smoke corpus: same generators, tiny counts — the tier-0 oracle
/// gate's world.
#[cfg(test)]
pub const SMOKE: Sizes = Sizes {
    parties: 64,
    transfers: 400,
    bomb1_m: 6,
    bomb2_m: 8,
};

/// ~0.1% of parties are hubs (never fewer than two).
const fn hubs(parties: u64) -> u64 {
    let h = parties / 1000;
    if h < 2 { 2 } else { h }
}

/// The value horizon — size-independent: the param policies fix their
/// thresholds against these constants, never against [`Sizes`].
pub const RG_BASE: i64 = 1_700_000_000;
pub const RG_HORIZON: i64 = 30_000_000;

fn party_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::PARTY.0, i));
    vec![Value::U64(i), Value::U64(rng.range(4))]
}

/// One endpoint under the hub law: 15% of draws land on a hub.
fn endpoint(rng: &mut Rng, parties: u64, h: u64) -> u64 {
    if rng.chance(3, 20) {
        rng.range(h)
    } else {
        h + rng.range(parties - h)
    }
}

/// The transfer rows: hub-skewed random edges, a 1-in-8 reciprocal
/// echo (same amount, same span, swapped endpoints), then the planted
/// wash ring 0→1→2→0 (amount `9_999`, one identical span). Ids are the
/// row position, so every row is distinct and both engines load
/// identical sets.
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

/// One bipartite bomb: sides A = `0..m` and B = `m..2m`, every cross
/// pair in BOTH directions (2m² rows), then one planted directed
/// triangle on t = {2m, 2m+1, 2m+2}.
///
/// THEOREM (the analytic oracle): the bipartite part is triangle-free.
/// Every generated bipartite edge crosses sides, so a directed 3-cycle
/// inside it would alternate A→B→A→B and close only through an A→A or
/// B→B edge — which this generator cannot emit. The planted ids touch
/// neither side. Hence the triangle query's full binding set is exactly
/// the 3 rotations of the planted cycle, asserted (not eyeballed) in
/// `rings/tests.rs` at smoke scale.
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

/// Every writable relation's rows, in containment order.
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
