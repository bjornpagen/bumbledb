//! Owned numerical wire expressions. Admission shares the entire query's
//! exact work/byte allowance with payoff imports, including unreachable arms.
use super::PayoffAdmission;
use crate::ingress::event_error;
use crate::runtime::RuntimeError;
use bumbledb::event::{Capacity, Error, ExactRational, NumberOp, ParameterCodecLimits};
use bumbledb::{NumberExpr as O, ObservationComponent, ObservationNumberCodecLimits, VarId};

#[derive(Debug)]
pub(crate) enum NumberExpr {
    Var(VarId),
    Integer(VarId),
    Component {
        observation: VarId,
        component: ObservationComponent,
    },
    Literal(Vec<u8>),
    Imported(Vec<u8>),
    Binary {
        op: NumberOp,
        left: Box<Self>,
        right: Box<Self>,
    },
    Negate(Box<Self>),
    Abs(Box<Self>),
    Pow {
        value: Box<Self>,
        exponent: u32,
    },
    OnDomain {
        value: Box<Self>,
        domain: Vec<u8>,
    },
}
impl PayoffAdmission<'_> {
    fn number_bytes(&mut self, bytes: &[u8]) -> Result<(), RuntimeError> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
        Ok(())
    }
    pub(super) fn number(&mut self, value: NumberExpr, depth: usize) -> Result<O, RuntimeError> {
        self.arithmetic
            .control()
            .checkpoint()
            .map_err(event_error)?;
        if depth > 128 {
            return Err(event_error(Error::Capacity(Capacity::ProgramNodes)));
        }
        Ok(match value {
            NumberExpr::Var(v) => O::Var(v),
            NumberExpr::Integer(v) => O::Integer(v),
            NumberExpr::Component {
                observation,
                component,
            } => O::Component {
                observation,
                component,
            },
            NumberExpr::Literal(bytes) => {
                self.number_bytes(&bytes)?;
                O::Literal(
                    ExactRational::from_bytes(&bytes, &mut self.arithmetic).map_err(event_error)?,
                )
            }
            NumberExpr::Imported(bytes) => {
                self.number_bytes(&bytes)?;
                O::Imported(
                    bumbledb::ObservationNumberImport::from_bytes(
                        &bytes,
                        ObservationNumberCodecLimits::default(),
                        &mut self.arithmetic,
                    )
                    .map_err(|e| crate::db_wire::engine_error(&e))?,
                )
            }
            NumberExpr::Binary { op, left, right } => O::Binary {
                op,
                left: Box::new(self.number(*left, depth + 1)?),
                right: Box::new(self.number(*right, depth + 1)?),
            },
            NumberExpr::Negate(value) => O::Negate(Box::new(self.number(*value, depth + 1)?)),
            NumberExpr::Abs(value) => O::Abs(Box::new(self.number(*value, depth + 1)?)),
            NumberExpr::Pow { value, exponent } => O::Pow {
                value: Box::new(self.number(*value, depth + 1)?),
                exponent,
            },
            NumberExpr::OnDomain { value, domain } => {
                self.number_bytes(&domain)?;
                O::OnDomain {
                    value: Box::new(self.number(*value, depth + 1)?),
                    domain: bumbledb::NumberDomain::from_bytes(
                        &domain,
                        ParameterCodecLimits::default(),
                        &mut self.arithmetic,
                    )
                    .map_err(|e| crate::db_wire::engine_error(&e))?,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Admit, FindTerm, Payoff, Query, Rule};
    use super::*;
    use bumbledb::event::{ArithmeticLimits, ExactArithmetic};
    use bumbledb::work::WorkContext;
    use bumbledb::{HeadTerm, ObservationNumber, ObservationNumberImport, ObservationNumberLimits};

    #[test]
    fn all_numerical_imports_share_payoff_work_and_reject_unreachable_corruption() {
        let work = WorkContext::new();
        let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &work);
        let scalar = ExactRational::one().to_bytes(&mut arithmetic).unwrap();
        let value = ObservationNumber::literal(
            ExactRational::one(),
            ObservationNumberLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
        let value = ObservationNumberImport::capture(
            &value,
            ObservationNumberCodecLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
        let input = || NumberExpr::Imported(value.bytes().to_vec());
        let mut reference = PayoffAdmission::new(&work);
        reference.number(input(), 1).unwrap();
        let spent = reference.arithmetic.operations();
        assert!(spent > 0);
        let mut bounded = PayoffAdmission::new(&work);
        bounded.arithmetic = ExactArithmetic::new(
            ArithmeticLimits {
                operations: spent,
                ..ArithmeticLimits::default()
            },
            &work,
        );
        bounded.number(input(), 1).unwrap();
        assert!(bounded.admit(Payoff::Imported(scalar)).is_err());
        let mut bounded = PayoffAdmission::new(&work);
        bounded.remaining = value.bytes().len();
        bounded.number(input(), 1).unwrap();
        assert!(bounded.number(input(), 1).is_err());
        let rule = |number| Rule {
            finds: vec![FindTerm::Number(number)],
            atoms: vec![],
            negated: vec![],
            conditions: vec![],
        };
        let query = Query {
            interiors: vec![],
            head: vec![HeadTerm::Var],
            rules: vec![
                rule(input()),
                rule(NumberExpr::Imported(b"BENO\x01".to_vec())),
            ],
            rec: None,
        };
        assert!(query.admit(&work).is_err());
        work.cancel();
        assert!(PayoffAdmission::new(&work).number(input(), 1).is_err());
    }
}
