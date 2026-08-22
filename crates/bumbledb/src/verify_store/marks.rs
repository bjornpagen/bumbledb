//! The marks pass: the global re-verification the capacity form's own
//! namespace cannot ride. **Closed-parent capacity statements** re-check
//! per sealed member axiom: those parents have no `F` rows to ride the
//! fact scan, so their roster walks here — dependent bounds resolve per
//! axiom row, each judged against its own resolved ceiling.
//! Ordinary-parent capacity statements ride the `F` pass (`facts.rs`),

use crate::encoding::encode_u64;
use crate::error::{Check, Error, Result};
use crate::schema::CapacityEnforcement;
use crate::storage::catalog::CatalogRead;
use crate::storage::commit::judgment;

use super::{StoreFinding, Sweep};

pub(super) fn sweep<C: CatalogRead + Copy>(
    s: &mut Sweep<'_, C>,
    checker: &mut judgment::Checker<'_, C>,
) -> Result<()> {
    let schema = s.schema;

    for (index, statement) in schema.capacities().iter().enumerate() {
        let CapacityEnforcement::Closed { .. } = &statement.enforcement else {
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

            let checks = s.selections.capacity(capacity_id);
            match checker.check_capacity(statement, checks, &parent) {
                Ok(Check::Holds) | Err(Error::Corruption(_)) => {}
                Ok(Check::Violated(violation)) => s.push(StoreFinding::Judgment(violation)),
                // A ray met at measure time (C10's judge-side refusal)
                Err(Error::CapacityRayMeasure { .. }) => {
                    s.malformed(&parent, "capacity measure of a ray");
                }
                Err(other) => return Err(other),
            }
        }
    }
    Ok(())
}
