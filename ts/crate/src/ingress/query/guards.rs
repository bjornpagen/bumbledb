//! Captured guard plans and predicates share whole-query worker admission.
use super::{PayoffAdmission, PredicateExpr};
use crate::{ingress::event_error, runtime::RuntimeError};
use bumbledb::event::SpaceId;
use bumbledb::{GuardOp, GuardPlanExpr, ObservationNumberCodecLimits, PredicateGuardPlan, VarId};

#[derive(Debug)]
pub(crate) enum GuardPlan {
    Captured {
        source: Vec<u8>,
        refinement: Option<SpaceId>,
    },
    Bound {
        source: VarId,
        refinement: Option<VarId>,
    },
}
#[derive(Debug)]
pub(crate) struct GuardExpr {
    pub(crate) plan: GuardPlan,
    pub(crate) predicate: PredicateExpr,
    pub(crate) companions: Vec<PredicateExpr>,
    pub(crate) sources: Vec<VarId>,
    pub(crate) operation: GuardOp,
}
impl PayoffAdmission<'_> {
    pub(super) fn guard(&mut self, value: GuardExpr) -> Result<bumbledb::GuardExpr, RuntimeError> {
        self.arithmetic
            .control()
            .checkpoint()
            .map_err(event_error)?;
        let plan = match value.plan {
            GuardPlan::Captured { source, refinement } => {
                self.number_bytes(&source)?;
                if refinement.is_some() {
                    self.number_bytes(&[0; 32])?;
                }
                GuardPlanExpr::Captured(
                    PredicateGuardPlan::from_parts(
                        &source,
                        refinement,
                        ObservationNumberCodecLimits::default(),
                        &mut self.arithmetic,
                    )
                    .map_err(|e| crate::db_wire::engine_error(&e))?,
                )
            }
            GuardPlan::Bound { source, refinement } => match refinement {
                None => GuardPlanExpr::Existing(source),
                Some(identity) => GuardPlanExpr::Refine { source, identity },
            },
        };
        let predicate = self.predicate(value.predicate, 2)?;
        let mut companions = crate::marshal::output_vec(value.companions.len())?;
        for companion in value.companions {
            companions.push(self.predicate(companion, 2)?);
        }
        Ok(bumbledb::GuardExpr {
            plan,
            predicate,
            companions,
            sources: value.sources,
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
            sources: Vec::new(),
            plan: GuardPlan::Captured {
                source,
                refinement: None,
            },
            companions: vec![PredicateExpr::Sign {
                number: NumberExpr::Literal(scalar.clone()),
                signs: PolynomialSigns::POSITIVE,
            }],
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
        bounded.remaining = bytes.len() + 2 * scalar.len();
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
