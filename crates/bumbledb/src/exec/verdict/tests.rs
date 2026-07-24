use super::*;
use crate::ir::VarId;
use crate::ir::validate::{ClassifiedComparison, DurationOperand, SealedConst};
use crate::ir::{CmpOp, Value};

fn compile(disjuncts: &[&[ClassifiedComparison]]) -> CompiledVerdict {
    // A flat width-1 layout: VarId(n) at slot n except VarId(0), an
    // interval at slots 0..2 shifting everything after by one.
    CompiledVerdict::compile(
        disjuncts,
        &|var: VarId| usize::from(var.0) + usize::from(var.0 > 0),
        &|var: VarId| if var.0 == 0 { 2 } else { 1 },
    )
}

fn duration_lt(bound: u64) -> ClassifiedComparison {
    ClassifiedComparison::Duration {
        interval: VarId(0),
        op: CmpOp::Lt,
        other: DurationOperand::Const(SealedConst::Literal(Value::U64(bound))),
    }
}

fn scalar_eq(var: u16, value: u64) -> ClassifiedComparison {
    ClassifiedComparison::VarConst {
        op: CmpOp::Eq,
        var: VarId(var),
        value: SealedConst::Literal(Value::U64(value)),
    }
}

/// The lattice's absorption laws at the fold: `Fails` absorbs the
/// conjunction (a failing sibling suppresses the ray), `Holds` absorbs
/// the disjunction (a holding sibling disjunct saves the binding) —
/// exactly `Verdict3.and`/`Verdict3.or` (R6), quantified over the one
/// partial predicate.
#[test]
fn the_kleene_fold_absorbs_exactly_as_ruled() {
    // Binding: v0 = ray [7, ∞), v1 = 3.
    let ray = |slot: usize| [7u64, u64::MAX, 3][slot];
    // Binding: v0 = [7, 9), v1 = 3.
    let bounded = |slot: usize| [7u64, 9, 3][slot];

    // One disjunct [Duration(v0) < 5]: Ray on the ray, Fails bounded
    // (duration 2 < 5 holds — flip the bound to see both).
    let lone: &[ClassifiedComparison] = &[duration_lt(5)];
    assert_eq!(compile(&[lone]).eval(&ray, &[]), Verdict3::Ray);
    assert_eq!(compile(&[lone]).eval(&bounded, &[]), Verdict3::Holds);

    // Fails absorbs And: [v1 == 4, Duration(v0) < 5] renders Fails on
    // the ray binding — the measure is unreached in every order.
    let absorbed: &[ClassifiedComparison] = &[scalar_eq(1, 4), duration_lt(5)];
    assert_eq!(compile(&[absorbed]).eval(&ray, &[]), Verdict3::Fails);

    // Holds absorbs Or: a sibling disjunct that holds saves the binding
    // whether or not this one measures a ray.
    let saved: &[&[ClassifiedComparison]] = &[lone, &[scalar_eq(1, 3)]];
    assert_eq!(compile(saved).eval(&ray, &[]), Verdict3::Holds);

    // Ray propagates: no disjunct holds, one rays.
    let poisoned: &[&[ClassifiedComparison]] = &[lone, &[scalar_eq(1, 4)]];
    assert_eq!(compile(poisoned).eval(&ray, &[]), Verdict3::Ray);
}

/// The fold is order-independent by construction: reversing disjuncts
/// and reversing each disjunct's leaves never moves the verdict.
#[test]
fn the_fold_is_order_blind() {
    let disjunct_a: &[ClassifiedComparison] = &[scalar_eq(1, 4), duration_lt(5)];
    let disjunct_b: &[ClassifiedComparison] = &[duration_lt(5), scalar_eq(1, 4)];
    let word = |slot: usize| [7u64, u64::MAX, 3][slot];
    assert_eq!(
        compile(&[disjunct_a, disjunct_b]).eval(&word, &[]),
        compile(&[disjunct_b, disjunct_a]).eval(&word, &[]),
    );
    assert_eq!(
        compile(&[disjunct_a]).eval(&word, &[]),
        compile(&[disjunct_b]).eval(&word, &[]),
    );
}
