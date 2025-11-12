#!/bin/bash
# Script to remove large target files from git history
# This MUST be run before you can push to GitHub

set -e

echo "=========================================="
echo "Removing large files from git history"
echo "=========================================="
echo ""

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not in a git repository"
    exit 1
fi

echo "Step 1: Removing target directories from all commits..."
git filter-branch --force --index-filter '
    git rm -r --cached --ignore-unmatch \
        target/ \
        crates/runtime/target/ \
        crates/primitives/target/ \
        Cargo.lock \
        2>/dev/null || true
' --prune-empty --tag-name-filter cat -- --all

echo ""
echo "Step 2: Cleaning up backup refs..."
git for-each-ref --format="%(refname)" refs/original/ | xargs -n 1 git update-ref -d 2>/dev/null || true

echo ""
echo "Step 3: Expiring reflog..."
git reflog expire --expire=now --all

echo ""
echo "Step 4: Running garbage collection..."
git gc --prune=now --aggressive

echo ""
echo "=========================================="
echo "Cleanup complete!"
echo "=========================================="
echo ""
echo "Verify with: git log --all --full-history --name-only | grep -E 'target/|Cargo.lock'"
echo "(Should return nothing)"
echo ""
echo "IMPORTANT: Now force push to remote:"
echo "  git push --force --all"
echo "  git push --force --tags"
echo ""
echo "WARNING: This rewrites history. Coordinate with your team first!"

