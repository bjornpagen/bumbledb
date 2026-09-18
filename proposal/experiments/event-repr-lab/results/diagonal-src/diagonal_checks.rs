//! Coordinate equalities and counter relations, with no world enumeration in
//! the symbolic constructors. Small tables are an independent differential oracle.
use super::carrier::*;
use super::relational_core::Algebra;

pub fn finite<C: Carrier>() {
    let mut cases = 0;
    for support in 1..256u64 {
        let mut c = C::new(&[support], 8);
        for pairs in [
            vec![],
            vec![(0, 0)],
            vec![(0, 1)],
            vec![(0, 1), (1, 2)],
            vec![(0, 1), (1, 0), (0, 1)],
            vec![(0, 2), (1, 2)],
        ] {
            let expected = support
                & bits(8, |w| {
                    pairs.iter().all(|&(a, b)| ((w >> a) ^ (w >> b)) & 1 == 0)
                })[0];
            let native = c.diagonal(&pairs).unwrap();
            assert_eq!(c.export(native), vec![expected]);
            assert_eq!(symbolic_diagonal(&mut c, &pairs).unwrap(), native);
            assert_eq!(table_diagonal(&mut c, &pairs).unwrap(), native);
            assert_eq!((&mut c).diagonal(&pairs).unwrap(), native);
            cases += 1;
        }
        assert!(c.diagonal(&[(0, 0), (3, 3)]).is_err());
        assert!(symbolic_diagonal(&mut c, &[(0, 0), (3, 3)]).is_err());
        assert!(table_diagonal(&mut c, &[(0, 0), (3, 3)]).is_err());
    }
    // Symmetric support does not establish a full product. Reject it before
    // accepting any relation program or performing identity shortcuts.
    assert!(Algebra::try_new(C::new(&[129], 8), 1, true, "native").is_err());
    assert!(Algebra::try_new(C::new(&[255], 8), 2, true, "native").is_err());
    for layout in ["face-major", "bit-major", "pair-major"] {
        let width = 3;
        let n = 1usize << (3 * width);
        let mut c = C::full_space(3 * width, face_order(3 * width, 3, layout)).unwrap();
        let [r, _, goal] = counter_inputs(&mut c, width);
        let expected_r = bits(n, |w| ((w >> width) & 7) == (w & 7) + 1);
        assert_eq!(c.export(r), expected_r);
        assert_eq!(c.export(goal), bits(n, |w| (w >> width) & 7 == 7));
        let reference = counter_outputs(&mut c, width);
        let expected_reach = bits(n, |w| (w & 7) <= ((w >> width) & 7));
        assert_eq!(c.export(reference[0]), expected_reach);
        assert_eq!(
            c.export(reference[1]),
            bits(n, |w| (w & 7) >= ((w >> width) & 7))
        );
        assert_eq!(c.export(reference[4]), bits(n, |w| w & 7 == 6));
        for mode in ["native", "symbolic", "table"] {
            let mut a = Algebra::try_new(c.clone(), width, true, mode).unwrap();
            let (reach, _) = a.closure(r);
            assert_eq!(reach, reference[0]);
            let may = a.product(reach, goal, 7 << width);
            assert_eq!(may, reference[3]);
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"diagonal_verification\",\"candidate\":\"{}\",\"support_cases\":{},\"passed\":true}}",
        C::NAME,
        cases
    );
}

pub fn symbolic<C: Carrier>() {
    let width = 20;
    let c = C::full_space(60, face_order(60, 3, "bit-major")).unwrap();
    let mut a = Algebra::try_new(c, width, true, "native").unwrap();
    let pairs: Vec<_> = (0..width).map(|i| (i, width + i)).collect();
    let id = a.base.inner.diagonal(&pairs).unwrap();
    assert_eq!(a.base.inner.count(id), 1u64 << 40);
    assert_eq!(a.compose(id, id), id);
    assert_eq!(a.rename(id, 0), id);
    let mut flip = a.base.inner.full();
    for i in 0..width {
        let x = a.base.inner.variable(i);
        let y = a.base.inner.variable(width + i);
        let eq = a.base.op(if i == 0 || i == 19 { 6 } else { 9 }, x, y);
        flip = a.base.op(8, flip, eq);
    }
    assert_eq!(a.compose(id, flip), flip);
    assert_eq!(a.compose(flip, id), flip);
    assert_eq!(a.compose(flip, flip), id);
    assert_eq!(a.rename(flip, 0), flip);
    assert_eq!(a.residual(id, flip), flip);
    assert_eq!(a.residual(flip, id), flip);
    let expected = a.base.op(14, id, flip);
    assert_eq!(a.closure(flip).0, expected);
    assert!(table_diagonal(&mut a.base.inner, &pairs).is_err());
    println!(
        "EVENT_LAB {{\"kind\":\"symbolic_relation_verification\",\"candidate\":\"{}\",\"coordinates\":60,\"identity_worlds\":1099511627776,\"passed\":true}}",
        C::NAME
    );
}

// A non-wrapping increment relation; all-one Y is the goal. The ripple-carry
// circuit shares intermediates but never iterates over endpoint values.
pub(super) fn counter_inputs<C: RegionOps>(c: &mut C, width: u32) -> [Id; 3] {
    let full = c.full();
    let mut carry = full;
    let mut relation = full;
    let mut goal = full;
    for i in 0..width {
        let x = c.variable(i);
        let y = c.variable(width + i);
        let sum = c.op(6, x, carry);
        let eq = c.op(9, y, sum);
        relation = c.op(8, relation, eq);
        carry = c.op(8, carry, x);
        goal = c.op(8, goal, y);
    }
    relation = c.op(4, relation, carry);
    [relation, full, goal]
}

// Independent analytical answer: successor closure is <=; converse is >=;
// every state reaches max; only max-1 has a next step into max.
pub(super) fn counter_outputs<C: RegionOps>(c: &mut C, width: u32) -> [Id; 5] {
    let full = c.full();
    let mut bounds = [full; 2];
    let mut penultimate = full;
    for i in 0..width {
        let x = c.variable(i);
        let y = c.variable(width + i);
        let same = c.op(9, x, y);
        for (side, &(a, b)) in [(x, y), (y, x)].iter().enumerate() {
            let lower = c.op(2, a, b);
            let equal_then = c.op(8, same, bounds[side]);
            bounds[side] = c.op(14, lower, equal_then);
        }
        let value = if i == 0 { c.not(x) } else { x };
        penultimate = c.op(8, penultimate, value);
    }
    [bounds[0], bounds[1], full, full, penultimate]
}
