#!/usr/bin/env bash
# Start .github/workflows/prepare-ios-release.yml on the current release branch and wait for
# it. The workflow builds the XCFramework, attaches it to the draft release vX.Y.Z and pushes
# a commit that sets its URL and checksum in the root Package.swift. This script builds
# nothing. When the run passes, it pulls that commit.
#
# Usage: scripts/prepare-ios-release.sh X.Y.Z
# Needs: gh, signed in with write access; the release branch pushed, with no local changes.
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/prepare-ios-release.sh X.Y.Z}"
have="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ "$version" = "$have" ] || {
  echo "error: requested version $version does not match Cargo.toml version $have" >&2
  exit 1
}

branch="$(git symbolic-ref --short HEAD)"
[ "$branch" != main ] || { echo "error: run this on the release branch, not on main" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || {
  echo "error: commit or discard the local changes first" >&2
  exit 1
}
git fetch -q origin "$branch"
[ "$(git rev-parse HEAD)" = "$(git rev-parse FETCH_HEAD)" ] || {
  echo "error: $branch differs from origin/$branch; push or pull first" >&2
  exit 1
}

# The newest run of this workflow on this branch. Run IDs only grow, so the run this script
# starts is the first one with a larger ID.
newest() {
  gh run list --workflow prepare-ios-release.yml --branch "$branch" --event workflow_dispatch \
    --limit 1 --json databaseId --jq '.[0].databaseId // 0'
}
before="$(newest)"
gh workflow run prepare-ios-release.yml --ref "$branch" -f version="$version"
run=""
for _ in $(seq 30); do
  sleep 2
  run="$(newest)"
  [ "$run" -gt "$before" ] && break
  run=""
done
[ -n "$run" ] || { echo "error: the workflow run did not start; see gh run list" >&2; exit 1; }

# Pull also after a failed run: a run that fails after its push has still moved the branch.
status=0
gh run watch "$run" --exit-status || status=$?
git pull -q --ff-only origin "$branch"
[ "$status" = 0 ] || exit "$status"
sed -n 's/.*\(releases\/download\/[^"]*\)".*/\1/p; s/.*checksum: "\([0-9a-f]*\)".*/checksum \1/p' Package.swift
