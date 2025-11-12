#!/bin/bash
# Aggressive script to remove ALL target directories from git history
# This uses a tree-filter which is slower but more thorough

set -e

echo "Starting aggressive removal of target directories from git history..."
echo "This will rewrite ALL commits and may take a long time."

# First, let's find all unique target directory paths in the history
echo "Finding all target directories in git history..."
TARGET_PATHS=$(git log --all --full-history --name-only --pretty=format: | grep -E '(^|/)target/' | sort -u)

if [ -z "$TARGET_PATHS" ]; then
    echo "No target directories found in git history."
    exit 0
fi

echo "Found target directories. Removing from all commits..."

# Use tree-filter to remove target directories from each commit
git filter-branch --force --tree-filter '
    find . -type d -name target -exec rm -rf {} + 2>/dev/null || true
    find . -type f -path "*/target/*" -delete 2>/dev/null || true
' --prune-empty --tag-name-filter cat -- --all

echo "Cleaning up backup refs..."
git for-each-ref --format="%(refname)" refs/original/ | xargs -n 1 git update-ref -d 2>/dev/null || true

echo "Expiring reflog..."
git reflog expire --expire=now --all

echo "Running aggressive garbage collection..."
git gc --prune=now --aggressive

echo ""
echo "Done! All target directories have been removed from git history."
echo ""
echo "Verify with: git log --all --full-history --name-only | grep target"
echo ""
echo "IMPORTANT: Force push required:"
echo "  git push --force --all"
echo "  git push --force --tags"

