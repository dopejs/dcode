use std::path::PathBuf;
use std::process::Command;

fn main() -> std::io::Result<()> {
    let codex_executable = sibling_codex_executable()?;
    let mut command = Command::new(codex_executable);
    command
        .args(std::env::args_os().skip(1))
        .env(codex_dcode_product::PRODUCT_DISTRIBUTION_ENV, "dcode");

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec())
    }

    #[cfg(not(unix))]
    {
        let status = command.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn sibling_codex_executable() -> std::io::Result<PathBuf> {
    let current_executable = std::env::current_exe()?;
    let executable_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    Ok(current_executable.with_file_name(executable_name))
}
