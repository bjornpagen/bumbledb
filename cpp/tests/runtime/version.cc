import std;
import bumbledb;

namespace {

struct CaseResult {
    std::string_view name;
    bool passed;
};

auto check_version_is_nonempty() -> CaseResult {
    return CaseResult{
        .name = "version() returns a nonempty string",
        .passed = !bdb::version().empty(),
    };
}

auto check_version_is_the_scaffold_seed() -> CaseResult {
    return CaseResult{
        .name = "version() returns the scaffold seed value",
        .passed = bdb::version() == std::string_view{"0.0.0-dev"},
    };
}

} // namespace

auto main() -> int {
    auto const results = std::array{
        check_version_is_nonempty(),
        check_version_is_the_scaffold_seed(),
    };

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
