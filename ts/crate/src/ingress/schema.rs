//! Owned schema grammar with unresolved Event envelopes. Copying does no graph
//! work; the worker interprets every value before name resolution and sealing.
use super::{Admit, CopyContext, ValueInput};
use crate::runtime::RuntimeError;
use bumbledb::WorkContext;
use bumbledb::schema::spec::SchemaSpecData as SchemaSpec;
use napi::bindgen_prelude::{Env, Object};

pub(crate) struct SchemaInput(SchemaSpec<ValueInput>);

impl SchemaInput {
    pub(crate) fn copy(env: Env, spec: &Object, work: &WorkContext) -> Result<Self, RuntimeError> {
        let copy = CopyContext::new(env, work);
        copy.finish(crate::marshal::schema_spec_with(spec, &|value| {
            copy.tagged_value(value)
        }))
        .map(Self)
    }

    // Shape metadata used only for copying row envelopes. Validation and row
    // admission still run on the worker before any schema or facts are sealed.
    pub(crate) fn row_shape(
        &self,
        relation: u32,
    ) -> Result<(&str, Vec<bumbledb::schema::FieldDescriptor>), RuntimeError> {
        let relation = self
            .0
            .relations
            .get(relation as usize)
            .ok_or(RuntimeError::InvalidArgument)?;
        let mut fields = Vec::new();
        if relation.closed.is_some() {
            fields.push(bumbledb::schema::FieldDescriptor {
                name: "id".into(),
                value_type: bumbledb::schema::ValueType::U64,
            });
        }
        fields.extend(
            relation
                .fields
                .iter()
                .map(|field| bumbledb::schema::FieldDescriptor {
                    name: field.name.clone(),
                    value_type: field.value_type,
                }),
        );
        Ok((&relation.name, fields))
    }

    pub(crate) fn resolve(
        self,
        work: &WorkContext,
    ) -> Result<(bumbledb::SchemaDescriptor, crate::FieldAttrsTable), RuntimeError> {
        self.admit(work)?.map_err(|error| RuntimeError::Engine {
            diagnostic: None,
            kind: crate::tags::error_family::SCHEMA,
            message: match error {
                crate::OpenOutcome::SchemaError(message)
                | crate::OpenOutcome::NewtypeMismatch(message) => message,
            },
        })
    }

    pub(crate) fn admit(self, work: &WorkContext) -> Result<crate::SchemaResolution, RuntimeError> {
        work.checkpoint()?;
        let spec = self.0.try_map_values(|value| value.admit(work))?;
        Ok(crate::resolve_spec(&spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeError;
    use bumbledb::event::{Space, SpaceId};
    use bumbledb::schema::spec::{
        FieldSpec, LiteralSetSpecData as LiteralSetSpec, LiteralSpecData as LiteralSpec,
        RelationSpecData as RelationSpec, SideSpecData as SideSpec,
        StatementSpecData as StatementSpec,
    };
    use bumbledb::schema::{ManifestDescriptor, Projection, ValidateDescriptor, ValueType};

    fn input(bytes: Vec<u8>) -> SchemaInput {
        SchemaInput(SchemaSpec {
            relations: ["A", "B"]
                .iter()
                .map(|name| RelationSpec {
                    name: (*name).into(),
                    closed: None,
                    fields: vec![
                        FieldSpec {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            newtype: None,
                        },
                        FieldSpec {
                            name: "event".into(),
                            value_type: ValueType::Event,
                            newtype: None,
                        },
                    ],
                })
                .collect(),
            statements: vec![
                StatementSpec::Fd {
                    relation: "B".into(),
                    projection: Projection::Fields(vec!["id".into()].into()),
                },
                StatementSpec::Containment {
                    bidirectional: false,
                    source: SideSpec {
                        relation: "A".into(),
                        projection: Projection::Fields(vec!["id".into()].into()),
                        selection: vec![(
                            "event".into(),
                            LiteralSetSpec::One(LiteralSpec::Value(ValueInput::Event(bytes))),
                        )],
                    },
                    target: SideSpec {
                        relation: "B".into(),
                        projection: Projection::Fields(vec!["id".into()].into()),
                        selection: vec![],
                    },
                },
            ],
        })
    }

    #[test]
    fn schema_envelopes_cross_threads_then_admit_and_capture() {
        let value = Space::new(SpaceId([118; 32]), 2, &()).unwrap().full();
        let pending = input(value.to_bytes(&()).unwrap());
        drop(value);
        std::thread::spawn(move || {
            let work = WorkContext::new();
            let (descriptor, attrs) = pending.resolve(&work).unwrap();
            let schema = descriptor.clone().validate_with_control(&work).unwrap();
            crate::marshal::DescriptorWire::capture(
                descriptor.manifest(),
                descriptor.statements.clone(),
                bumbledb::schema::fingerprint::fingerprint(&schema).to_string(),
                attrs.clone(),
                &work,
            )
            .unwrap();
            work.cancel();
            assert!(matches!(
                crate::marshal::DescriptorWire::capture(
                    descriptor.manifest(),
                    descriptor.statements.clone(),
                    String::new(),
                    attrs,
                    &work
                ),
                Err(bumbledb::event::Error::Cancelled)
            ));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn bad_schema_event_is_deferred_and_cancelled_admission_never_seals() {
        let work = WorkContext::new();
        assert!(matches!(
            input(b"BEVT\x01".to_vec()).resolve(&work),
            Err(RuntimeError::Engine { .. })
        ));
        work.cancel();
        assert!(matches!(
            input(b"BEVT\x01".to_vec()).resolve(&work),
            Err(RuntimeError::Work(bumbledb::WorkError::Cancelled))
        ));
    }
}
