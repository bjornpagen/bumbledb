use super::KeyProbePlan;
use super::fact_word::{FactOperand, fact_operand};
use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, Result};
use crate::image::view::{Const, Loaded, OperandAddr, Operands, holds};
use crate::obs;
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;
use crate::storage::dict;
use crate::storage::read;
use crate::storage::read::check_width;

fn const_bytes<Cat: CatalogRead>(
    catalog: &Cat,
    desc: bumbledb_theory::schema::ValueType,
    value: &Const,
    params: &[Const],
    out: &mut Vec<u8>,
) -> Result<()> {
    match value {
        Const::Word(w) => out.extend_from_slice(&w.to_be_bytes()),
        Const::Byte(b) => out.push(*b),

        Const::Words(words) => {
            for word in words {
                out.extend_from_slice(&word.to_be_bytes());
            }
        }
        Const::Interval { start, end } => {
            out.extend_from_slice(&start.to_be_bytes());
            if !matches!(
                desc,
                bumbledb_theory::schema::ValueType::FixedInterval { .. }
            ) {
                out.extend_from_slice(&end.to_be_bytes());
            }
        }
        Const::Param(p) => {
            return const_bytes(catalog, desc, &params[usize::from(p.0)], params, out);
        }
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a param-set binding never reaches the key-probe path")
        }
        Const::PendingIntern { bytes } => {
            let id = catalog
                .dict_lookup(bytes)?
                .map_or(dict::SENTINEL_ID, InternId::raw);
            out.extend_from_slice(&id.to_be_bytes());
        }
    }
    Ok(())
}

struct FactRow<'a, 'l, Cat: CatalogRead> {
    fact: crate::encoding::FactView<'a, 'l>,
    catalog: &'a Cat,
}

impl<Cat: CatalogRead> Operands for FactRow<'_, '_, Cat> {
    type Error = crate::error::Error;

    fn word(&self, at: OperandAddr) -> std::result::Result<u64, Self::Error> {
        match fact_operand(self.fact, at.field())? {
            FactOperand::Word(w) => Ok(w),
            FactOperand::Pair(..) | FactOperand::Block { .. } => {
                unreachable!("validated: word operands are scalar fields")
            }
        }
    }

    fn pair(&self, at: OperandAddr) -> std::result::Result<(u64, u64), Self::Error> {
        match fact_operand(self.fact, at.field())? {
            FactOperand::Pair(s, e) => Ok((s, e)),
            FactOperand::Word(_) | FactOperand::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        }
    }

    fn block(&self, at: OperandAddr) -> std::result::Result<([u64; 8], u8), Self::Error> {
        match fact_operand(self.fact, at.field())? {
            FactOperand::Block { words, count } => Ok((words, count)),
            FactOperand::Word(_) | FactOperand::Pair(..) => {
                unreachable!("validated: block operands are bytes<N>")
            }
        }
    }

    fn loaded(&self, at: OperandAddr) -> std::result::Result<Loaded, Self::Error> {
        Ok(match fact_operand(self.fact, at.field())? {
            FactOperand::Word(w) => Loaded::Word(w),
            FactOperand::Pair(s, e) => Loaded::Pair(s, e),
            FactOperand::Block { words, count } => Loaded::Block { words, count },
        })
    }

    fn intern(&self, bytes: &[u8]) -> std::result::Result<u64, Self::Error> {
        Ok(self
            .catalog
            .dict_lookup(bytes)?
            .map_or(dict::SENTINEL_ID, InternId::raw))
    }
}

fn fetch_checked<'c, Cat: CatalogRead>(
    catalog: &'c Cat,
    schema: &Schema,
    rel: bumbledb_theory::schema::RelationId,
    row_id: u64,
) -> Result<Cat::Value<'c>> {
    let stored = catalog.fetch_fact(rel, row_id)?.ok_or(Error::Corruption(
        CorruptionError::MissingFact {
            relation: rel,
            row_id,
        },
    ))?;
    check_width(schema, rel, row_id, stored.as_ref())?;
    Ok(stored)
}

/// # Errors
pub(crate) fn key_probe_fact<'c, Cat: CatalogRead>(
    plan: &KeyProbePlan,
    catalog: &'c Cat,
    schema: &Schema,
    params: &[Const],
    key_scratch: &mut Vec<u8>,
) -> Result<Option<Cat::Value<'c>>> {

    key_scratch.clear();
    if let super::KeyProbeKind::Uniqueness { statement, .. } = &plan.kind {
        read::begin_determinant_key(key_scratch, plan.relation, *statement);
    }
    let layout = schema.relation(plan.relation).layout();
    for (field, value) in plan.kind.key() {
        let desc = layout.field_type(usize::from(field.0));
        const_bytes(catalog, desc, value, params, key_scratch)?;
    }

    let mut probe_span = obs::span(obs::names::KEY_PROBE);

    // ruled 2026-07-23, R16): its determinant IS the row id, so the probe

    let stored = match &plan.kind {
        super::KeyProbeKind::Uniqueness { statement, .. }
            if let Some(crate::schema::StatementView::Key(_, key)) =
                schema.statement_checked(*statement)
                && key.form().as_fresh_row().is_some() =>
        {
            let row_id = match <[u8; 8]>::try_from(&key_scratch[read::DETERMINANT_KEY_HEADER..]) {
                Ok(word) => u64::from_be_bytes(word),
                Err(_) => unreachable!("KeyForm::FreshRow determinant is one encoded u64"),
            };
            match catalog.fetch_fact(plan.relation, row_id)? {
                Some(bytes) => {
                    check_width(schema, plan.relation, row_id, bytes.as_ref())?;
                    Some(bytes)
                }
                None => None,
            }
        }
        super::KeyProbeKind::Uniqueness { .. } => match catalog.determinant_row(key_scratch)? {
            Some(row_id) => Some(fetch_checked(catalog, schema, plan.relation, row_id)?),
            None => None,
        },
        super::KeyProbeKind::Membership { .. } => {
            let hash = crate::encoding::fact_hash(key_scratch);
            match catalog.membership_row(plan.relation, &hash)? {
                Some(row_id) => Some(fetch_checked(catalog, schema, plan.relation, row_id)?),
                None => None,
            }
        }
    };
    probe_span.set_flag(stored.is_some());
    let Some(stored) = stored else {
        return Ok(None); 
    };

    let fact = layout.encoded(stored.as_ref());
    let ops = FactRow { fact, catalog };
    for filter in &plan.remaining_filters {
        if !holds(filter, &ops, params)?.unwrap_or(false) {
            return Ok(None);
        }
    }
    Ok(Some(stored))
}
