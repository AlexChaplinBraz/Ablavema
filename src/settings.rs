//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use config::{Config, ConfigError, File as ConfigFile, FileFormat};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    fs::{create_dir_all, File},
    io::prelude::*,
    path::PathBuf,
    sync::RwLock,
};

lazy_static! {
    pub static ref CONFIG_PATH: PathBuf = if cfg!(target_os = "linux") {
        let bl_env = env::var_os("BLENDER_LAUNCHER_CONFIG");
        let xdg_env = env::var_os("XDG_CONFIG_HOME");

        if bl_env.is_some() {
            PathBuf::from(bl_env.unwrap().to_str().unwrap().to_string())
        } else if xdg_env.is_some() {
            PathBuf::from(format!(
                "{}/BlenderLauncher/config.toml",
                xdg_env.unwrap().to_str().unwrap()
            ))
        } else {
            PathBuf::from(format!(
                "{}/.config/BlenderLauncher/config.toml",
                env::var("HOME").unwrap()
            ))
        }
    } else if cfg!(target_os = "windows") {
        todo!("windows config");
    } else if cfg!(target_os = "macos") {
        todo!("macos config");
    } else {
        unreachable!("Unsupported OS config");
    };
}

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub default_package: String,
    pub use_latest_as_default: bool,
    pub update_daily: bool,
    pub keep_only_latest_daily: bool,
    pub update_experimental: bool,
    pub keep_only_latest_experimental: bool,
    pub update_stable: bool,
    pub keep_only_latest_stable: bool,
    pub update_lts: bool,
    pub keep_only_latest_lts: bool,
    pub packages_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub releases_db: PathBuf,
    pub interface: Interface,
}

impl Settings {
    pub fn load() -> Result<(), ConfigError> {
        if !CONFIG_PATH.exists() {
            let default = Settings::default();
            create_dir_all(CONFIG_PATH.parent().unwrap()).unwrap();
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
            update_daily: true,
            keep_only_latest_daily: false,
            update_experimental: true,
            keep_only_latest_experimental: false,
            update_stable: true,
            keep_only_latest_stable: false,
            update_lts: true,
            keep_only_latest_lts: false,
            packages_dir: PathBuf::from({
                if cfg!(target_os = "linux") {
                    "/home/alex/.config/BlenderLauncher/packages"
                } else if cfg!(target_os = "windows") {
                    todo!("windows config");
                } else if cfg!(target_os = "macos") {
                    todo!("macos config");
                } else {
                    unreachable!("Unsupported OS config");
                }
            }),
            releases_db: PathBuf::from({
                if cfg!(target_os = "linux") {
                    "/home/alex/.config/BlenderLauncher/releases_db.bin"
                } else if cfg!(target_os = "windows") {
                    todo!("windows config");
                } else if cfg!(target_os = "macos") {
                    todo!("macos config");
                } else {
                    unreachable!("Unsupported OS config");
                }
            }),
            temp_dir: PathBuf::from({
                if cfg!(target_os = "linux") {
                    "/home/alex/.cache/BlenderLauncher"
                } else if cfg!(target_os = "windows") {
                    todo!("windows config");
                } else if cfg!(target_os = "macos") {
                    todo!("macos config");
                } else {
                    unreachable!("Unsupported OS config");
                }
            }),
            interface: Interface::TUI,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Interface {
    GUI,
    TUI,
}
