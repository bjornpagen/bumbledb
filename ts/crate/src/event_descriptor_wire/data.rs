//! Fixed-depth plain descriptor grammar. Input copies no borrowed JS backing;
//! output transfers already canonical worker-owned payloads without decoding.
use bumbledb::event::{
    Capacity, Descriptor, DescriptorLimits, Error as EventError, FibreDescriptor, MapDescriptor,
    SpaceId,
};
use napi::JsValue;
use napi::bindgen_prelude::{Array, Env, Object, Uint8Array, Unknown};

use crate::ingress::{CopyContext, event_error};
use crate::marshal::{err, output_vec, req, req_at, req_text};

pub(crate) struct Budget<'a, 'work> {
    copy: &'a CopyContext<'work>,
    bytes: usize,
    items: usize,
}
impl<'a, 'work> Budget<'a, 'work> {
    pub(crate) fn new(copy: &'a CopyContext<'work>) -> Self {
        let limits = DescriptorLimits::default();
        Self {
            copy,
            bytes: limits.bytes,
            items: limits.items,
        }
    }

    pub(crate) fn item(&mut self) -> napi::Result<()> {
        self.copy.checkpoint()?;
        self.items = self.copy.checked(
            self.items
                .checked_sub(1)
                .ok_or_else(|| event_error(EventError::Capacity(Capacity::DescriptorItems))),
        )?;
        Ok(())
    }
    pub(crate) fn bytes(&mut self, value: Unknown) -> napi::Result<Vec<u8>> {
        self.item()?;
        let bytes = self.copy.bytes(value, self.bytes)?;
        self.bytes -= bytes.len();
        Ok(bytes)
    }
    fn identity(&mut self, object: &Object) -> napi::Result<SpaceId> {
        let bytes = self
            .copy
            .bytes(req(object, "identity", "Event descriptor")?, self.bytes)?;
        self.bytes -= bytes.len();
        Ok(SpaceId(bytes.try_into().map_err(|_| {
            err("Event identity needs 32 bytes".into())
        })?))
    }
    pub(crate) fn list<T>(&self, len: u32) -> napi::Result<Vec<T>> {
        if len as usize > self.items {
            return self.copy.checked(Err(event_error(EventError::Capacity(
                Capacity::DescriptorItems,
            ))));
        }
        self.copy.checked(output_vec(len as usize))
    }
}

pub(crate) fn fields(object: &Object, allowed: &[&str]) -> napi::Result<()> {
    let keys = Object::keys(object)?;
    if keys.len() != allowed.len() || keys.iter().any(|key| !allowed.contains(&key.as_str())) {
        return Err(err("unknown or missing Event descriptor field".into()));
    }
    Ok(())
}

fn map(object: &Object, budget: &mut Budget<'_, '_>) -> napi::Result<MapDescriptor> {
    budget.item()?;
    fields(object, &["source", "target", "readouts"])?;
    let source = budget.bytes(req(object, "source", "Event map")?)?;
    let target = budget.bytes(req(object, "target", "Event map")?)?;
    let readouts: Array = req(object, "readouts", "Event map")?;
    let mut values = budget.list(readouts.len())?;
    for index in 0..readouts.len() {
        values.push(budget.bytes(req_at(&readouts, index, "Event readouts")?)?);
    }
    Ok(MapDescriptor {
        source,
        target,
        readouts: values,
    })
}
fn map_field(
    object: &Object,
    key: &str,
    budget: &mut Budget<'_, '_>,
) -> napi::Result<MapDescriptor> {
    map(&req::<Object>(object, key, "Event descriptor")?, budget)
}
fn fibre(object: &Object, budget: &mut Budget<'_, '_>) -> napi::Result<FibreDescriptor> {
    budget.item()?;
    fields(object, &["identity", "left", "right", "reversed"])?;
    Ok(FibreDescriptor {
        identity: budget.identity(object)?,
        left: Box::new(map_field(object, "left", budget)?),
        right: Box::new(map_field(object, "right", budget)?),
        reversed: req(object, "reversed", "Event fibre")?,
    })
}
pub(crate) fn fibre_field(
    object: &Object,
    key: &str,
    budget: &mut Budget<'_, '_>,
) -> napi::Result<FibreDescriptor> {
    fibre(&req::<Object>(object, key, "Event descriptor")?, budget)
}

pub(super) fn parse(input: Unknown, copy: &CopyContext<'_>) -> napi::Result<Descriptor> {
    // Conversion only checks the JS object shape; Event/role admission is later.
    if input.get_type()? != napi::ValueType::Object {
        return Err(err("Event descriptor needs an object".into()));
    }
    let object: Object = input.coerce_to_object()?;
    let mut budget = Budget::new(copy);
    budget.item()?;
    let kind = req_text(&object, "kind", "Event descriptor")?;
    Ok(match kind.as_str() {
        "map" | "surjective" => {
            fields(&object, &["kind", "map"])?;
            let value = map_field(&object, "map", &mut budget)?;
            if kind == "map" {
                Descriptor::Map(value)
            } else {
                Descriptor::Surjective(value)
            }
        }
        "faces" => {
            fields(&object, &["kind", "identity", "environments"])?;
            let identity = budget.identity(&object)?;
            let inputs: Array = req(&object, "environments", "Event faces")?;
            let mut environments = budget.list(inputs.len())?;
            for index in 0..inputs.len() {
                environments.push(map(&req_at(&inputs, index, "Event faces")?, &mut budget)?);
            }
            Descriptor::Faces {
                identity,
                environments,
            }
        }
        "fibre" => {
            fields(&object, &["kind", "product"])?;
            Descriptor::Fibre(fibre_field(&object, "product", &mut budget)?)
        }
        "relation" => {
            fields(&object, &["kind", "product", "region"])?;
            Descriptor::Relation {
                product: fibre_field(&object, "product", &mut budget)?,
                region: budget.bytes(req(&object, "region", "Event relation")?)?,
            }
        }
        "composition" => {
            fields(&object, &["kind", "identity", "st", "tu", "su"])?;
            Descriptor::Composition {
                identity: budget.identity(&object)?,
                st: fibre_field(&object, "st", &mut budget)?,
                tu: fibre_field(&object, "tu", &mut budget)?,
                su: fibre_field(&object, "su", &mut budget)?,
            }
        }
        "square" => {
            fields(&object, &["kind", "product", "left", "right"])?;
            Descriptor::Square {
                product: fibre_field(&object, "product", &mut budget)?,
                left: map_field(&object, "left", &mut budget)?,
                right: map_field(&object, "right", &mut budget)?,
            }
        }
        _ => return Err(err("unknown Event descriptor kind".into())),
    })
}

fn map_object(env: &Env, value: MapDescriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    object.set("source", Uint8Array::from(value.source))?;
    object.set("target", Uint8Array::from(value.target))?;
    let mut readouts = output_vec(value.readouts.len())
        .map_err(|error| crate::runtime_wire::thrown(*env, error))?;
    readouts.extend(value.readouts.into_iter().map(Uint8Array::from));
    object.set("readouts", readouts)?;
    Ok(object)
}
pub(crate) fn fibre_object(env: &Env, value: FibreDescriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    object.set("identity", Uint8Array::from(value.identity.0.to_vec()))?;
    object.set("left", map_object(env, *value.left)?)?;
    object.set("right", map_object(env, *value.right)?)?;
    object.set("reversed", value.reversed)?;
    Ok(object)
}
pub(super) fn object(env: &Env, value: Descriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    match value {
        Descriptor::Map(value) => {
            object.set("kind", "map")?;
            object.set("map", map_object(env, value)?)?;
        }
        Descriptor::Surjective(value) => {
            object.set("kind", "surjective")?;
            object.set("map", map_object(env, value)?)?;
        }
        Descriptor::Faces {
            identity,
            environments,
        } => {
            object.set("kind", "faces")?;
            object.set("identity", Uint8Array::from(identity.0.to_vec()))?;
            let mut maps = output_vec(environments.len())
                .map_err(|error| crate::runtime_wire::thrown(*env, error))?;
            for value in environments {
                maps.push(map_object(env, value)?);
            }
            object.set("environments", maps)?;
        }
        Descriptor::Fibre(value) => {
            object.set("kind", "fibre")?;
            object.set("product", fibre_object(env, value)?)?;
        }
        Descriptor::Relation { product, region } => {
            object.set("kind", "relation")?;
            object.set("product", fibre_object(env, product)?)?;
            object.set("region", Uint8Array::from(region))?;
        }
        Descriptor::Composition {
            identity,
            st,
            tu,
            su,
        } => {
            object.set("kind", "composition")?;
            object.set("identity", Uint8Array::from(identity.0.to_vec()))?;
            for (name, value) in [("st", st), ("tu", tu), ("su", su)] {
                object.set(name, fibre_object(env, value)?)?;
            }
        }
        Descriptor::Square {
            product,
            left,
            right,
        } => {
            object.set("kind", "square")?;
            object.set("product", fibre_object(env, product)?)?;
            object.set("left", map_object(env, left)?)?;
            object.set("right", map_object(env, right)?)?;
        }
    }
    Ok(object)
}
