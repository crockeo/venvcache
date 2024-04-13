#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset
set -o xtrace

# Used inside of our Dockerfile to build and install sqlite3.
# We do this rather than downloading a pre-compiled binary,
# because we want to be able to link against MUSL.
output_file="${1:-}"
if [[ "$output_file" == "" ]]; then
    echo >&2 "Usage: $0 <output_file>"
    exit 1
fi

if [[ "${CC:-}" == "" ]]; then
    echo >&2 "WARN: Assuming `gcc` because no CC was provided."
    CC="gcc"
fi

sqlite_amalgamation="sqlite-amalgamation-3450200"
url="https://www.sqlite.org/2024/$sqlite_amalgamation.zip"
expected_sha="65230414820d43a6d1445d1d98cfe57e8eb9f7ac0d6a96ad6932e0647cce51db"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

curl \
    --location \
    --output "$tmpdir/sqlite.zip" \
    "$url"
echo "$expected_sha $tmpdir/sqlite.zip" | sha256sum --check

unzip \
    -d "$tmpdir" \
    "$tmpdir/sqlite.zip"

pushd "$tmpdir/$sqlite_amalgamation"
"$CC" \
    -shared \
    -o libsqlite3.so \
    sqlite3.c
popd
cp "$tmpdir/$sqlite_amalgamation/libsqlite3.so" "$output_file"
