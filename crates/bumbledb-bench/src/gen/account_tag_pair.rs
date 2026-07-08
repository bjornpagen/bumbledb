use crate::gen::Sizes;

/// One `AccountTag` pair by index — shared by `AccountTag` and `TagNote`
/// (the subset-by-construction compound FK). Two distinct pairs per
/// account, without rejection: pair `k` of account `a` uses tag
/// `(a + k*97) % tags` (97 coprime to 256) — except that hot accounts
/// always carry **tag 0** as their `k = 0` pair (the skew family's
/// guarantee, docs/architecture/50-validation.md).
///
/// # Panics
///
/// Only on a programmer-invariant violation: a hot-account count reaching
/// `tags - 97`, where a hot account's `k = 1` tag would collide with the
/// pinned tag 0 (the scale table tops out at 50 hot accounts).
#[must_use]
pub fn account_tag_pair(sizes: &Sizes, i: u64) -> (u64, u64) {
    let account = i / 2;
    let k = i % 2;
    assert!(
        sizes.hot_accounts() < sizes.tags - 97,
        "hot-account ids stay below the k = 1 collision point"
    );
    let tag = if k == 0 && account < sizes.hot_accounts() {
        0
    } else {
        (account + k * 97) % sizes.tags
    };
    (account, tag)
}
