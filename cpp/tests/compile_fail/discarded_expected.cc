// The nodiscard law (AGENTS.md §26): a discarded expected is a discarded
// error. Every fallible SDK operation is [[nodiscard]]; ignoring the result
// must fail the build, not swallow the failure.
import std;
import bumbledb;

namespace {

struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;
	std::string name;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Theory = bdb::schema<"Theory">(Service);

auto discard_a_fallible_result() -> void {
	// Db::ephemeral returns std::expected<Db, Error>; dropping it on the
	// floor discards both the database and any error it carried.
	bdb::Db::ephemeral("/tmp/bdb-discard-probe", Theory);
}

} // namespace

auto main() -> int {
	discard_a_fallible_result();
	return 0;
}
