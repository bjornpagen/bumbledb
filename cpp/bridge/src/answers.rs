//! The owned answers carrier (`TODO_CPP.md` §22–§23): execution crosses the
//! engine's flat `Answers` buffer WHOLE behind one opaque handle; C++
//! decodes cell by cell through bounds-checked accessors — never one FFI
//! call per cell on the engine side, never a panic from an index bug
//! (`Answers::get` panics on out-of-range; the bounds are checked HERE).
//!
//! The carrier is caller-owned and reusable (the engine's own warm-path
//! allocation contract): `bdb_answers_new` once, `bdb_snapshot_execute`
//! into it repeatedly (each execution clears it first, retaining
//! capacity), `bdb_answers_clear` for an explicit reset,
//! `bdb_answers_destroy` when done. Cell views borrow the carrier and are
//! valid only while it is alive and un-re-executed.

use bumbledb::Answers;

use crate::db::bdb_snapshot_ref;
use crate::error::{bdb_error, fail_engine};
use crate::query::bdb_prepared;
use crate::value::{answer_out, bdb_param, bdb_value, param_args, params_in};
use crate::{Fail, bdb_status, box_in, box_out, guard, mut_in, out, ref_in};

/// The opaque, reusable answers carrier.
pub struct bdb_answers {
    answers: Answers,
}

/// Mints an empty answers carrier (never fails; owns nothing yet).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_new() -> *mut bdb_answers {
    box_out(bdb_answers {
        answers: Answers::new(),
    })
}

/// Empties the carrier, retaining capacity (the zero-alloc reuse path).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_clear(answers: *mut bdb_answers) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        mut_in(answers)?.answers.clear();
        Ok(bdb_status::Ok)
    })
}

/// Number of answers (0 for a null handle).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_len(answers: *const bdb_answers) -> usize {
    match ref_in(answers) {
        Ok(answers) => answers.answers.len(),
        Err(_) => 0,
    }
}

/// Number of columns — the executed query's find terms, in order (0 for
/// a null handle).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_arity(answers: *const bdb_answers) -> usize {
    match ref_in(answers) {
        Ok(answers) => answers.answers.arity(),
        Err(_) => 0,
    }
}

/// One answer cell, viewed — string/bytes payloads BORROW the carrier
/// and are valid only while it is alive, un-cleared, and un-re-executed.
/// Bounds-checked bridge-side: `BDB_STATUS_MISUSE` out of range, never a
/// panic.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_get(
    answers: *const bdb_answers,
    row: usize,
    column: usize,
    out_value: *mut bdb_value,
) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        let answers = &ref_in(answers)?.answers;
        // The bridge-side bounds check (§22): the engine's `get` asserts.
        if row >= answers.len() || column >= answers.arity() {
            return Err(Fail::Misuse);
        }
        out(out_value, answer_out(answers.get(row, column)))?;
        Ok(bdb_status::Ok)
    })
}

/// Frees the carrier (invalidating every view borrowed from it).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_answers_destroy(answers: *mut bdb_answers) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(answers)?);
        Ok(bdb_status::Ok)
    })
}

/// Executes a prepared query against the snapshot with positional
/// params, filling the caller's reusable carrier (cleared first,
/// capacity retained — the `execute_into` lane, §23). The prepared handle
/// is taken exclusively for the call (`&mut` on the engine side — one
/// execution at a time, §20/§22); executing a prepared query against a
/// snapshot of a different database is the engine's own typed
/// `BDB_ERROR_FOREIGN_PREPARED`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_snapshot_execute(
    snapshot: *const bdb_snapshot_ref,
    prepared: *mut bdb_prepared,
    params: *const bdb_param,
    param_count: usize,
    answers: *mut bdb_answers,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let snap = ref_in(snapshot)?.snapshot()?;
        let prepared = mut_in(prepared)?;
        let owned = params_in(params, param_count)?;
        let args = param_args(&owned)?;
        let carrier = mut_in(answers)?;
        carrier.answers.clear();
        snap.execute_args(&mut prepared.prepared, &args, &mut carrier.answers)
            .map_err(|error| fail_engine(error, None))?;
        Ok(bdb_status::Ok)
    })
}
