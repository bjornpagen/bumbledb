use super::blocks::Block64;
use super::carrier::*;
use super::diagram::{Anchored, Diagram};
use super::finite::*;
use super::packed::Packed;
use super::transfer::*;
use std::sync::Arc;

pub struct Fixture {
    pub packet: Packet,
    pub support: Vec<u64>,
    pub expected: Vec<Vec<u64>>,
    pub n: usize,
}
fn source<C: Carrier>(fixtures: &mut Vec<Fixture>) {
    for nbits in [3, 8] {
        let n = 1usize << nbits;
        let support = bits(n, |w| w % 5 != 0);
        let bank: Vec<_> = (0..4)
            .map(|i| bits(n, |w| mix(w as u64 + i * 91) % 3 != 0))
            .collect();
        for reverse in [false, true] {
            let order = if reverse {
                (0..nbits).collect()
            } else {
                (0..nbits).rev().collect()
            };
            let mut c = C::with_order(&support, n, order);
            let mut roots: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
            roots.extend([c.full(), c.empty()]);
            let neg = c.not(roots[0]);
            roots.push(neg);
            let expected = roots.iter().map(|&r| c.export(r)).collect();
            let packet = c.packet(&(0..nbits as u64).collect::<Vec<_>>(), &roots);
            assert_eq!(Packet::decode(&packet.encode()).unwrap(), packet);
            // Same layout + denotations, unrelated resident allocation order.
            let mut other = C::with_order(
                &support,
                n,
                if reverse {
                    (0..nbits).collect()
                } else {
                    (0..nbits).rev().collect()
                },
            );
            other.import(&bits(n, |w| w % 7 == 0));
            let mut imported = vec![0; bank.len()];
            for i in (0..bank.len()).rev() {
                imported[i] = other.import(&bank[i]);
            }
            imported.extend([other.full(), other.empty()]);
            let neg = other.not(imported[0]);
            imported.push(neg);
            assert_eq!(
                packet.encode(),
                other.packet(&packet.names, &imported).encode(),
                "{} allocation independence",
                C::NAME
            );
            fixtures.push(Fixture {
                packet,
                support: support.clone(),
                expected,
                n,
            });
        }
    }
}
pub fn fixtures() -> Vec<Fixture> {
    let mut f = vec![];
    macro_rules! add {($($t:ty),*)=>{$(source::<$t>(&mut f);)*};}
    add!(Finite<Dense>,Finite<Dense<true>>,Finite<Sparse>,Finite<Roaring>,Finite<Runs>,Diagram<2,false>,Diagram<2,true>,Diagram<4,true>,Anchored,Packed<1>,Packed<4>,Packed<8>,Packed<64>,Block64);
    f
}
fn target<T: Carrier>(fixtures: &[Fixture]) {
    for f in fixtures {
        let nbits = f.packet.names.len() as u32;
        let c = T::with_order(&f.support, f.n, (0..nbits).collect());
        let owner = Space::new(c, f.packet.names.clone());
        let images: Vec<_> = (0..nbits).collect();
        let batch = restore::<T, T>(&f.packet, &owner, &images, MapKind::Extension).unwrap();
        assert_eq!(
            batch.export(),
            f.expected,
            "{} cross-carrier import",
            T::NAME
        );
        // Repeated import converges to exactly the destination's published keys.
        let again = restore::<T, T>(&f.packet, &owner, &images, MapKind::Extension).unwrap();
        assert_eq!(batch.keys(), again.keys());
        assert_eq!(
            Packet::decode(&batch.packet().encode()).unwrap(),
            batch.packet()
        );
        assert_eq!(
            owner.resolve(EventKey {
                space: batch.keys()[0].space,
                region: u64::MAX
            }),
            Err(Error::Unpublished)
        );
        let other = Space::new(
            T::with_order(&f.support, f.n, (0..nbits).rev().collect()),
            f.packet.names.clone(),
        );
        assert_eq!(other.resolve(batch.keys()[0]), Err(Error::Scope));
        let weak = Arc::downgrade(&owner);
        drop(owner);
        drop(again);
        assert!(weak.upgrade().is_some());
        assert_eq!(Arc::strong_count(&batch.owner), 1);
        let copied = batch.keys().to_vec();
        assert_eq!(Arc::strong_count(&batch.owner), 1);
        assert_eq!(copied, batch.keys());
        drop(batch);
        assert!(weak.upgrade().is_none());
    }
    println!(
        "EVENT_LAB {{\"kind\":\"transfer_finite\",\"target\":\"{}\",\"source_packets\":{},\"passed\":true}}",
        T::NAME,
        fixtures.len()
    );
}
fn maps() {
    // Exhaustive finite support projections under an actual coordinate embedding.
    let mut cases = 0;
    for source_support in 1..16 {
        let source = Finite::<Dense>::new(&[source_support], 4).packet(&[10, 11], &[]);
        for target_support in 1..256 {
            let target = Finite::<Dense>::new(&[target_support], 8).packet(&[20, 21, 22], &[]);
            let projected = (0..8).fold(0u64, |v, w| {
                v | (((target_support >> w) & 1) << (((w >> 2) & 1) | ((w & 1) << 1)))
            });
            let expected = if projected & !source_support != 0 {
                Err(Error::OutsideSource)
            } else if projected != source_support {
                Err(Error::NotTotal)
            } else {
                Ok(())
            };
            assert_eq!(
                check_map::<Packed<8>>(&source, &target, &[2, 0], MapKind::Extension),
                expected
            );
            assert_eq!(
                check_map::<Packed<8>>(&source, &target, &[2, 0], MapKind::Restriction),
                if projected & !source_support == 0 {
                    Ok(())
                } else {
                    Err(Error::OutsideSource)
                }
            );
            cases += 1;
        }
    }
    // Lifting into a correlated extension, then an explicit restriction.
    let mut c = Packed::<8>::new(&[15], 4);
    let a = c.import(&[10]);
    let neg = c.not(a);
    let p = c.packet(&[10, 11], &[a, neg, c.full(), c.empty()]);
    let support = bits(8, |w| ((w >> 1) & 1) == ((w >> 2) & 1));
    let owner = Space::new(Block64::new(&support, 8), vec![20, 21, 22]);
    let batch = restore::<Packed<8>, Block64>(&p, &owner, &[2, 0], MapKind::Extension).unwrap();
    let expected: Vec<_> = [10u64, 5, 15, 0]
        .iter()
        .map(|mask| {
            bits(8, |w| {
                at(&support, w) && mask >> (((w >> 2) & 1) | ((w & 1) << 1)) & 1 != 0
            })
        })
        .collect();
    assert_eq!(batch.export(), expected);
    let restricted = Space::new(Block64::new(&[3], 8), vec![20, 21, 22]);
    assert!(matches!(
        restore::<Packed<8>, Block64>(&p, &restricted, &[2, 0], MapKind::Extension),
        Err(Error::NotTotal)
    ));
    let q = restore::<Packed<8>, Block64>(&p, &restricted, &[2, 0], MapKind::Restriction).unwrap();
    assert_eq!(q.export()[2], vec![3]);
    for bad in [vec![0, 0], vec![0, 3], vec![0]] {
        assert_eq!(
            check_map::<Packed<8>>(&p, &owner.support_packet(), &bad, MapKind::Extension),
            Err(Error::Map)
        );
    }
    let mut empty = p.clone();
    empty.support = 0;
    assert_eq!(
        check_map::<Packed<8>>(&empty, &owner.support_packet(), &[2, 0], MapKind::Extension),
        Err(Error::EmptySupport)
    );
    println!(
        "EVENT_LAB {{\"kind\":\"transfer_support_maps\",\"cases\":{},\"passed\":true}}",
        cases
    );
}
fn malformed(p: &Packet) {
    let bytes = p.encode();
    for end in 0..bytes.len() {
        assert!(Packet::decode(&bytes[..end]).is_err());
    }
    let mut b = bytes.clone();
    b.push(0);
    assert!(Packet::decode(&b).is_err());
    let mut b = bytes;
    b[0] ^= 1;
    assert!(Packet::decode(&b).is_err());
    let mut bad = p.clone();
    bad.roots.push(u32::MAX);
    assert_eq!(bad.validate(), Err(Error::Reference));
    let mut bad = p.clone();
    bad.names[1] = bad.names[0];
    assert_eq!(bad.validate(), Err(Error::Coordinate));
    let mut bad = p.clone();
    bad.order[1] = bad.order[0];
    assert_eq!(bad.validate(), Err(Error::Coordinate));
    let mut bad = p.clone();
    bad.nodes = vec![Node::Split {
        coords: vec![0],
        children: vec![0, 2],
    }];
    bad.roots = vec![];
    bad.support = 0;
    assert_eq!(bad.validate(), Err(Error::Reference));
    bad.nodes = vec![Node::Table {
        coords: vec![0],
        words: vec![4],
    }];
    assert_eq!(bad.validate(), Err(Error::Shape));
    bad.nodes = vec![
        Node::Table {
            coords: vec![0],
            words: vec![1],
        },
        Node::Split {
            coords: vec![0],
            children: vec![0, 2],
        },
    ];
    assert_eq!(bad.validate(), Err(Error::Order));
}
fn high_source<C: Carrier>(packets: &mut Vec<Packet>) {
    let order = (0..20).rev().flat_map(|i| [i + 20, i]).collect();
    let mut c = C::full_space(40, order).unwrap();
    let mut equal = c.full();
    for i in 0..20 {
        let a = c.variable(i);
        let b = c.variable(i + 20);
        let eq = c.op(9, a, b);
        equal = c.op(8, equal, eq);
    }
    let neg = c.not(equal);
    let p = c.packet(
        &(0..40).collect::<Vec<_>>(),
        &[equal, neg, c.full(), c.empty()],
    );
    assert_eq!(Packet::decode(&p.encode()).unwrap(), p);
    packets.push(p);
}
fn high_target<C: Carrier>(packets: &[Packet]) {
    for p in packets {
        let order = (0..20).flat_map(|i| [i, i + 20]).collect();
        let mut c = C::full_space(40, order).unwrap();
        let mut equal = c.full();
        for i in (0..20).rev() {
            let a = c.variable(i);
            let b = c.variable(i + 20);
            let eq = c.op(9, a, b);
            equal = c.op(8, equal, eq);
        }
        let neg = c.not(equal);
        let expected = vec![equal, neg, c.full(), c.empty()];
        let owner = Space::new(c, (0..40).collect());
        let canonical = owner.publish(&expected);
        let out =
            restore::<C, C>(p, &owner, &(0..40).collect::<Vec<_>>(), MapKind::Extension).unwrap();
        assert_eq!(
            out.counts(),
            vec![1 << 20, (1u64 << 40) - (1 << 20), 1 << 40, 0]
        );
        assert_eq!(out.keys(), canonical.keys());
        // Check destination canonical identity against an independently built equality.
        let again =
            restore::<C, C>(p, &owner, &(0..40).collect::<Vec<_>>(), MapKind::Extension).unwrap();
        assert_eq!(out.keys(), again.keys());
    }
}
pub fn all() {
    assert_eq!(std::mem::size_of::<EventKey>(), 16);
    let f = fixtures();
    malformed(&f[0].packet);
    macro_rules! targets {($($t:ty),*)=>{$(target::<$t>(&f);)*};}
    targets!(Finite<Dense>,Finite<Dense<true>>,Finite<Sparse>,Finite<Roaring>,Finite<Runs>,Diagram<2,false>,Diagram<2,true>,Diagram<4,true>,Anchored,Packed<1>,Packed<4>,Packed<8>,Packed<64>,Block64);
    maps();
    let mut packets = vec![];
    macro_rules! sources {($($t:ty),*)=>{$(high_source::<$t>(&mut packets);)*};}
    sources!(Diagram<2,false>,Diagram<2,true>,Diagram<4,true>,Anchored,Packed<1>,Packed<4>,Packed<8>,Packed<64>,Block64);
    macro_rules! high {($($t:ty),*)=>{$(high_target::<$t>(&packets);)*};}
    high!(Diagram<2,false>,Diagram<2,true>,Diagram<4,true>,Anchored,Packed<1>,Packed<4>,Packed<8>,Packed<64>,Block64);
    println!(
        "EVENT_LAB {{\"kind\":\"transfer_symbolic\",\"bits\":40,\"source_target_pairs\":81,\"packet_bytes\":{:?},\"passed\":true}}",
        packets.iter().map(|p| p.encode().len()).collect::<Vec<_>>()
    );
}
