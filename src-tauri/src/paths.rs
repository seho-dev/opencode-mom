use std::env;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Resolved locations of every file the application manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    home: PathBuf,
    user_config_dir: PathBuf,
    opencode: PathBuf,
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
        let project_root = env::var_os("OPENCODE_PROJECT_ROOT").map(PathBuf::from);
        // opencode (and this app) always use `<home>/.config/opencode`, on every
        // platform. Do NOT use dirs::config_dir() here: on Windows it resolves to
        // %APPDATA%\Roaming, which is a different, often empty, config file.
        let user_config_dir = home.join(".config");
        let opencode = config.unwrap_or_else(|| {
            directory
                .unwrap_or_else(|| user_config_dir.join("opencode"))
                .join("opencode.jsonc")
        });
        Ok(Self {
            home,
            user_config_dir,
            opencode,
            project_root,
        })
    }

    pub fn for_home(home: &Path) -> Self {
        Self::with_parts(home.to_path_buf(), None, None)
    }

    pub fn with_parts(home: PathBuf, config: Option<PathBuf>, directory: Option<PathBuf>) -> Self {
        let user_config_dir = home.join(".config");
        let opencode = config.unwrap_or_else(|| {
            directory
                .unwrap_or_else(|| user_config_dir.join("opencode"))
                .join("opencode.jsonc")
        });
        Self {
            home,
            user_config_dir,
            opencode,
            project_root: None,
        }
    }

    pub fn with_project_root(mut self, project_root: impl Into<PathBuf>) -> Self {
        self.project_root = Some(project_root.into());
        self
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
            .join("oh-my-opencode-slim.jsonc")
    }

    pub fn omo_file(&self) -> PathBuf {
        self.home.join(".omo").join("omo.jsonc")
    }

    pub fn global_agents_dir(&self) -> PathBuf {
        self.user_config_dir.join("opencode").join("agents")
    }

    pub fn project_agents_dir(&self) -> Option<PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".opencode").join("agents"))
    }
}
