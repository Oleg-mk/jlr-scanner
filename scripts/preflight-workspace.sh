# Everything the CI `rust` job runs, for the portable crates. Run inside the
# Rust container; see scripts/preflight.ps1. Fails on the first refusal, so
# the exit code is the answer.
set -e
cd /w
export CARGO_TARGET_DIR=/w/target/linux

echo "--- cargo fmt --check (the whole workspace, shell crate included) ---"
cargo fmt --all -- --check

echo "--- cargo clippy -D warnings ---"
cargo clippy --workspace --exclude prowlone-shell --exclude transport-serial --all-targets -- -D warnings

echo "--- cargo test ---"
cargo test --workspace --exclude prowlone-shell --exclude transport-serial
