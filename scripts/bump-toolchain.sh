#!/usr/bin/env bash
# Moves rust-toolchain.toml to the newest nightly that ships every component CI
# uses, repairs the tree for it, runs the lint and test lanes, and commits.
#
#   scripts/bump-toolchain.sh [nightly-YYYY-MM-DD]
#
# The tree must be clean. The old-vs-new micro report lands in
# target/toolchain-bump/report.md and never blocks the bump.
set -euo pipefail
cd "$(dirname "$0")/.."

hosts="aarch64-apple-darwin aarch64-unknown-linux-gnu aarch64-unknown-linux-musl x86_64-unknown-linux-gnu"
miri_hosts="aarch64-apple-darwin x86_64-unknown-linux-gnu"
report_dir=target/toolchain-bump
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# One "<package> <target>" line per component a qualifying nightly must ship.
required() {
	local host package
	for host in $hosts; do
		for package in rustc cargo rust-std rustfmt-preview clippy-preview; do
			echo "$package $host"
		done
	done
	for host in $miri_hosts; do
		echo "miri-preview $host"
	done
	echo 'rust-src *'
}

# Reads a channel manifest; prints "<package> <target>" for each available component.
available() {
	awk '
		/^\[pkg\.[^.]+\.target\..*\]$/ {
			split($0, part, ".")
			package = part[2]
			target = substr($0, length("[pkg." package ".target.") + 1)
			sub(/\]$/, "", target)
			gsub(/"/, "", target)
			next
		}
		/^\[/ { package = ""; next }
		package != "" && $0 == "available = true" { print package, target }
	'
}

# Succeeds when the manifest file $1 ships every required component.
qualifies() {
	[ -z "$(comm -23 <(required | sort -u) <(available < "$1" | sort -u))" ]
}

fetch_manifest() {
	curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "https://static.rust-lang.org/dist/$1/channel-rust-nightly.toml"
}

days_ago() {
	date -u -d "-$1 day" +%F 2>/dev/null || date -u -v-"$1"d +%F
}

newest_qualifying() {
	local back day
	for back in $(seq 0 30); do
		day=$(days_ago "$back")
		if fetch_manifest "$day" "$work/manifest" 2>/dev/null && qualifies "$work/manifest"; then
			echo "nightly-$day"
			return 0
		fi
	done
	echo "bump-toolchain: no qualifying nightly in the last 31 days" >&2
	return 1
}

# Non-blocking micro measurement with the active toolchain into $1.
micro() {
	cargo run --locked --profile gate -p bumbledb-bench -- micro --levels all --out "$1" ||
		echo "bump-toolchain: micro run failed; the report will lack it" >&2
}

# Deletes the #[expect] lints the compiler reports unfulfilled in either
# feature configuration, and the attribute once no lint is left in it.
remove_unfulfilled_expectations() {
	cargo clippy --locked --workspace --all-targets --message-format=json > "$work/default.jsonl"
	cargo clippy --locked --workspace --all-targets --all-features --message-format=json > "$work/all.jsonl"
	node - "$work/default.jsonl" "$work/all.jsonl" <<'JS'
const fs = require("node:fs")

const tokens = new Map()
for (const log of process.argv.slice(2)) {
	for (const line of fs.readFileSync(log, "utf8").split("\n")) {
		if (!line.startsWith("{")) continue
		const message = JSON.parse(line)
		if (message.reason !== "compiler-message") continue
		if (message.message.code?.code !== "unfulfilled_lint_expectations") continue
		for (const span of message.message.spans.filter((span) => span.is_primary)) {
			const file = tokens.get(span.file_name) ?? new Map()
			tokens.set(span.file_name, file)
			file.set(span.byte_start, span.byte_end)
		}
	}
}

const skipString = (text, i) => {
	for (i++; text[i] !== '"'; i++) if (text[i] === "\\") i++
	return i
}

// Index just past the bracket or parenthesis that closes the one at `open`.
const close = (text, open) => {
	let depth = 0
	for (let i = open; i < text.length; i++) {
		const c = text[i]
		if (c === '"') i = skipString(text, i)
		else if (c === "[" || c === "(") depth++
		else if ((c === "]" || c === ")") && --depth === 0) return i + 1
	}
	throw Error(`unterminated attribute at byte ${open}`)
}

// Top-level items of the parenthesized list at `open`, as [start, end) ranges.
const items = (text, open) => {
	const end = close(text, open) - 1
	const list = []
	let start = open + 1
	let depth = 0
	for (let i = open + 1; i < end; i++) {
		const c = text[i]
		if (c === '"') i = skipString(text, i)
		else if (c === "(" || c === "[") depth++
		else if (c === ")" || c === "]") depth--
		else if (c === "," && depth === 0) {
			list.push([start, i])
			start = i + 1
		}
	}
	list.push([start, end])
	return { end, list: list.filter(([a, b]) => text.slice(a, b).trim() !== "") }
}

const attributeAround = (text, at) => {
	for (let i = at; i >= 0; i--) {
		if (text[i] !== "#") continue
		const bracket = text[i + 1] === "[" ? i + 1 : text.startsWith("![", i + 1) ? i + 2 : -1
		if (bracket < 0) continue
		const end = close(text, bracket)
		if (end > at) return { start: i, body: bracket + 1, end }
	}
	throw Error(`no attribute encloses byte ${at}`)
}

const removeAttribute = (text, attr) => {
	const lineStart = text.lastIndexOf("\n", attr.start - 1) + 1
	const newline = text.indexOf("\n", attr.end)
	const lineEnd = newline < 0 ? text.length : newline
	if (text.slice(lineStart, attr.start).trim() === "" && text.slice(attr.end, lineEnd).trim() === "") {
		return text.slice(0, lineStart) + text.slice(Math.min(lineEnd + 1, text.length))
	}
	return text.slice(0, attr.start) + text.slice(attr.end)
}

const joined = (text, list) => list.map(([a, b]) => text.slice(a, b).trim()).join(", ")

const rewrite = (text, attr, stale) => {
	let call = attr.body
	let open
	for (;;) {
		call = text.indexOf("expect", call)
		if (call < 0 || call >= attr.end) throw Error(`no expect(...) holds bytes ${stale}`)
		open = text.indexOf("(", call)
		const closed = open < 0 ? -1 : close(text, open)
		if (/^expect\s*\($/.test(text.slice(call, open + 1)) && stale.every(([s, e]) => s > open && e < closed)) break
		call++
	}
	const { end, list } = items(text, open)
	const kept = list.filter(([a, b]) => !stale.some(([s, e]) => s >= a && e <= b))
	if (kept.some(([a, b]) => !/^\s*reason\s*=/.test(text.slice(a, b)))) {
		return text.slice(0, open + 1) + joined(text, kept) + text.slice(end)
	}
	const inner = text.slice(attr.body, attr.end - 1)
	if (text.slice(attr.body, call).trim() === "" && text.slice(end + 1, attr.end - 1).trim() === "") {
		return removeAttribute(text, attr)
	}
	const cfg = /^\s*cfg_attr\s*\(/.exec(inner)
	if (cfg) {
		const outer = items(text, attr.body + cfg[0].length - 1)
		const rest = outer.list.filter(([a, b]) => !(a <= call && end < b))
		if (rest.length === outer.list.length) throw Error(`expect(...) at byte ${call} is not a cfg_attr item`)
		if (rest.length === 1) return removeAttribute(text, attr)
		return text.slice(0, attr.body + cfg[0].length) + joined(text, rest) + text.slice(outer.end)
	}
	throw Error(`unsupported attribute shape around byte ${call}`)
}

for (const [file, spans] of tokens) {
	let text = fs.readFileSync(file).toString("latin1")
	const byAttribute = new Map()
	for (const [start, end] of spans) {
		const attr = attributeAround(text, start)
		const group = byAttribute.get(attr.start) ?? { attr, stale: [] }
		group.stale.push([start, end])
		byAttribute.set(attr.start, group)
	}
	for (const { attr, stale } of [...byAttribute.values()].sort((x, y) => y.attr.start - x.attr.start)) {
		const line = text.slice(0, attr.start).split("\n").length
		console.log(`${file}:${line}: unfulfilled ${stale.map(([s, e]) => text.slice(s, e)).join(", ")}`)
		text = rewrite(text, attr, stale)
	}
	fs.writeFileSync(file, Buffer.from(text, "latin1"))
}
JS
}

if [ -n "$(git status --porcelain)" ]; then
	echo "bump-toolchain: the tree must be clean; the bump commits everything it changes" >&2
	exit 1
fi

current=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
case "${1:-}" in
"")
	next=$(newest_qualifying)
	;;
nightly-????-??-??)
	next=$1
	fetch_manifest "${next#nightly-}" "$work/manifest"
	qualifies "$work/manifest" || {
		echo "bump-toolchain: $next lacks a required component" >&2
		exit 1
	}
	;;
*)
	echo "usage: scripts/bump-toolchain.sh [nightly-YYYY-MM-DD]" >&2
	exit 2
	;;
esac
if [[ ! "$next" > "$current" ]]; then
	echo "bump-toolchain: $current is current (newest qualifying: $next)"
	exit 0
fi
echo "bump-toolchain: $current -> $next"

mkdir -p "$report_dir"
micro "$work/old.json"

sed "s/^channel = \".*\"$/channel = \"$next\"/" rust-toolchain.toml > "$work/toolchain"
cat "$work/toolchain" > rust-toolchain.toml
rustup toolchain install

cargo fmt --all
cargo clippy --locked --fix --allow-dirty --allow-staged --workspace --all-targets
cargo clippy --locked --fix --allow-dirty --allow-staged --workspace --all-targets --all-features
remove_unfulfilled_expectations
cargo fmt --all
# Staged, so the lanes' tree-clean check sees only what the lanes themselves change.
git add -A

scripts/ci.sh lint
scripts/ci.sh test

micro "$work/new.json"
{
	echo "# $current -> $next"
	echo
	if [ -f "$work/old.json" ] && [ -f "$work/new.json" ]; then
		cargo run --locked --profile gate -p bumbledb-bench -- micro --compare "$work/old.json" "$work/new.json" ||
			echo "The micro comparison failed."
	else
		echo "No micro comparison: a micro run failed."
	fi
	echo
	echo "Changed files:"
	echo
	git diff --cached --stat | sed 's/^/    /'
} > "$report_dir/report.md"

git commit -q -m "Toolchain: $next"
git log -1 --format='%h %s'
