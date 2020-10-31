//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![allow(dead_code, unused_imports, unused_variables)]
use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    packages_path: PathBuf,
    temp_path: PathBuf,
    interface: Interface,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
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
        if conf.exists() {
            settings.merge(File::new(&config_path, FileFormat::Toml))?;
        } else {
            let default = Settings::default();
            std::fs::create_dir_all(conf.parent().unwrap()).unwrap();
            let mut conf_file = std::fs::File::create(conf).unwrap();

            conf_file
                .write_all(toml::to_string(&default).unwrap().as_bytes())
                .unwrap();

            settings.merge(File::new(&config_path, FileFormat::Toml))?;
        }

        settings.try_into()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            packages_path: PathBuf::from({
                if cfg!(target_os = "linux") {
                    "/otp/BlenderLauncher"
                } else if cfg!(target_os = "windows") {
                    todo!("windows config");
                } else if cfg!(target_os = "macos") {
                    todo!("macos config");
                } else {
                    unreachable!("Unsupported OS config");
                }
            }),
            temp_path: PathBuf::from({
                if cfg!(target_os = "linux") {
                    "/tmp/BlenderLauncher"
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
enum Interface {
    GUI,
    TUI,
}
