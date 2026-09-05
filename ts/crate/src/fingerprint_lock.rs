//! Cross-host fingerprint lock: one theory, one pinned digest.
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
        id: u64 as HolderId,
        name: str,
        digest: bytes<16>,
        at: interval<u64>,
    }

    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        status: u64 as StatusId,
        active: interval<i64> as ActiveDuring,
        lease: interval<u64, 7> as Lease,
    }

    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }
    relation AuditTrail { account: u64 as AccountId, rate_bps: i64 }

    // The weighted-capacity extension (capacity cutover, dossier § 4.2):
    // the weight descriptor, the dependent bound, and the Duration pair
    // all enter the lock's encoding surface — statement for statement the
    // SDK twin's tail.
    relation Pool {
        id: u64 as PoolId,
        supply: u64,
        open: interval<u64>,
    }

    relation Device {
        id: u64 as DeviceId,
        pool: u64 as PoolId,
        watts: u64,
        ran: interval<u64>,
    }

    // The successor issues no database identity: the old `fresh` modifier
    // is DECLARED key statements now (`R(id) -> R;`) — same identity law,
    // spelled in the statement grammar the encoding surface hashes.
    Holder(id) -> Holder;
    Account(id) -> Account;
    Pool(id) -> Pool;
    Device(id) -> Device;
    SavingsTerms(account) -> SavingsTerms;
    Account(holder) <= Holder(id);
    Account(kind) <= Kind(id);
    Account(status) <= Status(id);
    Account(id | status == Frozen) == SavingsTerms(account);
    Holder(id | name == {"alpha", "beta"}) <= Holder(id);
    Holder(id | at == 5..18446744073709551615, digest == b"0123456789abcdef") <= Holder(id);
    SavingsTerms(account | rate_bps == -3) <= SavingsTerms(account);
    Holder(id) <={0..3} Account(holder);
    Holder(id) <=[Duration(active)]{2..*} Account(holder | status == Frozen);
    Holder(id) <={1} Account(holder | status == Open);
    Holder(id) <={0} Account(holder | kind == Failed);
    Holder(id) <={1..4} Account(holder | kind == DirectPass);
    Device(pool) <= Pool(id);
    Pool(id) <=[watts]{0..supply} Device(pool);
    Pool(id) <=[watts]{0..100} Device(pool);
    Pool(id) <=[watts]{1..*} Device(pool);
    Pool(id) <=[Duration(ran)]{0..Duration(open)} Device(pool);
    // Lock extension, statement for statement the SDK twin's tail:
    // the ψ-on-closed containment (the member set {DirectPass} folds at
    // validate) and the generator-less `==` pair — no key statement touches
    // `rate_bps`, so the TS side's class laws name that class by least
    // coordinate while the hash below proves they contribute zero bytes.
    Account(kind) <= Kind(id | mastered == true);
    SavingsTerms(account, rate_bps) -> SavingsTerms;
    AuditTrail(account, rate_bps) -> AuditTrail;
    SavingsTerms(account, rate_bps) == AuditTrail(account, rate_bps);
}

/// The pinned cross-host fingerprint of the `CrossHost` theory. The macro
/// twin must keep hashing to this constant: a silent encoding-surface change
/// in the engine (or the `schema!` grammar) is exactly the drift this lock
/// exists to catch. The 0.x TS twin (`test/fingerprint.test.ts`) died with
/// the fresh-modifier grammar; when the SDK regrows a fingerprint pin it
/// bakes THIS constant (F3 note in implementation/packets/P06.md).
/// `18446744073709551615` above is `u64::MAX` — the `at` selection literal
/// is the unbounded ray `[5, ∞)`.
const PIN: &str = "84481bd32f7182df21ea5aea542c05d13a7dd52fe378cdb05c4fef9558624d31";

/// A self-cleaning per-test store directory (the engine's integration
/// `TempDir` twin — this crate deliberately has no dev-dependencies). The
/// pid suffix keeps concurrent suite runs (other checkouts, co-tenant
/// agents) from wiping each other's live stores.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-node-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
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
        crate::hex_fingerprint(&fingerprint(&schema).0),
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
    drop(
        Db::create(&dir.0, CrossHost.descriptor())
            .expect("descriptor create")
            .expect("accepted"),
    );
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
