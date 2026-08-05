use std::ffi::OsStr;
use std::path::Path;
use std::sync::OnceLock;

/// Product-level defaults supplied by a downstream Codex distribution.
///
/// Core crates should use this metadata only for product identity and defaults;
/// model execution and agent behavior must remain independent of the selected
/// distribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductInfo {
    pub display_name: &'static str,
    pub long_name: &'static str,
    pub cli_name: &'static str,
    pub description: &'static str,
    pub home_env: &'static str,
    pub default_home_dir: &'static str,
    pub default_model_provider: &'static str,
    pub default_model: Option<&'static str>,
    pub github_repository: &'static str,
    pub version: &'static str,
}

/// DCode release version, intentionally independent from upstream workspace versions.
pub const DCODE_VERSION: &str = "0.2.1";

pub const CODEX_PRODUCT: ProductInfo = ProductInfo {
    display_name: "Codex",
    long_name: "OpenAI Codex",
    cli_name: "codex",
    description: "OpenAI's command-line coding agent",
    home_env: "CODEX_HOME",
    default_home_dir: ".codex",
    default_model_provider: "openai",
    default_model: None,
    github_repository: "openai/codex",
    version: env!("CARGO_PKG_VERSION"),
};

pub const DCODE_PRODUCT: ProductInfo = ProductInfo {
    display_name: "DCode",
    long_name: "DCode",
    cli_name: "dcode",
    description: "DeepSeek-powered command-line coding agent",
    home_env: "DCODE_HOME",
    default_home_dir: ".dcode",
    default_model_provider: "deepseek",
    default_model: Some("deepseek-v4-flash"),
    github_repository: "dopejs/dcode",
    version: DCODE_VERSION,
};

static CURRENT_PRODUCT: OnceLock<ProductInfo> = OnceLock::new();
pub const PRODUCT_DISTRIBUTION_ENV: &str = "CODEX_DISTRIBUTION";

/// Returns the product represented by the running executable.
///
/// Unit tests and upstream binaries keep Codex behavior by default. A binary
/// named `dcode` (including target-suffixed release binaries) selects DCode.
pub fn current_product() -> &'static ProductInfo {
    CURRENT_PRODUCT.get_or_init(|| {
        if std::env::var(PRODUCT_DISTRIBUTION_ENV).as_deref() == Ok("dcode") {
            return DCODE_PRODUCT;
        }
        std::env::current_exe()
            .ok()
            .as_deref()
            .map(product_for_executable)
            .unwrap_or(CODEX_PRODUCT)
    })
}

pub fn product_for_executable(executable: &Path) -> ProductInfo {
    product_for_executable_name(executable.file_stem())
}

fn product_for_executable_name(file_stem: Option<&OsStr>) -> ProductInfo {
    let is_dcode = file_stem
        .and_then(OsStr::to_str)
        .is_some_and(|name| name == "dcode" || name.starts_with("dcode-"));
    if is_dcode {
        DCODE_PRODUCT
    } else {
        CODEX_PRODUCT
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
