use std::env;
use std::path::{Path, PathBuf};

pub use crate::core::error::ConfigPathError;

#[derive(Debug, Clone, Copy)]
pub struct HomeEnv<'a> {
    home: Option<&'a Path>,
    userprofile: Option<&'a Path>,
}

impl<'a> HomeEnv<'a> {
    pub const fn new(home: Option<&'a Path>, userprofile: Option<&'a Path>) -> Self {
        Self { home, userprofile }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    home: PathBuf,
}

impl ConfigPaths {
    pub fn from_environment() -> Result<Self, ConfigPathError> {
        let home = env::var_os("HOME").map(PathBuf::from);
        let userprofile = env::var_os("USERPROFILE").map(PathBuf::from);
        Self::from_owned_home(home, userprofile)
    }

    pub fn from_home_env(env: HomeEnv<'_>) -> Result<Self, ConfigPathError> {
        let home = env
            .home
            .or(env.userprofile)
            .ok_or(ConfigPathError::MissingHome)?;
        Ok(Self {
            home: home.to_path_buf(),
        })
    }

    pub fn omo_switch_dir(&self) -> PathBuf {
        self.home.join(".config").join("omo-switch")
    }

    pub fn opencode_dir(&self) -> PathBuf {
        self.home.join(".config").join("opencode")
    }

    pub fn groups_file(&self) -> PathBuf {
        self.omo_switch_dir().join("groups.json")
    }

    pub fn state_file(&self) -> PathBuf {
        self.omo_switch_dir().join("state.json")
    }

    pub fn oh_my_openagent_file(&self) -> PathBuf {
        self.opencode_dir().join("oh-my-openagent.json")
    }

    pub fn opencode_file(&self) -> PathBuf {
        self.opencode_dir().join("opencode.json")
    }

    fn from_owned_home(
        home: Option<PathBuf>,
        userprofile: Option<PathBuf>,
    ) -> Result<Self, ConfigPathError> {
        home.or(userprofile)
            .map(|home| Self { home })
            .ok_or(ConfigPathError::MissingHome)
    }
}
