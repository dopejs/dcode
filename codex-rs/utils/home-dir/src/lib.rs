use codex_utils_absolute_path::AbsolutePathBuf;
use dirs::home_dir;
use std::path::PathBuf;

/// Returns the path to the DCode configuration directory.
///
/// `DCODE_HOME` takes precedence. An explicitly configured `CODEX_HOME` remains
/// supported for compatibility; otherwise the directory defaults to `~/.dcode`.
///
/// - If either environment override is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - Without an override, this function does not verify that the default directory exists.
pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf> {
    let dcode_home_env = std::env::var("DCODE_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    let codex_home_env = std::env::var("CODEX_HOME")
        .ok()
        .filter(|val| !val.is_empty());
    find_codex_home_from_env(dcode_home_env.as_deref(), codex_home_env.as_deref())
}

fn find_codex_home_from_env(
    dcode_home_env: Option<&str>,
    codex_home_env: Option<&str>,
) -> std::io::Result<AbsolutePathBuf> {
    let (configured_home, env_name) = match (dcode_home_env, codex_home_env) {
        (Some(value), _) => (Some(value), "DCODE_HOME"),
        (None, Some(value)) => (Some(value), "CODEX_HOME"),
        (None, None) => (None, "DCODE_HOME"),
    };
    match configured_home {
        Some(val) => {
            let path = PathBuf::from(val);
            let metadata = std::fs::metadata(&path).map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("{env_name} points to {val:?}, but that path does not exist"),
                ),
                _ => std::io::Error::new(
                    err.kind(),
                    format!("failed to read {env_name} {val:?}: {err}"),
                ),
            })?;

            if !metadata.is_dir() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("{env_name} points to {val:?}, but that path is not a directory"),
                ))
            } else {
                let canonical = path.canonicalize().map_err(|err| {
                    std::io::Error::new(
                        err.kind(),
                        format!("failed to canonicalize {env_name} {val:?}: {err}"),
                    )
                })?;
                AbsolutePathBuf::from_absolute_path(canonical)
            }
        }
        None => {
            let mut p = home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find home directory",
                )
            })?;
            p.push(".dcode");
            AbsolutePathBuf::from_absolute_path(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::find_codex_home_from_env;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use dirs::home_dir;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::io::ErrorKind;
    use tempfile::TempDir;

    #[test]
    fn find_codex_home_env_missing_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let missing = temp_home.path().join("missing-codex-home");
        let missing_str = missing
            .to_str()
            .expect("missing codex home path should be valid utf-8");

        let err =
            find_codex_home_from_env(Some(missing_str), None).expect_err("missing DCODE_HOME");
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.to_string().contains("DCODE_HOME"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_env_file_path_is_fatal() {
        let temp_home = TempDir::new().expect("temp home");
        let file_path = temp_home.path().join("codex-home.txt");
        fs::write(&file_path, "not a directory").expect("write temp file");
        let file_str = file_path
            .to_str()
            .expect("file codex home path should be valid utf-8");

        let err = find_codex_home_from_env(Some(file_str), None).expect_err("file DCODE_HOME");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn find_codex_home_env_valid_directory_canonicalizes() {
        let temp_home = TempDir::new().expect("temp home");
        let temp_str = temp_home
            .path()
            .to_str()
            .expect("temp codex home path should be valid utf-8");

        let resolved = find_codex_home_from_env(Some(temp_str), None).expect("valid DCODE_HOME");
        let expected = temp_home
            .path()
            .canonicalize()
            .expect("canonicalize temp home");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn find_codex_home_without_env_uses_default_home_dir() {
        let resolved =
            find_codex_home_from_env(/*dcode_home_env*/ None, /*codex_home_env*/ None)
                .expect("default DCODE_HOME");
        let mut expected = home_dir().expect("home dir");
        expected.push(".dcode");
        let expected = AbsolutePathBuf::from_absolute_path(expected).expect("absolute home");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn dcode_home_takes_precedence_over_legacy_codex_home() {
        let dcode_home = TempDir::new().expect("DCode temp home");
        let codex_home = TempDir::new().expect("Codex temp home");

        let resolved =
            find_codex_home_from_env(dcode_home.path().to_str(), codex_home.path().to_str())
                .expect("valid DCode home");

        let expected = AbsolutePathBuf::from_absolute_path(
            dcode_home
                .path()
                .canonicalize()
                .expect("canonical DCode home"),
        )
        .expect("absolute DCode home");
        assert_eq!(resolved, expected);
    }
}
