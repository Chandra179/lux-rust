#!/usr/bin/env bash
set -euo pipefail

# Fixed collation so [a-z] and [[:alnum:]] mean ASCII, whatever the user's locale.
export LC_ALL=C

usage() {
	echo "Usage: $0 <new-name>"
	echo "Example: $0 acme"
	echo ""
	echo "Renames the project's Lux references to <new-name>, a lowercase name"
	echo "matching [a-z][a-z0-9_-]*. From acme it derives:"
	echo "  acme-server   crate and binary (also acme-web and acme.db)"
	echo "  acme_server   library name"
	echo "  Acme          title in the README and web UI"
	echo "Run it from the repository root. Needs GNU sed (on macOS: brew install"
	echo "gnu-sed, then put its gnubin directory first in PATH)."
	exit 1
}

die() {
	echo "$*" >&2
	exit 1
}

warn() {
	echo "  warning: $*" >&2
}

if [ $# -ne 1 ]; then
	usage
fi

new="$1"
case "$new" in
-h | --help) usage ;;
esac

# [a-z][a-z0-9_-]*, and no trailing separator (it would give 'acme--server').
case "$new" in
[!a-z]* | *[!a-z0-9_-]* | *[-_])
	die "Invalid project name '$new': use lowercase letters, digits, '-' and '_', starting with a letter and not ending with '-' or '_' (for example: acme, my-app)."
	;;
esac

if [ ! -f Cargo.toml ] || [ ! -f server/Cargo.toml ]; then
	die "Cargo.toml or server/Cargo.toml not found; run this script from the repository root."
fi

if [[ "$(sed --version 2>/dev/null || true)" != *"GNU sed"* ]]; then
	die "GNU sed is required (on macOS: brew install gnu-sed, then put its gnubin directory first in PATH)."
fi

# title_case turns 'my-app' or 'my_app' into 'My App'.
title_case() {
	local word out=""
	for word in $(tr '_-' '  ' <<<"$1"); do
		out+="${out:+ }$(tr '[:lower:]' '[:upper:]' <<<"${word:0:1}")${word:1}"
	done
	printf '%s' "$out"
}

# Current names, read from the files that define them.
old_kebab="$(sed -n '/^\[package\]/,/^\[/s/^name = "\(.*\)-server"$/\1/p' server/Cargo.toml | head -n 1)"
old_snake="$(sed -n '/^\[lib\]/,/^\[/s/^name = "\(.*\)_server"$/\1/p' server/Cargo.toml | head -n 1)"
[ -n "$old_kebab" ] || die "Could not read the current project name from the [package] name in server/Cargo.toml (expected <name>-server)."
[ -n "$old_snake" ] || old_snake="${old_kebab//-/_}"
old_title="$(title_case "$old_kebab")"
for name in "$old_kebab" "$old_snake" "$old_title"; do
	if [[ ! "$name" =~ ^[A-Za-z0-9\ _-]+$ ]]; then
		die "Unexpected current project name '$name'; rename it by hand."
	fi
done

new_kebab="${new//_/-}"
new_snake="${new//-/_}"
new_title="$(title_case "$new")"

if [ "$old_kebab" = "$new_kebab" ] && [ "$old_title" = "$new_title" ]; then
	echo "Old and new project names are identical. Nothing to do."
	exit 0
fi

echo "Renaming project '$old_kebab' -> '$new_kebab' (crate ${old_kebab}-server -> ${new_kebab}-server, library ${old_snake}_server -> ${new_snake}_server, title '$old_title' -> '$new_title')"

# A name is a token when it is not glued to other name characters, so
# 'lux-server' is not found inside 'my-lux-server'.
tok='[:alnum:]_-'

# token_script <old> <new> prints the sed commands that replace <old> with <new>
# wherever it stands alone as a token. Adjacent matches share their one
# separator character, so matches are parked under a placeholder in two passes
# and written out last; that also keeps <new> from being rescanned when it
# contains <old> (acme -> acme-corp).
token_script() {
	local re="${1//./\\.}" pass
	for pass in 1 2; do
		printf 's/(^|[^%s])%s($|[^%s])/\\1\\x01\\2/g\n' "$tok" "$re" "$tok"
	done
	printf 's/\\x01/%s/g\n' "$2"
}

# Every project-name form that carries the name inside text files, most
# specific first.
needles=("${old_kebab}-server" "${old_snake}_server" "${old_kebab}-web" "${old_kebab}.db" "$old_title")
script="$(
	token_script "${old_kebab}-server" "${new_kebab}-server"
	token_script "${old_snake}_server" "${new_snake}_server"
	token_script "${old_kebab}-web" "${new_kebab}-web"
	token_script "${old_kebab}.db" "${new_kebab}.db"
	token_script "$old_title" "$new_title"
)"

# find_text <find-action...> runs find over the repository's own text files:
# everything except build output, dependencies, Git metadata, the lockfiles
# (handled below), and this script.
find_text() {
	find . \( -path ./.git -o -path ./target -o -path ./web/node_modules -o -path ./web/dist \) -prune -o \
		-type f ! -name Cargo.lock ! -name package-lock.json ! -path ./scripts/rename-project.sh "$@"
}

# 1. Rewrite the name in every text file that carries it: Cargo manifest, Rust
#    sources and tests, config, Makefile, Containerfile, web sources, docs.
grep_args=()
for needle in "${needles[@]}"; do
	grep_args+=(-e "$needle")
done
changed="$(find_text -exec grep -qIF "${grep_args[@]}" {} \; -print -exec sed -E -i -e "$script" {} +)"
count=0
if [ -n "$changed" ]; then
	count="$(printf '%s\n' "$changed" | wc -l | tr -d ' ')"
fi
echo "  updated $count text files (crate, library, web package, datastore names, titles)"

# 2. The Containerfile's service user and group are the bare project name.
if [ -f Containerfile ]; then
	sed -E -i -e "$(token_script "$old_kebab" "$new_kebab")" Containerfile
	echo "  updated Containerfile user and group"
fi

# 3. The lockfiles are not scanned above; rename only their package entries.
if [ -f Cargo.lock ]; then
	sed -E -i -e "$(token_script "${old_kebab}-server" "${new_kebab}-server")" Cargo.lock
	echo "  updated Cargo.lock"
fi
if [ -f web/package-lock.json ]; then
	sed -E -i -e "$(token_script "${old_kebab}-web" "${new_kebab}-web")" web/package-lock.json
	echo "  updated web/package-lock.json"
fi

# 4. Let cargo put the renamed package back in Cargo.lock's sorted position
#    (so 'cargo build --locked' passes) and refit imports and line widths that
#    the new name changed.
if command -v cargo >/dev/null 2>&1; then
	if out="$(cargo update --workspace --offline 2>&1)"; then
		echo "  refreshed Cargo.lock with cargo update --workspace"
	else
		warn "cargo update --workspace --offline failed; run 'cargo update --workspace' yourself."
		printf '%s\n' "$out" >&2
	fi
	if out="$(cargo fmt --all 2>&1)"; then
		echo "  formatted Rust sources with cargo fmt"
	else
		warn "cargo fmt --all failed; run it yourself."
		printf '%s\n' "$out" >&2
	fi
else
	warn "cargo not found; run 'cargo update --workspace' and 'cargo fmt --all' yourself."
fi

# 5. Report what still mentions the old name so it can be reviewed by hand.
#    Skipped when the new name contains the old one (acme -> acme-corp), which
#    would flag every renamed line.
shopt -s nocasematch
if [[ "$new_kebab $new_snake $new_title" == *"$old_kebab"* ]]; then
	shopt -u nocasematch
	echo "  skipped the check for leftover '$old_kebab' references (the new name contains it)"
else
	shopt -u nocasematch
	left="$(find_text -exec grep -nIiE -e "(^|[^[:alnum:]])(${old_kebab}|${old_snake})([^[:alnum:]]|\$)" /dev/null {} + || true)"
	if [ -n "$left" ]; then
		warn "these lines still mention '$old_kebab'; review them by hand:"
		printf '%s\n' "$left" | sed 's|^\./|    |' >&2
	else
		echo "  no references to '$old_kebab' left"
	fi
fi

echo "Done. Review 'git diff', then run 'cargo test --workspace' and 'npm --prefix web ci && npm --prefix web run typecheck' to verify the renamed project."
