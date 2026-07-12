/// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`.
pub const POINT: &str =
    "SELECT DISTINCT t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0 WHERE t0.\"id\" = ?1";

/// `containment_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
/// Account(id = ?0, holder = h), Holder(id = h, name)` with the
/// posting's account pinned by the same param on both sides.
pub const CONTAINMENT_WALK: &str = "SELECT DISTINCT t2.\"name\", t0.\"amount\" FROM \"Posting\" AS t0, \"Account\" AS t1, \"Holder\" AS t2 WHERE t0.\"account\" = ?1 AND t1.\"id\" = ?1 AND t1.\"holder\" = t2.\"id\"";

/// balance — `Q(a, Sum(amount)) :- Posting(id, account = a, amount),
/// Account(id = a, holder = ?0)` — the id binding makes the fold a
/// true ledger balance (duplicate amounts count once each).
pub const BALANCE: &str = "SELECT v0, SUM(v1) FROM (SELECT DISTINCT t0.\"account\" AS v0, t0.\"amount\" AS v1, t0.\"id\" AS v2 FROM \"Posting\" AS t0, \"Account\" AS t1 WHERE t0.\"account\" = t1.\"id\" AND t1.\"holder\" = ?1) GROUP BY v0";

/// `no_tag` — `Q(p) :- Posting(id = p), ¬PostingTag(posting = p,
/// tag = Fee)`: the negation family (postings with no fee tag). One
/// `NOT EXISTS` correlated subquery per negated atom, appended to the
/// core's WHERE; correlation reuses the positive alias `t0`.
pub const NO_TAG: &str = "SELECT DISTINCT t0.\"id\" FROM \"Posting\" AS t0 WHERE NOT EXISTS (SELECT 1 FROM \"PostingTag\" AS n0 WHERE n0.\"posting\" = t0.\"id\" AND n0.\"tag\" = 0)";

/// `self_negation` — `Q(c) :- OrgParent(child = c, parent = p),
/// ¬OrgParent(child = p)`: children whose parent is a root. The negated
/// relation is also joined positively — the subquery's `n0` alias space
/// is disjoint from `t0..`, so the self-negation is aliased fresh.
pub const SELF_NEGATION: &str = "SELECT DISTINCT t0.\"child\" FROM \"OrgParent\" AS t0 WHERE NOT EXISTS (SELECT 1 FROM \"OrgParent\" AS n0 WHERE n0.\"child\" = t0.\"parent\")";

/// `in_three` — `Q(e) :- Posting(entry = e, account ∈ ?set0)` with the
/// set bound to `{3, 7, 9}`: the param-set family. The elements render
/// as literals, re-rendered per execution — prepared-statement parity is
/// not claimed for set-bound families.
pub const IN_THREE: &str =
    "SELECT DISTINCT t0.\"entry\" FROM \"Posting\" AS t0 WHERE t0.\"account\" IN (3, 7, 9)";

/// `in_empty` — the same query with the empty set: membership over
/// nothing is false, rendered as the honest constant (`IN ()` is
/// unwritable SQL and `IN (NULL)` is the three-valued NULL trap).
pub const IN_EMPTY: &str = "SELECT DISTINCT t0.\"entry\" FROM \"Posting\" AS t0 WHERE 1 = 0";

/// membership — `Q(o) :- Posting(account = a, at = t),
/// Mandate(account = a, org = o, active ∋ t)`: mandates active at some
/// posting's instant. The point variable's test lands after the atom
/// conjuncts (its scalar anchor may bind in any atom):
/// `start <= t AND t < end`.
pub const MEMBERSHIP: &str = "SELECT DISTINCT t1.\"org\" FROM \"Posting\" AS t0, \"Mandate\" AS t1 WHERE t0.\"account\" = t1.\"account\" AND t1.\"active_start\" <= t0.\"at\" AND t0.\"at\" < t1.\"active_end\"";

/// `membership_param` — `Q(o) :- Posting(account = ?0, at = ?1),
/// Mandate(account = ?0, org = o, active ∋ ?1)`: the at-instant probe
/// through a param. The instant's placeholder repeats (`?2` twice, one
/// bound value); the account param repeats across atoms (`?1` twice).
pub const MEMBERSHIP_PARAM: &str = "SELECT DISTINCT t1.\"org\" FROM \"Posting\" AS t0, \"Mandate\" AS t1 WHERE t0.\"account\" = ?1 AND t0.\"at\" = ?2 AND t1.\"account\" = ?1 AND t1.\"active_start\" <= ?2 AND ?2 < t1.\"active_end\"";

/// intersects — `Q(o1, o2) :- Mandate(account = a, org = o1, active = u),
/// Mandate(account = a, org = o2, active = v),
/// Allen(u, v, INTERSECTS)`: the mandate-intersection join. The mask
/// renders as its per-basic endpoint formulas OR'd (the 9 sharing
/// basics), under the query's SELECT DISTINCT — deliberately naive.
pub const INTERSECTS: &str = "SELECT DISTINCT t0.\"org\", t1.\"org\" FROM \"Mandate\" AS t0, \"Mandate\" AS t1 WHERE t0.\"account\" = t1.\"account\" AND ((t0.\"active_start\" < t1.\"active_start\" AND t1.\"active_start\" < t0.\"active_end\" AND t0.\"active_end\" < t1.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t0.\"active_end\" < t1.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_end\" < t1.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" < t1.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" < t1.\"active_start\" AND t1.\"active_end\" < t0.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t1.\"active_end\" < t0.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_start\" < t1.\"active_end\" AND t1.\"active_end\" < t0.\"active_end\"))";

/// `covers_param` — `Q(o) :- Mandate(org = o, active = v),
/// Allen(v, ?0, COVERS)` with `?0` anchored interval-typed by the
/// comparison: the 4-basic ⊇ composite (equals ∪ finished-by ∪ contains
/// ∪ started-by) OR'd over the param's halves as two placeholders.
pub const COVERS_PARAM: &str = "SELECT DISTINCT t0.\"org\" FROM \"Mandate\" AS t0 WHERE ((t0.\"active_start\" = ?1 AND t0.\"active_end\" = ?2) OR (t0.\"active_start\" < ?1 AND t0.\"active_end\" = ?2) OR (t0.\"active_start\" < ?1 AND ?2 < t0.\"active_end\") OR (t0.\"active_start\" = ?1 AND ?2 < t0.\"active_end\"))";

/// `contains_point` — `Q(o, t) :- Mandate(org = o, active = v),
/// Posting(at = t), Contains(v, t)`: point containment as a predicate —
/// the membership form over an already-bound term.
pub const CONTAINS_POINT: &str = "SELECT DISTINCT t0.\"org\", t1.\"at\" FROM \"Mandate\" AS t0, \"Posting\" AS t1 WHERE t0.\"active_start\" <= t1.\"at\" AND t1.\"at\" < t0.\"active_end\"";

/// `interval_eq` — `Q(a1, a2) :- Mandate(account = a1, active = u),
/// Mandate(account = a2, active = v), Eq(u, v)`: interval value equality
/// is pairwise equality on the halves.
pub const INTERVAL_EQ: &str = "SELECT DISTINCT t0.\"account\", t1.\"account\" FROM \"Mandate\" AS t0, \"Mandate\" AS t1 WHERE t0.\"active_start\" = t1.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\"";

/// `interval_eq_literal` — `Q(o) :- Mandate(org = o,
/// active = [1700, 1800))`: an interval literal in a binding is value
/// equality, split into its halves.
pub const INTERVAL_EQ_LITERAL: &str = "SELECT DISTINCT t0.\"org\" FROM \"Mandate\" AS t0 WHERE t0.\"active_start\" = 1700 AND t0.\"active_end\" = 1800";

/// `interval_eq_param` — `Q(o) :- Mandate(org = o, active = ?0)`: a
/// param anchored only by an interval-field position resolves to the
/// interval reading (value equality); its halves bind as two
/// placeholders, start then end.
pub const INTERVAL_EQ_PARAM: &str = "SELECT DISTINCT t0.\"org\" FROM \"Mandate\" AS t0 WHERE t0.\"active_start\" = ?1 AND t0.\"active_end\" = ?2";

/// `count_distinct` — `Q(h, CountDistinct(i)) :- Account(id = a,
/// holder = h), Posting(account = a, instrument = i)`: distinct
/// instruments per holder — `COUNT(DISTINCT x)` over the distinct full
/// binding set, never over the joined bag.
pub const COUNT_DISTINCT: &str = "SELECT v0, COUNT(DISTINCT v2) FROM (SELECT DISTINCT t0.\"holder\" AS v0, t0.\"id\" AS v1, t1.\"instrument\" AS v2 FROM \"Account\" AS t0, \"Posting\" AS t1 WHERE t0.\"id\" = t1.\"account\") GROUP BY v0";

/// `arg_max` — `Q(a, ArgMax_at(p)) :- Posting(id = p, account = a,
/// at = t)`: latest-posting-per-account, the join-back template. The
/// distinct binding set `d` joins its per-group `MAX` key; the outer
/// `SELECT DISTINCT` keeps ties on both sides.
pub const ARG_MAX: &str = "WITH d AS (SELECT DISTINCT t0.\"account\" AS v0, t0.\"id\" AS v1, t0.\"at\" AS v2 FROM \"Posting\" AS t0) SELECT DISTINCT d.v0, d.v1 FROM d JOIN (SELECT v0, MAX(v2) AS mk FROM d GROUP BY v0) m ON d.v0 = m.v0 AND d.v2 = m.mk";

/// chain — `Q(src, amount, at) :- Posting(entry = e, account = a,
/// amount, at), JournalEntry(id = e, source = src),
/// Account(id = a, currency = Usd)` with `at >= ?0`: the multi-hop walk
/// across postings/entries/accounts, a closed-vocabulary literal pinning the
/// account side.
pub const CHAIN: &str = "SELECT DISTINCT t1.\"source\", t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0, \"JournalEntry\" AS t1, \"Account\" AS t2 WHERE t0.\"entry\" = t1.\"id\" AND t0.\"account\" = t2.\"id\" AND t2.\"currency\" = 0 AND t0.\"at\" >= ?1";

/// range — `Q(id, amount) :- Posting(id, amount, at)` with
/// `at >= ?0, at < ?1`: the pure scan family.
pub const RANGE: &str = "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0 WHERE t0.\"at\" >= ?1 AND t0.\"at\" < ?2";

/// stats — `Q(c, Min(at), Max(amount), Count) :- Posting(account = a,
/// amount, at), Account(id = a, currency = c)`: the literal-free full
/// fold grouped by currency, over the normative distinct subquery.
pub const STATS: &str = "SELECT v0, MIN(v2), MAX(v1), COUNT(*) FROM (SELECT DISTINCT t1.\"currency\" AS v0, t0.\"amount\" AS v1, t0.\"at\" AS v2, t0.\"account\" AS v3 FROM \"Posting\" AS t0, \"Account\" AS t1 WHERE t0.\"account\" = t1.\"id\") GROUP BY v0";

/// string — `Q(id, amount) :- Posting(id, amount, instrument = i),
/// Instrument(id = i, symbol = ?0)`: the interned-string point lookup
/// (a never-interned miss rotates through).
pub const STRING: &str = "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0, \"Instrument\" AS t1 WHERE t0.\"instrument\" = t1.\"id\" AND t1.\"symbol\" = ?1";

/// skew — `Q(p, amount) :- Posting(id = p, amount),
/// PostingTag(posting = p, tag = ?0)`: the skewed tag join (tag 0 is
/// the generator's hot tag).
pub const SKEW: &str = "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0, \"PostingTag\" AS t1 WHERE t0.\"id\" = t1.\"posting\" AND t1.\"tag\" = ?1";

/// spread — `Q(x, y) :- Posting(entry = e, amount = x),
/// Posting(entry = e, amount = y)` with `x < y`: the duplicate-witness
/// projection (distinct entries can witness one `(x, y)` pair).
pub const SPREAD: &str = "SELECT DISTINCT t0.\"amount\", t1.\"amount\" FROM \"Posting\" AS t0, \"Posting\" AS t1 WHERE t0.\"entry\" = t1.\"entry\" AND t0.\"amount\" < t1.\"amount\"";

/// triangle — `Q(a) :- Posting(account = a, instrument = i),
/// Posting(entry = w, instrument = i), Posting(entry = w, account = a)`
/// with `?0 <= a < ?1`: the cyclic-ish self-join for WCOJ honesty.
pub const TRIANGLE: &str = "SELECT DISTINCT t0.\"account\" FROM \"Posting\" AS t0, \"Posting\" AS t1, \"Posting\" AS t2 WHERE t0.\"instrument\" = t1.\"instrument\" AND t1.\"entry\" = t2.\"entry\" AND t0.\"account\" = t2.\"account\" AND t0.\"account\" >= ?1 AND t0.\"account\" < ?2";

/// `postings_without_tag` — `Q(p, amount) :- Posting(id = p,
/// account = ?0, amount), ¬PostingTag(posting = p)`: one account's
/// untagged postings — the negation family.
pub const POSTINGS_WITHOUT_TAG: &str = "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0 WHERE t0.\"account\" = ?1 AND NOT EXISTS (SELECT 1 FROM \"PostingTag\" AS n0 WHERE n0.\"posting\" = t0.\"id\")";

/// `mandate_overlap` — `Q(a1, a2) :- Mandate(account = a1, org = ?0,
/// active = u), Mandate(account = a2, org = ?0, active = v),
/// Allen(u, v, INTERSECTS)`: account pairs whose mandates under one org
/// intersect in time — an Allen-mask **join** across accounts, not a
/// filter; the mask renders as its 9 sharing basics OR'd.
pub const MANDATE_OVERLAP: &str = "SELECT DISTINCT t0.\"account\", t1.\"account\" FROM \"Mandate\" AS t0, \"Mandate\" AS t1 WHERE t0.\"org\" = ?1 AND t1.\"org\" = ?1 AND ((t0.\"active_start\" < t1.\"active_start\" AND t1.\"active_start\" < t0.\"active_end\" AND t0.\"active_end\" < t1.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t0.\"active_end\" < t1.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_end\" < t1.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" < t1.\"active_start\" AND t0.\"active_end\" = t1.\"active_end\") OR (t0.\"active_start\" < t1.\"active_start\" AND t1.\"active_end\" < t0.\"active_end\") OR (t0.\"active_start\" = t1.\"active_start\" AND t1.\"active_end\" < t0.\"active_end\") OR (t1.\"active_start\" < t0.\"active_start\" AND t0.\"active_start\" < t1.\"active_end\" AND t1.\"active_end\" < t0.\"active_end\"))";

/// `arg_max_global` — `Q(ArgMax_at(p)) :- Posting(id = p, at = t)`: the
/// global-group variant omits the GROUP BY and the group join keys; an
/// empty `d` joins nothing (the NULL extreme matches no row), so the
/// empty input yields the empty set with no HAVING patch.
pub const ARG_MAX_GLOBAL: &str = "WITH d AS (SELECT DISTINCT t0.\"id\" AS v0, t0.\"at\" AS v1 FROM \"Posting\" AS t0) SELECT DISTINCT d.v0 FROM d JOIN (SELECT MAX(v1) AS mk FROM d) m ON d.v1 = m.mk";
