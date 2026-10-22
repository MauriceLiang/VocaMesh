use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use vocamesh_core::ServiceError;

const ERR_WORKSPACE: ServiceError = ServiceError::new(
    "execution.workspace-invalid",
    "工作目录无效或包含无法安全处理的内容",
);
const MAX_FILES: usize = 100_000;
const MAX_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) fn validate_workspace(path: &str, app_data: &Path) -> Result<PathBuf, ServiceError> {
    let canonical = fs::canonicalize(path).map_err(|_| ERR_WORKSPACE)?;
    let app_data = fs::canonicalize(app_data).map_err(|_| ERR_WORKSPACE)?;
    if !canonical.is_dir() {
        return Err(ERR_WORKSPACE);
    }
    if is_too_broad(&canonical)
        || canonical.starts_with(&app_data)
        || app_data.starts_with(&canonical)
    {
        return Err(ERR_WORKSPACE);
    }
    Ok(canonical)
}

fn is_too_broad(path: &Path) -> bool {
    if path.parent().is_none() {
        return true;
    }
    for variable in ["HOME", "USERPROFILE"] {
        if let Some(home) = std::env::var_os(variable)
            .map(PathBuf::from)
            .and_then(|p| fs::canonicalize(p).ok())
        {
            if home == path || home.starts_with(path) {
                return true;
            }
        }
    }
    #[cfg(unix)]
    for system in [
        "/System",
        "/etc",
        "/usr",
        "/bin",
        "/sbin",
        "/Library",
        "/Applications",
    ] {
        if Path::new(system) == path || path.starts_with(system) {
            return true;
        }
    }
    #[cfg(windows)]
    for variable in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(system) = std::env::var_os(variable)
            .map(PathBuf::from)
            .and_then(|p| fs::canonicalize(p).ok())
        {
            if system == path || path.starts_with(system) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn create_filtered_snapshot(
    source: &Path,
    destination: &Path,
) -> Result<(), ServiceError> {
    fs::create_dir_all(destination).map_err(|_| ERR_WORKSPACE)?;
    let mut copied_files = 0usize;
    let mut copied_bytes = 0u64;
    if let Err(error) = copy_tree(source, destination, &mut copied_files, &mut copied_bytes) {
        let _ = fs::remove_dir_all(destination);
        return Err(error);
    }
    Ok(())
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    files: &mut usize,
    bytes: &mut u64,
) -> Result<(), ServiceError> {
    for entry in fs::read_dir(source).map_err(|_| ERR_WORKSPACE)? {
        let entry = entry.map_err(|_| ERR_WORKSPACE)?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| ERR_WORKSPACE)?;
        if metadata.file_type().is_symlink() || sensitive_name(&name_text, metadata.is_dir()) {
            continue;
        }
        let target = destination.join(&name);
        if metadata.is_dir() {
            fs::create_dir(&target).map_err(|_| ERR_WORKSPACE)?;
            copy_tree(&entry.path(), &target, files, bytes)?;
        } else if metadata.is_file() {
            *files += 1;
            if *files > MAX_FILES || metadata.len() > MAX_BYTES.saturating_sub(*bytes) {
                return Err(ERR_WORKSPACE);
            }
            let input = fs::File::open(entry.path()).map_err(|_| ERR_WORKSPACE)?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|_| ERR_WORKSPACE)?;
            let remaining = MAX_BYTES.saturating_sub(*bytes);
            let copied = std::io::copy(&mut input.take(remaining + 1), &mut output)
                .map_err(|_| ERR_WORKSPACE)?;
            if copied > remaining {
                return Err(ERR_WORKSPACE);
            }
            *bytes += copied;
        }
    }
    Ok(())
}

fn sensitive_name(name: &str, directory: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    if directory {
        return matches!(
            lower.as_str(),
            ".git"
                | ".ssh"
                | ".aws"
                | ".config"
                | ".claude"
                | ".codex"
                | ".opencode"
                | ".secrets"
                | ".gnupg"
                | ".azure"
                | ".kube"
                | ".docker"
                | "credentials"
                | "secrets"
                | "node_modules"
                | "target"
                | "dist"
                | ".venv"
        );
    }
    lower == ".env"
        || lower.starts_with(".env.")
        || lower == ".envrc"
        || matches!(
            lower.as_str(),
            ".npmrc"
                | ".netrc"
                | ".pypirc"
                | ".git-credentials"
                | "id_rsa"
                | "id_dsa"
                | "id_ecdsa"
                | "id_ed25519"
                | "authorized_keys"
        )
        || [".pem", ".key", ".p8", ".der", ".asc", ".p12", ".pfx"]
            .iter()
            .any(|ext| lower.ends_with(ext))
        || ["secret", "credential", "token", "private"]
            .iter()
            .any(|marker| lower.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn filtered_snapshot_omits_credentials_and_symlinks() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        fs::write(source.path().join("README.md"), "safe").unwrap();
        fs::write(source.path().join(".env"), "secret").unwrap();
        for directory in [
            ".ssh",
            ".aws",
            ".config",
            ".claude",
            ".codex",
            ".opencode",
            ".git",
            "credentials",
        ] {
            fs::create_dir(source.path().join(directory)).unwrap();
            fs::write(source.path().join(directory).join("secret.json"), "secret").unwrap();
        }
        fs::write(source.path().join("id_ecdsa"), "private").unwrap();
        fs::write(source.path().join("id_rsa"), "private").unwrap();
        fs::write(source.path().join("id_ed25519"), "private").unwrap();
        fs::write(source.path().join("client.pem"), "private").unwrap();
        fs::write(source.path().join("auth-token.txt"), "secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", source.path().join("outside")).unwrap();

        create_filtered_snapshot(source.path(), destination.path()).unwrap();
        assert_eq!(
            fs::read_to_string(destination.path().join("README.md")).unwrap(),
            "safe"
        );
        assert!(!destination.path().join(".env").exists());
        for directory in [
            ".ssh",
            ".aws",
            ".config",
            ".claude",
            ".codex",
            ".opencode",
            ".git",
            "credentials",
        ] {
            assert!(!destination.path().join(directory).exists());
        }
        assert!(!destination.path().join("id_ecdsa").exists());
        assert!(!destination.path().join("id_rsa").exists());
        assert!(!destination.path().join("id_ed25519").exists());
        assert!(!destination.path().join("client.pem").exists());
        assert!(!destination.path().join("auth-token.txt").exists());
        assert!(!destination.path().join("outside").exists());
    }

    #[test]
    fn common_roots_are_rejected() {
        let temp = tempdir().unwrap();
        let app_data = temp.path().join("app-data");
        let workspace = temp.path().join("workspace");
        fs::create_dir(&app_data).unwrap();
        fs::create_dir(&workspace).unwrap();
        assert!(validate_workspace(workspace.to_str().unwrap(), &app_data).is_ok());
        assert!(validate_workspace(temp.path().to_str().unwrap(), &app_data).is_err());
        assert!(validate_workspace(app_data.to_str().unwrap(), &app_data).is_err());
        #[cfg(unix)]
        assert!(validate_workspace("/", &app_data).is_err());
        if let Some(home) = std::env::var_os("HOME") {
            assert!(validate_workspace(&home.to_string_lossy(), &app_data).is_err());
        }
    }
}
