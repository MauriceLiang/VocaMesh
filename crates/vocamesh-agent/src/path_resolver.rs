use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use vocamesh_core::{Provider, ToolSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Missing,
    InvalidManual {
        path: PathBuf,
    },
    Found {
        path: PathBuf,
        canonical_path: PathBuf,
        source: ToolSource,
    },
}

pub fn resolve(provider: Provider, manual: Option<&Path>) -> Result<Resolution, std::io::Error> {
    let path_value = std::env::var_os("PATH");
    let standard_locations = standard_locations();
    resolve_with_paths(provider, manual, path_value.as_deref(), &standard_locations)
}

fn resolve_with_paths(
    provider: Provider,
    manual: Option<&Path>,
    path_value: Option<&OsStr>,
    standard_locations: &[PathBuf],
) -> Result<Resolution, std::io::Error> {
    if let Some(manual) = manual {
        if is_provider_executable(manual, provider) && is_executable_file(manual) {
            let canonical_path = manual.canonicalize()?;
            return Ok(Resolution::Found {
                path: manual.to_path_buf(),
                canonical_path,
                source: ToolSource::Manual,
            });
        }
        return Ok(Resolution::InvalidManual {
            path: manual.to_path_buf(),
        });
    }

    for directory in normalized_directories(path_value) {
        if let Some((path, canonical_path)) = find_in_directory(&directory, provider) {
            return Ok(Resolution::Found {
                path,
                canonical_path,
                source: ToolSource::Path,
            });
        }
    }

    for directory in normalized_directories_for_list(standard_locations) {
        if let Some((path, canonical_path)) = find_in_directory(&directory, provider) {
            return Ok(Resolution::Found {
                path,
                canonical_path,
                source: ToolSource::StandardLocation,
            });
        }
    }

    Ok(Resolution::Missing)
}

pub fn validate_manual_path(
    path: &Path,
    provider: Provider,
) -> Result<(PathBuf, PathBuf), std::io::Error> {
    if !path.is_absolute() || !is_provider_executable(path, provider) || !is_executable_file(path) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid executable path",
        ));
    }
    Ok((path.to_path_buf(), path.canonicalize()?))
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
        matches!(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("exe" | "com" | "cmd" | "bat")
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

fn is_provider_executable(path: &Path, provider: Provider) -> bool {
    #[cfg(windows)]
    {
        let expected = provider.executable();
        let stem_matches = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.eq_ignore_ascii_case(expected));
        let extension_supported = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .is_some_and(|extension| matches!(extension.as_str(), "exe" | "com" | "cmd" | "bat"));
        stem_matches && extension_supported
    }
    #[cfg(not(windows))]
    {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == provider.executable())
    }
}

fn find_in_directory(directory: &Path, provider: Provider) -> Option<(PathBuf, PathBuf)> {
    #[cfg(windows)]
    {
        for extension in [".exe", ".com", ".cmd", ".bat"] {
            let candidate = directory.join(format!("{}{extension}", provider.executable()));
            if is_executable_file(&candidate) {
                return candidate
                    .canonicalize()
                    .ok()
                    .map(|canonical_path| (candidate, canonical_path));
            }
        }
    }
    #[cfg(not(windows))]
    {
        let candidate = directory.join(provider.executable());
        if is_provider_executable(&candidate, provider) && is_executable_file(&candidate) {
            return candidate
                .canonicalize()
                .ok()
                .map(|canonical_path| (candidate, canonical_path));
        }
    }
    None
}

fn normalized_directories(path_value: Option<&OsStr>) -> Vec<PathBuf> {
    let Some(path_value) = path_value else {
        return Vec::new();
    };
    let paths = std::env::split_paths(path_value).collect::<Vec<_>>();
    normalized_directories_for_list(&paths)
}

fn normalized_directories_for_list(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .iter()
        .filter(|path| path.is_absolute())
        .filter_map(|path| {
            let canonical = path.canonicalize().ok()?;
            if !canonical.is_dir() || !seen.insert(canonical.clone()) {
                return None;
            }
            Some(path.to_path_buf())
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn standard_locations() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        directories.extend([
            home.join(".local/bin"),
            home.join(".bun/bin"),
            home.join(".cargo/bin"),
            home.join(".volta/bin"),
            home.join(".asdf/shims"),
            home.join(".local/share/mise/shims"),
            home.join(".npm-global/bin"),
        ]);
    }
    directories
}

#[cfg(not(target_os = "macos"))]
fn standard_locations() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn executable_name(provider: Provider) -> String {
        #[cfg(windows)]
        {
            format!("{}.exe", provider.executable())
        }
        #[cfg(not(windows))]
        {
            provider.executable().to_owned()
        }
    }

    #[test]
    fn manual_path_must_be_absolute_file_and_executable() {
        let relative = PathBuf::from("relative").join(executable_name(Provider::Codex));
        assert!(validate_manual_path(&relative, Provider::Codex).is_err());
        let directory = tempfile::tempdir().unwrap();
        assert!(validate_manual_path(directory.path(), Provider::Codex).is_err());
        let file = directory.path().join(executable_name(Provider::Codex));
        fs::write(&file, b"not actually executed by this test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(validate_manual_path(&file, Provider::Codex).is_err());
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(
                validate_manual_path(&file, Provider::Codex).unwrap(),
                (file.clone(), file.canonicalize().unwrap())
            );
        }
    }

    #[test]
    fn manual_path_must_match_provider_executable_name() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join(executable_name(Provider::Claude));
        fs::write(&file, b"test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(validate_manual_path(&file, Provider::Codex).is_err());
        assert!(validate_manual_path(&file, Provider::Claude).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn path_search_returns_display_and_canonical_executable_paths() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("codex-real");
        let alias = directory.path().join(executable_name(Provider::Codex));
        fs::write(&target, b"test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt};
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
            symlink(&target, &alias).unwrap();
        }
        let resolved = resolve_with_paths(
            Provider::Codex,
            None,
            Some(directory.path().as_os_str()),
            &[],
        )
        .unwrap();
        let Resolution::Found {
            path,
            canonical_path,
            source,
        } = resolved
        else {
            panic!("expected executable to be found")
        };
        assert_eq!(path, alias);
        assert_eq!(canonical_path, target.canonicalize().unwrap());
        assert_eq!(source, ToolSource::Path);
    }

    #[test]
    fn standard_locations_are_checked_after_path_in_declared_order() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let first_cli = first.path().join(executable_name(Provider::Codex));
        let second_cli = second.path().join(executable_name(Provider::Codex));
        fs::write(&first_cli, b"first").unwrap();
        fs::write(&second_cli, b"second").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for cli in [&first_cli, &second_cli] {
                fs::set_permissions(cli, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        let path_hit = resolve_with_paths(
            Provider::Codex,
            None,
            Some(second.path().as_os_str()),
            &[first.path().to_path_buf()],
        )
        .unwrap();
        assert!(
            matches!(path_hit, Resolution::Found { path, source: ToolSource::Path, .. } if path == second_cli)
        );

        let standard_hit = resolve_with_paths(
            Provider::Codex,
            None,
            None,
            &[second.path().to_path_buf(), first.path().to_path_buf()],
        )
        .unwrap();
        assert!(
            matches!(standard_hit, Resolution::Found { path, source: ToolSource::StandardLocation, .. } if path == second_cli)
        );
    }

    #[cfg(unix)]
    #[test]
    fn dangling_manual_symlink_is_rejected() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let alias = directory.path().join("codex");
        symlink(directory.path().join("missing-target"), &alias).unwrap();
        assert!(validate_manual_path(&alias, Provider::Codex).is_err());
    }

    #[test]
    fn relative_and_empty_path_entries_are_ignored() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join(executable_name(Provider::Codex));
        fs::write(&file, b"test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let relative_path =
            std::env::join_paths([PathBuf::from(""), PathBuf::from("relative")]).unwrap();
        assert_eq!(
            resolve_with_paths(Provider::Codex, None, Some(&relative_path), &[]).unwrap(),
            Resolution::Missing
        );
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
        let (resolved, _) = find_in_directory(directory.path(), Provider::Codex).unwrap();
        assert!(is_windows_script(&resolved));
    }
}
