// Cookbook recipe 24 — The closure idiom (ts/COOKBOOK.md §24): both
// dialects agree, root for root. The HOST dialect is a loop over one
// prepared ∈-set query (the frontier discipline IS semi-naive
// evaluation's Δ); the ENGINE dialect is one stratified program under the
// fixpoint driver (`bdb::program` / `bdb::rec` / `.idb` — the
// self-recursion cut as construction law); and the complement program
// negates the FINISHED stratum (`.not_idb` — negation OF a lower stratum
// is engine-legal; the strata judge refuses only negation *through* a
// cycle).
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r24 <64-hex>"); all three shapes prepare through the real engine
// validator; the host loop and the native program answer the SAME
// closure, root for root; and the complement equals scan-minus-closure.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct NodeRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct ParentRow {
	std::uint64_t child;
	std::uint64_t parent;
};

inline constexpr auto Node = bdb::relation<"Node", NodeRow>;
inline constexpr auto Parent = bdb::relation<"Parent", ParentRow>;

inline constexpr auto Closure = bdb::schema<"Closure">(Node, Parent,

                                                       bdb::key(Parent.child), bdb::contained(bdb::on(Parent.child), bdb::on(Node.id)),
                                                       bdb::contained(bdb::on(Parent.parent), bdb::on(Node.id)));

// The loop's one query — the frontier's children, one ∈-set probe (the
// runtime set param binds a span at execute).
inline constexpr auto Step = bdb::query(Closure).rule([](auto r) consteval {
	auto vars = r.vars(Parent);
	return r
	    .match(Parent,
	           {
	               .child = vars.child,
	               .parent = bdb::set_param<"frontier">(),
	           })
	    .find({}, bdb::as<"c">(vars.child));
});

// The same closure, one stratified program under the fixpoint driver
// (?root seeds the predicate; the output is the finished set's own
// identity projection — an idb atom is a positive occurrence, so it
// grounds its variables and no re-grounding join exists).
inline constexpr auto Reach = bdb::program(
    Closure,
    bdb::rec<"reach">(
        [](auto r) consteval {
	        auto vars = r.vars(Node);
	        return r.match(Node, {.id = vars.id}).where(bdb::eq(vars.id, bdb::param<"root">())).find({}, bdb::as<"c">(vars.id));
        },
        [](auto r) consteval {
	        auto vars = r.vars(Parent);
	        return r.match(Parent, {.child = vars.child, .parent = vars.parent})
	            .idb(bdb::pred<"reach">, bdb::bind<"c">(vars.parent))
	            .find({}, bdb::as<"c">(vars.child));
        }),
    bdb::output([](auto r) consteval {
	    auto vars = r.vars(Node);
	    return r.idb(bdb::pred<"reach">, bdb::bind<"c">(vars.id)).find({}, bdb::as<"c">(vars.id));
    }));

// The EDB anti-join (recipe 3's negation shape on this schema): the
// nodes NO Parent row claims as a child — a negated atom binds nothing,
// only rejects.
inline constexpr auto Roots = bdb::query(Closure).rule([](auto r) consteval {
	auto node = r.vars(Node);
	return r.match(Node, {.id = node.id}).not_match(Parent, {.child = node.id}).find({}, bdb::as<"c">(node.id));
});

// The complement — negation OF the finished stratum.
inline constexpr auto Unreached = bdb::program(
    Closure,
    bdb::rec<"reach">(
        [](auto r) consteval {
	        auto vars = r.vars(Node);
	        return r.match(Node, {.id = vars.id}).where(bdb::eq(vars.id, bdb::param<"root">())).find({}, bdb::as<"c">(vars.id));
        },
        [](auto r) consteval {
	        auto vars = r.vars(Parent);
	        return r.match(Parent, {.child = vars.child, .parent = vars.parent})
	            .idb(bdb::pred<"reach">, bdb::bind<"c">(vars.parent))
	            .find({}, bdb::as<"c">(vars.child));
        }),
    bdb::output([](auto r) consteval {
	    auto vars = r.vars(Node);
	    return r.match(Node, {.id = vars.id}).not_idb(bdb::pred<"reach">, bdb::bind<"c">(vars.id)).find({}, bdb::as<"c">(vars.id));
    }));

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

/// The golden of one recipe: the fixtures file is one `rNN <64-hex>` line
/// per recipe (ts/test/cookbook.test.ts reads the same file).
auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
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

auto slurp(std::string_view path) -> std::optional<std::string> {
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

auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r24-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t root;
	std::uint64_t mid;
	std::uint64_t leaf;
	std::uint64_t lone;
};

/// The three-level forest plus one unreachable node (the alloc + insert
/// spelling — the dialect's pinned mint-on-insert form).
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const mint = [&](std::uint64_t& out, std::string name) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Node.id).and_then([&](std::uint64_t minted) -> std::expected<bool, bdb::Error> {
				out = minted;
				return tx.insert(Node, NodeRow{.id = minted, .name = std::move(name)});
			});
		};
		auto rows_land = mint(ids.root, std::string{"root"})
		                     .and_then([&](bool) {
			                     return mint(ids.mid, std::string{"mid"});
		                     })
		                     .and_then([&](bool) {
			                     return mint(ids.leaf, std::string{"leaf"});
		                     })
		                     .and_then([&](bool) {
			                     return mint(ids.lone, std::string{"lone"});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Parent, ParentRow{.child = ids.mid, .parent = ids.root});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Parent, ParentRow{.child = ids.leaf, .parent = ids.mid});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(ids);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

template<auto Query>
auto answer_set(bdb::Answers<Query> const& answers) -> std::vector<std::uint64_t> {
	auto out = std::vector<std::uint64_t>{};
	for (auto const& row : answers.rows()) {
		out.push_back(row.c);
	}
	std::ranges::sort(out);
	return out;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r24") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r24 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r24 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Closure);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits Closure", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r24 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto step = db->prepare<Step>();
	auto reach = db->prepare<Reach>();
	auto unreached = db->prepare<Unreached>();
	auto roots = db->prepare<Roots>();
	results.push_back(CaseResult{
	    .name = "step / reach / unreached / roots all prepare through the "
	            "engine validator (∈-set, recursion, stratum negation, "
	            "EDB anti-join)",
	    .passed = step.has_value() && reach.has_value() && unreached.has_value() && roots.has_value(),
	});
	if (!step.has_value() || !reach.has_value() || !unreached.has_value() || !roots.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "the three-level forest lands (alloc + insert)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// The host loop — the frontier discipline IS semi-naive evaluation's
	// Δ: probe the prepared ∈-set query with each round's fresh set.
	auto seen = std::vector<std::uint64_t>{ids->root};
	auto frontier = std::vector<std::uint64_t>{ids->root};
	auto loop_sound = true;
	while (!frontier.empty()) {
		auto next = db->execute(*step, {.frontier = std::span<std::uint64_t const>{frontier}});
		if (!next.has_value()) {
			loop_sound = false;
			break;
		}
		auto fresh = std::vector<std::uint64_t>{};
		for (auto const& row : next->rows()) {
			if (!std::ranges::contains(seen, row.c)) {
				fresh.push_back(row.c);
			}
		}
		for (auto const c : fresh) {
			seen.push_back(c);
		}
		frontier = std::move(fresh);
	}
	std::ranges::sort(seen);

	// The native program answers the same closure, root for root.
	auto native = db->execute(*reach, {.root = ids->root}).transform([](bdb::Answers<Reach> answers) {
		return answer_set<Reach>(answers);
	});
	auto expected = std::vector<std::uint64_t>{ids->root, ids->mid, ids->leaf};
	std::ranges::sort(expected);
	results.push_back(CaseResult{
	    .name = "the two dialects agree, root for root",
	    .passed = loop_sound && native.has_value() && *native == seen && seen == expected,
	});

	// The complement lands in-plan: every node the closure never
	// reached, judged against the full scan.
	auto complement = db->execute(*unreached, {.root = ids->root}).transform([](bdb::Answers<Unreached> answers) {
		return answer_set<Unreached>(answers);
	});
	auto everyone = std::vector<std::uint64_t>{};
	auto scanned = db->scan(Node);
	if (scanned.has_value()) {
		for (auto row = std::size_t{0}; row != scanned->len(); ++row) {
			auto const cell = scanned->cell({.row = row, .column = 0});
			if (cell.has_value() && std::holds_alternative<std::uint64_t>(*cell)) {
				everyone.push_back(std::get<std::uint64_t>(*cell));
			}
		}
	}
	auto outside = std::vector<std::uint64_t>{};
	for (auto const id : everyone) {
		if (!std::ranges::contains(seen, id)) {
			outside.push_back(id);
		}
	}
	std::ranges::sort(outside);
	results.push_back(CaseResult{
	    .name = "negation of the finished stratum answers the complement "
	            "(scan minus closure)",
	    .passed =
	        complement.has_value() && scanned.has_value() && *complement == outside && *complement == std::vector<std::uint64_t>{ids->lone},
	});

	// The EDB anti-join: the nodes no Parent row claims as a child —
	// root and lone (mid and leaf are somebody's children).
	auto parentless = db->execute(*roots, {}).transform([](bdb::Answers<Roots> answers) {
		return answer_set<Roots>(answers);
	});
	auto expected_roots = std::vector<std::uint64_t>{ids->root, ids->lone};
	std::ranges::sort(expected_roots);
	results.push_back(CaseResult{
	    .name = "the EDB anti-join answers the parentless nodes",
	    .passed = parentless.has_value() && *parentless == expected_roots,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r24_closure <fixtures-file>");
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
