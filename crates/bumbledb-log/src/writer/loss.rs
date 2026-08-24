//! The one loss path: count the race, surface deposition, then carry
//! the pending through the disposable law.

use std::sync::atomic::Ordering;

use bumbledb::{Value, Violations};

use crate::braids::BraidId;

use super::{
    AckMode, ContentionCause, Core, Deposition, Inner, ObjectStore, Result, StepHook, Theory,
};

/// The loss ledger of one discipline run: how many races were lost, and
/// the slot the last one was lost at.
#[derive(Default)]
pub(crate) struct Live {
    pub(crate) losses: u32,
    pub(crate) tip: u64,
}

impl<T, S, H> Inner<T, S, H>
where
    T: Theory + Clone + Send + Sync + 'static,
    S: ObjectStore + 'static,
    H: StepHook + 'static,
{
    /// The one loss path: count the loss, surface the deposition signal
    /// where local acks made the writer resident, then carry the
    /// pending bytes through the disposable law — discard the
    /// directory, re-open through the replica to the current tip, and
    /// re-persist the carried pending before any re-judgment, so
    /// recovery stays crash-idempotent at every prefix.
    pub(crate) fn lose(
        &self,
        core: &mut Core<T>,
        braid: BraidId,
        slot: u64,
        winner_bytes: &[u8],
        live: &mut Live,
    ) -> Result<()> {
        live.losses += 1;
        live.tip = slot;
        self.losses.fetch_add(1, Ordering::Relaxed);
        if core.ack == AckMode::Local
            && core.deposition.is_none()
            && let Ok(winner) = self.codec.decode(winner_bytes)
        {
            core.deposition = Some(Deposition {
                braid,
                slot,
                resident: self.writer_id,
                usurper: winner.header.writer,
            });
            core.ack = AckMode::Published;
        }
        let carried = core.chain.pending.clone();
        self.re_establish(core, carried)
    }

    /// Maps the terminal re-judgment's rejection onto the contention
    /// payload: the violation names its statement, and the cited fact
    /// carries the offending raw values — engine-produced, never
    /// reconstructed.
    pub(crate) fn hot_key(&self, violations: &Violations) -> ContentionCause {
        let violation = violations
            .get(0)
            .expect("a rejection carries at least one violation");
        let statement = violation.statement_id(&self.schema);
        let values: Box<[Value]> = violations
            .cited_facts(0)
            .first()
            .map(|fact| Box::from(fact.values()))
            .unwrap_or_default();
        ContentionCause::HotKey { statement, values }
    }
}
