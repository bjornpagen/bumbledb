// bumbledb_foreign — the primary module interface of the quarantine
// boundary (TODO_CPP §31, AGENTS.md): the raw C ABI re-export (:abi) and
// the safe RAII adaptation surface over it (:raii). The pre-schema spec
// lane imports this module explicitly and dies with it; the umbrella
// `bumbledb` module deliberately does NOT re-export it.
export module bumbledb_foreign;

export import :abi;
export import :raii;
