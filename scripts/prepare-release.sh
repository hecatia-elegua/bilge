#!/usr/bin/env bash
# Bump workspace versions and check CHANGELOG.md before a release.
# Intended for Linux/GitHub Actions; uses only POSIX awk + bash (no GNU sed -i).
#
# For 0.x bumps, minor versions are for breaking changes: `0.3.0 -> 0.4.0`.
# For 1.x bumps, it works as usual with semantic versioning.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

usage() {
    echo "usage: $0 <patch|minor|major|--set X.Y.Z>" >&2
    exit 2
}

is_xyz() {
    [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
}

current_version() {
    awk '
        $0 == "[workspace.package]" { in_ws = 1; next }
        in_ws && /^\[/ { exit }
        in_ws && $1 == "version" && $2 == "=" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' Cargo.toml
}

bump_version() {
    local current="$1" kind="$2"
    local major minor patch
    IFS=. read -r major minor patch <<<"$current"
    case "$kind" in
        patch) patch=$((10#$patch + 1)) ;;
        minor)
            minor=$((10#$minor + 1))
            patch=0
            ;;
        major)
            major=$((10#$major + 1))
            minor=0
            patch=0
            ;;
        *) usage ;;
    esac
    echo "${major}.${minor}.${patch}"
}

# Literal substring checks so '.' in 1.2.3 is not a regex wildcard.
changelog_heading_ok() {
    local version="$1"
    awk -v v="$version" '
        BEGIN { want = "## [" v "]" }
        {
            line = $0
            sub(/\r$/, "", line)
        }
        index(line, want) == 1 {
            rest = substr(line, length(want) + 1)
            if (rest == " - Unreleased" || rest ~ /^ - [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/) {
                found_heading = 1
            }
        }
        END { exit found_heading ? 0 : 1 }
    ' CHANGELOG.md
}

changelog_has_bullets() {
    local version="$1"
    awk -v v="$version" '
        BEGIN { want = "## [" v "]" }
        {
            line = $0
            sub(/\r$/, "", line)
        }
        index(line, want) == 1 { p = 1; next }
        p && index(line, "## ") == 1 { exit found ? 0 : 1 }
        p && index(line, "- ") == 1 { found = 1 }
        END { exit found ? 0 : 1 }
    ' CHANGELOG.md
}

check_changelog() {
    local version="$1"
    if ! changelog_heading_ok "$version"; then
        echo "CHANGELOG.md must contain '## [${version}] - Unreleased' (or that heading already dated YYYY-MM-DD)" >&2
        exit 1
    fi
    if ! changelog_has_bullets "$version"; then
        echo "CHANGELOG.md heading ## [${version}] has no bullet entries" >&2
        exit 1
    fi
}

rewrite_changelog() {
    local version="$1" today="$2"
    local tmp
    tmp="$(mktemp)"
    awk -v v="$version" -v today="$today" '
        BEGIN {
            want = "## [" v "]"
            dated = want " - " today
        }
        {
            line = $0
            sub(/\r$/, "", line)
            if (NR == 1) sub(/^\357\273\277/, "", line)
        }
        NR == 1 && line != "# Changelog" {
            print "expected CHANGELOG.md to start with \"# Changelog\"" > "/dev/stderr"
            exit 1
        }
        { lines[NR] = line; n = NR }
        END {
            i = 2
            while (i <= n && lines[i] == "") i++
            first_h = (i <= n) ? lines[i] : ""
            if (first_h != "## [Unreleased]") {
                print "# Changelog"
                print ""
                print "## [Unreleased]"
                print ""
                start = (lines[1] == "# Changelog") ? 2 : 1
                while (start <= n && lines[start] == "") start++
            } else {
                start = 1
            }
            for (j = start; j <= n; j++) {
                line = lines[j]
                if (index(line, want) == 1) {
                    rest = substr(line, length(want) + 1)
                    if (rest == " - Unreleased") line = dated
                    dated_ok = 1
                }
                print line
            }
            if (!dated_ok) {
                print "failed to date changelog heading for " v > "/dev/stderr"
                exit 1
            }
        }
    ' CHANGELOG.md >"$tmp"
    mv "$tmp" CHANGELOG.md
}

apply_version() {
    local current="$1" new="$2"
    local tmp
    tmp="$(mktemp)"
    awk -v old="$current" -v new="$new" '
        $0 == "[workspace.package]" { in_ws = 1 }
        in_ws && /^\[/ && $0 != "[workspace.package]" { in_ws = 0 }
        in_ws && $1 == "version" && $2 == "=" {
            got = $3
            gsub(/"/, "", got)
            if (got != old) {
                print "workspace version is " got ", expected " old > "/dev/stderr"
                exit 1
            }
            print "version = \"" new "\""
            ws_done = 1
            next
        }
        {
            prefix = "bilge-impl = { version = \"=" old "\""
            if (index($0, prefix) == 1) {
                sub("version = \"=" old "\"", "version = \"=" new "\"")
                pin_done = 1
            }
            print
        }
        END {
            if (!ws_done) {
                print "did not find version in [workspace.package]" > "/dev/stderr"
                exit 1
            }
            if (!pin_done) {
                print "did not find bilge-impl = { version = \"=" old "\" ... }" > "/dev/stderr"
                exit 1
            }
        }
    ' Cargo.toml >"$tmp"
    mv "$tmp" Cargo.toml
}

extract_notes() {
    local version="$1"
    awk -v v="$version" '
        BEGIN { want = "## [" v "]" }
        {
            line = $0
            sub(/\r$/, "", line)
        }
        index(line, want) == 1 { p = 1; next }
        p && index(line, "## ") == 1 { exit }
        p { print line }
    ' CHANGELOG.md | sed -e '1{/^$/d;}'
}

assert_applied() {
    local new="$1"
    local got
    got="$(current_version)"
    if [[ "$got" != "$new" ]]; then
        echo "workspace version is ${got}, expected ${new}" >&2
        exit 1
    fi
    if ! awk -v new="$new" '
        index($0, "bilge-impl = { version = \"=" new "\"") == 1 { found = 1 }
        END { exit found ? 0 : 1 }
    ' Cargo.toml; then
        echo "bilge-impl pin is not =${new}" >&2
        exit 1
    fi
    if ! awk -v v="$new" '
        BEGIN { want = "## [" v "] - " }
        {
            line = $0
            sub(/\r$/, "", line)
        }
        index(line, want) == 1 && substr(line, length(want) + 1) ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ {
            ok = 1
        }
        END { exit ok ? 0 : 1 }
    ' CHANGELOG.md; then
        echo "CHANGELOG.md heading for ${new} is not dated" >&2
        exit 1
    fi
}

[[ $# -ge 1 ]] || usage

current="$(current_version)"
if [[ -z "$current" ]]; then
    echo "could not read [workspace.package] version from Cargo.toml" >&2
    exit 1
fi
if ! is_xyz "$current"; then
    echo "invalid current version: ${current} (need X.Y.Z)" >&2
    exit 1
fi

if [[ "$1" == "--set" ]]; then
    [[ $# -eq 2 ]] || usage
    new="$2"
else
    [[ $# -eq 1 ]] || usage
    new="$(bump_version "$current" "$1")"
fi
if ! is_xyz "$new"; then
    echo "invalid version: ${new} (need X.Y.Z)" >&2
    exit 2
fi

check_changelog "$new"

# UTC so CI and local runs produce the same date; not the machine timezone.
today="$(date -u +%F)"
rewrite_changelog "$new" "$today"
apply_version "$current" "$new"
assert_applied "$new"

if [[ -d .git ]] && command -v git >/dev/null; then
    git --no-pager diff --check -- Cargo.toml CHANGELOG.md
    git --no-pager diff -- Cargo.toml CHANGELOG.md
fi

echo "version=${new}"
mkdir -p target
NOTES="$ROOT/target/release-notes.md"
extract_notes "$new" >"$NOTES"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "version=${new}" >>"$GITHUB_OUTPUT"
    echo "notes_file=${NOTES}" >>"$GITHUB_OUTPUT"
fi
