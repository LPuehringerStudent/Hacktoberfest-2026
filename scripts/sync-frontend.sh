#!/usr/bin/env bash
# Mirror the Lovable repo into frontend/ (one-way: Lovable -> this repo).
# Usage: scripts/sync-frontend.sh <lovable-repo-url> [branch]
#   e.g. scripts/sync-frontend.sh https://github.com/<owner>/<lovable-repo>.git
# Run it on a branch, then open a PR as usual.
set -euo pipefail

url="${1:?usage: $0 <lovable-repo-url> [branch]}"
branch="${2:-main}"
cd "$(git rev-parse --show-toplevel)"

if [ "$(git branch --show-current)" = "main" ]; then
  echo "On main — switch to a branch first: git switch -c <you>/sync-frontend" >&2
  exit 1
fi

if [ -d frontend ] && [ -n "$(git log --grep='git-subtree-dir: frontend' -1 --format=%H)" ]; then
  git subtree pull --prefix=frontend "$url" "$branch" --squash -m "chore(frontend): sync from Lovable"
else
  # First sync: drop the placeholder README so subtree add has an empty prefix
  if [ -e frontend ]; then
    git rm -r -q frontend
    git commit -q -m "chore(frontend): remove placeholder before first sync"
  fi
  git subtree add --prefix=frontend "$url" "$branch" --squash -m "chore(frontend): import from Lovable"
fi
