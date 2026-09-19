//! All exports are encoded on the worker. The combined response has its own
//! byte/item limit, including repeated embedded spaces in derived descriptors.
use bumbledb::event::{
    AdmittedDescriptor, Capacity, CoordinateMap, DescriptorLimits, Error, Event, FibreProduct,
    SurjectiveMap,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

use super::encoded;
use crate::ingress::event_error;
use crate::marshal::output_vec;
use crate::runtime::RuntimeError;

pub struct Inspection {
    kind: &'static str,
    fields: Vec<(&'static str, Payload)>,
}
enum Payload {
    Bytes(Vec<u8>),
    List(Vec<Vec<u8>>),
}

struct Budget {
    bytes: usize,
    items: usize,
}
impl Budget {
    fn add(&mut self, len: usize) -> Result<(), RuntimeError> {
        self.items = self
            .items
            .checked_sub(1)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorItems)))?;
        self.bytes = self
            .bytes
            .checked_sub(len)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
        Ok(())
    }
    fn bytes(&mut self, value: Vec<u8>) -> Result<Payload, RuntimeError> {
        self.add(value.len())?;
        Ok(Payload::Bytes(value))
    }
    fn event(&mut self, value: &Event, work: &WorkContext) -> Result<Payload, RuntimeError> {
        self.bytes(value.to_bytes(work).map_err(event_error)?)
    }
    fn descriptor(
        &mut self,
        value: &AdmittedDescriptor,
        work: &WorkContext,
    ) -> Result<Payload, RuntimeError> {
        self.bytes(encoded(value, work)?)
    }
    fn onto(&mut self, value: &SurjectiveMap, work: &WorkContext) -> Result<Payload, RuntimeError> {
        self.descriptor(&AdmittedDescriptor::Surjective(value.clone()), work)
    }
    fn map(&mut self, value: &CoordinateMap, work: &WorkContext) -> Result<Payload, RuntimeError> {
        self.descriptor(&AdmittedDescriptor::Map(value.clone()), work)
    }
    fn product(
        &mut self,
        value: &FibreProduct,
        work: &WorkContext,
    ) -> Result<Payload, RuntimeError> {
        self.descriptor(&AdmittedDescriptor::Fibre(value.clone()), work)
    }
    fn list<T>(
        &mut self,
        values: &[T],
        work: &WorkContext,
        encode: impl Fn(&T, &WorkContext) -> Result<Vec<u8>, RuntimeError>,
    ) -> Result<Payload, RuntimeError> {
        self.add(0)?;
        if values.len() > self.items {
            return Err(event_error(Error::Capacity(Capacity::DescriptorItems)));
        }
        let mut result = output_vec(values.len())?;
        for value in values {
            work.checkpoint()?;
            let bytes = encode(value, work)?;
            self.add(bytes.len())?;
            result.push(bytes);
        }
        Ok(Payload::List(result))
    }
}

pub(super) fn inspect(
    value: &AdmittedDescriptor,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    inspect_with_limits(value, DescriptorLimits::default(), work)
}
fn inspect_with_limits(
    value: &AdmittedDescriptor,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    work.checkpoint()?;
    let mut budget = Budget {
        bytes: limits.bytes,
        items: limits.items,
    };
    budget.add(0)?;
    let mut fields = output_vec(5)?;
    let kind = match value {
        AdmittedDescriptor::Map(_) | AdmittedDescriptor::Surjective(_) => {
            let map = match value {
                AdmittedDescriptor::Map(map) => map,
                AdmittedDescriptor::Surjective(map) => map.map(),
                _ => unreachable!(),
            };
            fields.push(("source", budget.event(&map.source().full(), work)?));
            fields.push(("target", budget.event(&map.target().full(), work)?));
            fields.push((
                "readouts",
                budget.list(map.readouts(), work, |v, w| {
                    v.to_bytes(w).map_err(event_error)
                })?,
            ));
            if matches!(value, AdmittedDescriptor::Map(_)) {
                "map"
            } else {
                "surjective"
            }
        }
        AdmittedDescriptor::Faces(product) => {
            fields.push(("space", budget.event(&product.space().full(), work)?));
            fields.push((
                "projections",
                budget.list(product.projections(), work, |v, w| {
                    encoded(&AdmittedDescriptor::Surjective(v.clone()), w)
                })?,
            ));
            fields.push((
                "environments",
                budget.list(product.environments(), work, |v, w| {
                    encoded(&AdmittedDescriptor::Surjective(v.clone()), w)
                })?,
            ));
            "faces"
        }
        AdmittedDescriptor::Fibre(product) => {
            fields.push(("space", budget.event(&product.space().full(), work)?));
            fields.push(("left", budget.onto(product.left(), work)?));
            fields.push(("right", budget.onto(product.right(), work)?));
            fields.push((
                "leftEnvironment",
                budget.onto(product.left_environment(), work)?,
            ));
            fields.push((
                "rightEnvironment",
                budget.onto(product.right_environment(), work)?,
            ));
            "fibre"
        }
        AdmittedDescriptor::Relation(relation) => {
            fields.push(("region", budget.event(relation.region(), work)?));
            fields.push(("product", budget.product(relation.product(), work)?));
            fields.push(("input", budget.event(&relation.input().full(), work)?));
            fields.push(("output", budget.event(&relation.output().full(), work)?));
            "relation"
        }
        AdmittedDescriptor::Composition(plan) => {
            fields.push((
                "workspace",
                budget.event(&plan.workspace().space().full(), work)?,
            ));
            fields.push((
                "products",
                budget.list(&plan.products(), work, |v, w| {
                    encoded(&AdmittedDescriptor::Fibre((*v).clone()), w)
                })?,
            ));
            fields.push((
                "projections",
                budget.list(&plan.views(), work, |v, w| {
                    encoded(&AdmittedDescriptor::Surjective((*v).clone()), w)
                })?,
            ));
            "composition"
        }
        AdmittedDescriptor::Square(square) => {
            fields.push(("product", budget.product(square.product(), work)?));
            fields.push(("joint", budget.onto(square.joint(), work)?));
            fields.push(("left", budget.map(square.left(), work)?));
            fields.push(("right", budget.map(square.right(), work)?));
            "square"
        }
    };
    work.checkpoint()?;
    Ok(Inspection { kind, fields })
}

impl Inspection {
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("kind", self.kind)?;
        for (name, value) in self.fields {
            match value {
                Payload::Bytes(bytes) => object.set(name, Uint8Array::from(bytes))?,
                Payload::List(values) => {
                    let mut list = output_vec(values.len())
                        .map_err(|error| crate::runtime_wire::thrown(*env, error))?;
                    list.extend(values.into_iter().map(Uint8Array::from));
                    object.set(name, list)?;
                }
            }
        }
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{Space, SpaceId};
    use bumbledb::work::WorkError;

    #[test]
    fn inspection_bounds_combined_payload_and_keeps_cancellation() {
        let work = WorkContext::new();
        let space = Space::new(SpaceId([7; 32]), 1, &work).unwrap();
        let map = CoordinateMap::coordinates(&space, &space, &[0], &work).unwrap();
        let value = AdmittedDescriptor::Map(map);
        for limits in [
            DescriptorLimits {
                bytes: 1,
                ..DescriptorLimits::default()
            },
            DescriptorLimits {
                items: 1,
                ..DescriptorLimits::default()
            },
        ] {
            assert!(matches!(
                inspect_with_limits(&value, limits, &work),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        let result = inspect(&value, &work).unwrap();
        assert_eq!(result.kind, "map");
        assert_eq!(result.fields.len(), 3);
        work.cancel();
        assert!(matches!(
            inspect(&value, &work),
            Err(RuntimeError::Work(WorkError::Cancelled))
        ));
    }
}
