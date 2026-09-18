//! Relation grammar retains explicit full products and composition workspaces.
use super::{
    Delimiter, Import, ImportKind, Name, Parse, Region, Scope, TokenTree, Tokens, end,
    expect_ident, expect_punct, fail, imported, peek_punct, peek_span, take_paren_group,
};

pub(super) fn event(name: Name, tokens: &mut Tokens, depth: usize) -> Parse<Region> {
    let (mut args, _) = take_paren_group(tokens, "Event relation arguments")?;
    let relation = Box::new(expression(&mut args, 0, depth + 1)?);
    let result = if matches!(name.text.as_str(), "Region" | "Domain" | "Range") {
        Region::View {
            operation: name.text,
            relation,
        }
    } else {
        expect_punct(&mut args, ',', "`,` before the modal Event operand")?;
        Region::Modal {
            operation: name.text,
            relation,
            input: Box::new(super::expression(&mut args, 0, depth + 1)?),
        }
    };
    end(&mut args)?;
    Ok(result)
}

pub(crate) enum Relation {
    Bind {
        event: Region,
        faces: Name,
    },
    Identity(Name),
    Test {
        event: Region,
        faces: Name,
    },
    Not(Box<Self>),
    Apply {
        op: &'static str,
        left: Box<Self>,
        right: Box<Self>,
    },
    Converse(Box<Self>),
    Product {
        operation: String,
        left: Box<Self>,
        right: Box<Self>,
        plan: Name,
    },
    Star {
        relation: Box<Self>,
        plan: Name,
    },
}

pub(super) fn expression(tokens: &mut Tokens, precedence: u8, depth: usize) -> Parse<Relation> {
    if precedence == 3 {
        return unary(tokens, depth);
    }
    let (symbol, op) = match precedence {
        0 => ('|', "OR"),
        1 => ('^', "XOR"),
        _ => ('&', "AND"),
    };
    let mut left = expression(tokens, precedence + 1, depth)?;
    while peek_punct(tokens, symbol) {
        tokens.next();
        let right = expression(tokens, precedence + 1, depth)?;
        left = Relation::Apply {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn unary(tokens: &mut Tokens, depth: usize) -> Parse<Relation> {
    if depth > 128 {
        return fail(
            peek_span(tokens),
            "query!: relation expression is too deeply nested",
        );
    }
    if peek_punct(tokens, '!') {
        tokens.next();
        return Ok(Relation::Not(Box::new(unary(tokens, depth + 1)?)));
    }
    if matches!(tokens.peek(), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
    {
        let (mut args, _) = take_paren_group(tokens, "relation expression")?;
        let value = expression(&mut args, 0, depth + 1)?;
        end(&mut args)?;
        return Ok(value);
    }
    let name = expect_ident(tokens, "a typed relation constructor")?;
    let (mut args, _) = take_paren_group(tokens, "relation arguments")?;
    let value = match name.text.as_str() {
        "Relation" | "TestRelation" => {
            let event = super::expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "`,` before the imported faces")?;
            let faces = expect_ident(&mut args, "a `use faces` import")?;
            if name.text == "Relation" {
                Relation::Bind { event, faces }
            } else {
                Relation::Test { event, faces }
            }
        }
        "Id" => Relation::Identity(expect_ident(&mut args, "a `use faces` import")?),
        "Converse" => Relation::Converse(Box::new(expression(&mut args, 0, depth + 1)?)),
        "Star" => {
            let relation = Box::new(expression(&mut args, 0, depth + 1)?);
            expect_punct(
                &mut args,
                ',',
                "`,` before the explicit closure product plan",
            )?;
            let plan = expect_ident(&mut args, "a `use product` import")?;
            Relation::Star { relation, plan }
        }
        "Compose" | "LeftResidual" | "RightResidual" => {
            let left = Box::new(expression(&mut args, 0, depth + 1)?);
            expect_punct(&mut args, ',', "`,` before the right relation")?;
            let right = Box::new(expression(&mut args, 0, depth + 1)?);
            expect_punct(&mut args, ',', "`,` before the explicit product plan")?;
            let plan = expect_ident(&mut args, "a `use product` import")?;
            Relation::Product {
                operation: name.text,
                left,
                right,
                plan,
            }
        }
        _ => return fail(name.span, "query!: unknown typed relation constructor"),
    };
    end(&mut args)?;
    Ok(value)
}

impl Relation {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import], depth: usize) -> Parse<String> {
        if depth > 128 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: relation expression is too deeply nested",
            );
        }
        let prefix = "::bumbledb::RelationExpr";
        let child = |r: &Self| {
            r.emit(scope, imports, depth + 1)
                .map(|s| format!("::std::boxed::Box::new({s})"))
        };
        Ok(match self {
            Self::Bind { event, faces } | Self::Test { event, faces } => {
                let captured = imported(faces, imports, ImportKind::Faces)?;
                let expression = event.emit(scope, imports, depth + 1)?;
                let (variant, field) = if matches!(self, Self::Bind { .. }) {
                    ("Bind", "region")
                } else {
                    ("Test", "predicate")
                };
                format!(
                    "{prefix}::{variant} {{ faces: {captured}, {field}: ::std::boxed::Box::new({expression}) }}"
                )
            }
            Self::Identity(faces) => format!(
                "{prefix}::Identity {{ faces: {} }}",
                imported(faces, imports, ImportKind::Faces)?
            ),
            Self::Not(value) => format!("{prefix}::Not({})", child(value)?),
            Self::Converse(value) => format!("{prefix}::Converse({})", child(value)?),
            Self::Apply { op, left, right } => format!(
                "{prefix}::Apply {{ op: ::bumbledb::event::BoolOp4::{op}, left: {}, right: {} }}",
                child(left)?,
                child(right)?
            ),
            Self::Product {
                operation,
                left,
                right,
                plan,
            } => format!(
                "{prefix}::Product {{ operation: ::bumbledb::RelationProductOp::{operation}, plan: {}, left: {}, right: {} }}",
                imported(plan, imports, ImportKind::Product)?,
                child(left)?,
                child(right)?
            ),
            Self::Star { relation, plan } => format!(
                "{prefix}::Star {{ plan: {}, relation: {} }}",
                imported(plan, imports, ImportKind::Product)?,
                child(relation)?
            ),
        })
    }
}
