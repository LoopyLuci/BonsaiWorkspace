//! Atomic self-hosting binary replacement.
//!
//! `replace_running_binary(new_path)` swaps the current executable for a new
//! one without a window where neither binary exists.  After the rename the
//! caller should re-exec (`std::process::Command::new(current_exe)`) to
//! activate the new binary.

use std::path::Path;

/// Atomically replace the running binary with `new_path`.
///
/// On POSIX the kernel keeps the old inode alive until the last fd is closed,
/// so the running process is unaffected; only new `exec()` calls see the new
/// binary.
///
/// On Windows `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` is used.
pub fn replace_running_binary(new_path: &Path) -> std::io::Result<()> {
    let current_exe = std::env::current_exe()?;
    atomic_replace(new_path, &current_exe)
}

/// Atomically rename `src` to `dst`, replacing `dst` if it exists.
pub fn atomic_replace(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        // On POSIX, rename(2) is atomic on the same filesystem.
        std::fs::rename(src, dst)?;
        // Ensure the binary is executable.
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(dst)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(dst, perms)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        fn to_wide(p: &Path) -> Vec<u16> {
            p.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        let src_w = to_wide(src);
        let dst_w = to_wide(dst);

        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

        let ok = unsafe {
            windows_move_file_ex(
                src_w.as_ptr(),
                dst_w.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Re-exec the current process with the new binary, passing all existing args.
/// This function does not return on success.
pub fn reexec_self() -> std::io::Result<std::convert::Infallible> {
    let exe = std::env::current_exe()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let err = std::process::Command::new(&exe).args(&args).spawn()?.wait();
    // If we reach here, something went wrong starting the child.
    Err(std::io::Error::other(format!("re-exec failed: {:?}", err)))
}

// ── Windows shim (only compiled on Windows) ───────────────────────────────────

#[cfg(windows)]
extern "system" {
    fn MoveFileExW(lpExistingFileName: *const u16, lpNewFileName: *const u16, dwFlags: u32) -> i32;
}

#[cfg(windows)]
unsafe fn windows_move_file_ex(src: *const u16, dst: *const u16, flags: u32) -> i32 {
    MoveFileExW(src, dst, flags)
}
