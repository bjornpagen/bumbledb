//! All numerical imports in a query share one worker admission budget.
use super::{Admit, FindTerm, Interior, Payoff, Query, Rule};
use crate::ingress::{MAX_EVENT_BYTES, event_error};
use crate::marshal::output_vec;
use crate::runtime::RuntimeError;
use bumbledb::event::{ArithmeticLimits, Capacity, Error, ExactArithmetic, SourceDescriptorLimits};
use bumbledb::work::WorkContext;

pub(super) struct PayoffAdmission<'a> {
    arithmetic: ExactArithmetic<'a>,
    remaining: usize,
}
impl<'a> PayoffAdmission<'a> {
    pub(super) fn new(work: &'a WorkContext) -> Self {
        Self {
            arithmetic: ExactArithmetic::new(ArithmeticLimits::default(), work),
            remaining: MAX_EVENT_BYTES,
        }
    }
    pub(super) fn admit(&mut self, input: Payoff) -> Result<bumbledb::PayoffExpr, RuntimeError> {
        use bumbledb::PayoffExpr as O;
        Ok(match input {
            Payoff::Integer(v) => O::Integer(v),
            Payoff::Ratio {
                numerator,
                denominator,
            } => O::Ratio {
                numerator,
                denominator,
            },
            Payoff::Imported(bytes) => {
                self.remaining = self
                    .remaining
                    .checked_sub(bytes.len())
                    .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
                O::Imported(
                    bumbledb::PayoffImport::from_bytes(
                        &bytes,
                        SourceDescriptorLimits::default(),
                        &mut self.arithmetic,
                    )
                    .map_err(event_error)?,
                )
            }
        })
    }
    fn rule(&mut self, input: Rule, work: &WorkContext) -> Result<bumbledb::Rule, RuntimeError> {
        let mut finds = output_vec(input.finds.len())?;
        for find in input.finds {
            work.checkpoint()?;
            finds.push(match find {
                FindTerm::Expectation { value, when, given } => bumbledb::FindTerm::Expectation {
                    value: self.admit(value)?,
                    when,
                    given,
                },
                other => other.admit(work)?,
            });
        }
        Ok(bumbledb::Rule {
            finds,
            atoms: input.atoms.admit(work)?,
            negated: input.negated.admit(work)?,
            conditions: input.conditions.admit(work)?,
        })
    }
    fn rules(
        &mut self,
        input: Vec<Rule>,
        work: &WorkContext,
    ) -> Result<Vec<bumbledb::Rule>, RuntimeError> {
        let mut result = output_vec(input.len())?;
        for rule in input {
            result.push(self.rule(rule, work)?);
        }
        Ok(result)
    }
}
impl Admit for Query {
    type Output = bumbledb::Query;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        let mut budget = PayoffAdmission::new(work);
        let mut interiors = output_vec(self.interiors.len())?;
        for Interior { rules } in self.interiors {
            interiors.push(bumbledb::Interior {
                rules: budget.rules(rules, work)?,
            });
        }
        Ok(bumbledb::Query {
            interiors,
            head: self.head,
            rules: budget.rules(self.rules, work)?,
            rec: self.rec.admit(work)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::ExactRational;
    use bumbledb::{HeadTerm, VarId};

    #[test]
    fn payoff_admission_is_cumulative_and_rejects_late_imports_on_empty_rules() {
        let work = WorkContext::new();
        let bytes = ExactRational::from(7u64)
            .to_bytes(&mut ExactArithmetic::new(
                ArithmeticLimits::default(),
                &work,
            ))
            .unwrap();
        let mut first = PayoffAdmission::new(&work);
        first.admit(Payoff::Imported(bytes.clone())).unwrap();
        let mut bounded = PayoffAdmission::new(&work);
        bounded.arithmetic = ExactArithmetic::new(
            ArithmeticLimits {
                operations: first.arithmetic.operations(),
                ..ArithmeticLimits::default()
            },
            &work,
        );
        bounded.admit(Payoff::Imported(bytes.clone())).unwrap();
        assert!(bounded.admit(Payoff::Imported(bytes.clone())).is_err());
        let mut bounded = PayoffAdmission::new(&work);
        bounded.remaining = bytes.len();
        bounded.admit(Payoff::Imported(bytes.clone())).unwrap();
        assert!(bounded.admit(Payoff::Imported(bytes.clone())).is_err());
        let rule = |bytes| Rule {
            finds: vec![FindTerm::Expectation {
                value: Payoff::Imported(bytes),
                when: VarId(0),
                given: VarId(1),
            }],
            atoms: vec![],
            negated: vec![],
            conditions: vec![],
        };
        let input = Query {
            interiors: vec![],
            head: vec![HeadTerm::Var],
            rules: vec![rule(bytes), rule(b"BERA\x01".to_vec())],
            rec: None,
        };
        assert!(input.admit(&work).is_err());
        work.cancel();
        assert!(
            PayoffAdmission::new(&work)
                .admit(Payoff::Imported(b"BERA\x01".to_vec()))
                .is_err()
        );
    }
}
