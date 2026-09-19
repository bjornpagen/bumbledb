//! Strict, copied plain BEAC descriptions; no JS object is a certificate.
use crate::event_descriptor_wire::data::{Budget, fibre_field, fibre_object, fields};
use crate::ingress::CopyContext;
use crate::marshal::{err, req, req_text};
use bumbledb::event::{ActionArenaDescriptor, ActionDescriptor, RelationDescriptor};
use napi::JsValue;
use napi::bindgen_prelude::{Env, Object, Uint8Array, Unknown};

fn arena(object: &Object, budget: &mut Budget<'_, '_>) -> napi::Result<ActionArenaDescriptor> {
    budget.item()?;
    fields(object, &["actions", "transition"])?;
    let actions = fibre_field(object, "actions", budget)?;
    let transition: Object = req(object, "transition", "Event action arena")?;
    budget.item()?;
    fields(&transition, &["product", "region"])?;
    Ok(ActionArenaDescriptor {
        actions,
        transition: RelationDescriptor {
            product: fibre_field(&transition, "product", budget)?,
            region: budget.bytes(req(&transition, "region", "Event action transition")?)?,
        },
    })
}

pub(super) fn parse(input: Unknown, copy: &CopyContext<'_>) -> napi::Result<ActionDescriptor> {
    if input.get_type()? != napi::ValueType::Object {
        return Err(err("Event action needs an object".into()));
    }
    let object = input.coerce_to_object()?;
    let kind = req_text(&object, "kind", "Event action")?;
    fields(
        &object,
        match kind.as_str() {
            "arena" => &["kind", "arena"],
            "reach" => &["kind", "arena", "goal", "policy"],
            "safe" => &["kind", "arena", "invariant", "policy"],
            _ => return Err(err("unknown Event action kind".into())),
        },
    )?;
    let mut budget = Budget::new(copy);
    budget.item()?;
    let arena = arena(
        &req::<Object>(&object, "arena", "Event action")?,
        &mut budget,
    )?;
    if kind == "arena" {
        return Ok(ActionDescriptor::Arena(arena));
    }
    let field = if kind == "reach" { "goal" } else { "invariant" };
    let objective = budget.bytes(req(&object, field, "Event action")?)?;
    let policy: Unknown = req(&object, "policy", "Event action")?;
    let policy = if policy.get_type()? == napi::ValueType::Null {
        None
    } else {
        Some(budget.bytes(policy)?)
    };
    Ok(if kind == "reach" {
        ActionDescriptor::Reach {
            arena,
            goal: objective,
            policy,
        }
    } else {
        ActionDescriptor::Safe {
            arena,
            invariant: objective,
            policy,
        }
    })
}
fn arena_object(env: &Env, value: ActionArenaDescriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    object.set("actions", fibre_object(env, value.actions)?)?;
    let mut transition = Object::new(env)?;
    transition.set("product", fibre_object(env, value.transition.product)?)?;
    transition.set("region", Uint8Array::from(value.transition.region))?;
    object.set("transition", transition)?;
    Ok(object)
}
pub(super) fn object(env: &Env, value: ActionDescriptor) -> napi::Result<Object<'_>> {
    let mut object = Object::new(env)?;
    let arena = match value {
        ActionDescriptor::Arena(arena) => {
            object.set("kind", "arena")?;
            arena
        }
        ActionDescriptor::Reach {
            arena,
            goal,
            policy,
        } => {
            object.set("kind", "reach")?;
            object.set("goal", Uint8Array::from(goal))?;
            object.set("policy", policy.map(Uint8Array::from))?;
            arena
        }
        ActionDescriptor::Safe {
            arena,
            invariant,
            policy,
        } => {
            object.set("kind", "safe")?;
            object.set("invariant", Uint8Array::from(invariant))?;
            object.set("policy", policy.map(Uint8Array::from))?;
            arena
        }
    };
    object.set("arena", arena_object(env, arena)?)?;
    Ok(object)
}
