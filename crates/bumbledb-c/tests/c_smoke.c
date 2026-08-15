/*
 * Pure-C compile smoke for bumbledb_c.h. Compiled via cc as C, never C++.
 * Unresolved bdb_* symbols are fine: this translation unit is archived,
 * not linked. A host that links libbumbledb_c resolves them.
 */
#include "bumbledb_c.h"

int bdb_c_smoke(void) {
	const char *version = bdb_version();
	uint32_t abi = bdb_abi_version();
	struct bdb_value value;
	struct bdb_string_view view;
	if (version == NULL) {
		return 1;
	}
	if (abi == 0) {
		return 2;
	}
	value.kind = (uint32_t)BDB_VALUE_KIND_U64;
	value.u64_value = 0;
	view.data = NULL;
	view.len = 0;
	(void)value;
	(void)view;
	(void)BDB_STATUS_OK;
	(void)BDB_CALLBACK_CONTROL_OK;
	return 0;
}
