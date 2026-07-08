use super::{fact_word, GuardPlan};
use crate::encoding::encode_u64;
use crate::error::Result;
use crate::image::view::{Const, FilterPredicate};
use crate::obs;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::{dict, read};

/// Resolves a constant to its canonical guard-key bytes. A `PendingIntern`
/// that missed the dictionary resolves to the never-minted sentinel id —
/// the ensuing `U`/`M` probe then misses (empty result), never an insert,
/// never an error.
fn const_bytes(
    txn: &ReadTxn<'_>,
    value: &Const,
    params: &[Const],
    out: &mut Vec<u8>,
) -> Result<()> {
    match value {
        Const::Word(w) => out.extend_from_slice(&w.to_be_bytes()),
        Const::Byte(b) => out.push(*b),
        Const::Param(p) => {
            return const_bytes(txn, &params[usize::from(p.0)], params, out);
        }
        Const::PendingIntern { tag, bytes } => {
            let id = dict::lookup_tagged(txn, *tag, bytes)?.unwrap_or(dict::SENTINEL_ID);
            out.extend_from_slice(&encode_u64(id));
        }
    }
    Ok(())
}

/// The constant's column word (for filter checks on the fetched fact). A
/// dictionary miss resolves to the sentinel id, so `Eq` filters fail and
/// `Ne` filters pass — per-operator miss semantics with no special cases.
fn const_word(txn: &ReadTxn<'_>, value: &Const, params: &[Const]) -> Result<u64> {
    match value {
        Const::Word(w) => Ok(*w),
        Const::Byte(b) => Ok(u64::from(*b)),
        Const::Param(p) => const_word(txn, &params[usize::from(p.0)], params),
        Const::PendingIntern { tag, bytes } => {
            Ok(dict::lookup_tagged(txn, *tag, bytes)?.unwrap_or(dict::SENTINEL_ID))
        }
    }
}

/// The probe half of the guard: key from constants, one `U`/`M` get, one
/// `F` fetch, remaining filters on the fact bytes. `None` = miss or a
/// failed filter — an empty result, never an error.
///
/// # Errors
///
/// `Lmdb`/`Corruption` from the storage reads.
pub(crate) fn guard_probe_fact<'t>(
    plan: &GuardPlan,
    txn: &'t ReadTxn<'_>,
    schema: &Schema,
    params: &[Const],
    key_scratch: &mut Vec<u8>,
) -> Result<Option<&'t [u8]>> {
    // Build the guard key in the caller's reused scratch; a dictionary
    // miss lands the sentinel id in the key, and the probe below misses.
    key_scratch.clear();
    for (_, value) in &plan.key {
        const_bytes(txn, value, params, key_scratch)?;
    }

    let mut probe_span = obs::span(obs::names::GUARD_PROBE, obs::Category::Execute);
    let row_id = match plan.constraint {
        Some(constraint) => read::unique_row(txn, plan.relation, constraint, key_scratch)?,
        None => read::fact_row(txn, plan.relation, key_scratch)?,
    };
    probe_span.set_args(u64::from(row_id.is_some()), 0);
    let Some(row_id) = row_id else {
        return Ok(None); // miss: empty result
    };
    let fact = read::fetch(txn, schema, plan.relation, row_id)?;

    // Remaining filters run on the fact bytes.
    for filter in &plan.remaining_filters {
        let pass = match filter {
            FilterPredicate::Compare { field, op, value } => {
                let expected = const_word(txn, value, params)?;
                op.compare(&fact_word(schema, plan, fact, *field), &expected)
            }
            FilterPredicate::FieldsCompare { left, right, op } => op.compare(
                &fact_word(schema, plan, fact, *left),
                &fact_word(schema, plan, fact, *right),
            ),
        };
        if !pass {
            return Ok(None);
        }
    }
    Ok(Some(fact))
}
