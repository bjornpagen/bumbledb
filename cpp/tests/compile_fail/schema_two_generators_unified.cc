import std;
import bumbledb;

struct RepoRow {
	[[= bdb::fresh]] std::uint64_t id;
};

struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;
};

inline constexpr auto Repo = bdb::relation<"Repo", RepoRow>;
inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;

inline constexpr auto Broken = bdb::schema<"Broken">(Repo, Service,

                                                     bdb::contained(bdb::on(Repo.id), bdb::on(Service.id)));
