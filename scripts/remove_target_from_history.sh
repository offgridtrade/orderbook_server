#!/bin/bash
# Script to remove target directories from git history
# WARNING: This rewrites git history. Coordinate with your team before running.

set -e

echo "Removing target directories from git history..."
echo "This may take a while depending on repository size."

# Remove all target directories from all branches and tags
git filter-branch --force --index-filter '
    git rm -rf --cached --ignore-unmatch \
        target/ \
        crates/*/target/ \
        crates/*/*/target/ \
        2>/dev/null || true
' --prune-empty --tag-name-filter cat -- --all

# Clean up backup refs
git for-each-ref --format="%(refname)" refs/original/ | xargs -n 1 git update-ref -d 2>/dev/null || true

# Force garbage collection
git reflog expire --expire=now --all
git gc --prune=now --aggressive

echo "Done! Target directories have been removed from git history."
echo ""
echo "IMPORTANT: If you've already pushed to remote, you'll need to force push:"
echo "  git push --force --all"
echo "  git push --force --tags"
echo ""
echo "WARNING: Coordinate with your team before force pushing!"

