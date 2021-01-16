//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{gui::style::Theme, package::Package};
use bincode;
use device_query::Keycode;
use directories_next::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    fs::{create_dir_all, File},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock,
    },
    time::{Duration, SystemTime},
};

const CONFIG_NAME: &str = "config.bin";
static PORTABLE: AtomicBool = AtomicBool::new(false);
// TODO: Use this to lock GUI buttons and CLI functionality.
pub static CAN_CONNECT: AtomicBool = AtomicBool::new(true);
pub static ONLY_CLI: AtomicBool = AtomicBool::new(true);
pub static LAUNCH_GUI: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref PROJECT_DIRS: ProjectDirs =
        ProjectDirs::from("", "", "BlenderLauncher").unwrap();
    static ref PORTABLE_PATH: PathBuf = current_exe().unwrap().parent().unwrap().to_path_buf();
    pub static ref CONFIG_PATH: PathBuf = {
        if PORTABLE_PATH.join("portable").exists() {
            PORTABLE.store(true, Ordering::Relaxed);
            PORTABLE_PATH.join(CONFIG_NAME)
        } else {
            let config_path = PROJECT_DIRS.config_dir().to_path_buf();
            create_dir_all(&config_path).unwrap();
            config_path.join(CONFIG_NAME)
        }
    };
    pub static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::init());
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub default_package: Option<Package>,
    pub bypass_launcher: bool,
    pub modifier_key: ModifierKey,
    pub use_latest_as_default: bool,
    pub check_updates_at_launch: bool,
    pub minutes_between_updates: u64,
    pub update_daily: bool,
    pub update_branched: bool,
    pub update_stable: bool,
    pub update_lts: bool,
    pub keep_only_latest_daily: bool,
    pub keep_only_latest_branched: bool,
    pub keep_only_latest_stable: bool,
    pub keep_only_latest_lts: bool,
    pub packages_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub releases_db: PathBuf,
    pub last_update_time: SystemTime,
    pub theme: Theme,
}

impl Settings {
    fn init() -> Self {
        let mut settings = if !CONFIG_PATH.exists() {
            let default = Settings::default();
            let conf_file = File::create(&*CONFIG_PATH).unwrap();
            bincode::serialize_into(conf_file, &default).unwrap();
            default
        } else {
            let conf_file = File::open(&*CONFIG_PATH).unwrap();
            let settings: Settings = bincode::deserialize_from(conf_file).unwrap_or_else(|_| {
                // This is in case the Settings struct changed,
                // which would just the settings with defaults.
                let default = Settings::default();
                let conf_file = File::create(&*CONFIG_PATH).unwrap();
                bincode::serialize_into(conf_file, &default).unwrap();
                default
            });
            settings
        };

        if PORTABLE.load(Ordering::Relaxed) {
            settings.packages_dir = PORTABLE_PATH.join("packages");
            settings.releases_db = PORTABLE_PATH.join("releases_db.bin");
            settings.cache_dir = PORTABLE_PATH.join("cache");

            create_dir_all(&settings.packages_dir).unwrap();
            create_dir_all(&settings.releases_db.parent().unwrap()).unwrap();
            create_dir_all(&settings.cache_dir).unwrap();
        } else {
            create_dir_all(&settings.packages_dir).unwrap();
            create_dir_all(&settings.releases_db.parent().unwrap()).unwrap();
            create_dir_all(&settings.cache_dir).unwrap();
        }

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

        Self {
            default_package: None,
            bypass_launcher: false,
            modifier_key: ModifierKey::Shift,
            use_latest_as_default: true,
            check_updates_at_launch: true,
            minutes_between_updates,
            update_daily: true,
            update_branched: true,
            update_stable: true,
            update_lts: true,
            keep_only_latest_daily: false,
            keep_only_latest_branched: false,
            keep_only_latest_stable: false,
            keep_only_latest_lts: false,
            // TODO: Let the user change these locations.
            packages_dir: PROJECT_DIRS.data_local_dir().to_path_buf(),
            releases_db: PROJECT_DIRS.config_dir().join("releases_db.bin"),
            cache_dir: PROJECT_DIRS.cache_dir().to_path_buf(),
            last_update_time: SystemTime::now()
                .checked_sub(Duration::from_secs(minutes_between_updates * 60))
                .unwrap_or_else(|| SystemTime::now()),
            theme: Theme::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModifierKey {
    Shift,
    Control,
    Alt,
}

impl ModifierKey {
    pub const ALL: [ModifierKey; 3] = [ModifierKey::Shift, ModifierKey::Control, ModifierKey::Alt];

    pub fn get_keycode(&self) -> Keycode {
        match self {
            ModifierKey::Shift => Keycode::LShift,
            ModifierKey::Control => Keycode::LControl,
            ModifierKey::Alt => Keycode::LAlt,
        }
    }
}

impl std::fmt::Display for ModifierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let printable = match self {
            ModifierKey::Shift => "shift",
            ModifierKey::Control => "ctrl",
            ModifierKey::Alt => "alt",
        };
        write!(f, "{}", printable)
    }
}
