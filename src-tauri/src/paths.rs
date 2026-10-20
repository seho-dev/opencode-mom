use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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
        let absolute = |path: PathBuf| {
            if path.is_absolute() {
                path
            } else {
                working_directory.join(path)
            }
        };
        let config = config.map(absolute);
        let directory = directory.map(absolute);
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
        if let Some(directory) = &directory {
            global_config_files.push(directory.join("opencode.json"));
            global_config_files.push(directory.join("opencode.jsonc"));
            global_skill_dirs.push(directory.join("skill"));
            global_skill_dirs.push(directory.join("skills"));
            agent_dirs.push(directory.join("agent"));
            agent_dirs.push(directory.join("agents"));
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
        let mut global_resource_dirs = agent_dirs.clone();
        global_resource_dirs.extend(global_skill_dirs.iter().cloned());
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
        for file in &self.managed_config_files {
            let file = resolved_resource_path(file);
            if configs.contains(&file) {
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
