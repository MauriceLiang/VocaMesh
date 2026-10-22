use std::path::{Path, PathBuf};
use vocamesh_core::{Provider, ToolSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Missing,
    InvalidManual { path: PathBuf },
    Found { path: PathBuf, source: ToolSource },
}

pub fn resolve(provider: Provider, manual: Option<&Path>) -> Result<Resolution, std::io::Error> {
    if let Some(manual) = manual {
        if is_executable_file(manual) {
            return Ok(Resolution::Found {
                path: manual.to_path_buf(),
                source: ToolSource::Manual,
            });
        }
        return Ok(Resolution::InvalidManual {
            path: manual.to_path_buf(),
        });
    }

    let executable = provider.executable();
    if let Some(path_value) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path_value) {
            if let Some(path) = find_in_directory(&directory, executable) {
                return Ok(Resolution::Found {
                    path,
                    source: ToolSource::Path,
                });
            }
        }
    }

    #[cfg(target_os = "macos")]
    for directory in standard_macos_locations() {
        if let Some(path) = find_in_directory(&directory, executable) {
            return Ok(Resolution::Found {
                path,
                source: ToolSource::StandardLocation,
            });
        }
    }

    Ok(Resolution::Missing)
}

pub fn validate_manual_path(path: &Path) -> Result<PathBuf, std::io::Error> {
    if !path.is_absolute() || !is_executable_file(path) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid executable path",
        ));
    }
    path.canonicalize()
}

pub fn is_windows_script(path: &Path) -> bool {
    #[cfg(windows)]
    {
        matches!(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("cmd" | "bat")
        )
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        path.extension().is_some_and(|extension| {
            let extension = extension.to_string_lossy();
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
                .split(';')
                .any(|allowed| {
                    allowed
                        .trim_start_matches('.')
                        .eq_ignore_ascii_case(&extension)
                })
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

fn find_in_directory(directory: &Path, executable: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let mut extensions: Vec<String> = std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| extension.to_ascii_lowercase())
            .collect();
        if extensions.is_empty() {
            extensions.push(".exe".to_owned());
        }
        for extension in extensions {
            let candidate = directory.join(format!("{executable}{extension}"));
            if is_executable_file(&candidate) {
                return candidate.canonicalize().ok();
            }
        }
    }
    #[cfg(not(windows))]
    {
        let candidate = directory.join(executable);
        if is_executable_file(&candidate) {
            return candidate.canonicalize().ok();
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn standard_macos_locations() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        directories.extend([home.join(".local/bin"), home.join(".bun/bin")]);
    }
    directories
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn manual_path_must_be_absolute_file_and_executable() {
        assert!(validate_manual_path(Path::new("relative/claude")).is_err());
        let directory = tempfile::tempdir().unwrap();
        assert!(validate_manual_path(directory.path()).is_err());
        let file = directory.path().join("codex");
        fs::write(&file, b"not actually executed by this test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(validate_manual_path(&file).is_err());
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(
                validate_manual_path(&file).unwrap(),
                file.canonicalize().unwrap()
            );
        }
    }

    #[test]
    fn reports_missing_provider_without_scanning_user_directories() {
        let result = resolve(
            Provider::Codex,
            Some(Path::new("/definitely/missing/codex")),
        )
        .unwrap();
        assert_eq!(
            result,
            Resolution::InvalidManual {
                path: PathBuf::from("/definitely/missing/codex")
            }
        );
    }

    #[cfg(windows)]
    #[test]
    fn path_ext_finds_cmd_shim_but_classifies_it_as_unsafe_to_launch() {
        let directory = tempfile::tempdir().unwrap();
        let shim = directory.path().join("codex.cmd");
        fs::write(&shim, "@echo off\r\n").unwrap();
        let resolved = find_in_directory(directory.path(), "codex").unwrap();
        assert!(is_windows_script(&resolved));
    }
}
