//! The token cursor, spanned errors, and the literal grammar `schema!` and
//! `query!` share. Every error carries the span of the token it is about.
use std::iter::Peekable;

use bumbledb_theory::{F64, Value};
use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree, token_stream};
use quote::{quote, quote_spanned};

/// One or more diagnostics, each at its own token.
pub(crate) struct Error {
    diagnostics: Vec<(Span, String)>,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            diagnostics: vec![(span, message.into())],
        }
    }

    pub(crate) fn many(diagnostics: Vec<(Span, String)>) -> Self {
        Self { diagnostics }
    }

    pub(crate) fn to_compile_error(&self) -> TokenStream {
        self.diagnostics
            .iter()
            .map(|(span, message)| {
                let mut message = Literal::string(message);
                message.set_span(*span);
                quote_spanned!(*span=> ::core::compile_error! { #message })
            })
            .collect()
    }
}

pub(crate) fn fail<T>(span: Span, message: impl Into<String>) -> Result<T> {
    Err(Error::new(span, message))
}

/// A peekable walk over one token stream. `end` is the span reported when
/// the stream runs out: the closing delimiter of the group it came from.
#[derive(Clone)]
pub(crate) struct Cursor {
    name: &'static str,
    tokens: Peekable<token_stream::IntoIter>,
    end: Span,
}

impl Cursor {
    pub(crate) fn new(name: &'static str, stream: TokenStream, end: Span) -> Self {
        Self {
            name,
            tokens: stream.into_iter().peekable(),
            end,
        }
    }

    pub(crate) fn of(&self, group: &Group) -> Self {
        Self::new(self.name, group.stream(), group.span_close())
    }

    pub(crate) fn fail<T>(&self, span: Span, message: impl std::fmt::Display) -> Result<T> {
        fail(span, format!("{}: {message}", self.name))
    }

    pub(crate) fn peek(&mut self) -> Option<&TokenTree> {
        self.tokens.peek()
    }

    pub(crate) fn next(&mut self) -> Option<TokenTree> {
        self.tokens.next()
    }

    pub(crate) fn is_empty(&mut self) -> bool {
        self.tokens.peek().is_none()
    }

    pub(crate) fn end(&self) -> Span {
        self.end
    }

    pub(crate) fn peek_span(&mut self) -> Span {
        self.tokens.peek().map_or(self.end, TokenTree::span)
    }

    pub(crate) fn peek_punct(&mut self, ch: char) -> bool {
        matches!(self.tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }

    pub(crate) fn peek_keyword(&mut self, keyword: &str) -> bool {
        matches!(self.tokens.peek(), Some(TokenTree::Ident(ident)) if ident == keyword)
    }

    pub(crate) fn peek_ident(&mut self) -> bool {
        matches!(self.tokens.peek(), Some(TokenTree::Ident(_)))
    }

    pub(crate) fn peek_group(&mut self, delimiter: Delimiter) -> bool {
        matches!(self.tokens.peek(), Some(TokenTree::Group(g)) if g.delimiter() == delimiter)
    }

    /// Whether the token after the next one opens a parenthesized group:
    /// the `Name(…)` shape.
    pub(crate) fn peek_call(&mut self) -> bool {
        let mut ahead = self.clone();
        ahead.next().is_some() && ahead.peek_group(Delimiter::Parenthesis)
    }

    pub(crate) fn eat_punct(&mut self, ch: char) -> bool {
        let found = self.peek_punct(ch);
        if found {
            self.tokens.next();
        }
        found
    }

    pub(crate) fn eat_keyword(&mut self, keyword: &str) -> bool {
        let found = self.peek_keyword(keyword);
        if found {
            self.tokens.next();
        }
        found
    }

    pub(crate) fn unexpected<T>(&mut self, what: &str) -> Result<T> {
        match self.tokens.next() {
            Some(other) => self.fail(other.span(), format!("expected {what}, found `{other}`")),
            None => self.fail(self.end, format!("expected {what}")),
        }
    }

    pub(crate) fn ident(&mut self, what: &str) -> Result<Ident> {
        match self.tokens.peek() {
            Some(TokenTree::Ident(_)) => {
                let Some(TokenTree::Ident(ident)) = self.tokens.next() else {
                    unreachable!("peeked an ident");
                };
                Ok(ident)
            }
            _ => self.unexpected(what),
        }
    }

    pub(crate) fn punct(&mut self, ch: char, what: &str) -> Result<Span> {
        match self.tokens.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == ch => {
                let span = p.span();
                self.tokens.next();
                Ok(span)
            }
            _ => self.unexpected(what),
        }
    }

    pub(crate) fn group(&mut self, delimiter: Delimiter, what: &str) -> Result<Group> {
        match self.tokens.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == delimiter => {
                let Some(TokenTree::Group(group)) = self.tokens.next() else {
                    unreachable!("peeked a group");
                };
                Ok(group)
            }
            _ => self.unexpected(what),
        }
    }

    /// Consumes an optional `,` between list items; anything else that is
    /// not the end of the list is an error.
    pub(crate) fn list_separator(&mut self) -> Result<()> {
        if self.eat_punct(',') || self.is_empty() {
            return Ok(());
        }
        self.unexpected("`,`")
    }

    /// Whether a literal starts here: a literal token, `-` before one, or
    /// `true`/`false`.
    pub(crate) fn peek_lit(&mut self) -> bool {
        match self.tokens.peek() {
            Some(TokenTree::Literal(_)) => true,
            Some(TokenTree::Ident(ident)) => ident == "true" || ident == "false",
            Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
                let mut ahead = self.clone();
                ahead.next();
                matches!(ahead.peek(), Some(TokenTree::Literal(_)))
            }
            _ => false,
        }
    }

    /// One literal: `true`, `false`, an integer, a float, `a..b` over
    /// integers or floats, `"str"`, or `b"bytes"`.
    pub(crate) fn lit(&mut self) -> Result<Lit> {
        let start = self.peek_span();
        if let Some(TokenTree::Ident(ident)) = self.tokens.peek() {
            let value = match ident.to_string().as_str() {
                "true" => true,
                "false" => false,
                _ => return self.unexpected("a literal"),
            };
            self.tokens.next();
            return Ok(Lit {
                kind: LitKind::Bool(value),
                span: start,
            });
        }
        let first = self.number_or_text()?;
        let kind = match first {
            Scalar::Text(kind) => kind,
            Scalar::Int(start) if self.peek_punct('.') => {
                self.range_dots()?;
                match self.number_or_text()? {
                    Scalar::Int(end) => LitKind::IntInterval(start, end),
                    _ => {
                        return self.fail(start.span, "an interval spells both bounds as integers");
                    }
                }
            }
            Scalar::Float(start_value, span) if self.peek_punct('.') => {
                self.range_dots()?;
                match self.number_or_text()? {
                    Scalar::Float(end, _) => LitKind::FloatInterval(start_value, end),
                    _ => {
                        return self.fail(
                            span,
                            "a float interval spells both bounds as floats — write `0.0..1.0`",
                        );
                    }
                }
            }
            Scalar::Int(int) => LitKind::Int(int),
            Scalar::Float(value, _) => LitKind::Float(value),
        };
        Ok(Lit { kind, span: start })
    }

    /// One integer literal, with an optional `-`, never continued into an
    /// interval.
    pub(crate) fn int(&mut self, what: &str) -> Result<Int> {
        if !self.peek_lit() {
            return self.unexpected(what);
        }
        let span = self.peek_span();
        match self.number_or_text()? {
            Scalar::Int(int) => Ok(int),
            Scalar::Float(..) | Scalar::Text(_) => self.fail(span, format!("expected {what}")),
        }
    }

    fn range_dots(&mut self) -> Result<()> {
        self.punct('.', "`..`")?;
        self.punct('.', "`..`")?;
        Ok(())
    }

    /// An integer (with an optional `-`), a float, or a string literal.
    fn number_or_text(&mut self) -> Result<Scalar> {
        let negative = self.peek_punct('-');
        let sign = self.peek_span();
        if negative {
            self.tokens.next();
        }
        let literal = match self.tokens.peek() {
            Some(TokenTree::Literal(_)) => {
                let Some(TokenTree::Literal(literal)) = self.tokens.next() else {
                    unreachable!("peeked a literal");
                };
                literal
            }
            _ => return self.unexpected("a literal"),
        };
        let span = if negative {
            sign.join(literal.span()).unwrap_or(sign)
        } else {
            literal.span()
        };
        let text = literal.to_string();
        if let Some(body) = text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
            if negative {
                return self.fail(span, "`-` applies to numbers only");
            }
            let bytes = unescape(body, true);
            let text = String::from_utf8(bytes).expect("rustc lexed a UTF-8 string literal");
            return Ok(Scalar::Text(LitKind::Str(text)));
        }
        if let Some(body) = text.strip_prefix("b\"").and_then(|t| t.strip_suffix('"')) {
            if negative {
                return self.fail(span, "`-` applies to numbers only");
            }
            return Ok(Scalar::Text(LitKind::Bytes(unescape(body, false))));
        }
        if let Some(value) = float_text(&text) {
            let Ok(value) = value else {
                return self.fail(span, format!("invalid f64 literal `{text}`"));
            };
            if !value.is_finite() {
                return self.fail(span, "f64 literals must be finite");
            }
            return Ok(Scalar::Float(
                F64::from(if negative { -value } else { value }),
                span,
            ));
        }
        let Some((magnitude, suffix)) = int_text(&text) else {
            return self.fail(
                span,
                format!(
                    "`{text}` is not a supported literal — integers are u64/i64 with an \
                     optional 0x/0o/0b radix, `_` separators and a u64/i64 suffix"
                ),
            );
        };
        Ok(Scalar::Int(Int {
            negative,
            magnitude,
            suffix,
            span,
        }))
    }
}

enum Scalar {
    Int(Int),
    Float(F64, Span),
    Text(LitKind),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Suffix {
    U64,
    I64,
}

/// An integer literal as written: sign, magnitude, optional type suffix.
#[derive(Clone, Copy)]
pub(crate) struct Int {
    pub(crate) negative: bool,
    pub(crate) magnitude: u128,
    pub(crate) suffix: Option<Suffix>,
    pub(crate) span: Span,
}

impl Int {
    pub(crate) fn to_u64(self) -> Option<u64> {
        if self.negative || self.suffix == Some(Suffix::I64) {
            return None;
        }
        u64::try_from(self.magnitude).ok()
    }

    pub(crate) fn to_i64(self) -> Option<i64> {
        if self.suffix == Some(Suffix::U64) {
            return None;
        }
        let magnitude = i128::try_from(self.magnitude).ok()?;
        i64::try_from(if self.negative { -magnitude } else { magnitude }).ok()
    }
}

pub(crate) enum LitKind {
    Bool(bool),
    Int(Int),
    Float(F64),
    IntInterval(Int, Int),
    FloatInterval(F64, F64),
    Str(String),
    Bytes(Vec<u8>),
}

pub(crate) struct Lit {
    pub(crate) kind: LitKind,
    pub(crate) span: Span,
}

/// `Some` when the token is spelled as a float: a `.`, an exponent, or the
/// `f64` suffix on a decimal literal.
fn float_text(text: &str) -> Option<std::result::Result<f64, std::num::ParseFloatError>> {
    if !text.starts_with(|c: char| c.is_ascii_digit())
        || text.starts_with("0x")
        || text.starts_with("0o")
        || text.starts_with("0b")
        || !(text.contains(['.', 'e', 'E']) || text.ends_with("f64"))
    {
        return None;
    }
    let digits = text.strip_suffix("f64").unwrap_or(text).replace('_', "");
    Some(digits.parse::<f64>())
}

fn int_text(text: &str) -> Option<(u128, Option<Suffix>)> {
    let (digits, suffix) = if let Some(digits) = text.strip_suffix("u64") {
        (digits, Some(Suffix::U64))
    } else if let Some(digits) = text.strip_suffix("i64") {
        (digits, Some(Suffix::I64))
    } else {
        (text, None)
    };
    let (radix, digits) = match digits.as_bytes() {
        [b'0', b'x', ..] => (16, &digits[2..]),
        [b'0', b'o', ..] => (8, &digits[2..]),
        [b'0', b'b', ..] => (2, &digits[2..]),
        _ => (10, digits),
    };
    let digits = digits.replace('_', "");
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    Some((u128::from_str_radix(&digits, radix).ok()?, suffix))
}

/// Decodes the escapes of a string (`unicode`) or byte-string literal body.
/// rustc already lexed the token, so every escape is well formed.
fn unescape(body: &str, unicode: bool) -> Vec<u8> {
    const LEXED: &str = "rustc lexed the literal";
    let mut out = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut utf8 = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            continue;
        }
        match chars.next().expect(LEXED) {
            'n' => out.push(b'\n'),
            'r' => out.push(b'\r'),
            't' => out.push(b'\t'),
            '\\' => out.push(b'\\'),
            '\'' => out.push(b'\''),
            '"' => out.push(b'"'),
            '0' => out.push(0),
            'x' => {
                let high = chars.next().and_then(|c| c.to_digit(16)).expect(LEXED);
                let low = chars.next().and_then(|c| c.to_digit(16)).expect(LEXED);
                out.push(u8::try_from(high * 16 + low).expect("two hex digits fit a byte"));
            }
            'u' if unicode => {
                assert_eq!(chars.next(), Some('{'), "{LEXED}");
                let mut code = 0u32;
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    if c != '_' {
                        code = code * 16 + c.to_digit(16).expect(LEXED);
                    }
                }
                let mut utf8 = [0u8; 4];
                let c = char::from_u32(code).expect(LEXED);
                out.extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            }
            // A backslash before a newline swallows it and the next line's
            // leading whitespace.
            '\n' => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            other => unreachable!("{LEXED}; found escape `\\{other}`"),
        }
    }
    out
}

pub(crate) fn uuid_tokens(bytes: &[u8; 16]) -> TokenStream {
    let bytes = Literal::byte_string(bytes);
    quote!(::bumbledb::Uuid::from_bytes(*#bytes))
}

pub(crate) fn f64_tokens(value: bumbledb_theory::F64) -> TokenStream {
    let bits = value.to_bits();
    quote!(::bumbledb::F64::from_bits(#bits))
}

/// A `Value` as the expression that builds it.
pub(crate) fn value_tokens(value: &Value) -> TokenStream {
    match value {
        Value::Bool(v) => quote!(::bumbledb::Value::Bool(#v)),
        Value::U64(v) => quote!(::bumbledb::Value::U64(#v)),
        Value::I64(v) => quote!(::bumbledb::Value::I64(#v)),
        Value::F64(v) => {
            let v = f64_tokens(*v);
            quote!(::bumbledb::Value::F64(#v))
        }
        Value::String(text) => {
            let text: &str = text;
            quote!(::bumbledb::Value::String(::std::boxed::Box::<str>::from(#text)))
        }
        Value::FixedBytes(bytes) => {
            let bytes = Literal::byte_string(bytes);
            quote!(::bumbledb::Value::FixedBytes(::std::boxed::Box::<[u8]>::from(&#bytes[..])))
        }
        Value::Uuid(id) => {
            let id = uuid_tokens(id.as_bytes());
            quote!(::bumbledb::Value::Uuid(#id))
        }
        Value::IntervalU64(interval) => {
            let (start, end) = interval.bounds();
            quote!(::bumbledb::Value::IntervalU64(
                ::bumbledb::Interval::<u64>::const_new(#start, #end).expect("a nonempty interval")
            ))
        }
        Value::IntervalI64(interval) => {
            let (start, end) = interval.bounds();
            quote!(::bumbledb::Value::IntervalI64(
                ::bumbledb::Interval::<i64>::const_new(#start, #end).expect("a nonempty interval")
            ))
        }
        Value::IntervalF64(interval) => {
            let (start, end) = interval.bounds();
            let (start, end) = (f64_tokens(start), f64_tokens(end));
            quote!(::bumbledb::Value::IntervalF64(
                ::bumbledb::Interval::<::bumbledb::F64>::new(#start, #end)
                    .expect("a nonempty interval")
            ))
        }
    }
}
