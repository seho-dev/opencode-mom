use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::protected_data::SecretBytes;

use super::{failure, nullable_string, password_valid, validate_selection, AppError, Selection};

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
    let normalized = validate_selection(Selection {
        url: selection.url.clone(),
        password: None,
    })?;
    let encrypted_password = selection
        .password
        .as_deref()
        .map(|password| {
            if normalized.url.is_none() {
                return Err(failure("shared target cannot contain a password"));
            }
            if password.is_empty() || !password_valid(password) {
                return Err(failure("invalid configured password"));
            }
            let bytes =
                protect(password.as_bytes()).map_err(|_| failure("password protection failed"))?;
            if bytes.is_empty() || bytes.len() > MAX_CIPHER_BYTES {
                return Err(failure("invalid protected password size"));
            }
            Ok(bytes)
        })
        .transpose()?;
    let bytes = serde_json::to_vec(&WindowsSelection {
        url: normalized.url,
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
        let plaintext =
            SecretBytes(unprotect(&ciphertext).map_err(|_| failure("password decryption failed"))?);
        if plaintext.0.len() > MAX_PASSWORD_BYTES {
            return Err(failure("invalid decrypted password size"));
        }
        let password =
            std::str::from_utf8(&plaintext.0).map_err(|_| failure("invalid decrypted password"))?;
        if password.is_empty() || !password_valid(password) {
            return Err(failure("invalid decrypted password"));
        }
        selection.password = Some(password.to_owned());
    }
    Ok(selection)
}
