module bumbledb.version;

import std;

namespace bdb {

auto version() -> std::string_view
{
    return "0.0.0-dev";
}

} // namespace bdb
