#!/usr/bin/env bash
set -u

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir/.."

failed=0

run_step() {
    local label="$1"
    shift
    local executable="$1"
    if ! command -v "$executable" >/dev/null 2>&1; then
        printf '[BLOCKED] %s (missing %s)\n' "$label" "$executable"
        failed=1
        return
    fi
    printf '\n[RUN] %s\n' "$label"
    if "$@"; then
        printf '[PASS] %s\n' "$label"
    else
        local result=$?
        printf '[FAIL] %s (exit %s)\n' "$label" "$result"
        failed=1
    fi
}

run_step 'cargo fmt --all -- --check' cargo fmt --all -- --check
run_step 'cargo clippy --workspace --all-targets --all-features --offline -- -D warnings' cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
run_step 'cargo test --workspace --offline --quiet' cargo test --workspace --offline --quiet
run_step 'cargo check --workspace --all-targets --all-features --offline' cargo check --workspace --all-targets --all-features --offline
run_step 'pnpm --dir frontend typecheck' pnpm --dir frontend typecheck
run_step 'pnpm --dir frontend test' pnpm --dir frontend test
run_step 'pnpm --dir frontend build' pnpm --dir frontend build
run_step 'git diff --check' git diff --check

if (( failed != 0 )); then
    printf '\nLocal verification failed or was blocked.\n'
    exit 1
fi
printf '\nAll local verification steps passed.\n'
