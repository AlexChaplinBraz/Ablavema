//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    fs::{create_dir_all, File},
    path::PathBuf,
    sync::{atomic::AtomicBool, RwLock},
    time::Duration,
    time::SystemTime,
};

const CONFIG_NAME: &str = "config.bin";

pub static LAUNCH_GUI: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("", "", "BlenderLauncher").unwrap();
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
    pub static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::init());
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
    pub last_update_time: SystemTime,
}

impl Settings {
    pub fn init() -> Self {
        let settings = if !CONFIG_PATH.exists() {
            let default = Settings::default();
            let conf_file = File::create(&*CONFIG_PATH).unwrap();
            bincode::serialize_into(conf_file, &default).unwrap();
            default
        } else {
            let conf_file = File::open(&*CONFIG_PATH).unwrap();
            let settings: Settings = bincode::deserialize_from(conf_file).unwrap_or_else(|_| {
                let default = Settings::default();
                let conf_file = File::create(&*CONFIG_PATH).unwrap();
                bincode::serialize_into(conf_file, &default).unwrap();
                default
            });
            settings
        };

        create_dir_all(&settings.packages_dir).unwrap();
        create_dir_all(&settings.releases_db.parent().unwrap()).unwrap();
        create_dir_all(&settings.cache_dir).unwrap();

        settings
    }

    pub fn save(&self) {
        let conf_file = File::create(&*CONFIG_PATH).unwrap();
        bincode::serialize_into(conf_file, self).unwrap();
    }
}

impl Default for Settings {
    fn default() -> Self {
        let minutes_between_updates = 60;

        let current_exe = current_exe().unwrap();
        let portable_path = current_exe.parent().unwrap().to_path_buf();
        let portable_file = portable_path.join("portable");

        Self {
            default_package: String::new(),
            use_latest_as_default: true,
            check_updates_at_launch: true,
            minutes_between_updates,
            update_daily: true,
            update_experimental: true,
            update_stable: true,
            update_lts: true,
            keep_only_latest_daily: false,
            keep_only_latest_experimental: false,
            keep_only_latest_stable: false,
            keep_only_latest_lts: false,
            packages_dir: {
                if portable_file.exists() {
                    portable_path.join("packages")
                } else {
                    PROJECT_DIRS.data_dir().to_path_buf()
                }
            },
            releases_db: {
                if portable_file.exists() {
                    portable_path.join("releases_db.bin")
                } else {
                    PROJECT_DIRS.config_dir().join("releases_db.bin")
                }
            },
            cache_dir: {
                if portable_file.exists() {
                    portable_path.join("cache")
                } else {
                    PROJECT_DIRS.cache_dir().to_path_buf()
                }
            },
            last_update_time: SystemTime::now()
                .checked_sub(Duration::from_secs(minutes_between_updates * 60))
                .unwrap_or_else(|| SystemTime::now()),
        }
    }
}
