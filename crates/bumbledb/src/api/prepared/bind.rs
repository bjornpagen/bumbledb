use super::{Const, ExecPlan, Executor, FilterPredicate, PreparedQuery, ValueType};

use crate::error::{Error, Result};
use crate::ir::{ParamId, Value};
use crate::storage::dict;
use crate::storage::env::ReadTxn;

impl PreparedQuery<'_> {
    /// Rebuilds the executor scratch at a different batch size — the
    /// tuning/test surface for D4's measurement-owned constant. Allocation
    /// happens here, outside any measured window. A no-op for guard
    /// probes. Hidden: a measurement affordance, not a knob on the
    /// no-knobs surface (`docs/architecture/00-product.md`).
    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        if let ExecPlan::FreeJoin(plan) = &self.plan {
            self.executor = Some(Executor::with_batch_size(plan, batch));
        }
    }

    /// The identity check at every execution entry (`execute` and
    /// `profile`; `execute_collect` and `explain` route through them):
    /// a snapshot of any environment other than the preparing one is a
    /// typed error before anything else runs. One u64 compare — with the
    /// entry guarded, the view memo needs no environment epoch in its
    /// generation keys.
    pub(super) fn check_snapshot(&self, txn: &ReadTxn<'_>) -> Result<()> {
        if txn.env_instance() == self.env_instance {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
        }
    }

    /// Binds and converts parameters; `Ok(false)` = a String/Bytes value
    /// that was never interned (the query is empty on this snapshot).
    pub(super) fn bind_params(&mut self, txn: &ReadTxn<'_>, params: &[Value]) -> Result<()> {
        if params.len() != self.param_types.len() {
            return Err(Error::ParamCountMismatch {
                expected: self.param_types.len(),
                supplied: params.len(),
            });
        }
        self.resolved_params.clear();
        self.missed_params.clear();
        for (idx, value) in params.iter().enumerate() {
            let (resolved, missed) = bind_param(txn, idx, value, &self.param_types[idx])?;
            self.resolved_params.push(resolved);
            self.missed_params.push(missed);
        }
        Ok(())
    }
}

/// Resolves every occurrence's symbolic predicate constants for this
/// execution — residual filters into `out_filters`, selection words into
/// `out_selections`. `Ok(false)` = a dictionary miss under an `Eq`
/// predicate (filter or selection), which empties the whole conjunctive
/// query (the short-circuit is sound for `Eq` only — a missed value
/// under `Ne` resolves to the sentinel id and matches everything).
pub(super) fn resolve_predicates(
    txn: &ReadTxn<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out_filters: &mut [Vec<FilterPredicate>],
    out_selections: &mut [Vec<u64>],
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        out_filters[occ_idx].clear();
        for filter in &occurrence.filters {
            let Some(resolved) = resolve_filter(txn, filter, params, missed)? else {
                return Ok(false);
            };
            out_filters[occ_idx].push(resolved);
        }
        out_selections[occ_idx].clear();
        for selection in &occurrence.selections {
            let Some(word) = resolve_selection(txn, selection, params, missed)? else {
                return Ok(false);
            };
            out_selections[occ_idx].push(word);
        }
    }
    Ok(true)
}

/// Resolves one selection's constant to the column word its trie level
/// probes with. `Ok(None)` = a dictionary miss — the Eq short-circuit.
fn resolve_selection(
    txn: &ReadTxn<'_>,
    selection: &crate::plan::fj::Selection,
    params: &[Const],
    missed: &[bool],
) -> Result<Option<u64>> {
    let word_of = |constant: &Const| match constant {
        Const::Word(w) => *w,
        Const::Byte(b) => u64::from(*b),
        Const::Param(_) | Const::PendingIntern { .. } => {
            unreachable!("bind_param resolves params to column form")
        }
    };
    Ok(match &selection.value {
        Const::Word(w) => Some(*w),
        Const::Byte(b) => Some(u64::from(*b)),
        Const::Param(p) => {
            if missed[usize::from(p.0)] {
                None
            } else {
                Some(word_of(&params[usize::from(p.0)]))
            }
        }
        Const::PendingIntern { tag, bytes } => dict::lookup_tagged(txn, *tag, bytes)?,
    })
}

/// Converts a bound param value to column form. A String or Bytes value
/// that was never interned resolves to the sentinel intern id, flagged
/// `missed` so `Eq` uses can short-circuit to the empty result.
fn bind_param(
    txn: &ReadTxn<'_>,
    index: usize,
    value: &Value,
    expected: &ValueType,
) -> Result<(Const, bool)> {
    // The shared compatibility check (kind, enum range, UTF-8) — one rule
    // with validation and the dynamic write path.
    if crate::ir::value_matches(value, expected).is_err() {
        return Err(Error::ParamTypeMismatch {
            param: ParamId(u16::try_from(index).expect("param ids fit u16")),
            expected: expected.clone(),
        });
    }
    let resolved = match value {
        Value::Bool(v) => Const::Byte(u8::from(*v)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(u64::from_be_bytes(crate::encoding::encode_i64(*v))),
        Value::String(bytes) => {
            let text = std::str::from_utf8(bytes).expect("value_matches validated UTF-8");
            match dict::lookup_str(txn, text)? {
                Some(id) => Const::Word(id),
                None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
            }
        }
        Value::Bytes(bytes) => match dict::lookup_bytes(txn, bytes)? {
            Some(id) => Const::Word(id),
            None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
        },
    };
    Ok((resolved, false))
}

/// Substitutes symbolic constants into an executable filter. `Ok(None)` =
/// a dictionary miss under `Eq` (the whole-query empty short-circuit); a
/// miss under any other operator resolves to the sentinel intern id, whose
/// word comparison yields the correct per-operator semantics (`Ne` matches
/// every stored value). The `Eq` arms here are unreachable through the
/// production pipeline — `split_filters` routes every Eq-constant into
/// selections — and stay as belt-and-braces for the same reason
/// `check_selections` exists: `PlanOccurrence` is plain data.
fn resolve_filter(
    txn: &ReadTxn<'_>,
    filter: &FilterPredicate,
    params: &[Const],
    missed: &[bool],
) -> Result<Option<FilterPredicate>> {
    let FilterPredicate::Compare { field, op, value } = filter else {
        return Ok(Some(filter.clone()));
    };
    let resolved = match value {
        Const::Word(_) | Const::Byte(_) => value.clone(),
        Const::Param(p) => {
            if missed[usize::from(p.0)] && *op == crate::ir::CmpOp::Eq {
                return Ok(None);
            }
            params[usize::from(p.0)].clone()
        }
        Const::PendingIntern { tag, bytes } => match dict::lookup_tagged(txn, *tag, bytes)? {
            Some(id) => Const::Word(id),
            None if *op == crate::ir::CmpOp::Eq => return Ok(None),
            None => Const::Word(dict::SENTINEL_ID),
        },
    };
    Ok(Some(FilterPredicate::Compare {
        field: *field,
        op: *op,
        value: resolved,
    }))
}
