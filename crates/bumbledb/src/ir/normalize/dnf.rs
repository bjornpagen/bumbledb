//! DNF distribution — OR as data (`docs/architecture/20-query-ir.md`,
//! § the input condition grammar). The input grammar admits nested OR
//! ([`ConditionTree`]); **DNF of a query is a set of rules**, so
//! validation distributes each rule's trees to disjunctive normal form
//! and each disjunct becomes a rule: atoms cloned, conditions = the
//! disjunct's leaves. The validated artifact carries only the flat
//! [`LoweredRule`] — no `Or` survives the boundary, and the planner and
//! executor never learn disjunction existed.
//!
//! Pure functions, deliberately: lowering-then-evaluating ≡ evaluating
//! the tree naively is the differential suite's property, proven against
//! the naive model's direct tree evaluation (which never lowers).

use crate::ir::{Atom, Comparison, ConditionTree, FindTerm, Rule};

/// One Or-free rule — the only rule shape downstream of validation: the
/// input [`Rule`] with its condition trees distributed away. Everything
/// past the boundary (typing, normalization, planning, execution) reads
/// exactly this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredRule {
    /// One term per head position, cloned from the input rule.
    pub finds: Vec<FindTerm>,
    /// The input rule's positive atoms, cloned per disjunct.
    pub atoms: Vec<Atom>,
    /// The input rule's negated atoms, cloned per disjunct.
    pub negated: Vec<Atom>,
    /// The disjunct's leaves — a flat conjunction.
    pub conditions: Vec<Comparison>,
    /// Written-rule provenance (ruled 2026-07-23, R2): the index of the
    /// input rule this disjunct was minted from, stamped by
    /// `lower_rules` and cleared to `None` when [`collapse`] merges
    /// duplicates minted by DIFFERENT written rules. A rule set whose
    /// every member carries one shared index is **DNF-derived** — its
    /// disjuncts share one variable scope, so the union dedup re-keys
    /// on the shared slot arrays (the or-transparency law,
    /// `docs/architecture/20-query-ir.md` § aggregation); any other set
    /// is hand-written and keys the head projection.
    pub written: Option<u16>,
    /// The full mint set: EVERY written rule this disjunct belongs to —
    /// `written`'s uncompressed form, unioned by [`collapse`] where a
    /// cross-written merge erases `written`. The ray-probe verdict fold
    /// (the Kleene algebra, ruled 2026-07-23, R6) reads it: a written
    /// rule's verdict at a binding is the Or over exactly its own
    /// disjuncts, so a collapsed duplicate must still count for each
    /// rule that minted it.
    pub minted: Vec<u16>,
}

/// The nesting depth of a rule's condition trees — a leaf is depth 1, a
/// node one more than its deepest child, the empty combinations depth 1,
/// no trees depth 0. Computed **iteratively** (an explicit work list):
/// this is the check for [`crate::ir::MAX_CONDITION_DEPTH`], so it must
/// itself survive the hostile input it exists to reject — every
/// *recursive* tree walk ([`disjunct_count`], [`distribute`], the
/// renderer) runs only after validation judged this bound.
#[must_use]
pub fn nesting_depth(trees: &[ConditionTree]) -> usize {
    let mut work: Vec<(&ConditionTree, usize)> = trees.iter().map(|tree| (tree, 1)).collect();
    let mut max = 0;
    while let Some((tree, depth)) = work.pop() {
        max = max.max(depth);
        match tree {
            ConditionTree::Leaf(_) => {}
            ConditionTree::And(children) | ConditionTree::Or(children) => {
                work.extend(children.iter().map(|child| (child, depth + 1)));
            }
        }
    }
    max
}

/// The number of DNF terms [`distribute`] would produce for the rule,
/// computed structurally **without materializing** — the cap
/// (`ValidationError::DnfExceedsRules`) is judged on this count, so the
/// exponential case is rejected before a single disjunct is built.
/// Saturating: a count past `usize::MAX` is still "past the cap".
#[must_use]
pub fn disjunct_count(rule: &Rule) -> usize {
    conjunction_count(&rule.conditions)
}

/// Terms of a conjunction of trees: the product of the children's counts
/// (the empty conjunction is one term — the empty leaf list).
fn conjunction_count(trees: &[ConditionTree]) -> usize {
    trees.iter().map(tree_count).fold(1, usize::saturating_mul)
}

fn tree_count(tree: &ConditionTree) -> usize {
    match tree {
        ConditionTree::Leaf(_) => 1,
        ConditionTree::And(children) => conjunction_count(children),
        // The empty disjunction is zero terms: `Or([])` is false and the
        // rule denotes nothing.
        ConditionTree::Or(children) => children
            .iter()
            .map(tree_count)
            .fold(0, usize::saturating_add),
    }
}

/// Distributes one rule's condition trees to DNF: one [`LoweredRule`] per
/// term, atoms and finds cloned, conditions = that term's leaves in
/// left-to-right tree order. Callers judge the cap on
/// [`disjunct_count`] **first** — distribution materializes every term.
/// Provenance is the caller's to stamp (`lower_rules` writes the written
/// index; the terms leave here unstamped).
#[must_use]
pub fn distribute(rule: &Rule) -> Vec<LoweredRule> {
    conjunction_terms(&rule.conditions)
        .into_iter()
        .map(|conditions| LoweredRule {
            finds: rule.finds.clone(),
            atoms: rule.atoms.clone(),
            negated: rule.negated.clone(),
            conditions,
            written: None,
            minted: Vec::new(),
        })
        .collect()
}

/// DNF terms of a conjunction of trees: the cross product of the
/// children's term sets (one empty term for the empty conjunction).
fn conjunction_terms(trees: &[ConditionTree]) -> Vec<Vec<Comparison>> {
    let mut terms: Vec<Vec<Comparison>> = vec![Vec::new()];
    for tree in trees {
        let rhs = tree_terms(tree);
        terms = terms
            .iter()
            .flat_map(|lhs| {
                rhs.iter()
                    .map(|term| lhs.iter().chain(term).cloned().collect())
            })
            .collect();
    }
    terms
}

/// DNF terms of one tree: a leaf is one one-leaf term, `And` distributes
/// (cross product), `Or` unions (concatenation).
fn tree_terms(tree: &ConditionTree) -> Vec<Vec<Comparison>> {
    match tree {
        ConditionTree::Leaf(comparison) => vec![vec![comparison.clone()]],
        ConditionTree::And(children) => conjunction_terms(children),
        ConditionTree::Or(children) => children.iter().flat_map(tree_terms).collect(),
    }
}

/// Collapses duplicate rules after distribution — set semantics at the
/// representation level, the duplicate-statement machinery's sibling
/// (`schema/validate.rs` rejects identical normalized statements; here
/// the duplicate is a fact of the distribution, so it collapses instead).
/// Normalized-form equality: finds, atoms, and negated atoms verbatim;
/// condition lists as **sets** — order- and multiplicity-insensitive,
/// because a conjunction is idempotent and commutative
/// (`lean/Bumbledb/Query/Denotation.lean`:
/// `ruleAnswers_conditions_congr` identifies the set-equal pair's
/// answers, then `union_idempotent` deletes the duplicate). First
/// occurrence wins, so rule order (hence diagnostic indices) stays
/// deterministic.
#[must_use]
pub fn collapse(rules: Vec<LoweredRule>) -> Vec<LoweredRule> {
    let mut kept: Vec<LoweredRule> = Vec::with_capacity(rules.len());
    for rule in rules {
        if let Some(earlier) = kept
            .iter_mut()
            .find(|earlier| same_normalized_body(earlier, &rule))
        {
            // A duplicate minted by a DIFFERENT written rule makes the
            // survivor shared vocabulary of neither: provenance clears,
            // so the set reads as hand-written (the R2 provenance split
            // is judged on every member carrying ONE written index).
            // The mint SET unions regardless — the verdict fold (R6)
            // still counts the disjunct for every rule that wrote it.
            if earlier.written != rule.written {
                earlier.written = None;
            }
            for idx in rule.minted {
                if !earlier.minted.contains(&idx) {
                    earlier.minted.push(idx);
                }
            }
        } else {
            kept.push(rule);
        }
    }
    kept
}

fn same_normalized_body(a: &LoweredRule, b: &LoweredRule) -> bool {
    a.finds == b.finds
        && a.atoms == b.atoms
        && a.negated == b.negated
        && condition_set_eq(&a.conditions, &b.conditions)
}

/// Set equality by mutual containment — the lists are small (they are
/// one rule's conjuncts) and [`Comparison`] carries no order.
fn condition_set_eq(a: &[Comparison], b: &[Comparison]) -> bool {
    a.iter().all(|p| b.contains(p)) && b.iter().all(|p| a.contains(p))
}
