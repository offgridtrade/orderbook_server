#!/bin/bash
# Script to remove target directories using BFG Repo-Cleaner (best method)
# Falls back to git filter-branch if BFG is not available

set -e

echo "Attempting to remove target directories from git history..."

# Check if BFG is available
if command -v bfg &> /dev/null; then
    echo "Using BFG Repo-Cleaner (recommended method)..."
    
    # Create a mirror clone in /tmp
    MIRROR_DIR="/tmp/orderbook_server_$(date +%s).git"
    echo "Creating mirror clone in $MIRROR_DIR..."
    git clone --mirror . "$MIRROR_DIR"
    cd "$MIRROR_DIR"
    
    # Remove target directories
    echo "Removing target directories..."
    bfg --delete-folders target
    bfg --delete-folders crates/runtime/target  
    bfg --delete-folders crates/primitives/target
    
    # Clean up
    echo "Cleaning up..."
    git reflog expire --expire=now --all
    git gc --prune=now --aggressive
    
    echo ""
    echo "BFG cleanup completed in mirror: $MIRROR_DIR"
    echo "To apply changes:"
    echo "  1. Review the changes in the mirror"
    echo "  2. Copy .git directory: cp -r $MIRROR_DIR/.git /path/to/original/repo/"
    echo "  3. Force push: git push --force --all"
    
elif command -v git-filter-repo &> /dev/null; then
    echo "Using git-filter-repo (modern alternative)..."
    git filter-repo --path target --invert-paths
    git filter-repo --path crates/runtime/target --invert-paths
    git filter-repo --path crates/primitives/target --invert-paths
    echo "Done! Now force push: git push --force --all"
    
else
    echo "BFG and git-filter-repo not found. Using git filter-branch..."
    echo "For best results, install BFG: brew install bfg"
    echo ""
    
    # Comprehensive git filter-branch
    git filter-branch --force --index-filter '
        git ls-files | grep -E "(^|/)target/" | xargs -r git rm -rf --cached --ignore-unmatch 2>/dev/null || true
    ' --prune-empty --tag-name-filter cat -- --all
    
    # Clean up
    git for-each-ref --format="%(refname)" refs/original/ | xargs -n 1 git update-ref -d 2>/dev/null || true
    git reflog expire --expire=now --all
    git gc --prune=now --aggressive
    
    echo "Done! Now force push: git push --force --all"
fi

