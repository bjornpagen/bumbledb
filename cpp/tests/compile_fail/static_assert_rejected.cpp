// Harness seed case: a translation unit the compiler must reject via a
// static_assert with a distinctive, pinned message. It imports a project
// module first, proving compile-fail cases sit on the real module graph.
import std;
import bumbledb.version;

static_assert(sizeof(int) == 0,
    "bumbledb compile-fail harness seed: this translation unit is intentionally rejected");
