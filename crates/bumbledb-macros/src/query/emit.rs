//! `query!` output: the IR value, built from tokens that resolve relations,
//! fields and handles through the theory's `schema!` constants.
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote};

use super::{
    AggOp, Atom, Binding, CmpOp, Cond, HeadTerm, Import, Item, Leaf, Name, Param, Query, Rule,
    SelValue, Term, refuse,
};
use crate::lex::{Result, value_tokens};

pub(super) fn query(
    theory: &TokenStream,
    imports: &[Import],
    query: &Query,
) -> Result<TokenStream> {
    let locals = Locals::new();
    let rec_name = query.rec.as_ref().map(|rec| {
        if rec.name.text == "rec" {
            query.interiors.len().to_string()
        } else {
            rec.name.text.clone()
        }
    });
    let mut emitter = Emitter {
        theory,
        params: Params::default(),
        derived: derived_roster(imports, query, rec_name.clone(), &locals),
    };
    let with_imports = !imports.is_empty();
    let prelude = if with_imports {
        import_prelude(imports, &locals)
    } else {
        TokenStream::new()
    };
    let mut lets = TokenStream::new();
    let mut stages = Vec::new();
    let interiors = &locals.interiors;
    for (index, group) in query.interiors.iter().enumerate() {
        let rules = group
            .rules
            .iter()
            .map(|rule| emitter.rule(rule))
            .collect::<Result<Vec<_>>>()?;
        if with_imports {
            lets.extend(quote! {
                #interiors.push(::bumbledb::Interior { rules: ::std::vec![#(#rules),*] });
            });
        } else {
            let stage = format_ident!("interior{index}", span = Span::mixed_site());
            lets.extend(quote!(let #stage = ::std::vec![#(#rules),*];));
            stages.push(quote!(::bumbledb::Interior { rules: #stage }));
        }
    }
    let rec = match (&query.rec, rec_name) {
        (Some(group), Some(rec_name)) => {
            let base = group
                .base
                .iter()
                .map(|rule| emitter.rec_rule(rule))
                .collect::<Result<Vec<_>>>()?;
            let step = group
                .rec
                .iter()
                .map(|rule| emitter.rec_step(rule, &rec_name))
                .collect::<Result<Vec<_>>>()?;
            let (base, step) = (non_empty(&base), non_empty(&step));
            Some(quote!(::bumbledb::Rec { base: #base, rec: #step }))
        }
        _ => None,
    };
    let main = query
        .main
        .iter()
        .map(|rule| emitter.rule(rule))
        .collect::<Result<Vec<_>>>()?;
    let interiors_expr = if with_imports {
        quote!(#interiors)
    } else {
        quote!(::std::vec![#(#stages),*])
    };
    let (rules, head) = (&locals.rules, &locals.head);
    let build = match rec {
        None => quote!(::bumbledb::Query::cq(#interiors_expr, #head, #rules)),
        Some(rec) => quote!(::bumbledb::Query::reach(#interiors_expr, #rec, #head, #rules)),
    };
    let query_expr = quote!({
        #prelude
        #lets
        let #rules = ::std::vec![#(#main),*];
        let #head = ::bumbledb::Rule::head(&#rules[0]);
        #build
    });
    super::template::wrap(&query_expr, &emitter.params, &query.main[0].head)
}

/// Hygienic names for the expansion's own locals, so spliced `use`
/// expressions can neither see nor shadow them.
struct Locals {
    interiors: Ident,
    base: Ident,
    rules: Ident,
    head: Ident,
}

impl Locals {
    fn new() -> Self {
        let local = |name: &str| Ident::new(name, Span::mixed_site());
        Self {
            interiors: local("interiors"),
            base: local("base"),
            rules: local("rules"),
            head: local("head"),
        }
    }
}

fn non_empty(items: &[TokenStream]) -> TokenStream {
    let (first, rest) = items
        .split_first()
        .map_or((None, &[][..]), |(f, r)| (Some(f), r));
    quote!(::bumbledb::NonEmpty::new(#first, ::std::vec![#(#rest),*]))
}

/// A derived table in scope: its local name and the `InteriorId` expression
/// addressing it.
struct Derived {
    name: String,
    id: TokenStream,
}

/// Imports first (their head stage ids are runtime values), then declared
/// interiors, then the rec. Imports splice stages ahead of the declared
/// ones, so declared ids shift by the runtime base.
fn derived_roster(
    imports: &[Import],
    query: &Query,
    rec_name: Option<String>,
    locals: &Locals,
) -> Vec<Derived> {
    let base = &locals.base;
    let declared = |index: usize| {
        let index = Literal::usize_unsuffixed(index);
        if imports.is_empty() {
            quote!(#index)
        } else {
            quote!(#base + #index)
        }
    };
    let mut derived: Vec<Derived> = imports
        .iter()
        .enumerate()
        .map(|(index, import)| {
            let id = import_ident(index);
            Derived {
                name: import.name.text.clone(),
                id: quote!(#id),
            }
        })
        .collect();
    for (index, group) in query.interiors.iter().enumerate() {
        derived.push(Derived {
            name: group.name.text.clone(),
            id: declared(index),
        });
    }
    if let Some(name) = rec_name {
        derived.push(Derived {
            name,
            id: declared(query.interiors.len()),
        });
    }
    derived
}

fn import_ident(index: usize) -> Ident {
    format_ident!("import{index}", span = Span::mixed_site())
}

/// Splices each import's interiors and main rules as derived stages, every
/// internal `Interior(id)` shifted by the stage offset; the imported template
/// is cloned, never mutated.
fn import_prelude(imports: &[Import], locals: &Locals) -> TokenStream {
    let interiors = &locals.interiors;
    let base = &locals.base;
    let splices = imports.iter().enumerate().map(|(index, import)| {
        let id = import_ident(index);
        let expr = &import.expr;
        let recursive = format!(
            "query!: `use {}` imports a recursive template; consume its result instead",
            import.name.text
        );
        let parameterized = format!(
            "query!: `use {}` imports a parameterized template; bind values in the importing \
             query",
            import.name.text
        );
        quote! {
            let #id: u32 = {
                let imported: &::bumbledb::Query = #expr;
                assert!(imported.rec().is_none(), #recursive);
                assert!(
                    imported.interiors().iter().all(|stage| stage.rules.iter().all(param_free))
                        && imported.rules().iter().all(param_free),
                    #parameterized
                );
                let offset = u32::try_from(#interiors.len()).expect("query!: too many interiors");
                for stage in imported.interiors() {
                    #interiors.push(::bumbledb::Interior {
                        rules: stage.rules.iter().map(|rule| shift_rule(rule, offset)).collect(),
                    });
                }
                #interiors.push(::bumbledb::Interior {
                    rules: imported.rules().iter().map(|rule| shift_rule(rule, offset)).collect(),
                });
                u32::try_from(#interiors.len() - 1).expect("query!: too many interiors")
            };
        }
    });
    quote! {
        fn shift_atom(atom: &::bumbledb::Atom, offset: u32) -> ::bumbledb::Atom {
            ::bumbledb::Atom {
                source: match atom.source {
                    ::bumbledb::AtomSource::Interior(id) => {
                        ::bumbledb::AtomSource::Interior(::bumbledb::InteriorId(id.0 + offset))
                    }
                    other => other,
                },
                bindings: atom.bindings.clone(),
            }
        }
        fn shift_rule(rule: &::bumbledb::Rule, offset: u32) -> ::bumbledb::Rule {
            ::bumbledb::Rule {
                finds: rule.finds.clone(),
                atoms: rule.atoms.iter().map(|atom| shift_atom(atom, offset)).collect(),
                negated: rule.negated.iter().map(|atom| shift_atom(atom, offset)).collect(),
                conditions: rule.conditions.clone(),
            }
        }
        fn term_free(term: &::bumbledb::Term) -> bool {
            !matches!(term, ::bumbledb::Term::Param(_) | ::bumbledb::Term::ParamSet(_))
        }
        fn cond_free(tree: &::bumbledb::ConditionTree) -> bool {
            match tree {
                ::bumbledb::ConditionTree::Leaf(cmp) => term_free(&cmp.lhs) && term_free(&cmp.rhs),
                ::bumbledb::ConditionTree::And(children)
                | ::bumbledb::ConditionTree::Or(children) => children.iter().all(cond_free),
            }
        }
        fn param_free(rule: &::bumbledb::Rule) -> bool {
            rule.atoms
                .iter()
                .chain(&rule.negated)
                .all(|atom| atom.bindings.iter().all(|binding| term_free(&binding.1)))
                && rule.conditions.iter().all(cond_free)
        }
        let mut #interiors: ::std::vec::Vec<::bumbledb::Interior> = ::std::vec::Vec::new();
        #(#splices)*
        let #base: u32 = u32::try_from(#interiors.len()).expect("query!: too many interiors");
    }
}

/// A named param's shape at its first use. An engine param slot is scalar
/// or set, never both, so one name keeps one shape.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParamShape {
    Scalar,
    Set,
}

impl ParamShape {
    fn describe(self) -> &'static str {
        match self {
            Self::Scalar => "a scalar (`== ?p` or comparison) position",
            Self::Set => "a set (`field in ?p`) position",
        }
    }
}

/// The query's param spelling: none yet, named, or positional — never both.
#[derive(Default)]
pub(super) enum Params {
    #[default]
    Empty,
    Named(Vec<(Name, ParamShape)>),
    Index,
}

impl Params {
    fn resolve(&mut self, param: &Param, shape: ParamShape) -> Result<u16> {
        let mixed = "named and positional ?params cannot mix — pick one spelling per query";
        match param {
            Param::Named(name) => {
                if matches!(self, Self::Index) {
                    return refuse(name.span, mixed);
                }
                if matches!(self, Self::Empty) {
                    *self = Self::Named(Vec::new());
                }
                let Self::Named(named) = self else {
                    unreachable!("set to Named above");
                };
                let seen = named.iter().position(|(seen, _)| seen.text == name.text);
                if let Some(existing) = seen.map(|position| named[position].1)
                    && existing != shape
                {
                    return refuse(
                        name.span,
                        format!(
                            "?{} is used as both {} and {} — use two names",
                            name.text,
                            existing.describe(),
                            shape.describe()
                        ),
                    );
                }
                let position = seen.unwrap_or_else(|| {
                    named.push((name.clone(), shape));
                    named.len() - 1
                });
                u16::try_from(position).or_else(|_| refuse(name.span, "too many params"))
            }
            Param::Index { index, span } => {
                if matches!(self, Self::Named(_)) {
                    return refuse(*span, mixed);
                }
                *self = Self::Index;
                Ok(*index)
            }
        }
    }
}

#[derive(Default)]
struct Scope {
    vars: Vec<String>,
    punned: Vec<String>,
}

impl Scope {
    fn intern(&mut self, name: &Name) -> Result<u16> {
        let position = self
            .vars
            .iter()
            .position(|var| *var == name.text)
            .unwrap_or_else(|| {
                self.vars.push(name.text.clone());
                self.vars.len() - 1
            });
        u16::try_from(position).or_else(|_| refuse(name.span, "too many variables"))
    }

    fn pun(&mut self, name: &Name) -> Result<u16> {
        if self.punned.contains(&name.text) {
            return refuse(
                name.span,
                format!(
                    "`{}` is punned twice — bind it explicitly: `field: var`",
                    name.text
                ),
            );
        }
        self.punned.push(name.text.clone());
        self.intern(name)
    }

    fn head_var(&self, name: &Name) -> Result<u16> {
        match self.vars.iter().position(|var| *var == name.text) {
            Some(position) => {
                u16::try_from(position).or_else(|_| refuse(name.span, "too many variables"))
            }
            None => refuse(
                name.span,
                format!(
                    "head variable `{}` is not bound in the rule body",
                    name.text
                ),
            ),
        }
    }
}

fn var(id: u16) -> TokenStream {
    let id = Literal::u16_unsuffixed(id);
    quote!(::bumbledb::Term::Var(::bumbledb::VarId(#id)))
}

fn param_term(id: u16, shape: ParamShape) -> TokenStream {
    let id = Literal::u16_unsuffixed(id);
    match shape {
        ParamShape::Scalar => quote!(::bumbledb::Term::Param(::bumbledb::ParamId(#id))),
        ParamShape::Set => quote!(::bumbledb::Term::ParamSet(::bumbledb::ParamId(#id))),
    }
}

fn literal_term(value: &bumbledb_theory::Value) -> TokenStream {
    let value = value_tokens(value);
    quote!(::bumbledb::Term::Literal(#value))
}

fn handle_term(handle_id: &TokenStream) -> TokenStream {
    quote!(::bumbledb::Term::Literal(::bumbledb::Value::U64(#handle_id)))
}

#[derive(Default)]
struct Body {
    scope: Scope,
    atoms: Vec<TokenStream>,
    negated: Vec<TokenStream>,
    conditions: Vec<TokenStream>,
}

struct Emitter<'a> {
    theory: &'a TokenStream,
    params: Params,
    derived: Vec<Derived>,
}

impl Emitter<'_> {
    fn param(&mut self, param: &Param, shape: ParamShape) -> Result<TokenStream> {
        Ok(param_term(self.params.resolve(param, shape)?, shape))
    }

    fn term(&mut self, scope: &mut Scope, term: &Term) -> Result<TokenStream> {
        match term {
            Term::Var(name) => Ok(var(scope.intern(name)?)),
            Term::Param(param) => self.param(param, ParamShape::Scalar),
            Term::Lit(value) => Ok(literal_term(value)),
        }
    }

    /// `Q::H`: the handle `H` of the theory's closed relation `Q`.
    fn qualified_handle(&self, qualifier: &Name, handle: &Name) -> TokenStream {
        let theory = self.theory;
        let (qualifier, handle) = (qualifier.ident(), handle.ident());
        handle_term(&quote!(#theory::#qualifier.__extension().#handle))
    }

    fn atom(&mut self, scope: &mut Scope, atom: &Atom) -> Result<TokenStream> {
        if let Some(derived) = self.derived.iter().find(|d| d.name == atom.source.text) {
            let id = derived.id.clone();
            let bindings = self.position_bindings(scope, atom)?;
            return Ok(quote!(::bumbledb::Atom {
                source: ::bumbledb::AtomSource::Interior(::bumbledb::InteriorId(#id)),
                bindings: ::std::vec![#(#bindings),*],
            }));
        }
        if atom.source.is_lowercase() || atom.source.position().is_some() {
            return refuse(
                atom.source.span,
                format!(
                    "unknown derived table `{}` — lowercase names are interiors or the rec; \
                     relations are UpperCamel",
                    atom.source.text
                ),
            );
        }
        let theory = self.theory;
        let source = atom.source.ident();
        let relation = quote!(#theory::#source);
        let mut bindings = Vec::with_capacity(atom.bindings.len());
        for binding in &atom.bindings {
            let label = binding.label();
            if label.position().is_some() {
                return refuse(
                    label.span,
                    format!(
                        "`{}` — numeric labels address derived-table positions; a relation's \
                         fields are named",
                        label.text
                    ),
                );
            }
            let field = label.ident();
            let term = match binding {
                Binding::Pun(name) => var(scope.pun(name)?),
                Binding::Var { var: name, .. } => var(scope.intern(name)?),
                Binding::Value {
                    value: SelValue::Lit(value),
                    ..
                } => literal_term(value),
                Binding::Value {
                    value: SelValue::Param(param),
                    ..
                } => self.param(param, ParamShape::Scalar)?,
                Binding::Value {
                    value:
                        SelValue::Handle {
                            qualifier: None,
                            handle,
                        },
                    ..
                } => {
                    let handle = handle.ident();
                    handle_term(&quote!(#relation.__references().#field.#handle))
                }
                Binding::Value {
                    value:
                        SelValue::Handle {
                            qualifier: Some(qualifier),
                            handle,
                        },
                    ..
                } => self.qualified_handle(qualifier, handle),
                Binding::SetParam { param, .. } => self.param(param, ParamShape::Set)?,
            };
            bindings.push(quote!((#relation.#field, #term)));
        }
        Ok(quote!(::bumbledb::Atom {
            source: ::bumbledb::AtomSource::Edb(#relation.relation()),
            bindings: ::std::vec![#(#bindings),*],
        }))
    }

    /// A derived-table atom's bindings: all bare (positions 0, 1, … in
    /// order) or all labelled with positions; never mixed.
    fn position_bindings(&mut self, scope: &mut Scope, atom: &Atom) -> Result<Vec<TokenStream>> {
        let mut bare = None;
        for binding in &atom.bindings {
            let label = binding.label();
            let is_bare = matches!(binding, Binding::Pun(_));
            let numeric = label.position().is_some();
            if is_bare && numeric {
                return refuse(
                    label.span,
                    "a bare derived-table binding is a variable (`reach(m, a)`); a position \
                     takes a form: `2: x`, `0 == …`, `0 in ?p`",
                );
            }
            if !is_bare && !numeric {
                return refuse(
                    label.span,
                    format!(
                        "`{}` — a derived-table atom addresses head positions, never names: \
                         `reach(m, a)` or `2: x`",
                        label.text
                    ),
                );
            }
            if *bare.get_or_insert(is_bare) != is_bare {
                return refuse(
                    label.span,
                    "bare variables and position labels cannot mix in one derived-table atom",
                );
            }
        }
        let mut bindings = Vec::with_capacity(atom.bindings.len());
        if bare == Some(true) {
            for (position, binding) in atom.bindings.iter().enumerate() {
                let term = var(scope.intern(binding.label())?);
                let position = Literal::usize_unsuffixed(position);
                bindings.push(quote!((::bumbledb::FieldId(#position), #term)));
            }
            return Ok(bindings);
        }
        let dense = !atom.bindings.is_empty()
            && atom.bindings.iter().enumerate().all(|(index, binding)| {
                matches!(binding, Binding::Var { label, .. }
                    if label.position() == u16::try_from(index).ok())
            });
        if dense {
            return refuse(
                atom.bindings[0].label().span,
                "dense in-order derived-table bindings are written bare: `reach(m, a)`; \
                 `i: v` is the sparse spelling",
            );
        }
        for binding in &atom.bindings {
            let term = match binding {
                Binding::Pun(_) => unreachable!("all bindings are labelled"),
                Binding::Var { var: name, .. } => var(scope.intern(name)?),
                Binding::Value {
                    value: SelValue::Lit(value),
                    ..
                } => literal_term(value),
                Binding::Value {
                    value: SelValue::Param(param),
                    ..
                } => self.param(param, ParamShape::Scalar)?,
                Binding::Value {
                    value:
                        SelValue::Handle {
                            qualifier: None,
                            handle,
                        },
                    ..
                } => {
                    return refuse(
                        handle.span,
                        "a bare handle resolves through its field, and a derived-table \
                         position has no field — qualify it: `Kind::Focus`",
                    );
                }
                Binding::Value {
                    value:
                        SelValue::Handle {
                            qualifier: Some(qualifier),
                            handle,
                        },
                    ..
                } => self.qualified_handle(qualifier, handle),
                Binding::SetParam { param, .. } => self.param(param, ParamShape::Set)?,
            };
            let position = Literal::u16_unsuffixed(binding.label().position().unwrap_or_default());
            bindings.push(quote!((::bumbledb::FieldId(#position), #term)));
        }
        Ok(bindings)
    }

    fn cond(&mut self, scope: &mut Scope, cond: &Cond) -> Result<TokenStream> {
        let leaf = |op: TokenStream, lhs: TokenStream, rhs: TokenStream| {
            quote!(::bumbledb::ConditionTree::Leaf(::bumbledb::Comparison {
                op: #op,
                lhs: #lhs,
                rhs: #rhs,
            }))
        };
        Ok(match cond {
            Cond::Leaf(Leaf::Allen { lhs, mask, rhs }) => {
                let (lhs, rhs) = (self.term(scope, lhs)?, self.term(scope, rhs)?);
                let mask = mask.iter().map(Name::ident);
                leaf(
                    quote!(::bumbledb::CmpOp::Allen { mask: #(::bumbledb::AllenMask::#mask)|* }),
                    lhs,
                    rhs,
                )
            }
            Cond::Leaf(Leaf::Membership { element, container }) => {
                let element = self.term(scope, element)?;
                let container = self.term(scope, container)?;
                leaf(quote!(::bumbledb::CmpOp::PointIn), container, element)
            }
            Cond::Leaf(Leaf::Cmp { op, lhs, rhs }) => {
                let (lhs, rhs) = (self.term(scope, lhs)?, self.term(scope, rhs)?);
                let op = match op {
                    CmpOp::Eq => quote!(Eq),
                    CmpOp::Ne => quote!(Ne),
                    CmpOp::Lt => quote!(Lt),
                    CmpOp::Le => quote!(Le),
                    CmpOp::Gt => quote!(Gt),
                    CmpOp::Ge => quote!(Ge),
                };
                leaf(quote!(::bumbledb::CmpOp::#op), lhs, rhs)
            }
            Cond::And(children) | Cond::Or(children) => {
                let children = children
                    .iter()
                    .map(|child| self.cond(scope, child))
                    .collect::<Result<Vec<_>>>()?;
                let variant = if matches!(cond, Cond::And(_)) {
                    quote!(And)
                } else {
                    quote!(Or)
                };
                quote!(::bumbledb::ConditionTree::#variant(::std::vec![#(#children),*]))
            }
        })
    }

    fn find(scope: &Scope, term: &HeadTerm) -> Result<TokenStream> {
        Ok(match term {
            HeadTerm::Var(name) => {
                let id = Literal::u16_unsuffixed(scope.head_var(name)?);
                quote!(::bumbledb::FindTerm::Var(::bumbledb::VarId(#id)))
            }
            HeadTerm::Count { .. } => quote!(::bumbledb::FindTerm::Count),
            HeadTerm::Agg { op, over, .. } => {
                let id = Literal::u16_unsuffixed(scope.head_var(over)?);
                let op = match op {
                    AggOp::Pack => {
                        return Ok(quote!(::bumbledb::FindTerm::Pack {
                            over: ::bumbledb::VarId(#id)
                        }));
                    }
                    AggOp::Sum => quote!(Sum),
                    AggOp::Mean => quote!(Mean),
                    AggOp::Min => quote!(Min),
                    AggOp::Max => quote!(Max),
                };
                quote!(::bumbledb::FindTerm::Aggregate {
                    op: ::bumbledb::FoldOp::#op,
                    over: ::bumbledb::VarId(#id),
                })
            }
        })
    }

    /// A rec head projects variables only: nothing aggregates through the
    /// recursive cycle.
    fn projection(scope: &Scope, head: &[HeadTerm]) -> Result<Vec<TokenStream>> {
        let refusal = "a rec head projects bound variables only — nothing aggregates through \
                       the recursive cycle (interiors may aggregate)";
        head.iter()
            .map(|term| match term {
                HeadTerm::Var(name) => {
                    let id = Literal::u16_unsuffixed(scope.head_var(name)?);
                    Ok(quote!(::bumbledb::VarId(#id)))
                }
                HeadTerm::Count { span, .. } => refuse(*span, refusal),
                HeadTerm::Agg { over, .. } => refuse(over.span, refusal),
            })
            .collect()
    }

    fn body(&mut self, rule: &Rule) -> Result<Body> {
        let mut body = Body::default();
        for item in &rule.items {
            match item {
                Item::Atom(atom) => body.atoms.push(self.atom(&mut body.scope, atom)?),
                Item::Negated(atom) => body.negated.push(self.atom(&mut body.scope, atom)?),
                Item::Cond(cond) => body.conditions.push(self.cond(&mut body.scope, cond)?),
            }
        }
        Ok(body)
    }

    fn rule(&mut self, rule: &Rule) -> Result<TokenStream> {
        let Body {
            scope,
            atoms,
            negated,
            conditions,
        } = self.body(rule)?;
        let finds = rule
            .head
            .iter()
            .map(|term| Self::find(&scope, term))
            .collect::<Result<Vec<_>>>()?;
        Ok(quote!(::bumbledb::Rule {
            finds: ::std::vec![#(#finds),*],
            atoms: ::std::vec![#(#atoms),*],
            negated: ::std::vec![#(#negated),*],
            conditions: ::std::vec![#(#conditions),*],
        }))
    }

    fn rec_rule(&mut self, rule: &Rule) -> Result<TokenStream> {
        if let Some(Item::Negated(atom)) = rule
            .items
            .iter()
            .find(|item| matches!(item, Item::Negated(_)))
        {
            return refuse(atom.source.span, "a rec rule negates nothing");
        }
        let Body {
            scope,
            atoms,
            conditions,
            ..
        } = self.body(rule)?;
        let finds = Self::projection(&scope, &rule.head)?;
        Ok(quote!(::bumbledb::RecRule {
            finds: ::std::vec![#(#finds),*],
            atoms: ::std::vec![#(#atoms),*],
            conditions: ::std::vec![#(#conditions),*],
        }))
    }

    fn rec_step(&mut self, rule: &Rule, rec_name: &str) -> Result<TokenStream> {
        let mut scope = Scope::default();
        let mut self_bindings = None;
        let (mut atoms, mut conditions) = (Vec::new(), Vec::new());
        for item in &rule.items {
            match item {
                Item::Negated(atom) => {
                    return refuse(atom.source.span, "a rec rule negates nothing");
                }
                Item::Cond(cond) => conditions.push(self.cond(&mut scope, cond)?),
                Item::Atom(atom) if atom.source.text == rec_name => {
                    if self_bindings.is_some() {
                        return refuse(atom.source.span, "a recursive arm has one self-atom");
                    }
                    self_bindings = Some(self.position_bindings(&mut scope, atom)?);
                }
                Item::Atom(atom) => atoms.push(self.atom(&mut scope, atom)?),
            }
        }
        let finds = Self::projection(&scope, &rule.head)?;
        let Some(self_bindings) = self_bindings else {
            return refuse(Span::call_site(), "a recursive arm names its rec");
        };
        Ok(quote!(::bumbledb::RecStep {
            finds: ::std::vec![#(#finds),*],
            self_bindings: ::std::vec![#(#self_bindings),*],
            atoms: ::std::vec![#(#atoms),*],
            conditions: ::std::vec![#(#conditions),*],
        }))
    }
}
