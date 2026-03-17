#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

require_command() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "error: required command not found: $cmd" >&2
        exit 1
    fi
}

require_clean_tree() {
    if [[ -n "$(git status --short)" ]]; then
        echo "error: working tree is not clean" >&2
        echo "commit or stash changes before running a release" >&2
        exit 1
    fi
}

extract_current_version() {
    sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1
}

extract_file_version() {
    local file="$1"
    sed -n 's/^version = "\(.*\)"$/\1/p' "$file" | head -n 1
}

extract_changelog_versions() {
    sed -n 's/^## \[\([^]]*\)\] - .*$/\1/p' CHANGELOG.md
}

version_exists_in_changelog() {
    local version="$1"
    grep -Eq "^## \[$version\] - " CHANGELOG.md
}

create_changelog_stub() {
    local version="$1"

    perl -0pi -e 's/\A(# Changelog\n\nAll notable changes to this project are documented in this file\.\n\nThe format is based on \[Keep a Changelog\]\(https:\/\/keepachangelog\.com\/\),\nand this project adheres to \[Semantic Versioning\]\(https:\/\/semver\.org\/\)\.\n\n)/$1 . "## ['"$version"'] - Unreleased\n\n### Changed\n\n- Release preparation.\n\n"/se' CHANGELOG.md
}

prompt() {
    local label="$1"
    local default_value="${2:-}"
    local answer

    if [[ -n "$default_value" ]]; then
        read -r -p "$label [$default_value]: " answer
        printf '%s\n' "${answer:-$default_value}"
    else
        read -r -p "$label: " answer
        printf '%s\n' "$answer"
    fi
}

confirm() {
    local label="$1"
    local default_value="${2:-y}"
    local suffix="[y/N]"

    if [[ "$default_value" == "y" ]]; then
        suffix="[Y/n]"
    fi

    local answer
    read -r -p "$label $suffix " answer
    answer="${answer,,}"

    if [[ -z "$answer" ]]; then
        answer="$default_value"
    fi

    [[ "$answer" == "y" || "$answer" == "yes" ]]
}

update_version_file() {
    local file="$1"
    local old="$2"
    local new="$3"

    perl -0pi -e "s/version = \"\Q$old\E\"/version = \"$new\"/" "$file"
}

update_changelog_date() {
    local version="$1"
    local release_date="$2"

    perl -0pi -e "s/^## \[\Q$version\E\] - Unreleased$/## [$version] - $release_date/m" CHANGELOG.md
}

ensure_branch_state() {
    local branch
    branch="$(git branch --show-current)"
    if [[ "$branch" != "master" ]]; then
        echo "error: current branch is '$branch', expected 'master'" >&2
        exit 1
    fi

    git fetch origin master --tags

    local local_head remote_head
    local_head="$(git rev-parse HEAD)"
    remote_head="$(git rev-parse origin/master)"

    if [[ "$local_head" != "$remote_head" ]]; then
        echo "error: local master is not aligned with origin/master" >&2
        echo "local : $local_head" >&2
        echo "remote: $remote_head" >&2
        echo "pull or reset intentionally before releasing" >&2
        exit 1
    fi
}

require_command git
require_command perl

require_clean_tree
ensure_branch_state

workspace_version="$(extract_current_version)"
python_project_version="$(extract_file_version bindings/python/pyproject.toml)"
python_crate_version="$(extract_file_version bindings/python/Cargo.toml)"

if [[ "$workspace_version" != "$python_project_version" || "$workspace_version" != "$python_crate_version" ]]; then
    echo "error: version metadata is already inconsistent" >&2
    echo "workspace Cargo.toml        : $workspace_version" >&2
    echo "bindings/python/pyproject   : $python_project_version" >&2
    echo "bindings/python/Cargo.toml  : $python_crate_version" >&2
    echo "align them first, then rerun the release script" >&2
    exit 1
fi

current_version="$workspace_version"
default_version="$current_version"

echo "Current workspace version: $current_version"
echo "Branch: master"
echo "Remote: origin"
echo

release_version="$(prompt "Release version (without leading v)" "$default_version")"

if [[ ! "$release_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-((alpha|beta|rc)\.[0-9]+))?$ ]]; then
    echo "error: invalid version format: $release_version" >&2
    echo "expected X.Y.Z or X.Y.Z-{alpha|beta|rc}.N" >&2
    exit 1
fi

tag="v$release_version"

if git rev-parse "$tag" >/dev/null 2>&1; then
    echo "error: tag already exists: $tag" >&2
    exit 1
fi

if ! version_exists_in_changelog "$release_version"; then
    echo
    echo "No changelog entry exists for $release_version."
    echo
    if confirm "Create a minimal changelog stub for $release_version?" "y"; then
        create_changelog_stub "$release_version"
    else
        echo "Known changelog versions:"
        extract_changelog_versions | sed 's/^/  - /'
        echo
        echo "Update CHANGELOG.md first, then rerun this script." >&2
        exit 1
    fi
fi

echo
echo "This will:"
echo "  - update Cargo/Python version metadata to $release_version"
echo "  - create annotated tag $tag on origin/master"
echo "  - push the tag so .github/workflows/release.yml creates the GitHub Release"
echo

if ! confirm "Continue?" "y"; then
    echo "aborted"
    exit 0
fi

update_version_file Cargo.toml "$current_version" "$release_version"
update_version_file bindings/python/pyproject.toml "$current_version" "$release_version"
update_version_file bindings/python/Cargo.toml "$current_version" "$release_version"
update_changelog_date "$release_version" "$(date +%F)"

if ! git diff --quiet -- Cargo.toml bindings/python/pyproject.toml bindings/python/Cargo.toml CHANGELOG.md; then
    git add Cargo.toml bindings/python/pyproject.toml bindings/python/Cargo.toml CHANGELOG.md
    git commit -m "chore: bump version to $release_version"
fi

git tag -a "$tag" -m "Release $tag"
git push origin master
git push origin "$tag"

cat <<EOF

Release triggered.

Next:
  1. Watch the Actions run for $tag.
  2. Verify the release page after the workflow completes.

Workflow:
  https://github.com/TorstenCScholz/rtfkit/actions/workflows/release.yml
Release:
  https://github.com/TorstenCScholz/rtfkit/releases/tag/$tag
EOF
