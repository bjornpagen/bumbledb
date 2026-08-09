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
	bdb::Db::ephemeral("/tmp/bdb-discard-probe", Theory);
}

}

auto main() -> int {
	discard_a_fallible_result();
	return 0;
}
