use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

use super::settings::durable_write;

trait Backend {
    fn active_scheme(&mut self) -> Result<Uuid, AppError>;
    fn read(&mut self, scheme: Uuid, battery: bool) -> Result<u32, AppError>;
    fn write(&mut self, scheme: Uuid, battery: bool, value: u32) -> Result<(), AppError>;
    fn refresh(&mut self, scheme: Uuid) -> Result<(), AppError>;
    fn execution(&mut self, busy: bool) -> Result<(), AppError>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Original {
    scheme: Uuid,
    ac: u32,
    dc: u32,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    originals: Vec<Original>,
}

struct Policy<B: Backend> {
    backend: B,
    path: PathBuf,
    journal: Journal,
    loaded: bool,
    applied: Option<Uuid>,
}

impl<B: Backend> Policy<B> {
    fn new(backend: B, directory: PathBuf) -> Self {
        Self {
            backend,
            path: directory.join("windows-lid-journal.json"),
            journal: Journal::default(),
            loaded: false,
            applied: None,
        }
    }

    fn load(&mut self) -> Result<(), AppError> {
        if self.loaded {
            return Ok(());
        }
        self.journal = match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                AppError::configuration(format!("Invalid Windows lid recovery journal: {error}"))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Journal::default(),
            Err(error) => return Err(AppError::io("read lid recovery journal", &self.path, error)),
        };
        let mut schemes = std::collections::HashSet::new();
        if self
            .journal
            .originals
            .iter()
            .any(|original| !schemes.insert(original.scheme))
        {
            return Err(AppError::configuration(
                "Duplicate scheme in lid recovery journal",
            ));
        }
        self.loaded = true;
        Ok(())
    }

    fn restore_original(&mut self, original: &Original) -> Result<(), AppError> {
        let mut errors = Vec::new();
        let mut expected = Vec::new();
        for (battery, value) in [(false, original.ac), (true, original.dc)] {
            match self.backend.read(original.scheme, battery) {
                Ok(current) => {
                    // Restore only our applied value; a different value belongs to an external editor.
                    let target = if current == 0 { value } else { current };
                    expected.push((battery, target));
                    if current == 0 && value != 0 {
                        if let Err(error) = self.backend.write(original.scheme, battery, value) {
                            errors.push(error.to_string());
                        }
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
        }
        if let Err(error) = self.backend.refresh(original.scheme) {
            errors.push(error.to_string());
        }
        for (battery, value) in expected {
            match self.backend.read(original.scheme, battery) {
                Ok(actual) if actual == value => {}
                Ok(actual) => errors.push(format!(
                    "Lid restore readback mismatch: expected {value}, found {actual}"
                )),
                Err(error) => errors.push(error.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::configuration(errors.join("; ")))
        }
    }

    fn restore(&mut self) -> Result<(), AppError> {
        self.load()?;
        let mut unresolved = Vec::new();
        let mut errors = Vec::new();
        for original in self.journal.originals.clone() {
            if let Err(error) = self.restore_original(&original) {
                errors.push(error.to_string());
                unresolved.push(original);
            }
        }
        if unresolved.len() != self.journal.originals.len() {
            // Keep the old journal in memory if persisting progress fails; replay is conditional.
            durable_write(
                &self.path,
                &Journal {
                    originals: unresolved.clone(),
                },
            )?;
            self.journal.originals = unresolved;
        }
        self.applied = None;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::configuration(format!(
                "Windows lid policy restoration failed: {}",
                errors.join("; ")
            )))
        }
    }

    fn prepare(&mut self) -> Result<(), AppError> {
        self.restore()
    }

    fn apply(&mut self) -> Result<(), AppError> {
        self.load()?;
        let scheme = self.backend.active_scheme()?;
        if self.applied == Some(scheme) {
            if self.backend.read(scheme, false)? != 0 || self.backend.read(scheme, true)? != 0 {
                return Err(AppError::configuration(
                    "Lid policy changed externally; protection was not renewed",
                ));
            }
            self.backend.execution(true)?;
            if self.backend.active_scheme()? != scheme {
                return Err(AppError::configuration(
                    "Active power scheme changed during renewal",
                ));
            }
            return Ok(());
        }
        self.restore()?;
        if self.backend.active_scheme()? != scheme {
            return Err(AppError::configuration(
                "Active power scheme changed while preparing lid protection",
            ));
        }
        let original = Original {
            scheme,
            ac: self.backend.read(scheme, false)?,
            dc: self.backend.read(scheme, true)?,
        };
        let journal = Journal {
            originals: vec![original],
        };
        durable_write(&self.path, &journal)?;
        self.journal = journal;
        let result = (|| {
            self.backend.write(scheme, false, 0)?;
            self.backend.write(scheme, true, 0)?;
            self.backend.refresh(scheme)?;
            if self.backend.active_scheme()? != scheme
                || self.backend.read(scheme, false)? != 0
                || self.backend.read(scheme, true)? != 0
            {
                return Err(AppError::configuration(
                    "Lid protection readback failed or active scheme changed",
                ));
            }
            self.backend.execution(true)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.applied = Some(scheme);
                Ok(())
            }
            Err(error) => match self.idle() {
                Ok(()) => Err(error),
                Err(restore) => Err(AppError::configuration(format!(
                    "{error}; rollback failed: {restore}"
                ))),
            },
        }
    }

    fn idle(&mut self) -> Result<(), AppError> {
        let execution = self.backend.execution(false);
        let restored = self.restore();
        match (execution, restored) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(execution), Err(restore)) => {
                Err(AppError::configuration(format!("{execution}; {restore}")))
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) struct NativeController {
    policy: Policy<NativeBackend>,
}

#[cfg(target_os = "windows")]
impl NativeController {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            policy: Policy::new(NativeBackend, directory),
        }
    }
}

#[cfg(target_os = "windows")]
impl super::PowerController for NativeController {
    fn prepare(&mut self) -> Result<(), AppError> {
        self.policy.prepare()
    }
    fn busy(&mut self, ttl: std::time::Duration) -> Result<(), AppError> {
        if ttl.is_zero() || ttl > super::LEASE {
            return Err(AppError::validation("Invalid lid protection lease"));
        }
        // Windows has no policy TTL; the serialized app timer restores on deadline/exit.
        self.policy.apply()
    }
    fn idle(&mut self) -> Result<(), AppError> {
        self.policy.idle()
    }
    fn stop(&mut self) -> Result<(), AppError> {
        self.policy.idle()
    }
}

#[cfg(target_os = "windows")]
struct NativeBackend;

#[cfg(target_os = "windows")]
mod ffi {
    pub use windows_sys::Win32::System::SystemServices::{
        GUID_LIDCLOSE_ACTION as LID, GUID_SYSTEM_BUTTON_SUBGROUP as BUTTONS,
    };
}

#[cfg(target_os = "windows")]
fn win_result(code: u32, context: &str) -> Result<(), AppError> {
    if code == 0 {
        Ok(())
    } else {
        Err(AppError::configuration(format!(
            "{context}: Windows error {code}"
        )))
    }
}

#[cfg(target_os = "windows")]
impl Backend for NativeBackend {
    fn active_scheme(&mut self) -> Result<Uuid, AppError> {
        use windows_sys::Win32::{Foundation::LocalFree, System::Power::PowerGetActiveScheme};
        let mut pointer = std::ptr::null_mut();
        let code = unsafe { PowerGetActiveScheme(std::ptr::null_mut(), &mut pointer) };
        if code != 0 {
            if !pointer.is_null() {
                unsafe {
                    LocalFree(pointer.cast());
                }
            }
            win_result(code, "Get active power scheme")?;
        }
        if pointer.is_null() {
            return Err(AppError::configuration(
                "Windows returned no active power scheme",
            ));
        }
        let guid = unsafe { *pointer };
        unsafe {
            LocalFree(pointer.cast());
        }
        Ok(Uuid::from_fields(
            guid.data1,
            guid.data2,
            guid.data3,
            &guid.data4,
        ))
    }
    fn read(&mut self, scheme: Uuid, battery: bool) -> Result<u32, AppError> {
        use windows_sys::Win32::System::Power::{PowerReadACValueIndex, PowerReadDCValueIndex};
        let guid = windows_sys::core::GUID::from_u128(scheme.as_u128());
        let mut value = 0;
        let code = unsafe {
            if battery {
                PowerReadDCValueIndex(
                    std::ptr::null_mut(),
                    &guid,
                    &ffi::BUTTONS,
                    &ffi::LID,
                    &mut value,
                )
            } else {
                PowerReadACValueIndex(
                    std::ptr::null_mut(),
                    &guid,
                    &ffi::BUTTONS,
                    &ffi::LID,
                    &mut value,
                )
            }
        };
        win_result(code, "Read lid action")?;
        Ok(value)
    }
    fn write(&mut self, scheme: Uuid, battery: bool, value: u32) -> Result<(), AppError> {
        use windows_sys::Win32::System::Power::{PowerWriteACValueIndex, PowerWriteDCValueIndex};
        let guid = windows_sys::core::GUID::from_u128(scheme.as_u128());
        let code = unsafe {
            if battery {
                PowerWriteDCValueIndex(std::ptr::null_mut(), &guid, &ffi::BUTTONS, &ffi::LID, value)
            } else {
                PowerWriteACValueIndex(std::ptr::null_mut(), &guid, &ffi::BUTTONS, &ffi::LID, value)
            }
        };
        win_result(code, "Write lid action")
    }
    fn refresh(&mut self, scheme: Uuid) -> Result<(), AppError> {
        use windows_sys::Win32::System::Power::PowerSetActiveScheme;
        // Never activate an old scheme while restoring it.
        if self.active_scheme()? == scheme {
            let guid = windows_sys::core::GUID::from_u128(scheme.as_u128());
            win_result(
                unsafe { PowerSetActiveScheme(std::ptr::null_mut(), &guid) },
                "Refresh active power scheme",
            )?;
        }
        Ok(())
    }
    fn execution(&mut self, busy: bool) -> Result<(), AppError> {
        use windows_sys::Win32::System::Power::{
            SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
        };
        let flags = ES_CONTINUOUS | if busy { ES_SYSTEM_REQUIRED } else { 0 };
        if unsafe { SetThreadExecutionState(flags) } == 0 {
            Err(AppError::configuration(format!(
                "Set idle execution request: {}",
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
