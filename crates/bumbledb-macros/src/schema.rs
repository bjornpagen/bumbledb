//! `schema!`: the declaration grammar's parse. Names keep their source
//! idents, so every later error and every emitted item points at the token
//! the user wrote.
mod check;
mod emit;
mod lower;

use bumbledb_theory::schema::{FixedIntervalElement, IntervalElement};
use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};

use crate::lex::{Cursor, Lit, Result};

/// The emitted items carry the invocation's span, so lints and errors on
/// generated code treat it as the macro's, not as code the user wrote.
pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let schema = parse(input)?;
    let (descriptor, spans) = lower::descriptor(&schema)?;
    check::check(&schema, &descriptor, &spans)?;
    Ok(call_site(emit::schema(&schema, &descriptor)?))
}

fn call_site(stream: TokenStream) -> TokenStream {
    stream
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &token {
                let mut inner = Group::new(group.delimiter(), call_site(group.stream()));
                inner.set_span(Span::call_site());
                token = TokenTree::Group(inner);
            }
            token.set_span(Span::call_site());
            token
        })
        .collect()
}

#[derive(Clone, Copy)]
enum FieldTy {
    Bool,
    U64,
    I64,
    F64,
    Uuid,
    Str,
    FixedBytes(u16),
    Interval(IntervalElement),
    FixedInterval(FixedIntervalElement, u64),
}

struct Field {
    name: Ident,
    ty: FieldTy,
    newtype: Option<Ident>,
}

/// A relation's declared fields; a closed relation's synthetic `id` field
/// (field 0, typed by the handle newtype) is not among them.
struct Relation {
    name: Ident,
    fields: Vec<Field>,
    closed: Option<Closed>,
}

struct Closed {
    newtype: Ident,
    rows: Vec<ClosedRow>,
}

/// One extension row: its handle and one literal per declared field, in
/// field order.
struct ClosedRow {
    handle: Ident,
    values: Vec<Literal>,
}

enum Literal {
    Lit(Lit),
    Handle(Ident),
}

enum Literals {
    One(Literal),
    Many(Vec<Literal>, Span),
}

struct Binding {
    field: Ident,
    literals: Literals,
}

struct Side {
    relation: Ident,
    projection: Vec<Ident>,
    selection: Vec<Binding>,
}

enum Weight {
    Unit,
    Field(Ident),
    Duration(Ident),
}

enum Bound {
    Lit(u64),
    Field(Ident),
    Duration(Ident),
}

enum Window {
    Exact(Bound),
    Range(Bound, Bound),
    Floor(Bound),
}

enum Statement {
    Key {
        relation: Ident,
        projection: Vec<Ident>,
    },
    Containment {
        source: Side,
        target: Side,
        bidirectional: bool,
    },
    Capacity {
        target: Side,
        weight: Weight,
        window: Window,
        window_span: Span,
        source: Side,
    },
}

struct Schema {
    name: Ident,
    relations: Vec<Relation>,
    statements: Vec<Statement>,
}

impl Schema {
    fn relation(&self, name: &Ident) -> Option<&Relation> {
        self.relations
            .iter()
            .find(|relation| relation.name == *name)
    }

    fn field_ty(&self, relation: &Ident, field: &Ident) -> Option<FieldTy> {
        let relation = self.relation(relation)?;
        if relation.closed.is_some() && field == "id" {
            return Some(FieldTy::U64);
        }
        relation
            .fields
            .iter()
            .find(|f| f.name == *field)
            .map(|f| f.ty)
    }
}

fn parse(input: TokenStream) -> Result<Schema> {
    let mut c = Cursor::new("schema!", input, Span::call_site());
    if !c.eat_keyword("pub") {
        return c.unexpected("the schema header `pub Name;`");
    }
    let name = c.ident("the schema name")?;
    c.punct(';', "`;` after the schema name")?;
    let mut schema = Schema {
        name,
        relations: Vec::new(),
        statements: Vec::new(),
    };
    while !c.is_empty() {
        let ident = c.ident("`relation`, `closed relation`, or a statement")?;
        if ident == "closed" {
            if !c.eat_keyword("relation") {
                return c.unexpected("`relation` after `closed`");
            }
            schema.relations.push(parse_closed_relation(&mut c)?);
        } else if ident == "relation" {
            let name = c.ident("a relation name")?;
            let body = c.group(Delimiter::Brace, "a relation body `{ … }`")?;
            let fields = parse_fields(&mut c.of(&body))?;
            schema.relations.push(Relation {
                name,
                fields,
                closed: None,
            });
        } else {
            schema.statements.push(parse_statement(ident, &mut c)?);
        }
    }
    Ok(schema)
}

fn parse_fields(c: &mut Cursor) -> Result<Vec<Field>> {
    let mut fields: Vec<Field> = Vec::new();
    while !c.is_empty() {
        let name = c.ident("a field name")?;
        if !c.peek_punct(':') {
            return c.fail(
                name.span(),
                format!(
                    "unknown field modifier `{name}` — a field is `name: type` or \
                     `name: type as NewType`"
                ),
            );
        }
        c.next();
        fields.push(parse_field(name, c)?);
        c.list_separator()?;
    }
    Ok(fields)
}

fn parse_field(name: Ident, c: &mut Cursor) -> Result<Field> {
    let ty_name = c.ident("a type: bool, u64, i64, f64, uuid, str, bytes<N> or interval<E>")?;
    let ty = match ty_name.to_string().as_str() {
        "bool" => FieldTy::Bool,
        "u64" => FieldTy::U64,
        "i64" => FieldTy::I64,
        "f64" => FieldTy::F64,
        "uuid" => FieldTy::Uuid,
        "str" => FieldTy::Str,
        "bytes" => {
            if !c.peek_punct('<') {
                return c.fail(
                    ty_name.span(),
                    "unknown type `bytes` — write `bytes<N>`: the width is part of the type",
                );
            }
            c.next();
            let width = count(c, "the bytes<N> width")?;
            c.punct('>', "`>`")?;
            let Ok(width) = u16::try_from(width.0) else {
                return c.fail(width.1, "a bytes<N> width is at most 64");
            };
            FieldTy::FixedBytes(width)
        }
        "interval" => parse_interval(&name, c)?,
        other => return c.fail(ty_name.span(), format!("unknown type `{other}`")),
    };
    let newtype = if c.peek_keyword("as") {
        let as_span = c.peek_span();
        c.next();
        if matches!(ty, FieldTy::Bool | FieldTy::Str) {
            return c.fail(
                as_span,
                "`as NewType` applies to u64, i64, f64, uuid, bytes<N> and interval fields",
            );
        }
        Some(c.ident("a newtype name")?)
    } else {
        None
    };
    Ok(Field { name, ty, newtype })
}

fn parse_interval(name: &Ident, c: &mut Cursor) -> Result<FieldTy> {
    c.punct('<', "`<`")?;
    let element = c.ident("an interval element: u64, i64 or f64")?;
    let element = match element.to_string().as_str() {
        "u64" => IntervalElement::U64,
        "i64" => IntervalElement::I64,
        "f64" => IntervalElement::F64,
        other => {
            return c.fail(
                element.span(),
                format!("an interval element is u64, i64 or f64, found `{other}`"),
            );
        }
    };
    if !c.eat_punct(',') {
        c.punct('>', "`>` or `, width>`")?;
        return Ok(FieldTy::Interval(element));
    }
    let (width, span) = count(c, "the interval width")?;
    c.punct('>', "`>`")?;
    match element {
        IntervalElement::U64 => Ok(FieldTy::FixedInterval(FixedIntervalElement::U64, width)),
        IntervalElement::I64 => Ok(FieldTy::FixedInterval(FixedIntervalElement::I64, width)),
        IntervalElement::F64 => c.fail(
            span,
            format!(
                "field `{name}`: `interval<f64, w>` has no exact length on the dense line — \
                 write `interval<f64>`"
            ),
        ),
    }
}

/// A non-negative integer literal and its span.
fn count(c: &mut Cursor, what: &str) -> Result<(u64, Span)> {
    let int = c.int(what)?;
    match int.to_u64() {
        Some(value) => Ok((value, int.span)),
        None => c.fail(int.span, format!("{what} is a non-negative u64")),
    }
}

fn parse_closed_relation(c: &mut Cursor) -> Result<Relation> {
    let name = c.ident("a relation name")?;
    if !c.eat_keyword("as") {
        return c.unexpected(&format!(
            "`as NewType` — closed relation `{name}` needs a handle newtype"
        ));
    }
    let newtype = c.ident("the handle newtype's name")?;
    let fields = if c.peek_group(Delimiter::Brace) {
        let body = c.group(Delimiter::Brace, "a column block")?;
        parse_fields(&mut c.of(&body))?
    } else {
        Vec::new()
    };
    for field in &fields {
        if field.name == "id" || field.name == "from_id" {
            return c.fail(
                field.name.span(),
                format!(
                    "closed relation `{name}` declares a column `{}` — the handle owns that name",
                    field.name
                ),
            );
        }
        if matches!(field.ty, FieldTy::Interval(IntervalElement::F64)) {
            return c.fail(
                field.name.span(),
                "a closed relation column cannot be `interval<f64>` — its accessor is a const \
                 fn and a dense interval has no const constructor; use two f64 columns",
            );
        }
    }
    c.punct('=', "`=` before the extension")?;
    let extension = c.group(Delimiter::Brace, "the extension `{ … }`")?;
    let rows = parse_extension(&name, &fields, &mut c.of(&extension))?;
    c.punct(';', "`;` after the extension")?;
    Ok(Relation {
        name,
        fields,
        closed: Some(Closed { newtype, rows }),
    })
}

fn parse_extension(name: &Ident, fields: &[Field], c: &mut Cursor) -> Result<Vec<ClosedRow>> {
    let mut rows: Vec<ClosedRow> = Vec::new();
    while !c.is_empty() {
        let handle = c.ident("a handle")?;
        let mut entries: Vec<(Ident, Literal)> = Vec::new();
        if c.peek_group(Delimiter::Brace) {
            let block = c.group(Delimiter::Brace, "a row's column block")?;
            let mut row = c.of(&block);
            while !row.is_empty() {
                let column = row.ident("a column name")?;
                row.punct(':', "`:`")?;
                let literal = parse_literal(&mut row)?;
                if !fields.iter().any(|f| f.name == column) {
                    return c.fail(
                        column.span(),
                        format!("closed relation `{name}` has no column `{column}`"),
                    );
                }
                if entries.iter().any(|(seen, _)| *seen == column) {
                    return c.fail(
                        column.span(),
                        format!("row `{handle}` supplies the column `{column}` twice"),
                    );
                }
                entries.push((column, literal));
                row.list_separator()?;
            }
        }
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            let Some(index) = entries.iter().position(|(column, _)| *column == field.name) else {
                return c.fail(
                    handle.span(),
                    format!("row `{handle}` is missing the column `{}`", field.name),
                );
            };
            values.push(entries.swap_remove(index).1);
        }
        rows.push(ClosedRow { handle, values });
        c.list_separator()?;
    }
    Ok(rows)
}

fn parse_literal(c: &mut Cursor) -> Result<Literal> {
    if c.peek_lit() {
        return Ok(Literal::Lit(c.lit()?));
    }
    Ok(Literal::Handle(c.ident("a literal or a handle")?))
}

fn parse_literals(c: &mut Cursor) -> Result<Literals> {
    if !c.peek_group(Delimiter::Brace) {
        return Ok(Literals::One(parse_literal(c)?));
    }
    let group = c.group(Delimiter::Brace, "a literal set")?;
    let mut set = c.of(&group);
    let mut literals = Vec::new();
    while !set.is_empty() {
        literals.push(parse_literal(&mut set)?);
        set.list_separator()?;
    }
    Ok(Literals::Many(literals, group.span()))
}

fn parse_side(relation: Ident, c: &mut Cursor) -> Result<Side> {
    let mut projection = Vec::new();
    while !c.is_empty() && !c.peek_punct('|') {
        projection.push(c.ident("a field name")?);
        if !c.eat_punct(',') && !c.is_empty() && !c.peek_punct('|') {
            return c.unexpected("`,` or `|`");
        }
    }
    let mut selection = Vec::new();
    if c.eat_punct('|') {
        while !c.is_empty() {
            let field = c.ident("a selected field name")?;
            c.punct('=', "`==`")?;
            c.punct('=', "`==`")?;
            let literals = parse_literals(c)?;
            selection.push(Binding { field, literals });
            c.list_separator()?;
        }
    }
    Ok(Side {
        relation,
        projection,
        selection,
    })
}

fn parse_statement_side(c: &mut Cursor) -> Result<Side> {
    let relation = c.ident("a relation name")?;
    let group = c.group(Delimiter::Parenthesis, "a projection list `(…)`")?;
    parse_side(relation, &mut c.of(&group))
}

fn parse_statement(relation: Ident, c: &mut Cursor) -> Result<Statement> {
    let group = c.group(Delimiter::Parenthesis, "a projection list `(…)`")?;
    let left = parse_side(relation, &mut c.of(&group))?;
    let statement = if c.eat_punct('-') {
        c.punct('>', "`->`")?;
        let right = c.ident("the key's relation")?;
        parse_key(c, left, &right)?
    } else if c.eat_punct('<') {
        c.punct('=', "`<=`")?;
        let weight = if c.peek_group(Delimiter::Bracket) {
            let group = c.group(Delimiter::Bracket, "a weight")?;
            let weight = parse_weight(&mut c.of(&group))?;
            if !c.peek_group(Delimiter::Brace) {
                return c.unexpected("a window `{lo..hi}` after the weight");
            }
            Some(weight)
        } else {
            None
        };
        if c.peek_group(Delimiter::Brace) {
            let group = c.group(Delimiter::Brace, "a window")?;
            let window = parse_window(&mut c.of(&group))?;
            let source = parse_statement_side(c)?;
            Statement::Capacity {
                target: left,
                weight: weight.unwrap_or(Weight::Unit),
                window,
                window_span: group.span(),
                source,
            }
        } else {
            Statement::Containment {
                source: left,
                target: parse_statement_side(c)?,
                bidirectional: false,
            }
        }
    } else if c.peek_punct('=') {
        c.next();
        c.punct('=', "`==`")?;
        Statement::Containment {
            source: left,
            target: parse_statement_side(c)?,
            bidirectional: true,
        }
    } else {
        return c.unexpected("`->`, `<=`, `<=[w]{lo..hi}` or `==`");
    };
    c.punct(';', "`;` after the statement")?;
    Ok(statement)
}

fn parse_key(c: &mut Cursor, left: Side, right: &Ident) -> Result<Statement> {
    if let Some(binding) = left.selection.first() {
        return c.fail(
            binding.field.span(),
            "a key takes no selection — the form is `R(X) -> R`",
        );
    }
    let fields = left
        .projection
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let rel = &left.relation;
    if *right != *rel {
        return c.fail(
            right.span(),
            format!(
                "a key closes over its own relation: `{rel}({fields}) -> {rel}`; \
                 `-> {right}` is not a key"
            ),
        );
    }
    Ok(Statement::Key {
        relation: left.relation,
        projection: left.projection,
    })
}

/// `Duration(field)` or `field`; `name.more` is refused at the dot.
fn measured_field(c: &mut Cursor, what: &str) -> Result<(Ident, bool)> {
    let name = c.ident(what)?;
    if name == "Duration" && c.peek_group(Delimiter::Parenthesis) {
        let group = c.group(Delimiter::Parenthesis, "`(field)`")?;
        let mut inner = c.of(&group);
        let field = inner.ident("the measured field")?;
        if !inner.is_empty() {
            return inner.unexpected("`)`");
        }
        return Ok((field, true));
    }
    if c.peek_punct('.') {
        let mut ahead = c.clone();
        ahead.next();
        if !ahead.peek_punct('.') {
            let dot = c.peek_span();
            return c.fail(
                dot,
                format!(
                    "`{name}.…` is refused — a weight or bound reads a field of its own row; \
                     state the join as a law and read a local column"
                ),
            );
        }
    }
    Ok((name, false))
}

fn parse_weight(c: &mut Cursor) -> Result<Weight> {
    if c.is_empty() {
        return c.fail(
            c.end(),
            "the weight `[]` names no measure — write `[field]` or `[Duration(field)]`",
        );
    }
    let (field, duration) = measured_field(c, "the weight field")?;
    if !c.is_empty() {
        return c.unexpected("`]`");
    }
    Ok(if duration {
        Weight::Duration(field)
    } else {
        Weight::Field(field)
    })
}

fn parse_bound(c: &mut Cursor, what: &str) -> Result<Bound> {
    if c.peek_ident() {
        let (field, duration) = measured_field(c, what)?;
        return Ok(if duration {
            Bound::Duration(field)
        } else {
            Bound::Field(field)
        });
    }
    Ok(Bound::Lit(count(c, what)?.0))
}

fn parse_window(c: &mut Cursor) -> Result<Window> {
    if c.is_empty() {
        return c.fail(
            c.end(),
            "the window `{}` names no bounds — write `{n}`, `{lo..hi}` or `{lo..*}`",
        );
    }
    if c.peek_punct('.') {
        let dot = c.peek_span();
        return c.fail(dot, "a window states its floor: write `{0..hi}`");
    }
    let lo = parse_bound(c, "the window's lower bound")?;
    if c.is_empty() {
        return Ok(Window::Exact(lo));
    }
    c.punct('.', "`..`")?;
    c.punct('.', "`..`")?;
    if c.is_empty() {
        return c.fail(
            c.end(),
            "a window states its ceiling: write `{lo..*}` for a floor",
        );
    }
    let window = if c.eat_punct('*') {
        Window::Floor(lo)
    } else {
        Window::Range(lo, parse_bound(c, "the window's upper bound")?)
    };
    if !c.is_empty() {
        return c.unexpected("`}`");
    }
    Ok(window)
}
