# Share one warm target dir across this repo's worktrees (each land runs in a
# fresh worktree; a per-worktree target/ means a cold build every time).
export CARGO_TARGET_DIR := env("CARGO_TARGET_DIR", home_directory() / ".cache" / "targets" / "health-cli")

# Gate run by the land queue before merging.
test-gate:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Coverage report (slow, instrumented build); not part of test-gate.
coverage:
    cargo llvm-cov --html
