/* bumbledb_c.h — the generated C ABI of the bumbledb C++ SDK bridge.
 *
 * GENERATED FILE — DO NOT EDIT. Regenerate from cpp/bridge with:
 *     cbindgen --config cbindgen.toml --crate bumbledb-cpp-bridge \
 *         --output ../foreign/bumbledb_c.h
 * (pinned cbindgen 0.29.4; see cpp/bridge/cbindgen.toml)
 *
 * Boundary protocol: every fallible function returns a bdb_status and
 * takes a trailing bdb_error** out-param — BDB_STATUS_OK (no error),
 * BDB_STATUS_ABORTED (the caller's callback aborted; no error),
 * BDB_STATUS_ERROR (a bdb_error* is written; the caller owns it and
 * frees it with bdb_error_destroy), BDB_STATUS_MISUSE (a contract
 * violation — null required pointer, stale snapshot/tx ref, index out
 * of range; no error is allocated).
 *
 * Lexical capabilities: bdb_snapshot_ref / bdb_tx_ref are valid ONLY
 * inside the callback they are passed to and are invalidated when it
 * returns. They are never owned or destroyed by the caller.
 * bdb_db_write_from may be called from inside a read callback with that
 * callback's still-live snapshot ref; nested writes are refused with a
 * typed BDB_ERROR_KIND_ENVIRONMENT_LOCKED error.
 *
 * View lifetimes: bdb_value string/bytes payloads handed OUT borrow the
 * carrier named at the accessor (bdb_row_set, bdb_answers, bdb_error)
 * and die with it (or on the carrier's next clear/execute). Views handed
 * IN are copied before the call returns; no caller memory is retained.
 */

#ifndef BUMBLEDB_C_H
#define BUMBLEDB_C_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

// The status every fallible export returns (module doc: the boundary
// protocol).
typedef enum bdb_status {
  BDB_STATUS_OK = 0,
  BDB_STATUS_ERROR = 1,
  BDB_STATUS_ABORTED = 2,
  BDB_STATUS_MISUSE = 3,
} bdb_status;

// The value tag — one constant per `bumbledb::Value` variant.
typedef enum bdb_value_kind {
  BDB_VALUE_KIND_BOOL,
  BDB_VALUE_KIND_U64,
  BDB_VALUE_KIND_I64,
  BDB_VALUE_KIND_STRING,
  BDB_VALUE_KIND_FIXED_BYTES,
  BDB_VALUE_KIND_INTERVAL_U64,
  BDB_VALUE_KIND_INTERVAL_I64,
  BDB_VALUE_KIND_ALLEN_MASK,
} bdb_value_kind;

// The execute-parameter tag: a scalar (a [`bdb_value`]) or a param set
// (a value array — points only; the engine types the elements).
typedef enum bdb_param_kind {
  BDB_PARAM_KIND_SCALAR,
  BDB_PARAM_KIND_SET,
} bdb_param_kind;

// The structural value-type tag (`bumbledb::schema::ValueType`, spelled
// C).
typedef enum bdb_value_type_kind {
  BDB_VALUE_TYPE_KIND_BOOL,
  BDB_VALUE_TYPE_KIND_U64,
  BDB_VALUE_TYPE_KIND_I64,
  BDB_VALUE_TYPE_KIND_STRING,
  BDB_VALUE_TYPE_KIND_FIXED_BYTES,
  BDB_VALUE_TYPE_KIND_INTERVAL,
} bdb_value_type_kind;

// An interval's element domain.
typedef enum bdb_interval_element {
  BDB_INTERVAL_ELEMENT_U64,
  BDB_INTERVAL_ELEMENT_I64,
} bdb_interval_element;

// A literal's tag: a plain tagged value, or a closed relation's handle
// by name.
typedef enum bdb_literal_kind {
  BDB_LITERAL_KIND_VALUE,
  BDB_LITERAL_KIND_HANDLE,
} bdb_literal_kind;

// A statement's form tag.
typedef enum bdb_statement_spec_kind {
  BDB_STATEMENT_SPEC_KIND_FD,
  BDB_STATEMENT_SPEC_KIND_CONTAINMENT,
  BDB_STATEMENT_SPEC_KIND_CAPACITY,
} bdb_statement_spec_kind;

// A σ binding's right side: one literal or a literal set (read
// disjunctively).
typedef enum bdb_literal_set_kind {
  BDB_LITERAL_SET_KIND_ONE,
  BDB_LITERAL_SET_KIND_MANY,
} bdb_literal_set_kind;

// A capacity weight's tag.
typedef enum bdb_weight_kind {
  BDB_WEIGHT_KIND_UNIT,
  BDB_WEIGHT_KIND_FIELD,
  BDB_WEIGHT_KIND_DURATION_FIELD,
} bdb_weight_kind;

// A capacity window's tag.
typedef enum bdb_capacity_window_kind {
  BDB_CAPACITY_WINDOW_KIND_EXACT,
  BDB_CAPACITY_WINDOW_KIND_RANGE,
  BDB_CAPACITY_WINDOW_KIND_FLOOR,
} bdb_capacity_window_kind;

// A capacity bound's tag.
typedef enum bdb_bound_kind {
  BDB_BOUND_KIND_LIT,
  BDB_BOUND_KIND_FIELD,
  BDB_BOUND_KIND_DURATION_FIELD,
} bdb_bound_kind;

// A callback's control return: `Ok` commits (write) / completes (read);
// `Abort` abandons — the write delta drops, LMDB untouched, and the outer
// call returns `BDB_STATUS_ABORTED` (the ts bridge's abort sentinel,
// spelled as control flow).
typedef enum bdb_callback_control {
  BDB_CALLBACK_CONTROL_OK = 0,
  BDB_CALLBACK_CONTROL_ABORT = 1,
} bdb_callback_control;

// The C error kind — one constant per engine error family, plus the
// bridge-synthesized `Panic`.
typedef enum bdb_error_kind {
  BDB_ERROR_KIND_SCHEMA,
  BDB_ERROR_KIND_SCHEMA_MISMATCH,
  BDB_ERROR_KIND_FORMAT_MISMATCH,
  BDB_ERROR_KIND_ALREADY_INITIALIZED,
  BDB_ERROR_KIND_NOT_INITIALIZED,
  BDB_ERROR_KIND_ENVIRONMENT_LOCKED,
  BDB_ERROR_KIND_STORE_KIND_MISMATCH,
  BDB_ERROR_KIND_DESCRIPTOR_MISSING,
  BDB_ERROR_KIND_READERS_FULL,
  BDB_ERROR_KIND_VALIDATION,
  BDB_ERROR_KIND_COMMIT_REJECTED,
  BDB_ERROR_KIND_COMMIT_SYNC,
  BDB_ERROR_KIND_GENERATION_MOVED,
  BDB_ERROR_KIND_FOREIGN_SNAPSHOT,
  BDB_ERROR_KIND_FOREIGN_PREPARED,
  BDB_ERROR_KIND_FACT_SHAPE,
  BDB_ERROR_KIND_CLOSED_RELATION_WRITE,
  BDB_ERROR_KIND_FRESH_EXHAUSTED,
  BDB_ERROR_KIND_BULK_LOAD,
  BDB_ERROR_KIND_PARAM,
  BDB_ERROR_KIND_MEASURE_OF_RAY,
  BDB_ERROR_KIND_CAPACITY_RAY_MEASURE,
  BDB_ERROR_KIND_FIXPOINT_BUDGET_EXCEEDED,
  BDB_ERROR_KIND_OVERFLOW,
  BDB_ERROR_KIND_RESULT_BYTES_OVERFLOW,
  BDB_ERROR_KIND_CORRUPTION,
  BDB_ERROR_KIND_IO,
  BDB_ERROR_KIND_LMDB,
  BDB_ERROR_KIND_PANIC,
} bdb_error_kind;

// A violated statement's form tag (`bumbledb::StatementKind`, spelled C).
typedef enum bdb_statement_kind {
  BDB_STATEMENT_KIND_FUNCTIONALITY,
  BDB_STATEMENT_KIND_CONTAINMENT,
  BDB_STATEMENT_KIND_CAPACITY,
} bdb_statement_kind;

// A containment citation's violated side; `None` for key and capacity
// citations.
typedef enum bdb_violation_direction {
  BDB_VIOLATION_DIRECTION_NONE,
  BDB_VIOLATION_DIRECTION_SOURCE_UNSATISFIED,
  BDB_VIOLATION_DIRECTION_TARGET_REQUIRED,
} bdb_violation_direction;

// A head position's tag.
typedef enum bdb_head_term_kind {
  BDB_HEAD_TERM_KIND_VAR,
  BDB_HEAD_TERM_KIND_AGGREGATE,
} bdb_head_term_kind;

// The var-free aggregate-op kind at a head position
// (`bumbledb::ir::HeadOp`).
typedef enum bdb_head_op {
  BDB_HEAD_OP_SUM,
  BDB_HEAD_OP_MIN,
  BDB_HEAD_OP_MAX,
  BDB_HEAD_OP_COUNT,
  BDB_HEAD_OP_COUNT_DISTINCT,
  BDB_HEAD_OP_ARG_MAX,
  BDB_HEAD_OP_ARG_MIN,
  BDB_HEAD_OP_PACK,
} bdb_head_op;

// A find term's tag (`bumbledb::ir::FindTerm`).
typedef enum bdb_find_term_kind {
  BDB_FIND_TERM_KIND_VAR,
  BDB_FIND_TERM_KIND_MEASURE,
  BDB_FIND_TERM_KIND_AGGREGATE,
  BDB_FIND_TERM_KIND_AGGREGATE_MEASURE,
} bdb_find_term_kind;

// An Arg-restriction key position's tag.
typedef enum bdb_arg_key_kind {
  BDB_ARG_KEY_KIND_VAR,
  BDB_ARG_KEY_KIND_MEASURE,
} bdb_arg_key_kind;

// An atom source's tag: a stored relation (`Edb`) or a predicate of the
// same program (`Idb`).
typedef enum bdb_atom_source_kind {
  BDB_ATOM_SOURCE_KIND_EDB,
  BDB_ATOM_SOURCE_KIND_IDB,
} bdb_atom_source_kind;

// A term's tag (`bumbledb::ir::Term`).
typedef enum bdb_term_kind {
  BDB_TERM_KIND_VAR,
  BDB_TERM_KIND_PARAM,
  BDB_TERM_KIND_PARAM_SET,
  BDB_TERM_KIND_LITERAL,
  BDB_TERM_KIND_MEASURE,
} bdb_term_kind;

// A condition node's tag.
typedef enum bdb_condition_kind {
  BDB_CONDITION_KIND_LEAF,
  BDB_CONDITION_KIND_AND,
  BDB_CONDITION_KIND_OR,
} bdb_condition_kind;

// A comparison operator's tag (`bumbledb::ir::CmpOp`). For `PointIn`
// the lhs is the INTERVAL term and the rhs the point term (the engine's
// ordered lowering; the notation reads point-first).
typedef enum bdb_cmp_op_kind {
  BDB_CMP_OP_KIND_EQ,
  BDB_CMP_OP_KIND_NE,
  BDB_CMP_OP_KIND_LT,
  BDB_CMP_OP_KIND_LE,
  BDB_CMP_OP_KIND_GT,
  BDB_CMP_OP_KIND_GE,
  BDB_CMP_OP_KIND_ALLEN,
  BDB_CMP_OP_KIND_POINT_IN,
} bdb_cmp_op_kind;

// The Allen mask position's tag: a literal mask or a param resolved at
// bind.
typedef enum bdb_mask_term_kind {
  BDB_MASK_TERM_KIND_LITERAL,
  BDB_MASK_TERM_KIND_PARAM,
} bdb_mask_term_kind;

// The opaque, reusable answers carrier.
typedef struct bdb_answers bdb_answers;

// The opaque database handle: the engine behind an `Arc` (prepared
// queries co-own it below the boundary — never visible to C++), the
// admitted descriptor (violation rendering, fingerprint readback), and
// the bridge-level writer flag (§17: re-entrant writes are refused typed
// BEFORE the engine's assertion).
typedef struct bdb_db bdb_db;

// The opaque error handle: kind + rendered message + the structured
// payloads the C++ SDK reads back. Owned by the caller after a
// `BDB_STATUS_ERROR` return; freed by [`bdb_error_destroy`].
typedef struct bdb_error bdb_error;

// The opaque prepared-query handle. Field order is load-bearing: the
// prepared value borrows the engine through the `Arc` and must drop
// first (the Node bridge's `PreparedHandle`, verbatim).
typedef struct bdb_prepared bdb_prepared;

// The owned row carrier for scans and point reads: engine values copied
// out whole (one crossing), decoded cell by cell C++-side. Views handed
// out by [`bdb_row_set_get`] borrow this carrier and die with it.
typedef struct bdb_row_set bdb_row_set;

// A borrowed snapshot capability, valid ONLY inside the read callback it
// was passed to (§16). Never owned by C++, never destroyed by C++; every
// use re-checks the alive flag the bridge clears when the callback
// returns, so a stashed ref answers `BDB_STATUS_MISUSE` instead of being
// replayed.
typedef struct bdb_snapshot_ref bdb_snapshot_ref;

// A borrowed write-transaction capability, valid ONLY inside the write
// callback (§17) — the [`bdb_snapshot_ref`] discipline, mutably. Carries
// its engine pointer so `bdb_tx_alloc` can resolve fresh fields without
// a second handle argument.
typedef struct bdb_tx_ref bdb_tx_ref;

// A borrowed UTF-8 text view (NOT NUL-terminated; the length is the
// contract). A null `data` with `len == 0` is the empty string; a null
// `data` under a nonzero `len` is misuse. In optional positions
// (`bdb_field_spec.newtype`) a null `data` means ABSENT.
typedef struct bdb_string_view {
  const uint8_t *data;
  size_t len;
} bdb_string_view;

// A borrowed raw-byte view (`bytes<N>` payloads). Null/len rules as
// [`bdb_string_view`].
typedef struct bdb_bytes_view {
  const uint8_t *data;
  size_t len;
} bdb_bytes_view;

// One tagged value. Only the fields the `kind` names are read; the rest
// are ignored inbound and zeroed outbound. Boring and flat by design —
// no union, no packing.
typedef struct bdb_value {
  enum bdb_value_kind kind;
  bool bool_value;
  uint64_t u64_value;
  int64_t i64_value;
  // `String`: UTF-8 text (checked at the boundary).
  struct bdb_string_view string_value;
  // `FixedBytes`: exactly the field's N bytes (the engine checks N).
  struct bdb_bytes_view bytes_value;
  // `IntervalU64`: half-open `[start, end)`, `start < end` checked.
  uint64_t interval_u64_start;
  uint64_t interval_u64_end;
  // `IntervalI64`: half-open `[start, end)`, `start < end` checked.
  int64_t interval_i64_start;
  int64_t interval_i64_end;
  // `AllenMask`: the low-13-bit mask (checked at the boundary).
  uint16_t allen_mask;
} bdb_value;

// One positional execution argument — the C mirror of the engine's
// public `ParamArg` shape (Scalar | Set; an Allen mask travels as a
// scalar `AllenMask` value).
typedef struct bdb_param {
  enum bdb_param_kind kind;
  // `Scalar`: the value.
  struct bdb_value scalar;
  // `Set`: `set_len` tagged values.
  const struct bdb_value *set;
  size_t set_len;
} bdb_param;

// One structural value type. `fixed_len` is read for `FixedBytes`;
// `element` / `has_width` / `width` for `Interval` (`has_width == false`
// is the general 16-byte interval; `true` the fixed-width family).
typedef struct bdb_value_type {
  enum bdb_value_type_kind kind;
  uint16_t fixed_len;
  enum bdb_interval_element element;
  bool has_width;
  uint64_t width;
} bdb_value_type;

// One field: name, structural type, optional host newtype label (null
// `data` = absent; carried for closed-handle resolution only, dropped at
// descriptor lowering), and the `fresh` mark.
typedef struct bdb_field_spec {
  struct bdb_string_view name;
  struct bdb_value_type value_type;
  struct bdb_string_view newtype;
  bool fresh;
} bdb_field_spec;

// One literal as spelled.
typedef struct bdb_literal {
  enum bdb_literal_kind kind;
  struct bdb_value value;
  struct bdb_string_view handle;
} bdb_literal;

// One ground axiom of a closed relation: the handle plus one literal per
// declared intrinsic column, in field-declaration order.
typedef struct bdb_closed_row {
  struct bdb_string_view handle;
  const struct bdb_literal *values;
  size_t value_count;
} bdb_closed_row;

// A closed relation's closed half: the handle newtype and the ground
// axioms, fused (absent `closed` on the relation = ordinary relation).
typedef struct bdb_closed_spec {
  struct bdb_string_view newtype;
  const struct bdb_closed_row *rows;
  size_t row_count;
} bdb_closed_spec;

// One relation: name, declared fields, and closedness (`closed` null =
// ordinary).
typedef struct bdb_relation_spec {
  struct bdb_string_view name;
  const struct bdb_field_spec *fields;
  size_t field_count;
  const struct bdb_closed_spec *closed;
} bdb_relation_spec;

// One literal set. `One` reads `literals[0]` (`literal_count` must be
// 1); `Many` reads all `literal_count` entries.
typedef struct bdb_literal_set {
  enum bdb_literal_set_kind kind;
  const struct bdb_literal *literals;
  size_t literal_count;
} bdb_literal_set;

// One σ binding: `field == literal-or-set`.
typedef struct bdb_selection_binding {
  struct bdb_string_view field;
  struct bdb_literal_set set;
} bdb_selection_binding;

// One side of a containment/capacity statement:
// `Relation(projection… | selection…)`, all names.
typedef struct bdb_side {
  struct bdb_string_view relation;
  const struct bdb_string_view *projection;
  size_t projection_count;
  const struct bdb_selection_binding *selection;
  size_t selection_count;
} bdb_side;

// A capacity weight; `field` is read for `Field`/`DurationField`.
typedef struct bdb_weight {
  enum bdb_weight_kind kind;
  struct bdb_string_view field;
} bdb_weight;

// One capacity bound; `lit` for `Lit`, `field` for
// `Field`/`DurationField`.
typedef struct bdb_bound {
  enum bdb_bound_kind kind;
  uint64_t lit;
  struct bdb_string_view field;
} bdb_bound;

// One capacity window: `Exact` reads `lo` as the exact bound; `Floor`
// reads `lo`; `Range` reads `lo` and `hi`.
typedef struct bdb_capacity_window {
  enum bdb_capacity_window_kind kind;
  struct bdb_bound lo;
  struct bdb_bound hi;
} bdb_capacity_window;

// One dependency statement. `Fd` reads `fd_relation` +
// `fd_projection`; `Containment` reads `source`/`target`/`bidirectional`;
// `Capacity` reads `target`/`weight`/`window`/`source` (the operator's
// read order).
typedef struct bdb_statement_spec {
  enum bdb_statement_spec_kind kind;
  struct bdb_string_view fd_relation;
  const struct bdb_string_view *fd_projection;
  size_t fd_projection_count;
  struct bdb_side source;
  struct bdb_side target;
  bool bidirectional;
  struct bdb_weight weight;
  struct bdb_capacity_window window;
} bdb_statement_spec;

// The whole schema spec: relations then statements, declaration order —
// the order IS the id mint.
typedef struct bdb_schema_spec {
  const struct bdb_relation_spec *relations;
  size_t relation_count;
  const struct bdb_statement_spec *statements;
  size_t statement_count;
} bdb_schema_spec;

// The 64 lowercase hex chars of the store's schema fingerprint — the
// cross-host identity (NOT NUL-terminated; the width is the type).
typedef struct bdb_fingerprint {
  uint8_t hex[64];
} bdb_fingerprint;

// The read callback: synchronous, on the calling thread, with a
// snapshot ref valid only until it returns.
typedef enum bdb_callback_control (*bdb_read_callback)(void *context,
                                                       const struct bdb_snapshot_ref *snapshot);

// The write callback: synchronous, on the calling thread, with a tx ref
// valid only until it returns. `Ok` commits the delta (the engine judges
// dependencies against the final state); `Abort` drops it — LMDB never
// saw a fact.
typedef enum bdb_callback_control (*bdb_write_callback)(void *context,
                                                        struct bdb_tx_ref *transaction);

// One borrowed bulk-import row: `value_count` tagged values in
// declaration order.
typedef struct bdb_row_view {
  const struct bdb_value *values;
  size_t value_count;
} bdb_row_view;

// One rendered violation of a rejected commit, viewed: the statement's
// fingerprint-pinned id, its form tag, its canonical spelling (borrowed
// from the owning [`bdb_error`]), the containment direction where the
// form has one, and the capacity measure (u128 as two u64 words) where
// the form has one.
typedef struct bdb_violation {
  uint16_t statement;
  enum bdb_statement_kind kind;
  struct bdb_string_view spelling;
  enum bdb_violation_direction direction;
  bool has_measure;
  uint64_t measure_lo;
  uint64_t measure_hi;
} bdb_violation;

// One head position; `op` is read for `Aggregate`.
typedef struct bdb_head_term {
  enum bdb_head_term_kind kind;
  enum bdb_head_op op;
} bdb_head_term;

// One rule-scoped aggregate op: the kind, plus the Arg key for
// `ArgMax`/`ArgMin` (ignored for every other kind).
typedef struct bdb_agg_op {
  enum bdb_head_op kind;
  enum bdb_arg_key_kind arg_key_kind;
  uint16_t arg_key_var;
} bdb_agg_op;

// One find term. `var` is read for `Var`/`Measure`; `op` plus
// `has_over`/`over` for `Aggregate` (`has_over == false` is the nullary
// `Count`); `op` plus `over` for `AggregateMeasure`.
typedef struct bdb_find_term {
  enum bdb_find_term_kind kind;
  uint16_t var;
  struct bdb_agg_op op;
  bool has_over;
  uint16_t over;
} bdb_find_term;

// One term. `var` is read for `Var`/`Measure`, `param` for
// `Param`/`ParamSet`, `literal` for `Literal`.
typedef struct bdb_term {
  enum bdb_term_kind kind;
  uint16_t var;
  uint16_t param;
  struct bdb_value literal;
} bdb_term;

// One atom binding: `(field, term)`. Absence of a field is the
// wildcard — bind only what the rule constrains.
typedef struct bdb_binding {
  uint16_t field;
  struct bdb_term term;
} bdb_binding;

// One atom. `relation` is read for `Edb`, `pred` for `Idb`.
typedef struct bdb_atom {
  enum bdb_atom_source_kind source_kind;
  uint32_t relation;
  uint16_t pred;
  const struct bdb_binding *bindings;
  size_t binding_count;
} bdb_atom;

// One comparison operator; the mask fields are read for `Allen` only
// (`mask` for a `Literal` mask term, `mask_param` for a `Param` one).
typedef struct bdb_cmp_op {
  enum bdb_cmp_op_kind kind;
  enum bdb_mask_term_kind mask_kind;
  uint16_t mask;
  uint16_t mask_param;
} bdb_cmp_op;

// One comparison condition.
typedef struct bdb_comparison {
  struct bdb_cmp_op op;
  struct bdb_term lhs;
  struct bdb_term rhs;
} bdb_comparison;

// One condition-tree node. `cmp` is read for `Leaf`;
// `children`/`child_count` for `And`/`Or`. Nesting past the engine's
// `MAX_CONDITION_DEPTH` is refused at marshal (the engine's own bound,
// re-checked here so the recursion is stack-safe on hostile input).
typedef struct bdb_condition {
  enum bdb_condition_kind kind;
  struct bdb_comparison cmp;
  const struct bdb_condition *children;
  size_t child_count;
} bdb_condition;

// One rule: finds against the head, positive atoms, negated atoms,
// conditions (conjoined).
typedef struct bdb_rule {
  const struct bdb_find_term *finds;
  size_t find_count;
  const struct bdb_atom *atoms;
  size_t atom_count;
  const struct bdb_atom *negated;
  size_t negated_count;
  const struct bdb_condition *conditions;
  size_t condition_count;
} bdb_rule;

// One predicate: the head shape its rules align against, and the rules.
typedef struct bdb_predicate {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_predicate;

// The whole program: predicates (`pred` = index) and the output
// predicate. A query is the one-predicate program with `output == 0`.
typedef struct bdb_program {
  const struct bdb_predicate *predicates;
  size_t predicate_count;
  uint16_t output;
} bdb_program;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Mints an empty answers carrier (never fails; owns nothing yet).
struct bdb_answers *bdb_answers_new(void);

// Empties the carrier, retaining capacity (the zero-alloc reuse path).
enum bdb_status bdb_answers_clear(struct bdb_answers *answers);

// Number of answers (0 for a null handle).
size_t bdb_answers_len(const struct bdb_answers *answers);

// Number of columns — the executed query's find terms, in order (0 for
// a null handle).
size_t bdb_answers_arity(const struct bdb_answers *answers);

// One answer cell, viewed — string/bytes payloads BORROW the carrier
// and are valid only while it is alive, un-cleared, and un-re-executed.
// Bounds-checked bridge-side: `BDB_STATUS_MISUSE` out of range, never a
// panic.
enum bdb_status bdb_answers_get(const struct bdb_answers *answers,
                                size_t row,
                                size_t column,
                                struct bdb_value *out_value);

// Frees the carrier (invalidating every view borrowed from it).
enum bdb_status bdb_answers_destroy(struct bdb_answers *answers);

// Executes a prepared query against the snapshot with positional
// params, filling the caller's reusable carrier (cleared first,
// capacity retained — the `execute_into` lane, §23). The prepared handle
// is taken exclusively for the call (`&mut` on the engine side — one
// execution at a time, §20/§22); executing a prepared query against a
// snapshot of a different database is the engine's own typed
// `BDB_ERROR_KIND_FOREIGN_PREPARED`.
enum bdb_status bdb_snapshot_execute(const struct bdb_snapshot_ref *snapshot,
                                     struct bdb_prepared *prepared,
                                     const struct bdb_param *params,
                                     size_t param_count,
                                     struct bdb_answers *answers,
                                     struct bdb_error **out_error);

// Creates a fresh DURABLE store at `path` from a schema spec. Schema
// resolution/validation failures are `BDB_ERROR_KIND_SCHEMA`.
enum bdb_status bdb_db_create(struct bdb_string_view path,
                              const struct bdb_schema_spec *spec,
                              struct bdb_db **out_db,
                              struct bdb_error **out_error);

// Opens an existing durable store, verifying format version, store
// kind, and schema fingerprint (`BDB_ERROR_KIND_SCHEMA_MISMATCH` on drift).
enum bdb_status bdb_db_open(struct bdb_string_view path,
                            const struct bdb_schema_spec *spec,
                            struct bdb_db **out_db,
                            struct bdb_error **out_error);

// Opens or initializes an EPHEMERAL store at `path` (`MDB_NOSYNC`; a
// machine crash loses the store by the kind's own claim — every other
// semantic is identical to a durable store).
enum bdb_status bdb_db_ephemeral(struct bdb_string_view path,
                                 const struct bdb_schema_spec *spec,
                                 struct bdb_db **out_db,
                                 struct bdb_error **out_error);

// Destroys the handle: prepared queries keep their own engine reference
// (the `Arc` below the boundary), so the environment — and its exclusive
// lock — releases when the last of them is destroyed.
enum bdb_status bdb_db_destroy(struct bdb_db *db);

// The open store's schema fingerprint, 64 lowercase hex chars — the
// cross-host identity readback (the Node bridge's
// `dbFingerprint`, verbatim): `create` stored this exact value and
// `open` verified it, so the descriptor's fingerprint IS the store's.
// Dumb-bridge legal: validation and blake3 are the ENGINE's own
// functions re-run on the already-admitted descriptor; the bridge only
// hex-encodes the 32 bytes.
enum bdb_status bdb_db_fingerprint(const struct bdb_db *db,
                                   struct bdb_fingerprint *out_fingerprint,
                                   struct bdb_error **out_error);

// Runs `callback` over one consistent read snapshot (§16): the engine's
// `Db::read` closure model, synchronous on the calling thread. The
// snapshot ref is invalidated when the callback returns.
// `BDB_STATUS_ABORTED` when the callback returned `Abort`.
enum bdb_status bdb_db_read(const struct bdb_db *db,
                            bdb_read_callback callback,
                            void *context,
                            struct bdb_error **out_error);

// Runs `callback` as the single writer (§17): the engine's `Db::write`
// closure model. `Ok` from the callback commits — the dependency
// judgment runs against the final state, and a rejection is
// `BDB_ERROR_KIND_COMMIT_REJECTED` carrying the complete violation set.
// `Abort` drops the delta (`BDB_STATUS_ABORTED`; LMDB untouched).
// Re-entrant writes on this handle are refused with
// `BDB_ERROR_KIND_ENVIRONMENT_LOCKED` before the engine's assertion.
enum bdb_status bdb_db_write(const struct bdb_db *db,
                             bdb_write_callback callback,
                             void *context,
                             struct bdb_error **out_error);

// `bdb_db_write` conditional on a still-live snapshot (§18): the
// engine's `Db::write_from`. Callable from inside the read callback that
// owns `snapshot` (the sanctioned nesting — module doc). A
// state-changing commit since the snapshot returns
// `BDB_ERROR_KIND_GENERATION_MOVED` (payload: witnessed/current); retry is
// host policy.
enum bdb_status bdb_db_write_from(const struct bdb_db *db,
                                  const struct bdb_snapshot_ref *snapshot,
                                  bdb_write_callback callback,
                                  void *context,
                                  struct bdb_error **out_error);

// Records an insert into the delta; `out_changed` = whether the final
// state changed. Values are the relation's sealed fields in declaration
// order; shape violations are typed `BDB_ERROR_KIND_FACT_SHAPE` — nothing is
// judged until commit.
enum bdb_status bdb_tx_insert(const struct bdb_tx_ref *transaction,
                              uint32_t relation,
                              const struct bdb_value *values,
                              size_t value_count,
                              bool *out_changed,
                              struct bdb_error **out_error);

// Records a delete into the delta; `out_changed` = whether the final
// state changed.
enum bdb_status bdb_tx_delete(const struct bdb_tx_ref *transaction,
                              uint32_t relation,
                              const struct bdb_value *values,
                              size_t value_count,
                              bool *out_changed,
                              struct bdb_error **out_error);

// Final-state membership (base + pending delta — the view the commit
// judgment judges, which is what makes check-then-act race-free).
enum bdb_status bdb_tx_contains(const struct bdb_tx_ref *transaction,
                                uint32_t relation,
                                const struct bdb_value *values,
                                size_t value_count,
                                bool *out_contains,
                                struct bdb_error **out_error);

// Final-state point lookup through a key statement (`key_values` in the
// statement's projection order). A hit writes a one-row
// [`bdb_row_set`] the caller owns; a miss writes null.
enum bdb_status bdb_tx_get(const struct bdb_tx_ref *transaction,
                           uint32_t relation,
                           uint16_t key_statement,
                           const struct bdb_value *key_values,
                           size_t key_value_count,
                           struct bdb_row_set **out_row,
                           struct bdb_error **out_error);

// Mints the next fresh value for `(relation, field)` — resolve-once,
// mint-per-row is the engine's own split (`Db::fresh_field` +
// `WriteTx::alloc_at`); the bridge re-resolves per call because the C
// surface carries no witness type (ids at this surface are data; a
// mis-aimed pair is typed `BDB_ERROR_KIND_FACT_SHAPE`).
enum bdb_status bdb_tx_alloc(const struct bdb_tx_ref *transaction,
                             uint32_t relation,
                             uint16_t field,
                             uint64_t *out_id,
                             struct bdb_error **out_error);

// Committed-state membership of one dynamic fact (sealed field order).
enum bdb_status bdb_snapshot_contains(const struct bdb_snapshot_ref *snapshot,
                                      uint32_t relation,
                                      const struct bdb_value *values,
                                      size_t value_count,
                                      bool *out_contains,
                                      struct bdb_error **out_error);

// Committed-state point lookup of the full fact through a key statement
// (`key_values` in the statement's projection order). A hit writes a
// one-row [`bdb_row_set`] the caller owns; a miss writes null.
enum bdb_status bdb_snapshot_get(const struct bdb_snapshot_ref *snapshot,
                                 uint32_t relation,
                                 uint16_t key_statement,
                                 const struct bdb_value *key_values,
                                 size_t key_value_count,
                                 struct bdb_row_set **out_row,
                                 struct bdb_error **out_error);

// Full-relation export in `row_id` order (the ETL/derivation read):
// one owned [`bdb_row_set`] crossing, iterated C++-side — never one FFI
// call per cell (§37).
enum bdb_status bdb_snapshot_scan(const struct bdb_snapshot_ref *snapshot,
                                  uint32_t relation,
                                  struct bdb_row_set **out_rows,
                                  struct bdb_error **out_error);

// Bulk import (`Db::bulk_load_dyn`): atomic 4096-row chunks; prior
// chunks stay committed on failure — `out_committed` always carries the
// durable count (§24), and a failure is `BDB_ERROR_KIND_BULK_LOAD` (the same
// count readable via `bdb_error_get_bulk_committed`, the underlying
// cause in the message). The importer owns dependency ordering: a
// bidirectional statement cluster must land within one chunk.
enum bdb_status bdb_db_bulk_load(const struct bdb_db *db,
                                 uint32_t relation,
                                 const struct bdb_row_view *rows,
                                 size_t row_count,
                                 uint64_t *out_committed,
                                 struct bdb_error **out_error);

// Number of rows.
size_t bdb_row_set_len(const struct bdb_row_set *rows);

// The row's cell count (sealed field order — every row of one scan has
// the relation's arity).
size_t bdb_row_set_arity(const struct bdb_row_set *rows, size_t row);

// One cell, viewed — string/bytes payloads BORROW the row set and die
// with it. Bounds-checked: `BDB_STATUS_MISUSE` out of range.
enum bdb_status bdb_row_set_get(const struct bdb_row_set *rows,
                                size_t row,
                                size_t column,
                                struct bdb_value *out_value);

// Frees a row set (invalidating every view borrowed from it).
enum bdb_status bdb_row_set_destroy(struct bdb_row_set *rows);

// The error's kind. A null handle answers `Panic` — the accessor cannot
// carry a status, and `Panic` is the one kind that always means "stop
// trusting this process's bridge state".
enum bdb_error_kind bdb_error_get_kind(const struct bdb_error *error);

// The rendered message, borrowed from the error (valid until
// `bdb_error_destroy`). UTF-8, NOT NUL-terminated — the length is the
// contract.
enum bdb_status bdb_error_get_message(const struct bdb_error *error,
                                      struct bdb_string_view *out_message);

// The `GenerationMoved` payload: the witnessed and current generations.
// `BDB_STATUS_MISUSE` when the error is not `BDB_ERROR_KIND_GENERATION_MOVED`.
enum bdb_status bdb_error_get_generation_moved(const struct bdb_error *error,
                                               uint64_t *out_witnessed,
                                               uint64_t *out_current);

// The `BulkLoad` payload: facts durable in the chunks committed before
// the failure. `BDB_STATUS_MISUSE` when the error is
// not `BDB_ERROR_KIND_BULK_LOAD`.
enum bdb_status bdb_error_get_bulk_committed(const struct bdb_error *error,
                                             uint64_t *out_committed);

// The rendered violation count of a `BDB_ERROR_KIND_COMMIT_REJECTED` error
// (0 for every other kind, and for a null handle).
size_t bdb_error_violation_count(const struct bdb_error *error);

// One rendered violation, viewed (the spelling borrows from the error —
// valid until `bdb_error_destroy`). Bounds-checked:
// `BDB_STATUS_MISUSE` past [`bdb_error_violation_count`].
enum bdb_status bdb_error_get_violation(const struct bdb_error *error,
                                        size_t index,
                                        struct bdb_violation *out_violation);

// Frees an error. Exactly once per owned error; a null pointer is
// misuse.
enum bdb_status bdb_error_destroy(struct bdb_error *error);

// Prepares a program against the database: the engine validates,
// normalizes, reads statistics, and plans ONCE; the returned handle is
// reusable across snapshots of this database (`&mut` per execution —
// one execution at a time; the handle is not thread-shareable).
// Validation (roster) failures are `BDB_ERROR_KIND_VALIDATION`.
enum bdb_status bdb_db_prepare(const struct bdb_db *db,
                               const struct bdb_program *program,
                               struct bdb_prepared **out_prepared,
                               struct bdb_error **out_error);

// Releases a prepared query (its plan, memo, and engine reference).
enum bdb_status bdb_prepared_destroy(struct bdb_prepared *prepared);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* BUMBLEDB_C_H */
