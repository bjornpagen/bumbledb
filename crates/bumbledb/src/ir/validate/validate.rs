use super::{
    Context, IdbSignatures, ParamKind, Predicate, RuleTyping, TypeSlot, ValidatedProgram,
    ValidatedQuery,
};
use crate::error::ValidationError;
use crate::ir::normalize::{LoweredRule, collapse, disjunct_count, distribute, nesting_depth};
use crate::ir::{
    AggOp, FindTerm, MAX_CONDITION_DEPTH, MAX_PREDICATES, MAX_RULES, ParamId, PredId, Program,
    Query, VarId,
};
use crate::schema::Schema;
use bumbledb_theory::schema::ValueType;
use std::collections::{BTreeMap, BTreeSet};

/// Validates a query against the schema, yielding the sealed witness.
///
/// The program shape first (empty rule set, the rule cap, empty head);
/// then **DNF distribution** — each rule's condition trees distribute
/// and each disjunct becomes a rule, with the blowup capped at
/// [`MAX_RULES`] on the structural term count (before materializing)
/// and duplicates collapsed by normalized-form equality; then each
/// lowered rule under the per-rule roster with its own typing fixpoint
/// — a rule validates exactly as a conjunctive query did — and finally
/// the query-global param unification (params are one binding surface
/// across rules; variables never cross them).
///
/// Duplicate and even statically contradictory conditions (`x < 5,
/// x > 9`) are accepted deliberately: the semantics are exact (an empty
/// result), and the "write the query you mean" roster rejects only
/// shapes with no meaning at all (constant and self comparisons) — it
/// does not extend to statically false conjunctions. The empty
/// disjunction (`Or([])`) rides the same ruling: it is constant false,
/// its rule lowers to zero rules, and only a program whose *every* rule
/// vanishes is rejected — as the empty union it now is.
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item; see the module docs.
/// Rule-local payloads name positions inside the first failing
/// **lowered** rule.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    // An `Idb` atom at the QUERY boundary refuses inside the per-rule
    // roster with the screen's own vocabulary (`UnknownPredicate`): a
    // bare query has no predicate address space —
    // [`IdbSignatures::EMPTY`] — and a `ValidatedQuery` cannot carry a
    // fixpoint. Recursion's surface is the program boundary
    // ([`validate_program`], executed by
    // [`crate::Db::prepare`]'s per-stratum driver); the
    // degenerate embedding runs the other way — a no-`Idb` program IS
    // its output query (`lean/Bumbledb/Exec/Fixpoint.lean:
    // degenerate_embedding`).
    let lowered = lower_rules(&query.head, &query.rules)?;

    let mut pinned_row: Vec<ValueType> = Vec::new();
    let mut rules = Vec::with_capacity(lowered.len());
    let mut params = ParamTables::default();
    for (rule_idx, rule) in lowered.iter().enumerate() {
        check_head_alignment(&query.head, rule, rule_idx)?;
        let (typing, ctx) = validate_rule(schema, &IdbSignatures::EMPTY, rule)?;
        // Every rule derives the predicate: rule 0's resolved positional
        // input row pins the head, and every later rule must agree
        // position by position (the input row, not the signature — a
        // `CountDistinct` position anchors its *input* type across
        // rules, though its signature column is U64 regardless).
        let row = input_row(rule, &typing);
        if rule_idx == 0 {
            pinned_row = row;
        } else if let Some(position) = (0..row.len()).find(|i| row[*i] != pinned_row[*i]) {
            return Err(ValidationError::HeadTypeMismatch {
                rule: rule_idx,
                position,
            });
        }
        params.unify(ctx)?;
        rules.push(typing);
    }
    params.check_masks_and_density()?;

    // The predicate, derived ONCE — rule 0 speaks for every rule (the
    // alignment above), and nothing downstream re-derives the signature.
    let predicate = super::Predicate::derive(&lowered[0], &rules[0]);

    Ok(seal(lowered, predicate, rules, &params))
}

/// Validates a program against the schema — the recursion cut's boundary
/// (`docs/architecture/20-query-ir.md` § engine recursion), yielding the sealed
/// per-predicate witnesses. The roster, in order:
///
/// 1. **Program shape**: the predicate cap ([`MAX_PREDICATES`]), the
///    output screen, and per predicate the whole query shape roster
///    (rule cap, empty edges, nesting, DNF distribution, head-shape
///    alignment) — a predicate validates exactly as a query did.
/// 2. **The well-formedness screen and the strata judge**
///    (`ir/validate/strata.rs`): every `Idb` source names a real
///    predicate and addresses inside its arity
///    (`lean/Bumbledb/Query/Syntax.lean: Program.WellFormed`), the
///    dependency graph's SCC condensation is computed iteratively, and
///    the safety roster refuses `NegationThroughCycle`,
///    `AggregationThroughCycle`, and `MeasureInRecursiveHead` — so
///    recursive heads project bound variables only, which is the
///    termination theorem's premise
///    (`lean/Bumbledb/Exec/Fixpoint.lean: program_den_finite`).
/// 3. **The executable-class item**
///    ([`ValidationError::AggregateInteriorPredicate`] /
///    [`ValidationError::MeasureInteriorPredicate`]): fold-headed AND
///    measure-projecting predicates are legal only at the output —
///    interior predicates are projection-shaped word-row tables of
///    bound variables (the Lean cut's own class:
///    `lean/Bumbledb/Query/Syntax.lean: PRule` heads are variable
///    rows, `finds : List VarId`).
/// 4. **The signature fixpoint**: each predicate's sealed signature
///    derives from its first rule whose `Idb` targets are all already
///    sealed — chaotic iteration, at most one pass per predicate — and
///    a predicate that never seals is the typed
///    `UnresolvedPredicateSignature`. Then the strict pass: every rule
///    of every predicate under the full per-rule roster with all
///    signatures sealed, head types aligned against the sealing rule's
///    row, params unified **program-globally** (one binding surface).
///
/// A sealed [`ValidatedProgram`] executes whole: the per-stratum
/// fixpoint driver (`api/prepared/fixpoint.rs`) consumes the witness —
/// `Idb` occurrences included — and computes
/// `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram`'s answers
/// (`program_eval_sound`).
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item. Predicates validate
/// in `PredId` order; rule-local payloads name positions inside the
/// first failing rule of the first failing predicate.
///
/// # Panics
///
/// Only on programmer-invariant violations (the sealing loop seals
/// every predicate before the witnesses assemble).
pub fn validate_program(
    schema: &Schema,
    program: &Program,
) -> Result<ValidatedProgram, ValidationError> {
    if program.predicates.len() > MAX_PREDICATES {
        return Err(ValidationError::TooManyPredicates {
            count: program.predicates.len(),
        });
    }
    if usize::from(program.output.0) >= program.predicates.len() {
        return Err(ValidationError::UnknownOutputPredicate {
            pred: program.output,
        });
    }
    let lowered: Vec<Vec<LoweredRule>> = program
        .predicates
        .iter()
        .map(|def| lower_rules(&def.head, &def.rules))
        .collect::<Result<_, _>>()?;
    // Head-shape alignment per predicate (the type half runs after the
    // typing fixpoint, against the sealing rule's row).
    for (def, rules) in program.predicates.iter().zip(&lowered) {
        for (rule_idx, rule) in rules.iter().enumerate() {
            check_head_alignment(&def.head, rule, rule_idx)?;
        }
    }
    let arities: Vec<usize> = program
        .predicates
        .iter()
        .map(|def| def.head.len())
        .collect();
    let strata = super::strata::stratify(&arities, &lowered)?;

    // The executable-class roster item beside the strata judge: a fold-
    // headed predicate is legal only AT the output — a fold's answers
    // materialize at finalize, on the output's head-owned sink, while an
    // interior predicate's answers are a transient word-row table read
    // by `Idb` occurrences (the Lean cut cannot even represent a
    // program-level fold head: `lean/Bumbledb/Query/Syntax.lean: PRule`
    // has `finds : List VarId`, and `lean/Bumbledb/Exec/Fixpoint.lean:
    // evalProgram` computes projection heads only). Aggregation *of*
    // lower strata stays legal (20-query-ir.md § engine recursion): the OUTPUT
    // predicate folds over finished `Idb` sets freely.
    for (index, def) in program.predicates.iter().enumerate() {
        let pred = PredId(u16::try_from(index).expect("predicate count capped at 16"));
        if pred == program.output {
            continue;
        }
        if def
            .head
            .iter()
            .any(|term| matches!(term, crate::ir::HeadTerm::Aggregate(_)))
        {
            return Err(ValidationError::AggregateInteriorPredicate { pred });
        }
        // The measure half of the same item: a `Measure` find lowers to a
        // `HeadTerm::Var` position (a value column), so the head scan
        // above never sees it — but `PRule.finds : List VarId` cannot
        // represent it either, recursive or not. The recursive form was
        // already refused by the strata roster (`MeasureInRecursiveHead`);
        // this arm refuses the non-recursive interior remainder.
        if lowered[index]
            .iter()
            .flat_map(|rule| &rule.finds)
            .any(|term| matches!(term, FindTerm::Measure(_)))
        {
            return Err(ValidationError::MeasureInteriorPredicate { pred });
        }
    }

    let (sealed, pinned_rows) = seal_signatures(schema, &arities, &lowered)?;

    // The strict pass: the full per-rule roster with every signature
    // sealed, the head-type alignment against the sealing rule's row,
    // and program-global param unification.
    let count = program.predicates.len();
    let mut params = ParamTables::default();
    let mut typings: Vec<Vec<RuleTyping>> = Vec::with_capacity(count);
    for index in 0..count {
        let sigs = IdbSignatures {
            arities: &arities,
            sealed: &sealed,
        };
        let mut rules = Vec::with_capacity(lowered[index].len());
        for (rule_idx, rule) in lowered[index].iter().enumerate() {
            let (typing, ctx) = validate_rule(schema, &sigs, rule)?;
            let row = input_row(rule, &typing);
            if let Some(position) = (0..row.len()).find(|i| row[*i] != pinned_rows[index][*i]) {
                return Err(ValidationError::HeadTypeMismatch {
                    rule: rule_idx,
                    position,
                });
            }
            params.unify(ctx)?;
            rules.push(typing);
        }
        typings.push(rules);
    }
    params.check_masks_and_density()?;

    let predicates = lowered
        .into_iter()
        .zip(sealed)
        .zip(typings)
        .map(|((lowered, predicate), rules)| {
            seal(
                lowered,
                predicate.expect("every predicate sealed above"),
                rules,
                &params,
            )
        })
        .collect();
    Ok(ValidatedProgram {
        predicates,
        output: program.output,
        strata,
    })
}

/// The sealing loop — the one signature derivation, quantified over
/// predicates: a predicate seals from its FIRST rule whose `Idb`
/// targets are all sealed (for a no-`Idb` predicate that is rule 0,
/// exactly the query path's pinning). Chaotic iteration; every pass
/// seals at least one predicate or stops, so the loop is bounded by the
/// predicate cap. A rule eligible for sealing validates with every
/// anchor available, so its errors are real and propagate. A predicate
/// that never seals — every rule reads a same-SCC predicate whose own
/// signature is still underived — is the typed
/// `UnresolvedPredicateSignature`. Returns the sealed signatures
/// (every entry `Some`) and each predicate's pinned input row.
#[expect(
    clippy::type_complexity,
    reason = "the paired seal products are consumed immediately by the strict pass"
)]
fn seal_signatures(
    schema: &Schema,
    arities: &[usize],
    lowered: &[Vec<LoweredRule>],
) -> Result<(Vec<Option<Predicate>>, Vec<Vec<ValueType>>), ValidationError> {
    let count = lowered.len();
    let mut sealed: Vec<Option<Predicate>> = vec![None; count];
    let mut pinned_rows: Vec<Vec<ValueType>> = vec![Vec::new(); count];
    loop {
        let mut progress = false;
        for index in 0..count {
            if sealed[index].is_some() {
                continue;
            }
            let eligible = lowered[index].iter().find(|rule| {
                idb_targets(rule)
                    .all(|pred| sealed.get(usize::from(pred.0)).is_some_and(Option::is_some))
            });
            let Some(rule) = eligible else { continue };
            let sigs = IdbSignatures {
                arities,
                sealed: &sealed,
            };
            let (typing, _ctx) = validate_rule(schema, &sigs, rule)?;
            pinned_rows[index] = input_row(rule, &typing);
            sealed[index] = Some(Predicate::derive(rule, &typing));
            progress = true;
        }
        if !progress {
            break;
        }
    }
    if let Some(index) = sealed.iter().position(Option::is_none) {
        return Err(ValidationError::UnresolvedPredicateSignature {
            pred: PredId(u16::try_from(index).expect("predicate count capped at 16")),
        });
    }
    Ok((sealed, pinned_rows))
}

/// The `Idb` predicates a lowered rule reads, positive and negated —
/// the sealing loop's eligibility surface.
fn idb_targets(rule: &LoweredRule) -> impl Iterator<Item = PredId> + '_ {
    rule.atoms
        .iter()
        .chain(&rule.negated)
        .filter_map(|atom| atom.source.idb())
}

/// One predicate's witness, sealed from its lowered rules, signature,
/// typings, and the (program-global on the program path) param tables.
fn seal(
    lowered: Vec<LoweredRule>,
    predicate: Predicate,
    rules: Vec<RuleTyping>,
    params: &ParamTables,
) -> ValidatedQuery {
    let set_params = params
        .param_kinds
        .iter()
        .filter_map(|(param, kind)| matches!(kind, ParamKind::Set).then_some(*param))
        .collect();
    ValidatedQuery {
        lowered,
        predicate,
        rules,
        param_types: params.param_types.clone(),
        set_params,
        point_params: params.point_params.clone(),
        mask_params: params.mask_params.clone(),
    }
}

/// The query-shape half of the roster, per predicate: empty rule set,
/// the rule cap, empty head, the nesting boundary check, DNF
/// distribution under its structural cap, and the Arg-across-rules
/// refusal — yielding the Or-free lowered rules everything downstream
/// reads.
fn lower_rules(
    head: &[crate::ir::HeadTerm],
    rules: &[crate::ir::Rule],
) -> Result<Vec<LoweredRule>, ValidationError> {
    if rules.is_empty() {
        return Err(ValidationError::EmptyRuleSet);
    }
    if rules.len() > MAX_RULES {
        return Err(ValidationError::TooManyRules { count: rules.len() });
    }
    if head.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }

    // The nesting boundary check runs before ANY recursive tree walk
    // (the trust-boundary law: `disjunct_count` and `distribute` recurse
    // by depth, so a hostile depth must be a typed rejection here, judged
    // by the iterative `nesting_depth`, never a stack exhaustion there).
    for (rule_idx, rule) in rules.iter().enumerate() {
        let depth = nesting_depth(&rule.conditions);
        if depth > MAX_CONDITION_DEPTH {
            return Err(ValidationError::ConditionNestingTooDeep {
                rule: rule_idx,
                depth,
                cap: MAX_CONDITION_DEPTH,
            });
        }
    }

    // DNF distribution: the cap is judged on the structural term count —
    // no disjunct of an exponential case is ever materialized — and the
    // distributed program collapses duplicates (set semantics at the
    // representation level).
    let produced = rules
        .iter()
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            produced,
            cap: MAX_RULES,
        });
    }
    let distributed = rules
        .iter()
        .enumerate()
        .flat_map(|(written, rule)| {
            // The written-rule provenance stamp (R2): rule counts are
            // capped above, so the index fits.
            let written = u16::try_from(written).expect("rule count capped");
            distribute(rule).into_iter().map(move |mut lowered| {
                lowered.written = Some(written);
                lowered
            })
        })
        .collect();
    let lowered = collapse(distributed);
    if lowered.is_empty() {
        // Every rule's disjunction was empty: the program lowered to the
        // empty union — no query.
        return Err(ValidationError::EmptyRuleSet);
    }
    // Arg-restriction across rules is undefined — the restriction key is
    // a rule-scoped variable outside the head's vocabulary, and rules
    // need not even agree on its type — so it refuses at the boundary,
    // judged on the LOWERED rule count (a DNF blowup of one Arg rule
    // refuses too). Modeling answer: one Arg query per disjunct,
    // host-merged (20-query-ir § aggregation).
    if lowered.len() > 1
        && head.iter().any(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Aggregate(
                    crate::ir::HeadOp::ArgMax | crate::ir::HeadOp::ArgMin
                )
            )
        })
    {
        return Err(ValidationError::ArgAcrossRules {
            rules: lowered.len(),
        });
    }
    // Cross-rule fold-free nullary `Count` refuses beside the Arg
    // refusal (ruled 2026-07-23, R1): under the head-projection law a
    // fold-free head admits one projection per group, so the Count is
    // definitionally the constant 1 — uninformative, made
    // unrepresentable, same modeling answer (one Count per disjunct,
    // host-merged). Judged on PROVENANCE, not the lowered count: a
    // DNF-derived rule set is exempt — or-transparency (R2) keeps its
    // fold domain the written rule's full binding set, so its Count
    // counts.
    if lowered.len() > 1
        && dnf_derived(&lowered).is_none()
        && head.iter().any(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Count)
            )
        })
        && head.iter().all(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Var | crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Count)
            )
        })
    {
        return Err(ValidationError::CountAcrossRules {
            rules: lowered.len(),
        });
    }
    Ok(lowered)
}

/// The provenance judgment (ruled 2026-07-23, R2): `Some(written)` iff
/// every lowered rule carries the ONE shared written-rule index — the
/// set is DNF-derived from that rule and the union dedup re-keys on
/// the shared slot arrays. `None` is a hand-written rule set (or a
/// cross-written collapse), which keys the head projection.
pub(crate) fn dnf_derived(lowered: &[LoweredRule]) -> Option<u16> {
    let first = lowered.first()?.written?;
    lowered
        .iter()
        .all(|rule| rule.written == Some(first))
        .then_some(first)
}

/// Head alignment, the shape half: arity, then var-vs-aggregate-op kind
/// position by position (types are checked against the pinned row after
/// the rule's own typing fixpoint resolves them).
fn check_head_alignment(
    head: &[crate::ir::HeadTerm],
    rule: &LoweredRule,
    rule_idx: usize,
) -> Result<(), ValidationError> {
    if rule.finds.len() != head.len() {
        return Err(ValidationError::HeadArityMismatch {
            rule: rule_idx,
            expected: head.len(),
            found: rule.finds.len(),
        });
    }
    for (position, (term, head_term)) in rule.finds.iter().zip(head).enumerate() {
        if term.head_term() != *head_term {
            return Err(ValidationError::HeadAggregateMismatch {
                rule: rule_idx,
                position,
            });
        }
    }
    Ok(())
}

/// The per-rule roster — exactly the conjunctive query's checks, over one
/// rule's own variable scope and its own bivalent-anchor typing fixpoint;
/// `idb` is the target-signature surface `Idb` anchors resolve against
/// (empty on the query path).
fn validate_rule(
    schema: &Schema,
    idb: &IdbSignatures<'_>,
    rule: &LoweredRule,
) -> Result<(RuleTyping, Context), ValidationError> {
    if rule.atoms.is_empty() {
        return Err(ValidationError::NoPositiveAtoms);
    }
    // The planner caps are roster items: rejected here, at the boundary,
    // so nothing downstream (normalize's u16 occurrence ids, the DP's
    // bitmask table, the 128-bit variable bitsets) ever sees an
    // over-limit rule. Negated atoms are occurrences too — each one is
    // an anti-probe the DP places — so they count. Per rule: each rule
    // plans alone (the rule cap is counted independently, at the top).
    let occurrences = rule.atoms.len() + rule.negated.len();
    if occurrences > crate::plan::planner::MAX_OCCURRENCES {
        return Err(ValidationError::TooManyAtoms { count: occurrences });
    }
    for (index, term) in rule.finds.iter().enumerate() {
        if rule.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, idb, rule)?;
    let classified = ctx.check_comparisons(rule)?;
    ctx.check_membership_domains()?;
    // The group key (non-aggregated find variables) is computed once and
    // shared between the find checks and the witness. A measure find's
    // variable is a group-key variable: the position projects a value per
    // binding (the measure word), so its variable is grouped-by exactly
    // like a plain projected variable — and an aggregate over it would be
    // constant per group (`AggregateOverGroupKey`).
    let group_key: BTreeSet<VarId> = rule
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) | FindTerm::Measure(var) => Some(*var),
            FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => None,
        })
        .collect();
    ctx.check_finds(rule, &group_key)?;
    if ctx.var_types.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_types.len(),
        });
    }

    // The variable types are already resolved (`resolve_bivalents`
    // consumed the inference slots into `var_types` during
    // `check_comparisons`): the typing takes them verbatim.
    let var_types = ctx.var_types.clone();
    let closed_vars = ctx.closed_vars.clone();
    Ok((
        RuleTyping {
            var_types,
            group_key,
            classified,
            closed_vars,
        },
        ctx,
    ))
}

/// One rule's positional INPUT contribution to the alignment check: a
/// variable position carries the variable's type; an aggregate position
/// its fold input type (the nullary `Count` is `U64`; an Arg position
/// carries the *carried* variable's type — the key is rule-internal).
/// Alignment-only — the signature is [`super::Predicate::derive`],
/// never this row (their one divergence: a `CountDistinct` input).
fn input_row(rule: &LoweredRule, typing: &RuleTyping) -> Vec<ValueType> {
    let var_type = |var: &VarId| typing.var_types[var].clone();
    rule.finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => var_type(var),
            // The measure is u64 by definition — projected or folded.
            FindTerm::Measure(_) | FindTerm::AggregateMeasure { .. } => ValueType::U64,
            FindTerm::Aggregate { op, over } => match op {
                AggOp::Count => ValueType::U64,
                // A Pack position's row entry is an interval — the packed
                // segment shares its input's interval type, so the pinned
                // row carries `Interval(E)` there like any interval find.
                AggOp::Sum
                | AggOp::Min
                | AggOp::Max
                | AggOp::CountDistinct
                | AggOp::ArgMax { .. }
                | AggOp::ArgMin { .. }
                | AggOp::Pack => var_type(&over.expect("validated: only Count is nullary")),
            },
        })
        .collect()
}

/// The query-global param tables, unified across the rules' independent
/// typing fixpoints: one binding surface, so every rule's resolution of a
/// param must agree — in type, in scalar-vs-set role, and in
/// value-vs-mask role.
#[derive(Default)]
struct ParamTables {
    param_types: BTreeMap<ParamId, ValueType>,
    param_kinds: BTreeMap<ParamId, ParamKind>,
    mask_params: BTreeSet<ParamId>,
    point_params: BTreeSet<ParamId>,
}

impl ParamTables {
    /// Absorbs one rule's resolved param state, diagnosing cross-rule
    /// disagreements with the same errors the per-rule checks use.
    fn unify(&mut self, ctx: Context) -> Result<(), ValidationError> {
        // Point-position params (the point-domain law): anchored at an
        // interval position and resolved element-typed — their bound
        // values are points, so bind rejects the domain ceiling.
        for param in &ctx.interval_position_params {
            if matches!(
                ctx.param_slots.get(param),
                Some(TypeSlot::Mono(ValueType::U64 | ValueType::I64))
            ) {
                self.point_params.insert(*param);
            }
        }
        for (param, slot) in ctx.param_slots {
            let value_type = match slot {
                TypeSlot::Mono(value_type) => value_type,
                TypeSlot::Bivalent { .. } => unreachable!("resolve_bivalents ran"),
            };
            match self.param_types.get(&param) {
                Some(existing) if *existing != value_type => {
                    return Err(ValidationError::ParamTypeConflict { param });
                }
                Some(_) => {}
                None => {
                    self.param_types.insert(param, value_type);
                }
            }
        }
        for (param, kind) in ctx.param_kinds {
            match self.param_kinds.get(&param) {
                Some(existing) if *existing != kind => {
                    return Err(ValidationError::ParamScalarAndSet { param });
                }
                Some(_) => {}
                None => {
                    self.param_kinds.insert(param, kind);
                }
            }
        }
        self.mask_params.extend(ctx.mask_params);
        Ok(())
    }

    /// The two whole-program param rules, checked after every rule
    /// contributed: a mask param with any value anchor anywhere (a mask
    /// is not a data-model type), and id density — jointly across value
    /// and mask params, across all rules (a gap would be a positional
    /// slot at execution whose supplied value is never type-checked).
    fn check_masks_and_density(&self) -> Result<(), ValidationError> {
        for param in &self.mask_params {
            if self.param_types.contains_key(param) {
                return Err(ValidationError::ParamTypeConflict { param: *param });
            }
        }
        for (position, param) in self.param_kinds.keys().enumerate() {
            if usize::from(param.0) != position {
                return Err(ValidationError::ParamIdGap {
                    param: ParamId(u16::try_from(position).expect("param ids fit u16")),
                });
            }
        }
        Ok(())
    }
}
