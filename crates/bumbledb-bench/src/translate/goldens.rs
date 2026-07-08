/// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`.
pub const POINT: &str =
    "SELECT DISTINCT t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0 WHERE t0.\"id\" = ?1";

/// `fk_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
/// Account(id = a, holder = h), Holder(id = h, name)` with the
/// posting's account equated to the account's id.
pub const FK_WALK: &str = "SELECT DISTINCT t2.\"name\", t0.\"amount\" FROM \"Posting\" AS t0, \"Account\" AS t1, \"Holder\" AS t2 WHERE t0.\"account\" = ?1 AND t1.\"id\" = ?1 AND t1.\"holder\" = t2.\"id\"";

/// balance — `Q(a, Sum(amount)) :- Posting(id, account = a, amount),
/// Account(id = a, holder = ?0)` — the id binding makes the fold a
/// true ledger balance (duplicate amounts count once each).
pub const BALANCE: &str = "SELECT v0, SUM(v1) FROM (SELECT DISTINCT t0.\"account\" AS v0, t0.\"amount\" AS v1, t0.\"id\" AS v2 FROM \"Posting\" AS t0, \"Account\" AS t1 WHERE t0.\"account\" = t1.\"id\" AND t1.\"holder\" = ?1) GROUP BY v0";

/// chain — `Q(region, amount, at) :- Posting(account = a, amount, at),
/// Account(id = a, holder = h, status = Open), Holder(id = h, region)`
/// with `at >= ?0`.
pub const CHAIN: &str = "SELECT DISTINCT t2.\"region\", t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0, \"Account\" AS t1, \"Holder\" AS t2 WHERE t0.\"account\" = t1.\"id\" AND t1.\"status\" = 0 AND t1.\"holder\" = t2.\"id\" AND t0.\"at\" >= ?1";

/// range — `Q(id, amount) :- Posting(id, amount, at)` with
/// `at >= ?0`, `at < ?1` — the pure scan family.
pub const RANGE: &str = "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0 WHERE t0.\"at\" >= ?1 AND t0.\"at\" < ?2";

/// stats — `Q(k, Min(at), Max(amount), Count) :- Posting(instrument =
/// i, amount, at), Instrument(id = i, kind = k)` — the literal-free
/// full fold.
pub const STATS: &str = "SELECT v0, MIN(v2), MAX(v1), COUNT(*) FROM (SELECT DISTINCT t1.\"kind\" AS v0, t0.\"amount\" AS v1, t0.\"at\" AS v2, t0.\"instrument\" AS v3 FROM \"Posting\" AS t0, \"Instrument\" AS t1 WHERE t0.\"instrument\" = t1.\"id\") GROUP BY v0";

/// string — `Q(id, amount) :- Posting(id, amount, memo = ?0)` — the
/// interned-string point family (misses included).
pub const STRING: &str =
    "SELECT DISTINCT t0.\"id\", t0.\"amount\" FROM \"Posting\" AS t0 WHERE t0.\"memo\" = ?1";

/// skew — `Q(label, amount) :- Posting(account = a, amount),
/// AccountTag(account = a, tag = t), Tag(id = t, label)` with
/// `label = ?0` — the small-side/hot-side shape where dynamic cover
/// choice decides.
pub const SKEW: &str = "SELECT DISTINCT t2.\"label\", t0.\"amount\" FROM \"Posting\" AS t0, \"AccountTag\" AS t1, \"Tag\" AS t2 WHERE t0.\"account\" = t1.\"account\" AND t1.\"tag\" = t2.\"id\" AND t2.\"label\" = ?1";

/// spread — `Q(x, y) :- Posting(transfer = t, amount = x),
/// Posting(transfer = t, amount = y)` with `x < y` — the cross-atom
/// residual family.
pub const SPREAD: &str = "SELECT DISTINCT t0.\"amount\", t1.\"amount\" FROM \"Posting\" AS t0, \"Posting\" AS t1 WHERE t0.\"transfer\" = t1.\"transfer\" AND t0.\"amount\" < t1.\"amount\"";

/// triangle — `Q(a) :- Posting(account = a, instrument = i),
/// Posting(instrument = i, transfer = w), Posting(transfer = w,
/// account = a)` with `?0 <= a < ?1` — the cyclic family.
pub const TRIANGLE: &str = "SELECT DISTINCT t0.\"account\" FROM \"Posting\" AS t0, \"Posting\" AS t1, \"Posting\" AS t2 WHERE t0.\"instrument\" = t1.\"instrument\" AND t1.\"transfer\" = t2.\"transfer\" AND t0.\"account\" = t2.\"account\" AND t0.\"account\" >= ?1 AND t0.\"account\" < ?2";
