use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Preserve the last successfully written configuration before replacing it.
/// A missing source is allowed so first-run setup remains simple.
pub fn backup_existing(path: &Path) -> io::Result<Option<std::path::PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let backup = path.with_extension("bak");
    let mut source = fs::File::open(path)?;
    let mut target = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&backup)?;
    io::copy(&mut source, &mut target)?;
    target.sync_all()?;
    Ok(Some(backup))
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with_mode(path, bytes, None)
}

/// Write a secret without creating a backup and apply restrictive permissions
/// where the host operating system exposes POSIX file modes.
pub fn write_secret_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with_mode(path, bytes, Some(0o600))
}

/// Read a secret and repair permissive POSIX mode bits before returning it.
/// Windows uses the ACL inherited from the data directory.
pub fn read_secret(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "secret path is not a regular file",
            ));
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }
    fs::read(path)
}

fn write_atomic_with_mode(path: &Path, bytes: &[u8], _mode: Option<u32>) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let temporary = path.with_extension("station-tmp");
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        if let Some(mode) = _mode {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }
        let mut f = options.open(&temporary)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        replace(&temporary, path)?;
        #[cfg(unix)]
        if let Some(mode) = _mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
#[cfg(not(windows))]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            a.as_ptr(),
            b.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backs_up_existing_file_and_allows_missing_file() {
        let root = crate::test_temp_dir("ultimatepr-storage");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("station.conf");
        assert_eq!(backup_existing(&path).unwrap(), None);
        fs::write(&path, b"callsign=SP1ABC\n").unwrap();
        let backup = backup_existing(&path).unwrap().unwrap();
        assert_eq!(fs::read(backup).unwrap(), b"callsign=SP1ABC\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn secret_round_trip_preserves_exact_bytes() {
        let root = crate::test_temp_dir("ultimatepr-secret-round-trip");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("test.secret");
        let secret = "Pąss-42_Ż".as_bytes();
        write_secret_atomic(&path, secret).unwrap();
        assert_eq!(read_secret(&path).unwrap(), secret);
        let _ = fs::remove_dir_all(root);
    }
}
