# The Tauri shell crate: the one that needs GTK and WebKit, which the slim
# image has not got. Run inside rust:1.98-bookworm; see scripts/preflight.ps1.
# `cargo fmt` is not repeated here: the workspace pass covers this crate too.
set -e
echo "--- system libraries for the shell crate ---"
apt-get update -qq >/dev/null
apt-get install -y -qq libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libudev-dev pkg-config >/dev/null

cd /w/apps/scanner/src-tauri
export CARGO_TARGET_DIR=/w/target/linux-shell

echo "--- cargo clippy -D warnings ---"
cargo clippy --all-targets -- -D warnings

echo "--- cargo test (includes the end-to-end bench test) ---"
cargo test
