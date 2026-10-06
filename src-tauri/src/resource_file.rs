use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub path: PathBuf,
    pub aliases: Vec<PathBuf>,
    pub content: String,
    identity: fs::Metadata,
}

impl Snapshot {
    pub fn read(logical: &Path) -> Result<Option<Self>, AppError> {
        Self::read_with(logical, |_| {})
    }

    pub fn read_with(
        logical: &Path,
        after_resolve: impl FnOnce(&Path),
    ) -> Result<Option<Self>, AppError> {
        Self::read_inner(logical, after_resolve, |_| {})
    }

    fn read_inner(
        logical: &Path,
        after_resolve: impl FnOnce(&Path),
        before_read: impl FnOnce(&Path),
    ) -> Result<Option<Self>, AppError> {
        let path = match fs::canonicalize(logical) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                absent(logical)?;
                return Ok(None);
            }
            Err(error) => return Err(AppError::io("resolve", logical, error)),
        };
        after_resolve(logical);
        let identity = regular_file(&path)?;
        before_read(&path);
        let content =
            fs::read_to_string(&path).map_err(|error| AppError::io("read", &path, error))?;
        validate_source(logical, &path)?;
        validate_metadata(&path, &identity)?;
        Ok(Some(Self {
            path,
            aliases: vec![logical.to_owned()],
            content,
            identity,
        }))
    }

    pub fn validate(&self) -> Result<(), AppError> {
        for alias in &self.aliases {
            validate_source(alias, &self.path)?;
        }
        validate_metadata(&self.path, &self.identity)?;
        let current = fs::read_to_string(&self.path)
            .map_err(|error| AppError::io("read", &self.path, error))?;
        for alias in &self.aliases {
            validate_source(alias, &self.path)?;
        }
        validate_metadata(&self.path, &self.identity)?;
        if current != self.content {
            return Err(changed(&self.path));
        }
        Ok(())
    }
}

pub(crate) fn absent(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io("inspect", path, error)),
        Ok(_) => Err(changed(path)),
    }
}

pub(crate) fn replace(
    updates: &[(Snapshot, String)],
    mut before_replace: impl FnMut(&Path) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let mut staged = Vec::new();
    let mut backups = Vec::new();
    let result = (|| {
        for (snapshot, replacement) in updates {
            snapshot.validate()?;
            let suffix = uuid::Uuid::new_v4().simple().to_string();
            let filename = snapshot
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let backup = snapshot
                .path
                .with_file_name(format!(".{filename}.{suffix}.bak"));
            write_new(&backup, snapshot.content.as_bytes())?;
            backups.push(backup);
            let temporary = snapshot
                .path
                .with_file_name(format!(".{filename}.{suffix}.tmp"));
            staged.push(temporary.clone());
            write_new(&temporary, replacement.as_bytes())?;
        }
        // ponytail: per-file atomicity and a last-check/rename race; .bak files recover partial failure.
        for ((snapshot, _), temporary) in updates.iter().zip(&staged) {
            before_replace(&snapshot.path)?;
            snapshot.validate()?;
            fs::set_permissions(temporary, snapshot.identity.permissions())
                .map_err(|error| AppError::io("set permissions", temporary, error))?;
            fs::rename(temporary, &snapshot.path).map_err(|error| {
                AppError::io("replace (original retained in .bak)", &snapshot.path, error)
            })?;
        }
        Ok(())
    })();
    for temporary in staged {
        let _ = fs::remove_file(temporary);
    }
    if result.is_ok() {
        for backup in backups {
            if let Err(error) = fs::remove_file(&backup) {
                eprintln!(
                    "Saved resource, but could not remove recovery backup {}: {error}",
                    backup.display()
                );
            }
        }
    }
    result
}

pub(crate) struct Directory {
    pub logical: PathBuf,
    pub resolved: PathBuf,
    identity: fs::Metadata,
}

impl Directory {
    pub fn ensure(logical: &Path) -> Result<Self, AppError> {
        let resolved = match fs::canonicalize(logical) {
            Ok(resolved) => resolved,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                absent(logical)?;
                let parent = Self::ensure(logical.parent().ok_or_else(|| changed(logical))?)?;
                parent.validate()?;
                let resolved = parent
                    .resolved
                    .join(logical.file_name().ok_or_else(|| changed(logical))?);
                fs::create_dir(&resolved)
                    .map_err(|error| AppError::io("create directory", logical, error))?;
                parent.validate()?;
                resolved
            }
            Err(error) => return Err(AppError::io("resolve directory", logical, error)),
        };
        let directory = Self {
            logical: logical.to_owned(),
            identity: fs::symlink_metadata(&resolved)
                .map_err(|error| AppError::io("inspect directory", &resolved, error))?,
            resolved,
        };
        directory.validate()?;
        Ok(directory)
    }

    pub fn create_child(&self, name: &str) -> Result<Self, AppError> {
        self.validate()?;
        let resolved = self.resolved.join(name);
        absent(&resolved)?;
        fs::create_dir(&resolved)
            .map_err(|error| AppError::io("create directory", &resolved, error))?;
        self.validate()?;
        let child = Self {
            logical: self.logical.join(name),
            identity: fs::symlink_metadata(&resolved)
                .map_err(|error| AppError::io("inspect directory", &resolved, error))?,
            resolved,
        };
        child.validate()?;
        Ok(child)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        let current = fs::symlink_metadata(&self.resolved)
            .map_err(|error| AppError::io("inspect directory", &self.resolved, error))?;
        if fs::canonicalize(&self.logical).map_err(|_| changed(&self.logical))? != self.resolved
            || !current.is_dir()
        {
            return Err(changed(&self.logical));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if current.dev() != self.identity.dev() || current.ino() != self.identity.ino() {
                return Err(changed(&self.logical));
            }
        }
        #[cfg(not(unix))]
        if current.created().ok() != self.identity.created().ok() {
            return Err(changed(&self.logical));
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn create_file(&self, name: &str, content: &str) -> Result<(), AppError> {
        self.create_file_checked(name, content, || Ok(()))
    }

    pub fn create_file_checked(
        &self,
        name: &str,
        content: &str,
        before_publish: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        self.validate()?;
        let target = self.resolved.join(name);
        absent(&target)?;
        let temporary = self
            .resolved
            .join(format!(".{name}.{}.tmp", uuid::Uuid::new_v4().simple()));
        let result = (|| {
            write_new(&temporary, content.as_bytes())?;
            before_publish()?;
            self.validate()?;
            // Hard linking publishes complete content without replacing any existing file or symlink.
            fs::hard_link(&temporary, &target)
                .map_err(|error| AppError::io("create", &target, error))?;
            Ok(())
        })();
        let _ = fs::remove_file(temporary);
        result
    }
}

#[cfg(all(test, unix))]
#[path = "resource_file/tests.rs"]
mod safety_tests;

fn changed(path: &Path) -> AppError {
    AppError::configuration(format!(
        "{} changed during deletion or save; retry",
        path.display()
    ))
}

fn regular_file(path: &Path) -> Result<fs::Metadata, AppError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| AppError::io("inspect", path, error))?;
    if !metadata.is_file() {
        return Err(AppError::configuration(format!(
            "{} is not a regular file; operation did not complete",
            path.display()
        )));
    }
    Ok(metadata)
}

fn validate_metadata(path: &Path, expected: &fs::Metadata) -> Result<(), AppError> {
    let current = regular_file(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if current.dev() != expected.dev()
            || current.ino() != expected.ino()
            || current.mode() != expected.mode()
        {
            return Err(changed(path));
        }
    }
    #[cfg(not(unix))]
    if current.created().ok() != expected.created().ok()
        || current.permissions().readonly() != expected.permissions().readonly()
    {
        return Err(changed(path));
    }
    Ok(())
}

fn validate_source(logical: &Path, resolved: &Path) -> Result<(), AppError> {
    if fs::canonicalize(logical).map_err(|_| changed(logical))? != resolved {
        return Err(changed(logical));
    }
    regular_file(resolved)?;
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| AppError::io("create", path, error))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| AppError::io("write", path, error))
}
