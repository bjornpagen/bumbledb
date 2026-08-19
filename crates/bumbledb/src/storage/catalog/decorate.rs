//! Best-effort citation decoration through a still-live candidate dict.

use crate::error::{CitedFact, Result, Violation, Violations};
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;

/// Decodes cited facts through the candidate catalog's dictionary.
/// Secondary failure leaves the sealed set undecorated.
pub(crate) fn decorate_violations<C: CatalogRead>(
    violations: Violations,
    schema: &Schema,
    catalog: &C,
) -> Violations {
    match decode_cited(schema, catalog, &violations) {
        Ok(cited) => violations.attach_cited(cited),
        Err(_) => violations,
    }
}

fn decode_cited<C: CatalogRead>(
    schema: &Schema,
    catalog: &C,
    violations: &Violations,
) -> Result<Vec<Box<[CitedFact]>>> {
    let mut cited = Vec::with_capacity(violations.len());
    for violation in violations {
        let (relation, facts): (_, Vec<&[u8]>) = match violation {
            Violation::Functionality { .. } => {
                let crate::schema::StatementView::Key(_, key) =
                    schema.statement(violation.statement_id())
                else {
                    unreachable!("a Functionality citation names a key statement");
                };
                (
                    key.relation,
                    std::iter::once(violation.fact())
                        .chain(violation.incumbent())
                        .collect(),
                )
            }
            Violation::Containment { fact, .. } => {
                let crate::schema::StatementView::Containment(_, containment) =
                    schema.statement(violation.statement_id())
                else {
                    unreachable!("a Containment citation names a containment statement");
                };
                (containment.source.relation, vec![fact.as_ref()])
            }
            Violation::Capacity { fact, .. } => {
                let crate::schema::StatementView::Capacity(_, capacity) =
                    schema.statement(violation.statement_id())
                else {
                    unreachable!("a Capacity citation names a capacity statement");
                };
                (capacity.target.relation, vec![fact.as_ref()])
            }
        };
        let layout = schema.relation(relation).layout();
        let expected = layout.fact_width();
        let decoded = facts
            .into_iter()
            .map(|bytes| {
                if bytes.len() != expected {
                    return Err(crate::error::Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("cited fact width"),
                    ));
                }
                let values = crate::encoding::decode_values(layout.encoded(bytes), |id| {
                    let id = crate::encoding::InternId::from_raw(id);
                    let raw = catalog.dict_resolve(id)?;
                    std::str::from_utf8(raw.as_ref())
                        .map(Box::from)
                        .map_err(|_| {
                            crate::error::Error::Corruption(
                                crate::error::CorruptionError::NonUtf8Intern(id.raw()),
                            )
                        })
                })?;
                Ok(CitedFact::new(
                    relation,
                    layout.field_count(),
                    values.into_boxed_slice(),
                ))
            })
            .collect::<Result<Box<[CitedFact]>>>()?;
        cited.push(decoded);
    }
    Ok(cited)
}
