// The quarantine boundary (TODO_CPP §31, AGENTS.md §5.3): the ONE
// translation unit allowed to include the generated C ABI header. It
// re-exports the raw ABI surface — opaque handles, tag enums, view
// structs, callback types, and the 37 functions — inside bdb::foreign so
// the dialect layer above consumes a named module, never the header.
// Raw pointers and preprocessing are legal HERE and nowhere above.
module;

#include "bumbledb_c.h"

export module bumbledb_foreign:abi;

namespace bdb::foreign {

// --- opaque handles (always spoken of through pointers; never dereferenced
// host-side) -----------------------------------------------------------------
export using ::bdb_answers;
export using ::bdb_db;
export using ::bdb_error;
export using ::bdb_prepared;
export using ::bdb_row_set;
export using ::bdb_snapshot_ref;
export using ::bdb_tx_ref;

// --- status / kind / control enums -------------------------------------------
export using ::bdb_status;
export using ::bdb_value_kind;
export using ::bdb_param_kind;
export using ::bdb_value_type_kind;
export using ::bdb_interval_element;
export using ::bdb_literal_kind;
export using ::bdb_statement_spec_kind;
export using ::bdb_literal_set_kind;
export using ::bdb_weight_kind;
export using ::bdb_capacity_window_kind;
export using ::bdb_bound_kind;
export using ::bdb_callback_control;
export using ::bdb_error_kind;
export using ::bdb_statement_kind;
export using ::bdb_violation_direction;
export using ::bdb_head_term_kind;
export using ::bdb_head_op;
export using ::bdb_find_term_kind;
export using ::bdb_arg_key_kind;
export using ::bdb_atom_source_kind;
export using ::bdb_term_kind;
export using ::bdb_condition_kind;
export using ::bdb_cmp_op_kind;
export using ::bdb_mask_term_kind;

// --- value / param / spec views ----------------------------------------------
export using ::bdb_string_view;
export using ::bdb_bytes_view;
export using ::bdb_value;
export using ::bdb_param;
export using ::bdb_value_type;
export using ::bdb_field_spec;
export using ::bdb_literal;
export using ::bdb_closed_row;
export using ::bdb_closed_spec;
export using ::bdb_relation_spec;
export using ::bdb_literal_set;
export using ::bdb_selection_binding;
export using ::bdb_side;
export using ::bdb_weight;
export using ::bdb_bound;
export using ::bdb_capacity_window;
export using ::bdb_statement_spec;
export using ::bdb_schema_spec;
export using ::bdb_fingerprint;
export using ::bdb_row_view;
export using ::bdb_violation;

// --- program / query IR views ------------------------------------------------
export using ::bdb_head_term;
export using ::bdb_agg_op;
export using ::bdb_find_term;
export using ::bdb_term;
export using ::bdb_binding;
export using ::bdb_atom;
export using ::bdb_cmp_op;
export using ::bdb_comparison;
export using ::bdb_condition;
export using ::bdb_rule;
export using ::bdb_predicate;
export using ::bdb_program;

// --- callback types ------------------------------------------------------------
export using ::bdb_read_callback;
export using ::bdb_write_callback;

// --- the 37 functions ----------------------------------------------------------
// answers carrier
export using ::bdb_answers_new;
export using ::bdb_answers_clear;
export using ::bdb_answers_len;
export using ::bdb_answers_arity;
export using ::bdb_answers_get;
export using ::bdb_answers_destroy;

// execution
export using ::bdb_snapshot_execute;

// database lifecycle
export using ::bdb_db_create;
export using ::bdb_db_open;
export using ::bdb_db_ephemeral;
export using ::bdb_db_destroy;
export using ::bdb_db_fingerprint;
export using ::bdb_db_read;
export using ::bdb_db_write;
export using ::bdb_db_write_from;

// write transaction
export using ::bdb_tx_insert;
export using ::bdb_tx_delete;
export using ::bdb_tx_contains;
export using ::bdb_tx_get;
export using ::bdb_tx_alloc;

// snapshot reads
export using ::bdb_snapshot_contains;
export using ::bdb_snapshot_get;
export using ::bdb_snapshot_scan;

// bulk import
export using ::bdb_db_bulk_load;

// row sets
export using ::bdb_row_set_len;
export using ::bdb_row_set_arity;
export using ::bdb_row_set_get;
export using ::bdb_row_set_destroy;

// errors
export using ::bdb_error_get_kind;
export using ::bdb_error_get_message;
export using ::bdb_error_get_generation_moved;
export using ::bdb_error_get_bulk_committed;
export using ::bdb_error_violation_count;
export using ::bdb_error_get_violation;
export using ::bdb_error_destroy;

// prepared queries
export using ::bdb_db_prepare;
export using ::bdb_prepared_destroy;

} // namespace bdb::foreign
