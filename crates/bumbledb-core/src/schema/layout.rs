use std::collections::BTreeSet;

use super::{
    AccessComponent, AccessComponentRole, AccessLayout, ConstraintDescriptor, IndexKind,
    RelationDescriptor, Result, SchemaDescriptor, SchemaError,
};
use crate::schema::{FACT_ID_BYTES, INDEX_KEY_OVERHEAD_BYTES};

impl SchemaDescriptor {
    /// Computes all current-state index layouts and validates key lengths.
    pub fn access_layouts(&self, max_key_size: usize) -> Result<Vec<AccessLayout>> {
        let mut layouts = Vec::new();

        for (relation_id, relation) in self.relations.iter().enumerate() {
            let relation_id = relation_id as u16;
            let candidates = relation.access_candidates();

            for (index_id, candidate) in candidates.into_iter().enumerate() {
                let index_id = index_id as u16;
                let components = relation.access_components(&candidate.name, &candidate.fields)?;
                let encoded_len = INDEX_KEY_OVERHEAD_BYTES
                    + components
                        .iter()
                        .map(|component| component.encoded_width)
                        .sum::<usize>()
                    + FACT_ID_BYTES;

                if encoded_len > max_key_size {
                    return Err(SchemaError::KeyLayoutTooLarge {
                        relation: relation.name.clone(),
                        index: candidate.name,
                        actual: encoded_len,
                        max: max_key_size,
                    });
                }

                layouts.push(AccessLayout {
                    relation_name: relation.name.clone(),
                    relation_id,
                    index_name: candidate.name,
                    index_id,
                    kind: candidate.kind,
                    leading_fields: candidate.fields,
                    components,
                    encoded_len,
                });
            }
        }

        Ok(layouts)
    }
}

impl RelationDescriptor {
    fn access_candidates(&self) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();

        candidates.push(IndexCandidate {
            name: "fact_set".to_owned(),
            kind: IndexKind::FactSet,
            fields: self.fields.iter().map(|field| field.name.clone()).collect(),
        });

        for constraint in &self.constraints {
            if let ConstraintDescriptor::Unique { name, fields } = constraint {
                candidates.push(IndexCandidate {
                    name: format!("unique_{name}"),
                    kind: IndexKind::Unique,
                    fields: fields.clone(),
                });
            }
        }

        for constraint in &self.constraints {
            if let ConstraintDescriptor::ForeignKey { name, fields, .. } = constraint {
                candidates.push(IndexCandidate {
                    name: format!("by_fk_{name}"),
                    kind: IndexKind::ForeignKey,
                    fields: fields.clone(),
                });
            }
        }

        for field in &self.fields {
            if field.indexing.range {
                candidates.push(IndexCandidate {
                    name: format!("by_{}", field.name),
                    kind: IndexKind::Range,
                    fields: vec![field.name.clone()],
                });
            }
        }

        for index in &self.indexes {
            candidates.push(IndexCandidate {
                name: index.name.clone(),
                kind: index.kind,
                fields: index.fields.clone(),
            });
        }

        candidates
    }

    fn access_components(
        &self,
        index_name: &str,
        leading_fields: &[String],
    ) -> Result<Vec<AccessComponent>> {
        let mut components = Vec::with_capacity(leading_fields.len());
        let mut seen = BTreeSet::new();

        for field_name in leading_fields {
            let field = self
                .field(field_name)
                .ok_or_else(|| SchemaError::UnknownField {
                    relation: self.name.clone(),
                    field: field_name.clone(),
                })?;

            if !seen.insert(field.name.clone()) {
                return Err(SchemaError::DuplicateIndexField {
                    relation: self.name.clone(),
                    index: index_name.to_owned(),
                    field: field.name.clone(),
                });
            }
            components.push(AccessComponent::new(field, AccessComponentRole::Leading));
        }

        Ok(components)
    }
}

#[derive(Clone, Debug)]
struct IndexCandidate {
    name: String,
    kind: IndexKind,
    fields: Vec<String>,
}

pub(super) fn generated_index_names(relation: &RelationDescriptor) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.insert("fact_set".to_owned());
    for field in &relation.fields {
        if field.indexing.range {
            names.insert(format!("by_{}", field.name));
        }
    }
    for constraint in &relation.constraints {
        match constraint {
            ConstraintDescriptor::Unique { name, .. } => {
                names.insert(format!("unique_{name}"));
            }
            ConstraintDescriptor::ForeignKey { name, .. } => {
                names.insert(format!("by_fk_{name}"));
            }
        }
    }
    names
}
