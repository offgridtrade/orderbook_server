# Removing Target Directories from Git History

The large files in `target/` directories are still in git history. Here are the best methods to remove them:

## Method 1: BFG Repo-Cleaner (Recommended - Fastest & Most Reliable)

BFG is specifically designed for removing large files from git history.

1. **Install BFG:**
   ```bash
   brew install bfg  # macOS
   # OR download from: https://rtyley.github.io/bfg-repo-cleaner/
   ```

2. **Clone a fresh copy of your repo:**
   ```bash
   cd /tmp
   git clone --mirror https://github.com/offgridtrade/orderbook_server.git
   ```

3. **Remove target directories:**
   ```bash
   cd orderbook_server.git
   bfg --delete-folders target
   bfg --delete-folders crates/runtime/target
   bfg --delete-folders crates/primitives/target
   ```

4. **Clean up and push:**
   ```bash
   git reflog expire --expire=now --all
   git gc --prune=now --aggressive
   git push --force
   ```

## Method 2: git filter-repo (Modern Alternative)

1. **Install git-filter-repo:**
   ```bash
   brew install git-filter-repo  # macOS
   # OR: pip3 install git-filter-repo
   ```

2. **Remove target directories:**
   ```bash
   git filter-repo --path target --invert-paths
   git filter-repo --path crates/runtime/target --invert-paths
   git filter-repo --path crates/primitives/target --invert-paths
   ```

3. **Force push:**
   ```bash
   git push --force --all
   git push --force --tags
   ```

## Method 3: Manual git filter-branch (Already Attempted)

If the previous attempts didn't work, try this more comprehensive version:

```bash
# Remove all target directories recursively
git filter-branch --force --index-filter '
    git ls-files | grep -E "(^|/)target/" | xargs git rm -rf --cached --ignore-unmatch
' --prune-empty --tag-name-filter cat -- --all

# Clean up
git for-each-ref --format="%(refname)" refs/original/ | xargs -n 1 git update-ref -d
git reflog expire --expire=now --all
git gc --prune=now --aggressive
```

## After Any Method

1. **Verify files are gone:**
   ```bash
   git log --all --full-history --name-only | grep target
   # Should return nothing
   ```

2. **Force push (coordinate with team first!):**
   ```bash
   git push --force --all
   git push --force --tags
   ```

## Important Notes

- ⚠️ **This rewrites git history** - coordinate with your team
- Team members will need to re-clone or reset their repos
- Make sure `.gitignore` includes `target/` and `**/target/` (already done)
- BFG is the fastest and most reliable method for large files

