//! Raw-ABI tests: every case calls the extern "C"
//! surface exactly as C would — views built in the test frame, callbacks
//! as C function pointers, errors destroyed through the ABI. No C harness
//! needed; the point is that the FOREIGN BRIDGE is correct.
#![expect(
    unsafe_code,
    reason = "the raw-ABI tests play the C caller: context pointers round-trip \
              through *mut c_void exactly as the header contract describes"
)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use bumbledb::RelationId;
use bumbledb::schema::StatementDescriptor;

use crate::answers::{
    bdb_answers_arity, bdb_answers_clear, bdb_answers_destroy, bdb_answers_get, bdb_answers_len,
    bdb_answers_new, bdb_instance_execute,
};
use crate::db::{
    bdb_admission_tag, bdb_db, bdb_db_admission, bdb_db_create, bdb_db_destroy, bdb_db_ephemeral,
    bdb_db_fingerprint, bdb_db_open, bdb_db_read, bdb_db_write, bdb_db_write_from, bdb_fingerprint,
    bdb_fresh_range, bdb_fresh_range_tag, bdb_instance_admission, bdb_instance_builder,
    bdb_instance_builder_admit, bdb_instance_builder_new, bdb_instance_contains, bdb_instance_get,
    bdb_instance_ref, bdb_instance_scan, bdb_mutation_report, bdb_owned_instance_destroy,
    bdb_row_set, bdb_row_set_arity, bdb_row_set_destroy, bdb_row_set_get, bdb_row_set_len,
    bdb_tx_contains, bdb_tx_delete, bdb_tx_get, bdb_tx_insert, bdb_tx_ref, bdb_tx_reserve,
    bdb_witness, bdb_write_admission, test_only_trigger_panic,
};
use crate::error::{
    bdb_error, bdb_error_destroy, bdb_error_get_kind, bdb_error_get_message, bdb_error_kind,
    bdb_statement_kind, bdb_violation, bdb_violations_destroy,
    bdb_violations_get, bdb_violations_len,
};
use crate::query::{
    bdb_agg_op, bdb_atom, bdb_atom_source_kind, bdb_binding, bdb_cmp_op, bdb_cmp_op_kind,
    bdb_comparison, bdb_condition, bdb_condition_kind, bdb_cq, bdb_db_prepare, bdb_find_term,
    bdb_find_term_kind, bdb_head_op, bdb_head_term, bdb_head_term_kind, bdb_prepared,
    bdb_prepared_destroy, bdb_query, bdb_query_kind, bdb_query_payload, bdb_rule, bdb_term,
    bdb_term_kind,
};
use crate::schema::{
    bdb_bound, bdb_bound_kind, bdb_capacity_window, bdb_capacity_window_kind, bdb_field_spec,
    bdb_interval_element, bdb_relation_spec, bdb_schema_spec, bdb_side, bdb_statement_spec,
    bdb_statement_spec_kind, bdb_value_type, bdb_value_type_kind, bdb_weight, bdb_weight_kind,
    schema_spec_in,
};
use crate::value::{bdb_param, bdb_param_kind, bdb_string_view, bdb_value, bdb_value_kind};
use crate::{bdb_abi_version, bdb_callback_control, bdb_status, bdb_version};

// ---------------------------------------------------------------------------
// C-caller plumbing
// ---------------------------------------------------------------------------

fn sv(text: &str) -> bdb_string_view {
    bdb_string_view {
        data: text.as_ptr(),
        len: text.len(),
    }
}

fn no_sv() -> bdb_string_view {
    bdb_string_view {
        data: null(),
        len: 0,
    }
}

fn v_bool(v: bool) -> bdb_value {
    let mut value = bdb_value::blank(bdb_value_kind::Bool);
    value.bool_value = u8::from(v);
    value
}

fn v_u64(v: u64) -> bdb_value {
    let mut value = bdb_value::blank(bdb_value_kind::U64);
    value.u64_value = v;
    value
}

fn v_i64(v: i64) -> bdb_value {
    let mut value = bdb_value::blank(bdb_value_kind::I64);
    value.i64_value = v;
    value
}

fn v_str(text: &str) -> bdb_value {
    let mut value = bdb_value::blank(bdb_value_kind::String);
    value.string_value = sv(text);
    value
}

fn v_interval_i64(start: i64, end: i64) -> bdb_value {
    let mut value = bdb_value::blank(bdb_value_kind::IntervalI64);
    value.interval_i64_start = start;
    value.interval_i64_end = end;
    value
}

/// A fresh store path under the system temp root. The path must not
/// exist: `create` refuses any existing destination, including an empty
/// directory.
fn temp_store(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-c-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Destroys an ABI error, asserting the destroy path itself.
fn destroy_error(error: *mut bdb_error) {
    assert!(!error.is_null(), "expected an error payload");
    assert_eq!(bdb_error_destroy(error), bdb_status::Ok);
}

fn error_kind(error: *const bdb_error) -> bdb_error_kind {
    bdb_error_get_kind(error)
}

fn error_message(error: *const bdb_error) -> String {
    let mut view = no_sv();
    assert_eq!(bdb_error_get_message(error, &raw mut view), bdb_status::Ok);
    let bytes = unsafe { std::slice::from_raw_parts(view.data, view.len) };
    String::from_utf8(bytes.to_vec()).expect("error message is UTF-8")
}

// Closure-to-C-callback trampolines: the generic F is smuggled through the
// context pointer, exactly as a C caller smuggles its state.
extern "C" fn read_trampoline<F: FnMut(*const bdb_instance_ref, *const bdb_witness) -> bdb_callback_control>(
    context: *mut c_void,
    instance: *const bdb_instance_ref,
    witness: *const bdb_witness,
) -> u32 {
    let f = unsafe { &mut *context.cast::<F>() };
    u32::from(f(instance, witness))
}

extern "C" fn write_trampoline<F: FnMut(*mut bdb_tx_ref) -> bdb_callback_control>(
    context: *mut c_void,
    transaction: *mut bdb_tx_ref,
) -> u32 {
    let f = unsafe { &mut *context.cast::<F>() };
    u32::from(f(transaction))
}

fn empty_db_admission() -> bdb_db_admission {
    bdb_db_admission {
        tag: bdb_admission_tag::Empty,
        value: crate::db::bdb_db_admission_value {
            accepted: null_mut(),
        },
    }
}

fn empty_write_admission() -> bdb_write_admission {
    bdb_write_admission {
        tag: bdb_admission_tag::Empty,
        value: crate::db::bdb_write_admission_value {
            accepted_generation: 0,
        },
    }
}

fn empty_instance_admission() -> bdb_instance_admission {
    bdb_instance_admission {
        tag: bdb_admission_tag::Empty,
        value: crate::db::bdb_instance_admission_value {
            accepted: null_mut(),
        },
    }
}

fn db_read<F: FnMut(*const bdb_instance_ref, *const bdb_witness) -> bdb_callback_control>(
    db: *const bdb_db,
    mut f: F,
) -> (bdb_status, *mut bdb_error) {
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_db_read(
        db,
        Some(read_trampoline::<F>),
        (&raw mut f).cast::<c_void>(),
        &raw mut error,
    );
    (status, error)
}

fn db_write_admit<F: FnMut(*mut bdb_tx_ref) -> bdb_callback_control>(
    db: *const bdb_db,
    mut f: F,
) -> (bdb_status, bdb_write_admission, *mut bdb_error) {
    let mut error: *mut bdb_error = null_mut();
    let mut admission = empty_write_admission();
    let status = bdb_db_write(
        db,
        Some(write_trampoline::<F>),
        (&raw mut f).cast::<c_void>(),
        &raw mut admission,
        &raw mut error,
    );
    (status, admission, error)
}

fn db_write<F: FnMut(*mut bdb_tx_ref) -> bdb_callback_control>(
    db: *const bdb_db,
    f: F,
) -> (bdb_status, *mut bdb_error) {
    let (status, _admission, error) = db_write_admit(db, f);
    (status, error)
}

fn db_write_from_admit<F: FnMut(*mut bdb_tx_ref) -> bdb_callback_control>(
    db: *const bdb_db,
    witness: *const bdb_witness,
    mut f: F,
) -> (bdb_status, bdb_write_admission, *mut bdb_error) {
    let mut error: *mut bdb_error = null_mut();
    let mut admission = empty_write_admission();
    let status = bdb_db_write_from(
        db,
        witness,
        Some(write_trampoline::<F>),
        (&raw mut f).cast::<c_void>(),
        &raw mut admission,
        &raw mut error,
    );
    (status, admission, error)
}

fn db_write_from<F: FnMut(*mut bdb_tx_ref) -> bdb_callback_control>(
    db: *const bdb_db,
    witness: *const bdb_witness,
    f: F,
) -> (bdb_status, *mut bdb_error) {
    let (status, _admission, error) = db_write_from_admit(db, witness, f);
    (status, error)
}

// ---------------------------------------------------------------------------
// The Service/Outage theory, as C views
// ---------------------------------------------------------------------------

const SERVICE: u32 = 0;
const OUTAGE: u32 = 1;
const OUTAGE_SERVICE: u16 = 0;
const OUTAGE_WINDOW: u16 = 1;

fn vt(kind: bdb_value_type_kind) -> bdb_value_type {
    bdb_value_type {
        kind: u32::from(kind),
        fixed_len: 0,
        element: u32::from(bdb_interval_element::U64),
        has_width: 0,
        width: 0,
    }
}

fn field(name: &str, value_type: bdb_value_type, fresh: bool) -> bdb_field_spec {
    bdb_field_spec {
        name: sv(name),
        value_type,
        newtype: no_sv(),
        fresh: u8::from(fresh),
    }
}

fn blank_side() -> bdb_side {
    bdb_side {
        relation: no_sv(),
        projection: null(),
        projection_count: 0,
        selection: null(),
        selection_count: 0,
    }
}

fn blank_statement(kind: bdb_statement_spec_kind) -> bdb_statement_spec {
    bdb_statement_spec {
        kind: u32::from(kind),
        fd_relation: no_sv(),
        fd_projection: null(),
        fd_projection_count: 0,
        source: blank_side(),
        target: blank_side(),
        bidirectional: 0,
        weight: bdb_weight {
            kind: u32::from(bdb_weight_kind::Unit),
            field: no_sv(),
        },
        window: bdb_capacity_window {
            kind: u32::from(bdb_capacity_window_kind::Exact),
            lo: bdb_bound {
                kind: u32::from(bdb_bound_kind::Lit),
                lit: 0,
                field: no_sv(),
            },
            hi: bdb_bound {
                kind: u32::from(bdb_bound_kind::Lit),
                lit: 0,
                field: no_sv(),
            },
        },
    }
}

/// Builds the §39 Uptime spec as borrowed C views on this frame and runs
/// `f` against it: Service { fresh id: u64, name: str }, Outage
/// { service: u64, window: interval<i64> }, contained(Outage.service ⊆
/// Service.id), key(Outage.service, Outage.window).
fn with_uptime_spec<R>(f: impl FnOnce(&bdb_schema_spec) -> R) -> R {
    let mut interval_i64 = vt(bdb_value_type_kind::Interval);
    interval_i64.element = u32::from(bdb_interval_element::I64);
    let service_fields = [
        field("id", vt(bdb_value_type_kind::U64), true),
        field("name", vt(bdb_value_type_kind::String), false),
    ];
    let outage_fields = [
        field("service", vt(bdb_value_type_kind::U64), false),
        field("window", interval_i64, false),
    ];
    let relations = [
        bdb_relation_spec {
            name: sv("Service"),
            fields: service_fields.as_ptr(),
            field_count: service_fields.len(),
            closed: null(),
        },
        bdb_relation_spec {
            name: sv("Outage"),
            fields: outage_fields.as_ptr(),
            field_count: outage_fields.len(),
            closed: null(),
        },
    ];
    let source_projection = [sv("service")];
    let target_projection = [sv("id")];
    let mut containment = blank_statement(bdb_statement_spec_kind::Containment);
    containment.source = bdb_side {
        relation: sv("Outage"),
        projection: source_projection.as_ptr(),
        projection_count: source_projection.len(),
        selection: null(),
        selection_count: 0,
    };
    containment.target = bdb_side {
        relation: sv("Service"),
        projection: target_projection.as_ptr(),
        projection_count: target_projection.len(),
        selection: null(),
        selection_count: 0,
    };
    let key_projection = [sv("service"), sv("window")];
    let mut key = blank_statement(bdb_statement_spec_kind::Fd);
    key.fd_relation = sv("Outage");
    key.fd_projection = key_projection.as_ptr();
    key.fd_projection_count = key_projection.len();
    let statements = [containment, key];
    let spec = bdb_schema_spec {
        relations: relations.as_ptr(),
        relation_count: relations.len(),
        statements: statements.as_ptr(),
        statement_count: statements.len(),
    };
    f(&spec)
}

fn take_accepted_db(admission: bdb_db_admission) -> *mut bdb_db {
    assert_eq!(admission.tag, bdb_admission_tag::Accepted);
    unsafe { admission.value.accepted }
}

fn create_uptime(path: &std::path::Path) -> *mut bdb_db {
    with_uptime_spec(|spec| {
        let mut admission = empty_db_admission();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_create(
            sv(path.to_str().expect("utf-8 temp path")),
            spec,
            &raw mut admission,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Ok, "create: {}", err_text(error));
        take_accepted_db(admission)
    })
}

fn err_text(error: *mut bdb_error) -> String {
    if error.is_null() {
        return "(no error)".to_string();
    }
    let text = error_message(error);
    destroy_error(error);
    text
}

/// The materialized key-statement ids the keyed-read tests aim at,
/// resolved through the ENGINE's own lowering of the same C views (the
/// bridge marshal under test feeds the engine introspection).
fn key_statement_ids() -> (u16, u16) {
    let spec = with_uptime_spec(|view| schema_spec_in(view).ok().expect("spec marshals"));
    let descriptor = spec.descriptor().expect("descriptor admits");
    let statements = descriptor.materialized_statements();
    let mut outage_key = None;
    let mut service_key = None;
    for (id, statement) in statements.iter().enumerate() {
        if let StatementDescriptor::Functionality { relation, .. } = statement {
            let id = u16::try_from(id).expect("statement id fits u16");
            if *relation == RelationId(OUTAGE) && outage_key.is_none() {
                outage_key = Some(id);
            }
            if *relation == RelationId(SERVICE) && service_key.is_none() {
                service_key = Some(id);
            }
        }
    }
    (
        outage_key.expect("outage key materialized"),
        service_key.expect("service fresh key materialized"),
    )
}

/// Inserts one Service row (fresh-allocated id, returned) and one Outage
/// row over `window` through the write ABI, committing.
fn seed_service_outage(db: *mut bdb_db, name: &str, window: (i64, i64)) -> u64 {
    let mut minted = 0u64;
    let (status, error) = db_write(db, |tx| {
        let mut range = bdb_fresh_range {
            tag: bdb_fresh_range_tag::Empty,
            start: 0,
            end_exclusive: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_reserve(tx, SERVICE, 0, 1, &raw mut range, &raw mut error),
            bdb_status::Ok,
            "reserve: {}",
            err_text(error)
        );
        let id = range.start;
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let service_row = [v_u64(id), v_str(name)];
        assert_eq!(
            bdb_tx_insert(
                tx,
                SERVICE,
                service_row.as_ptr(),
                service_row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(report.changed, 1);
        let outage_row = [v_u64(id), v_interval_i64(window.0, window.1)];
        assert_eq!(
            bdb_tx_insert(
                tx,
                OUTAGE,
                outage_row.as_ptr(),
                outage_row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(report.changed, 1);
        minted = id;
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "commit: {}", err_text(error));
    minted
}

// ---------------------------------------------------------------------------
// The DownAt query (§39), as C views
// ---------------------------------------------------------------------------

fn blank_agg() -> bdb_agg_op {
    bdb_agg_op {
        kind: u32::from(bdb_head_op::Count),
    }
}

fn term(kind: bdb_term_kind, index: u16) -> bdb_term {
    bdb_term {
        kind: u32::from(kind),
        var: index,
        param: index,
        literal: bdb_value::blank(bdb_value_kind::Bool),
    }
}

fn find_var(var: u16) -> bdb_find_term {
    bdb_find_term {
        kind: u32::from(bdb_find_term_kind::Var),
        var,
        op: blank_agg(),
        over: 0,
    }
}

fn head_var() -> bdb_head_term {
    bdb_head_term {
        kind: u32::from(bdb_head_term_kind::Var),
        op: u32::from(bdb_head_op::Count),
    }
}

fn cmp_op(kind: bdb_cmp_op_kind) -> bdb_cmp_op {
    bdb_cmp_op {
        kind: u32::from(kind),
        mask: 0,
    }
}

/// `DownAt(t) = { service | Outage(service, window), t in window }` —
/// one Edb atom over Outage, a `PointIn` condition (interval lhs, point
/// rhs), one projected find.
fn with_down_at_query<R>(f: impl FnOnce(&bdb_query) -> R) -> R {
    let bindings = [
        bdb_binding {
            field: OUTAGE_SERVICE,
            term: term(bdb_term_kind::Var, 0),
        },
        bdb_binding {
            field: OUTAGE_WINDOW,
            term: term(bdb_term_kind::Var, 1),
        },
    ];
    let atoms = [bdb_atom {
        source_kind: u32::from(bdb_atom_source_kind::Edb),
        relation: OUTAGE,
        interior: 0,
        bindings: bindings.as_ptr(),
        binding_count: bindings.len(),
    }];
    let conditions = [bdb_condition {
        kind: u32::from(bdb_condition_kind::Leaf),
        cmp: bdb_comparison {
            op: cmp_op(bdb_cmp_op_kind::PointIn),
            lhs: term(bdb_term_kind::Var, 1),
            rhs: term(bdb_term_kind::Param, 0),
        },
        children: null(),
        child_count: 0,
    }];
    let finds = [find_var(0)];
    let rules = [bdb_rule {
        finds: finds.as_ptr(),
        find_count: finds.len(),
        atoms: atoms.as_ptr(),
        atom_count: atoms.len(),
        negated: null(),
        negated_count: 0,
        conditions: conditions.as_ptr(),
        condition_count: conditions.len(),
    }];
    let head = [head_var()];
    let query = bdb_query {
        kind: u32::from(bdb_query_kind::Cq),
        payload: bdb_query_payload {
            cq: bdb_cq {
                interiors: null(),
                interior_count: 0,
                head: head.as_ptr(),
                head_count: head.len(),
                rules: rules.as_ptr(),
                rule_count: rules.len(),
            },
        },
    };
    f(&query)
}

/// `NamesOf(ids) = { name | Service(id, name), id in ids }` — the set
/// param lane plus a string find.
fn with_names_of_query<R>(f: impl FnOnce(&bdb_query) -> R) -> R {
    let bindings = [
        bdb_binding {
            field: 0,
            term: term(bdb_term_kind::ParamSet, 0),
        },
        bdb_binding {
            field: 1,
            term: term(bdb_term_kind::Var, 0),
        },
    ];
    let atoms = [bdb_atom {
        source_kind: u32::from(bdb_atom_source_kind::Edb),
        relation: SERVICE,
        interior: 0,
        bindings: bindings.as_ptr(),
        binding_count: bindings.len(),
    }];
    let finds = [find_var(0)];
    let rules = [bdb_rule {
        finds: finds.as_ptr(),
        find_count: finds.len(),
        atoms: atoms.as_ptr(),
        atom_count: atoms.len(),
        negated: null(),
        negated_count: 0,
        conditions: null(),
        condition_count: 0,
    }];
    let head = [head_var()];
    let query = bdb_query {
        kind: u32::from(bdb_query_kind::Cq),
        payload: bdb_query_payload {
            cq: bdb_cq {
                interiors: null(),
                interior_count: 0,
                head: head.as_ptr(),
                head_count: head.len(),
                rules: rules.as_ptr(),
                rule_count: rules.len(),
            },
        },
    };
    f(&query)
}

fn prepare(db: *mut bdb_db, query: &bdb_query) -> *mut bdb_prepared {
    let mut prepared: *mut bdb_prepared = null_mut();
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_db_prepare(db, query, &raw mut prepared, &raw mut error);
    assert_eq!(status, bdb_status::Ok, "prepare: {}", err_text(error));
    prepared
}

// ---------------------------------------------------------------------------
// §35 cases
// ---------------------------------------------------------------------------

#[test]
fn create_refuses_existing_destination() {
    let dir = temp_store("exists");
    std::fs::create_dir_all(&dir).expect("pre-create empty destination");
    with_uptime_spec(|spec| {
        let mut admission = empty_db_admission();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_create(
            sv(dir.to_str().expect("utf-8 temp path")),
            spec,
            &raw mut admission,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::DestinationExists);
        destroy_error(error);
        assert_eq!(admission.tag, bdb_admission_tag::Empty);
    });
}

#[test]
fn create_open_ephemeral_close() {
    let dir = temp_store("lifecycle");
    let db = create_uptime(&dir);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);

    // Reopen under the same spec: the stored fingerprint verifies.
    let db = with_uptime_spec(|spec| {
        let mut db: *mut bdb_db = null_mut();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_open(
            sv(dir.to_str().expect("utf-8 path")),
            spec,
            &raw mut db,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Ok, "open: {}", err_text(error));
        db
    });
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);

    // The ephemeral kind is its own constructor and store.
    let ephemeral_dir = temp_store("lifecycle-ephemeral");
    let db = with_uptime_spec(|spec| {
        let mut admission = empty_db_admission();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_ephemeral(
            sv(ephemeral_dir.to_str().expect("utf-8 path")),
            spec,
            &raw mut admission,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Ok, "ephemeral: {}", err_text(error));
        take_accepted_db(admission)
    });
    seed_service_outage(db, "scratch", (0, 5));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);

    // Destroying a null handle is misuse, not a crash.
    assert_eq!(bdb_db_destroy(null_mut()), bdb_status::Misuse);
}

#[test]
fn schema_spec_crossing_admits_and_fingerprints() {
    let dir = temp_store("fingerprint");
    let db = create_uptime(&dir);
    let mut fingerprint = bdb_fingerprint { hex: [0; 64] };
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_db_fingerprint(db, &raw mut fingerprint, &raw mut error);
    assert_eq!(status, bdb_status::Ok, "fingerprint: {}", err_text(error));
    assert!(
        fingerprint
            .hex
            .iter()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "64 lowercase hex chars"
    );
    assert!(
        fingerprint.hex.iter().any(|byte| *byte != b'0'),
        "a real digest, not zeroes"
    );
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn read_callback_sees_committed_state() {
    let dir = temp_store("read");
    let db = create_uptime(&dir);
    let id = seed_service_outage(db, "search", (10, 20));
    let (status, error) = db_read(db, |instance, _witness| {
        let row = [v_u64(id), v_str("search")];
        let mut contains: u8 = 0;
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_contains(
                instance,
                SERVICE,
                row.as_ptr(),
                row.len(),
                &raw mut contains,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(contains, 1);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn write_abort_commits_nothing() {
    let dir = temp_store("abort");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        let row = [v_u64(7), v_str("ghost")];
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_insert(
                tx,
                SERVICE,
                row.as_ptr(),
                row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(report.changed, 1);
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert!(error.is_null(), "abort is not an error");
    let (status, error) = db_read(db, |instance, _witness| {
        let row = [v_u64(7), v_str("ghost")];
        let mut contains: u8 = 1;
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_contains(
                instance,
                SERVICE,
                row.as_ptr(),
                row.len(),
                &raw mut contains,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(contains, 0, "the aborted delta never touched LMDB");
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one linear ABI walk over both keyed-read lanes and the delete \
              path — clearer kept together than split across stores"
)]
fn insert_delete_contains_get_dyn() {
    let dir = temp_store("dyn");
    let db = create_uptime(&dir);
    let (outage_key, service_key) = key_statement_ids();
    let id = seed_service_outage(db, "api", (100, 200));

    // Keyed reads on both the declared pointwise key and the
    // fresh-implied key, snapshot side.
    let (status, error) = db_read(db, |instance, _witness| {
        let mut error: *mut bdb_error = null_mut();
        let keys = [v_u64(id), v_interval_i64(100, 200)];
        let mut row: *mut bdb_row_set = null_mut();
        assert_eq!(
            bdb_instance_get(
                instance,
                OUTAGE,
                outage_key,
                keys.as_ptr(),
                keys.len(),
                &raw mut row,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert!(!row.is_null(), "keyed hit");
        assert_eq!(bdb_row_set_len(row), 1);
        assert_eq!(bdb_row_set_arity(row), 2);
        let mut cell = bdb_value::blank(bdb_value_kind::Bool);
        assert_eq!(bdb_row_set_get(row, 0, 1, &raw mut cell), bdb_status::Ok);
        assert_eq!(cell.kind, u32::from(bdb_value_kind::IntervalI64));
        assert_eq!((cell.interval_i64_start, cell.interval_i64_end), (100, 200));
        assert_eq!(
            bdb_row_set_get(row, 1, 0, &raw mut cell),
            bdb_status::Misuse,
            "row-set access is bounds-checked"
        );
        assert_eq!(bdb_row_set_destroy(row), bdb_status::Ok);

        let service_keys = [v_u64(id)];
        let mut row: *mut bdb_row_set = null_mut();
        assert_eq!(
            bdb_instance_get(
                instance,
                SERVICE,
                service_key,
                service_keys.as_ptr(),
                service_keys.len(),
                &raw mut row,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert!(!row.is_null());
        let mut name = bdb_value::blank(bdb_value_kind::Bool);
        assert_eq!(bdb_row_set_get(row, 0, 1, &raw mut name), bdb_status::Ok);
        assert_eq!(name.kind, u32::from(bdb_value_kind::String));
        let text =
            unsafe { std::slice::from_raw_parts(name.string_value.data, name.string_value.len) };
        assert_eq!(text, b"api");
        assert_eq!(bdb_row_set_destroy(row), bdb_status::Ok);

        // A missing key writes null, not an error.
        let misses = [v_u64(id + 999)];
        let mut row: *mut bdb_row_set = null_mut();
        assert_eq!(
            bdb_instance_get(
                instance,
                SERVICE,
                service_key,
                misses.as_ptr(),
                misses.len(),
                &raw mut row,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert!(row.is_null(), "a miss is null");
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));

    // Delete/insert + final-state point reads, tx side.
    let (status, error) = db_write(db, |tx| {
        let mut error: *mut bdb_error = null_mut();
        let outage_row = [v_u64(id), v_interval_i64(100, 200)];
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        assert_eq!(
            bdb_tx_delete(
                tx,
                OUTAGE,
                outage_row.as_ptr(),
                outage_row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(report.changed, 1, "the fact existed");
        assert_eq!(
            bdb_tx_delete(
                tx,
                OUTAGE,
                outage_row.as_ptr(),
                outage_row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(report.changed, 0, "double delete is a no-op");
        let mut contains: u8 = 1;
        assert_eq!(
            bdb_tx_contains(
                tx,
                OUTAGE,
                outage_row.as_ptr(),
                outage_row.len(),
                &raw mut contains,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(contains, 0, "the final-state view sees the pending delete");
        let keys = [v_u64(id), v_interval_i64(100, 200)];
        let (outage_key, _) = key_statement_ids();
        let mut row: *mut bdb_row_set = null_mut();
        assert_eq!(
            bdb_tx_get(
                tx,
                OUTAGE,
                outage_key,
                keys.as_ptr(),
                keys.len(),
                &raw mut row,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert!(row.is_null(), "the deleted fact resolves nowhere");
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "write: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn scan_exports_rows() {
    let dir = temp_store("scan");
    let db = create_uptime(&dir);
    seed_service_outage(db, "alpha", (0, 10));
    seed_service_outage(db, "beta", (10, 20));
    let (status, error) = db_read(db, |instance, _witness| {
        let mut rows: *mut bdb_row_set = null_mut();
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_scan(instance, SERVICE, &raw mut rows, &raw mut error),
            bdb_status::Ok
        );
        assert_eq!(bdb_row_set_len(rows), 2);
        let mut names = Vec::new();
        for row in 0..2 {
            let mut cell = bdb_value::blank(bdb_value_kind::Bool);
            assert_eq!(bdb_row_set_get(rows, row, 1, &raw mut cell), bdb_status::Ok);
            assert_eq!(cell.kind, u32::from(bdb_value_kind::String));
            let text = unsafe {
                std::slice::from_raw_parts(cell.string_value.data, cell.string_value.len)
            };
            names.push(String::from_utf8(text.to_vec()).expect("utf-8 name"));
        }
        names.sort();
        assert_eq!(names, ["alpha", "beta"]);
        assert_eq!(bdb_row_set_destroy(rows), bdb_status::Ok);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn write_from_ok_and_moved() {
    let dir = temp_store("write-from");
    let db = create_uptime(&dir);
    let id = seed_service_outage(db, "gamma", (0, 10));

    // The sanctioned nesting: write_from with the read callback's own
    // still-live witness.
    let (status, error) = db_read(db, |_instance, witness| {
        let (status, error) = db_write_from(db, witness, |tx| {
            let row = [v_u64(id), v_interval_i64(50, 60)];
            let mut report = bdb_mutation_report {
                submitted: 0,
                changed: 0,
            };
            let mut error: *mut bdb_error = null_mut();
            assert_eq!(
                bdb_tx_insert(
                    tx,
                    OUTAGE,
                    row.as_ptr(),
                    row.len(),
                    1,
                    &raw mut report,
                    &raw mut error,
                ),
                bdb_status::Ok
            );
            bdb_callback_control::Ok
        });
        assert_eq!(status, bdb_status::Ok, "write_from: {}", err_text(error));
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));

    // A state-changing commit after the witness: ConditionalWrite::Moved,
    // reported as a write-admission arm under OK, not an error kind.
    let (status, error) = db_read(db, |_instance, witness| {
        let (status, error) = db_write(db, |tx| {
            let row = [v_u64(id), v_interval_i64(70, 80)];
            let mut report = bdb_mutation_report {
                submitted: 0,
                changed: 0,
            };
            let mut error: *mut bdb_error = null_mut();
            assert_eq!(
                bdb_tx_insert(
                    tx,
                    OUTAGE,
                    row.as_ptr(),
                    row.len(),
                    1,
                    &raw mut report,
                    &raw mut error,
                ),
                bdb_status::Ok
            );
            bdb_callback_control::Ok
        });
        assert_eq!(
            status,
            bdb_status::Ok,
            "interleaved write: {}",
            err_text(error)
        );

        let mut entered = false;
        let (status, admission, error) = db_write_from_admit(db, witness, |_tx| {
            entered = true;
            bdb_callback_control::Ok
        });
        assert_eq!(status, bdb_status::Ok, "moved is not an error: {}", err_text(error));
        assert!(error.is_null());
        assert!(!entered, "the closure never runs on a moved generation");
        assert_eq!(admission.tag, bdb_admission_tag::Moved);
        let moved = unsafe { admission.value.moved };
        assert!(
            moved.current > moved.witnessed,
            "the clock moved past the witness"
        );
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn nested_write_is_refused_typed() {
    let dir = temp_store("nested");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |_tx| {
        let (status, error) = db_write(db, |_inner| bdb_callback_control::Ok);
        assert_eq!(status, bdb_status::Error);
        assert_eq!(
            error_kind(error),
            bdb_error_kind::BusyHandle,
            "the bridge refuses BEFORE the engine assertion"
        );
        assert!(error_message(error).contains("re-entrant"));
        destroy_error(error);
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert!(error.is_null());
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn stale_witness_is_misuse() {
    let dir = temp_store("stale");
    let db = create_uptime(&dir);
    let mut stashed: *const bdb_witness = null();
    let (status, error) = db_read(db, |_instance, witness| {
        stashed = witness;
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    // The witness lives in a retired heap slot, so this replay is a
    // real MISUSE (alive=false), not a use-after-free of a stack frame.
    let (status, error) = db_write_from(db, stashed, |_tx| bdb_callback_control::Ok);
    assert_eq!(status, bdb_status::Misuse);
    assert!(error.is_null(), "misuse allocates no error");
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn stale_instance_ref_is_misuse() {
    let dir = temp_store("stale-instance");
    let db = create_uptime(&dir);
    let mut stashed: *const bdb_instance_ref = null();
    let (status, error) = db_read(db, |instance, _witness| {
        stashed = instance;
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    let mut contains: u8 = 1;
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_instance_contains(
        stashed,
        SERVICE,
        null(),
        0,
        &raw mut contains,
        &raw mut error,
    );
    assert_eq!(status, bdb_status::Misuse);
    assert!(error.is_null(), "misuse allocates no error");
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn stashed_instance_ref_after_db_destroy_is_misuse() {
    let dir = temp_store("stashed-after-destroy");
    let db = create_uptime(&dir);
    let mut stashed: *const bdb_instance_ref = null();
    let (status, error) = db_read(db, |instance, _witness| {
        stashed = instance;
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
    let mut contains: u8 = 1;
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_instance_contains(
        stashed,
        SERVICE,
        null(),
        0,
        &raw mut contains,
        &raw mut error,
    );
    assert_eq!(status, bdb_status::Misuse);
    assert!(error.is_null(), "misuse allocates no error");
}

#[test]
fn foreign_prepared_is_refused_at_the_bridge() {
    let dir_a = temp_store("foreign-prepared-a");
    let dir_b = temp_store("foreign-prepared-b");
    let db_a = create_uptime(&dir_a);
    let db_b = create_uptime(&dir_b);
    let prepared = with_down_at_query(|query| prepare(db_a, query));
    let answers = bdb_answers_new();
    let (status, error) = db_read(db_b, |instance, _witness| {
        let mut error: *mut bdb_error = null_mut();
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Scalar),
            scalar: v_i64(15),
            set: null(),
            set_len: 0,
        }];
        let status = bdb_instance_execute(
            instance,
            prepared,
            params.as_ptr(),
            params.len(),
            answers,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error, "foreign execute: {}", err_text(error));
        assert_eq!(error_kind(error), bdb_error_kind::ForeignPrepared);
        destroy_error(error);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_answers_destroy(answers), bdb_status::Ok);
    assert_eq!(bdb_prepared_destroy(prepared), bdb_status::Ok);
    assert_eq!(bdb_db_destroy(db_a), bdb_status::Ok);
    assert_eq!(bdb_db_destroy(db_b), bdb_status::Ok);
}

#[test]
fn collection_insert_and_shape_failure_persists_nothing() {
    let dir = temp_store("insert-collection");
    let db = create_uptime(&dir);

    let names: Vec<String> = (0..100).map(|i| format!("svc-{i}")).collect();
    let mut cells: Vec<bdb_value> = Vec::with_capacity(200);
    for (i, name) in names.iter().enumerate() {
        cells.push(v_u64(i as u64));
        cells.push(v_str(name));
    }
    let (status, error) = db_write(db, |tx| {
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_insert(
                tx,
                SERVICE,
                cells.as_ptr(),
                2,
                100,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok,
            "insert: {}",
            err_text(error)
        );
        assert_eq!(report.submitted, 100);
        assert_eq!(report.changed, 100);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "commit: {}", err_text(error));

    let (status, error) = db_write(db, |tx| {
        let row = [v_u64(9_999_999)];
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            row.as_ptr(),
            row.len(),
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::FactShape);
        destroy_error(error);
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert!(error.is_null());
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn prepare_execute_scalar_param_and_decode() {
    let dir = temp_store("execute");
    let db = create_uptime(&dir);
    let id = seed_service_outage(db, "down-at", (10, 20));
    seed_service_outage(db, "up-at", (100, 200));

    let prepared = with_down_at_query(|query| prepare(db, query));
    let answers = bdb_answers_new();

    let (status, error) = db_read(db, |instance, _witness| {
        let mut error: *mut bdb_error = null_mut();
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Scalar),
            scalar: v_i64(15),
            set: null(),
            set_len: 0,
        }];
        assert_eq!(
            bdb_instance_execute(
                instance,
                prepared,
                params.as_ptr(),
                params.len(),
                answers,
                &raw mut error,
            ),
            bdb_status::Ok,
            "execute: {}",
            err_text(error)
        );
        assert_eq!(bdb_answers_len(answers), 1);
        assert_eq!(bdb_answers_arity(answers), 1);
        let mut cell = bdb_value::blank(bdb_value_kind::Bool);
        assert_eq!(
            bdb_answers_get(answers, 0, 0, &raw mut cell),
            bdb_status::Ok
        );
        assert_eq!(cell.kind, u32::from(bdb_value_kind::U64));
        assert_eq!(cell.u64_value, id);
        assert_eq!(
            bdb_answers_get(answers, 1, 0, &raw mut cell),
            bdb_status::Misuse,
            "answers access is bounds-checked bridge-side"
        );

        // Re-execute into the SAME carrier (capacity reuse, §23): a
        // parameter matching nothing leaves it validly empty.
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Scalar),
            scalar: v_i64(9_999),
            set: null(),
            set_len: 0,
        }];
        assert_eq!(
            bdb_instance_execute(
                instance,
                prepared,
                params.as_ptr(),
                params.len(),
                answers,
                &raw mut error,
            ),
            bdb_status::Ok,
            "re-execute: {}",
            err_text(error)
        );
        assert_eq!(bdb_answers_len(answers), 0, "cleared, not appended");

        // A mistyped param is the engine's typed bind refusal.
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Scalar),
            scalar: v_bool(true),
            set: null(),
            set_len: 0,
        }];
        let status = bdb_instance_execute(
            instance,
            prepared,
            params.as_ptr(),
            params.len(),
            answers,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::Param);
        destroy_error(error);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));

    assert_eq!(bdb_answers_clear(answers), bdb_status::Ok);
    assert_eq!(bdb_answers_destroy(answers), bdb_status::Ok);
    assert_eq!(bdb_prepared_destroy(prepared), bdb_status::Ok);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn execute_set_param_decodes_strings() {
    let dir = temp_store("set-param");
    let db = create_uptime(&dir);
    let alpha = seed_service_outage(db, "alpha", (0, 10));
    let _beta = seed_service_outage(db, "beta", (10, 20));
    let gamma = seed_service_outage(db, "gamma", (20, 30));

    let prepared = with_names_of_query(|query| prepare(db, query));
    let answers = bdb_answers_new();

    let (status, error) = db_read(db, |instance, _witness| {
        let mut error: *mut bdb_error = null_mut();
        let set = [v_u64(alpha), v_u64(gamma)];
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Set),
            scalar: bdb_value::blank(bdb_value_kind::Bool),
            set: set.as_ptr(),
            set_len: set.len(),
        }];
        assert_eq!(
            bdb_instance_execute(
                instance,
                prepared,
                params.as_ptr(),
                params.len(),
                answers,
                &raw mut error,
            ),
            bdb_status::Ok,
            "execute: {}",
            err_text(error)
        );
        assert_eq!(bdb_answers_len(answers), 2);
        let mut names = Vec::new();
        for row in 0..2 {
            let mut cell = bdb_value::blank(bdb_value_kind::Bool);
            assert_eq!(
                bdb_answers_get(answers, row, 0, &raw mut cell),
                bdb_status::Ok
            );
            assert_eq!(cell.kind, u32::from(bdb_value_kind::String));
            let text = unsafe {
                std::slice::from_raw_parts(cell.string_value.data, cell.string_value.len)
            };
            names.push(String::from_utf8(text.to_vec()).expect("utf-8 answer"));
        }
        names.sort();
        assert_eq!(names, ["alpha", "gamma"]);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));

    assert_eq!(bdb_answers_destroy(answers), bdb_status::Ok);
    assert_eq!(bdb_prepared_destroy(prepared), bdb_status::Ok);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn pointwise_key_violation_is_rejected_and_renderable() {
    let dir = temp_store("rejection");
    let db = create_uptime(&dir);
    let id = seed_service_outage(db, "flaky", (10, 20));
    let (status, admission, error) = db_write_admit(db, |tx| {
        // Overlaps [10, 20) for the same service: the pointwise key
        // (service, window) convicts at commit.
        let row = [v_u64(id), v_interval_i64(15, 25)];
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_insert(
                tx,
                OUTAGE,
                row.as_ptr(),
                row.len(),
                1,
                &raw mut report,
                &raw mut error,
            ),
            bdb_status::Ok,
            "nothing is judged until commit"
        );
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "rejection is not an error: {}", err_text(error));
    assert!(error.is_null());
    assert_eq!(admission.tag, bdb_admission_tag::Rejected);
    let violations = unsafe { admission.value.rejected };
    assert!(bdb_violations_len(violations) >= 1);
    let mut violation = bdb_violation {
        statement: 0,
        kind: bdb_statement_kind::Containment,
        spelling: no_sv(),
        payload: crate::error::bdb_violation_payload { functionality: 0 },
    };
    assert_eq!(
        bdb_violations_get(violations, 0, &raw mut violation),
        bdb_status::Ok
    );
    assert_eq!(violation.kind, bdb_statement_kind::Functionality);
    let spelling =
        unsafe { std::slice::from_raw_parts(violation.spelling.data, violation.spelling.len) };
    let spelling = std::str::from_utf8(spelling).expect("utf-8 spelling");
    assert!(
        spelling.contains("Outage"),
        "the canonical spelling cites the relation: {spelling}"
    );
    assert_eq!(
        bdb_violations_get(violations, usize::MAX, &raw mut violation),
        bdb_status::Misuse,
        "violation access is bounds-checked"
    );
    assert_eq!(bdb_violations_destroy(violations), bdb_status::Ok);

    // The rejected delta committed nothing.
    let (status, error) = db_read(db, |instance, _witness| {
        let row = [v_u64(id), v_interval_i64(15, 25)];
        let mut contains: u8 = 1;
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_contains(
                instance,
                OUTAGE,
                row.as_ptr(),
                row.len(),
                &raw mut contains,
                &raw mut error,
            ),
            bdb_status::Ok
        );
        assert_eq!(contains, 0);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn open_failure_is_typed_and_destroyable() {
    let missing =
        std::env::temp_dir().join(format!("bumbledb-c-{}-never-created", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing);
    let (status, error) = with_uptime_spec(|spec| {
        let mut db: *mut bdb_db = null_mut();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_open(
            sv(missing.to_str().expect("utf-8 path")),
            spec,
            &raw mut db,
            &raw mut error,
        );
        assert!(db.is_null());
        (status, error)
    });
    assert_eq!(status, bdb_status::Error);
    assert!(!error_message(error).is_empty());
    destroy_error(error);
}

#[test]
fn marshal_refusals_are_typed_fact_shape() {
    let dir = temp_store("marshal");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        // An empty interval is unrepresentable in the engine: the bridge
        // refuses it at marshal, typed.
        let row = [v_u64(1), v_interval_i64(20, 10)];
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_insert(
            tx,
            OUTAGE,
            row.as_ptr(),
            row.len(),
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::Marshal);
        destroy_error(error);
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert!(error.is_null());
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn panic_maps_to_bdb_error_panic() {
    let mut error: *mut bdb_error = null_mut();
    let status = test_only_trigger_panic(&raw mut error);
    assert_eq!(status, bdb_status::Error);
    assert_eq!(error_kind(error), bdb_error_kind::Panic);
    assert!(error_message(error).contains("panic"));
    destroy_error(error);
}

extern "C" fn invalid_callback_control(
    _context: *mut c_void,
    _instance: *const bdb_instance_ref,
    _witness: *const bdb_witness,
) -> u32 {
    99
}

#[test]
fn destroy_during_read_callback_is_misuse() {
    let dir = temp_store("destroy-reentrant");
    let db = create_uptime(&dir);
    let (status, error) = db_read(db, |_, _| {
        assert_eq!(bdb_db_destroy(db), bdb_status::Misuse);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn nested_reads_are_concurrent() {
    let dir = temp_store("nested-read");
    let db = create_uptime(&dir);
    let (status, error) = db_read(db, |_, _| {
        let (inner_status, inner_error) = db_read(db, |_, _| bdb_callback_control::Ok);
        assert_eq!(
            inner_status,
            bdb_status::Ok,
            "concurrent MVCC reads: {}",
            err_text(inner_error)
        );
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn null_out_db_is_misuse_and_does_not_lock_the_path() {
    let dir = temp_store("null-out");
    let path = sv(dir.to_str().expect("utf-8 temp path"));
    with_uptime_spec(|spec| {
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_create(path, spec, null_mut(), &raw mut error);
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        let mut admission = empty_db_admission();
        let status = bdb_db_create(path, spec, &raw mut admission, &raw mut error);
        assert_eq!(
            status,
            bdb_status::Ok,
            "create after null out: {}",
            err_text(error)
        );
        assert_eq!(bdb_db_destroy(take_accepted_db(admission)), bdb_status::Ok);
    });
}

#[test]
fn insert_null_out_report_does_not_commit() {
    let dir = temp_store("insert-null");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        let row = [v_u64(1), v_str("solo")];
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            row.as_ptr(),
            row.len(),
            1,
            null_mut(),
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        // Ok after misuse: the call must not have applied. Abort would
        // hide a live delta behind a dropped commit.
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "write: {}", err_text(error));
    let (status, error) = db_read(db, |instance, _witness| {
        let mut rows: *mut bdb_row_set = null_mut();
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_scan(instance, SERVICE, &raw mut rows, &raw mut error),
            bdb_status::Ok,
            "scan: {}",
            err_text(error)
        );
        assert_eq!(bdb_row_set_len(rows), 0, "null out_report must not insert");
        assert_eq!(bdb_row_set_destroy(rows), bdb_status::Ok);
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn reserve_null_out_range_does_not_mint() {
    let dir = temp_store("reserve-null");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_reserve(tx, SERVICE, 0, 1, null_mut(), &raw mut error);
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "write: {}", err_text(error));
    let (status, error) = db_write(db, |tx| {
        let mut range = bdb_fresh_range {
            tag: bdb_fresh_range_tag::Empty,
            start: 0,
            end_exclusive: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_reserve(tx, SERVICE, 0, 1, &raw mut range, &raw mut error),
            bdb_status::Ok,
            "reserve: {}",
            err_text(error)
        );
        assert_eq!(
            (range.start, range.end_exclusive),
            (0, 1),
            "null out_range must not advance the sequence"
        );
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted, "write: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn empty_insert_and_reserve_are_mutations() {
    let dir = temp_store("empty-mut");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        let mut report = bdb_mutation_report {
            submitted: 99,
            changed: 99,
        };
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_tx_insert(tx, SERVICE, null(), 2, 0, &raw mut report, &raw mut error,),
            bdb_status::Ok,
            "empty insert: {}",
            err_text(error)
        );
        assert_eq!(report.submitted, 0);
        assert_eq!(report.changed, 0);

        assert_eq!(
            bdb_tx_delete(tx, SERVICE, null(), 2, 0, &raw mut report, &raw mut error,),
            bdb_status::Ok,
            "empty delete: {}",
            err_text(error)
        );
        assert_eq!(report.submitted, 0);
        assert_eq!(report.changed, 0);

        let mut range = bdb_fresh_range {
            tag: bdb_fresh_range_tag::Empty,
            start: 7,
            end_exclusive: 7,
        };
        assert_eq!(
            bdb_tx_reserve(tx, SERVICE, 0, 0, &raw mut range, &raw mut error),
            bdb_status::Ok,
            "empty reserve: {}",
            err_text(error)
        );
        assert_eq!(range.tag, bdb_fresh_range_tag::Empty, "empty is the tag");
        assert_eq!(
            bdb_tx_reserve(tx, SERVICE, 0, 1, &raw mut range, &raw mut error),
            bdb_status::Ok,
            "reserve(1) after empty: {}",
            err_text(error)
        );
        assert_eq!(range.tag, bdb_fresh_range_tag::NonEmpty);
        assert_eq!(
            (range.start, range.end_exclusive),
            (0, 1),
            "empty did not mint; the first minted id is still 0"
        );
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted, "write: {}", err_text(error));
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn zero_width_nonzero_rows_is_shape_not_panic() {
    let dir = temp_store("zero-width");
    let db = create_uptime(&dir);
    let (status, error) = db_write(db, |tx| {
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_insert(tx, SERVICE, null(), 0, 1, &raw mut report, &raw mut error);
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::Marshal);
        destroy_error(error);
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert!(error.is_null());
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn invalid_enum_tag_and_bool_are_misuse() {
    let dir = temp_store("bad-tag");
    let db = create_uptime(&dir);
    let (status, _error) = db_write(db, |tx| {
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        let mut row = [v_u64(1), v_str("x")];
        row[0].kind = 99;
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            row.as_ptr(),
            row.len(),
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());

        let mut flag = v_bool(true);
        flag.bool_value = 2;
        let row = [v_u64(1), flag];
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            row.as_ptr(),
            row.len(),
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn invalid_callback_control_is_misuse() {
    let dir = temp_store("bad-control");
    let db = create_uptime(&dir);
    let mut error: *mut bdb_error = null_mut();
    let status = bdb_db_read(
        db,
        Some(invalid_callback_control),
        null_mut(),
        &raw mut error,
    );
    assert_eq!(status, bdb_status::Misuse);
    assert!(error.is_null());
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn slice_overflow_and_unaligned_are_misuse() {
    let dir = temp_store("slice");
    let db = create_uptime(&dir);
    let (status, _error) = db_write(db, |tx| {
        let dummy = v_u64(1);
        let mut report = bdb_mutation_report {
            submitted: 0,
            changed: 0,
        };
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            &raw const dummy,
            usize::MAX,
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());

        let bytes = [0u8; 64];
        #[expect(
            clippy::cast_ptr_alignment,
            reason = "the test constructs an unaligned pointer so slice_in can refuse it"
        )]
        let unaligned = bytes.as_ptr().wrapping_add(1).cast::<bdb_value>();
        let status = bdb_tx_insert(
            tx,
            SERVICE,
            unaligned,
            1,
            1,
            &raw mut report,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        bdb_callback_control::Abort
    });
    assert_eq!(status, bdb_status::Aborted);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn prepared_exclusive_execute_and_destroy() {
    use std::sync::atomic::Ordering;

    let dir = temp_store("prepared-excl");
    let db = create_uptime(&dir);
    let prepared = with_down_at_query(|query| prepare(db, query));
    let answers = bdb_answers_new();
    let (status, error) = db_read(db, |instance, _witness| {
        unsafe {
            (*prepared).in_execute.store(true, Ordering::SeqCst);
        }
        let mut error: *mut bdb_error = null_mut();
        let params = [bdb_param {
            kind: u32::from(bdb_param_kind::Scalar),
            scalar: v_i64(15),
            set: null(),
            set_len: 0,
        }];
        let status = bdb_instance_execute(
            instance,
            prepared,
            params.as_ptr(),
            params.len(),
            answers,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert_eq!(error_kind(error), bdb_error_kind::BusyHandle);
        destroy_error(error);
        assert_eq!(bdb_prepared_destroy(prepared), bdb_status::Misuse);
        unsafe {
            (*prepared).in_execute.store(false, Ordering::SeqCst);
        }
        bdb_callback_control::Ok
    });
    assert_eq!(status, bdb_status::Ok, "read: {}", err_text(error));
    assert_eq!(bdb_answers_destroy(answers), bdb_status::Ok);
    assert_eq!(bdb_prepared_destroy(prepared), bdb_status::Ok);
    assert_eq!(bdb_db_destroy(db), bdb_status::Ok);
}

#[test]
fn store_error_overwrite_frees_the_previous() {
    let missing_a = std::env::temp_dir().join(format!("bumbledb-c-{}-never-a", std::process::id()));
    let missing_b = std::env::temp_dir().join(format!("bumbledb-c-{}-never-b", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing_a);
    let _ = std::fs::remove_dir_all(&missing_b);
    with_uptime_spec(|spec| {
        let mut db: *mut bdb_db = null_mut();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_open(
            sv(missing_a.to_str().expect("utf-8")),
            spec,
            &raw mut db,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert!(!error.is_null());
        let status = bdb_db_open(
            sv(missing_b.to_str().expect("utf-8")),
            spec,
            &raw mut db,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Error);
        assert!(!error.is_null());
        destroy_error(error);
    });
}

#[test]
fn version_symbols() {
    let version = unsafe { std::ffi::CStr::from_ptr(bdb_version()) };
    let version = version.to_str().expect("utf-8 version");
    assert!(
        version.starts_with("bumbledb-c "),
        "version string: {version}"
    );
    assert!(
        version.contains(env!("CARGO_PKG_VERSION")),
        "version string: {version}"
    );
    assert_eq!(bdb_abi_version(), 3);
}

#[test]
fn consumed_builder_cannot_admit_twice() {
    with_uptime_spec(|spec| {
        let mut builder: *mut bdb_instance_builder = null_mut();
        let mut error: *mut bdb_error = null_mut();
        assert_eq!(
            bdb_instance_builder_new(spec, &raw mut builder, &raw mut error),
            bdb_status::Ok,
            "builder new: {}",
            err_text(error)
        );
        let mut admission = empty_instance_admission();
        let status = bdb_instance_builder_admit(&raw mut builder, &raw mut admission, &raw mut error);
        assert_eq!(status, bdb_status::Ok, "admit: {}", err_text(error));
        assert!(builder.is_null(), "admit nulls the caller's pointer");
        assert_ne!(admission.tag, bdb_admission_tag::Empty);
        match admission.tag {
            bdb_admission_tag::Accepted => {
                let instance = unsafe { admission.value.accepted };
                assert_eq!(bdb_owned_instance_destroy(instance), bdb_status::Ok);
            }
            bdb_admission_tag::Rejected => {
                let violations = unsafe { admission.value.rejected };
                assert_eq!(bdb_violations_destroy(violations), bdb_status::Ok);
            }
            other => panic!("unexpected admission tag {other:?}"),
        }

        let mut second = empty_instance_admission();
        let status = bdb_instance_builder_admit(&raw mut builder, &raw mut second, &raw mut error);
        assert_eq!(status, bdb_status::Misuse);
        assert!(error.is_null());
        assert_eq!(second.tag, bdb_admission_tag::Empty);
    });
}

#[test]
fn create_ok_never_returns_empty_admission() {
    let dir = temp_store("empty-tag");
    with_uptime_spec(|spec| {
        let mut admission = empty_db_admission();
        let mut error: *mut bdb_error = null_mut();
        let status = bdb_db_create(
            sv(dir.to_str().expect("utf-8 temp path")),
            spec,
            &raw mut admission,
            &raw mut error,
        );
        assert_eq!(status, bdb_status::Ok, "create: {}", err_text(error));
        assert_ne!(admission.tag, bdb_admission_tag::Empty);
        assert_eq!(bdb_db_destroy(take_accepted_db(admission)), bdb_status::Ok);
    });
}

#[test]
fn c_header_compiles_as_c_without_cxx() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("bdb-c-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&out).expect("c-smoke out dir");
    let obj = out.join("c_smoke.o");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = std::process::Command::new(cc)
        .arg("-c")
        .arg("-std=c11")
        .arg("-o")
        .arg(&obj)
        .arg("-I")
        .arg(manifest.join("include"))
        .arg(manifest.join("tests/c_smoke.c"))
        .status()
        .expect("spawn the C compiler");
    assert!(
        status.success(),
        "bumbledb_c.h must compile as C (no C++ compiler)"
    );
}
