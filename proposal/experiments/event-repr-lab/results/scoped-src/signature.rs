//! Exact support-relative pair classification. The caller resolves ownership
//! before supplying resident IDs. Predicate masks are data over fifteen classes.
use super::carrier::*;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Cells,
    Direct,
}
impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cells => "cells",
            Self::Direct => "direct",
        }
    }
}

// Input cell code is 2*a+b. Undo operand sorting, then public complement bits.
pub fn orient(signature: u8, flips: u8, swapped: bool) -> u8 {
    let mut out = 0;
    for cell in 0..4u8 {
        let target = if swapped {
            ((cell & 1) << 1) | ((cell & 2) >> 1)
        } else {
            cell
        } ^ flips;
        out |= ((signature >> cell) & 1) << target;
    }
    out
}

pub fn word_occupancy(support: &[u64], a: &[u64], b: &[u64]) -> u8 {
    assert_eq!(support.len(), a.len());
    assert_eq!(support.len(), b.len());
    let mut sig = 0;
    for ((&s, &a), &b) in support.iter().zip(a).zip(b) {
        sig |= (s & !a & !b != 0) as u8;
        sig |= ((s & !a & b != 0) as u8) << 1;
        sig |= ((s & a & !b != 0) as u8) << 2;
        sig |= ((s & a & b != 0) as u8) << 3;
        if sig == 15 {
            break;
        }
    }
    sig
}

pub fn cells<C: RegionOps>(c: &mut C, a: Id, b: Id) -> u8 {
    let mut signature = 0;
    for cell in 0..4 {
        let region = c.op(1 << cell, a, b);
        signature |= ((region != c.empty()) as u8) << cell;
    }
    signature
}

fn trivial<C: RegionOps>(c: &C, a: Id, b: Id) -> Option<u8> {
    let (empty, full) = (c.empty(), c.full());
    if a == b {
        return Some(if a == empty {
            1
        } else if a == full {
            8
        } else {
            9
        });
    }
    if a == empty {
        return Some(if b == full { 2 } else { 3 });
    }
    if a == full {
        return Some(if b == empty { 4 } else { 12 });
    }
    if b == empty {
        return Some(5);
    }
    if b == full {
        return Some(10);
    }
    if C::BIT_COMPLEMENT && a == (b ^ 1) {
        return Some(6);
    }
    None
}

pub struct Cache {
    scope: u64,
    enabled: bool,
    memo: FxHashMap<(Id, Id), u8>,
}
impl Cache {
    pub fn new(scope: u64, enabled: bool) -> Self {
        Self {
            scope,
            enabled,
            memo: FxHashMap::default(),
        }
    }
    pub fn entries(&self) -> usize {
        self.memo.len()
    }
    pub fn bytes(&self) -> usize {
        self.memo.capacity() * (std::mem::size_of::<((Id, Id), u8)>() + 1)
    }
    pub fn classify<C: RegionOps>(
        &mut self,
        scope: u64,
        c: &mut C,
        a: Id,
        b: Id,
        mode: Mode,
    ) -> u8 {
        assert_eq!(scope, self.scope, "signature memo cannot cross scopes");
        if let Some(sig) = trivial(c, a, b) {
            return sig;
        }
        let (mut left, mut right, flips) = if C::BIT_COMPLEMENT {
            (a & !1, b & !1, (2 * (a & 1) + (b & 1)) as u8)
        } else {
            (a, b, 0)
        };
        let swapped = left > right;
        if swapped {
            std::mem::swap(&mut left, &mut right);
        }
        let key = (left, right);
        if self.enabled {
            if let Some(&sig) = self.memo.get(&key) {
                return orient(sig, flips, swapped);
            }
        }
        let sig = match mode {
            Mode::Cells => cells(c, left, right),
            Mode::Direct => c
                .direct_signature(left, right)
                .expect("direct classification unavailable"),
        };
        assert_ne!(sig, 0, "nonempty admitted scope must occupy a cell");
        if self.enabled {
            self.memo.insert(key, sig);
        }
        orient(sig, flips, swapped)
    }
}

pub fn keep_table(name: &str) -> [u8; 16] {
    std::array::from_fn(|sig| {
        u8::from(
            sig != 0
                && match name {
                    "included" => sig & 4 == 0,
                    "disjoint" => sig & 8 == 0,
                    "overlap" => sig & 8 != 0,
                    _ => panic!("unknown relationship predicate"),
                },
        )
    })
}

pub fn verify<C: Carrier>() {
    let mut cases = 0;
    for support in 1..16u64 {
        let mut c = C::new(&[support], 4);
        let ids: Vec<_> = (0..16).map(|v| c.import(&[v])).collect();
        for enabled in [false, true] {
            let mut cache = Cache::new(123, enabled);
            for a in 0..16usize {
                for b in 0..16usize {
                    let expected = word_occupancy(&[support], &[a as u64], &[b as u64]);
                    if C::DIRECT_SIGNATURE {
                        let before = (c.nodes(), c.bytes());
                        assert_eq!(c.direct_signature(ids[a], ids[b]), Some(expected));
                        assert_eq!(
                            cache.classify(123, &mut c, ids[a], ids[b], Mode::Direct),
                            expected
                        );
                        assert_eq!(
                            (c.nodes(), c.bytes()),
                            before,
                            "read-only classifier grew arena"
                        );
                    }
                    assert_eq!(
                        cache.classify(123, &mut c, ids[a], ids[b], Mode::Cells),
                        expected
                    );
                    // Bypass the outer memo, so cells remain an independent construction.
                    assert_eq!(cells(&mut c, ids[a], ids[b]), expected);
                    cases += 1;
                }
            }
        }
    }
    for n in [65, 257, 1024] {
        let support = bits(n, |w| w % 7 != 0 && w % 5 != 1);
        let mut c = C::new(&support, n);
        let raw: Vec<_> = (0..6)
            .map(|seed| bits(n, |w| mix((seed * n + w) as u64) % 5 < 2))
            .collect();
        let ids: Vec<_> = raw.iter().map(|r| c.import(r)).collect();
        for (i, &a) in ids.iter().enumerate() {
            for (j, &b) in ids.iter().enumerate() {
                let expected = word_occupancy(&support, &raw[i], &raw[j]);
                if C::DIRECT_SIGNATURE {
                    assert_eq!(c.direct_signature(a, b), Some(expected));
                }
                assert_eq!(cells(&mut c, a, b), expected);
                cases += 1;
            }
        }
    }
    // Ownership is checked before any constant/equal-ID result is returned.
    let mut c = C::new(&[15], 4);
    let empty = c.empty();
    let proper = c.import(&[3]);
    let owner = super::transfer::Space::new(c, vec![0, 1]);
    let input = owner.publish(&[empty, proper]);
    let other = super::transfer::Space::new(C::new(&[15], 4), vec![0, 1]);
    assert_eq!(
        other.inspect_inputs(input.keys(), |_, _| -> () {
            panic!("foreign input reached constant shortcut");
        }),
        Err(super::transfer::Error::Scope)
    );
    let invalid = super::transfer::EventKey {
        space: input.keys()[0].space,
        region: u64::MAX - 2,
    };
    assert_eq!(
        owner.inspect_inputs(&[invalid], |_, _| -> () {
            panic!("unpublished input reached classifier");
        }),
        Err(super::transfer::Error::Unpublished)
    );
    let answer = owner
        .inspect_inputs(input.keys(), |c, scope| {
            Cache::new(scope, false).classify(scope, c, empty, proper, Mode::Cells)
        })
        .unwrap();
    assert_eq!(answer, 3);
    println!(
        "EVENT_LAB {{\"kind\":\"signature_verification\",\"candidate\":\"{}\",\"cases\":{},\"direct\":{},\"passed\":true}}",
        C::NAME,
        cases,
        C::DIRECT_SIGNATURE
    );
}

pub fn symbolic<C: Carrier>() {
    if !C::DIRECT_SIGNATURE {
        return;
    }
    let order = (0..30).flat_map(|i| [i, i + 30]).collect();
    let mut c = C::full_space(60, order).unwrap();
    let equal = c
        .diagonal(&(0..30).map(|i| (i, i + 30)).collect::<Vec<_>>())
        .unwrap();
    let bit = c.variable(0);
    let subset = c.op(8, equal, bit);
    assert_eq!(c.direct_signature(equal, subset), Some(13));
    let inverse = c.not(equal);
    let roots = [c.empty(), c.full(), equal, bit, subset, inverse];
    for &a in &roots {
        for &b in &roots {
            let before = (c.nodes(), c.bytes());
            let direct = c.direct_signature(a, b).unwrap();
            assert_eq!((c.nodes(), c.bytes()), before);
            assert_eq!(direct, cells(&mut c, a, b));
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"signature_symbolic\",\"candidate\":\"{}\",\"coordinates\":60,\"cases\":36,\"passed\":true}}",
        C::NAME
    );
}
