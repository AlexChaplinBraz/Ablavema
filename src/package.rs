use crate::settings::get_setting;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use iced::button;
use serde::{Deserialize, Serialize};
use std::{fs::remove_dir_all, mem};
use timeago::{self, TimeUnit::Minutes};
use versions::Versioning;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Package {
    // TODO: Add "label" field so users can describe what a package is for if needed.
    pub version: Versioning,
    pub name: String,
    pub build: Build,
    pub date: NaiveDateTime,
    pub commit: String,
    pub url: String,
    pub os: Os,
    pub changelog: Vec<Change>,
    pub bookmarked: bool,
    #[serde(skip)]
    pub bookmark_button: button::State,
    #[serde(skip)]
    pub state: PackageState,
    #[serde(skip)]
    pub status: PackageStatus,
}

impl Package {
    pub fn get_formatted_date_time(&self) -> String {
        let mut formatter = timeago::Formatter::new();
        formatter.num_items(2);
        formatter.min_unit(Minutes);
        let duration = Utc::now().naive_utc().signed_duration_since(self.date);
        format!(
            "{} ({})",
            self.date.format("%B %d, %Y - %T"),
            // TODO: Properly get date-time based on timezone.
            // It worked miraculously for me all this time, but now `Utc::now()` gives me
            // a time one hour behind the date scraped from blender.org so this is inaccurate.
            // I should switch from NaiveDateTime to DateTime so I can properly calculate time.
            formatter.convert(duration.to_std().unwrap_or_default())
        )
    }

    pub fn remove(&self) {
        let path = get_setting().packages_dir.join(&self.name);
        let _ = remove_dir_all(path);
        println!("Removed: {}", self.name);
    }

    pub fn take(&mut self) -> Self {
        mem::take(self)
    }
}

impl Default for Package {
    fn default() -> Self {
        Package {
            version: Versioning::default(),
            name: String::default(),
            build: Build::Archived,
            date: NaiveDateTime::new(
                NaiveDate::from_ymd(1999, 12, 31),
                NaiveTime::from_hms(23, 59, 59),
            ),
            commit: String::default(),
            url: String::default(),
            os: Os::Linux,
            changelog: Vec::default(),
            bookmarked: false,
            bookmark_button: Default::default(),
            state: PackageState::default(),
            status: PackageStatus::default(),
        }
    }
}

impl Eq for Package {}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.build {
            Build::Daily(_) | Build::Experimental(_) => self
                .build
                .cmp(&other.build)
                .then(self.date.cmp(&other.date).reverse()),
            Build::Stable | Build::Lts | Build::Archived => {
                Ord::cmp(&self.version, &other.version).reverse()
            }
        }
    }
}

impl PartialEq for Package {
    // TODO: Consider what to do in case of having the same package name but different date.
    // Not really to be solved here, but I remember once where there were no commits in the
    // daily build for an entire day so it was the same package name but with a different date.
    // This would only bring trouble when trying to have both of them installed, but ultimately
    // being the same package means the worst that could happen is that it updates for no gain
    // whatsoever, removing the older package.
    fn eq(&self, other: &Self) -> bool {
        match self.build {
            Build::Daily(_) | Build::Experimental(_) => {
                self.build == other.build && self.date == other.date
            }
            Build::Stable | Build::Lts | Build::Archived => {
                self.name == other.name && self.version == other.version
            }
        }
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Build {
    Daily(String),
    Experimental(String),
    Stable,
    Lts,
    Archived,
}

impl std::fmt::Display for Build {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let printable = match self {
            Build::Daily(s) | Build::Experimental(s) => s,
            Build::Stable => "Stable Release",
            Build::Lts => "LTS Release",
            Build::Archived => "Archived Release",
        };
        write!(f, "{}", printable)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Change {
    pub text: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub enum PackageState {
    Fetched {
        install_button: button::State,
    },
    Downloading {
        progress: f32,
        cancel_button: button::State,
    },
    Extracting {
        progress: f32,
        cancel_button: button::State,
    },
    Installed {
        open_button: button::State,
        open_file_button: button::State,
        set_default_button: button::State,
        remove_button: button::State,
    },
    Errored {
        error_message: String,
        retry_button: button::State,
    },
}

impl PackageState {
    pub fn default_installed() -> Self {
        PackageState::Installed {
            open_button: Default::default(),
            open_file_button: Default::default(),
            set_default_button: Default::default(),
            remove_button: Default::default(),
        }
    }
}

impl Default for PackageState {
    fn default() -> Self {
        Self::Fetched {
            install_button: button::State::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PackageStatus {
    Update,
    New,
    Old,
}

impl Default for PackageStatus {
    fn default() -> Self {
        Self::Old
    }
}
