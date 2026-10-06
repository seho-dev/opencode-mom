use std::env;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Resolved locations of every file the application manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    home: PathBuf,
    user_config_dir: PathBuf,
    opencode: PathBuf,
    global_config_files: Vec<PathBuf>,
    global_skill_dirs: Vec<PathBuf>,
    working_directory: PathBuf,
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
            global_dir.join("skills"),
        ];
        if let Some(directory) = &directory {
            global_config_files.push(directory.join("opencode.json"));
            global_config_files.push(directory.join("opencode.jsonc"));
            global_skill_dirs.push(directory.join("skills"));
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
        let opencode = resolve_opencode_file(config, directory, &user_config_dir);
        Self {
            home,
            user_config_dir,
            opencode,
            global_config_files,
            global_skill_dirs,
            working_directory,
        }
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

    pub fn global_config_files(&self) -> &[PathBuf] {
        &self.global_config_files
    }

    pub fn global_skill_dirs(&self) -> &[PathBuf] {
        &self.global_skill_dirs
    }

    pub fn native_global_skills_dir(&self) -> PathBuf {
        self.user_config_dir.join("opencode/skills")
    }

    pub fn resolve_skill_path(&self, source: &str) -> PathBuf {
        if let Some(relative) = source
            .strip_prefix("~/")
            .or_else(|| source.strip_prefix("~\\"))
        {
            return self.home.join(relative);
        }
        if source == "~" {
            return self.home.clone();
        }
        let path = PathBuf::from(source);
        if path.is_absolute() {
            path
        } else {
            self.working_directory.join(path)
        }
    }
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
