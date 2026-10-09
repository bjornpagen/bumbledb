//! Every log artifact decodes totally: valid frames round-trip, and hostile
//! bytes (truncations, single-byte mutations, random noise, giant counts)
//! are refused or decode to a value that re-encodes to exactly those bytes.

use bumbledb::{Schema, SchemaFingerprint};
use bumbledb_log::{
    Body, CheckpointKey, Command, CommandDigest, CommandRef, Comparison, DatabaseId, Decided,
    Delta, Entry, Evidence, FrameError, Freeze, Genesis, Head, ImageDigest, Ledger, Migration,
    Millis, Mode, Nonce, Outcome, Precondition, Receipt, Rejection, RequestId, Revision, Seq, Thaw,
    Verdict,
};

use crate::support::{Rng, items, migration_id, schema};

fn rejection() -> Rejection {
    Rejection {
        migration: migration_id("0002_bad"),
        evidence: Evidence::new(vec![7, 7, 7].into()).unwrap(),
    }
}

fn entries(schema: &Schema) -> Vec<Entry> {
    let nonce = Nonce([9; 16]);
    let command = |request: u8| CommandRef {
        request: RequestId([request; 16]),
        digest: CommandDigest([request.wrapping_mul(3); 32]),
    };
    let bodies = vec![
        Body::Genesis(Genesis {
            database: DatabaseId([1; 16]),
            initial: migration_id("0000_init"),
            schema: SchemaFingerprint([2; 32]),
        }),
        Body::Commands(
            vec![
                Decided {
                    command: command(1),
                    verdict: Verdict::Committed {
                        changes: items(schema, &[(1, 10), (2, 20)], &[(3, 30)]),
                        delta: Delta::new(2, 1).unwrap(),
                    },
                },
                Decided {
                    command: command(2),
                    verdict: Verdict::NoChange,
                },
                Decided {
                    command: command(3),
                    verdict: Verdict::PreconditionFailed {
                        expected: Revision(4),
                        observed: Revision(5),
                    },
                },
                Decided {
                    command: command(4),
                    verdict: Verdict::InvariantRejected(
                        Evidence::new(vec![1, 2, 3].into()).unwrap(),
                    ),
                },
            ]
            .into(),
        ),
        Body::Freeze(Freeze {
            migration: migration_id("0001_tags"),
            lease_millis: 60_000,
        }),
        Body::Migration(Migration {
            migration: migration_id("0001_tags"),
            schema: SchemaFingerprint([4; 32]),
            image: ImageDigest([5; 32]),
        }),
        Body::Thaw(Thaw::Lifted),
        Body::Thaw(Thaw::Rejected(rejection())),
    ];
    bodies
        .into_iter()
        .map(|body| Entry { nonce, body })
        .collect()
}

fn receipts() -> Vec<Receipt> {
    let command = CommandRef {
        request: RequestId([6; 16]),
        digest: CommandDigest([8; 32]),
    };
    [
        Outcome::Committed(Delta::new(0, 3).unwrap()),
        Outcome::NoChange,
        Outcome::PreconditionFailed {
            expected: Revision(1),
            observed: Revision(2),
        },
        Outcome::InvariantRejected(Evidence::new(vec![4; 40].into()).unwrap()),
    ]
    .into_iter()
    .map(|outcome| Receipt {
        command,
        seq: Seq::new(17).unwrap(),
        revision: Revision(9),
        outcome,
    })
    .collect()
}

fn heads() -> Vec<Head> {
    let open = Head {
        database: DatabaseId([3; 16]),
        seq: Seq::new(12).unwrap(),
        revision: Revision(7),
        schema: SchemaFingerprint([1; 32]),
        ledger: Ledger {
            applied: vec![migration_id("0000_init"), migration_id("0001_tags")],
            rejected: vec![rejection()],
        },
        mode: Mode::Open,
    };
    let frozen = Head {
        mode: Mode::Frozen {
            freeze: Freeze {
                migration: migration_id("0002_bad"),
                lease_millis: 5,
            },
            since: Millis(1_700_000_000_000),
        },
        ..open.clone()
    };
    vec![open, frozen]
}

/// Every prefix and every single-byte mutation of `valid` is refused or
/// decodes to a value that re-encodes to exactly the mutated bytes.
fn sweep(valid: &[u8], decode: impl Fn(&[u8]) -> Result<Vec<u8>, FrameError>) {
    assert_eq!(decode(valid).as_deref(), Ok(valid), "round trip");
    for len in 0..valid.len() {
        assert!(
            decode(&valid[..len]).is_err(),
            "prefix of {len} bytes decoded"
        );
    }
    let mut mutated = valid.to_vec();
    for at in 0..valid.len() {
        for mask in [0x01, 0x80, 0xff] {
            mutated[at] ^= mask;
            if let Ok(encoded) = decode(&mutated) {
                assert_eq!(encoded, mutated, "noncanonical decode at byte {at}");
            }
            mutated[at] ^= mask;
        }
    }
    let mut extended = valid.to_vec();
    extended.push(0);
    assert_eq!(decode(&extended), Err(FrameError::Trailing));
}

#[test]
fn entries_round_trip_and_refuse_hostile_bytes() {
    let schema = schema();
    for entry in entries(&schema) {
        sweep(&entry.encode(), |bytes| {
            Entry::parse(&schema, bytes).map(|entry| entry.encode())
        });
    }
}

#[test]
fn receipts_and_heads_round_trip_and_refuse_hostile_bytes() {
    for receipt in receipts() {
        let bytes = receipt.encode();
        assert_eq!(Receipt::decode(&bytes).unwrap(), receipt);
        sweep(&bytes, |bytes| {
            Receipt::decode(bytes).map(|receipt| receipt.encode())
        });
    }
    for head in heads() {
        let bytes = head.encode();
        assert_eq!(Head::decode(&bytes).unwrap(), head);
        sweep(&bytes, |bytes| {
            Head::decode(bytes).map(|head| head.encode())
        });
    }
}

#[test]
fn commands_bind_their_digest_and_refuse_hostile_bytes() {
    let schema = schema();
    let changes = items(&schema, &[(1, 1)], &[]);
    let command = Command::seal(RequestId([1; 16]), Precondition::None, changes.clone());
    let parsed = Command::parse(&schema, &command.encode()).unwrap();
    assert_eq!(parsed.reference(), command.reference());
    let conditional = Command::seal(
        RequestId([1; 16]),
        Precondition::ExactRevision(Revision(0)),
        changes,
    );
    assert_ne!(conditional.reference().digest, command.reference().digest);
    sweep(&conditional.encode(), |bytes| {
        Command::parse(&schema, bytes).map(|command| command.encode())
    });
}

#[test]
fn random_noise_never_decodes_into_a_panic() {
    let schema = schema();
    let mut rng = Rng::new(0x5eed);
    let valid = entries(&schema)[1].encode();
    for round in 0..4_000 {
        let len = usize::try_from(rng.below(96)).unwrap();
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.next().to_be_bytes()[0]).collect();
        if round % 2 == 0 {
            // Keep a valid header so the noise reaches the field decoders.
            let header = valid.len().min(22);
            bytes.splice(0..0, valid[..header].iter().copied());
        }
        let _ = Entry::parse(&schema, &bytes);
        let _ = Receipt::decode(&bytes);
        let _ = Head::decode(&bytes);
        let _ = Command::parse(&schema, &bytes);
    }
}

#[test]
fn giant_counts_are_refused_before_allocating() {
    let schema = schema();
    let mut bytes = entries(&schema)[1].encode();
    // magic(4) kind(1) nonce(16) tag(1), then the command count.
    bytes[22..26].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        Entry::parse(&schema, &bytes).err(),
        Some(FrameError::Length)
    );
    let mut empty = entries(&schema)[1].encode();
    empty.truncate(22);
    empty.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(Entry::parse(&schema, &empty).err(), Some(FrameError::Value));
}

#[test]
fn checkpoint_keys_list_newest_first_and_parse_canonically() {
    let key = |seq| CheckpointKey {
        seq: Seq::new(seq).unwrap(),
        schema: SchemaFingerprint([0xab; 32]),
        digest: ImageDigest([0x01; 32]),
    };
    let mut keys: Vec<String> = [3, 1_000, 42, 7]
        .into_iter()
        .map(|seq| key(seq).format())
        .collect();
    keys.sort();
    let order: Vec<u64> = keys
        .iter()
        .map(|text| CheckpointKey::parse(text).unwrap().seq.get())
        .collect();
    assert_eq!(order, [1_000, 42, 7, 3]);
    let text = key(42).format();
    assert_eq!(CheckpointKey::parse(&text), Some(key(42)));
    assert_eq!(CheckpointKey::parse(&text.to_uppercase()), None);
    assert_eq!(CheckpointKey::parse(&text[..text.len() - 1]), None);
    assert_eq!(CheckpointKey::parse(&format!("{text}-x")), None);
    assert_eq!(CheckpointKey::parse("ckpt/18446744073709551615-x-y"), None);
}

#[test]
fn the_ledger_compares_four_ways() {
    let ids = |names: &[&str]| {
        names
            .iter()
            .map(|name| migration_id(name))
            .collect::<Vec<_>>()
    };
    let ledger = Ledger {
        applied: ids(&["a", "b"]),
        rejected: Vec::new(),
    };
    assert_eq!(ledger.compare(&ids(&["a", "b"])), Comparison::Equal);
    assert_eq!(
        ledger.compare(&ids(&["a", "b", "c"])),
        Comparison::Behind { next: 2 }
    );
    assert_eq!(ledger.compare(&ids(&["a"])), Comparison::Ahead);
    assert_eq!(
        ledger.compare(&ids(&["a", "x", "c"])),
        Comparison::Diverged { index: 1 }
    );
    let mut tampered = ids(&["a", "b"]);
    tampered[1].hash.0[0] ^= 1;
    assert_eq!(ledger.compare(&tampered), Comparison::Diverged { index: 1 });
}
