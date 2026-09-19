//! Shape-only parsing: opaque owned numerical payloads are admitted on workers.
use super::{EventBudget, Object, err, exact_fields, ordinal, req, req_text, var_in};
use crate::ingress::query::NumberExpr;
use bumbledb::ObservationComponent;
use bumbledb::event::NumberOp;

fn child(
    obj: &Object,
    key: &str,
    depth: usize,
    budget: &mut EventBudget<'_, '_>,
) -> napi::Result<NumberExpr> {
    parse(
        &req::<Object>(obj, key, "numerical child")?,
        depth + 1,
        budget,
    )
}
pub(super) fn bytes(
    obj: &Object,
    key: &str,
    budget: &mut EventBudget<'_, '_>,
) -> napi::Result<Vec<u8>> {
    let value = budget
        .copy
        .bytes(req(obj, key, "numerical bytes")?, budget.bytes)?;
    budget.bytes -= value.len();
    Ok(value)
}
pub(super) fn parse(
    obj: &Object,
    depth: usize,
    budget: &mut EventBudget<'_, '_>,
) -> napi::Result<NumberExpr> {
    budget.copy.checkpoint()?;
    if depth > 128 || budget.nodes == 0 {
        return Err(err("numerical expression exceeds shape budget".into()));
    }
    budget.nodes -= 1;
    let kind = req_text(obj, "kind", "numerical expression")?;
    Ok(match kind.as_str() {
        "var" | "integer" => {
            exact_fields(obj, &["kind", "var"])?;
            let var = var_in(obj, "var", "numerical variable")?;
            if kind == "var" {
                NumberExpr::Var(var)
            } else {
                NumberExpr::Integer(var)
            }
        }
        "component" => {
            exact_fields(obj, &["kind", "observation", "component"])?;
            let component = match req_text(obj, "component", "observation component")?.as_str() {
                "value" => ObservationComponent::Value,
                "numerator" => ObservationComponent::Numerator,
                "evidenceMass" => ObservationComponent::EvidenceMass,
                _ => return Err(err("unknown numerical observation component".into())),
            };
            NumberExpr::Component {
                observation: var_in(obj, "observation", "observation variable")?,
                component,
            }
        }
        "literal" | "imported" => {
            exact_fields(obj, &["kind", "bytes"])?;
            let bytes = bytes(obj, "bytes", budget)?;
            if kind == "literal" {
                NumberExpr::Literal(bytes)
            } else {
                NumberExpr::Imported(bytes)
            }
        }
        "add" | "subtract" | "multiply" | "divide" | "min" | "max" => {
            exact_fields(obj, &["kind", "left", "right"])?;
            let op = match kind.as_str() {
                "add" => NumberOp::Add,
                "subtract" => NumberOp::Subtract,
                "multiply" => NumberOp::Multiply,
                "divide" => NumberOp::Divide,
                "min" => NumberOp::Min,
                _ => NumberOp::Max,
            };
            NumberExpr::Binary {
                op,
                left: Box::new(child(obj, "left", depth, budget)?),
                right: Box::new(child(obj, "right", depth, budget)?),
            }
        }
        "negate" | "abs" => {
            exact_fields(obj, &["kind", "value"])?;
            let value = Box::new(child(obj, "value", depth, budget)?);
            if kind == "negate" {
                NumberExpr::Negate(value)
            } else {
                NumberExpr::Abs(value)
            }
        }
        "pow" => {
            exact_fields(obj, &["kind", "value", "exponent"])?;
            let exponent = ordinal(
                req(obj, "exponent", "numerical exponent")?,
                "numerical exponent",
            )?;
            NumberExpr::Pow {
                value: Box::new(child(obj, "value", depth, budget)?),
                exponent,
            }
        }
        "onDomain" => {
            exact_fields(obj, &["kind", "value", "domain"])?;
            NumberExpr::OnDomain {
                value: Box::new(child(obj, "value", depth, budget)?),
                domain: bytes(obj, "domain", budget)?,
            }
        }
        _ => return Err(err("unknown numerical expression".into())),
    })
}
