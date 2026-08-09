export module bumbledb:version;

import std;

export namespace bdb {

/**
 * The SDK's own version string, not the engine's.
 */
[[nodiscard]] auto version() -> std::string_view {
	return "0.0.0-dev";
}

}
