//! Guard-probe access path dispatch (PRD 23): the point-lookup fast path
//! that routes qualifying queries around the join machinery entirely
//! (`docs/architecture/30-execution.md` — access paths; `40-storage.md`'s
//! `U`/`M` read-side readers).
//!
//! The dispatch is a **representation**, not a runtime mode: classification
//! happens once at prepare time into the two-variant [`ExecPlan`]; the
//! branch exists exactly once. No images are touched on the guard path —
//! it works identically on a cold, just-committed database (the latency
//! property the decision exists for).

use crate::encoding::{encode_u64, field_bytes};
use crate::error::Result;
use crate::exec::run::{Bindings, Sink};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::NormalizedQuery;
use crate::ir::{CmpOp, VarId};
use crate::plan::fj::ValidatedPlan;
use crate::schema::{ConstraintId, FieldId, RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::{dict, read};

/// The prepared execution plan: either the guard-probe fast path or the
/// Free Join engine.
#[derive(Debug)]
pub enum ExecPlan {
    GuardProbe(GuardPlan),
    FreeJoin(ValidatedPlan),
}

impl ExecPlan {
    /// The binding-slot order (shared vocabulary between both variants so
    /// sinks are built identically).
    #[must_use]
    pub fn slots(&self) -> Vec<VarId> {
        match self {
            Self::GuardProbe(guard) => guard.vars.iter().map(|(_, v)| *v).collect(),
            Self::FreeJoin(plan) => plan.slots().to_vec(),
        }
    }

    /// The slot index of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        match self {
            Self::GuardProbe(guard) => guard
                .vars
                .iter()
                .position(|(_, v)| *v == var)
                .expect("guard plans bind every variable"),
            Self::FreeJoin(plan) => plan.slot_of(var),
        }
    }

    /// The distinct-bindings elision flag (trivially true for a guard
    /// probe: at most one binding exists).
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match self {
            Self::GuardProbe(_) => true,
            Self::FreeJoin(plan) => plan.distinct_bindings(),
        }
    }
}

/// The point-lookup plan: one `U`-guard (or `M`-membership) get, one `F`
/// fetch, a decode — no images, no COLT, no plan search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPlan {
    pub relation: RelationId,
    /// The probed unique constraint; `None` means every field is constant
    /// and the probe is a full-fact `M` membership check.
    pub constraint: Option<ConstraintId>,
    /// The key constants in guard-key field order.
    pub key: Vec<(FieldId, Const)>,
    /// Filters not consumed by the key, checked on the fetched fact
    /// (fields outside the unique key may still be constrained).
    pub remaining_filters: Vec<FilterPredicate>,
    /// Variables decoded from the fetched fact: `(field, var)`; slot order
    /// is this order.
    pub vars: Vec<(FieldId, VarId)>,
}

/// Classifies a normalized query: `Some(GuardPlan)` iff it is guard-probe
/// eligible — exactly one atom occurrence, no residuals, and the
/// occurrence's Eq-constant fields cover a unique constraint (including
/// serial auto-uniques) or the full fact.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths).
#[must_use]
pub fn classify(normalized: &NormalizedQuery, schema: &Schema) -> Option<GuardPlan> {
    let [occurrence] = normalized.occurrences.as_slice() else {
        return None;
    };
    if !normalized.residuals.is_empty() {
        return None;
    }
    let relation = schema.relation(occurrence.relation);

    // The fields pinned to constants by Eq filters, with their constants.
    let constant_of = |field: FieldId| {
        occurrence.filters.iter().find_map(|f| match f {
            FilterPredicate::Compare {
                field: candidate,
                op: CmpOp::Eq,
                value,
            } if *candidate == field => Some(value.clone()),
            _ => None,
        })
    };

    // Prefer a unique-constraint probe; fall back to the full-fact
    // membership check when every field is constant.
    let (constraint, key_fields): (Option<ConstraintId>, Vec<FieldId>) = relation
        .unique_constraints()
        .iter()
        .find(|cid| {
            relation
                .constraint(**cid)
                .fields()
                .iter()
                .all(|f| constant_of(*f).is_some())
        })
        .map(|cid| (Some(*cid), relation.constraint(*cid).fields().to_vec()))
        .or_else(|| {
            let all: Vec<FieldId> = (0..relation.fields().len())
                .map(|i| FieldId(u16::try_from(i).expect("validated schema")))
                .collect();
            all.iter()
                .all(|f| constant_of(*f).is_some())
                .then_some((None, all))
        })?;

    let key: Vec<(FieldId, Const)> = key_fields
        .iter()
        .map(|f| (*f, constant_of(*f).expect("checked above")))
        .collect();
    // Filters not consumed by the key: everything except one Eq filter per
    // key field (the consumed constant).
    let mut consumed: Vec<FieldId> = key_fields;
    let remaining_filters: Vec<FilterPredicate> = occurrence
        .filters
        .iter()
        .filter(|f| match f {
            FilterPredicate::Compare {
                field,
                op: CmpOp::Eq,
                ..
            } => {
                if let Some(idx) = consumed.iter().position(|c| c == field) {
                    consumed.swap_remove(idx);
                    false
                } else {
                    true
                }
            }
            _ => true,
        })
        .cloned()
        .collect();

    Some(GuardPlan {
        relation: occurrence.relation,
        constraint,
        key,
        remaining_filters,
        vars: occurrence.vars.clone(),
    })
}

/// Resolves a constant to its canonical guard-key bytes. `None` means a
/// `PendingIntern` missed the dictionary: the literal cannot match any
/// fact — empty result, never an insert, never an error.
fn const_bytes(
    txn: &ReadTxn<'_>,
    value: &Const,
    params: &[Const],
    out: &mut Vec<u8>,
) -> Result<bool> {
    match value {
        Const::Word(w) => out.extend_from_slice(&w.to_be_bytes()),
        Const::Byte(b) => out.push(*b),
        Const::Param(p) => {
            return const_bytes(txn, &params[usize::from(p.0)], params, out);
        }
        Const::PendingIntern { tag, bytes } => {
            let Some(id) = dict::lookup_tagged(txn, *tag, bytes)? else {
                return Ok(false);
            };
            out.extend_from_slice(&encode_u64(id));
        }
    }
    Ok(true)
}

/// The constant's column word (for filter checks on the fetched fact).
fn const_word(txn: &ReadTxn<'_>, value: &Const, params: &[Const]) -> Result<Option<u64>> {
    match value {
        Const::Word(w) => Ok(Some(*w)),
        Const::Byte(b) => Ok(Some(u64::from(*b))),
        Const::Param(p) => const_word(txn, &params[usize::from(p.0)], params),
        Const::PendingIntern { tag, bytes } => Ok(dict::lookup_tagged(txn, *tag, bytes)?),
    }
}

/// One field's column word sliced straight out of canonical fact bytes.
fn fact_word(schema: &Schema, plan: &GuardPlan, fact: &[u8], field: FieldId) -> u64 {
    let layout = schema.relation(plan.relation).layout();
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    match bytes.len() {
        1 => u64::from(bytes[0]),
        _ => u64::from_be_bytes(bytes.try_into().expect("8-byte field")),
    }
}

/// Executes the guard probe: guard key from constants, one `U`/`M` get,
/// one `F` fetch, remaining filters on the fact bytes, then the single
/// binding through the ordinary sink (sinks are reused, not special-cased).
///
/// # Errors
///
/// `Lmdb`/`Corruption` from the storage reads. A missing key or a failed
/// filter is not an error: the result is simply empty.
pub fn execute_guard<S: Sink>(
    plan: &GuardPlan,
    txn: &ReadTxn<'_>,
    schema: &Schema,
    params: &[Const],
    bindings: &mut Bindings,
    sink: &mut S,
) -> Result<()> {
    // Build the guard key; a PendingIntern miss empties the query.
    let mut key_bytes = Vec::new();
    for (_, value) in &plan.key {
        if !const_bytes(txn, value, params, &mut key_bytes)? {
            return Ok(());
        }
    }

    let row_id = match plan.constraint {
        Some(constraint) => read::unique_row(txn, plan.relation, constraint, &key_bytes)?,
        None => read::fact_row(txn, plan.relation, &key_bytes)?,
    };
    let Some(row_id) = row_id else {
        return Ok(()); // miss: empty result
    };
    let fact = read::fetch(txn, schema, plan.relation, row_id)?;

    // Remaining filters run on the fact bytes.
    for filter in &plan.remaining_filters {
        let pass = match filter {
            FilterPredicate::Compare { field, op, value } => {
                let Some(expected) = const_word(txn, value, params)? else {
                    return Ok(()); // unresolvable intern: cannot match
                };
                op.compare(&fact_word(schema, plan, fact, *field), &expected)
            }
            FilterPredicate::FieldsEqual { left, right } => {
                fact_word(schema, plan, fact, *left) == fact_word(schema, plan, fact, *right)
            }
        };
        if !pass {
            return Ok(());
        }
    }

    // The single binding, through the ordinary sink.
    bindings.reset();
    for (slot, (field, _)) in plan.vars.iter().enumerate() {
        bindings.set(slot, fact_word(schema, plan, fact, *field));
    }
    sink.emit(bindings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::exec::colt::Colt;
    use crate::exec::run::{Executor, NoopCounters};
    use crate::exec::sink::{AggregateSink, FindSpec, ProjectionSink};
    use crate::image::view::apply;
    use crate::ir::normalize::{OccId, Occurrence, PlacedComparison};
    use crate::ir::{AggOp, ParamId};
    use crate::plan::fj::{binary2fj, factor, validate};
    use crate::plan::planner::JoinOrder;
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::BTreeSet;

    /// Account(id serial u64, holder u64, name string).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const ACCOUNT: RelationId = RelationId(0);

    fn occurrence(vars: &[(u16, u16)], filters: Vec<FilterPredicate>) -> Occurrence {
        Occurrence {
            occ_id: OccId(0),
            relation: ACCOUNT,
            vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
            filters,
        }
    }

    fn eq_filter(field: u16, value: Const) -> FilterPredicate {
        FilterPredicate::Compare {
            field: FieldId(field),
            op: CmpOp::Eq,
            value,
        }
    }

    fn single(occurrence: Occurrence) -> NormalizedQuery {
        NormalizedQuery {
            occurrences: vec![occurrence],
            residuals: vec![],
        }
    }

    /// Commits accounts (id, holder, name) and returns the environment.
    fn populated(dir: &TempDir, schema: &Schema, rows: &[(u64, u64, &str)]) -> Environment {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (id, holder, name) in rows {
            let name_id = delta.intern_str(&view, name).expect("intern");
            let mut bytes = Vec::new();
            encode_fact(
                &[
                    ValueRef::U64(*id),
                    ValueRef::U64(*holder),
                    ValueRef::String(name_id),
                ],
                schema.relation(ACCOUNT).layout(),
                &mut bytes,
            );
            delta.insert(&view, ACCOUNT, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        env
    }

    // ---------- classification ----------

    #[test]
    fn fully_unique_bound_single_atom_classifies_as_guard_probe() {
        let schema = schema();
        let normalized = single(occurrence(
            &[(1, 0), (2, 1)],
            vec![eq_filter(0, Const::Word(5))], // id = 5, the serial auto-unique
        ));
        let plan = classify(&normalized, &schema).expect("guard probe");
        assert_eq!(plan.constraint, Some(ConstraintId(0)));
        assert_eq!(plan.key, vec![(FieldId(0), Const::Word(5))]);
        assert!(plan.remaining_filters.is_empty());
    }

    #[test]
    fn a_second_atom_or_a_residual_stays_free_join() {
        let schema = schema();
        let occ = occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]);
        let two_atoms = NormalizedQuery {
            occurrences: vec![occ.clone(), occ.clone()],
            residuals: vec![],
        };
        assert!(classify(&two_atoms, &schema).is_none());

        let with_residual = NormalizedQuery {
            occurrences: vec![occurrence(
                &[(1, 0), (2, 1)],
                vec![eq_filter(0, Const::Word(5))],
            )],
            residuals: vec![PlacedComparison {
                op: CmpOp::Lt,
                lhs: VarId(0),
                rhs: VarId(1),
            }],
        };
        assert!(classify(&with_residual, &schema).is_none());
    }

    #[test]
    fn a_partially_bound_unique_stays_free_join() {
        let schema = schema();
        // Only a non-key field is constant: no unique coverage, not full.
        let normalized = single(occurrence(
            &[(0, 0), (2, 1)],
            vec![eq_filter(1, Const::Word(9))],
        ));
        assert!(classify(&normalized, &schema).is_none());
    }

    #[test]
    fn extra_filters_survive_as_remaining() {
        let schema = schema();
        let normalized = single(occurrence(
            &[(2, 0)],
            vec![
                eq_filter(0, Const::Word(5)),
                eq_filter(1, Const::Word(7)), // outside the key
            ],
        ));
        let plan = classify(&normalized, &schema).expect("guard probe");
        assert_eq!(plan.remaining_filters, vec![eq_filter(1, Const::Word(7))]);
    }

    // ---------- execution ----------

    fn run_guard(
        plan: &GuardPlan,
        env: &Environment,
        schema: &Schema,
        params: &[Const],
    ) -> Vec<Vec<u64>> {
        let txn = env.read_txn().expect("txn");
        let mut bindings = Bindings::new(plan.vars.len());
        let mut sink = ProjectionSink::new((0..plan.vars.len()).collect());
        execute_guard(plan, &txn, schema, params, &mut bindings, &mut sink).expect("execute");
        sink.rows().map(<[u64]>::to_vec).collect()
    }

    #[test]
    fn hit_miss_and_filter_rejection() {
        let dir = TempDir::new("guard-hit-miss");
        let schema = schema();
        let env = populated(&dir, &schema, &[(5, 7, "alice"), (6, 8, "bob")]);
        let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
        let plan = classify(&normalized, &schema).expect("guard probe");
        assert_eq!(run_guard(&plan, &env, &schema, &[]), vec![vec![7]]);

        // Miss: no such id.
        let missing = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(99))]));
        let plan = classify(&missing, &schema).expect("guard probe");
        assert!(run_guard(&plan, &env, &schema, &[]).is_empty());

        // Hit, but a remaining filter rejects the fetched fact.
        let rejected = single(occurrence(
            &[(1, 0)],
            vec![
                eq_filter(0, Const::Word(5)),
                eq_filter(1, Const::Word(999)), // holder is 7, not 999
            ],
        ));
        let plan = classify(&rejected, &schema).expect("guard probe");
        assert!(run_guard(&plan, &env, &schema, &[]).is_empty());
    }

    #[test]
    fn param_driven_keys_resolve_at_bind_time() {
        let dir = TempDir::new("guard-param");
        let schema = schema();
        let env = populated(&dir, &schema, &[(5, 7, "alice")]);
        let normalized = single(occurrence(
            &[(1, 0)],
            vec![eq_filter(0, Const::Param(ParamId(0)))],
        ));
        let plan = classify(&normalized, &schema).expect("guard probe");
        assert_eq!(
            run_guard(&plan, &env, &schema, &[Const::Word(5)]),
            vec![vec![7]]
        );
        assert!(run_guard(&plan, &env, &schema, &[Const::Word(6)]).is_empty());
    }

    #[test]
    fn pending_intern_miss_is_empty_and_never_interns() {
        let dir = TempDir::new("guard-intern-miss");
        let schema = schema();
        let env = populated(&dir, &schema, &[(5, 7, "alice")]);
        // Full-fact-ish probe via the name field being part of no unique:
        // instead, probe by id but filter on a never-interned name.
        let normalized = single(occurrence(
            &[(1, 0)],
            vec![
                eq_filter(0, Const::Word(5)),
                eq_filter(
                    2,
                    Const::PendingIntern {
                        tag: 0,
                        bytes: Box::from(&b"ghost"[..]),
                    },
                ),
            ],
        ));
        let plan = classify(&normalized, &schema).expect("guard probe");
        assert!(run_guard(&plan, &env, &schema, &[]).is_empty());
        // The read path never interned the ghost string.
        let txn = env.read_txn().expect("txn");
        assert_eq!(dict::lookup_str(&txn, "ghost").expect("lookup"), None);
    }

    #[test]
    fn guard_and_free_join_paths_agree_by_construction() {
        let dir = TempDir::new("guard-equivalence");
        let schema = schema();
        let env = populated(&dir, &schema, &[(5, 7, "alice"), (6, 8, "bob")]);
        let normalized = single(occurrence(
            &[(1, 0), (2, 1)],
            vec![eq_filter(0, Const::Word(6))],
        ));

        // Guard path.
        let guard = classify(&normalized, &schema).expect("guard probe");
        let mut guard_rows = run_guard(&guard, &env, &schema, &[]);
        guard_rows.sort_unstable();

        // Free Join path over the same normalized query.
        let order = JoinOrder {
            order: vec![OccId(0)],
            estimates: vec![0],
        };
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        let plan =
            validate(&fj, &normalized, &schema, vec![0], &BTreeSet::new()).expect("valid plan");
        let txn = env.read_txn().expect("txn");
        let image = crate::image::build(&txn, &schema, ACCOUNT).expect("build");
        let view = std::sync::Arc::new(apply(
            &image,
            &normalized.occurrences[0].filters,
            &[],
            Vec::new(),
        ));
        let columns: Vec<Vec<usize>> = plan.occurrences()[0]
            .trie_schema
            .iter()
            .map(|level| {
                level
                    .iter()
                    .map(|var| {
                        let (field, _) = plan.occurrences()[0]
                            .vars
                            .iter()
                            .find(|(_, v)| v == var)
                            .expect("plan vars");
                        usize::from(field.0)
                    })
                    .collect()
            })
            .collect();
        let mut colts = vec![Colt::new(&view, columns)];
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = ProjectionSink::new(
            [VarId(0), VarId(1)]
                .iter()
                .map(|v| plan.slot_of(*v))
                .collect(),
        );
        Executor::new(&plan).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        let mut fj_rows: Vec<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
        fj_rows.sort_unstable();

        assert_eq!(guard_rows, fj_rows);
        assert_eq!(guard_rows.len(), 1);
    }

    #[test]
    fn aggregate_over_a_point_lookup_folds_one_binding() {
        let dir = TempDir::new("guard-aggregate");
        let schema = schema();
        let env = populated(&dir, &schema, &[(5, 7, "alice")]);
        let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
        let plan = classify(&normalized, &schema).expect("guard probe");
        let txn = env.read_txn().expect("txn");
        let mut bindings = Bindings::new(1);
        let mut sink = AggregateSink::new(
            vec![FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            }],
            1,
            true,
        );
        execute_guard(&plan, &txn, &schema, &[], &mut bindings, &mut sink).expect("execute");
        assert_eq!(sink.into_rows().expect("rows"), vec![vec![1]]);
    }

    // No image build can occur on the guard path: `execute_guard` takes no
    // image, view, or cache argument — the property holds by API shape, on
    // a cold database in every test above.
}
