//! Linux shared-memory primitives for the DMI ring channel.
//!
//! Uses `memfd_create` to allocate an anonymous, unlinkable memory region
//! backed by the kernel's page cache. The file descriptor can be passed to a
//! child process via SCM_RIGHTS, enabling true zero-copy IPC.

use std::io;
use std::os::unix::io::{FromRawFd, OwnedFd};

/// Create an anonymous shared-memory region of `size` bytes using `memfd_create`.
///
/// The region is sealed immediately after creation (no further `ftruncate`
/// calls) so that the receiver can memory-map it read-only without races.
///
/// Returns the `OwnedFd` for the memfd. The caller is responsible for
/// `mmap`-ing it on both sides.
pub fn memfd_create_ring(size: usize) -> io::Result<OwnedFd> {
    use std::ffi::CString;
    let name = CString::new("bonsai-dmi-ring").unwrap();
    let fd = unsafe {
        // MFD_CLOEXEC | MFD_ALLOW_SEALING
        libc::syscall(libc::SYS_memfd_create, name.as_ptr(), 0x0001u32 | 0x0002u32) as i32
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let owned = unsafe { OwnedFd::from_raw_fd(fd) };

    // Extend the memfd to the requested size.
    let rc = unsafe { libc::ftruncate(fd, size as libc::off_t) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }

    // Seal: no further size changes allowed.
    unsafe {
        // F_ADD_SEALS = 1033, F_SEAL_SHRINK = 0x0002, F_SEAL_GROW = 0x0004
        libc::fcntl(fd, 1033, 0x0002 | 0x0004);
    }

    Ok(owned)
}

/// Map a memfd into the process's virtual address space as read-write.
///
/// # Safety
/// The returned pointer is valid for `size` bytes and must be unmapped with
/// `libc::munmap` when no longer needed.
pub unsafe fn mmap_memfd(fd: i32, size: usize) -> io::Result<*mut u8> {
    let ptr = libc::mmap(
        std::ptr::null_mut(),
        size,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED,
        fd,
        0,
    );
    if ptr == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    Ok(ptr as *mut u8)
}

/// Unmap a previously mapped region.
///
/// # Safety
/// `ptr` must have been returned by [`mmap_memfd`] with the same `size`.
pub unsafe fn unmap_region(ptr: *mut u8, size: usize) {
    libc::munmap(ptr as *mut libc::c_void, size);
}
