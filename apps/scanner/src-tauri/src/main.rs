// Without this a release build on Windows is a console program and opens a
// black terminal window beside the application; seen on the owner's first
// run of the CI build, 2026-09-05. Debug builds keep the console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    jlr_scanner_shell::run();
}
