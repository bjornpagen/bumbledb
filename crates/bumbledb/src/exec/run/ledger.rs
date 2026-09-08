//! Cooperative cancellation inside join recursion. Poll explored entries,
//! not just output rows, so selective joins that emit nothing still stop.
use super::{Colt, ExecLedger, Executor, Poison};
use crate::exec::sink::STEP_QUANTUM;
use crate::work::{WorkContext, WorkError};

impl Executor {
    /// Bind the operation once, before any COLT force/select or execution.
    /// Both prepared queries and direct executor callers pass the same COLTs
    /// they will execute. Pools remain owned by the COLTs; rebinding changes
    /// only the context used for subsequent work.
    pub(crate) fn begin_work(&mut self, work: &WorkContext, colts: &mut [Colt]) {
        for colt in colts {
            colt.bind(Some(work));
        }
        self.ledger = Some(ExecLedger {
            work: work.clone(),
            pending: 0,
        });
    }

    /// Flush sub-quantum explored work and release the execution ledger.
    /// Reusable pools stay with the COLTs that own the memory.
    pub(super) fn end_work(&mut self) {
        if let Some(ledger) = &mut self.ledger
            && ledger.pending > 0
            && let Err(error) = poll(ledger)
        {
            self.poison(Poison::Work(error));
        }
        self.ledger = None;
    }

    /// Note `yielded` explored cover entries; at the published quantum,
    /// poll cancellation. Returns `false` after poisoning the
    /// drive — callers unwind, and `execute` surfaces the typed error.
    #[inline]
    pub(super) fn note_explored(&mut self, yielded: usize) -> bool {
        let Some(ledger) = &mut self.ledger else {
            return true;
        };
        if let Err(error) = ledger.note_explored(yielded) {
            self.poison(Poison::Work(error));
            return false;
        }
        true
    }

    /// Surface a COLT force/growth refusal as typed Work poison.
    /// `Ok` values pass through — including `Ok((0, token))` drain-end
    /// and `Ok(None)` miss. `Err` is never rewritten as a miss.
    pub(super) fn colt_ok<T>(&mut self, result: Result<T, WorkError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.poison(Poison::Work(error));
                None
            }
        }
    }
}

impl ExecLedger {
    /// Poll at the same bounded exploration quantum from recursion or a
    /// borrowed fused scan; the caller retains ownership of error poison.
    #[inline]
    pub(super) fn note_explored(&mut self, yielded: usize) -> Result<(), WorkError> {
        self.pending = self
            .pending
            .saturating_add(u32::try_from(yielded).unwrap_or(u32::MAX));
        if self.pending < STEP_QUANTUM {
            return Ok(());
        }
        poll(self)
    }
}

#[cold]
fn poll(ledger: &mut ExecLedger) -> Result<(), WorkError> {
    ledger.pending = 0;
    ledger.work.checkpoint()
}
