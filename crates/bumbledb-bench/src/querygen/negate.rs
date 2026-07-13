//! The negation pass: 0–2 anti-join atoms appended to any shape
//! (`docs/architecture/20-query-ir.md` § negation). Every variable
//! placed in a negated atom is drawn from the positive atoms' anchors
//! by provenance — a negated atom binds nothing, only rejects — and the
//! binding shapes sweep the space: key-covered (a fresh key bound; at
//! most one witness) and open (non-key bindings over the multiply-
//! witnessed relations — rejection must not depend on witness count),
//! with occasional literal, param, param-set, and membership bindings
//! inside.

use bumbledb::{Term, Value};

use crate::corpus_gen::Rng;
use crate::querygen::Builder;
use crate::querygen::target::ids;

/// The templates a query's anchors make available.
#[derive(Clone, Copy)]
enum Template {
    /// `¬PostingTag(posting = v [, tag = X])` — open; even postings
    /// carry TWO tags in the corpus (the duplicate-witness negated
    /// side), odd postings none.
    TagOnPosting,
    /// `¬Mandate(account = v [, active ∋ t])` — open; membership inside
    /// negation when an i64 anchor exists.
    MandateOnAccount,
    /// `¬Posting(account = v [, reconciled = literal])` — open,
    /// multiply witnessed (an account has many postings).
    PostingOnAccount,
    /// `¬JournalEntry(id = v [, source = X])` — key-covered (fresh).
    EntryById,
    /// `¬Holder(id = v)` — key-covered (fresh).
    HolderById,
    /// `¬Org()` / `¬OrgParent()` — the zero-binding negated gate.
    Gate,
}

/// Appends 0–2 negated atoms (60% none, 30% one, 10% two).
pub(super) fn negate(b: &mut Builder, rng: &mut Rng) {
    let count = match rng.range(10) {
        0..=5 => 0,
        6..=8 => 1,
        _ => 2,
    };
    for _ in 0..count {
        let mut templates: Vec<Template> = Vec::new();
        if b.anchored_at(&[(ids::POSTING, ids::posting::ID)]).is_some() {
            templates.push(Template::TagOnPosting);
        }
        if b.anchored_at(&[
            (ids::POSTING, ids::posting::ACCOUNT),
            (ids::ACCOUNT, ids::account::ID),
        ])
        .is_some()
        {
            templates.push(Template::MandateOnAccount);
            templates.push(Template::PostingOnAccount);
        }
        if b.anchored_at(&[
            (ids::POSTING, ids::posting::ENTRY),
            (ids::JOURNAL_ENTRY, ids::journal_entry::ID),
        ])
        .is_some()
        {
            templates.push(Template::EntryById);
        }
        if b.anchored_at(&[
            (ids::ACCOUNT, ids::account::HOLDER),
            (ids::HOLDER, ids::holder::ID),
        ])
        .is_some()
        {
            templates.push(Template::HolderById);
        }
        let template = if templates.is_empty() || rng.chance(1, 8) {
            Template::Gate
        } else {
            templates[usize::try_from(rng.range(templates.len() as u64)).expect("small")]
        };
        apply(b, rng, template);
    }
}

/// A closed-vocabulary binding term (a u64 row id): literal, param, or
/// param set — the "occasional literal/param/set bindings inside" mix.
fn vocab_term(b: &mut Builder, rng: &mut Rng, rows: u64) -> Option<Term> {
    match rng.range(4) {
        0 => None,
        1 => Some(Term::Literal(Value::U64(rng.range(rows)))),
        2 => Some(Term::Param(b.fresh_param())),
        _ => Some(Term::ParamSet(b.fresh_param())),
    }
}

fn apply(b: &mut Builder, rng: &mut Rng, template: Template) {
    match template {
        Template::TagOnPosting => {
            let v = b
                .anchored_at(&[(ids::POSTING, ids::posting::ID)])
                .expect("template gated on anchor");
            let atom = b.negated_atom(ids::POSTING_TAG);
            b.bind_negated(atom, ids::posting_tag::POSTING, Term::Var(v));
            if let Some(term) = vocab_term(b, rng, 3) {
                b.bind_negated(atom, ids::posting_tag::TAG, term);
            }
        }
        Template::MandateOnAccount => {
            let v = b
                .anchored_at(&[
                    (ids::POSTING, ids::posting::ACCOUNT),
                    (ids::ACCOUNT, ids::account::ID),
                ])
                .expect("template gated on anchor");
            let atom = b.negated_atom(ids::MANDATE);
            // The account equality binding comes FIRST and always: a
            // negated atom whose only bindings are memberships is the
            // Cartesian degenerate (`40-execution.md`) — the cost-bound
            // rule makes the membership-only form unemittable here.
            b.bind_negated(atom, ids::mandate::ACCOUNT, Term::Var(v));
            // Membership inside negation: a positive-bound i64 point
            // when one exists (no mandate covers the instant).
            let point = b.anchored_at(&[
                (ids::POSTING, ids::posting::AT),
                (ids::POSTING, ids::posting::AMOUNT),
                (ids::JOURNAL_ENTRY, ids::journal_entry::CREATED_AT),
            ]);
            if let (Some(t), true) = (point, rng.chance(1, 2)) {
                b.bind_negated(atom, ids::mandate::ACTIVE, Term::Var(t));
            }
        }
        Template::PostingOnAccount => {
            let v = b
                .anchored_at(&[
                    (ids::POSTING, ids::posting::ACCOUNT),
                    (ids::ACCOUNT, ids::account::ID),
                ])
                .expect("template gated on anchor");
            let atom = b.negated_atom(ids::POSTING);
            b.bind_negated(atom, ids::posting::ACCOUNT, Term::Var(v));
            if rng.chance(1, 2) {
                b.bind_negated(
                    atom,
                    ids::posting::RECONCILED,
                    Term::Literal(Value::Bool(rng.chance(1, 2))),
                );
            }
        }
        Template::EntryById => {
            let v = b
                .anchored_at(&[
                    (ids::POSTING, ids::posting::ENTRY),
                    (ids::JOURNAL_ENTRY, ids::journal_entry::ID),
                ])
                .expect("template gated on anchor");
            let atom = b.negated_atom(ids::JOURNAL_ENTRY);
            b.bind_negated(atom, ids::journal_entry::ID, Term::Var(v));
            if let Some(term) = vocab_term(b, rng, 3) {
                b.bind_negated(atom, ids::journal_entry::SOURCE, term);
            }
        }
        Template::HolderById => {
            let v = b
                .anchored_at(&[
                    (ids::ACCOUNT, ids::account::HOLDER),
                    (ids::HOLDER, ids::holder::ID),
                ])
                .expect("template gated on anchor");
            let atom = b.negated_atom(ids::HOLDER);
            b.bind_negated(atom, ids::holder::ID, Term::Var(v));
        }
        Template::Gate => {
            let relation = if rng.chance(1, 2) {
                ids::ORG
            } else {
                ids::ORG_PARENT
            };
            b.negated_atom(relation);
        }
    }
}
