//! Constructive Event heads; this grammar binds only existing body variables.
use super::{
    Import, ImportKind, Name, Parse, Scope, Tokens, expect_ident, expect_punct, fail, parse_int,
    peek_punct, peek_span, take_paren_group,
};
use proc_macro::{Delimiter, TokenTree};
mod relations;

pub(super) enum Region {
    Leaf {
        op: &'static str,
        name: Name,
    },
    Not(Box<Self>),
    Apply {
        op: &'static str,
        left: Box<Self>,
        right: Box<Self>,
    },
    Ite(Box<Self>, Box<Self>, Box<Self>),
    Count {
        minimum: usize,
        maximum: usize,
        values: Vec<Self>,
    },
    Map {
        operation: String,
        input: Box<Self>,
        name: Name,
    },
    View {
        operation: String,
        relation: Box<relations::Relation>,
    },
    Modal {
        operation: String,
        relation: Box<relations::Relation>,
        input: Box<Self>,
    },
}

pub(super) struct Test {
    op: String,
    values: Vec<Region>,
}

pub(super) fn region(tokens: &mut Tokens) -> Parse<Region> {
    let (mut args, _) = take_paren_group(tokens, "Event's expression")?;
    let value = expression(&mut args, 0, 0)?;
    end(&mut args)?;
    Ok(value)
}

pub(super) fn test(tokens: &mut Tokens) -> Parse<Test> {
    let (mut body, _) = take_paren_group(tokens, "Test's structural predicate")?;
    let op = expect_ident(&mut body, "IsEmpty/IsFull/Subset/Equal/Disjoint/Covers")?;
    let arity = match op.text.as_str() {
        "IsEmpty" | "IsFull" => 1,
        "Subset" | "Equal" | "Disjoint" | "Covers" => 2,
        _ => return fail(op.span, "query!: unknown Event structural test"),
    };
    let (mut args, _) = take_paren_group(&mut body, "the test's arguments")?;
    let mut values = vec![expression(&mut args, 0, 0)?];
    if arity == 2 {
        expect_punct(&mut args, ',', "`,` between Event operands")?;
        values.push(expression(&mut args, 0, 0)?);
    }
    end(&mut args)?;
    end(&mut body)?;
    Ok(Test {
        op: op.text,
        values,
    })
}

pub(super) fn probability(tokens: &mut Tokens) -> Parse<(Region, Region)> {
    let (mut args, _) = take_paren_group(tokens, "Probability's Event and evidence")?;
    let event = expression(&mut args, 0, 0)?;
    expect_punct(&mut args, ',', "`,` between Event and evidence")?;
    let given = expression(&mut args, 0, 0)?;
    end(&mut args)?;
    Ok((event, given))
}

fn end(tokens: &mut Tokens) -> Parse<()> {
    if let Some(extra) = tokens.next() {
        fail(
            extra.span(),
            "query!: unexpected token after Event expression",
        )
    } else {
        Ok(())
    }
}

fn expression(tokens: &mut Tokens, precedence: u8, depth: usize) -> Parse<Region> {
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
        left = Region::Apply {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn unary(tokens: &mut Tokens, depth: usize) -> Parse<Region> {
    if depth > 128 {
        return fail(
            peek_span(tokens),
            "query!: Event expression is too deeply nested",
        );
    }
    if peek_punct(tokens, '!') {
        tokens.next();
        return Ok(Region::Not(Box::new(unary(tokens, depth + 1)?)));
    }
    if matches!(tokens.peek(), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
    {
        let (mut inner, _) = take_paren_group(tokens, "Event expression")?;
        let value = expression(&mut inner, 0, depth + 1)?;
        end(&mut inner)?;
        return Ok(value);
    }
    let name = expect_ident(tokens, "a bound Event variable or constructor")?;
    let op = match name.text.as_str() {
        "Empty" => "Empty",
        "Full" => "Full",
        "Region" | "Domain" | "Range" | "May" | "All" | "Must" | "Post" => {
            return relations::event(name, tokens, depth);
        }
        "Pullback" | "Image" | "UniversalImage" | "NonvacuousImage" | "Possible" | "Guaranteed" => {
            let (mut args, _) = take_paren_group(tokens, "Event readout arguments")?;
            let input = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "`,` before the imported map")?;
            let map = expect_ident(&mut args, "a `use map` import")?;
            end(&mut args)?;
            return Ok(Region::Map {
                operation: name.text,
                input: Box::new(input),
                name: map,
            });
        }
        "Ite" | "AtLeast" | "AtMost" | "Exactly" => {
            let (mut args, _) = take_paren_group(tokens, "Event constructor arguments")?;
            if name.text == "Ite" {
                let c = expression(&mut args, 0, depth + 1)?;
                expect_punct(&mut args, ',', "`,` after condition")?;
                let h = expression(&mut args, 0, depth + 1)?;
                expect_punct(&mut args, ',', "`,` after high branch")?;
                let l = expression(&mut args, 0, depth + 1)?;
                end(&mut args)?;
                return Ok(Region::Ite(Box::new(c), Box::new(h), Box::new(l)));
            }
            let count = parse_int(&mut args, "a nonnegative cardinality threshold")?;
            if count.negative {
                return fail(
                    name.span,
                    "query!: cardinality threshold must be nonnegative",
                );
            }
            let text = count
                .text
                .trim_end_matches("u64")
                .trim_end_matches("i64")
                .replace('_', "");
            let (radix, digits) = if let Some(v) = text.strip_prefix("0x") {
                (16, v)
            } else if let Some(v) = text.strip_prefix("0o") {
                (8, v)
            } else if let Some(v) = text.strip_prefix("0b") {
                (2, v)
            } else {
                (10, text.as_str())
            };
            let Ok(count) = usize::from_str_radix(digits, radix) else {
                return fail(name.span, "query!: cardinality threshold exceeds usize");
            };
            expect_punct(&mut args, ',', "`,` and at least one Event scope anchor")?;
            let mut values = vec![expression(&mut args, 0, depth + 1)?];
            while peek_punct(&mut args, ',') {
                args.next();
                values.push(expression(&mut args, 0, depth + 1)?);
                if values.len() > 4096 {
                    return fail(name.span, "query!: Event roster exceeds 4096 positions");
                }
            }
            end(&mut args)?;
            let (minimum, maximum) = match name.text.as_str() {
                "AtLeast" => (count, usize::MAX),
                "AtMost" => (0, count),
                _ => (count, count),
            };
            return Ok(Region::Count {
                minimum,
                maximum,
                values,
            });
        }
        _ => return Ok(Region::Leaf { op: "Var", name }),
    };
    let (mut args, _) = take_paren_group(tokens, "the Event scope anchor")?;
    let name = expect_ident(&mut args, "a bound Event variable")?;
    end(&mut args)?;
    Ok(Region::Leaf { op, name })
}

impl Region {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import], depth: usize) -> Parse<String> {
        if depth > 128 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: Event expression is too deeply nested",
            );
        }
        let child = |value: &Self| {
            value
                .emit(scope, imports, depth + 1)
                .map(|s| format!("::std::boxed::Box::new({s})"))
        };
        let prefix = "::bumbledb::EventExpr";
        Ok(match self {
            Self::Leaf { op, name } => format!(
                "{prefix}::{op}(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            Self::Not(value) => format!("{prefix}::Not({})", child(value)?),
            Self::Apply { op, left, right } => format!(
                "{prefix}::Apply {{ op: ::bumbledb::event::BoolOp4::{op}, left: {}, right: {} }}",
                child(left)?,
                child(right)?
            ),
            Self::Ite(c, h, l) => format!(
                "{prefix}::Ite {{ condition: {}, high: {}, low: {} }}",
                child(c)?,
                child(h)?,
                child(l)?
            ),
            Self::Count {
                minimum,
                maximum,
                values,
            } => {
                let values = values
                    .iter()
                    .map(|v| v.emit(scope, imports, depth + 1))
                    .collect::<Parse<Vec<_>>>()?
                    .join(",");
                format!(
                    "{prefix}::Cardinality {{ minimum: {minimum}, maximum: {maximum}, events: ::std::vec![{values}] }}"
                )
            }
            Self::Map {
                operation,
                input,
                name,
            } => {
                let captured = imported(name, imports, ImportKind::Map)?;
                format!(
                    "{prefix}::Map {{ operation: ::bumbledb::event::MapOp::{operation}, map: {captured}, input: {} }}",
                    child(input)?
                )
            }
            Self::View {
                operation,
                relation,
            } => format!(
                "{prefix}::Relation {{ operation: ::bumbledb::RelationViewOp::{operation}, relation: ::std::boxed::Box::new({}) }}",
                relation.emit(scope, imports, depth + 1)?
            ),
            Self::Modal {
                operation,
                relation,
                input,
            } => format!(
                "{prefix}::Modal {{ operation: ::bumbledb::event::ModalOp::{operation}, relation: ::std::boxed::Box::new({}), input: {} }}",
                relation.emit(scope, imports, depth + 1)?,
                child(input)?
            ),
        })
    }
}

fn imported(name: &Name, imports: &[Import], kind: ImportKind) -> Parse<String> {
    let Some((index, _)) = imports
        .iter()
        .enumerate()
        .find(|(_, import)| import.kind == kind && import.name.text == name.text)
    else {
        return fail(
            name.span,
            match kind {
                ImportKind::Map => "query!: readout requires a declared `use map` import",
                ImportKind::Faces => "query!: relation requires a declared `use faces` import",
                ImportKind::Product => {
                    "query!: composition/residual/closure requires a declared `use product` import"
                }
                ImportKind::Template => unreachable!("Event imports"),
            },
        );
    };
    Ok(format!("__event_import{index}.clone()"))
}

impl Test {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import]) -> Parse<String> {
        let values = self
            .values
            .iter()
            .map(|value| value.emit(scope, imports, 0))
            .collect::<Parse<Vec<_>>>()?
            .join(",");
        Ok(format!("::bumbledb::EventTest::{}({values})", self.op))
    }
}
