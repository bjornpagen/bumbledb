//! Owned truth programs. Numeric leaves and predicate imports share the whole
//! query's existing exact admission counter, including unreachable producers.
use super::{NumberExpr, PayoffAdmission};
use crate::{ingress::event_error, runtime::RuntimeError};
use bumbledb::event::{BoolOp4, Capacity, Error, ParameterCodecLimits, PolynomialSigns};
use bumbledb::{ObservationNumberCodecLimits, PredicateExpr as O, VarId};

#[derive(Debug)]
pub(crate) enum PredicateExpr {
    Var(VarId),
    Sign {
        number: NumberExpr,
        signs: PolynomialSigns,
    },
    Imported(Vec<u8>),
    Negate(Box<Self>),
    Binary {
        op: BoolOp4,
        left: Box<Self>,
        right: Box<Self>,
    },
    OnDomain {
        value: Box<Self>,
        domain: Vec<u8>,
    },
}
impl PayoffAdmission<'_> {
    pub(super) fn predicate(
        &mut self,
        value: PredicateExpr,
        depth: usize,
    ) -> Result<O, RuntimeError> {
        self.arithmetic
            .control()
            .checkpoint()
            .map_err(event_error)?;
        if depth > 128 {
            return Err(event_error(Error::Capacity(Capacity::ProgramNodes)));
        }
        Ok(match value {
            PredicateExpr::Var(v) => O::Var(v),
            PredicateExpr::Sign { number, signs } => O::Sign {
                number: self.number(number, depth + 1)?,
                signs,
            },
            PredicateExpr::Imported(bytes) => {
                self.number_bytes(&bytes)?;
                O::Imported(
                    bumbledb::ObservationPredicateImport::from_bytes(
                        &bytes,
                        ObservationNumberCodecLimits::default(),
                        &mut self.arithmetic,
                    )
                    .map_err(|e| crate::db_wire::engine_error(&e))?,
                )
            }
            PredicateExpr::Negate(value) => O::Negate(Box::new(self.predicate(*value, depth + 1)?)),
            PredicateExpr::Binary { op, left, right } => O::Binary {
                op,
                left: Box::new(self.predicate(*left, depth + 1)?),
                right: Box::new(self.predicate(*right, depth + 1)?),
            },
            PredicateExpr::OnDomain { value, domain } => {
                self.number_bytes(&domain)?;
                O::OnDomain {
                    value: Box::new(self.predicate(*value, depth + 1)?),
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
    use bumbledb::event::{ArithmeticLimits, ExactArithmetic, ExactRational};
    use bumbledb::{
        HeadTerm, ObservationNumber, ObservationNumberLimits, ObservationPredicateImport,
        PredicateQuantifier, WorkContext,
    };

    #[test]
    fn predicates_share_numerical_and_payoff_admission_and_check_unreachable_imports() {
        let work = WorkContext::new();
        let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &work);
        let limits = ObservationNumberLimits::default();
        let number =
            ObservationNumber::literal(ExactRational::one(), limits, &mut arithmetic).unwrap();
        let predicate = number
            .where_sign(PolynomialSigns::POSITIVE, limits, &mut arithmetic)
            .unwrap();
        let imported = ObservationPredicateImport::capture(
            &predicate,
            ObservationNumberCodecLimits::default(),
            &mut arithmetic,
        )
        .unwrap();
        let scalar = ExactRational::one().to_bytes(&mut arithmetic).unwrap();
        let input = || PredicateExpr::Imported(imported.bytes().to_vec());
        let mut reference = PayoffAdmission::new(&work);
        reference.predicate(input(), 1).unwrap();
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
        bounded.predicate(input(), 1).unwrap();
        assert!(
            bounded
                .number(NumberExpr::Literal(scalar.clone()), 1)
                .is_err()
        );
        assert!(bounded.admit(Payoff::Imported(scalar)).is_err());
        let mut bounded = PayoffAdmission::new(&work);
        bounded.remaining = imported.bytes().len();
        bounded.predicate(input(), 1).unwrap();
        assert!(bounded.predicate(input(), 1).is_err());
        for find in [
            FindTerm::Predicate(PredicateExpr::Imported(b"BENP\x01".to_vec())),
            FindTerm::PredicateTest {
                predicate: PredicateExpr::Imported(b"BENP\x01".to_vec()),
                quantifier: PredicateQuantifier::Always,
            },
        ] {
            let rule = |find| Rule {
                finds: vec![find],
                atoms: vec![],
                negated: vec![],
                conditions: vec![],
            };
            let query = Query {
                interiors: vec![],
                head: vec![HeadTerm::Var],
                rules: vec![rule(FindTerm::Predicate(input())), rule(find)],
                rec: None,
            };
            assert!(query.admit(&work).is_err());
        }
        work.cancel();
        assert!(PayoffAdmission::new(&work).predicate(input(), 1).is_err());
    }
}
