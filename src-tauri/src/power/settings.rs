use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    pub enabled: bool,
}

pub(super) fn load(directory: &Path) -> Result<Settings, AppError> {
    let path = directory.join("power.json");
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| AppError::configuration(format!("Invalid power.json: {error}"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(error) => Err(AppError::io("read power preference", &path, error)),
    }
}

pub(super) fn save(directory: &Path, enabled: bool) -> Result<(), AppError> {
    durable_write(&directory.join("power.json"), &Settings { enabled })
}

pub(super) fn durable_write(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    let directory = path
        .parent()
        .ok_or_else(|| AppError::validation("Missing journal directory"))?;
    fs::create_dir_all(directory)?;
    let bytes = serde_json::to_vec(value)
        .map_err(|error| AppError::configuration(format!("Serialize power data: {error}")))?;
    let temporary = path.with_extension("pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(AppError::io(
                "persist power data",
                path,
                std::io::Error::last_os_error(),
            ));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(&temporary, path)?;
        fs::File::open(directory)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
