use std::collections::BTreeMap;

use super::{
    AntiProbe, NormalizedQuery, OccId, Occurrence, Role, SlotWidth,
    lower_literal::{lower_literal, point_word},
    place_comparisons::place_comparisons,
};
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::validate::{RuleWitness, ValidatedQuery};
use crate::ir::{Atom, CmpOp, Term, Value, VarId};
use crate::schema::Schema;
use bumbledb_theory::schema::{FieldId, ValueType};

/// Lowers the witness into paper form, rule by rule: one
/// [`NormalizedQuery`] per rule, in rule order — the normalized artifact
/// is a list because the query is a program. The query path: no `Idb`
/// occurrence exists in a sealed [`ValidatedQuery`] (the query boundary
/// has no predicate address space), so the signature surface is empty.
///
/// # Panics
///
/// Only on programmer-invariant violations already excluded by validation
/// (e.g. a comparison variable bound by no atom).
#[must_use]
pub fn normalize(schema: &Schema, query: &ValidatedQuery) -> Vec<NormalizedQuery> {
    normalize_predicate(schema, query, &[])
}

/// [`normalize`] with the program's `Idb` typing surface: `signatures`
/// holds every predicate's sealed signature in `PredId` order, and an
/// `Idb` binding's field type reads the target's column — `FieldId(i)`
/// is head position `i` (`docs/architecture/20-query-ir.md` § engine recursion; the
/// positional reading `lean/Bumbledb/Exec/Fixpoint.lean: tupleFact`
/// promises). Everything else is the conjunctive lowering, verbatim.
///
/// # Panics
///
/// As [`normalize`].
#[must_use]
pub fn normalize_predicate(
    schema: &Schema,
    query: &ValidatedQuery,
    signatures: &[&crate::ir::validate::Predicate],
) -> Vec<NormalizedQuery> {
    query
        .rules()
        .map(|rule| normalize_rule(schema, signatures, &rule))
        .collect()
}

/// Lowers one rule — exactly the conjunctive query's lowering, over the
/// rule's own variable scope.
fn normalize_rule(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    rule: &RuleWitness<'_>,
) -> NormalizedQuery {
    normalize_rule_with(schema, signatures, rule, rule.classified_comparisons())
}

/// A written rule's **ray probe** (the Kleene verdict algebra, ruled
/// 2026-07-23, R6): the rule's atoms, negations, and memberships with
/// every condition replaced by ONE filter — `measured` intersects the
/// ray probe `[MAX−1, ∞)`, which only rays do (bounded ends encode
/// strictly below the ∞ sentinel and half-open adjacency excludes the
/// touch) — so the probe enumerates exactly the bindings whose measured
/// interval is a ray. The prepared query folds each one's verdict over
/// the written rule's disjuncts (`exec/verdict.rs`) and raises
/// `MeasureOfRay` iff some verdict is Ray.
#[must_use]
pub fn normalize_ray_probe(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    rule: &RuleWitness<'_>,
    measured: VarId,
) -> NormalizedQuery {
    let probe = match rule.var_type(measured) {
        ValueType::Interval {
            element: bumbledb_theory::schema::IntervalElement::U64,
            ..
        } => Value::IntervalU64(
            bumbledb_theory::interval::Interval::ray(u64::MAX - 1).expect("below the ceiling"),
        ),
        ValueType::Interval {
            element: bumbledb_theory::schema::IntervalElement::I64,
            ..
        } => Value::IntervalI64(
            bumbledb_theory::interval::Interval::ray(i64::MAX - 1).expect("below the ceiling"),
        ),
        other => unreachable!("validated: the measure reads an interval variable, got {other:?}"),
    };
    let is_ray = crate::ir::validate::ClassifiedComparison::AllenVarConst {
        var: measured,
        other: crate::ir::validate::SealedConst::Literal(probe),
        mask: crate::image::view::MaskConst::Mask(bumbledb_theory::allen::AllenMask::INTERSECTS),
    };
    normalize_rule_with(schema, signatures, rule, std::slice::from_ref(&is_ray))
}

/// [`normalize_rule`]'s body over an explicit comparison list — the one
/// extra caller is the ray probe, whose comparisons are not the rule's.
fn normalize_rule_with(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    rule: &RuleWitness<'_>,
    comparisons: &[crate::ir::validate::ClassifiedComparison],
) -> NormalizedQuery {
    let positive = rule.rule().atoms.len();
    let mut occurrences: Vec<Occurrence> = Vec::with_capacity(positive + rule.rule().negated.len());
    for (idx, atom) in rule.rule().atoms.iter().enumerate() {
        occurrences.push(lower_atom(
            schema,
            signatures,
            rule,
            idx,
            Role::Positive,
            atom,
        ));
    }
    for (idx, atom) in rule.rule().negated.iter().enumerate() {
        occurrences.push(lower_atom(
            schema,
            signatures,
            rule,
            positive + idx,
            Role::Negated,
            atom,
        ));
    }

    // One anti-probe descriptor per negated occurrence: the probe keys are
    // its variable bindings; its constant bindings became its filter list
    // above, evaluated inside the probe.
    let anti_probes: Vec<AntiProbe> = occurrences[positive..]
        .iter()
        .map(|occurrence| AntiProbe {
            occurrence: occurrence.occ_id,
            probe_bindings: occurrence.vars.clone(),
        })
        .collect();

    let (residuals, word_residuals, allen_residuals, duration_residuals) =
        place_comparisons(comparisons, &mut occurrences);

    // The binding-slot widths — the two-slot interval layout, decided at
    // [`SlotWidth`] and exported here to the plan witness.
    let slot_widths: BTreeMap<VarId, SlotWidth> = rule
        .var_types()
        .map(|(var, value_type)| (var, SlotWidth::of(value_type)))
        .collect();

    // Nothing single-occurrence survives to the residual list
    // (docs/architecture/20-query-ir.md, § normalization step 5) — across
    // every residual kind: whole-value, decomposed word, and Allen mask
    // comparisons.
    debug_assert!(
        residuals
            .iter()
            .map(|r| (r.lhs, r.rhs))
            .chain(word_residuals.iter().map(|r| (r.lhs.var, r.rhs.var)))
            .chain(allen_residuals.iter().map(|r| (r.lhs, r.rhs)))
            .chain(duration_residuals.iter().map(|r| (r.interval, r.scalar)))
            .all(|(lhs, rhs)| {
                !occurrences
                    .iter()
                    .filter(|occ| occ.role.participates())
                    .any(|occ| {
                        occ.vars.iter().any(|(_, v)| *v == lhs)
                            && occ.vars.iter().any(|(_, v)| *v == rhs)
                    })
            })
    );

    // The statically-empty fold (fold.rs), last: with every comparison
    // placed, each participating occurrence's constant filters fold per
    // slot and the contradiction rules judge the rule on constants —
    // stage-2-known emptiness becomes the rule's verdict
    // (docs/architecture/20-query-ir.md, § normalization).
    let dead = super::fold::fold(schema, &mut occurrences);

    NormalizedQuery {
        occurrences,
        residuals,
        word_residuals,
        allen_residuals,
        duration_residuals,
        anti_probes,
        slot_widths,
        dead,
    }
}

/// Whether a binding is a **membership** position: an interval field bound
/// to an element-typed term — the term is a point in the field's interval,
/// never its value (`docs/architecture/20-query-ir.md`, the membership
/// rule; validation resolved every term's type).
fn is_membership(field_type: &ValueType, term_type: &ValueType) -> bool {
    matches!(field_type, ValueType::Interval { .. })
        && !matches!(term_type, ValueType::Interval { .. })
}

/// Lowers one atom (positive or negated — the rules are identical; only
/// the role differs) into an occurrence: variable positions plus the
/// filters lowered out of its constant, repeated-variable, and membership
/// bindings.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the two-pass binding walk, one arm per term kind
fn lower_atom(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    witness: &RuleWitness<'_>,
    idx: usize,
    role: Role,
    atom: &Atom,
) -> Occurrence {
    let occ_id = OccId(u16::try_from(idx).expect("validated: occurrence count fits u16"));
    // Field types come from the stored relation, or — for an `Idb` atom
    // — from the target's sealed signature columns (`FieldId(i)` is head
    // position `i`, typed by `Predicate.columns[i].ty`; the literal
    // encodings are value-driven, so a predicate column lowers exactly
    // as a stored field of its type does).
    let field_type = |field: FieldId| -> &ValueType {
        match atom.source {
            crate::ir::AtomSource::Edb(relation_id) => {
                &schema.relation(relation_id).field(field).value_type
            }
            crate::ir::AtomSource::Idb(pred) => {
                &signatures[usize::from(pred.0)].columns[usize::from(field.0)].ty
            }
        }
    };

    // Pass 1 — variable positions: the first *domain* binding of each
    // variable (a scalar field, or an interval field read by value).
    // Membership positions bind no variable — they are conditions, lowered
    // to filters in pass 2. `Term::Measure` never appears in a binding
    // (validation: `DurationInBinding`), so both passes match it
    // unreachable.
    let mut vars: Vec<(FieldId, VarId)> = Vec::new();
    for (field, term) in &atom.bindings {
        if let Term::Var(var) = term {
            if is_membership(field_type(*field), witness.var_type(*var)) {
                continue;
            }
            if !vars.iter().any(|(_, v)| v == var) {
                vars.push((*field, *var));
            }
        }
    }

    // Pass 2 — filters, in binding order.
    let mut filters = Vec::new();
    for (field, term) in &atom.bindings {
        let field_type = field_type(*field);
        match term {
            Term::Var(var) => {
                if is_membership(field_type, witness.var_type(*var)) {
                    // Membership: `start ≤ var < end`. With the point
                    // variable scalar-bound in this atom, the condition is
                    // a same-fact field composition; otherwise it reads the
                    // variable's binding once bound (the point-membership
                    // scan, docs/architecture/40-execution.md).
                    filters.push(match vars.iter().find(|(_, v)| v == var) {
                        Some((point_field, _)) => FilterPredicate::FieldsPointIn {
                            interval: *field,
                            point: *point_field,
                        },
                        None => FilterPredicate::PointIn {
                            field: *field,
                            point: ResolvedWordSource::Var(*var),
                        },
                    });
                } else {
                    // A repeated variable keeps its first field binding as
                    // the variable position; subsequent positions lower to
                    // same-fact equality filters.
                    let (first_field, _) = vars
                        .iter()
                        .find(|(_, v)| v == var)
                        .expect("pass 1 recorded every domain-bound variable");
                    if first_field != field {
                        filters.push(FilterPredicate::FieldsCompare {
                            left: *first_field,
                            right: *field,
                            op: CmpOp::Eq,
                        });
                    }
                }
            }
            Term::Param(param) => {
                if is_membership(field_type, witness.param_type(*param)) {
                    filters.push(FilterPredicate::PointIn {
                        field: *field,
                        point: ResolvedWordSource::Param(*param),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::Param(*param),
                    });
                }
            }
            Term::ParamSet(param) => {
                if matches!(field_type, ValueType::Interval { .. }) {
                    // A set holds points (validation anchored the element
                    // type): any element in the field's interval.
                    filters.push(FilterPredicate::AnyPointIn {
                        field: *field,
                        set: Const::ParamSet(*param),
                    });
                } else {
                    // The selection-level set marker: an Eq compare the
                    // plan routes into `selections`, carried as a word set
                    // at bind (docs/architecture/20-query-ir.md, § param
                    // sets; executor side is PRD 17).
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::ParamSet(*param),
                    });
                }
            }
            Term::Measure(_) => unreachable!("validated: no measure in bindings"),
            Term::Literal(value) => {
                let membership = matches!(field_type, ValueType::Interval { .. })
                    && !matches!(value, Value::IntervalU64(..) | Value::IntervalI64(..));
                if membership {
                    filters.push(FilterPredicate::PointIn {
                        field: *field,
                        point: ResolvedWordSource::Word(point_word(value)),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: lower_literal(value),
                    });
                }
            }
        }
    }

    Occurrence {
        occ_id,
        source: atom.source,
        role,
        vars,
        filters,
    }
}
