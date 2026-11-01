#[cfg(any(windows, test))]
use crate::error::AppError;

#[cfg(windows)]
pub(crate) fn protect(bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    dpapi(bytes, true)
}

#[cfg(all(not(windows), test))]
pub(crate) fn protect(_bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    Err(AppError::configuration(
        "native Windows data protection is unavailable",
    ))
}

#[cfg(windows)]
pub(crate) fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    dpapi(bytes, false)
}

#[cfg(all(not(windows), test))]
pub(crate) fn unprotect(_bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    Err(AppError::configuration(
        "native Windows data protection is unavailable",
    ))
}

pub(crate) struct SecretBytes(pub(crate) Vec<u8>);

#[cfg(test)]
thread_local! {
    pub(crate) static WIPED_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl SecretBytes {
    pub(crate) fn into_inner(mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

pub(crate) fn wipe(bytes: &mut [u8]) {
    #[cfg(test)]
    let length = bytes.len();
    for byte in bytes {
        // SAFETY: each byte is a live, exclusively borrowed u8; zero is a valid value.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    #[cfg(test)]
    WIPED_BYTES.with(|count| count.set(count.get() + length));
}

#[cfg(windows)]
fn dpapi(bytes: &[u8], protecting: bool) -> Result<Vec<u8>, AppError> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    struct LocalBlob {
        blob: CRYPT_INTEGER_BLOB,
        wipe: bool,
    }
    impl Drop for LocalBlob {
        fn drop(&mut self) {
            if !self.blob.pbData.is_null() {
                if self.wipe && self.blob.cbData as usize <= isize::MAX as usize {
                    // SAFETY: DPAPI owns this LocalAlloc output until LocalFree below; cbData
                    // is its initialized byte length, fits a Rust slice, and this guard has
                    // exclusive access. u8 requires no extra alignment or validity checks.
                    wipe(unsafe {
                        std::slice::from_raw_parts_mut(self.blob.pbData, self.blob.cbData as usize)
                    });
                }
                // SAFETY: DPAPI allocates pbData with LocalAlloc; this guard frees it once,
                // after its last use, with the matching allocator.
                if !unsafe { LocalFree(self.blob.pbData.cast()) }.is_null() {
                    use std::io::Write;
                    let _ = writeln!(std::io::stderr(), "Windows protected-data cleanup failed");
                }
            }
        }
    }

    if bytes.is_empty() || bytes.len() > u32::MAX as usize {
        return Err(AppError::configuration(
            "Windows data protection input size is invalid",
        ));
    }
    let mut input = SecretBytes(bytes.to_vec());
    let blob = CRYPT_INTEGER_BLOB {
        cbData: input.0.len() as u32,
        pbData: input.0.as_mut_ptr(),
    };
    let mut output = LocalBlob {
        blob: CRYPT_INTEGER_BLOB::default(),
        wipe: !protecting,
    };
    let succeeded = if protecting {
        // SAFETY: input owns cbData initialized bytes for this synchronous call;
        // output is a writable initialized blob. Null optional pointers are allowed,
        // UI is forbidden, and LocalBlob owns the returned LocalAlloc allocation.
        unsafe {
            CryptProtectData(
                &blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.blob,
            )
        }
    } else {
        // SAFETY: input owns the initialized ciphertext for this synchronous call;
        // output is writable, the null description suppresses a second allocation,
        // optional pointers may be null, and LocalBlob owns/wipes the plaintext.
        unsafe {
            CryptUnprotectData(
                &blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.blob,
            )
        }
    };
    drop(input);
    if succeeded == 0
        || output.blob.pbData.is_null()
        || output.blob.cbData == 0
        || output.blob.cbData as usize > isize::MAX as usize
    {
        return Err(AppError::configuration("Windows data protection failed"));
    }
    // SAFETY: successful DPAPI returned a nonnull allocation of cbData initialized bytes;
    // the checked length fits a Rust slice, u8 has alignment one, and output remains
    // alive throughout the copy and frees it only afterward.
    Ok(unsafe {
        std::slice::from_raw_parts(output.blob.pbData, output.blob.cbData as usize).to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipes_live_bytes_and_transfers_owned_bytes_without_wiping_them() {
        let mut bytes = b"secret".to_vec();
        wipe(&mut bytes);
        assert_eq!(bytes, [0; 6]);
        assert_eq!(SecretBytes(b"owned".to_vec()).into_inner(), b"owned");
        assert!(SecretBytes(Vec::new()).into_inner().is_empty());
        let before = WIPED_BYTES.with(|count| count.get());
        drop(SecretBytes(b"discard".to_vec()));
        assert_eq!(WIPED_BYTES.with(|count| count.get()) - before, 7);
        let unwind = std::panic::catch_unwind(|| {
            let _secret = SecretBytes(b"unwind".to_vec());
            panic!("injected secret owner unwind");
        });
        assert!(unwind.is_err());
        assert_eq!(WIPED_BYTES.with(|count| count.get()) - before, 13);
    }
}
