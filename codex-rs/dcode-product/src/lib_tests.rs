use std::path::Path;

use pretty_assertions::assert_eq;

use super::CODEX_PRODUCT;
use super::DCODE_PRODUCT;
use super::product_for_executable;

#[test]
fn detects_dcode_release_binary_names() {
    for executable in [
        "dcode",
        "dcode.exe",
        "dcode-aarch64-apple-darwin",
        "dcode-x86_64-pc-windows-msvc.exe",
    ] {
        assert_eq!(product_for_executable(Path::new(executable)), DCODE_PRODUCT);
    }
}

#[test]
fn preserves_codex_for_upstream_and_test_binaries() {
    for executable in ["codex", "codex.exe", "codex_cli-deadbeef", "tui-unit-tests"] {
        assert_eq!(product_for_executable(Path::new(executable)), CODEX_PRODUCT);
    }
}
