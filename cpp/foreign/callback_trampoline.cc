/**
 * Exception wall for C ABI read/write callbacks. Compiled with
 * exceptions enabled (this TU only): a throw from a throw-enabled caller
 * becomes Abort and never unwinds into Rust. In-tree SDK TUs are
 * -fno-exceptions and cannot throw.
 */
#include <cstdint>
#include <exception>

struct bdb_snapshot_ref;
struct bdb_tx_ref;

extern "C" std::uint32_t bdb_invoke_read_callback(
    std::uint32_t (*callback)(void* context, bdb_snapshot_ref const* snapshot), void* context, bdb_snapshot_ref const* snapshot) {
	try {
		return callback(context, snapshot);
	} catch (...) {
		/* BDB_CALLBACK_CONTROL_ABORT — never let a C++ exception enter Rust */
		return 1;
	}
}

extern "C" std::uint32_t bdb_invoke_write_callback(
    std::uint32_t (*callback)(void* context, bdb_tx_ref* transaction), void* context, bdb_tx_ref* transaction) {
	try {
		return callback(context, transaction);
	} catch (...) {
		/* BDB_CALLBACK_CONTROL_ABORT — never let a C++ exception enter Rust */
		return 1;
	}
}
