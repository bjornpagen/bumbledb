//! Shape parsing owns buffers. Worker admission replays all opaque derivations.
use super::{EventBudget, Object, err, exact_fields, numbers, ordinal, req, req_text, var_in};
use crate::ingress::query::PredicateExpr;
use bumbledb::event::{BoolOp4, PolynomialSigns};

fn child(
    obj: &Object,
    key: &str,
    depth: usize,
    budget: &mut EventBudget<'_, '_>,
) -> napi::Result<PredicateExpr> {
    parse(
        &req::<Object>(obj, key, "predicate child")?,
        depth + 1,
        budget,
    )
}
pub(super) fn parse(
    obj: &Object,
    depth: usize,
    budget: &mut EventBudget<'_, '_>,
) -> napi::Result<PredicateExpr> {
    budget.copy.checkpoint()?;
    if depth > 128 || budget.nodes == 0 {
        return Err(err("predicate expression exceeds shape budget".into()));
    }
    budget.nodes -= 1;
    Ok(
        match req_text(obj, "kind", "predicate expression")?.as_str() {
            "var" => {
                exact_fields(obj, &["kind", "var"])?;
                PredicateExpr::Var(var_in(obj, "var", "predicate variable")?)
            }
            "sign" => {
                exact_fields(obj, &["kind", "number", "signs"])?;
                let signs = u8::try_from(ordinal(req(obj, "signs", "sign mask")?, "sign mask")?)
                    .ok()
                    .and_then(PolynomialSigns::new)
                    .ok_or_else(|| err("invalid sign mask".into()))?;
                PredicateExpr::Sign {
                    number: numbers::parse(
                        &req::<Object>(obj, "number", "numerical child")?,
                        depth + 1,
                        budget,
                    )?,
                    signs,
                }
            }
            "imported" => {
                exact_fields(obj, &["kind", "bytes"])?;
                PredicateExpr::Imported(numbers::bytes(obj, "bytes", budget)?)
            }
            "negate" => {
                exact_fields(obj, &["kind", "value"])?;
                PredicateExpr::Negate(Box::new(child(obj, "value", depth, budget)?))
            }
            "apply" => {
                exact_fields(obj, &["kind", "op", "left", "right"])?;
                let op = u8::try_from(ordinal(req(obj, "op", "truth table")?, "truth table")?)
                    .ok()
                    .and_then(BoolOp4::new)
                    .ok_or_else(|| err("invalid truth table".into()))?;
                PredicateExpr::Binary {
                    op,
                    left: Box::new(child(obj, "left", depth, budget)?),
                    right: Box::new(child(obj, "right", depth, budget)?),
                }
            }
            "onDomain" => {
                exact_fields(obj, &["kind", "value", "domain"])?;
                PredicateExpr::OnDomain {
                    value: Box::new(child(obj, "value", depth, budget)?),
                    domain: numbers::bytes(obj, "domain", budget)?,
                }
            }
            _ => return Err(err("unknown predicate expression".into())),
        },
    )
}
