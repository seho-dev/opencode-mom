use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

use crate::error::AppError;

/// Resolved locations of every file the application manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    home: PathBuf,
    user_config_dir: PathBuf,
    opencode: PathBuf,
    global_config_files: Vec<PathBuf>,
    managed_config_files: Vec<PathBuf>,
    global_resource_dirs: Vec<PathBuf>,
    global_skill_dirs: Vec<PathBuf>,
    agent_dirs: Vec<PathBuf>,
    command_dirs: Vec<PathBuf>,
    // Retained only to prevent other resources from writing into plugin roots.
    plugin_dirs: Vec<PathBuf>,
    resource_write_dir: PathBuf,
    global_instructions: PathBuf,
    global_write_roots: Vec<(PathBuf, PathBuf)>,
    global_write_files: Vec<(PathBuf, PathBuf)>,
    override_path_sources: Vec<(PathBuf, PathBuf)>,
    project_root: Option<PathBuf>,
}

impl ConfigPaths {
    pub fn from_environment() -> Result<Self, AppError> {
        let home = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .ok_or_else(|| {
                AppError::validation("HOME or USERPROFILE environment variable is not set")
            })?;
        let config = env::var_os("OPENCODE_CONFIG").map(PathBuf::from);
        let directory = env::var_os("OPENCODE_CONFIG_DIR").map(PathBuf::from);
        let user_config_dir = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        Ok(Self::with_resource_parts(
            home,
            config,
            directory,
            user_config_dir,
            env::current_dir()?,
        ))
    }

    pub fn for_home(home: &Path) -> Self {
        Self::with_parts(home.to_path_buf(), None, None)
    }

    pub fn with_parts(home: PathBuf, config: Option<PathBuf>, directory: Option<PathBuf>) -> Self {
        let user_config_dir = home.join(".config");
        let working_directory = env::current_dir().unwrap_or_else(|_| home.clone());
        Self::with_resource_parts(home, config, directory, user_config_dir, working_directory)
    }

    pub fn with_resource_parts(
        home: PathBuf,
        config: Option<PathBuf>,
        directory: Option<PathBuf>,
        user_config_dir: PathBuf,
        working_directory: PathBuf,
    ) -> Self {
        let mut override_path_sources = Vec::new();
        let mut absolute = |path: PathBuf, normalize: bool| {
            let path = if path.is_absolute() {
                path
            } else {
                working_directory.join(path)
            };
            if !normalize {
                return path;
            }
            let normalized = normalize_override_path(&path);
            if path.components().any(|part| part == Component::ParentDir) {
                override_path_sources.push((path, normalized.clone()));
            }
            normalized
        };
        let config = config.map(|path| {
            // Keep invalid filenames for config-source validation, not a normalized directory.
            let normalize = path.file_name().is_some();
            absolute(path, normalize)
        });
        let directory = directory.map(|path| absolute(path, true));
        let global_dir = user_config_dir.join("opencode");
        let mut global_config_files = vec![
            global_dir.join("opencode.json"),
            global_dir.join("opencode.jsonc"),
        ];
        let mut global_skill_dirs = vec![
            home.join(".claude/skills"),
            home.join(".agents/skills"),
            global_dir.join("skill"),
            global_dir.join("skills"),
        ];
        let mut agent_dirs = vec![global_dir.join("agent"), global_dir.join("agents")];
        let mut command_dirs = vec![global_dir.join("command"), global_dir.join("commands")];
        let mut plugin_dirs = vec![global_dir.join("plugins")];
        if let Some(directory) = &directory {
            global_config_files.push(directory.join("opencode.json"));
            global_config_files.push(directory.join("opencode.jsonc"));
            global_skill_dirs.push(directory.join("skill"));
            global_skill_dirs.push(directory.join("skills"));
            agent_dirs.push(directory.join("agent"));
            agent_dirs.push(directory.join("agents"));
            command_dirs.push(directory.join("command"));
            command_dirs.push(directory.join("commands"));
            plugin_dirs.push(directory.join("plugins"));
        }
        if let Some(config) = &config {
            global_config_files.push(config.clone());
        }
        // Keep the last occurrence so an explicit override retains its precedence.
        let deduplicate = |paths: &mut Vec<PathBuf>| {
            let mut seen = std::collections::BTreeSet::new();
            paths.reverse();
            paths.retain(|path| seen.insert(path.clone()));
            paths.reverse();
        };
        deduplicate(&mut global_config_files);
        deduplicate(&mut global_skill_dirs);
        deduplicate(&mut agent_dirs);
        deduplicate(&mut command_dirs);
        deduplicate(&mut plugin_dirs);
        let mut global_resource_dirs = agent_dirs.clone();
        global_resource_dirs.extend(global_skill_dirs.iter().cloned());
        global_resource_dirs.extend(command_dirs.iter().cloned());
        global_resource_dirs.extend(plugin_dirs.iter().cloned());
        let resource_write_dir = directory.clone().unwrap_or_else(|| global_dir.clone());
        let global_instructions = directory.as_ref().unwrap_or(&global_dir).join("AGENTS.md");
        let global_write_roots = command_dirs
            .iter()
            .map(|path| (path.clone(), resolved_resource_path(path)))
            .collect();
        let global_write_files = global_config_files
            .iter()
            .chain(std::iter::once(&global_instructions))
            .map(|path| (path.clone(), resolved_resource_path(path)))
            .collect();
        let opencode = resolve_opencode_file(config, directory, &user_config_dir);
        Self {
            home,
            user_config_dir,
            opencode,
            managed_config_files: global_config_files.clone(),
            global_config_files,
            global_resource_dirs,
            global_skill_dirs,
            agent_dirs,
            command_dirs,
            plugin_dirs,
            resource_write_dir,
            global_instructions,
            global_write_roots,
            global_write_files,
            override_path_sources,
            project_root: None,
        }
    }

    pub(crate) fn home_dir(&self) -> &Path {
        &self.home
    }

    pub fn app_config_dir(&self) -> PathBuf {
        self.home.join(".config").join("opencode-mom")
    }

    pub fn config_file(&self) -> PathBuf {
        self.app_config_dir().join("config.json")
    }

    pub fn opencode_file(&self) -> PathBuf {
        self.opencode.clone()
    }

    pub fn slim_file(&self) -> PathBuf {
        self.user_config_dir
            .join("opencode")
            .join("oh-my-opencode-slim.json")
    }

    pub fn omo_file(&self) -> PathBuf {
        self.home.join(".omo").join("omo.jsonc")
    }

    pub fn global_agents_dir(&self) -> PathBuf {
        self.user_config_dir.join("opencode").join("agents")
    }

    pub fn agent_dirs(&self) -> &[PathBuf] {
        &self.agent_dirs
    }

    pub fn command_dirs(&self) -> &[PathBuf] {
        &self.command_dirs
    }

    fn validate_override_paths(&self) -> Result<(), AppError> {
        if self
            .override_path_sources
            .iter()
            .any(|(source, normalized)| normalize_override_path(source) != *normalized)
        {
            return Err(AppError::configuration(
                "Global configuration path changed; reload",
            ));
        }
        Ok(())
    }

    pub fn commands_write_dir(&self) -> PathBuf {
        self.resource_write_dir.join("commands")
    }

    pub fn global_instructions_file(&self) -> PathBuf {
        self.global_instructions.clone()
    }

    pub fn project_directory(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub fn validate_managed_write(&self, path: &Path) -> Result<(), AppError> {
        let invalid = || AppError::validation("Write target is outside managed configuration");
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err(invalid());
        }
        self.validate_resource_root()?;
        let resolved = checked_resource_path(path)?;
        let parent = checked_resource_path(path.parent().ok_or_else(invalid)?)?;
        if self.plugin_dirs.iter().any(|dir| {
            path.starts_with(dir)
                || is_within_boundary(dir, &resolved)
                || is_within_boundary(dir, &parent)
        }) {
            return Err(invalid());
        }
        if let Some(root) = &self.project_root {
            let command_path =
                |target: &Path| self.command_dirs.iter().any(|dir| target.starts_with(dir));
            if !command_path(path)
                || !command_path(&resolved)
                || !command_path(&parent)
                || !resolved.starts_with(root)
                || !parent.starts_with(root)
            {
                return Err(invalid());
            }
            if self
                .agent_dirs
                .iter()
                .chain(&self.global_skill_dirs)
                .any(|dir| is_within_boundary(dir, &resolved) || is_within_boundary(dir, &parent))
                || self
                    .global_config_files
                    .iter()
                    .any(|file| resolved_resource_path(file) == resolved)
            {
                return Err(invalid());
            }
            if self
                .global_resource_dirs
                .iter()
                .any(|dir| is_within_boundary(dir, &resolved) || is_within_boundary(dir, &parent))
                || self
                    .global_write_files
                    .iter()
                    .any(|(file, _)| resolved_resource_path(file) == resolved)
            {
                return Err(AppError::validation(
                    "Project resources overlap Global configuration",
                ));
            }
        } else {
            self.validate_override_paths()?;
            if let Some((logical, captured)) = self
                .global_write_files
                .iter()
                .find(|(logical, captured)| logical == path || captured == path)
            {
                if logical != &self.global_instructions {
                    return Err(invalid());
                }
                if checked_resource_path(logical)? == *captured {
                    return Ok(());
                }
                return Err(AppError::configuration(
                    "Global configuration source changed; reload",
                ));
            }
            let allowed = self
                .global_write_roots
                .iter()
                .any(|(logical, _)| path.starts_with(logical));
            if !allowed {
                return Err(invalid());
            }
            for (logical, captured) in &self.global_write_roots {
                if checked_resource_path(logical)? != *captured {
                    return Err(AppError::configuration(
                        "Global resource directory changed; reload",
                    ));
                }
            }
            if !self
                .global_write_roots
                .iter()
                .any(|(_, root)| parent.starts_with(root))
            {
                return Err(invalid());
            }
            if !self
                .global_write_roots
                .iter()
                .any(|(_, root)| resolved.starts_with(root))
                && !fs::symlink_metadata(path)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    pub fn agent_config_files(&self) -> &[PathBuf] {
        if self.is_project() {
            &self.global_config_files
        } else {
            std::slice::from_ref(&self.opencode)
        }
    }

    pub fn is_project(&self) -> bool {
        self.project_root.is_some()
    }

    pub fn validate_resource_root(&self) -> Result<(), AppError> {
        if let Some(root) = &self.project_root {
            if canonical_project_directory(root)? != *root {
                return Err(AppError::configuration(format!(
                    "project root changed: {}",
                    root.display()
                )));
            }
            self.ensure_project_boundary(root)?;
        }
        Ok(())
    }

    pub fn validate_project_directory(&self, path: &Path) -> Result<PathBuf, AppError> {
        let root = canonical_project_directory(path)?;
        self.ensure_project_boundary(&root)?;
        Ok(root)
    }

    fn ensure_project_boundary(&self, root: &Path) -> Result<(), AppError> {
        let configs = [
            "opencode.json",
            "opencode.jsonc",
            ".opencode/opencode.json",
            ".opencode/opencode.jsonc",
        ]
        .map(|relative| resolved_resource_path(&root.join(relative)));
        let directories = [
            ".opencode/agent",
            ".opencode/agents",
            ".opencode/skill",
            ".opencode/skills",
            ".opencode/command",
            ".opencode/commands",
            ".opencode/plugins",
        ]
        .map(|relative| resolved_resource_path(&root.join(relative)));
        let overlap = || AppError::validation("Project resources overlap Global configuration");
        for directory in &self.global_resource_dirs {
            if is_within_boundary(directory, root)
                || configs
                    .iter()
                    .any(|path| is_within_boundary(directory, path))
                || directories.iter().any(|path| {
                    is_within_boundary(directory, path) || is_within_boundary(path, directory)
                })
            {
                return Err(overlap());
            }
        }
        for file in self
            .managed_config_files
            .iter()
            .chain(std::iter::once(&self.global_instructions))
        {
            let file = resolved_resource_path(file);
            if configs.contains(&file) || directories.iter().any(|dir| file.starts_with(dir)) {
                return Err(overlap());
            }
        }
        Ok(())
    }

    pub fn ensure_global_write(&self) -> Result<(), AppError> {
        if self.is_project() {
            return Err(AppError::validation("Project resources are read-only"));
        }
        Ok(())
    }

    pub fn for_project(&self, project_id: Option<Uuid>) -> Result<Self, AppError> {
        let Some(id) = project_id else {
            return Ok(self.clone());
        };
        let config = crate::models::load_config(&self.config_file())?;
        let project = config
            .projects
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(|| AppError::not_found("project not found"))?;
        let root = PathBuf::from(&project.path);
        let mut paths = self.clone();
        paths.global_config_files = vec![
            root.join("opencode.json"),
            root.join("opencode.jsonc"),
            root.join(".opencode/opencode.json"),
            root.join(".opencode/opencode.jsonc"),
        ];
        paths.agent_dirs = vec![root.join(".opencode/agent"), root.join(".opencode/agents")];
        paths.global_skill_dirs = vec![root.join(".opencode/skill"), root.join(".opencode/skills")];
        paths.command_dirs = vec![
            root.join(".opencode/command"),
            root.join(".opencode/commands"),
        ];
        paths.plugin_dirs = vec![root.join(".opencode/plugins")];
        paths.resource_write_dir = root.join(".opencode");
        paths.project_root = Some(root);
        paths.validate_resource_root()?;
        Ok(paths)
    }

    pub fn global_config_files(&self) -> &[PathBuf] {
        &self.global_config_files
    }

    pub fn global_skill_dirs(&self) -> &[PathBuf] {
        &self.global_skill_dirs
    }

    pub fn native_global_skills_dir(&self) -> PathBuf {
        self.user_config_dir.join("opencode/skills")
    }

    pub fn resolve_skill_path(&self, source: &str) -> Option<PathBuf> {
        if let Some(relative) = source
            .strip_prefix("~/")
            .or_else(|| source.strip_prefix("~\\"))
        {
            return Some(self.home.join(relative));
        }
        if source == "~" {
            return Some(self.home.clone());
        }
        let path = PathBuf::from(source);
        path.is_absolute().then_some(path)
    }
}

pub fn reject_project_write(project_id: Option<Uuid>) -> Result<(), AppError> {
    if project_id.is_some() {
        return Err(AppError::validation("Project resources are read-only"));
    }
    Ok(())
}

pub fn canonical_project_directory(path: &Path) -> Result<PathBuf, AppError> {
    if !path.is_absolute() {
        return Err(AppError::validation("project path must be absolute"));
    }
    let root = fs::canonicalize(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::not_found(format!("project root not found: {}", path.display()))
        } else {
            AppError::io("resolve project", path, error)
        }
    })?;
    if root.parent().is_none() {
        return Err(AppError::validation(
            "project path must not be a filesystem root",
        ));
    }
    if !root.is_dir() {
        return Err(AppError::validation("project path must be a directory"));
    }
    if root.to_str().is_none() {
        return Err(AppError::validation("project path must be valid UTF-8"));
    }
    let mut entries =
        fs::read_dir(&root).map_err(|error| AppError::io("read project", &root, error))?;
    if let Some(entry) = entries.next() {
        entry.map_err(|error| AppError::io("read project", &root, error))?;
    }
    Ok(root)
}

fn normalize_override_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                // Preserve logical aliases unless `..` traverses the symlink itself.
                match fs::symlink_metadata(&normalized) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        let Ok(target) = fs::canonicalize(&normalized) else {
                            return path.to_owned();
                        };
                        if !target.is_dir() {
                            return path.to_owned();
                        }
                        normalized = target;
                    }
                    Ok(metadata) if !metadata.is_dir() => return path.to_owned(),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return path.to_owned(),
                }
                normalized.pop();
            }
            part => normalized.push(part.as_os_str()),
        }
    }
    normalized
}

fn resolved_resource_path(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(path) => path,
        // ponytail: inaccessible aliases use ancestors; require resolved handles before project writes.
        Err(_) => match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => resolved_resource_path(parent).join(name),
            _ => path.to_owned(),
        },
    }
}

fn checked_resource_path(path: &Path) -> Result<PathBuf, AppError> {
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::symlink_metadata(path) {
                Ok(_) => {
                    return Err(AppError::configuration(format!(
                        "Unresolved source: {}",
                        path.display()
                    )))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::io("inspect", path, error)),
            }
            let parent = path
                .parent()
                .ok_or_else(|| AppError::configuration("Unresolved resource path"))?;
            let name = path
                .file_name()
                .ok_or_else(|| AppError::configuration("Invalid resource filename"))?;
            Ok(checked_resource_path(parent)?.join(name))
        }
        Err(error) => Err(AppError::io("resolve", path, error)),
    }
}

/// Checks whether `path` is within `base` directory (inclusive of base itself).
pub fn is_within_boundary(base: &Path, path: &Path) -> bool {
    let resolved_base = resolved_resource_path(base);
    let resolved_path = resolved_resource_path(path);
    resolved_path.starts_with(&resolved_base)
}

/// Resolve the opencode config file the CLI actually uses: explicit `OPENCODE_CONFIG`
/// wins, otherwise `opencode.jsonc` when present, falling back to `opencode.json`
/// (opencode reads both; a user may only have `.json`).
fn resolve_opencode_file(
    config: Option<PathBuf>,
    directory: Option<PathBuf>,
    user_config_dir: &Path,
) -> PathBuf {
    if let Some(config) = config {
        return config;
    }
    let dir = directory.unwrap_or_else(|| user_config_dir.join("opencode"));
    let jsonc = dir.join("opencode.jsonc");
    if jsonc.exists() {
        jsonc
    } else {
        dir.join("opencode.json")
    }
}

#[cfg(test)]
mod tests;
