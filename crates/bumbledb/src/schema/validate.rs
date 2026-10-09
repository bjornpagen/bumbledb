//! Sealing: [`bumbledb_theory::schema::check`] decides a declaration, and
//! its checked form becomes the [`Schema`] witness with typed statement
//! arenas and per-relation statement indexes.

use bumbledb_theory::schema::{
    CapacityResolution, Checked, CheckedRelation, CheckedStatement, Resolution, check,
};

use super::{
    CapacityEnforcement, CapacityId, CapacityStatement, ContainmentId, ContainmentStatement,
    Enforcement, KeyForm, KeyId, KeyStatement, Pairing, Relation, RelationBody, Schema,
    SchemaDescriptor, SealedRow, StatementId, StatementRef,
};
use crate::canonical::CanonicalRow;
use crate::error::SchemaError;

/// The admission boundary: a [`SchemaDescriptor`] (theory data) seals into
/// a [`Schema`].
pub trait ValidateDescriptor: Sized {
    /// # Errors
    /// The first [`SchemaError`] of [`check`].
    fn validate(self) -> Result<Schema, SchemaError>;
}

impl ValidateDescriptor for SchemaDescriptor {
    fn validate(self) -> Result<Schema, SchemaError> {
        Ok(seal(check(&self)?))
    }
}

fn arena_index(index: usize) -> u16 {
    u16::try_from(index).expect("statement count is checked")
}

fn seal(checked: Checked) -> Schema {
    let Checked {
        relations,
        statements,
    } = checked;
    let mut indexes = vec![RelationIndexes::default(); relations.len()];

    // Arena ids follow materialized order within each statement kind.
    let mut order = Vec::with_capacity(statements.len());
    let (mut keys, mut containments, mut capacities) = (0, 0, 0);
    for statement in &statements {
        order.push(match statement {
            CheckedStatement::Key { .. } => {
                keys += 1;
                StatementRef::Key(KeyId(arena_index(keys - 1)))
            }
            CheckedStatement::Containment { .. } => {
                containments += 1;
                StatementRef::Containment(ContainmentId(arena_index(containments - 1)))
            }
            CheckedStatement::Capacity { .. } => {
                capacities += 1;
                StatementRef::Capacity(CapacityId(arena_index(capacities - 1)))
            }
        });
    }
    let key_of = |statement: StatementId| match order[usize::from(statement.0)] {
        StatementRef::Key(key) => key,
        StatementRef::Containment(_) | StatementRef::Capacity(_) => {
            unreachable!("a resolution names a key statement")
        }
    };

    let mut sealed_keys = Vec::with_capacity(keys);
    let mut sealed_containments = Vec::with_capacity(containments);
    let mut sealed_capacities = Vec::with_capacity(capacities);
    let mut dependents: Vec<Vec<ContainmentId>> = vec![Vec::new(); keys];
    for (index, (statement, reference)) in statements.into_vec().into_iter().zip(&order).enumerate()
    {
        let id = StatementId(arena_index(index));
        match (statement, *reference) {
            (
                CheckedStatement::Key {
                    relation,
                    projection,
                    tail,
                },
                StatementRef::Key(key),
            ) => {
                indexes[relation.0 as usize].keys.push(key);
                sealed_keys.push(KeyStatement {
                    id,
                    relation,
                    projection,
                    form: tail.map_or(KeyForm::Scalar, |tail| KeyForm::Pointwise { tail }),
                });
            }
            (
                CheckedStatement::Containment {
                    source,
                    target,
                    resolution,
                    mirror,
                },
                StatementRef::Containment(containment),
            ) => {
                let enforcement = match resolution {
                    Resolution::ScalarProbe {
                        key,
                        key_projection,
                    } => Enforcement::ScalarProbe {
                        target_key: key_of(key),
                        key_projection,
                    },
                    Resolution::IntervalCoverage {
                        key,
                        key_projection,
                        source_tail,
                        target_tail,
                    } => Enforcement::IntervalCoverage {
                        target_key: key_of(key),
                        key_projection,
                        source_tail,
                        target_tail,
                    },
                    Resolution::Closed { members } => Enforcement::Closed { members },
                };
                if let Some(key) = enforcement.target_key() {
                    dependents[usize::from(key.0)].push(containment);
                }
                indexes[source.relation.0 as usize]
                    .outgoing
                    .push(containment);
                sealed_containments.push(ContainmentStatement {
                    id,
                    source,
                    target,
                    enforcement,
                    pairing: match mirror {
                        None => Pairing::OneWay,
                        Some(partner) => match order[usize::from(partner.0)] {
                            StatementRef::Containment(partner) => Pairing::Mirror(partner),
                            StatementRef::Key(_) | StatementRef::Capacity(_) => {
                                unreachable!("a mirror is a containment")
                            }
                        },
                    },
                });
            }
            (
                CheckedStatement::Capacity {
                    target,
                    weight,
                    lo,
                    hi,
                    source,
                    resolution,
                },
                StatementRef::Capacity(capacity),
            ) => {
                indexes[source.relation.0 as usize]
                    .capacity_sources
                    .push(capacity);
                indexes[target.relation.0 as usize]
                    .capacity_targets
                    .push(capacity);
                sealed_capacities.push(CapacityStatement {
                    id,
                    target,
                    weight,
                    lo,
                    hi,
                    source,
                    enforcement: match resolution {
                        CapacityResolution::ScalarProbe {
                            key,
                            key_projection,
                        } => CapacityEnforcement::ScalarProbe {
                            target_key: key_of(key),
                            key_projection,
                        },
                        CapacityResolution::Closed { members } => {
                            CapacityEnforcement::Closed { members }
                        }
                    },
                });
            }
            _ => unreachable!("order follows statement kinds"),
        }
    }

    let schema = Schema {
        identity: std::sync::OnceLock::new(),
        compiled: std::sync::OnceLock::new(),
        relations: relations
            .into_vec()
            .into_iter()
            .zip(indexes)
            .map(|(relation, indexes)| seal_relation(relation, indexes))
            .collect(),
        keys: sealed_keys.into_boxed_slice(),
        containments: sealed_containments.into_boxed_slice(),
        capacities: sealed_capacities.into_boxed_slice(),
        order: order.into_boxed_slice(),
        dependents: dependents.into_iter().map(Vec::into_boxed_slice).collect(),
    };
    // The identity is computed once, with the schema.
    let _ = super::fingerprint::fingerprint(&schema);
    schema
}

/// The statements that name one relation.
#[derive(Clone, Default)]
struct RelationIndexes {
    keys: Vec<KeyId>,
    outgoing: Vec<ContainmentId>,
    capacity_sources: Vec<CapacityId>,
    capacity_targets: Vec<CapacityId>,
}

fn seal_relation(relation: CheckedRelation, indexes: RelationIndexes) -> Relation {
    let CheckedRelation { name, fields, rows } = relation;
    let body = match rows {
        None => RelationBody::Ordinary,
        Some(rows) => RelationBody::Closed {
            extension: rows
                .into_vec()
                .into_iter()
                .map(|row| SealedRow {
                    row: CanonicalRow::encode(&fields, &row.values, &crate::WorkContext::new())
                        .expect("checked closed rows encode"),
                    handle: row.handle,
                    values: row.values,
                })
                .collect(),
        },
    };
    Relation {
        name,
        fields,
        keys: indexes.keys.into_boxed_slice(),
        outgoing: indexes.outgoing.into_boxed_slice(),
        capacity_sources: indexes.capacity_sources.into_boxed_slice(),
        capacity_targets: indexes.capacity_targets.into_boxed_slice(),
        body,
    }
}
