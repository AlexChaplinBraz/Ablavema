//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use config::{Config, ConfigError, File as ConfigFile, FileFormat};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    error::Error,
    fs::{create_dir_all, File},
    io::prelude::*,
    path::PathBuf,
    sync::RwLock,
};

// TODO: Consider not letting the user change any of the paths. This would
// eliminate some potential IO errors if the user sets them into weird places.
// TODO: Consider locking down the entire config into a binary so the user can't change it
// from outside the program. This would eliminate a good amount of potential user errors.

const CONFIG_NAME: &str = "config.toml";

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("", "", "BlenderLauncher").unwrap();
}

lazy_static! {
    static ref CONFIG_PATH: PathBuf = {
        let current_exe = current_exe().unwrap();
        let portable_path = current_exe.parent().unwrap().to_path_buf();
        let portable_file = portable_path.join("portable");

        if portable_file.exists() {
            portable_path.join(CONFIG_NAME)
        } else {
            let mut config_path = PROJECT_DIRS.config_dir().to_path_buf();
            create_dir_all(&config_path).unwrap();
            config_path.push(CONFIG_NAME);
            config_path
        }
    };
}

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub default_package: String,
    pub use_latest_as_default: bool,
    pub check_updates_at_launch: bool,
    pub minutes_between_updates: u64,
    pub update_daily: bool,
    pub update_experimental: bool,
    pub update_stable: bool,
    pub update_lts: bool,
    pub keep_only_latest_daily: bool,
    pub keep_only_latest_experimental: bool,
    pub keep_only_latest_stable: bool,
    pub keep_only_latest_lts: bool,
    pub packages_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub releases_db: PathBuf,
}

impl Settings {
    pub fn load() -> Result<(), ConfigError> {
        if !CONFIG_PATH.exists() {
            let default = Settings::default();
            let mut conf_file = File::create(&*CONFIG_PATH).unwrap();

            conf_file
                .write_all(toml::to_string(&default).unwrap().as_bytes())
                .unwrap();
        }

        SETTINGS.write().unwrap().merge(ConfigFile::new(
            &CONFIG_PATH.to_str().unwrap(),
            FileFormat::Toml,
        ))?;

        Ok(())
    }

    pub fn save() -> Result<(), Box<dyn Error>> {
        let config = SETTINGS.read().unwrap().clone();
        let settings: Settings = config.try_into().unwrap();
        let toml = toml::to_string(&settings)?;
        let mut file = File::create(&*CONFIG_PATH)?;
        file.write_all(toml.as_bytes())?;

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_package: String::new(),
            use_latest_as_default: true,
            check_updates_at_launch: true,
            minutes_between_updates: 60,
            update_daily: true,
            update_experimental: true,
            update_stable: true,
            update_lts: true,
            keep_only_latest_daily: false,
            keep_only_latest_experimental: false,
            keep_only_latest_stable: false,
            keep_only_latest_lts: false,
            packages_dir: {
                let current_exe = current_exe().unwrap();
                let mut portable_path = current_exe.parent().unwrap().to_path_buf();
                let portable_file = portable_path.join("portable");

                if portable_file.exists() {
                    portable_path.push("packages");
                    create_dir_all(&portable_path).unwrap();
                    portable_path
                } else {
                    let data_path = PROJECT_DIRS.data_dir().to_path_buf();
                    create_dir_all(&data_path).unwrap();
                    data_path
                }
            },
            releases_db: {
                let current_exe = current_exe().unwrap();
                let portable_path = current_exe.parent().unwrap().to_path_buf();
                let portable_file = portable_path.join("portable");

                if portable_file.exists() {
                    portable_path.join("releases_db.bin")
                } else {
                    let mut releases_db_path = PROJECT_DIRS.config_dir().to_path_buf();
                    create_dir_all(&releases_db_path).unwrap();
                    releases_db_path.push("releases_db.bin");
                    releases_db_path
                }
            },
            cache_dir: {
                let current_exe = current_exe().unwrap();
                let mut portable_path = current_exe.parent().unwrap().to_path_buf();
                let portable_file = portable_path.join("portable");

                if portable_file.exists() {
                    portable_path.push("cache");
                    create_dir_all(&portable_path).unwrap();
                    portable_path
                } else {
                    let cache_dir_path = PROJECT_DIRS.cache_dir().to_path_buf();
                    create_dir_all(&cache_dir_path).unwrap();
                    cache_dir_path
                }
            },
        }
    }
}
