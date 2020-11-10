//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![allow(dead_code, unused_imports, unused_variables)]
use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{env, error::Error};

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
    pub fn new() -> Result<(Self, String), ConfigError> {
        // TODO: Consider working directly with Config.
        let config_path;

        if cfg!(target_os = "linux") {
            let bl_env = env::var_os("BLENDER_LAUNCHER_CONFIG");
            let xdg_env = env::var_os("XDG_CONFIG_HOME");

            if bl_env.is_some() {
                config_path = bl_env.unwrap().to_str().unwrap().to_string();
            } else if xdg_env.is_some() {
                config_path = format!(
                    "{}/BlenderLauncher/config.toml",
                    xdg_env.unwrap().to_str().unwrap()
                );
            } else {
                config_path = format!(
                    "{}/.config/BlenderLauncher/config.toml",
                    env::var("HOME").unwrap()
                );
            }
        } else if cfg!(target_os = "windows") {
            todo!("windows config");
        } else if cfg!(target_os = "macos") {
            todo!("macos config");
        } else {
            unreachable!("Unsupported OS config");
        }

        let mut settings = Config::new();

        let conf = Path::new(&config_path);

        if !conf.exists() {
            let default = Settings::default();
            std::fs::create_dir_all(conf.parent().unwrap()).unwrap();
            let mut conf_file = std::fs::File::create(conf).unwrap();

            conf_file
                .write_all(toml::to_string(&default).unwrap().as_bytes())
                .unwrap();
        }

        settings.merge(File::new(&config_path, FileFormat::Toml))?;

        Ok((settings.try_into()?, config_path))
    }

    pub fn save(&self, config_path: &String) -> Result<(), Box<dyn Error>> {
        let toml = toml::to_string(self)?;
        let mut file = std::fs::File::create(config_path)?;
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
