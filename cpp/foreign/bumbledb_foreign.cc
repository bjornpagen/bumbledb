/**
 * The quarantine boundary's primary interface: the raw C ABI re-export
 * (:abi) and the safe RAII surface over it (:raii). The umbrella
 * `bumbledb` module deliberately does NOT re-export this module; only
 * the pre-schema spec lane imports it explicitly.
 */
export module bumbledb_foreign;

export import :abi;
export import :raii;
