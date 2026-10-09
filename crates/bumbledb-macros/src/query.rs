//! `query!`: the query notation's parse and rule classification.
//! ```text
//! query     := import* interior* recblock? main
//! import    := 'use' derived '=' expr ';'     // splices a nonrecursive template
//! interior  := 'interior' derived '(' head ')' '|' body ';'
//! recblock  := 'rec' derived '(' head ')' '|' body ';'
//! main      := ('(' head ')' '|' body ';')+
//! head      := headterm (',' headterm)*
//! headterm  := var | [name ':'] (Sum|Mean|Min|Max|Pack '(' var ')' | Count)
//! body      := item (',' item)*
//! item      := atom | '!' atom | cond
//! cond      := term 'in' term | Allen '(' term ',' mask ',' term ')'
//!            | term cmp term | ('and' | 'or') '(' cond (',' cond)* ')'
//! atom      := Relation '(' binding,* ')' | derived '(' var,* ')'
//!            | derived '(' pbind,* ')' | 'interior' N '(' … ')'
//! binding   := field | field ':' var | field '==' value | field 'in' ?param
//! pbind     := N ':' var | N '==' value | N 'in' ?param
//! term      := var | ?param | literal
//! literal   := bool | int | int..int | float | float..float
//!            | uuid:"…" | "str" | b"bytes"
//! ```
//! Consecutive lines naming one interior or rec union into one stage; a rec
//! line whose body names the rec is a recursive arm, else a base arm.
//! Relations, fields and handles resolve through the constants `schema!`
//! emits on the theory; derived-table names are local to the expansion.
mod emit;
mod template;

use bumbledb_theory::{Interval, Uuid, Value};
use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};

use crate::lex::{Cursor, Int, LitKind, Result, Suffix, fail};

pub(crate) use template::expand_params;

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let mut c = Cursor::new("query!", input, Span::call_site());
    let theory = parse_theory(&mut c)?;
    let block = c.group(Delimiter::Brace, "the rule block `{ … }`")?;
    if !c.is_empty() {
        return c.unexpected("the end of the input after the rule block");
    }
    let mut rules = c.of(&block);
    let imports = parse_imports(&mut rules)?;
    let mut parsed = Vec::new();
    while !rules.is_empty() {
        parsed.push(parse_rule(&mut rules)?);
    }
    if parsed.is_empty() {
        return c.fail(block.span(), "a query needs at least one rule");
    }
    let query = classify(parsed, block.span())?;
    for name in query.derived_names() {
        if imports.iter().any(|import| import.name.text == name.text) {
            return c.fail(
                name.span,
                format!(
                    "`{}` is already a `use` import — derived names are unique",
                    name.text
                ),
            );
        }
    }
    emit::query(&theory, &imports, &query)
}

fn refuse<T>(span: Span, message: impl std::fmt::Display) -> Result<T> {
    fail(span, format!("query!: {message}"))
}

/// `uuid:` starts a UUID literal; a bare `uuid` is an ordinary name.
fn peek_uuid(c: &Cursor) -> bool {
    let mut ahead = c.clone();
    ahead.eat_keyword("uuid") && ahead.peek_punct(':')
}

/// A name as written: an identifier or a numeric position/interior id.
#[derive(Clone)]
struct Name {
    text: String,
    span: Span,
}

impl Name {
    fn of(ident: &Ident) -> Self {
        Self {
            text: ident.to_string(),
            span: ident.span(),
        }
    }

    fn ident(&self) -> Ident {
        match self.text.strip_prefix("r#") {
            Some(raw) => Ident::new_raw(raw, self.span),
            None => Ident::new(&self.text, self.span),
        }
    }

    fn position(&self) -> Option<u16> {
        self.text.parse().ok()
    }

    fn is_lowercase(&self) -> bool {
        self.text.starts_with(|c: char| c.is_ascii_lowercase())
    }
}

enum Param {
    Named(Name),
    Index { index: u16, span: Span },
}

enum SelValue {
    Lit(Value),
    Param(Param),
    Handle {
        qualifier: Option<Name>,
        handle: Name,
    },
}

enum Term {
    Var(Name),
    Param(Param),
    Lit(Value),
}

enum Binding {
    Pun(Name),
    Var { label: Name, var: Name },
    Value { label: Name, value: SelValue },
    SetParam { label: Name, param: Param },
}

impl Binding {
    fn label(&self) -> &Name {
        match self {
            Self::Pun(label)
            | Self::Var { label, .. }
            | Self::Value { label, .. }
            | Self::SetParam { label, .. } => label,
        }
    }
}

struct Atom {
    source: Name,
    bindings: Vec<Binding>,
}

enum Leaf {
    Allen {
        lhs: Term,
        mask: Vec<Name>,
        rhs: Term,
    },
    Membership {
        element: Term,
        container: Term,
    },
    Cmp {
        op: CmpOp,
        lhs: Term,
        rhs: Term,
    },
}

#[derive(Clone, Copy)]
enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

enum Cond {
    Leaf(Leaf),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

enum Item {
    Atom(Atom),
    Negated(Atom),
    Cond(Cond),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AggOp {
    Sum,
    Mean,
    Min,
    Max,
    Pack,
}

enum HeadTerm {
    Var(Name),
    Count {
        label: Option<Name>,
        span: Span,
    },
    Agg {
        op: AggOp,
        over: Name,
        label: Option<Name>,
    },
}

struct Rule {
    head: Vec<HeadTerm>,
    items: Vec<Item>,
}

enum Intro {
    Bare,
    Interior(Name),
    Rec(Name),
}

struct InteriorGroup {
    name: Name,
    rules: Vec<Rule>,
}

struct RecGroup {
    name: Name,
    base: Vec<Rule>,
    rec: Vec<Rule>,
}

struct Query {
    interiors: Vec<InteriorGroup>,
    rec: Option<RecGroup>,
    main: Vec<Rule>,
}

impl Query {
    fn derived_names(&self) -> impl Iterator<Item = &Name> {
        self.interiors
            .iter()
            .map(|group| &group.name)
            .chain(self.rec.as_ref().map(|rec| &rec.name))
    }
}

/// One `use name = expr;` import: `expr` must evaluate to `&Query`.
struct Import {
    name: Name,
    expr: TokenStream,
}

/// The theory path (`Theory` or `crate::path::Theory`), kept as tokens.
fn parse_theory(c: &mut Cursor) -> Result<TokenStream> {
    let mut theory = TokenStream::new();
    loop {
        match c.peek() {
            Some(TokenTree::Ident(_) | TokenTree::Punct(_)) => {
                if c.peek_ident() || c.peek_punct(':') {
                    theory.extend(c.next());
                } else {
                    return c.unexpected("the shape `query!(Theory { rules })`");
                }
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => break,
            _ => return c.unexpected("the shape `query!(Theory { rules })`"),
        }
    }
    if theory.is_empty() {
        let span = c.peek_span();
        return c.fail(span, "name the theory first: `query!(Theory { rules })`");
    }
    Ok(theory)
}

fn parse_imports(c: &mut Cursor) -> Result<Vec<Import>> {
    let mut imports: Vec<Import> = Vec::new();
    while c.peek_keyword("use") {
        let keyword = c.ident("`use`")?;
        let name = Name::of(&c.ident("the imported template's local name")?);
        validate_derived_name(&name)?;
        if imports.iter().any(|import| import.name.text == name.text) {
            return c.fail(
                name.span,
                format!(
                    "`use {}` is already bound — derived names are unique",
                    name.text
                ),
            );
        }
        c.punct('=', "the import's `=`")?;
        let mut expr = TokenStream::new();
        while !c.peek_punct(';') {
            match c.next() {
                Some(token) => expr.extend([token]),
                None => return c.fail(keyword.span(), "a `use` import ends with `;`"),
            }
        }
        c.next();
        if expr.is_empty() {
            return c.fail(
                keyword.span(),
                "`use` takes a borrowed template — `use stats = &attempt_stats;`",
            );
        }
        imports.push(Import { name, expr });
    }
    Ok(imports)
}

fn validate_derived_name(name: &Name) -> Result<()> {
    if matches!(
        name.text.as_str(),
        "and" | "or" | "interior" | "rec" | "use"
    ) {
        return refuse(
            name.span,
            format!(
                "`{}` is reserved — `and`/`or` are the condition grammar, `interior`/`rec` \
                 introduce derived tables, `use` imports templates",
                name.text
            ),
        );
    }
    if !name.is_lowercase() {
        return refuse(
            name.span,
            format!(
                "derived-table names begin lowercase (`{}`) — UpperCamel names are relations",
                name.text
            ),
        );
    }
    Ok(())
}

/// `:-` is refused wherever a `:` could start it.
fn datalog_refusal<T>(span: Span) -> Result<T> {
    refuse(
        span,
        "`:-` is Datalog, not this notation — write `(head) | body;`",
    )
}

fn colon(c: &mut Cursor, what: &str) -> Result<()> {
    let span = c.punct(':', what)?;
    if c.peek_punct('-') {
        return datalog_refusal(span);
    }
    Ok(())
}

fn number(c: &mut Cursor, what: &str) -> Result<Name> {
    match c.next() {
        Some(TokenTree::Literal(literal)) => {
            let text = literal.to_string();
            if text.parse::<u16>().is_err() {
                return c.fail(literal.span(), format!("expected {what}, found `{text}`"));
            }
            Ok(Name {
                text,
                span: literal.span(),
            })
        }
        Some(other) => c.fail(other.span(), format!("expected {what}, found `{other}`")),
        None => c.fail(c.end(), format!("expected {what}")),
    }
}

fn parse_rule(c: &mut Cursor) -> Result<(Intro, Rule)> {
    let intro = if c.peek_ident() {
        let ident = c.ident("a rule")?;
        let name = Name::of(&ident);
        if c.peek_punct(':') {
            let span = c.peek_span();
            c.next();
            if c.peek_punct('-') {
                return datalog_refusal(span);
            }
            return c.fail(span, "expected the rule's head `(…)`");
        }
        match name.text.as_str() {
            "use" => {
                return c.fail(
                    name.span,
                    "`use` imports precede every rule — the order is imports, interiors, \
                     rec, main",
                );
            }
            "interior" => {
                let derived = if c.peek_ident() {
                    let derived = Name::of(&c.ident("an interior name")?);
                    validate_derived_name(&derived)?;
                    derived
                } else {
                    number(c, "an interior name or id")?
                };
                Intro::Interior(derived)
            }
            "rec" => {
                if c.peek_group(Delimiter::Parenthesis) {
                    Intro::Rec(name)
                } else {
                    let derived = Name::of(&c.ident("a rec name")?);
                    validate_derived_name(&derived)?;
                    Intro::Rec(derived)
                }
            }
            _ => {
                validate_derived_name(&name)?;
                return c.fail(
                    name.span,
                    format!(
                        "named heads require `interior` or `rec` — write `interior {0}(…)` \
                         or `rec {0}(…)`",
                        name.text
                    ),
                );
            }
        }
    } else {
        Intro::Bare
    };
    let head_group = c.group(Delimiter::Parenthesis, "a rule head `(…)`")?;
    let head = parse_separated(&mut c.of(&head_group), parse_head_term)?;
    if head.is_empty() {
        return c.fail(head_group.span(), "a head needs at least one term");
    }
    if c.peek_punct(':') {
        let span = c.peek_span();
        let mut ahead = c.clone();
        ahead.next();
        if ahead.peek_punct('-') {
            return datalog_refusal(span);
        }
    }
    c.punct('|', "`|` after the head")?;
    let mut items = Vec::new();
    loop {
        if c.peek_punct(';') {
            if items.is_empty() {
                let span = c.peek_span();
                return c.fail(span, "a rule body needs at least one atom");
            }
            c.next();
            break;
        }
        if c.is_empty() {
            return c.fail(c.end(), "a rule ends with `;`");
        }
        items.push(parse_item(c)?);
        if !c.eat_punct(',') && !c.peek_punct(';') {
            return c.unexpected("`,` or `;`");
        }
    }
    Ok((intro, Rule { head, items }))
}

fn parse_separated<T>(
    c: &mut Cursor,
    mut item: impl FnMut(&mut Cursor) -> Result<T>,
) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while !c.is_empty() {
        items.push(item(c)?);
        c.list_separator()?;
    }
    Ok(items)
}

enum Aggregate {
    Count,
    Fold(AggOp),
}

fn aggregate(name: &str) -> Option<Aggregate> {
    Some(Aggregate::Fold(match name {
        "Sum" => AggOp::Sum,
        "Mean" => AggOp::Mean,
        "Min" => AggOp::Min,
        "Max" => AggOp::Max,
        "Pack" => AggOp::Pack,
        "Count" => return Some(Aggregate::Count),
        _ => return None,
    }))
}

fn parse_agg(
    c: &mut Cursor,
    keyword: &Name,
    aggregate: Aggregate,
    label: Option<Name>,
) -> Result<HeadTerm> {
    let Aggregate::Fold(op) = aggregate else {
        return Ok(HeadTerm::Count {
            label,
            span: keyword.span,
        });
    };
    let group = c.group(Delimiter::Parenthesis, "the aggregate's argument `(var)`")?;
    let mut arg = c.of(&group);
    let over = Name::of(&arg.ident("a variable")?);
    if !arg.is_empty() {
        return arg.unexpected("`)` — an aggregate takes one variable");
    }
    Ok(HeadTerm::Agg { op, over, label })
}

fn parse_head_term(c: &mut Cursor) -> Result<HeadTerm> {
    if c.peek_punct('?') {
        let span = c.peek_span();
        return c.fail(
            span,
            "a ?param cannot appear in a head — params are inputs, not result columns",
        );
    }
    let name = Name::of(&c.ident("a head term")?);
    if c.peek_punct(':') {
        colon(c, "the head column's `:`")?;
        let agg = Name::of(&c.ident("an aggregate")?);
        let Some(op) = aggregate(&agg.text) else {
            return c.fail(
                agg.span,
                format!(
                    "`{}` is not an aggregate — a named head position takes \
                     Sum/Mean/Min/Max/Count/Pack",
                    agg.text
                ),
            );
        };
        return parse_agg(c, &agg, op, Some(name));
    }
    if let Some(op) = aggregate(&name.text) {
        return parse_agg(c, &name, op, None);
    }
    Ok(HeadTerm::Var(name))
}

fn parse_param(c: &mut Cursor) -> Result<Param> {
    let question = c.punct('?', "`?`")?;
    if c.peek_ident() {
        return Ok(Param::Named(Name::of(&c.ident("a param name")?)));
    }
    match c.next() {
        Some(TokenTree::Literal(literal)) => match literal.to_string().parse::<u16>() {
            Ok(index) => Ok(Param::Index {
                index,
                span: literal.span(),
            }),
            Err(_) => c.fail(
                literal.span(),
                format!("`?{literal}` is not a param name or index"),
            ),
        },
        _ => c.fail(question, "`?` starts a param — `?name` or `?N`"),
    }
}

/// A literal as the value it denotes. Unsuffixed integers are u64 unless
/// negative; an interval is i64 when either bound is.
fn literal(c: &mut Cursor) -> Result<Value> {
    if peek_uuid(c) {
        let keyword = c.ident("`uuid`")?;
        return uuid(c, keyword.span());
    }
    let lit = c.lit()?;
    let int = |c: &Cursor, int: Int, signed: bool| -> Result<Value> {
        let value = if signed {
            int.to_i64().map(Value::I64)
        } else {
            int.to_u64().map(Value::U64)
        };
        value.map_or_else(
            || {
                c.fail(
                    int.span,
                    format!(
                        "the literal does not fit {}",
                        if signed {
                            "i64"
                        } else {
                            "u64 (a negative literal is i64)"
                        }
                    ),
                )
            },
            Ok,
        )
    };
    let signed = |int: Int| {
        int.suffix
            .map_or(int.negative, |suffix| suffix == Suffix::I64)
    };
    Ok(match lit.kind {
        LitKind::Bool(value) => Value::Bool(value),
        LitKind::Int(value) => int(c, value, signed(value))?,
        LitKind::Float(value) => Value::F64(value),
        LitKind::IntInterval(start, end) => {
            let signed = signed(start) || signed(end);
            let interval = match (int(c, start, signed)?, int(c, end, signed)?) {
                (Value::I64(start), Value::I64(end)) => {
                    Interval::<i64>::new(start, end).map(Value::IntervalI64)
                }
                (Value::U64(start), Value::U64(end)) => {
                    Interval::<u64>::new(start, end).map(Value::IntervalU64)
                }
                _ => unreachable!("both bounds share one signedness"),
            };
            match interval {
                Some(value) => value,
                None => return empty_interval(c, lit.span),
            }
        }
        LitKind::FloatInterval(start, end) => match Interval::new(start, end) {
            Some(interval) => Value::IntervalF64(interval),
            None => return empty_interval(c, lit.span),
        },
        LitKind::Str(text) => Value::String(text.into_boxed_str()),
        LitKind::Bytes(bytes) => Value::FixedBytes(bytes.into_boxed_slice()),
    })
}

fn empty_interval<T>(c: &Cursor, span: Span) -> Result<T> {
    c.fail(
        span,
        "an interval literal is half-open and nonempty: start < end",
    )
}

fn uuid(c: &mut Cursor, keyword: Span) -> Result<Value> {
    colon(c, "`:` after `uuid`")?;
    let lit = match c.peek() {
        Some(TokenTree::Literal(_)) => c.lit()?,
        _ => return c.fail(keyword, "`uuid:` takes a quoted UUID — `uuid:\"…\"`"),
    };
    let LitKind::Str(text) = &lit.kind else {
        return c.fail(lit.span, "`uuid:` takes a quoted UUID — `uuid:\"…\"`");
    };
    match Uuid::parse_str(text) {
        Ok(id) => Ok(Value::Uuid(id)),
        Err(_) => c.fail(lit.span, "a UUID literal must have UUID syntax"),
    }
}

fn parse_sel_value(c: &mut Cursor) -> Result<SelValue> {
    if c.peek_punct('?') {
        return Ok(SelValue::Param(parse_param(c)?));
    }
    if c.peek_ident() && !c.peek_lit() && !peek_uuid(c) {
        let name = Name::of(&c.ident("a value")?);
        if c.peek_punct(':') {
            colon(c, "the handle path's `::`")?;
            c.punct(':', "the handle path's `::`")?;
            let handle = Name::of(&c.ident("a handle name")?);
            return Ok(SelValue::Handle {
                qualifier: Some(name),
                handle,
            });
        }
        return Ok(SelValue::Handle {
            qualifier: None,
            handle: name,
        });
    }
    Ok(SelValue::Lit(literal(c)?))
}

fn parse_label(c: &mut Cursor) -> Result<Name> {
    if matches!(c.peek(), Some(TokenTree::Literal(_))) {
        return number(c, "a field name or head position");
    }
    Ok(Name::of(&c.ident("a field name")?))
}

fn parse_binding(c: &mut Cursor) -> Result<Binding> {
    let label = parse_label(c)?;
    if c.peek_punct(':') {
        colon(c, "the binding's `:`")?;
        let var = Name::of(&c.ident("a variable")?);
        return Ok(Binding::Var { label, var });
    }
    if c.peek_punct('=') {
        c.punct('=', "`==`")?;
        c.punct('=', "`==`")?;
        return Ok(Binding::Value {
            label,
            value: parse_sel_value(c)?,
        });
    }
    if c.peek_keyword("in") {
        let keyword = c.ident("`in`")?;
        if !c.peek_punct('?') {
            return c.fail(
                keyword.span(),
                "a binding's `in` takes a ?param bound to a set; point-in-interval is a \
                 body item `?p in field`",
            );
        }
        return Ok(Binding::SetParam {
            label,
            param: parse_param(c)?,
        });
    }
    Ok(Binding::Pun(label))
}

fn parse_atom(c: &mut Cursor, source: Name) -> Result<Atom> {
    let group = c.group(Delimiter::Parenthesis, "the atom's bindings `(…)`")?;
    Ok(Atom {
        source,
        bindings: parse_separated(&mut c.of(&group), parse_binding)?,
    })
}

fn parse_term(c: &mut Cursor) -> Result<Term> {
    if c.peek_punct('?') {
        return Ok(Term::Param(parse_param(c)?));
    }
    if c.peek_ident() && !c.peek_lit() && !peek_uuid(c) {
        return Ok(Term::Var(Name::of(&c.ident("a term")?)));
    }
    Ok(Term::Lit(literal(c)?))
}

fn parse_mask(c: &mut Cursor) -> Result<Vec<Name>> {
    if c.peek_punct('?') {
        let span = c.peek_span();
        return c.fail(
            span,
            "an Allen mask is a literal: `INTERSECTS`, `MEETS`, a basic, or a `|` union",
        );
    }
    let mut names = vec![Name::of(&c.ident("a mask name")?)];
    while c.eat_punct('|') {
        names.push(Name::of(&c.ident("a mask name")?));
    }
    Ok(names)
}

fn parse_cmp_op(c: &mut Cursor) -> Result<CmpOp> {
    let (first, span) = match c.next() {
        Some(TokenTree::Punct(p)) => (p.as_char(), p.span()),
        Some(other) => {
            return c.fail(
                other.span(),
                format!("expected a comparison, found `{other}`"),
            );
        }
        None => return c.fail(c.end(), "expected a comparison"),
    };
    let eq = c.peek_punct('=');
    let op = match (first, eq) {
        ('=', true) => CmpOp::Eq,
        ('!', true) => CmpOp::Ne,
        ('<', true) => CmpOp::Le,
        ('>', true) => CmpOp::Ge,
        ('<', false) => return Ok(CmpOp::Lt),
        ('>', false) => return Ok(CmpOp::Gt),
        (':', _) if c.peek_punct('-') => return datalog_refusal(span),
        _ => return c.fail(span, format!("`{first}` is not a comparison operator")),
    };
    c.next();
    Ok(op)
}

fn finish_leaf(c: &mut Cursor, lhs: Term) -> Result<Leaf> {
    if c.eat_keyword("in") {
        return Ok(Leaf::Membership {
            element: lhs,
            container: parse_term(c)?,
        });
    }
    let op = parse_cmp_op(c)?;
    Ok(Leaf::Cmp {
        op,
        lhs,
        rhs: parse_term(c)?,
    })
}

fn parse_allen(c: &mut Cursor) -> Result<Leaf> {
    let group = c.group(Delimiter::Parenthesis, "Allen's three positions")?;
    let mut args = c.of(&group);
    let lhs = parse_term(&mut args)?;
    args.punct(',', "`,`")?;
    let mask = parse_mask(&mut args)?;
    args.punct(',', "`,`")?;
    let rhs = parse_term(&mut args)?;
    if !args.is_empty() {
        return args.unexpected("`)` — Allen takes exactly three positions");
    }
    Ok(Leaf::Allen { lhs, mask, rhs })
}

fn tree_refusal<T>(span: Span) -> Result<T> {
    refuse(
        span,
        "a condition tree takes comparisons only — atoms, negation and the binding \
         membership stay body items",
    )
}

fn parse_tree(c: &mut Cursor, name: &Name) -> Result<Vec<Cond>> {
    let group = c.group(Delimiter::Parenthesis, "the condition tree's conditions")?;
    let mut children = c.of(&group);
    if children.is_empty() {
        return c.fail(
            group.span(),
            format!("`{}(…)` takes at least one condition", name.text),
        );
    }
    parse_separated(&mut children, parse_cond)
}

fn parse_cond(c: &mut Cursor) -> Result<Cond> {
    if c.peek_punct('!') {
        let span = c.peek_span();
        return tree_refusal(span);
    }
    if c.peek_ident() && c.peek_call() {
        let name = Name::of(&c.ident("a condition")?);
        return match name.text.as_str() {
            "and" => Ok(Cond::And(parse_tree(c, &name)?)),
            "or" => Ok(Cond::Or(parse_tree(c, &name)?)),
            "Allen" => Ok(Cond::Leaf(parse_allen(c)?)),
            _ => tree_refusal(name.span),
        };
    }
    let lhs = parse_term(c)?;
    Ok(Cond::Leaf(finish_leaf(c, lhs)?))
}

/// `interior N(` — the nameless spelling of a derived-table atom.
fn peek_nameless_interior(c: &Cursor) -> bool {
    let mut ahead = c.clone();
    if !ahead.eat_keyword("interior") {
        return false;
    }
    matches!(ahead.next(), Some(TokenTree::Literal(_))) && ahead.peek_group(Delimiter::Parenthesis)
}

fn parse_source(c: &mut Cursor) -> Result<Name> {
    if peek_nameless_interior(c) {
        c.next();
        return number(c, "an interior id");
    }
    Ok(Name::of(&c.ident("an atom's relation")?))
}

fn parse_item(c: &mut Cursor) -> Result<Item> {
    if c.eat_punct('!') {
        let source = parse_source(c)?;
        return Ok(Item::Negated(parse_atom(c, source)?));
    }
    if c.peek_punct(':') {
        let span = c.peek_span();
        c.next();
        if c.peek_punct('-') {
            return datalog_refusal(span);
        }
        return c.fail(span, "expected an atom or a condition");
    }
    if peek_nameless_interior(c) {
        let source = parse_source(c)?;
        return Ok(Item::Atom(parse_atom(c, source)?));
    }
    if c.peek_ident() && c.peek_call() {
        let name = Name::of(&c.ident("an atom or a condition")?);
        match name.text.as_str() {
            "Allen" => return Ok(Item::Cond(Cond::Leaf(parse_allen(c)?))),
            "and" => return Ok(Item::Cond(Cond::And(parse_tree(c, &name)?))),
            "or" => return Ok(Item::Cond(Cond::Or(parse_tree(c, &name)?))),
            _ => {}
        }
        let atom = parse_atom(c, name)?;
        if c.peek_punct('=')
            || c.peek_punct('!')
            || c.peek_punct('<')
            || c.peek_punct('>')
            || c.peek_keyword("in")
        {
            return c.fail(
                atom.source.span,
                format!(
                    "`{}(…)` is an atom and cannot be compared",
                    atom.source.text
                ),
            );
        }
        return Ok(Item::Atom(atom));
    }
    let lhs = parse_term(c)?;
    Ok(Item::Cond(Cond::Leaf(finish_leaf(c, lhs)?)))
}

fn names_derived(rule: &Rule, derived: &str) -> bool {
    rule.items.iter().any(|item| match item {
        Item::Atom(atom) | Item::Negated(atom) => atom.source.text == derived,
        Item::Cond(_) => false,
    })
}

enum Phase {
    Interiors,
    Rec,
    Main,
}

fn classify(parsed: Vec<(Intro, Rule)>, block: Span) -> Result<Query> {
    let order = "the order is interiors, then rec, then main";
    let mut interiors: Vec<InteriorGroup> = Vec::new();
    let mut rec: Option<RecGroup> = None;
    let mut main: Vec<Rule> = Vec::new();
    let mut phase = Phase::Interiors;
    for (intro, rule) in parsed {
        match intro {
            Intro::Interior(name) => {
                match phase {
                    Phase::Rec => {
                        return refuse(
                            name.span,
                            format!("`interior` cannot follow `rec` — {order}"),
                        );
                    }
                    Phase::Main => {
                        return refuse(
                            name.span,
                            format!("`interior` cannot follow a main rule — {order}"),
                        );
                    }
                    Phase::Interiors => {}
                }
                if let Some(last) = interiors.last_mut()
                    && last.name.text == name.text
                {
                    last.rules.push(rule);
                    continue;
                }
                if interiors.iter().any(|group| group.name.text == name.text) {
                    return refuse(
                        name.span,
                        format!(
                            "interior `{0}` is not consecutive — write every `interior {0}(…)` \
                             line together",
                            name.text
                        ),
                    );
                }
                interiors.push(InteriorGroup {
                    name,
                    rules: vec![rule],
                });
            }
            Intro::Rec(name) => {
                if matches!(phase, Phase::Main) {
                    return refuse(
                        name.span,
                        format!("`rec` cannot follow a main rule — {order}"),
                    );
                }
                phase = Phase::Rec;
                if interiors.iter().any(|group| group.name.text == name.text) {
                    return refuse(
                        name.span,
                        format!(
                            "`{}` cannot be both `interior` and `rec` — derived names are unique",
                            name.text
                        ),
                    );
                }
                let self_name = if name.text == "rec" {
                    interiors.len().to_string()
                } else {
                    name.text.clone()
                };
                let recursive = names_derived(&rule, &self_name);
                let group = match &mut rec {
                    None => rec.insert(RecGroup {
                        name,
                        base: Vec::new(),
                        rec: Vec::new(),
                    }),
                    Some(group) if group.name.text == name.text => group,
                    Some(_) => {
                        return refuse(name.span, "a query has at most one `rec` name");
                    }
                };
                if recursive {
                    group.rec.push(rule);
                } else {
                    group.base.push(rule);
                }
            }
            Intro::Bare => {
                phase = Phase::Main;
                main.push(rule);
            }
        }
    }
    if main.is_empty() {
        return refuse(
            block,
            "a query needs a main rule `(head) | body;` — `interior` and `rec` declare \
             derived tables",
        );
    }
    if let Some(group) = &rec {
        let name = &group.name.text;
        if group.base.is_empty() {
            return refuse(
                group.name.span,
                format!("`rec {name}` has no base arm — a line whose body does not name `{name}`"),
            );
        }
        if group.rec.is_empty() {
            return refuse(
                group.name.span,
                format!("`rec {name}` has no recursive arm — a line whose body names `{name}`"),
            );
        }
    }
    Ok(Query {
        interiors,
        rec,
        main,
    })
}
