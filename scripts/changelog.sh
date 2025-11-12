#!/usr/bin/env bash
set -eo pipefail

run_unless_dry_run() {
    if [ "$DRY_RUN" = "true" ]; then
        echo "skipping due to dry run: $*" >&2
    else
        "$@"
    fi
}

# Set root to WORKSPACE_ROOT if provided, otherwise use git root
if [ -n "$WORKSPACE_ROOT" ]; then
    root=$WORKSPACE_ROOT
else
    # Find git repository root
    root=$(git rev-parse --show-toplevel 2>/dev/null)
    if [ -z "$root" ]; then
        echo "Error: Not in a git repository and WORKSPACE_ROOT not set" >&2
        exit 1
    fi
fi

crate=$CRATE_ROOT
crate_glob="${crate#"$root/"}/**"

if [[ "$crate" = */tests/* || "$crate" = *test-utils* ]]; then
    exit 0
fi

command=(git cliff --workdir "$root" --config "$root/cliff.toml" "${@}")
run_unless_dry_run "${command[@]}" --output "$root/CHANGELOG.md"
if [ -n "$crate" ] && [ "$root" != "$crate" ]; then
    run_unless_dry_run "${command[@]}" --include-path "$crate_glob" --output "$crate/CHANGELOG.md"
fi