use crate::{
    gui::{
        filters::Filters,
        sort_by::SortBy,
        style::Theme,
        tabs::{recent_files::RecentFiles, Tab},
    },
    package::Package,
};
use derive_deref::{Deref, DerefMut};
use device_query::Keycode;
use directories_next::ProjectDirs;
use lazy_static::{initialize, lazy_static};
use regex::Regex;
use ron::{
    from_str,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    env::var,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    time::{Duration, SystemTime},
};

pub fn init_settings() {
    initialize(&SETTINGS);
}

pub fn save_settings() {
    SETTINGS.read().unwrap().save()
}

pub fn get_setting() -> RwLockReadGuard<'static, Settings> {
    SETTINGS.read().unwrap()
}

pub fn set_setting() -> RwLockWriteGuard<'static, Settings> {
    SETTINGS.write().unwrap()
}

const CONFIG_NAME: &str = "config.ron";
pub const CONFIG_FILE_ENV: &str = "ABLAVEMA_CONFIG_FILE";
pub static PORTABLE: AtomicBool = AtomicBool::new(false);
pub static CAN_CONNECT: AtomicBool = AtomicBool::new(true);
pub static LAUNCH_GUI: AtomicBool = AtomicBool::new(false);
pub static FETCHING: AtomicBool = AtomicBool::new(false);
pub static INSTALLING: AtomicBool = AtomicBool::new(false);
// TODO: Consider making the text size user-adjustable.
// Would need for all elements and sizes to scale properly.
// Another requirement is for the window to remember its size and position.
pub const TEXT_SIZE: u16 = 16;

lazy_static! {
    pub static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("", "", "Ablavema").unwrap();
    static ref PORTABLE_PATH: PathBuf = current_exe().unwrap().parent().unwrap().to_path_buf();
    static ref CONFIG_PATH: PathBuf = {
        if PORTABLE_PATH.join("portable").exists() {
            PORTABLE.store(true, Ordering::Relaxed);
            PORTABLE_PATH.join(CONFIG_NAME)
        } else if let Ok(path) = var(CONFIG_FILE_ENV) {
            let config_path = PathBuf::from(path);
            create_dir_all(config_path.parent().unwrap()).unwrap();
            config_path
        } else {
            let config_path = PROJECT_DIRS.config_dir().to_path_buf();
            create_dir_all(&config_path).unwrap();
            config_path.join(CONFIG_NAME)
        }
    };
    static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::init());
    pub static ref ARCHIVE_DATE_RE: Regex = Regex::new(r"\d{2}-\w{3}-\d{4}\s\d{2}:\d{2}").unwrap();
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub recent_files: RecentFiles,
    pub bookmarks: Bookmarks,
    pub tab: Tab,
    pub default_package: Option<Package>,
    pub bypass_launcher: bool,
    pub modifier_key: ModifierKey,
    pub use_latest_as_default: bool,
    pub check_updates_at_launch: bool,
    pub minutes_between_updates: u64,
    pub update_daily_latest: bool,
    pub update_experimental_latest: bool,
    pub update_patch_latest: bool,
    pub update_stable_latest: bool,
    pub update_lts: bool,
    pub databases_dir: PathBuf,
    pub packages_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub last_update_time: SystemTime,
    pub filters: Filters,
    pub sort_by: SortBy,
    pub theme: Theme,
    pub self_updater: bool,
    pub check_self_updates_at_launch: bool,
}

impl Settings {
    fn init() -> Self {
        let mut settings: Settings = match read_to_string(&*CONFIG_PATH) {
            Ok(text) => match from_str(&text) {
                Ok(settings) => settings,
                Err(e) => {
                    eprintln!("Error reading config file: {}.\nUsing default settings.", e);
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };

        if PORTABLE.load(Ordering::Relaxed) {
            settings.databases_dir = PORTABLE_PATH.join("databases");
            settings.packages_dir = PORTABLE_PATH.join("packages");
            settings.cache_dir = PORTABLE_PATH.join("cache");
        }

        create_dir_all(&settings.databases_dir).unwrap();
        create_dir_all(&settings.packages_dir).unwrap();
        create_dir_all(&settings.cache_dir).unwrap();

        settings
    }

    fn save(&self) {
        let mut config_file = File::create(&*CONFIG_PATH).unwrap();
        let settings = to_string_pretty(&self, PrettyConfig::new()).unwrap();
        config_file.write_all(settings.as_bytes()).unwrap();
    }
}

impl Default for Settings {
    fn default() -> Self {
        let minutes_between_updates = 60;

        Self {
            recent_files: RecentFiles::default(),
            bookmarks: Bookmarks::default(),
            tab: Tab::default(),
            default_package: None,
            bypass_launcher: false,
            modifier_key: ModifierKey::Shift,
            use_latest_as_default: true,
            check_updates_at_launch: true,
            minutes_between_updates,
            update_daily_latest: true,
            update_experimental_latest: true,
            update_patch_latest: true,
            update_stable_latest: true,
            update_lts: true,
            databases_dir: PROJECT_DIRS.config_dir().to_path_buf(),
            packages_dir: PROJECT_DIRS.data_local_dir().to_path_buf(),
            cache_dir: PROJECT_DIRS.cache_dir().to_path_buf(),
            last_update_time: SystemTime::now()
                .checked_sub(Duration::from_secs(minutes_between_updates * 60))
                .unwrap_or_else(SystemTime::now),
            filters: Filters::default(),
            sort_by: SortBy::default(),
            theme: Theme::default(),
            self_updater: false,
            check_self_updates_at_launch: false,
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

#[derive(Debug, Default, Deref, DerefMut, Deserialize, Serialize)]
pub struct Bookmarks(Vec<String>);

impl Bookmarks {
    pub fn update(&mut self, package_name: String) {
        match self.iter().position(|bookmark| bookmark == &package_name) {
            Some(index) => {
                self.remove(index);
            }
            None => self.push(package_name),
        }
    }

    pub fn clean(&mut self, packages: &[Package]) {
        let mut indexes = Vec::new();

        for bookmark in self.iter() {
            if !packages.iter().any(|package| &package.name == bookmark) {
                if let Some(index) = self
                    .iter()
                    .position(|old_bookmark| old_bookmark == bookmark)
                {
                    indexes.push(index);
                }
            }
        }

        for (removed, index) in indexes.iter().enumerate() {
            self.remove(index - removed);
        }
    }
}
