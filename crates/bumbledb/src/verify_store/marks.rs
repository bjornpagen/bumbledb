//! The marks pass: the global re-verification the capacity form's own
//! namespace cannot ride. **Closed-parent capacity statements** re-check
//! per sealed member axiom: those parents have no `F` rows to ride the
//! fact scan, so their roster walks here (the domain-quantification move,
//! `docs/architecture/30-dependencies.md`) — dependent bounds resolve per
//! axiom row, each judged against its own resolved ceiling.
//! Ordinary-parent capacity statements ride the `F` pass (`facts.rs`),
//! one scan shared across every statement.

use crate::encoding::encode_u64;
use crate::error::{Error, Result, Violation};
use crate::schema::Enforcement;
use crate::storage::commit::judgment;

use super::{StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let schema = s.schema;
    let mut checker = judgment::Checker::new(s.txn.raw(), s.data, schema);

    // Every closed-parent capacity statement, every ψ-selected axiom: the
    // axiom's id encoding is the parent tuple, and the commit path's own
    // capacity check measures its child group.
    for (index, statement) in schema.capacities().iter().enumerate() {
        let Enforcement::Closed { .. } = &statement.enforcement else {
            continue;
        };
        let capacity_id =
            crate::schema::CapacityId(u16::try_from(index).expect("statement count fits u16"));
        let rows = schema
            .relation(statement.target.relation)
            .body()
            .closed_rows()
            .expect("the Closed enforcement arm resolves only against a closed target");
        for row_index in 0..rows.len() {
            let parent = encode_u64(u64::try_from(row_index).expect("row index fits u64"));
            // Fetched per row so the borrow of `s.selections` ends before
            // the finding push.
            let checks = s.selections.capacity(capacity_id);
            match checker.check_capacity(statement, checks, &parent) {
                Err(Error::CommitRejected { violations }) => {
                    for violation in violations {
                        let Violation::Capacity {
                            statement,
                            fact,
                            measure,
                        } = violation
                        else {
                            unreachable!("the capacity check cites capacity statements only");
                        };
                        s.push(StoreFinding::CapacityViolation {
                            statement,
                            fact,
                            measure,
                        });
                    }
                }
                // A ray met at measure time (C10's judge-side refusal)
                // is CONTENT under the sweeper's discipline: report,
                // never error.
                Err(Error::CapacityRayMeasure { .. }) => {
                    s.malformed(&parent, "capacity measure of a ray");
                }
                // A corruption inside the probe is a namespace desync
                // another pass convicts on its own.
                Ok(()) | Err(Error::Corruption(_)) => {}
                Err(other) => return Err(other),
            }
        }
    }
    Ok(())
}
