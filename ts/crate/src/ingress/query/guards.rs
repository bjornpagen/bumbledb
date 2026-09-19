//! Captured guard plans and predicates share whole-query worker admission.
use super::{PayoffAdmission, PredicateExpr};
use crate::{ingress::event_error, runtime::RuntimeError};
use bumbledb::event::SpaceId;
use bumbledb::{GuardOp, ObservationNumberCodecLimits, PredicateGuardPlan};

#[derive(Debug)]
pub(crate) struct GuardExpr {
    pub(crate) source: Vec<u8>,
    pub(crate) refinement: Option<SpaceId>,
    pub(crate) predicate: PredicateExpr,
    pub(crate) operation: GuardOp,
}
impl PayoffAdmission<'_> {
    pub(super) fn guard(&mut self, value: GuardExpr) -> Result<bumbledb::GuardExpr, RuntimeError> {
        self.arithmetic
            .control()
            .checkpoint()
            .map_err(event_error)?;
        self.number_bytes(&value.source)?;
        if value.refinement.is_some() {
            self.number_bytes(&[0; 32])?;
        }
        let plan = PredicateGuardPlan::from_parts(
            &value.source,
            value.refinement,
            ObservationNumberCodecLimits::default(),
            &mut self.arithmetic,
        )
        .map_err(|e| crate::db_wire::engine_error(&e))?;
        Ok(bumbledb::GuardExpr {
            plan,
            predicate: self.predicate(value.predicate, 2)?,
            operation: value.operation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Admit, FindTerm, NumberExpr, Payoff, Query, Rule};
    use super::*;
    use bumbledb::event::{
        ArithmeticLimits, ExactArithmetic, ExactRational, PolynomialSigns, Space,
    };
    use bumbledb::{HeadTerm, WorkContext};

    #[test]
    fn guard_source_and_predicate_share_worker_budgets_and_admit_unreachable_heads() {
        let work = WorkContext::new();
        let source = Space::new(SpaceId([225; 32]), 1, &work).unwrap();
        let bytes = source.full().to_bytes(&work).unwrap();
        let scalar = ExactRational::one()
            .to_bytes(&mut ExactArithmetic::new(
                ArithmeticLimits::default(),
                &work,
            ))
            .unwrap();
        let input = |source| GuardExpr {
            source,
            refinement: None,
            operation: GuardOp::Holds,
            predicate: PredicateExpr::Sign {
                number: NumberExpr::Literal(scalar.clone()),
                signs: PolynomialSigns::POSITIVE,
            },
        };
        let mut first = PayoffAdmission::new(&work);
        first.guard(input(bytes.clone())).unwrap();
        let steps = first.arithmetic.operations();
        assert!(steps > 0);
        let mut bounded = PayoffAdmission::new(&work);
        bounded.arithmetic = ExactArithmetic::new(
            ArithmeticLimits {
                operations: steps,
                ..ArithmeticLimits::default()
            },
            &work,
        );
        bounded.guard(input(bytes.clone())).unwrap();
        assert!(bounded.admit(Payoff::Imported(scalar.clone())).is_err());
        let mut bounded = PayoffAdmission::new(&work);
        bounded.remaining = bytes.len() + scalar.len();
        bounded.guard(input(bytes.clone())).unwrap();
        assert!(bounded.guard(input(bytes.clone())).is_err());
        let rule = |guard| Rule {
            finds: vec![FindTerm::Guard(guard)],
            atoms: vec![],
            negated: vec![],
            conditions: vec![],
        };
        for invalid in [
            b"BEVT\x01".to_vec(),
            source
                .coordinate(0, &work)
                .unwrap()
                .to_bytes(&work)
                .unwrap(),
        ] {
            assert!(
                Query {
                    interiors: vec![],
                    head: vec![HeadTerm::Var],
                    rules: vec![rule(input(bytes.clone())), rule(input(invalid))],
                    rec: None
                }
                .admit(&work)
                .is_err()
            );
        }
        work.cancel();
        assert!(PayoffAdmission::new(&work).guard(input(bytes)).is_err());
    }
}
