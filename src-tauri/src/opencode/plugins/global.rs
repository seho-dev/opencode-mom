use std::fs;
use std::path::{Path, PathBuf};

use crate::document::JsoncDoc;
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::Snapshot;

use super::{
    PluginConfigEntry, PluginConfigPreview, PluginConfigSource, PluginRuntime, PluginSource,
};

pub(super) struct GlobalPaths {
    files: Vec<(PathBuf, PathBuf)>,
    directories: Vec<(PathBuf, PathBuf)>,
}

impl GlobalPaths {
    pub(super) fn capture(paths: &ConfigPaths) -> Result<Self, AppError> {
        let capture = |path: &Path| Ok((path.to_owned(), resolved(path)?));
        Ok(Self {
            files: paths
                .global_config_files()
                .iter()
                .map(|path| capture(path))
                .collect::<Result<_, AppError>>()?,
            directories: paths
                .plugin_config_dirs()
                .map(capture)
                .collect::<Result<_, AppError>>()?,
        })
    }

    pub(super) fn ensure_current(&self) -> Result<(), AppError> {
        for (logical, captured) in self.files.iter().chain(&self.directories) {
            if resolved(logical)? != *captured {
                return Err(AppError::configuration(
                    "Global plugin source changed; refresh before continuing",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn filter(
        &self,
        mut preview: PluginConfigPreview,
    ) -> Result<PluginConfigPreview, AppError> {
        self.ensure_current()?;
        preview.data.retain(|source| match source {
            PluginConfigSource::Document {
                path: Some(path), ..
            } => matches_path(&self.files, path),
            PluginConfigSource::Directory { path } => matches_path(&self.directories, path),
            _ => false,
        });
        for source in &mut preview.data {
            if let PluginConfigSource::Document {
                path: Some(path),
                plugins,
            } = source
            {
                let current = match Snapshot::read(Path::new(path))? {
                    Some(snapshot) => read_declarations(&snapshot.content)?,
                    None => Vec::new(),
                };
                let allowed: Vec<_> = plugins.iter().map(package).map(str::to_owned).collect();
                *plugins = current
                    .into_iter()
                    .filter(|entry| allowed.iter().any(|value| value == package(entry)))
                    .collect();
            }
        }
        self.ensure_current()?;
        Ok(preview)
    }
}

fn matches_path(allowed: &[(PathBuf, PathBuf)], path: &str) -> bool {
    let path = Path::new(path);
    allowed
        .iter()
        .any(|(logical, canonical)| path == logical || path == canonical)
}

pub(super) fn package(entry: &PluginConfigEntry) -> &str {
    match entry {
        PluginConfigEntry::Native(value) => value,
        PluginConfigEntry::Package { package, .. } => package,
    }
}

pub(super) fn read_declarations(content: &str) -> Result<Vec<PluginConfigEntry>, AppError> {
    let invalid = || AppError::configuration("Invalid Global plugin declarations");
    if content.len() > super::MAX_BYTES as usize {
        return Err(invalid());
    }
    let document = JsoncDoc::parse(content)?;
    let Some(value) = document.raw().get("plugins") else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .filter(|values| values.len() <= 4096)
        .ok_or_else(invalid)?;
    values
        .iter()
        .map(|value| {
            let entry = if let Some(value) = value.as_str() {
                PluginConfigEntry::Native(value.to_owned())
            } else {
                let package = value
                    .get("package")
                    .and_then(|value| value.as_str())
                    .ok_or_else(invalid)?
                    .to_owned();
                let options = value
                    .get("options")
                    .map(|value| value.as_object().cloned().ok_or_else(invalid))
                    .transpose()?;
                PluginConfigEntry::Package { package, options }
            };
            if !super::bounded_text(package(&entry), 4096) {
                return Err(invalid());
            }
            Ok(entry)
        })
        .collect()
}

pub(super) fn local_declaration_matches(
    value: &str,
    document: &Path,
    target: &str,
) -> Result<bool, AppError> {
    let Some(root) = declared_path(value, document)? else {
        return Ok(false);
    };
    Ok(resolved(Path::new(target))? == root)
}

pub(super) fn declares_package(preview: &PluginConfigPreview, target: &str) -> bool {
    preview.data.iter().any(|source| match source {
        PluginConfigSource::Document { plugins, .. } => {
            plugins.iter().any(|entry| package(entry) == target)
        }
        _ => false,
    })
}

pub(super) fn filter_runtime(
    mut runtime: PluginRuntime,
    preview: &PluginConfigPreview,
) -> Result<PluginRuntime, AppError> {
    let mut local_roots = Vec::new();
    for source in &preview.data {
        match source {
            PluginConfigSource::Document {
                path: Some(path),
                plugins,
            } => {
                for entry in plugins {
                    if let Some(path) = declared_path(package(entry), Path::new(path))? {
                        let directory = fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir());
                        local_roots.push((path, directory));
                    }
                }
            }
            PluginConfigSource::Directory { path } => {
                local_roots.push((resolved(&Path::new(path).join("plugins"))?, true))
            }
            _ => {}
        }
    }
    let mut data = Vec::new();
    for mut item in runtime.data {
        item.can_remove = match &item.source {
            PluginSource::Package { target, .. } => declares_package(preview, target),
            PluginSource::Local { path } if super::absolute_path(path) => {
                let mut declared = false;
                for source in &preview.data {
                    if let PluginConfigSource::Document {
                        path: Some(document),
                        plugins,
                    } = source
                    {
                        for entry in plugins {
                            declared |= local_declaration_matches(
                                package(entry),
                                Path::new(document),
                                path,
                            )?;
                        }
                    }
                }
                declared
            }
            _ => false,
        };
        let keep = match &item.source {
            PluginSource::Package { target, .. } => declares_package(preview, target),
            PluginSource::Local { path } if super::absolute_path(path) => {
                let path = resolved(Path::new(path))?;
                local_roots.iter().any(|(root, directory)| {
                    path == *root || (*directory && path.starts_with(root))
                })
            }
            _ => false,
        };
        if keep {
            data.push(item);
        }
    }
    runtime.data = data;
    Ok(runtime)
}

fn declared_path(value: &str, document: &Path) -> Result<Option<PathBuf>, AppError> {
    let path = Path::new(value);
    let path = if path.is_absolute() {
        path.to_owned()
    } else if ["./", "../", ".\\", "..\\"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
    {
        let Some(parent) = document.parent() else {
            return Ok(None);
        };
        parent.join(path)
    } else if value.starts_with("file:") {
        let Some(path) = reqwest::Url::parse(value)
            .ok()
            .filter(|url| {
                url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            })
            .and_then(|url| url.to_file_path().ok())
        else {
            return Ok(None);
        };
        path
    } else {
        return Ok(None);
    };
    resolved(&path).map(Some)
}

fn resolved(path: &Path) -> Result<PathBuf, AppError> {
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if fs::symlink_metadata(path).is_ok() {
                return Err(AppError::configuration("Unresolved Global plugin source"));
            }
            match (path.parent(), path.file_name()) {
                (Some(parent), Some(name)) => Ok(resolved(parent)?.join(name)),
                _ => Err(AppError::configuration("Invalid Global plugin source path")),
            }
        }
        Err(_) => Err(AppError::configuration(
            "Unable to resolve Global plugin source",
        )),
    }
}
