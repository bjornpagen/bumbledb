//! C11 canonical plan/manifest codec: identity is the canonical FRAME, the
//! label is human-only, formatting never changes a digest, and every tamper
//! (edit/reorder/relabel/prefix) refuses with its exact position.
//! Maps to MIG-07 (native half of TS-MIG-01), OPS-001. Verification: `NotRun`
//! (F1 authors, does not execute).

#[path = "migration_support/mod.rs"]
mod support;

use bumbledb::Value;
use bumbledb_log::migration::compile::{CompileError, compile};
use bumbledb_log::migration::manifest::{
    Manifest, ManifestError, append_entry, bind_plans, parse_manifest, plan_set_digest, prefix_at,
    render_manifest, verify_manifest,
};
use bumbledb_log::migration::plan::{
    Loss, Operation, PlanError, PlanExpr, StepLabel, canonical_plan_bytes, decode_plan, parse_plan,
    plan_digest, render_plan,
};
use bumbledb_log::schema_file::{self, schema_id};

use support::{
    CAP, base_schema, digest_of, manifest, pinned_schema, plan_pinned, plan_tagged, plan_unpin,
    tagged_schema,
};

#[test]
fn canonical_bytes_roundtrip_and_carry_the_family_header() {
    let plan = plan_pinned();
    let bytes = canonical_plan_bytes(&plan, CAP).unwrap();
    assert!(
        bytes.starts_with(b"bumbledb.migration.v1\0"),
        "family string leads the frame"
    );
    assert_eq!(bytes[22..24], [0x00, 0x01], "layout 1 big-endian");
    assert_eq!(bytes[24], 1, "kind: plan");
    let decoded = decode_plan(&bytes, CAP).unwrap();
    assert_eq!(decoded, plan);
    // Every strict prefix refuses; trailing bytes refuse.
    for end in 0..bytes.len() {
        assert!(decode_plan(&bytes[..end], CAP).is_err(), "prefix {end}");
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_plan(&trailing, CAP).is_err());
}

#[test]
fn json_formatting_never_changes_identity_but_content_always_does() {
    let plan = plan_pinned();
    let digest = digest_of(&plan);
    // Reparse of the deterministic rendering: identical plan, identical
    // digest, byte-stable text.
    let text = render_plan(&plan);
    let reparsed = parse_plan(&text).unwrap();
    assert_eq!(reparsed, plan);
    assert_eq!(digest_of(&reparsed), digest);
    assert_eq!(render_plan(&reparsed), text);
    // Whitespace-mangled but semantically identical JSON: same digest.
    let mangled = text.replace(",\n  ", ",\n\n      ");
    let from_mangled = parse_plan(&mangled).unwrap();
    assert_eq!(digest_of(&from_mangled), digest);
    // A changed label changes the digest (labels are canonical data)…
    let mut relabeled = plan.clone();
    relabeled.label = StepLabel::new("0000-note-pinned-x").unwrap();
    assert_ne!(digest_of(&relabeled), digest);
    // …and a changed literal changes the digest.
    let mut edited = plan.clone();
    if let Operation::MapRelation { fields, .. } = &mut edited.operations[0] {
        fields[2].expression = PlanExpr::Literal(Value::Bool(true));
    }
    assert_ne!(digest_of(&edited), digest);
}

#[test]
fn labels_are_bounded_lowercase_and_never_identity() {
    assert!(StepLabel::new("0001-ok-label").is_ok());
    for bad in ["", "UPPER", "space here", "unicode-π", &"x".repeat(65)] {
        assert!(StepLabel::new(bad).is_err(), "{bad:?}");
    }
    // Same label, different plan bytes: different identity — the digest,
    // not the label, is what history and freeze intents bind.
    let a = plan_pinned();
    let mut b = plan_pinned();
    if let Operation::MapRelation { fields, .. } = &mut b.operations[0] {
        fields[2].expression = PlanExpr::Literal(Value::Bool(true));
    }
    assert_eq!(a.label, b.label);
    assert_ne!(digest_of(&a), digest_of(&b));
}

#[test]
fn unsupported_plan_version_refuses_before_anything_else() {
    let text = render_plan(&plan_pinned()).replace("\"planVersion\": 1", "\"planVersion\": 2");
    assert!(matches!(
        parse_plan(&text),
        Err(PlanError::Shape("unsupported planVersion"))
    ));
}

#[test]
fn manifest_verifies_and_every_tamper_names_its_position() {
    let manifest = manifest();
    verify_manifest(&manifest, CAP).unwrap();
    // Edited plan digest at entry 1.
    let mut edited = manifest.clone();
    edited.entries[1].plan_digest[0] ^= 1;
    assert!(matches!(
        verify_manifest(&edited, CAP),
        Err(ManifestError::PrefixMismatch { at: 1 })
    ));
    // Reordered entries: the sequence gap is detected first.
    let mut reordered = manifest.clone();
    reordered.entries.swap(0, 1);
    assert!(matches!(
        verify_manifest(&reordered, CAP),
        Err(ManifestError::SequenceGap { at: 0 })
    ));
    // A deleted entry breaks contiguity.
    let mut truncated = manifest.clone();
    truncated.entries.remove(0);
    assert!(matches!(
        verify_manifest(&truncated, CAP),
        Err(ManifestError::SequenceGap { at: 0 })
    ));
    // An edited recorded prefix refuses at its own position.
    let mut prefix_edit = manifest.clone();
    prefix_edit.entries[0].prefix_digest[31] ^= 0x80;
    assert!(matches!(
        verify_manifest(&prefix_edit, CAP),
        Err(ManifestError::PrefixMismatch { at: 0 })
    ));
    // A broken schema chain refuses.
    let mut chain_edit = manifest.clone();
    chain_edit.entries[1].from_schema.0[0] ^= 1;
    assert!(matches!(
        verify_manifest(&chain_edit, CAP),
        Err(ManifestError::SchemaChainBroken { at: 1 })
    ));
    // A duplicate label refuses even with correct digests elsewhere.
    let mut relabeled = manifest.clone();
    relabeled.entries[1].label = relabeled.entries[0].label.clone();
    assert!(matches!(
        verify_manifest(&relabeled, CAP),
        Err(ManifestError::DuplicateLabel { at: 1 } | ManifestError::PrefixMismatch { at: 1 })
    ));
}

#[test]
fn manifest_json_roundtrips_and_binds_its_base_digest() {
    let manifest = manifest();
    let text = render_manifest(&manifest, CAP).unwrap();
    let reparsed = parse_manifest(&text, CAP).unwrap();
    assert_eq!(reparsed, manifest);
    assert_eq!(render_manifest(&reparsed, CAP).unwrap(), text);
    // A doctored recorded base prefix digest refuses at parse.
    let doctored = text.replacen("\"basePrefixDigest\": \"", "\"basePrefixDigest\": \"00", 1);
    assert!(parse_manifest(&doctored, CAP).is_err());
}

#[test]
fn plan_binding_rejects_swapped_and_foreign_plans() {
    let manifest = manifest();
    let pinned = plan_pinned();
    let tagged = plan_tagged();
    bind_plans(&manifest, 0, &[&pinned, &tagged], CAP).unwrap();
    bind_plans(&manifest, 1, &[&tagged], CAP).unwrap();
    // Swapped order refuses on the first digest mismatch.
    assert!(matches!(
        bind_plans(&manifest, 0, &[&tagged, &pinned], CAP),
        Err(ManifestError::PlanDigestMismatch { at: 0 })
    ));
    // A non-suffix region refuses.
    assert!(matches!(
        bind_plans(&manifest, 2, &[&pinned], CAP),
        Err(ManifestError::NotASuffix)
    ));
    // Same bytes, different recorded sequence: entry mismatch. (The digest
    // covers the sequence, so this shows as a digest mismatch.)
    let mut renumbered = plan_pinned();
    renumbered.sequence = 1;
    assert!(matches!(
        bind_plans(&manifest, 1, &[&renumbered], CAP),
        Err(ManifestError::PlanDigestMismatch { at: 1 })
    ));
}

#[test]
fn plan_set_digest_binds_prefix_suffix_and_schemas() {
    let manifest = manifest();
    let whole = plan_set_digest(&manifest, 0, 2, CAP).unwrap();
    let first = plan_set_digest(&manifest, 0, 1, CAP).unwrap();
    let second = plan_set_digest(&manifest, 1, 1, CAP).unwrap();
    assert_ne!(whole, first);
    assert_ne!(whole, second);
    assert_ne!(first, second);
    // Empty and out-of-range suffixes refuse.
    assert!(matches!(
        plan_set_digest(&manifest, 0, 0, CAP),
        Err(ManifestError::NotASuffix)
    ));
    assert!(matches!(
        plan_set_digest(&manifest, 1, 2, CAP),
        Err(ManifestError::NotASuffix)
    ));
    // The prefix ladder is acyclic and reproducible.
    assert_eq!(
        prefix_at(&manifest, 2, CAP).unwrap(),
        manifest.entries[1].prefix_digest
    );
}

#[test]
fn append_entry_computes_exactly_what_verification_recomputes() {
    let mut grown = Manifest {
        base_schema: schema_id(&base_schema()).unwrap(),
        entries: vec![],
    };
    let entry = append_entry(&mut grown, &plan_pinned(), CAP).unwrap();
    assert_eq!(entry.plan_digest, digest_of(&plan_pinned()));
    verify_manifest(&grown, CAP).unwrap();
    // Appending out of order refuses.
    let mut wrong_seq = plan_tagged();
    wrong_seq.sequence = 5;
    assert!(matches!(
        append_entry(&mut grown, &wrong_seq, CAP),
        Err(ManifestError::SequenceGap { at: 1 })
    ));
}

#[test]
fn compile_requires_total_coverage_exact_types_and_the_final_validate() {
    // The happy paths.
    compile(&plan_pinned(), &base_schema(), &pinned_schema()).unwrap();
    compile(&plan_tagged(), &pinned_schema(), &tagged_schema()).unwrap();
    // Missing final validate.
    let mut no_validate = plan_pinned();
    no_validate.operations.pop();
    assert!(matches!(
        compile(&no_validate, &base_schema(), &pinned_schema()),
        Err(CompileError::MissingFinalValidate)
    ));
    // Validate naming the wrong schema.
    let mut wrong_validate = plan_pinned();
    if let Some(Operation::ValidateSchema { schema }) = wrong_validate.operations.last_mut() {
        schema.0[0] ^= 1;
    }
    assert!(matches!(
        compile(&wrong_validate, &base_schema(), &pinned_schema()),
        Err(CompileError::WrongFinalValidate)
    ));
    // A source relation left unconsumed refuses (nothing vanishes quietly).
    let mut uncovered = plan_pinned();
    uncovered.operations.remove(0);
    assert!(matches!(
        compile(&uncovered, &base_schema(), &pinned_schema()),
        Err(CompileError::MissingSourceCoverage { .. } | CompileError::MissingTargetCoverage { .. })
    ));
    // A type-mismatched backfill refuses with the core scalar judgment.
    let mut mistyped = plan_pinned();
    if let Operation::MapRelation { fields, .. } = &mut mistyped.operations[0] {
        fields[2].expression = PlanExpr::Literal(Value::U64(1));
    }
    assert!(matches!(
        compile(&mistyped, &base_schema(), &pinned_schema()),
        Err(CompileError::Type { .. })
    ));
    // Field maps must follow target declaration order exactly.
    let mut permuted = plan_pinned();
    if let Operation::MapRelation { fields, .. } = &mut permuted.operations[0] {
        fields.swap(0, 1);
    }
    assert!(matches!(
        compile(&permuted, &base_schema(), &pinned_schema()),
        Err(CompileError::FieldCoverage { .. })
    ));
    // An unknown source field refuses by name.
    let mut unknown = plan_pinned();
    if let Operation::MapRelation { fields, .. } = &mut unknown.operations[0] {
        fields[0].expression = PlanExpr::Field("nope".into());
    }
    assert!(matches!(
        compile(&unknown, &base_schema(), &pinned_schema()),
        Err(CompileError::UnknownField { .. })
    ));
    // The wrong descriptor for a recorded schema id refuses before work.
    assert!(matches!(
        compile(&plan_pinned(), &pinned_schema(), &pinned_schema()),
        Err(CompileError::SchemaIdMismatch { which: "from" })
    ));
}

#[test]
fn destructive_loss_needs_exactly_matching_acknowledgement() {
    // Dropping the `pinned` field without intent refuses…
    assert!(matches!(
        compile(&plan_unpin(false), &pinned_schema(), &base_schema()),
        Err(CompileError::MissingLossAck { .. })
    ));
    // …with intent it compiles…
    compile(&plan_unpin(true), &pinned_schema(), &base_schema()).unwrap();
    // …and a stale acknowledgement that acknowledges nothing refuses.
    let mut stale = plan_unpin(true);
    stale.destructive.push(Loss {
        relation: "Note".into(),
        field: Some("body".into()),
    });
    assert!(matches!(
        compile(&stale, &pinned_schema(), &base_schema()),
        Err(CompileError::StaleLossAck { .. })
    ));
}

#[test]
fn seed_rows_are_shape_checked_and_only_follow_their_producer() {
    // Arity mismatch refuses.
    let mut bad_arity = plan_tagged();
    if let Operation::Seed { rows, .. } = &mut bad_arity.operations[2] {
        rows.push(Box::from([
            Value::String("a".into()),
            Value::String("b".into()),
        ]));
    }
    assert!(matches!(
        compile(&bad_arity, &pinned_schema(), &tagged_schema()),
        Err(CompileError::SeedArity { .. })
    ));
    // Type mismatch refuses.
    let mut bad_type = plan_tagged();
    if let Operation::Seed { rows, .. } = &mut bad_type.operations[2] {
        rows[0] = Box::from([Value::U64(7)]);
    }
    assert!(matches!(
        compile(&bad_type, &pinned_schema(), &tagged_schema()),
        Err(CompileError::ValueShape { .. })
    ));
    // A seed before its producing operation refuses.
    let mut early = plan_tagged();
    early.operations.swap(1, 2);
    assert!(matches!(
        compile(&early, &pinned_schema(), &tagged_schema()),
        Err(CompileError::SeedBeforeProduce { .. })
    ));
}

#[test]
fn schema_file_render_parse_agree_with_the_plan_codec_identities() {
    // The one schema grammar: the ids plans cite are exactly what
    // schema_file computes for the rendered snapshots.
    for descriptor in [base_schema(), pinned_schema(), tagged_schema()] {
        let text = schema_file::render(&descriptor);
        let reparsed = schema_file::parse(&text).unwrap();
        assert_eq!(reparsed, descriptor);
        assert_eq!(
            schema_id(&reparsed).unwrap(),
            schema_id(&descriptor).unwrap()
        );
    }
    let plan = plan_pinned();
    assert_eq!(plan.from_schema, schema_id(&base_schema()).unwrap());
    assert_eq!(plan.to_schema, schema_id(&pinned_schema()).unwrap());
}

#[test]
fn plan_digest_is_domain_separated_from_every_other_role() {
    let bytes = canonical_plan_bytes(&plan_pinned(), CAP).unwrap();
    let digest = plan_digest(&bytes);
    // Not the raw blake3, not another domain: hashing the same bytes under
    // a different role can never alias plan identity.
    assert_ne!(digest, *blake3::hash(&bytes).as_bytes());
    assert_ne!(
        digest,
        blake3::derive_key(bumbledb_log::migration::PREFIX_DIGEST_DOMAIN, &bytes)
    );
}
