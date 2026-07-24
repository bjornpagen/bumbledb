//! The notation conformance corpus (PRD-M4): one grammar, mechanically
//! refereed. Every case in `tests/notation-corpus/` is a
//! (notation ⇄ `ProgramIr` JSON) pair both hosts replay — this suite
//! `query!`-compiles each case's notation, proves it real against a `Db`
//! of the corpus theory, pins the render fixed point, and encodes the
//! lowered IR to exactly the JSON the TS SDK produces by
//! `JSON.stringify` of its `ProgramIr` value
//! (`ts/test/notation-corpus.test.ts` is the other replayer). The corpus
//! README states the law: a disagreement is a trophy, not a merge
//! conflict.
//!
//! The checked-in documents are byte-pinned: the whole file is
//! recomputed from the case table and compared byte-identical, so
//! editing any case's notation, normalized text, or program fails here.
//! Regenerate after adding a case:
//! `cargo test -p bumbledb-query regenerate_the_notation_corpus -- --ignored`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use bumbledb::ir::render::{render, render_program};
use bumbledb::ir::{HeadOp, HeadTerm};
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AggOp, Atom, AtomSource, CmpOp, Comparison, ConditionTree, Db, FindTerm, MaskTerm,
    PredicateDef, Program, Query, Rule, Schema, Term, Theory, Value,
};
use bumbledb_query::query;

mod common;
use common::TempDir;

/// The corpus theory: the benchmark ledger, transcribed declaration for
/// declaration (the third transcription — schemas are data and travel as
/// text; `tests/notation.rs` carries the first). The ONE schema every
/// corpus case queries; the TS replayer declares the identical theory
/// structurally and `tests/notation-corpus/schema-fingerprint.txt` pins
/// the two constructions to one engine fingerprint.
mod ledger {
    bumbledb::schema! {
        pub Ledger;

        closed relation Currency as CurrencyId = { Usd, Eur, Gbp };
        closed relation Source as SourceId = { Manual, Import, System };
        closed relation Tag as TagId = { Fee, Rebate, Adjustment };

        relation Holder {
            id: u64 as HolderId, fresh,
            name: str,
        }
        relation Account {
            id: u64 as AccountId, fresh,
            holder: u64 as HolderId,
            currency: u64 as CurrencyId,
        }
        relation Instrument {
            id: u64 as InstrumentId, fresh,
            symbol: str,
        }
        relation JournalEntry {
            id: u64 as JournalEntryId, fresh,
            source: u64 as SourceId,
            created_at: i64,
        }
        relation Posting {
            id: u64 as PostingId, fresh,
            entry: u64 as JournalEntryId,
            account: u64 as AccountId,
            instrument: u64 as InstrumentId,
            amount: i64,
            at: i64,
        }
        relation PostingTag {
            posting: u64 as PostingId,
            tag: u64 as TagId,
        }
        relation Org {
            id: u64 as OrgId, fresh,
            name: str,
        }
        relation OrgParent {
            child: u64 as OrgId,
            parent: u64 as OrgId,
        }
        relation Mandate {
            account: u64 as AccountId,
            org: u64 as OrgId,
            active: interval<i64>,
        }

        Account(holder)      <= Holder(id);
        Account(currency)    <= Currency(id);
        Posting(entry)       <= JournalEntry(id);
        Posting(account)     <= Account(id);
        Posting(instrument)  <= Instrument(id);
        PostingTag(posting)  <= Posting(id);
        PostingTag(tag)      <= Tag(id);
        JournalEntry(source) <= Source(id);
        OrgParent(child)     <= Org(id);
        OrgParent(parent)    <= Org(id);
        Mandate(account)     <= Account(id);
        Mandate(org)         <= Org(id);
        Mandate(account, active) -> Mandate;
    }
}

// `Currency` rides along for the handle spellings: bare `Usd` resolves
// through the host enum in scope; `Currency::Usd` is the qualified form.
use ledger::{Currency, Ledger};

/// A process-unique store tag: several tests build the case table in
/// parallel threads, so a case's temp store may never collide by name.
fn unique_tag(tag: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("corpus-{tag}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Renders after proving the query real: prepared against a `Db` of the
/// theory (prepare runs the validation roster — the Rust twin of the TS
/// replayer's `dbPrepare` acceptance).
fn pin<S: Theory + Copy>(tag: &str, theory: S, query: &Query) -> String {
    let dir = TempDir::new(&unique_tag(tag));
    let db = Db::create(dir.path(), theory).expect("create the corpus theory's store");
    db.prepare(query)
        .unwrap_or_else(|error| panic!("case {tag}: the corpus query validates: {error:?}"));
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    render(&schema, query)
}

/// [`pin`]'s program twin.
fn pin_program<S: Theory + Copy>(tag: &str, theory: S, program: &Program) -> String {
    let dir = TempDir::new(&unique_tag(tag));
    let db = Db::create(dir.path(), theory).expect("create the corpus theory's store");
    db.prepare(program)
        .unwrap_or_else(|error| panic!("case {tag}: the corpus program validates: {error:?}"));
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    render_program(&schema, program)
}

// ---------------------------------------------------------------------
// The deterministic `ir::Program` → JSON encoder: exactly the documented
// wire shape — the key order the TS lowering's object literals insert
// (`ts/src/query/lower.ts`) and the corpus normalization for integers
// (ids/masks as JSON numbers; every `Value` scalar as a decimal STRING,
// because the TS side crosses u64/i64 as `bigint` and the corpus
// stringify replacer renders a bigint as its decimal string). Verified,
// not assumed: the TS replayer asserts `JSON.stringify` equality of its
// own `ProgramIr` against these bytes.
// ---------------------------------------------------------------------

/// JSON string escaping (the `JSON.stringify` subset the corpus needs).
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                write!(out, "\\u{:04x}", c as u32).expect("writing to a String cannot fail");
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn value_json(value: &Value) -> String {
    match value {
        Value::Bool(b) => format!("{{\"kind\":\"bool\",\"value\":{b}}}"),
        Value::U64(v) => format!("{{\"kind\":\"u64\",\"value\":\"{v}\"}}"),
        Value::I64(v) => format!("{{\"kind\":\"i64\",\"value\":\"{v}\"}}"),
        Value::String(bytes) => {
            let text = std::str::from_utf8(bytes).expect("a query string literal is UTF-8");
            format!("{{\"kind\":\"string\",\"value\":{}}}", json_string(text))
        }
        Value::FixedBytes(_) => {
            panic!(
                "a bytes literal has no canonical corpus JSON (Uint8Array does not JSON.stringify canonically) — keep bytes out of corpus cases"
            )
        }
        Value::IntervalU64(interval) => format!(
            "{{\"kind\":\"intervalU64\",\"start\":\"{}\",\"end\":\"{}\"}}",
            interval.start(),
            interval.end()
        ),
        Value::IntervalI64(interval) => format!(
            "{{\"kind\":\"intervalI64\",\"start\":\"{}\",\"end\":\"{}\"}}",
            interval.start(),
            interval.end()
        ),
        Value::AllenMask(mask) => format!("{{\"kind\":\"allenMask\",\"mask\":{}}}", mask.bits()),
    }
}

fn term_json(term: &Term) -> String {
    match term {
        Term::Var(v) => format!("{{\"kind\":\"var\",\"var\":{}}}", v.0),
        Term::Param(p) => format!("{{\"kind\":\"param\",\"param\":{}}}", p.0),
        Term::ParamSet(p) => format!("{{\"kind\":\"paramSet\",\"param\":{}}}", p.0),
        Term::Literal(value) => format!("{{\"kind\":\"literal\",\"value\":{}}}", value_json(value)),
        Term::Measure(v) => format!("{{\"kind\":\"measure\",\"var\":{}}}", v.0),
    }
}

fn agg_op_json(op: AggOp) -> String {
    match op {
        AggOp::Sum => "{\"kind\":\"sum\"}".to_string(),
        AggOp::Min => "{\"kind\":\"min\"}".to_string(),
        AggOp::Max => "{\"kind\":\"max\"}".to_string(),
        AggOp::Count => "{\"kind\":\"count\"}".to_string(),
        AggOp::CountDistinct => "{\"kind\":\"countDistinct\"}".to_string(),
        AggOp::ArgMax { key } => format!("{{\"kind\":\"argMax\",\"key\":{}}}", arg_key_json(key)),
        AggOp::ArgMin { key } => format!("{{\"kind\":\"argMin\",\"key\":{}}}", arg_key_json(key)),
        AggOp::Pack => "{\"kind\":\"pack\"}".to_string(),
    }
}

/// The Arg key's two spellings (R5): a variable key keeps its bare id
/// (the pre-R5 canonical form, corpus-stable), a measure key nests the
/// interval variable under `duration`.
fn arg_key_json(key: bumbledb::ArgKey) -> String {
    match key {
        bumbledb::ArgKey::Var(v) => v.0.to_string(),
        bumbledb::ArgKey::Measure(v) => format!("{{\"duration\":{}}}", v.0),
    }
}

fn find_json(find: &FindTerm) -> String {
    match find {
        FindTerm::Var(v) => format!("{{\"kind\":\"var\",\"var\":{}}}", v.0),
        FindTerm::Measure(v) => format!("{{\"kind\":\"measure\",\"var\":{}}}", v.0),
        FindTerm::Aggregate { op, over: None } => {
            format!("{{\"kind\":\"aggregate\",\"op\":{}}}", agg_op_json(*op))
        }
        FindTerm::Aggregate {
            op,
            over: Some(over),
        } => format!(
            "{{\"kind\":\"aggregate\",\"op\":{},\"over\":{}}}",
            agg_op_json(*op),
            over.0
        ),
        FindTerm::AggregateMeasure { op, over } => format!(
            "{{\"kind\":\"aggregateMeasure\",\"op\":{},\"over\":{}}}",
            agg_op_json(*op),
            over.0
        ),
    }
}

fn atom_json(atom: &Atom) -> String {
    let source = match atom.source {
        AtomSource::Edb(relation) => format!("{{\"kind\":\"edb\",\"relation\":{}}}", relation.0),
        AtomSource::Idb(pred) => format!("{{\"kind\":\"idb\",\"pred\":{}}}", pred.0),
    };
    let bindings = atom
        .bindings
        .iter()
        .map(|(field, term)| format!("[{},{}]", field.0, term_json(term)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"source\":{source},\"bindings\":[{bindings}]}}")
}

fn cmp_op_json(op: CmpOp) -> String {
    match op {
        CmpOp::Eq => "{\"kind\":\"eq\"}".to_string(),
        CmpOp::Ne => "{\"kind\":\"ne\"}".to_string(),
        CmpOp::Lt => "{\"kind\":\"lt\"}".to_string(),
        CmpOp::Le => "{\"kind\":\"le\"}".to_string(),
        CmpOp::Gt => "{\"kind\":\"gt\"}".to_string(),
        CmpOp::Ge => "{\"kind\":\"ge\"}".to_string(),
        CmpOp::PointIn => "{\"kind\":\"pointIn\"}".to_string(),
        CmpOp::Allen { mask } => {
            let mask = match mask {
                MaskTerm::Literal(mask) => {
                    format!("{{\"kind\":\"literal\",\"mask\":{}}}", mask.bits())
                }
                MaskTerm::Param(param) => format!("{{\"kind\":\"param\",\"param\":{}}}", param.0),
            };
            format!("{{\"kind\":\"allen\",\"mask\":{mask}}}")
        }
    }
}

fn comparison_json(cmp: &Comparison) -> String {
    format!(
        "{{\"op\":{},\"lhs\":{},\"rhs\":{}}}",
        cmp_op_json(cmp.op),
        term_json(&cmp.lhs),
        term_json(&cmp.rhs)
    )
}

fn condition_json(condition: &ConditionTree) -> String {
    match condition {
        ConditionTree::Leaf(cmp) => {
            format!("{{\"kind\":\"leaf\",\"cmp\":{}}}", comparison_json(cmp))
        }
        ConditionTree::And(children) => format!(
            "{{\"kind\":\"and\",\"children\":[{}]}}",
            children
                .iter()
                .map(condition_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ConditionTree::Or(children) => format!(
            "{{\"kind\":\"or\",\"children\":[{}]}}",
            children
                .iter()
                .map(condition_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn rule_json(rule: &Rule) -> String {
    format!(
        "{{\"finds\":[{}],\"atoms\":[{}],\"negated\":[{}],\"conditions\":[{}]}}",
        rule.finds
            .iter()
            .map(find_json)
            .collect::<Vec<_>>()
            .join(","),
        rule.atoms
            .iter()
            .map(atom_json)
            .collect::<Vec<_>>()
            .join(","),
        rule.negated
            .iter()
            .map(atom_json)
            .collect::<Vec<_>>()
            .join(","),
        rule.conditions
            .iter()
            .map(condition_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn head_term_json(term: HeadTerm) -> String {
    match term {
        HeadTerm::Var => "{\"kind\":\"var\"}".to_string(),
        HeadTerm::Aggregate(op) => {
            let name = match op {
                HeadOp::Sum => "sum",
                HeadOp::Min => "min",
                HeadOp::Max => "max",
                HeadOp::Count => "count",
                HeadOp::CountDistinct => "countDistinct",
                HeadOp::ArgMax => "argMax",
                HeadOp::ArgMin => "argMin",
                HeadOp::Pack => "pack",
            };
            format!("{{\"kind\":\"aggregate\",\"op\":\"{name}\"}}")
        }
    }
}

fn predicate_json(predicate: &PredicateDef) -> String {
    format!(
        "{{\"head\":[{}],\"rules\":[{}]}}",
        predicate
            .head
            .iter()
            .map(|term| head_term_json(*term))
            .collect::<Vec<_>>()
            .join(","),
        predicate
            .rules
            .iter()
            .map(rule_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// The whole program, compact — byte-for-byte what the TS side's
/// `JSON.stringify(programIr, bigintAsDecimalString)` produces.
fn program_json(program: &Program) -> String {
    format!(
        "{{\"predicates\":[{}],\"output\":{}}}",
        program
            .predicates
            .iter()
            .map(predicate_json)
            .collect::<Vec<_>>()
            .join(","),
        program.output.0
    )
}

// ---------------------------------------------------------------------
// The case table.
// ---------------------------------------------------------------------

/// One corpus case, fully computed: the checked-in document is a pure
/// function of this value.
struct Case {
    name: &'static str,
    /// Whether the TS query BUILDER can construct this case (`false` =
    /// hand-written `ProgramIr` on the TS side; still `dbPrepare`d).
    builder: bool,
    /// The grammar productions this case is coverage for.
    productions: &'static [&'static str],
    /// The notation exactly as pinned (whitespace-normalized against the
    /// compiled source tokens).
    notation: &'static str,
    /// The render of the lowered IR — the fixed-point text.
    normalized: String,
    /// The compact `ProgramIr` JSON.
    program_json: String,
}

/// Whitespace-erased comparison key: ties a pinned string to compiled
/// tokens (`stringify!` spaces tokens; the notation spaces for humans).
fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

macro_rules! corpus_case {
    // A plain query case (always TS-builder-expressible in this corpus).
    ($cases:ident, query, $name:literal, [$($production:literal),+ $(,)?], $notation:literal,
     { $($src:tt)+ }, { $($norm:tt)+ } $(,)?) => {{
        let lowered = query!(Ledger { $($src)+ });
        let reparsed = query!(Ledger { $($norm)+ });
        assert_eq!(
            reparsed, lowered,
            "case {}: the normalized text reparses to the identical IR",
            $name
        );
        assert_eq!(
            strip($notation),
            strip(stringify!($($src)+)),
            "case {}: the pinned notation is the compiled source",
            $name
        );
        let normalized = pin($name, Ledger, &lowered);
        assert_eq!(
            strip(&normalized),
            strip(stringify!($($norm)+)),
            "case {}: the compiled normalized tokens are the render's own output",
            $name
        );
        let encoded = program_json(&Program::from(lowered));
        $cases.push(Case {
            name: $name,
            builder: true,
            productions: &[$($production),+],
            notation: $notation,
            normalized,
            program_json: encoded,
        });
    }};
    // A program case; `builder` says whether the TS builder can spell it.
    ($cases:ident, program($builder:literal), $name:literal, [$($production:literal),+ $(,)?], $notation:literal,
     { $($src:tt)+ }, { $($norm:tt)+ } $(,)?) => {{
        let lowered = query!(Ledger { $($src)+ });
        let reparsed = query!(Ledger { $($norm)+ });
        assert_eq!(
            reparsed, lowered,
            "case {}: the normalized text reparses to the identical IR",
            $name
        );
        assert_eq!(
            strip($notation),
            strip(stringify!($($src)+)),
            "case {}: the pinned notation is the compiled source",
            $name
        );
        let normalized = pin_program($name, Ledger, &lowered);
        assert_eq!(
            strip(&normalized),
            strip(stringify!($($norm)+)),
            "case {}: the compiled normalized tokens are the render's own output",
            $name
        );
        let encoded = program_json(&lowered);
        $cases.push(Case {
            name: $name,
            builder: $builder,
            productions: &[$($production),+],
            notation: $notation,
            normalized,
            program_json: encoded,
        });
    }};
}

/// Builds (and self-checks) every corpus case. Each case validates
/// against a real store, round-trips through the renderer, and ties its
/// pinned strings to the compiled tokens — so the corpus can never say
/// something this crate did not compile.
#[allow(clippy::too_many_lines)]
fn cases() -> Vec<Case> {
    let mut cases: Vec<Case> = Vec::new();

    corpus_case!(cases, query, "holder-names",
        ["punning", "field-var"],
        "(name) | Holder(id: h, name);",
        { (name) | Holder(id: h, name); },
        { (v1) | Holder(id: v0, name: v1); });

    corpus_case!(cases, query, "amount-selection",
        ["eq-literal"],
        "(id) | Posting(id, amount == -100);",
        { (id) | Posting(id, amount == -100); },
        { (v0) | Posting(id: v0, amount == -100); });

    corpus_case!(cases, query, "usd-accounts",
        ["eq-handle"],
        "(id) | Account(id, currency == Usd);",
        { (id) | Account(id, currency == Usd); },
        { (v0) | Account(id: v0, currency == Usd); });

    corpus_case!(cases, query, "account-selection-param",
        ["eq-param"],
        "(id) | Posting(id, account == ?acct);",
        { (id) | Posting(id, account == ?acct); },
        { (v0) | Posting(id: v0, account == ?0); });

    corpus_case!(cases, query, "scalar-comparisons",
        ["ne", "lt", "le", "gt", "ge"],
        "(id) | Posting(id, entry, account, instrument, amount, at), \
         id == ?wanted, entry != 0, account < 10, instrument <= 10, amount > -10, at >= -10;",
        { (id) | Posting(id, entry, account, instrument, amount, at),
                 id == ?wanted, entry != 0, account < 10, instrument <= 10,
                 amount > -10, at >= -10; },
        { (v0) | Posting(id: v0, entry: v1, account: v2, instrument: v3, amount: v4, at: v5),
                 v0 == ?0, v1 != 0, v2 < 10, v3 <= 10, v4 > -10, v5 >= -10; });

    corpus_case!(cases, query, "currency-in-set",
        ["in-param"],
        "(id) | Account(id, currency in ?currencies);",
        { (id) | Account(id, currency in ?currencies); },
        { (v0) | Account(id: v0, currency in ?0); });

    corpus_case!(cases, query, "mandate-point-membership",
        ["point-in"],
        "(org) | Mandate(org, active), ?today in active;",
        { (org) | Mandate(org, active), ?today in active; },
        { (v0) | Mandate(org: v0, active: v1), ?0 in v1; });

    corpus_case!(cases, query, "mandate-window",
        ["allen-literal-mask"],
        "(org) | Mandate(org, active), Allen(active, INTERSECTS, ?window);",
        { (org) | Mandate(org, active), Allen(active, INTERSECTS, ?window); },
        { (v0) | Mandate(org: v0, active: v1), Allen(v1, INTERSECTS, ?0); });

    corpus_case!(cases, query, "mandate-adjacent",
        ["allen-mask-union"],
        "(org) | Mandate(org, active), Allen(active, BEFORE|MEETS, ?window);",
        { (org) | Mandate(org, active), Allen(active, BEFORE|MEETS, ?window); },
        { (v0) | Mandate(org: v0, active: v1), Allen(v1, BEFORE|MEETS, ?0); });

    corpus_case!(cases, query, "mandate-mask-param",
        ["allen-mask-param"],
        "(a, b) | Mandate(account: a, active: s), Mandate(account: b, active: t), \
         a < b, Allen(s, ?rel, t);",
        { (a, b) | Mandate(account: a, active: s), Mandate(account: b, active: t),
                   a < b, Allen(s, ?rel, t); },
        { (v0, v2) | Mandate(account: v0, active: v1), Mandate(account: v2, active: v3),
                     v0 < v2, Allen(v1, ?0, v3); });

    corpus_case!(cases, query, "dormant-holders",
        ["negation"],
        "(holder) | Account(id: a, holder), !Posting(account: a);",
        { (holder) | Account(id: a, holder), !Posting(account: a); },
        { (v1) | Account(id: v0, holder: v1), !Posting(account: v0); });

    corpus_case!(cases, query, "balances",
        ["agg-sum", "agg-count", "named-columns"],
        "(account, total: Sum(amount), n: Count) | Posting(account, amount);",
        { (account, total: Sum(amount), n: Count) | Posting(account, amount); },
        { (v0, Sum(v1), Count) | Posting(account: v0, amount: v1); });

    corpus_case!(cases, query, "entry-fanout",
        ["agg-count-distinct"],
        "(account, entries: CountDistinct(entry)) | Posting(entry, account);",
        { (account, entries: CountDistinct(entry)) | Posting(entry, account); },
        { (v1, CountDistinct(v0)) | Posting(entry: v0, account: v1); });

    corpus_case!(cases, query, "amount-floor",
        ["agg-min"],
        "(account, lo: Min(amount)) | Posting(account, amount);",
        { (account, lo: Min(amount)) | Posting(account, amount); },
        { (v0, Min(v1)) | Posting(account: v0, amount: v1); });

    corpus_case!(cases, query, "amount-ceiling",
        ["agg-max"],
        "(account, hi: Max(amount)) | Posting(account, amount);",
        { (account, hi: Max(amount)) | Posting(account, amount); },
        { (v0, Max(v1)) | Posting(account: v0, amount: v1); });

    corpus_case!(cases, query, "latest-posting",
        ["agg-arg-max"],
        "(ArgMax(id, at)) | Posting(id, at);",
        { (ArgMax(id, at)) | Posting(id, at); },
        { (ArgMax(v0, v1)) | Posting(id: v0, at: v1); });

    corpus_case!(cases, query, "earliest-posting",
        ["agg-arg-min"],
        "(ArgMin(id, at)) | Posting(id, at);",
        { (ArgMin(id, at)) | Posting(id, at); },
        { (ArgMin(v0, v1)) | Posting(id: v0, at: v1); });

    corpus_case!(cases, query, "mandate-pack",
        ["agg-pack"],
        "(org, busy: Pack(active)) | Mandate(org, active);",
        { (org, busy: Pack(active)) | Mandate(org, active); },
        { (v0, Pack(v1)) | Mandate(org: v0, active: v1); });

    corpus_case!(cases, query, "mandate-durations",
        ["duration"],
        "(org, Duration(active)) | Mandate(org, active);",
        { (org, Duration(active)) | Mandate(org, active); },
        { (v0, Duration(v1)) | Mandate(org: v0, active: v1); });

    corpus_case!(cases, query, "long-mandates",
        ["duration"],
        "(org, Sum(Duration(active))) | Mandate(org, active), Duration(active) >= 3600;",
        { (org, Sum(Duration(active))) | Mandate(org, active), Duration(active) >= 3600; },
        { (v0, Sum(Duration(v1))) | Mandate(org: v0, active: v1), Duration(v1) >= 3600; });

    corpus_case!(cases, query, "usd-or-eur-accounts",
        ["multi-rule-union"],
        "(id) | Account(id, currency == Usd);\n\
         (id) | Account(id, currency == Eur);",
        { (id) | Account(id, currency == Usd);
          (id) | Account(id, currency == Eur); },
        { (v0) | Account(id: v0, currency == Usd);
          (v0) | Account(id: v0, currency == Eur); });

    corpus_case!(cases, program(true), "org-reach-rooted",
        ["program-recursion", "idb-ordered-dense"],
        "reach(o) | Org(id: o), o == ?root;\n\
         reach(p) | OrgParent(child: c, parent: p), reach(c);\n\
         (p) | Org(id: p), reach(p);",
        { reach(o) | Org(id: o), o == ?root;
          reach(p) | OrgParent(child: c, parent: p), reach(c);
          (p) | Org(id: p), reach(p); },
        { p0(v0) | Org(id: v0), v0 == ?0;
          p0(v1) | OrgParent(child: v0, parent: v1), p0(v0);
          (v0) | Org(id: v0), p0(v0); });

    // The classic two-column closure: the recursive rule binds its head's
    // second position through the idb atom alone, which the TS builder's
    // join-position law (idb vars must be relation-bound) cannot spell —
    // hand-written `ProgramIr` on the TS side, still prepared.
    corpus_case!(cases, program(false), "org-reach",
        ["program-recursion", "idb-ordered-dense"],
        "reach(c, a) | OrgParent(child: c, parent: a);\n\
         reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);\n\
         (c, a) | reach(c, a);",
        { reach(c, a) | OrgParent(child: c, parent: a);
          reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
          (c, a) | reach(c, a); },
        { p0(v0, v1) | OrgParent(child: v0, parent: v1);
          p0(v0, v2) | OrgParent(child: v0, parent: v1), p0(v1, v2);
          (v0, v1) | p0(v0, v1); });

    corpus_case!(cases, program(false), "posted-sparse",
        ["idb-sparse", "idb-position-selection"],
        "posted(id, account, amount) | Posting(id, account, amount);\n\
         (x) | posted(2: x, 0 in ?wanted);",
        { posted(id, account, amount) | Posting(id, account, amount);
          (x) | posted(2: x, 0 in ?wanted); },
        { p0(v0, v1, v2) | Posting(id: v0, account: v1, amount: v2);
          (v0) | p0(2: v0, 0 in ?0); });

    corpus_case!(cases, program(false), "usd-selected",
        ["idb-position-selection"],
        "acct(id, currency) | Account(id, currency);\n\
         (a) | acct(0: a, 1 == Currency::Usd);",
        { acct(id, currency) | Account(id, currency);
          (a) | acct(0: a, 1 == Currency::Usd); },
        { p0(v0, v1) | Account(id: v0, currency: v1);
          (v0) | p0(0: v0, 1 == 0); });

    cases
}

/// Every grammar production the corpus must witness at least once — the
/// enumeration the corpus README states, asserted here: an uncovered
/// production fails, and a case naming an unknown production fails.
const REQUIRED_PRODUCTIONS: &[&str] = &[
    "punning",
    "field-var",
    "eq-literal",
    "eq-handle",
    "eq-param",
    "ne",
    "lt",
    "le",
    "gt",
    "ge",
    "in-param",
    "point-in",
    "allen-literal-mask",
    "allen-mask-union",
    "allen-mask-param",
    "negation",
    "agg-sum",
    "agg-min",
    "agg-max",
    "agg-count",
    "agg-count-distinct",
    "agg-arg-max",
    "agg-arg-min",
    "agg-pack",
    "duration",
    "named-columns",
    "multi-rule-union",
    "program-recursion",
    "idb-ordered-dense",
    "idb-sparse",
    "idb-position-selection",
];

/// One case's checked-in document, byte for byte: pretty header fields
/// for hand-reading, the program compact (the exact `JSON.stringify`
/// bytes the TS replayer compares against).
fn document(case: &Case) -> String {
    let productions = case
        .productions
        .iter()
        .map(|production| json_string(production))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"name\": {},\n  \"builder\": {},\n  \"productions\": [{}],\n  \"notation\": {},\n  \"normalized\": {},\n  \"program\": {}\n}}\n",
        json_string(case.name),
        case.builder,
        productions,
        json_string(case.notation),
        json_string(&case.normalized),
        case.program_json
    )
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/notation-corpus"
    ))
}

/// The corpus theory's engine fingerprint, 64 lowercase hex chars.
fn ledger_fingerprint_hex() -> String {
    let schema: Schema = Ledger
        .descriptor()
        .validate()
        .expect("the corpus theory lands");
    bumbledb::schema::fingerprint::fingerprint(&schema)
        .0
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
            hex
        })
}

/// Every checked-in case document replays byte-identical from the case
/// table: editing a case's notation, normalized text, or program fails
/// here (and each case has already validated against a real store and
/// round-tripped through the renderer inside [`cases`]).
#[test]
fn the_corpus_replays_byte_identical() {
    let cases = cases();
    let dir = corpus_dir();
    let mut expected = BTreeSet::new();
    for case in &cases {
        let file = format!("{}.json", case.name);
        let path = dir.join(&file);
        let pinned = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "read {}: {error} — regenerate with `cargo test -p bumbledb-query regenerate_the_notation_corpus -- --ignored`",
                path.display()
            )
        });
        assert_eq!(
            pinned,
            document(case),
            "case {}: the checked-in document replays byte-identical",
            case.name
        );
        expected.insert(file);
    }
    for entry in std::fs::read_dir(&dir).expect("the corpus directory exists") {
        let name = entry.expect("a readable corpus entry").file_name();
        let name = name.to_string_lossy().into_owned();
        if std::path::Path::new(&name)
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            assert!(
                expected.contains(&name),
                "orphan corpus file {name}: every case document belongs to the case table"
            );
        }
    }
}

/// The production-coverage enumeration: at least 20 cases, unique names,
/// every required production witnessed, no case naming an unknown one.
#[test]
fn every_production_is_covered() {
    let cases = cases();
    assert!(
        cases.len() >= 20,
        "the corpus holds at least 20 cases (got {})",
        cases.len()
    );
    let mut names = BTreeSet::new();
    let mut by_production: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for case in &cases {
        assert!(
            names.insert(case.name),
            "case name {} is declared twice",
            case.name
        );
        for production in case.productions {
            assert!(
                REQUIRED_PRODUCTIONS.contains(production),
                "case {} names the unknown production {production}",
                case.name
            );
            by_production.entry(production).or_default().push(case.name);
        }
    }
    for production in REQUIRED_PRODUCTIONS {
        assert!(
            by_production.contains_key(production),
            "the production {production} has no corpus case — the grammar is bigger than the corpus"
        );
    }
}

/// The corpus theory pins to ONE engine fingerprint both replayers read
/// (`schema-fingerprint.txt`) — the T5 mechanism, one line: the TS
/// replayer builds the same theory structurally and asserts its store's
/// `dbFingerprint` equals this hex, so the corpus schemas cannot drift.
#[test]
fn the_corpus_schema_fingerprint_matches_the_pin() {
    let path = corpus_dir().join("schema-fingerprint.txt");
    let pinned = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "read {}: {error} — regenerate with `cargo test -p bumbledb-query regenerate_the_notation_corpus -- --ignored`",
            path.display()
        )
    });
    assert_eq!(
        pinned.trim_end(),
        ledger_fingerprint_hex(),
        "the corpus theory's fingerprint matches the cross-host pin"
    );
}

/// Rewrites every case document (and the schema fingerprint pin) from
/// the case table — deterministic: identical bytes from identical
/// source, forever. `README.md` is hand-written and never regenerated.
#[test]
#[ignore = "regenerates the checked-in corpus; run explicitly after editing the case table"]
fn regenerate_the_notation_corpus() {
    let dir = corpus_dir();
    std::fs::create_dir_all(&dir).expect("create the corpus directory");
    for case in cases() {
        let path = dir.join(format!("{}.json", case.name));
        std::fs::write(&path, document(&case)).expect("write a corpus document");
    }
    std::fs::write(
        dir.join("schema-fingerprint.txt"),
        format!("{}\n", ledger_fingerprint_hex()),
    )
    .expect("write the schema fingerprint pin");
}
