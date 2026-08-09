// :version — the SDK's own version string (not the engine's).
export module bumbledb:version;

import std;

export namespace bdb {

/// The SDK's own version string (not the engine's): scaffold seed so the
/// module graph has one real exported module to build, import, and test.
auto version() -> std::string_view {
	return "0.0.0-dev";
}

} // namespace bdb
