// Cookbook recipe 22 — Union reads (ts/COOKBOOK.md §22): the whole-DU
// read is a SET OF RULES — one head, one rule per arm; disjunction is
// data at the top, never an execution node. The exclusivity theorem
// (recipe 2) is spent a third time here: rules selecting different `kind`
// handles are provably disjoint, so the executor elides cross-rule dedup.
// The shared head column `n` draws from DIFFERENT bare u64 columns per
// rule — `bdb::as<"n">(...)` decouples the head name from the field.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r22 <64-hex>"); `wholeDu` (two rules, same head {id, n}, handle
// literals per rule) prepares through the real engine validator.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Kind = bdb::closed<"Kind", "Card", "Ach">();

struct PaymentRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Kind.id> kind;
};

struct CardRow {
	std::uint64_t payment;
	std::uint64_t last4;
};

struct AchRow {
	std::uint64_t payment;
	std::uint64_t routing;
};

inline constexpr auto Payment = bdb::relation<"Payment", PaymentRow>;
inline constexpr auto Card = bdb::relation<"Card", CardRow>;
inline constexpr auto Ach = bdb::relation<"Ach", AchRow>;

inline constexpr auto Payments =
    bdb::schema<"Payments">(Kind, Payment, Card, Ach,

                            bdb::contained(bdb::on(Payment.kind), bdb::on(Kind.id)), bdb::key(Card.payment), bdb::key(Ach.payment),
                            bdb::mirrors(bdb::on(bdb::where(Payment, {.kind = Kind.Card}), Payment.id), bdb::on(Card.payment)),
                            bdb::mirrors(bdb::on(bdb::where(Payment, {.kind = Kind.Ach}), Payment.id), bdb::on(Ach.payment)));

// One query, two rules (set union): every rule derives the same head
// {id, n} — `n` is Card.last4 in one arm, Ach.routing in the other.
inline constexpr auto WholeDu = bdb::query(Payments)
                                    .rule([](auto r) consteval {
	                                    auto payment = r.vars(Payment);
	                                    auto card = r.vars(Card);
	                                    return r
	                                        .match(Payment,
	                                               {
	                                                   .id = payment.id,
	                                                   .kind = Kind.Card,
	                                               })
	                                        .match(Card,
	                                               {
	                                                   .payment = payment.id,
	                                                   .last4 = card.last4,
	                                               })
	                                        .find(
	                                            {
	                                                .id = payment.id,
	                                            },
	                                            bdb::as<"n">(card.last4));
                                    })
                                    .rule([](auto r) consteval {
	                                    auto payment = r.vars(Payment);
	                                    auto ach = r.vars(Ach);
	                                    return r
	                                        .match(Payment,
	                                               {
	                                                   .id = payment.id,
	                                                   .kind = Kind.Ach,
	                                               })
	                                        .match(Ach,
	                                               {
	                                                   .payment = payment.id,
	                                                   .routing = ach.routing,
	                                               })
	                                        .find(
	                                            {
	                                                .id = payment.id,
	                                            },
	                                            bdb::as<"n">(ach.routing));
                                    });

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

/// The golden of one recipe: the fixtures file is one `rNN <64-hex>` line
/// per recipe (ts/test/cookbook.test.ts reads the same file).
[[nodiscard]] auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
	for (auto const line_range : std::views::split(fixtures, '\n')) {
		auto const line = std::string_view{line_range};
		if (!line.starts_with(recipe)) {
			continue;
		}
		auto const rest = line.substr(recipe.size());
		if (!rest.starts_with(' ')) {
			continue;
		}
		auto hex = rest.substr(1);
		while (!hex.empty() && (hex.back() == '\r' || hex.back() == ' ')) {
			hex.remove_suffix(1);
		}
		if (hex.size() != 64) {
			return std::nullopt;
		}
		return std::string{hex};
	}
	return std::nullopt;
}

[[nodiscard]] auto slurp(std::string_view path) -> std::optional<std::string> {
	auto stream = std::ifstream{std::string{path}, std::ios::binary | std::ios::ate};
	if (!stream) {
		return std::nullopt;
	}
	auto const size = stream.tellg();
	if (size < 0) {
		return std::nullopt;
	}
	auto text = std::string(static_cast<std::size_t>(size), '\0');
	stream.seekg(0);
	stream.read(text.data(), size);
	if (!stream) {
		return std::nullopt;
	}
	return text;
}

[[nodiscard]] auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r22-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r22") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r22 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r22 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Payments);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Payments", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r22 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto whole_du = db->prepare<WholeDu>();
	results.push_back(CaseResult{
	    .name = "wholeDu (two rules, one head — the set union) prepares "
	            "through the engine validator",
	    .passed = whole_du.has_value(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r22_payments <fixtures-file>");
		return 1;
	}

	auto results = std::vector<CaseResult>{};
	run_cases(std::string_view{arguments[1]}, results);

	auto failures = std::size_t{0};
	for (auto const& result : results) {
		if (result.passed) {
			std::println("pass: {}", result.name);
		} else {
			std::println("FAIL: {}", result.name);
			++failures;
		}
	}
	return failures == 0 ? 0 : 1;
}
