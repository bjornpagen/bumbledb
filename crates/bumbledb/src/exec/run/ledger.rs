//! The bounded-quantum work poll inside the join recursion (chapter 12
//! §7): explored cover entries charge work units, every poll checkpoints
//! cancellation/deadline, and COLT pool growth is reserved against
//! working bytes — so a selective join that emits nothing still stops,
//! and a tiny working budget bounds the resident join's actual memory
//! (the bounded-restart trigger fires from join growth).
use super::{Colt, ExecLedger, Executor, Poison};
use crate::exec::sink::STEP_QUANTUM;
use crate::work::{WorkContext, WorkError};

impl Executor {
    /// Bind the operation once, before any COLT force/select or execution.
    /// Both prepared queries and direct executor callers pass the same COLTs
    /// they will execute. Pool reservations retain their original ownership;
    /// rebinding changes only the context used for subsequent work.
    pub(crate) fn begin_work(&mut self, work: &WorkContext, colts: &mut [Colt]) {
        for colt in colts {
            colt.bind(Some(work));
        }
        self.ledger = Some(ExecLedger {
            work: work.clone(),
            pending: 0,
        });
    }

    /// Flush any sub-quantum explored work, then drop the ledger and
    /// refund this execution's growth reservations (the pools stay resident
    /// with the prepared query; the charges are per-execution live-byte
    /// accounting, like the image build's transient slab charge).
    pub(super) fn end_work(&mut self, colts: &[Colt]) {
        if let Some(ledger) = &mut self.ledger
            && ledger.pending > 0
            && let Err(error) = poll(ledger)
        {
            self.poison(Poison::Work(error));
        }
        for colt in colts {
            // Reusable pool charges stay on the COLT; dropping the
            // execution ledger must not refund retained capacity.
            let _ = colt.charged_bytes();
        }
        self.ledger = None;
    }

    /// Note `yielded` explored cover entries; at the published quantum,
    /// poll the ledger (deadline/cancellation via the step charge) and
    /// charge COLT pool growth. Returns `false` after poisoning the
    /// drive — callers unwind, and `execute` surfaces the typed error.
    #[inline]
    pub(super) fn note_explored(&mut self, yielded: usize, _colts: &[Colt]) -> bool {
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
    /// Charge the same bounded exploration quantum from recursion or a
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
    let pending = u64::from(std::mem::replace(&mut ledger.pending, 0));
    ledger.work.step(pending)
}
