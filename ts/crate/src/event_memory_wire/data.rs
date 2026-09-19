use bumbledb::event::{BeliefActionDescriptor, BeliefDescriptor};
use napi::JsValue;
use napi::bindgen_prelude::{Array, Env, Object, Uint8Array, Unknown};

use crate::event_descriptor_wire::data::{Budget, fibre_field, fibre_object, fields};
use crate::ingress::CopyContext;
use crate::marshal::{err, output_vec, req, req_at};

pub(super) fn parse(input: Unknown, copy: &CopyContext<'_>) -> napi::Result<BeliefDescriptor> {
    if input.get_type()? != napi::ValueType::Object {
        return Err(err("Event memory needs an object".into()));
    }
    let object = input.coerce_to_object()?;
    fields(&object, &["source", "given", "observations", "actions"])?;
    let mut budget = Budget::new(copy);
    budget.item()?;
    let source = budget.bytes(req(&object, "source", "Event memory")?)?;
    let given = budget.bytes(req(&object, "given", "Event memory")?)?;
    let cells: Array = req(&object, "observations", "Event memory")?;
    let mut observations = budget.list(cells.len())?;
    for index in 0..cells.len() {
        observations.push(budget.bytes(req_at(&cells, index, "Event observations")?)?);
    }
    let roster: Array = req(&object, "actions", "Event memory")?;
    let mut actions = budget.list(roster.len())?;
    for index in 0..roster.len() {
        budget.item()?;
        let action: Object = req_at(&roster, index, "Event actions")?;
        fields(&action, &["product", "region"])?;
        actions.push(BeliefActionDescriptor {
            product: fibre_field(&action, "product", &mut budget)?,
            region: budget.bytes(req(&action, "region", "Event action")?)?,
        });
    }
    Ok(BeliefDescriptor {
        source,
        given,
        observations,
        actions,
    })
}

pub(super) fn object(env: &Env, data: BeliefDescriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    object.set("source", Uint8Array::from(data.source))?;
    object.set("given", Uint8Array::from(data.given))?;
    let mut cells =
        output_vec(data.observations.len()).map_err(|e| crate::runtime_wire::thrown(*env, e))?;
    cells.extend(data.observations.into_iter().map(Uint8Array::from));
    object.set("observations", cells)?;
    let mut actions =
        output_vec(data.actions.len()).map_err(|e| crate::runtime_wire::thrown(*env, e))?;
    for action in data.actions {
        let mut value = Object::new(env)?;
        value.set("product", fibre_object(env, action.product)?)?;
        value.set("region", Uint8Array::from(action.region))?;
        actions.push(value);
    }
    object.set("actions", actions)?;
    Ok(object)
}
