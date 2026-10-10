//! The typed template `query!` evaluates to, and `params!`. The template derefs
//! to `Query`, carries the param and column names, and for named params offers
//! `bind(params! { name: value, … })`: a typestate builder where an unknown name
//! is a missing method and a missing or repeated name is a type error. Value-to-
//! slot type agreement stays the engine's bind error at execution.
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream};
use quote::{format_ident, quote};

use super::emit::{ParamShape, Params};
use super::{AggOp, HeadTerm, Name, refuse};
use crate::lex::{Cursor, Result, fail};

/// Rust keywords cannot name a bind method.
const KEYWORDS: [&str; 53] = [
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "raw", "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

fn column_name(term: &HeadTerm) -> String {
    match term {
        HeadTerm::Var(name) => name.text.clone(),
        HeadTerm::Count { label, .. } => label
            .as_ref()
            .map_or_else(|| "Count".to_owned(), |label| label.text.clone()),
        HeadTerm::Agg { op, over, label } => label.as_ref().map_or_else(
            || {
                let op = match op {
                    AggOp::Sum => "Sum",
                    AggOp::Mean => "Mean",
                    AggOp::Min => "Min",
                    AggOp::Max => "Max",
                    AggOp::Pack => "Pack",
                };
                format!("{op}({})", over.text)
            },
            |label| label.text.clone(),
        ),
    }
}

pub(super) fn wrap(query: &TokenStream, params: &Params, head: &[HeadTerm]) -> Result<TokenStream> {
    let columns = head.iter().map(column_name);
    let named: &[(Name, ParamShape)] = match params {
        Params::Empty | Params::Index => &[],
        Params::Named(named) => named,
    };
    for (name, _) in named {
        if KEYWORDS.contains(&name.text.as_str()) {
            return refuse(
                name.span,
                format!(
                    "?{} is a Rust keyword and cannot name a bind method",
                    name.text
                ),
            );
        }
    }
    let param_names = named.iter().map(|(name, _)| &name.text);
    let bind = if matches!(params, Params::Index) {
        TokenStream::new()
    } else {
        bind(named)
    };
    Ok(quote!({
        #[allow(dead_code)]
        struct __BumbledbTemplate {
            query: ::bumbledb::Query,
        }
        impl ::core::ops::Deref for __BumbledbTemplate {
            type Target = ::bumbledb::Query;
            fn deref(&self) -> &::bumbledb::Query {
                &self.query
            }
        }
        #[allow(dead_code)]
        impl __BumbledbTemplate {
            const PARAM_NAMES: &'static [&'static str] = &[#(#param_names),*];
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            fn query(&self) -> &::bumbledb::Query {
                &self.query
            }
            fn into_query(self) -> ::bumbledb::Query {
                self.query
            }
            fn param_names(&self) -> &'static [&'static str] {
                Self::PARAM_NAMES
            }
            fn columns(&self) -> &'static [&'static str] {
                Self::COLUMNS
            }
        }
        #bind
        __BumbledbTemplate { query: #query }
    }))
}

/// The typestate builder: `__BumbledbParams<'a, P0, …>` with one state
/// parameter per named param, and one method per param defined only while
/// that param is unset.
fn bind(named: &[(Name, ParamShape)]) -> TokenStream {
    let count = named.len();
    let states: Vec<Ident> = (0..count)
        .map(|index| format_ident!("__P{index}"))
        .collect();
    let unset = (0..count).map(|_| quote!(__BumbledbUnset));
    let bound = (0..count).map(|_| quote!(__BumbledbBound));
    let nones = (0..count).map(|_| quote!(::core::option::Option::None));
    let methods = named.iter().enumerate().map(|(index, (name, shape))| {
        let others: Vec<&Ident> = states
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .map(|(_, state)| state)
            .collect();
        let state = |set: TokenStream| {
            let each = states.iter().enumerate().map(|(other, state)| {
                if other == index {
                    set.clone()
                } else {
                    quote!(#state)
                }
            });
            quote!(#(#each),*)
        };
        let before = state(quote!(__BumbledbUnset));
        let after = state(quote!(__BumbledbBound));
        let method = name.ident();
        let slot = Literal::usize_unsuffixed(index);
        let (param, arg) = match shape {
            ParamShape::Scalar => (
                quote!(value: impl __BumbledbScalarArg<'a>),
                quote!(__BumbledbScalarArg::into_arg(value)),
            ),
            ParamShape::Set => (
                quote!(values: &'a [::bumbledb::Value]),
                quote!(::bumbledb::ParamArg::Set(values)),
            ),
        };
        quote! {
            impl<'a, #(#others),*> __BumbledbParams<'a, #before> {
                #[allow(dead_code)]
                fn #method(mut self, #param) -> __BumbledbParams<'a, #after> {
                    self.args[#slot] = ::core::option::Option::Some(#arg);
                    __BumbledbParams {
                        args: self.args,
                        state: ::core::marker::PhantomData,
                    }
                }
            }
        }
    });
    let scalar = scalar_args();
    quote! {
        #[allow(dead_code)]
        struct __BumbledbUnset;
        #[allow(dead_code)]
        struct __BumbledbBound;
        #[allow(dead_code)]
        struct __BumbledbParams<'a, #(#states),*> {
            args: [::core::option::Option<::bumbledb::ParamArg<'a>>; #count],
            state: ::core::marker::PhantomData<(#(#states,)*)>,
        }
        #scalar
        #[allow(dead_code)]
        impl __BumbledbTemplate {
            fn bind<'a>(
                &self,
                fill: impl ::core::ops::FnOnce(
                    __BumbledbParams<'a, #(#unset),*>,
                ) -> __BumbledbParams<'a, #(#bound),*>,
            ) -> ::std::vec::Vec<::bumbledb::ParamArg<'a>> {
                let filled = fill(__BumbledbParams {
                    args: [#(#nones),*],
                    state: ::core::marker::PhantomData,
                });
                filled.args.into_iter().flatten().collect()
            }
        }
        #(#methods)*
    }
}

/// Scalar bind arguments: the `BindValue` roster plus `ParamArg` and
/// `BindValue` themselves.
fn scalar_args() -> TokenStream {
    let scalar = |ty: TokenStream, value: TokenStream| {
        quote! {
            impl<'a> __BumbledbScalarArg<'a> for #ty {
                fn into_arg(self) -> ::bumbledb::ParamArg<'a> {
                    ::bumbledb::ParamArg::Scalar(#value)
                }
            }
        }
    };
    let impls = [
        scalar(quote!(::bumbledb::BindValue<'a>), quote!(self)),
        scalar(quote!(bool), quote!(::bumbledb::BindValue::Bool(self))),
        scalar(quote!(u64), quote!(::bumbledb::BindValue::U64(self))),
        scalar(quote!(i64), quote!(::bumbledb::BindValue::I64(self))),
        scalar(
            quote!(::bumbledb::F64),
            quote!(::bumbledb::BindValue::F64(self)),
        ),
        scalar(
            quote!(f64),
            quote!(::bumbledb::BindValue::F64(::bumbledb::F64::from(self))),
        ),
        scalar(quote!(&'a str), quote!(::bumbledb::BindValue::Str(self))),
        scalar(
            quote!(&'a ::std::string::String),
            quote!(::bumbledb::BindValue::Str(self.as_str())),
        ),
        scalar(
            quote!(::bumbledb::Uuid),
            quote!(::bumbledb::BindValue::Uuid(self)),
        ),
        scalar(
            quote!(&'a [u8]),
            quote!(::bumbledb::BindValue::FixedBytes(self)),
        ),
        scalar(
            quote!(::bumbledb::Interval<u64>),
            quote!(::bumbledb::BindValue::IntervalU64(self.start(), self.end())),
        ),
        scalar(
            quote!(::bumbledb::Interval<i64>),
            quote!(::bumbledb::BindValue::IntervalI64(self.start(), self.end())),
        ),
        scalar(
            quote!(::bumbledb::Interval<::bumbledb::F64>),
            quote!(::bumbledb::BindValue::IntervalF64(self)),
        ),
    ];
    quote! {
        #[allow(dead_code)]
        trait __BumbledbScalarArg<'a> {
            fn into_arg(self) -> ::bumbledb::ParamArg<'a>;
        }
        impl<'a> __BumbledbScalarArg<'a> for ::bumbledb::ParamArg<'a> {
            fn into_arg(self) -> ::bumbledb::ParamArg<'a> {
                self
            }
        }
        #(#impls)*
    }
}

/// `params! { a: x, b: y }` → `|params| params.a(x).b(y)`: a builder
/// closure for a template's `bind`. Values keep their own tokens and spans.
pub(crate) fn expand_params(input: TokenStream) -> Result<TokenStream> {
    let mut c = Cursor::new("params!", input, Span::call_site());
    let builder = Ident::new("params", Span::mixed_site());
    let mut calls = TokenStream::new();
    let mut seen: Vec<Ident> = Vec::new();
    while !c.is_empty() {
        let name = c.ident("a param name")?;
        c.punct(':', "`:` after the param name")?;
        let mut value = TokenStream::new();
        while !c.is_empty() && !c.peek_punct(',') {
            value.extend(c.next());
        }
        if value.is_empty() {
            return fail(name.span(), "params!: an entry is `name: value`");
        }
        if KEYWORDS.contains(&name.to_string().as_str()) {
            return fail(
                name.span(),
                format!("params!: {name} is a Rust keyword and cannot name a bind method"),
            );
        }
        if seen.contains(&name) {
            return fail(
                name.span(),
                format!("params!: {name} is supplied twice — one value per param"),
            );
        }
        calls.extend([
            proc_macro2::TokenTree::Punct(Punct::new('.', Spacing::Alone)),
            proc_macro2::TokenTree::Ident(name.clone()),
            proc_macro2::TokenTree::Group(Group::new(Delimiter::Parenthesis, value)),
        ]);
        seen.push(name);
        c.list_separator()?;
    }
    Ok(quote!(|#builder| #builder #calls))
}
