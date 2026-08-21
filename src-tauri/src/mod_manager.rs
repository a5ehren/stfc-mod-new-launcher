use crate::errors::{io_context, LauncherError, LauncherResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;

pub fn parse_sha256(text: &str) -> LauncherResult<String> {
    let checksum = text
        .split_whitespace()
        .next()
        .ok_or_else(|| LauncherError::InvalidData {
            context: "parsing mod checksum".into(),
            message: "checksum file was empty".into(),
        })?;

    if !matches!(checksum.len(), 16 | 64) || !checksum.chars().all(|char| char.is_ascii_hexdigit())
    {
        return Err(LauncherError::InvalidData {
            context: "parsing mod checksum".into(),
            message: format!("checksum had invalid SHA-256 format: {checksum}"),
        });
    }

    Ok(checksum.to_ascii_lowercase())
}

pub fn sha256_file(path: &Path) -> LauncherResult<String> {
    let mut file = fs::File::open(path)
        .map_err(|err| io_context(format!("opening {}", path.display()), err))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| io_context(format!("reading {}", path.display()), err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Extracts a mod archive by replacing `target_dir`; use this only with staging paths.
pub fn extract_mod_archive(archive: &Path, target_dir: &Path) -> LauncherResult<()> {
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)
            .map_err(|err| io_context(format!("removing {}", target_dir.display()), err))?;
    }
    fs::create_dir_all(target_dir)
        .map_err(|err| io_context(format!("creating {}", target_dir.display()), err))?;

    let file = fs::File::open(archive)
        .map_err(|err| io_context(format!("opening {}", archive.display()), err))?;
    let decoder = zstd::Decoder::new(file)
        .map_err(|err| io_context(format!("decoding {}", archive.display()), err))?;
    let mut tar = tar::Archive::new(decoder);
    let entries = tar
        .entries()
        .map_err(|err| io_context(format!("reading entries from {}", archive.display()), err))?;
    for entry in entries {
        let mut entry = entry
            .map_err(|err| io_context(format!("reading entry from {}", archive.display()), err))?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(LauncherError::InvalidData {
                context: format!("extracting {}", archive.display()),
                message: format!("archive entry had unsupported type: {:?}", entry_type),
            });
        }

        let entry_path = entry.path().map_err(|err| LauncherError::InvalidData {
            context: format!("extracting {}", archive.display()),
            message: format!("archive entry path was invalid: {err}"),
        })?;
        let relative_path = validate_archive_entry_path(&entry_path, archive)?;
        let destination = target_dir.join(relative_path);
        if entry_type.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|err| io_context(format!("creating {}", destination.display()), err))?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| io_context(format!("creating {}", parent.display()), err))?;
            }
            entry.unpack(&destination).map_err(|err| {
                io_context(
                    format!(
                        "extracting {} to {}",
                        archive.display(),
                        destination.display()
                    ),
                    err,
                )
            })?;
        }
    }
    Ok(())
}

fn validate_archive_entry_path(path: &Path, archive: &Path) -> LauncherResult<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(invalid_archive_path(archive, path, "entry path was empty"));
    }

    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(invalid_archive_path(
            archive,
            path,
            "entry path used a Windows prefix",
        ));
    }
    if text.starts_with("\\\\") {
        return Err(invalid_archive_path(
            archive,
            path,
            "entry path used a Windows prefix",
        ));
    }

    let mut has_component = false;
    for component in path.components() {
        match component {
            Component::Prefix(_) => {
                return Err(invalid_archive_path(
                    archive,
                    path,
                    "entry path used a Windows prefix",
                ));
            }
            Component::RootDir => {
                return Err(invalid_archive_path(
                    archive,
                    path,
                    "entry path was absolute",
                ));
            }
            Component::ParentDir => {
                return Err(invalid_archive_path(
                    archive,
                    path,
                    "entry path contained a parent traversal",
                ));
            }
            Component::CurDir => {}
            Component::Normal(_) => {
                has_component = true;
            }
        }
    }

    if !has_component {
        return Err(invalid_archive_path(archive, path, "entry path was empty"));
    }

    Ok(path.to_path_buf())
}

fn invalid_archive_path(archive: &Path, path: &Path, message: &str) -> LauncherError {
    LauncherError::InvalidData {
        context: format!("extracting {}", archive.display()),
        message: format!("{message}: {}", path.display()),
    }
}

pub fn platform_library_name(platform: crate::models::Platform) -> &'static str {
    match platform {
        crate::models::Platform::Windows => "version.dll",
        crate::models::Platform::MacOs => "libstfc-community-mod.dylib",
    }
}

pub fn install_staged_library(
    staging_dir: &Path,
    mods_dir: &Path,
    platform: crate::models::Platform,
) -> LauncherResult<PathBuf> {
    let library_name = platform_library_name(platform);
    let replacement = staging_dir.join(library_name);
    let metadata = fs::symlink_metadata(&replacement).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            LauncherError::InvalidData {
                context: format!("installing {}", library_name),
                message: format!(
                    "staged platform library was missing at {}",
                    replacement.display()
                ),
            }
        } else {
            io_context(format!("checking {}", replacement.display()), err)
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(LauncherError::InvalidData {
            context: format!("installing {}", library_name),
            message: format!(
                "staged platform library was not a regular file at {}",
                replacement.display()
            ),
        });
    }

    fs::create_dir_all(mods_dir)
        .map_err(|err| io_context(format!("creating {}", mods_dir.display()), err))?;
    let target = mods_dir.join(library_name);

    let mut source = fs::File::open(&replacement)
        .map_err(|err| io_context(format!("opening {}", replacement.display()), err))?;
    let mut temp_file = NamedTempFile::new_in(mods_dir).map_err(|err| {
        io_context(
            format!("creating temporary file in {}", mods_dir.display()),
            err,
        )
    })?;
    std::io::copy(&mut source, &mut temp_file).map_err(|err| {
        io_context(
            format!(
                "copying {} to {}",
                replacement.display(),
                temp_file.path().display()
            ),
            err,
        )
    })?;
    temp_file
        .flush()
        .map_err(|err| io_context(format!("flushing {}", temp_file.path().display()), err))?;
    temp_file
        .as_file()
        .sync_all()
        .map_err(|err| io_context(format!("syncing {}", temp_file.path().display()), err))?;
    temp_file.into_temp_path().persist(&target).map_err(|err| {
        let tempfile::PathPersistError { error, path } = err;
        io_context(
            format!("persisting {} to {}", path.display(), target.display()),
            error,
        )
    })?;
    sync_directory(mods_dir)?;

    Ok(target)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> LauncherResult<()> {
    let dir = fs::File::open(path)
        .map_err(|err| io_context(format!("opening directory {}", path.display()), err))?;
    match dir.sync_all() {
        Ok(()) => Ok(()),
        Err(err) if matches!(err.kind(), ErrorKind::Unsupported | ErrorKind::InvalidInput) => {
            Ok(())
        }
        Err(err) => Err(io_context(
            format!("syncing directory {}", path.display()),
            err,
        )),
    }
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> LauncherResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_sha256_file_with_filename() {
        let parsed = parse_sha256("abcdef0123456789  stfc-community-mod-windows-x64.tar.zst\n")
            .expect("checksum");
        assert_eq!(parsed, "abcdef0123456789");
    }

    #[test]
    fn rejects_empty_sha256_file() {
        let error = parse_sha256("\n").expect_err("empty checksum rejected");
        assert!(error.to_string().contains("checksum file was empty"));
    }

    #[test]
    fn parses_uppercase_sha256_as_lowercase() {
        let parsed =
            parse_sha256("ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789\n")
                .expect("checksum");

        assert_eq!(
            parsed,
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        );
    }

    #[test]
    fn rejects_sha256_with_invalid_hex_characters() {
        let error =
            parse_sha256("abcdef0123456789abcdef0123456789abcdef0123456789abcdef012345678z\n")
                .expect_err("invalid checksum rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
    }

    #[test]
    fn hashes_file_larger_than_buffer() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("large.bin");
        let bytes: Vec<u8> = (0..70 * 1024).map(|index| (index % 251) as u8).collect();
        fs::write(&path, bytes).expect("write large file");

        let digest = sha256_file(&path).expect("hash file");

        assert_eq!(
            digest,
            "9f93b94cc515e1c04b96609b9c2f454b3d410f8eca0c81258fa36855957c7b86"
        );
    }

    #[test]
    fn extracts_single_file_from_tar_zst() {
        let root = tempfile::tempdir().expect("tempdir");
        let archive = root.path().join("mod.tar.zst");
        create_test_tar_zst(&archive, "version.dll", b"mod-bytes");
        let target = root.path().join("install");

        extract_mod_archive(&archive, &target).expect("extract");

        assert_eq!(
            std::fs::read(target.join("version.dll")).expect("read extracted"),
            b"mod-bytes"
        );
    }

    #[test]
    fn rejects_archive_entry_with_parent_traversal() {
        let root = tempfile::tempdir().expect("tempdir");
        let archive = root.path().join("mod.tar.zst");
        create_test_tar_zst_with_raw_file(&archive, b"../escape", b"escape");
        let target = root.path().join("install");

        let error = extract_mod_archive(&archive, &target).expect_err("traversal rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
        assert!(!root.path().join("escape").exists());
    }

    #[test]
    fn rejects_archive_entry_with_absolute_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let archive = root.path().join("mod.tar.zst");
        create_test_tar_zst_with_raw_file(&archive, b"/tmp/stfc-mod-escape", b"escape");
        let target = root.path().join("install");

        let error = extract_mod_archive(&archive, &target).expect_err("absolute path rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
    }

    #[test]
    fn rejects_archive_symlink_entry() {
        let root = tempfile::tempdir().expect("tempdir");
        let archive = root.path().join("mod.tar.zst");
        create_test_tar_zst_with_link(
            &archive,
            tar::EntryType::Symlink,
            "version.dll",
            "../outside-version.dll",
        );
        let target = root.path().join("install");

        let error = extract_mod_archive(&archive, &target).expect_err("symlink rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
        assert!(!target.join("version.dll").exists());
    }

    #[test]
    fn rejects_archive_hard_link_entry() {
        let root = tempfile::tempdir().expect("tempdir");
        let archive = root.path().join("mod.tar.zst");
        create_test_tar_zst_with_link(&archive, tar::EntryType::Link, "version.dll", "other.dll");
        let target = root.path().join("install");

        let error = extract_mod_archive(&archive, &target).expect_err("hard link rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
        assert!(!target.join("version.dll").exists());
    }

    #[test]
    fn installs_extracted_library_atomically() {
        let root = tempfile::tempdir().expect("tempdir");
        let staging = root.path().join("staging");
        let mods = root.path().join("mods");
        fs::create_dir_all(&staging).expect("staging dir");
        fs::write(staging.join("version.dll"), b"first-mod").expect("write first library");

        let installed = install_staged_library(&staging, &mods, crate::models::Platform::Windows)
            .expect("install first");

        assert_eq!(installed, mods.join("version.dll"));
        assert_eq!(
            fs::read(mods.join("version.dll")).expect("read installed"),
            b"first-mod"
        );

        fs::write(staging.join("version.dll"), b"second-mod").expect("write second library");
        let second = install_staged_library(&staging, &mods, crate::models::Platform::Windows)
            .expect("install second");

        assert_eq!(second, mods.join("version.dll"));
        assert_eq!(
            fs::read(mods.join("version.dll")).expect("read replaced"),
            b"second-mod"
        );
    }

    #[test]
    fn platform_library_names_match_supported_platforms() {
        assert_eq!(
            platform_library_name(crate::models::Platform::Windows),
            "version.dll"
        );
        assert_eq!(
            platform_library_name(crate::models::Platform::MacOs),
            "libstfc-community-mod.dylib"
        );
    }

    #[test]
    fn missing_platform_library_is_invalid_data() {
        let root = tempfile::tempdir().expect("tempdir");
        let staging = root.path().join("staging");
        let mods = root.path().join("mods");
        fs::create_dir_all(&staging).expect("staging dir");

        let error = install_staged_library(&staging, &mods, crate::models::Platform::Windows)
            .expect_err("missing library rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
        assert!(error.to_string().contains("version.dll"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_platform_library_is_invalid_data() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("tempdir");
        let staging = root.path().join("staging");
        let mods = root.path().join("mods");
        fs::create_dir_all(&staging).expect("staging dir");
        let external = root.path().join("outside-version.dll");
        fs::write(&external, b"external-bytes").expect("write external target");
        symlink(&external, staging.join("version.dll")).expect("create symlink");

        let error = install_staged_library(&staging, &mods, crate::models::Platform::Windows)
            .expect_err("symlink rejected");

        assert!(matches!(error, LauncherError::InvalidData { .. }));
        assert!(!mods.join("version.dll").exists());
    }

    fn create_test_tar_zst(archive: &Path, file_name: &str, bytes: &[u8]) {
        let file = fs::File::create(archive).expect("create archive");
        let encoder = zstd::Encoder::new(file, 0).expect("zstd encoder");
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, file_name, Cursor::new(bytes))
            .expect("append test file");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish zstd");
    }

    fn create_test_tar_zst_with_raw_file(archive: &Path, raw_path: &[u8], bytes: &[u8]) {
        let file = fs::File::create(archive).expect("create archive");
        let encoder = zstd::Encoder::new(file, 0).expect("zstd encoder");
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_old();
        assert!(raw_path.len() <= header.as_old().name.len());
        header.as_old_mut().name[..raw_path.len()].copy_from_slice(raw_path);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append(&header, Cursor::new(bytes))
            .expect("append raw test file");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish zstd");
    }

    fn create_test_tar_zst_with_link(
        archive: &Path,
        entry_type: tar::EntryType,
        path: &str,
        target: &str,
    ) {
        let file = fs::File::create(archive).expect("create archive");
        let encoder = zstd::Encoder::new(file, 0).expect("zstd encoder");
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(entry_type);
        header.set_size(0);
        header.set_mode(0o644);
        builder
            .append_link(&mut header, path, target)
            .expect("append test link");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish zstd");
    }
}
