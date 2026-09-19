//! Execution-owned observation identities. Word tokens only name finalized
//! values in this registry; equality never compares a rounded numerical value.
use std::collections::BTreeMap;

use crate::event::ExactArithmetic;
use crate::image::intern::InternerHandle;
use crate::ir::validate::{ObservationKind, SignatureColumn};
use crate::{AnswerValue, ExpectationAnswer, ProbabilityAnswer, Result};

#[derive(Debug, Default)]
pub(super) struct ObservationRegistry {
    probabilities: Vec<ProbabilityAnswer>,
    expectations: Vec<ExpectationAnswer>,
    probability_keys: BTreeMap<[u64; 4], u64>,
    expectation_keys: BTreeMap<([u64; 2], Box<[u8]>), u64>,
}

impl ObservationRegistry {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn checkpoint(&self) -> (usize, usize) {
        (self.probabilities.len(), self.expectations.len())
    }
    pub(super) fn rollback(&mut self, (probabilities, expectations): (usize, usize)) {
        self.probabilities.truncate(probabilities);
        self.expectations.truncate(expectations);
        self.probability_keys
            .retain(|_, token| *token < probabilities as u64);
        self.expectation_keys
            .retain(|_, token| *token < expectations as u64);
    }

    pub(super) fn get(&self, kind: ObservationKind, token: u64) -> Result<AnswerValue<'_>> {
        let index = usize::try_from(token).map_err(|_| crate::event::Error::UnknownKey)?;
        match kind {
            ObservationKind::Probability => {
                self.probabilities.get(index).map(AnswerValue::Probability)
            }
            ObservationKind::Expectation => {
                self.expectations.get(index).map(AnswerValue::Expectation)
            }
        }
        .ok_or_else(|| crate::event::Error::UnknownKey.into())
    }

    pub(super) fn copy(&mut self, value: AnswerValue<'_>) -> Result<u64> {
        match value {
            AnswerValue::Probability(value) => self.insert_probability(value.clone()),
            AnswerValue::Expectation(value) => self.insert_expectation(value.clone()),
            _ => unreachable!("observation registry resolves only observations"),
        }
    }

    fn insert_probability(&mut self, value: ProbabilityAnswer) -> Result<u64> {
        let event = value.event().key().words();
        let given = value.given().key().words();
        let key = [event[0], event[1], given[0], given[1]];
        if let Some(token) = self.probability_keys.get(&key) {
            return Ok(*token);
        }
        self.probabilities
            .try_reserve(1)
            .map_err(crate::event::Error::from)?;
        let token = u64::try_from(self.probabilities.len())
            .map_err(|_| crate::Error::ResultBytesOverflow)?;
        self.probabilities.push(value);
        self.probability_keys.insert(key, token);
        Ok(token)
    }

    fn insert_expectation(&mut self, value: ExpectationAnswer) -> Result<u64> {
        let key = (value.given().key().words(), value.function_identity.clone());
        if let Some(token) = self.expectation_keys.get(&key) {
            return Ok(*token);
        }
        self.expectations
            .try_reserve(1)
            .map_err(crate::event::Error::from)?;
        let token = u64::try_from(self.expectations.len())
            .map_err(|_| crate::Error::ResultBytesOverflow)?;
        self.expectations.push(value);
        self.expectation_keys.insert(key, token);
        Ok(token)
    }

    fn probability(
        &mut self,
        pair: &[u64],
        interner: &InternerHandle<'_>,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<u64> {
        let key: [u64; 4] = pair.try_into().expect("probability pair width");
        if let Some(token) = self.probability_keys.get(&key) {
            return Ok(*token);
        }
        let event = interner.resolve_event([key[0], key[1]])?;
        let given = interner.resolve_event([key[2], key[3]])?;
        self.insert_probability(ProbabilityAnswer::new(event, given, work)?)
    }

    /// All input rosters participate in admission before any contraction.
    pub(super) fn expectations(
        &mut self,
        inputs: &[crate::observation::ExpectationInput],
        control: &crate::WorkContext,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u64>> {
        let admitted = inputs
            .iter()
            .map(|input| input.admit(control, work))
            .collect::<Result<Vec<_>>>()?;
        admitted
            .into_iter()
            .map(|input| self.insert_expectation(ExpectationAnswer::new(input, work)?))
            .collect()
    }

    /// Translate one producer row to the derived relation's physical layout.
    /// Finalized projections already contain a registry token; producers do not.
    pub(super) fn row(
        &mut self,
        columns: &[SignatureColumn],
        row: &[u64],
        expectations: &[u64],
        interner: &InternerHandle<'_>,
        work: &mut ExactArithmetic<'_>,
        out: &mut Vec<u64>,
    ) -> Result<()> {
        out.clear();
        let mut offset = 0;
        for column in columns {
            match column {
                SignatureColumn::Probability => {
                    out.push(self.probability(&row[offset..offset + 4], interner, work)?);
                    offset += 4;
                }
                SignatureColumn::Expectation => {
                    let index = usize::try_from(row[offset])
                        .map_err(|_| crate::event::Error::UnknownKey)?;
                    out.push(
                        *expectations
                            .get(index)
                            .ok_or(crate::event::Error::UnknownKey)?,
                    );
                    offset += 1;
                }
                SignatureColumn::ProjectObservation(kind) => {
                    self.get(*kind, row[offset])?;
                    out.push(row[offset]);
                    offset += 1;
                }
                _ => {
                    let width = column.binding_type().slot_width().slots();
                    out.extend_from_slice(&row[offset..offset + width]);
                    offset += width;
                }
            }
        }
        debug_assert_eq!(offset, row.len());
        Ok(())
    }
}
