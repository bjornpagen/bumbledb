//! The cross-host fingerprint lock (bumbledb TODO.md §7, the pin the SDK
//! owes): ONE theory exercising every schema construct — fresh keys, `str`,
//! `bytes<N>`, general and fixed-width intervals INCLUDING a ray literal,
//! both closed tiers, containment with σ on both faces, a ψ-selected CLOSED
//! target (`Kind(id | mastered == true)` — the member set folds at
//! validate), `==` mirrors including a generator-less pair (`SavingsTerms ==
//! AuditTrail` over columns no mint touches — the TS side's law-computed
//! class names never leak into the hash), and every legal window spelling —
//! declared here through the engine's `schema!` macro and, in
//! `test/fingerprint.test.ts`, through the SDK's constructors. Each side independently asserts its engine-computed
//! fingerprint equals the ONE pinned constant [`PIN`], so `cargo test` and
//! `node --test` each run standalone while jointly proving the cross-host
//! bond: identical fingerprints mean `Db::open` on either side admits the
//! other side's store — the fingerprint is open's whole schema gate beyond
//! format version and store kind — and neither surface can fake the pin,
//! because each hex arrives through its own full pipeline (macro expansion →
//! validation → blake3 here; SDK lowering → napi marshaling → the same
//! engine across the FFI there).
//!
//! The store roundtrip below additionally drives the exact open lanes: a
//! store CREATED through the bridge's own typestate (`Db<SchemaDescriptor>`
//! — every JS-created store is this) opens under the macro twin, a
//! macro-created store opens under the runtime descriptor, and a twisted
//! twin is refused as `SchemaMismatch` — the lock has teeth.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::{Db, Theory as _};

bumbledb::schema! {
    pub CrossHost;

    closed relation Status as StatusId = { Open, Frozen };

    closed relation Kind as KindId {
        mastered: bool,
        weight: u64,
        span: interval<u64>,
    } = {
        DirectPass { mastered: true, weight: 2, span: 1..3 },
        Failed     { mastered: false, weight: 5, span: 3..5 },
    };

    relation Holder {
        id: u64 as HolderId, fresh,
        name: str,
        digest: bytes<16>,
        at: interval<u64>,
    }

    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        status: u64 as StatusId,
        active: interval<i64> as ActiveDuring,
        lease: interval<u64, 7> as Lease,
    }

    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }
    relation AuditTrail { account: u64 as AccountId, rate_bps: i64 }

    SavingsTerms(account) -> SavingsTerms;
    Account(holder) <= Holder(id);
    Account(kind) <= Kind(id);
    Account(status) <= Status(id);
    Account(id | status == Frozen) == SavingsTerms(account);
    Holder(id | name == {"alpha", "beta"}) <= Holder(id);
    Holder(id | at == 5..18446744073709551615, digest == b"0123456789abcdef") <= Holder(id);
    SavingsTerms(account | rate_bps == -3) <= SavingsTerms(account);
    Holder(id) <={0..3} Account(holder);
    Holder(id) <={2..*} Account(holder | status == Frozen);
    Holder(id) <={1} Account(holder | status == Open);
    Holder(id) <={0} Account(holder | kind == Failed);
    Holder(id) <={1..4} Account(holder | kind == DirectPass);
    // PRD-K7's lock extension, statement for statement the SDK twin's tail:
    // the ψ-on-closed containment (the member set {DirectPass} folds at
    // validate) and the generator-less `==` pair — no fresh field touches
    // `rate_bps`, so the TS side's class laws name that class by least
    // coordinate while the hash below proves they contribute zero bytes.
    Account(kind) <= Kind(id | mastered == true);
    SavingsTerms(account, rate_bps) -> SavingsTerms;
    AuditTrail(account, rate_bps) -> AuditTrail;
    SavingsTerms(account, rate_bps) == AuditTrail(account, rate_bps);
}

/// The pinned cross-host fingerprint of the `CrossHost` theory. The SAME
/// constant is baked into `test/fingerprint.test.ts`; a change here without
/// the twin change there (or vice versa) is exactly the drift this lock
/// exists to catch. `18446744073709551615` above is `u64::MAX` — the `at`
/// selection literal is the unbounded ray `[5, ∞)`
/// (`docs/architecture/10-data-model.md`).
const PIN: &str = "b330d46f8cf6c91d8e24a6d2c3f9cbde65c2c37f1b90eaffdc3e49a8ae346b0c";

/// The 64-char lowercase hex the JS side receives from `dbFingerprint`.
fn hex_of(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A self-cleaning per-test store directory (the engine's integration
/// `TempDir` twin — this crate deliberately has no dev-dependencies).
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-node-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_macro_twin_hashes_to_the_pinned_fingerprint() {
    let schema = CrossHost
        .descriptor()
        .validate()
        .expect("the twin theory seals");
    assert_eq!(
        hex_of(&fingerprint(&schema).0),
        PIN,
        "the schema! twin must hash to the cross-host pin \
         (test/fingerprint.test.ts carries the same constant)"
    );
}

#[test]
fn the_bridge_typestate_and_the_macro_twin_open_each_other_s_stores() {
    let dir = TempDir::new("fingerprint-lock");

    // Created through the bridge's exact typestate (`Db<SchemaDescriptor>`
    // — what every JS `dbCreate` produces), opened under the macro twin.
    drop(Db::create(&dir.0, CrossHost.descriptor()).expect("descriptor create"));
    drop(Db::open(&dir.0, CrossHost).expect("the macro twin opens the descriptor-created store"));

    // And the runtime lane (the bridge's `dbOpen`) reopens it as well.
    drop(Db::open(&dir.0, CrossHost.descriptor()).expect("descriptor reopen"));

    // Teeth: a twisted twin (one statement fewer) is the typed refusal.
    let mut twisted = CrossHost.descriptor();
    twisted.statements.pop();
    match Db::open(&dir.0, twisted).map(|_| ()) {
        Err(bumbledb::Error::SchemaMismatch { .. }) => {}
        other => panic!("a twisted twin must refuse as SchemaMismatch, got {other:?}"),
    }
}
