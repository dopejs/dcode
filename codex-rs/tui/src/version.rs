/// The user-visible version for the active distribution.
pub fn cli_version() -> &'static str {
    codex_dcode_product::current_product().version
}
