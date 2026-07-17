//! The hostile-IR seam: the structurally-free arm re-exported from the
//! bench crate, plus the two tiers TODO.md § PHASE A-FUZZ adds. The
//! free arm reaches every rejection but aligns finds, bindings, and
//! types only by luck; these tiers aim INSIDE accepted shapes so
//! `prepare` executions reach the planner, the strata judge, and the
//! pin pass behind the validator:
//!
//! * [`adversarial_query`] — schema-typed join chains over the target
//!   theory with hostile values at the legal extremes: raw-word
//!   literals, legal Allen masks, measure terms, aggregate modes, deep
//!   legal condition spines, aligned multi-rule unions.
//! * [`random_program`] — program-shaped IR over the recursion roster:
//!   `Idb` atoms, the canonical closure with coin-flip strata refusals,
//!   the `MAX_PREDICATES` fence, and a fully-free arm lifting the bench
//!   generator's queries into predicates.
//!
//! No tier owns validity logic: each aims at a verdict class and the
//! ENGINE judges — the oracles stay the hostile arm's (typed rejection,
//! totality, determinism).

pub use bumbledb_bench::corpus_gen::irgen::*;

use bumbledb::schema::{FieldDescriptor, IntervalElement, ValueType};
use bumbledb::{
    AggOp, AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm,
    HeadTerm, MAX_PREDICATES, MaskTerm, ParamId, PredId, PredicateDef, Program, Query, RelationId,
    Rule, Term, Value, VarId,
};
use bumbledb_bench::corpus_gen::Rng;
use bumbledb_bench::querygen::target;

/// A well-formed-but-adversarial query: one schema-typed join chain,
/// extreme-value dressing, and — each a coin — conditions, negation,
/// aggregates, and aligned sibling rules. Acceptance is the aim, not a
/// guarantee; the verdict stays the engine's.
pub fn adversarial_query(rng: &mut Rng) -> Query {
    let mut scope = Scope::default();
    let mut rule = chain_rule(rng, &mut scope);
    if let Some(condition) = typed_condition(rng, &scope) {
        rule.conditions.push(condition);
    }
    // The deep-but-legal spine: nesting just under the cap must prepare,
    // never exhaust the stack (the trust-boundary law from the accepted
    // side; the free arm owns the over-cap rejection).
    if rng.chance(1, 8)
        && let Some(leaf) = typed_condition(rng, &scope)
    {
        let mut spine = leaf;
        for _ in 0..40 + rng.range(20) {
            spine = ConditionTree::And(vec![spine]);
        }
        rule.conditions.push(spine);
    }
    if rng.chance(1, 4)
        && let Some(negated) = typed_negation(rng, &scope)
    {
        rule.negated.push(negated);
    }
    rule.finds = typed_finds(rng, &scope);
    let head = rule.head();
    let mut rules = vec![rule];
    // Aligned siblings: the clone keeps finds (and so the positional
    // type row) identical while a fresh literal narrows one atom — the
    // union path exercised from the accepted side.
    if rng.chance(1, 4) {
        for _ in 0..1 + rng.range(2) {
            let mut sibling = rules[0].clone();
            narrow_atom(rng, &mut sibling);
            rules.push(sibling);
        }
    }
    Query { head, rules }
}

/// The rule-scoped variable roster: every var minted at an atom binding,
/// so finds, conditions, and negation draw from bound-by-construction
/// pools.
#[derive(Default)]
struct Scope {
    vars: Vec<ValueType>,
    params: Vec<(ValueType, bool)>,
}

impl Scope {
    fn mint(&mut self, value_type: &ValueType) -> VarId {
        self.vars.push(value_type.clone());
        VarId(u16::try_from(self.vars.len() - 1).expect("var roster fits u16"))
    }

    /// A scalar param of the field's type — reused when one exists (the
    /// shared-param join), minted dense otherwise (`ParamIdGap` is the
    /// free arm's).
    fn param(&mut self, rng: &mut Rng, value_type: &ValueType, set: bool) -> ParamId {
        let existing: Vec<usize> = self
            .params
            .iter()
            .enumerate()
            .filter(|(_, (ty, is_set))| ty == value_type && *is_set == set)
            .map(|(id, _)| id)
            .collect();
        if !existing.is_empty() && rng.chance(1, 2) {
            let pick = existing[draw(rng, existing.len())];
            return ParamId(u16::try_from(pick).expect("param roster fits u16"));
        }
        self.params.push((value_type.clone(), set));
        ParamId(u16::try_from(self.params.len() - 1).expect("param roster fits u16"))
    }

    fn vars_of(&self, wanted: impl Fn(&ValueType) -> bool) -> Vec<(VarId, &ValueType)> {
        self.vars
            .iter()
            .enumerate()
            .filter(|(_, ty)| wanted(ty))
            .map(|(id, ty)| (VarId(u16::try_from(id).expect("var id fits u16")), ty))
            .collect()
    }
}

/// One join chain over the target theory: hop by hop on u64 columns,
/// every non-join binding typed by its field — a variable, an extreme
/// literal, or a (set) param.
fn chain_rule(rng: &mut Rng, scope: &mut Scope) -> Rule {
    let relations =
        u64::try_from(target::schema().relations().len()).expect("relation count fits u64");
    let mut atoms = Vec::new();
    let mut join: Option<VarId> = None;
    let hops = 1 + rng.range(3);
    for _ in 0..hops {
        let rel = RelationId(u32::try_from(rng.range(relations)).expect("relation id fits u32"));
        let fields = target::schema().relation(rel).fields();
        let Some(join_field) = u64_field(rng, fields) else {
            continue;
        };
        let join_var = join.unwrap_or_else(|| scope.mint(&ValueType::U64));
        let mut bindings = vec![(join_field, Term::Var(join_var))];
        for (index, field) in fields.iter().enumerate() {
            let field_id = FieldId(u16::try_from(index).expect("field id fits u16"));
            if field_id == join_field || !rng.chance(1, 2) {
                continue;
            }
            bindings.push((field_id, typed_term(rng, scope, &field.value_type)));
        }
        atoms.push(Atom {
            source: AtomSource::Edb(rel),
            bindings,
        });
        // The next hop joins on a FRESH u64 var half the time — chains,
        // not just stars.
        join = if rng.chance(1, 2) {
            Some(join_var)
        } else {
            None
        };
    }
    if atoms.is_empty() {
        // Every drawn relation lacked a u64 column (unreachable over the
        // ledger, kept total): one Holder scan.
        let var = scope.mint(&ValueType::U64);
        atoms.push(Atom {
            source: AtomSource::Edb(target::ids::HOLDER),
            bindings: vec![(FieldId(0), Term::Var(var))],
        });
    }
    Rule {
        finds: vec![],
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

/// One binding term for a field: a minted variable (weighted — the join
/// graph is the point), an extreme literal, or a typed (set) param.
/// Interval fields bind interval-typed terms only (identity, never the
/// membership shape — a point var's domain rule is the free arm's to
/// abuse), and set params stay off interval fields (`IntervalParamSet`).
fn typed_term(rng: &mut Rng, scope: &mut Scope, value_type: &ValueType) -> Term {
    let interval = matches!(value_type, ValueType::Interval { .. });
    match rng.range(8) {
        0 => Term::Literal(extreme_value(rng, value_type)),
        1 => Term::Param(scope.param(rng, value_type, false)),
        2 if !interval => Term::ParamSet(scope.param(rng, value_type, true)),
        _ => Term::Var(scope.mint(value_type)),
    }
}

/// The relation's u64 columns, one drawn — the join currency of the
/// ledger (every id and reference is u64).
fn u64_field(rng: &mut Rng, fields: &[FieldDescriptor]) -> Option<FieldId> {
    let pool: Vec<FieldId> = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.value_type == ValueType::U64)
        .map(|(index, _)| FieldId(u16::try_from(index).expect("field id fits u16")))
        .collect();
    if pool.is_empty() {
        return None;
    }
    Some(pool[draw(rng, pool.len())])
}

/// One well-typed condition over the bound roster: order comparisons on
/// the numeric pools (a measure term riding one side), `Eq`/`Ne`
/// anywhere typed, a legal Allen mask, or the point-in shape — every
/// operand distinct (`SelfComparison`) and at least one a variable
/// (`ConstantComparison`), both by construction.
fn typed_condition(rng: &mut Rng, scope: &Scope) -> Option<ConditionTree> {
    let u64s = scope.vars_of(|ty| *ty == ValueType::U64);
    let intervals = scope.vars_of(|ty| matches!(ty, ValueType::Interval { .. }));
    let comparison = match rng.range(4) {
        0 => {
            let (var, _) = *pick(rng, &u64s)?;
            let op = order_op(rng);
            let rhs = Term::Literal(Value::U64(extreme_u64(rng)));
            // A measure on the left one draw in three, where an interval
            // var exists: `|iv| < k` is the one legal measure position.
            let lhs = match pick(rng, &intervals) {
                Some((iv, _)) if rng.chance(1, 3) => Term::Measure(*iv),
                _ => Term::Var(var),
            };
            Comparison { op, lhs, rhs }
        }
        1 => {
            let (lhs, ty) = *pick(rng, &u64s)?;
            let rhs = match scope
                .vars_of(|other| other == ty)
                .iter()
                .find(|(other, _)| *other != lhs)
            {
                Some((other, _)) if rng.chance(1, 2) => Term::Var(*other),
                _ => Term::Literal(Value::U64(extreme_u64(rng))),
            };
            Comparison {
                op: if rng.chance(1, 2) {
                    CmpOp::Eq
                } else {
                    CmpOp::Ne
                },
                lhs: Term::Var(lhs),
                rhs,
            }
        }
        2 => {
            let (lhs, element) = interval_pair_side(rng, &intervals)?;
            Comparison {
                op: CmpOp::Allen {
                    mask: legal_mask(rng),
                },
                lhs: Term::Var(lhs),
                rhs: interval_partner(rng, &intervals, lhs, element),
            }
        }
        _ => {
            // The point-in shape, interval-left / point-right: a bound
            // interval var against an element-typed extreme point (the
            // ceiling itself is the free arm's rejection).
            let (iv, ty) = *pick(rng, &intervals)?;
            let point = match ty {
                ValueType::Interval {
                    element: IntervalElement::U64,
                    ..
                } => Value::U64(extreme_u64(rng).min(u64::MAX - 1)),
                _ => Value::I64(extreme_i64(rng).min(i64::MAX - 1)),
            };
            Comparison {
                op: CmpOp::PointIn,
                lhs: Term::Var(iv),
                rhs: Term::Literal(point),
            }
        }
    };
    Some(ConditionTree::Leaf(comparison))
}

/// One side of an Allen pair plus its element domain.
fn interval_pair_side(
    rng: &mut Rng,
    intervals: &[(VarId, &ValueType)],
) -> Option<(VarId, IntervalElement)> {
    let (var, ty) = *pick(rng, intervals)?;
    let element = match ty {
        ValueType::Interval { element, .. } => *element,
        _ => return None,
    };
    Some((var, element))
}

/// The Allen partner: a DISTINCT same-element interval var when one
/// exists, an extreme literal of the element otherwise.
fn interval_partner(
    rng: &mut Rng,
    intervals: &[(VarId, &ValueType)],
    lhs: VarId,
    element: IntervalElement,
) -> Term {
    let twin = intervals.iter().find(|(var, ty)| {
        *var != lhs && matches!(ty, ValueType::Interval { element: e, .. } if *e == element)
    });
    match twin {
        Some((var, _)) if rng.chance(1, 2) => Term::Var(*var),
        _ => Term::Literal(extreme_interval(rng, element)),
    }
}

/// A negated atom re-using one bound u64 var — the safety rule by
/// construction: every scope var occurs positively (minted at atom
/// bindings only), and negation binds nothing, only rejects.
fn typed_negation(rng: &mut Rng, scope: &Scope) -> Option<Atom> {
    let u64s = scope.vars_of(|ty| *ty == ValueType::U64);
    let (var, _) = *pick(rng, &u64s)?;
    Some(Atom {
        source: AtomSource::Edb(target::ids::POSTING_TAG),
        bindings: vec![(FieldId(0), Term::Var(var))],
    })
}

/// The find roster, one aggregate MODE per rule (fold, Arg, Pack, and
/// measure never mix — the mixing refusals stay the free arm's): plain
/// distinct vars, folds beside a group key, one Arg restriction, one
/// Pack over an interval var, or the measure family.
fn typed_finds(rng: &mut Rng, scope: &Scope) -> Vec<FindTerm> {
    let numeric = scope.vars_of(|ty| *ty == ValueType::U64 || *ty == ValueType::I64);
    let intervals = scope.vars_of(|ty| matches!(ty, ValueType::Interval { .. }));
    let all: Vec<VarId> = (0..scope.vars.len())
        .map(|id| VarId(u16::try_from(id).expect("var id fits u16")))
        .collect();
    let plain = |rng: &mut Rng| -> Vec<FindTerm> {
        let take = (1 + draw(rng, 3)).min(all.len());
        all[..take].iter().map(|var| FindTerm::Var(*var)).collect()
    };
    match rng.range(8) {
        0 => {
            // Folds: the group key is the leading var, the folds range
            // over LATER vars only (`AggregateOverGroupKey` avoided by
            // position, not by rule knowledge).
            let Some((over, _)) = numeric.iter().find(|(var, _)| *var != all[0]) else {
                return plain(rng);
            };
            let op = match rng.range(3) {
                0 => AggOp::Sum,
                1 => AggOp::Min,
                _ => AggOp::Max,
            };
            let mut finds = vec![
                FindTerm::Var(all[0]),
                FindTerm::Aggregate {
                    op,
                    over: Some(*over),
                },
            ];
            if rng.chance(1, 2) {
                finds.push(FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                });
            }
            finds
        }
        1 => {
            let Some((key, _)) = numeric.iter().find(|(var, _)| *var != all[0]) else {
                return plain(rng);
            };
            let Some(carried) = all.iter().find(|var| **var != *key && **var != all[0]) else {
                return plain(rng);
            };
            vec![
                FindTerm::Var(all[0]),
                FindTerm::Aggregate {
                    op: if rng.chance(1, 2) {
                        AggOp::ArgMax { key: *key }
                    } else {
                        AggOp::ArgMin { key: *key }
                    },
                    over: Some(*carried),
                },
            ]
        }
        2 => {
            let Some((packed, _)) = intervals.iter().find(|(var, _)| *var != all[0]) else {
                return plain(rng);
            };
            vec![
                FindTerm::Var(all[0]),
                FindTerm::Aggregate {
                    op: AggOp::Pack,
                    over: Some(*packed),
                },
            ]
        }
        3 => {
            let Some((over, _)) = intervals.first() else {
                return plain(rng);
            };
            if rng.chance(1, 2) {
                vec![FindTerm::Measure(*over)]
            } else {
                let op = match rng.range(3) {
                    0 => AggOp::Sum,
                    1 => AggOp::Min,
                    _ => AggOp::Max,
                };
                vec![FindTerm::AggregateMeasure { op, over: *over }]
            }
        }
        _ => plain(rng),
    }
}

/// One extra literal binding on the sibling's first atom, on a field it
/// leaves free — the clone stays aligned while its denotation narrows.
fn narrow_atom(rng: &mut Rng, rule: &mut Rule) {
    let Some(atom) = rule.atoms.first_mut() else {
        return;
    };
    let AtomSource::Edb(rel) = atom.source else {
        return;
    };
    let fields = target::schema().relation(rel).fields();
    let free: Vec<usize> = (0..fields.len())
        .filter(|index| {
            let id = FieldId(u16::try_from(*index).expect("field id fits u16"));
            atom.bindings.iter().all(|(bound, _)| *bound != id)
        })
        .collect();
    if free.is_empty() {
        return;
    }
    let index = free[draw(rng, free.len())];
    atom.bindings.push((
        FieldId(u16::try_from(index).expect("field id fits u16")),
        Term::Literal(extreme_value(rng, &fields[index].value_type)),
    ));
}

/// A program over the recursion roster: the canonical closure with
/// coin-flip strata refusals half the time, the free predicate lift
/// otherwise.
pub fn random_program(rng: &mut Rng) -> Program {
    if rng.chance(1, 2) {
        closure_program(rng)
    } else {
        free_program(rng)
    }
}

/// Reachability over `OrgParent(child, parent)` — the recursion
/// roster's canonical inhabitant — then coin-flip mutations reach the
/// strata judge's refusals FROM the accepted side: negation and
/// aggregation through the cycle, the `MAX_PREDICATES` fence, dangling
/// predicate/output ids, out-of-range head columns.
fn closure_program(rng: &mut Rng) -> Program {
    let parent = target::ids::ORG_PARENT;
    let edge = |a: VarId, b: VarId| Atom {
        source: AtomSource::Edb(parent),
        bindings: vec![(FieldId(0), Term::Var(a)), (FieldId(1), Term::Var(b))],
    };
    let base = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![edge(VarId(0), VarId(1))],
        negated: vec![],
        conditions: vec![],
    };
    let step = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: AtomSource::Idb(PredId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            edge(VarId(1), VarId(2)),
        ],
        negated: vec![],
        conditions: vec![],
    };
    let mut predicates = vec![PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![base.clone(), step],
    }];
    let mut output = PredId(0);

    // The census head: aggregation OVER the fixpoint, outside the cycle
    // — the legal strata shape.
    if rng.chance(1, 2) {
        predicates.push(PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
            rules: vec![Rule {
                finds: vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
                atoms: vec![Atom {
                    source: AtomSource::Idb(PredId(0)),
                    bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                }],
                negated: vec![],
                conditions: vec![],
            }],
        });
        output = PredId(1);
        // Aggregation THROUGH the cycle: the step also reads the census,
        // closing 0 → 1 → 0 through an aggregate head.
        if rng.chance(1, 8) {
            predicates[0].rules[1].atoms.push(Atom {
                source: AtomSource::Idb(PredId(1)),
                bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
            });
        }
    }
    // Negation through the cycle.
    if rng.chance(1, 8) {
        predicates[0].rules[1].negated.push(Atom {
            source: AtomSource::Idb(PredId(0)),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        });
    }
    // The fence, exactly AT and exactly PAST — both one clone loop.
    if rng.chance(1, 4) {
        let fence = if rng.chance(1, 2) {
            MAX_PREDICATES
        } else {
            MAX_PREDICATES + 1
        };
        while predicates.len() < fence {
            predicates.push(PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![base.clone()],
            });
        }
    }
    let span = u64::try_from(predicates.len()).expect("predicate count fits u64");
    // Dangling ids and the column overshoot — each a rare coin, so the
    // accepted spine dominates.
    if rng.chance(1, 8) {
        predicates[0].rules[1].atoms[0].source = AtomSource::Idb(PredId(
            u16::try_from(span + 1 + rng.range(3)).expect("id fits u16"),
        ));
    }
    if rng.chance(1, 8) {
        output = PredId(u16::try_from(span + 1 + rng.range(3)).expect("id fits u16"));
    }
    if rng.chance(1, 8) {
        predicates[0].rules[1].atoms[0].bindings[0].0 =
            FieldId(u16::try_from(2 + rng.range(6)).expect("field id fits u16"));
    }
    Program { predicates, output }
}

/// The free predicate lift: bench-arm queries become predicates, then
/// sources flip to `Idb` across the roster — strata shapes, dangling
/// ids, and signature knots arise freely, and the verdict is the
/// engine's.
fn free_program(rng: &mut Rng) -> Program {
    let count = 1 + draw(rng, 4);
    let mut predicates: Vec<PredicateDef> = (0..count)
        .map(|_| {
            let query = random_query(rng);
            PredicateDef {
                head: query.head,
                rules: query.rules,
            }
        })
        .collect();
    let span = u64::try_from(count).expect("predicate count fits u64");
    for predicate in &mut predicates {
        for rule in &mut predicate.rules {
            for atom in rule.atoms.iter_mut().chain(rule.negated.iter_mut()) {
                if rng.chance(1, 4) {
                    atom.source = AtomSource::Idb(PredId(
                        u16::try_from(rng.range(span + 2)).expect("pred id fits u16"),
                    ));
                }
            }
        }
    }
    let output = if rng.chance(7, 8) {
        PredId(u16::try_from(rng.range(span)).expect("pred id fits u16"))
    } else {
        PredId(u16::try_from(span + rng.range(3)).expect("pred id fits u16"))
    };
    Program { predicates, output }
}

/// Any legal literal mask: never ∅, never FULL — the vacuous pair stays
/// the free arm's.
fn legal_mask(rng: &mut Rng) -> MaskTerm {
    let bits = u16::try_from(1 + rng.range((1 << 13) - 2)).expect("13 bits fit u16");
    MaskTerm::Literal(AllenMask::new(bits).expect("nonzero sub-full draw is a mask"))
}

/// An extreme literal of one field type — raw entropy words land here,
/// so dictionary entries (encoding boundaries, ray sentinels) splice
/// straight into value positions.
fn extreme_value(rng: &mut Rng, value_type: &ValueType) -> Value {
    match value_type {
        ValueType::Bool => Value::Bool(rng.chance(1, 2)),
        ValueType::U64 => Value::U64(extreme_u64(rng)),
        ValueType::I64 => Value::I64(extreme_i64(rng)),
        ValueType::String => Value::String(Box::from(&b"Fee"[..])),
        ValueType::FixedBytes { len } => Value::FixedBytes(vec![0xA5; usize::from(*len)].into()),
        ValueType::Interval { element, width } => {
            let value = extreme_interval(rng, *element);
            match (value, width) {
                // A fixed-width column takes exact-width literals: the
                // drawn start, the declared span.
                (Value::IntervalU64(iv), Some(w)) => Value::IntervalU64(
                    bumbledb::Interval::<u64>::new(
                        iv.start().min(u64::MAX - 1 - w),
                        iv.start().min(u64::MAX - 1 - w) + w,
                    )
                    .expect("width-legal interval"),
                ),
                (Value::IntervalI64(iv), Some(w)) => {
                    let span = i64::try_from((*w).min(u64::try_from(i64::MAX).expect("half")))
                        .expect("clamped width fits i64");
                    let start = iv.start().clamp(i64::MIN, i64::MAX - 1 - span);
                    Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(start, start + span)
                            .expect("width-legal interval"),
                    )
                }
                (value, _) => value,
            }
        }
    }
}

/// An extreme interval of one element: the unit at zero, the ray, the
/// ceiling shoulder, a raw-word start.
fn extreme_interval(rng: &mut Rng, element: IntervalElement) -> Value {
    match element {
        IntervalElement::U64 => {
            let (start, end) = match rng.range(4) {
                0 => (0, 1),
                1 => (0, u64::MAX),
                2 => (u64::MAX - 1, u64::MAX),
                _ => {
                    let start = rng.u64().min(u64::MAX - 1);
                    (start, start + 1)
                }
            };
            Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("start < end"))
        }
        IntervalElement::I64 => {
            let (start, end) = match rng.range(4) {
                0 => (0, 1),
                1 => (i64::MIN, i64::MAX),
                2 => (i64::MAX - 1, i64::MAX),
                _ => {
                    let start = extreme_i64(rng).min(i64::MAX - 1);
                    (start, start + 1)
                }
            };
            Value::IntervalI64(bumbledb::Interval::<i64>::new(start, end).expect("start < end"))
        }
    }
}

fn extreme_u64(rng: &mut Rng) -> u64 {
    match rng.range(4) {
        0 => 0,
        1 => u64::MAX,
        2 => u64::from(u16::MAX) + 1,
        _ => rng.u64(),
    }
}

fn extreme_i64(rng: &mut Rng) -> i64 {
    match rng.range(4) {
        0 => i64::MIN,
        1 => i64::MAX,
        2 => 0,
        _ => i64::from_le_bytes(rng.u64().to_le_bytes()),
    }
}

fn order_op(rng: &mut Rng) -> CmpOp {
    match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

fn pick<'pool, T>(rng: &mut Rng, pool: &'pool [T]) -> Option<&'pool T> {
    if pool.is_empty() {
        return None;
    }
    Some(&pool[draw(rng, pool.len())])
}

fn draw(rng: &mut Rng, n: usize) -> usize {
    let n = u64::try_from(n).expect("count fits u64");
    usize::try_from(rng.range(n)).expect("draw fits usize")
}

#[cfg(test)]
mod tests {
    use super::{adversarial_query, random_program};
    use bumbledb::{AtomSource, MAX_PREDICATES};
    use bumbledb_bench::corpus_gen::Rng;
    use bumbledb_bench::querygen::target;

    /// Both tiers are deterministic in their entropy.
    #[test]
    fn the_same_bytes_yield_the_same_artifacts() {
        let bytes: Vec<u8> = (1..=96u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        assert_eq!(
            adversarial_query(&mut Rng::from_bytes(&bytes)),
            adversarial_query(&mut Rng::from_bytes(&bytes)),
            "same bytes, same query"
        );
        assert_eq!(
            random_program(&mut Rng::from_bytes(&bytes)),
            random_program(&mut Rng::from_bytes(&bytes)),
            "same bytes, same program"
        );
    }

    /// The adversarial tier's whole point: a strong majority of draws
    /// prepare — the free arm's accept rate is luck-bound. A missed aim
    /// stays a legal rejection; the assertion is on the RATE.
    #[test]
    fn the_adversarial_tier_biases_hard_toward_acceptance() {
        let dir = std::env::temp_dir().join("bumbledb-fuzz-irgen-adversarial");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, target::Target).expect("create");
        let mut accepted = 0u32;
        for seed in 0..512 {
            let query = adversarial_query(&mut Rng::new(seed));
            if db.prepare(&query).is_ok() {
                accepted += 1;
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            accepted >= 384,
            "the adversarial tier must prepare on at least 3/4 of draws, got {accepted}/512"
        );
    }

    /// The program arm reaches both verdict classes, recursion included:
    /// across a seed sweep some program with a REAL `Idb` cycle
    /// prepares, and some program is rejected.
    #[test]
    fn the_program_arm_reaches_recursion_and_both_verdict_classes() {
        let dir = std::env::temp_dir().join("bumbledb-fuzz-irgen-program");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, target::Target).expect("create");
        let mut accepted_recursive = 0u32;
        let mut rejected = 0u32;
        let mut fence = false;
        for seed in 0..512 {
            let program = random_program(&mut Rng::new(seed));
            if program.predicates.len() > MAX_PREDICATES {
                fence = true;
            }
            let recursive = program.predicates.iter().any(|pred| {
                pred.rules.iter().any(|rule| {
                    rule.atoms
                        .iter()
                        .any(|atom| matches!(atom.source, AtomSource::Idb(_)))
                })
            });
            match db.prepare(&program) {
                Ok(_) if recursive => accepted_recursive += 1,
                Ok(_) => {}
                Err(_) => rejected += 1,
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            accepted_recursive > 0,
            "no recursive program prepared in 512 seeds"
        );
        assert!(rejected > 0, "no program rejected in 512 seeds");
        assert!(fence, "the MAX_PREDICATES fence never arose");
    }
}
