use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::{failure, nullable_string, validate_selection, AppError, Selection};

pub(super) const MAX_FILE_BYTES: usize = 32 * 1024;
const MAX_CIPHER_BYTES: usize = 8192;
const MAX_PASSWORD_BYTES: usize = 4096;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WindowsSelection {
    #[serde(deserialize_with = "nullable_string")]
    url: Option<String>,
    #[serde(deserialize_with = "encrypted_bytes")]
    encrypted_password: Option<Vec<u8>>,
}

fn encrypted_bytes<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<u8>>, D::Error> {
    struct Bytes(Vec<u8>);
    impl<'de> Deserialize<'de> for Bytes {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct BoundedBytes;
            impl<'de> Visitor<'de> for BoundedBytes {
                type Value = Bytes;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a bounded nonempty encrypted byte array")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Bytes, A::Error> {
                    let mut bytes = Vec::new();
                    while let Some(byte) = sequence.next_element::<u8>()? {
                        if bytes.len() == MAX_CIPHER_BYTES {
                            return Err(de::Error::custom(
                                "encrypted password byte limit exceeded",
                            ));
                        }
                        bytes.push(byte);
                    }
                    if bytes.is_empty() {
                        return Err(de::Error::custom("empty encrypted password"));
                    }
                    Ok(Bytes(bytes))
                }
            }
            deserializer.deserialize_seq(BoundedBytes)
        }
    }
    Option::<Bytes>::deserialize(deserializer).map(|bytes| bytes.map(|bytes| bytes.0))
}

pub(super) fn encode(
    selection: &Selection,
    protect: impl FnOnce(&[u8]) -> Result<Vec<u8>, AppError>,
) -> Result<Vec<u8>, AppError> {
    let selection = validate_selection(selection.clone())?;
    let encrypted_password = selection
        .password
        .as_deref()
        .map(|password| {
            let bytes =
                protect(password.as_bytes()).map_err(|_| failure("password protection failed"))?;
            if bytes.is_empty() || bytes.len() > MAX_CIPHER_BYTES {
                return Err(failure("invalid protected password size"));
            }
            Ok(bytes)
        })
        .transpose()?;
    let bytes = serde_json::to_vec(&WindowsSelection {
        url: selection.url,
        encrypted_password,
    })
    .map_err(|_| failure("configuration serialization failed"))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(failure("configuration byte limit exceeded"));
    }
    Ok(bytes)
}

pub(super) fn decode(
    bytes: &[u8],
    unprotect: impl FnOnce(&[u8]) -> Result<Vec<u8>, AppError>,
) -> Result<Selection, AppError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(failure("configuration byte limit exceeded"));
    }
    let disk: super::super::active::JsonObject<WindowsSelection> =
        serde_json::from_slice(bytes).map_err(|_| failure("invalid encrypted configuration"))?;
    let disk = disk.0;
    let mut selection = validate_selection(Selection {
        url: disk.url,
        password: None,
    })?;
    if let Some(ciphertext) = disk.encrypted_password {
        if selection.url.is_none() {
            return Err(failure("shared target cannot contain a password"));
        }
        let mut plaintext =
            unprotect(&ciphertext).map_err(|_| failure("password decryption failed"))?;
        let password = if plaintext.len() > MAX_PASSWORD_BYTES {
            Err(failure("invalid decrypted password size"))
        } else {
            std::str::from_utf8(&plaintext)
                .map(str::to_owned)
                .map_err(|_| failure("invalid decrypted password"))
        };
        wipe(&mut plaintext);
        selection.password = Some(password?);
    }
    validate_selection(selection)
}

fn wipe(bytes: &mut [u8]) {
    for byte in bytes {
        // Volatile writes keep temporary plaintext clearing from being optimized away.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

#[cfg(windows)]
pub(super) fn protect(bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    dpapi(bytes, true)
}

#[cfg(windows)]
pub(super) fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    dpapi(bytes, false)
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
                if self.wipe {
                    wipe(unsafe {
                        std::slice::from_raw_parts_mut(self.blob.pbData, self.blob.cbData as usize)
                    });
                }
                unsafe { LocalFree(self.blob.pbData.cast()) };
            }
        }
    }

    let input_limit = if protecting {
        MAX_PASSWORD_BYTES
    } else {
        MAX_CIPHER_BYTES
    };
    if bytes.is_empty() || bytes.len() > input_limit {
        return Err(failure("invalid data protection input size"));
    }
    let mut input = bytes.to_vec();
    let blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_mut_ptr(),
    };
    let mut output = LocalBlob {
        blob: CRYPT_INTEGER_BLOB::default(),
        wipe: false,
    };
    // Default user scope: no LOCAL_MACHINE, optional entropy, prompt, or description output.
    let succeeded = unsafe {
        if protecting {
            CryptProtectData(
                &blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.blob,
            )
        } else {
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
    wipe(&mut input);
    let output_limit = if protecting {
        MAX_CIPHER_BYTES
    } else {
        MAX_PASSWORD_BYTES
    };
    if succeeded == 0
        || output.blob.pbData.is_null()
        || output.blob.cbData == 0
        || output.blob.cbData as usize > output_limit
    {
        return Err(failure("Windows data protection failed"));
    }
    output.wipe = !protecting;
    Ok(
        unsafe { std::slice::from_raw_parts(output.blob.pbData, output.blob.cbData as usize) }
            .to_vec(),
    )
}
