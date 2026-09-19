//! Captured source plans keep guard construction and exact transport explicit.
use super::{
    Import, ImportKind, Name, Parse, Scope, Tokens, events, expect_ident, expect_punct, fail,
    predicates, take_paren_group,
};

pub(super) struct Expression {
    predicate: predicates::Expression,
    plan: Name,
    operation: Name,
    input: Option<Name>,
}
fn end(tokens: &mut Tokens) -> Parse<()> {
    if let Some(extra) = tokens.next() {
        fail(extra.span(), "query!: unexpected token in guard expression")
    } else {
        Ok(())
    }
}
pub(super) fn parse(tokens: &mut Tokens) -> Parse<Expression> {
    let (mut body, _) = take_paren_group(tokens, "Guard's operation")?;
    let operation = expect_ident(&mut body, "Holds, Fails, Undefined, Lift or Descend")?;
    if !matches!(
        operation.text.as_str(),
        "Holds" | "Fails" | "Undefined" | "Lift" | "Descend"
    ) {
        return fail(operation.span, "query!: unknown guard operation");
    }
    let (mut args, _) = take_paren_group(&mut body, "guard operands")?;
    // One outer Guard node shares the predicate/numerical depth allowance.
    let predicate = predicates::expression(&mut args, 0, 1)?;
    expect_punct(&mut args, ',', "a comma before the guard plan")?;
    let plan = expect_ident(&mut args, "a declared guard plan")?;
    let input = if matches!(operation.text.as_str(), "Lift" | "Descend") {
        expect_punct(&mut args, ',', "a comma before the Event input")?;
        Some(expect_ident(&mut args, "an Event variable")?)
    } else {
        None
    };
    end(&mut args)?;
    end(&mut body)?;
    predicates::check_shape(&predicate, 2, &mut 1)?;
    Ok(Expression {
        predicate,
        plan,
        operation,
        input,
    })
}
impl Expression {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import]) -> Parse<String> {
        let plan = events::imported(&self.plan, imports, ImportKind::Guard)?;
        let predicate = self.predicate.emit(scope, imports, 1)?;
        let input = self.input.as_ref().map(|v| scope.head_var(v)).transpose()?;
        let operation = if let Some(v) = input {
            format!("{}(::bumbledb::VarId({v}))", self.operation.text)
        } else {
            self.operation.text.clone()
        };
        Ok(format!(
            "::bumbledb::GuardExpr {{ plan: {plan}, predicate: {predicate}, operation: ::bumbledb::GuardOp::{operation} }}"
        ))
    }
}
