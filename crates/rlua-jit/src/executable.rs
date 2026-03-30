use std::io;
use std::ptr::{self, NonNull};

use crate::JitError;

#[derive(Debug)]
pub(crate) struct ExecutableBuffer {
    ptr: NonNull<u8>,
    mapped_len: usize,
    #[allow(dead_code)]
    code_len: usize,
}

impl ExecutableBuffer {
    pub(crate) fn install(code: &[u8]) -> Result<Self, JitError> {
        if code.is_empty() {
            return Err(JitError::ExecutableBuffer(
                "cannot install an empty native trace".to_string(),
            ));
        }

        let page_size = page_size()?;
        let mapped_len = round_up_to_page(code.len(), page_size);
        let ptr = map_writable_pages(mapped_len)?;

        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), ptr.as_ptr(), code.len());
        }

        protect_executable(ptr, mapped_len)?;

        Ok(Self {
            ptr,
            mapped_len,
            code_len: code.len(),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr() as *const u8
    }

    #[allow(dead_code)]
    pub(crate) fn code_len(&self) -> usize {
        self.code_len
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr.as_ptr() as *mut libc::c_void, self.mapped_len);
        }
    }
}

fn page_size() -> Result<usize, JitError> {
    let raw = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if raw <= 0 {
        return Err(JitError::ExecutableBuffer(format!(
            "failed to query page size: {}",
            io::Error::last_os_error()
        )));
    }

    Ok(raw as usize)
}

fn round_up_to_page(len: usize, page_size: usize) -> usize {
    let remainder = len % page_size;
    if remainder == 0 {
        len
    } else {
        len + (page_size - remainder)
    }
}

fn map_writable_pages(len: usize) -> Result<NonNull<u8>, JitError> {
    #[allow(unused_mut)]
    let mut flags = libc::MAP_PRIVATE | libc::MAP_ANON;

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        flags |= libc::MAP_JIT;
    }

    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            flags,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return Err(JitError::ExecutableBuffer(format!(
            "mmap failed: {}",
            io::Error::last_os_error()
        )));
    }

    Ok(unsafe { NonNull::new_unchecked(ptr as *mut u8) })
}

fn protect_executable(ptr: NonNull<u8>, len: usize) -> Result<(), JitError> {
    let status = unsafe {
        libc::mprotect(
            ptr.as_ptr() as *mut libc::c_void,
            len,
            libc::PROT_READ | libc::PROT_EXEC,
        )
    };

    if status != 0 {
        unsafe {
            libc::munmap(ptr.as_ptr() as *mut libc::c_void, len);
        }
        return Err(JitError::ExecutableBuffer(format!(
            "mprotect failed: {}",
            io::Error::last_os_error()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_buffer_installs_machine_code_bytes() {
        let code = [0xC3u8];
        let buffer = ExecutableBuffer::install(&code).unwrap();
        let installed =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr(), buffer.code_len()) }.to_vec();

        assert_eq!(installed, code);
        assert_eq!(buffer.code_len(), 1);
    }
}
