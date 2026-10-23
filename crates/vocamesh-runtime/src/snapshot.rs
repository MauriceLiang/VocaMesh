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

pub(crate) struct ValidatedWorkspace {
    pub(crate) path: PathBuf,
    #[cfg(unix)]
    root: std::fs::File,
}

pub(crate) fn validate_workspace(
    path: &str,
    app_data: &Path,
) -> Result<ValidatedWorkspace, ServiceError> {
    let canonical = fs::canonicalize(path).map_err(|_| ERR_WORKSPACE)?;
    let app_data = fs::canonicalize(app_data).map_err(|_| ERR_WORKSPACE)?;
    let path_metadata = fs::metadata(&canonical).map_err(|_| ERR_WORKSPACE)?;
    if !path_metadata.is_dir() {
        return Err(ERR_WORKSPACE);
    }
    if is_too_broad(&canonical)
        || canonical.starts_with(&app_data)
        || app_data.starts_with(&canonical)
    {
        return Err(ERR_WORKSPACE);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let root = open_directory_path(&canonical).map_err(|_| ERR_WORKSPACE)?;
        let root_stat = file_stat(&root).map_err(|_| ERR_WORKSPACE)?;
        if root_stat.st_dev as u64 != path_metadata.dev()
            || root_stat.st_ino as u64 != path_metadata.ino()
        {
            return Err(ERR_WORKSPACE);
        }
        Ok(ValidatedWorkspace {
            path: canonical,
            root,
        })
    }
    #[cfg(not(unix))]
    {
        Ok(ValidatedWorkspace { path: canonical })
    }
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
            if system == path || path.starts_with(&system) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn create_filtered_snapshot(
    source: &ValidatedWorkspace,
    destination: &Path,
) -> Result<(), ServiceError> {
    let parent = destination.parent().ok_or(ERR_WORKSPACE)?;
    fs::create_dir_all(parent).map_err(|_| ERR_WORKSPACE)?;
    fs::create_dir(destination).map_err(|_| ERR_WORKSPACE)?;
    let result = copy_snapshot(source, destination);
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}

#[cfg(unix)]
fn copy_snapshot(source: &ValidatedWorkspace, destination: &Path) -> Result<(), ServiceError> {
    let mut files = 0usize;
    let mut bytes = 0u64;
    copy_directory_fd(&source.root, destination, &mut files, &mut bytes)
}

#[cfg(not(unix))]
fn copy_snapshot(source: &ValidatedWorkspace, destination: &Path) -> Result<(), ServiceError> {
    let mut files = 0usize;
    let mut bytes = 0u64;
    copy_tree_path(&source.path, destination, &mut files, &mut bytes)
}

#[cfg(unix)]
fn open_directory_path(path: &Path) -> std::io::Result<std::fs::File> {
    let canonical = fs::canonicalize(path)?;
    open_normalized_directory_path(&canonical)
}

#[cfg(unix)]
fn open_normalized_directory_path(path: &Path) -> std::io::Result<std::fs::File> {
    use std::{
        ffi::CString,
        os::{fd::FromRawFd, unix::ffi::OsStrExt},
    };

    if !path.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path must be absolute",
        ));
    }
    let root = CString::new("/").expect("static path has no NUL");
    let root_fd = unsafe {
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if root_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut directory = unsafe { std::fs::File::from_raw_fd(root_fd) };
    for component in path.components() {
        let std::path::Component::Normal(name) = component else {
            if matches!(component, std::path::Component::RootDir) {
                continue;
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "workspace path is not normalized",
            ));
        };
        let name = CString::new(name.as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path component")
        })?;
        directory = open_at(
            &directory,
            &name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )?;
    }
    Ok(directory)
}

#[cfg(unix)]
pub(crate) fn remove_snapshot(snapshot_root: &Path, request_id: &str) -> std::io::Result<()> {
    use std::ffi::CString;

    let root = match open_normalized_directory_path(snapshot_root) {
        Ok(root) => root,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let name = CString::new(request_id.as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid request id"))?;
    let metadata = match stat_at(&root, &name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "snapshot is not a directory",
        ));
    }
    let root_device = file_stat(&root)?.st_dev;
    remove_directory_entry_at(&root, &name, root_device)
}

#[cfg(unix)]
fn remove_directory_entry_at(
    parent: &std::fs::File,
    name: &std::ffi::CStr,
    root_device: libc::dev_t,
) -> std::io::Result<()> {
    use std::{
        ffi::CString,
        os::{fd::AsRawFd, unix::ffi::OsStrExt},
    };

    let before = stat_at(parent, name)?;
    match before.st_mode & libc::S_IFMT {
        libc::S_IFDIR => {
            if before.st_dev != root_device {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "snapshot crosses a filesystem boundary",
                ));
            }
            let child = open_at(
                parent,
                name,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?;
            let opened = file_stat(&child)?;
            if !same_file(&before, &opened) || opened.st_mode & libc::S_IFMT != libc::S_IFDIR {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "snapshot directory changed during cleanup",
                ));
            }
            for entry in directory_entries(&child)? {
                let child_name = CString::new(entry.as_bytes()).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid entry name")
                })?;
                remove_directory_entry_at(&child, &child_name, root_device)?;
            }
            let current = stat_at(parent, name)?;
            if !same_file(&before, &current) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "snapshot directory changed during cleanup",
                ));
            }
            let result =
                unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) };
            if result == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
        libc::S_IFREG | libc::S_IFLNK => {
            let current = stat_at(parent, name)?;
            if !same_file(&before, &current) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "snapshot entry changed during cleanup",
                ));
            }
            let result = unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) };
            if result == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "snapshot contains a special file",
        )),
    }
}

#[cfg(not(unix))]
pub(crate) fn remove_snapshot(snapshot_root: &Path, request_id: &str) -> std::io::Result<()> {
    let root_metadata = match fs::symlink_metadata(snapshot_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "snapshot root is not a directory",
        ));
    }
    let canonical_root = fs::canonicalize(snapshot_root)?;
    if canonical_root != snapshot_root {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "snapshot root is not normalized",
        ));
    }
    let target = canonical_root.join(request_id);
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "snapshot is not a directory",
        ));
    }
    fs::remove_dir_all(target)
}

#[cfg(unix)]
fn directory_entries(directory: &std::fs::File) -> std::io::Result<Vec<std::ffi::OsString>> {
    use std::{
        ffi::{CStr, CString},
        os::{fd::IntoRawFd, unix::ffi::OsStringExt},
    };

    let dot = CString::new(".").expect("static path has no NUL");
    let duplicate = open_at(
        directory,
        &dot,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
    )?
    .into_raw_fd();
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(std::io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        set_errno(0);
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::closedir(stream) };
            return if error.raw_os_error().unwrap_or(0) == 0 {
                Ok(names)
            } else {
                Err(error)
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            names.push(std::ffi::OsString::from_vec(name.to_vec()));
        }
    }
}

#[cfg(all(unix, target_vendor = "apple"))]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__error() = value };
}

#[cfg(all(unix, not(target_vendor = "apple")))]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__errno_location() = value };
}

#[cfg(unix)]
fn open_at(
    directory: &std::fs::File,
    name: &std::ffi::CStr,
    flags: libc::c_int,
) -> std::io::Result<std::fs::File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn stat_at(directory: &std::fs::File, name: &std::ffi::CStr) -> std::io::Result<libc::stat> {
    use std::os::fd::AsRawFd;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { stat.assume_init() })
}

#[cfg(unix)]
fn file_stat(file: &std::fs::File) -> std::io::Result<libc::stat> {
    use std::os::fd::AsRawFd;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { stat.assume_init() })
}

#[cfg(unix)]
fn same_file(left: &libc::stat, right: &libc::stat) -> bool {
    left.st_dev == right.st_dev && left.st_ino == right.st_ino
}

#[cfg(unix)]
fn copy_directory_fd(
    source: &std::fs::File,
    destination: &Path,
    files: &mut usize,
    bytes: &mut u64,
) -> Result<(), ServiceError> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let before_directory = source.metadata().map_err(|_| ERR_WORKSPACE)?;
    for name in directory_entries(source).map_err(|_| ERR_WORKSPACE)? {
        let name_text = name.to_string_lossy();
        let name_c = CString::new(name.as_bytes()).map_err(|_| ERR_WORKSPACE)?;
        let before = stat_at(source, &name_c).map_err(|_| ERR_WORKSPACE)?;
        let kind = before.st_mode & libc::S_IFMT;
        if sensitive_name(&name_text, kind == libc::S_IFDIR) {
            continue;
        }
        if kind == libc::S_IFLNK {
            return Err(ERR_WORKSPACE);
        }
        let target = destination.join(&name);
        if kind == libc::S_IFDIR {
            let child = open_at(
                source,
                &name_c,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
            .map_err(|_| ERR_WORKSPACE)?;
            let after = file_stat(&child).map_err(|_| ERR_WORKSPACE)?;
            if !same_file(&before, &after) || after.st_mode & libc::S_IFMT != libc::S_IFDIR {
                return Err(ERR_WORKSPACE);
            }
            fs::create_dir(&target).map_err(|_| ERR_WORKSPACE)?;
            copy_directory_fd(&child, &target, files, bytes)?;
        } else if kind == libc::S_IFREG {
            reserve_file_slot(files, bytes, before.st_size.max(0) as u64)?;
            let mut input = open_at(
                source,
                &name_c,
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
            .map_err(|_| ERR_WORKSPACE)?;
            let after = file_stat(&input).map_err(|_| ERR_WORKSPACE)?;
            if !same_file(&before, &after) || after.st_mode & libc::S_IFMT != libc::S_IFREG {
                return Err(ERR_WORKSPACE);
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|_| ERR_WORKSPACE)?;
            let before_copy = input.metadata().map_err(|_| ERR_WORKSPACE)?;
            copy_limited(&mut input, &mut output, bytes)?;
            if !metadata_unchanged(&input, &before_copy) {
                return Err(ERR_WORKSPACE);
            }
        } else {
            return Err(ERR_WORKSPACE);
        }
    }
    if !metadata_unchanged(source, &before_directory) {
        return Err(ERR_WORKSPACE);
    }
    Ok(())
}

#[cfg(not(unix))]
fn copy_tree_path(
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
        if sensitive_name(&name_text, metadata.is_dir()) {
            continue;
        }
        if metadata.file_type().is_symlink() {
            return Err(ERR_WORKSPACE);
        }
        let target = destination.join(&name);
        if metadata.is_dir() {
            fs::create_dir(&target).map_err(|_| ERR_WORKSPACE)?;
            copy_tree_path(&entry.path(), &target, files, bytes)?;
        } else if metadata.is_file() {
            reserve_file_slot(files, bytes, metadata.len())?;
            let mut input = fs::File::open(entry.path()).map_err(|_| ERR_WORKSPACE)?;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|_| ERR_WORKSPACE)?;
            let before_copy = input.metadata().map_err(|_| ERR_WORKSPACE)?;
            copy_limited(&mut input, &mut output, bytes)?;
            if !metadata_unchanged(&input, &before_copy) {
                return Err(ERR_WORKSPACE);
            }
        } else {
            return Err(ERR_WORKSPACE);
        }
    }
    Ok(())
}

fn metadata_unchanged(file: &std::fs::File, before: &fs::Metadata) -> bool {
    let Ok(after) = file.metadata() else {
        return false;
    };
    match (before.modified(), after.modified()) {
        (Ok(before_modified), Ok(after_modified)) => {
            before.len() == after.len() && before_modified == after_modified
        }
        _ => false,
    }
}

fn reserve_file_slot(
    files: &mut usize,
    bytes: &u64,
    metadata_len: u64,
) -> Result<(), ServiceError> {
    *files = files.saturating_add(1);
    if *files > MAX_FILES || metadata_len > MAX_BYTES.saturating_sub(*bytes) {
        return Err(ERR_WORKSPACE);
    }
    Ok(())
}

fn copy_limited<R: Read>(
    input: &mut R,
    output: &mut impl std::io::Write,
    bytes: &mut u64,
) -> Result<(), ServiceError> {
    let remaining = MAX_BYTES.saturating_sub(*bytes);
    let copied =
        std::io::copy(&mut input.take(remaining + 1), output).map_err(|_| ERR_WORKSPACE)?;
    if copied > remaining {
        return Err(ERR_WORKSPACE);
    }
    *bytes += copied;
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

    fn test_workspace(path: &Path) -> ValidatedWorkspace {
        let path = fs::canonicalize(path).unwrap();
        #[cfg(unix)]
        {
            let root = open_directory_path(&path).unwrap();
            ValidatedWorkspace { path, root }
        }
        #[cfg(not(unix))]
        {
            ValidatedWorkspace { path }
        }
    }

    #[cfg(unix)]
    #[test]
    fn safe_snapshot_cleanup_uses_directory_descriptors_and_does_not_follow_links() {
        use std::os::unix::fs::symlink;

        let data = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let snapshot_root = fs::canonicalize(data.path()).unwrap().join("run-snapshots");
        let snapshot = snapshot_root.join("00000000-0000-4000-8000-000000000001");
        fs::create_dir_all(&snapshot).unwrap();
        fs::write(snapshot.join("README.md"), "synthetic snapshot").unwrap();
        fs::write(outside.path().join("keep.txt"), "outside data").unwrap();
        symlink(outside.path(), snapshot.join("external-link")).unwrap();

        remove_snapshot(&snapshot_root, "00000000-0000-4000-8000-000000000001").unwrap();

        assert!(!snapshot.exists());
        assert_eq!(
            fs::read_to_string(outside.path().join("keep.txt")).unwrap(),
            "outside data"
        );
    }

    #[cfg(unix)]
    #[test]
    fn safe_snapshot_cleanup_rejects_a_symlinked_snapshot_root() {
        use std::os::unix::fs::symlink;

        let data = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let snapshot_root = fs::canonicalize(data.path()).unwrap().join("run-snapshots");
        let snapshot = outside.path().join("00000000-0000-4000-8000-000000000002");
        fs::create_dir(&snapshot).unwrap();
        fs::write(snapshot.join("keep.txt"), "outside data").unwrap();
        symlink(outside.path(), &snapshot_root).unwrap();

        assert!(remove_snapshot(&snapshot_root, "00000000-0000-4000-8000-000000000002").is_err());
        assert_eq!(
            fs::read_to_string(snapshot.join("keep.txt")).unwrap(),
            "outside data"
        );
    }

    #[test]
    fn filtered_snapshot_excludes_credentials_and_rejects_symlinks() {
        let source = tempdir().unwrap();
        let parent = tempdir().unwrap();
        let destination = parent.path().join("snapshot");
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
        let workspace = test_workspace(source.path());

        #[cfg(unix)]
        {
            assert!(create_filtered_snapshot(&workspace, &destination).is_err());
            assert!(!destination.exists());
            fs::remove_file(source.path().join("outside")).unwrap();
            create_filtered_snapshot(&workspace, &destination).unwrap();
        }
        #[cfg(not(unix))]
        create_filtered_snapshot(&workspace, &destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("README.md")).unwrap(),
            "safe"
        );
        assert!(!destination.join(".env").exists());
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
            assert!(!destination.join(directory).exists());
        }
        assert!(!destination.join("id_ecdsa").exists());
        assert!(!destination.join("id_rsa").exists());
        assert!(!destination.join("id_ed25519").exists());
        assert!(!destination.join("client.pem").exists());
        assert!(!destination.join("auth-token.txt").exists());
        assert!(!destination.join("outside").exists());
    }

    #[test]
    fn existing_destination_is_not_reused_or_removed() {
        let source = tempdir().unwrap();
        let parent = tempdir().unwrap();
        let destination = parent.path().join("snapshot");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("sentinel"), "keep").unwrap();
        let workspace = test_workspace(source.path());
        assert!(create_filtered_snapshot(&workspace, &destination).is_err());
        assert_eq!(
            fs::read_to_string(destination.join("sentinel")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn copy_limit_applies_to_actual_bytes_read() {
        let payload = b"growing file";
        let mut input = std::io::Cursor::new(payload.as_slice());
        let mut output = Vec::new();
        let mut bytes = MAX_BYTES - 4;
        assert!(copy_limited(&mut input, &mut output, &mut bytes).is_err());
        assert!(bytes <= MAX_BYTES);
    }

    #[test]
    fn file_count_and_metadata_size_limits_are_enforced() {
        let mut files = MAX_FILES;
        assert!(reserve_file_slot(&mut files, &0, 0).is_err());
        let mut files = 0;
        assert!(reserve_file_slot(&mut files, &MAX_BYTES, 1).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn replaced_file_symlink_is_rejected_by_descriptor_relative_open() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let source = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let file = source.path().join("entry.txt");
        fs::write(&file, "inside").unwrap();
        fs::write(outside.path().join("sentinel"), "outside").unwrap();
        let workspace = test_workspace(source.path());
        let directory = workspace.root;
        let name = CString::new(file.file_name().unwrap().as_bytes()).unwrap();
        let before = stat_at(&directory, &name).unwrap();
        assert_eq!(before.st_mode & libc::S_IFMT, libc::S_IFREG);
        fs::remove_file(&file).unwrap();
        std::os::unix::fs::symlink(outside.path().join("sentinel"), &file).unwrap();
        assert!(open_at(
            &directory,
            &name,
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_replaced_by_external_symlink_blocks_the_snapshot() {
        let source = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let parent = tempdir().unwrap();
        let destination = parent.path().join("snapshot");
        fs::create_dir(source.path().join("nested")).unwrap();
        fs::write(source.path().join("nested/inside.txt"), "inside").unwrap();
        fs::write(outside.path().join("sentinel"), "outside").unwrap();
        let workspace = test_workspace(source.path());
        fs::rename(
            source.path().join("nested"),
            source.path().join("saved-nested"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), source.path().join("nested")).unwrap();

        assert!(create_filtered_snapshot(&workspace, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(
            fs::read_to_string(outside.path().join("sentinel")).unwrap(),
            "outside"
        );
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
