use crate::settings::get_setting;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, fs::remove_dir_all, mem};
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
    #[serde(skip)]
    pub state: PackageState,
    #[serde(skip)]
    pub status: PackageStatus,
    #[serde(skip)]
    pub index: usize,
    #[serde(skip)]
    pub build_type: BuildType,
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
            build: Build::StableArchive,
            date: NaiveDateTime::new(
                NaiveDate::from_ymd(1999, 12, 31),
                NaiveTime::from_hms(23, 59, 59),
            ),
            commit: String::default(),
            url: String::default(),
            os: Os::Linux,
            changelog: Vec::default(),
            state: PackageState::default(),
            status: PackageStatus::default(),
            index: 0,
            build_type: BuildType::None,
        }
    }
}

impl Eq for Package {}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.build {
            Build::DailyLatest(_)
            | Build::DailyArchive(_)
            | Build::ExperimentalLatest(_)
            | Build::ExperimentalArchive(_)
            | Build::PatchLatest(_)
            | Build::PatchArchive(_) => self
                .build
                .cmp(&other.build)
                .then(self.date.cmp(&other.date).reverse()),
            Build::StableLatest | Build::StableArchive | Build::Lts => {
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
            Build::DailyLatest(_)
            | Build::DailyArchive(_)
            | Build::ExperimentalLatest(_)
            | Build::ExperimentalArchive(_)
            | Build::PatchLatest(_)
            | Build::PatchArchive(_) => self.build == other.build && self.date == other.date,
            Build::StableLatest | Build::StableArchive | Build::Lts => {
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
    DailyLatest(String),
    DailyArchive(String),
    ExperimentalLatest(String),
    ExperimentalArchive(String),
    PatchLatest(String),
    PatchArchive(String),
    StableLatest,
    StableArchive,
    Lts,
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
    Fetched,
    Downloading { progress: f32 },
    Extracting { progress: f32 },
    Installed,
    Errored { message: String },
}

impl Default for PackageState {
    fn default() -> Self {
        Self::Fetched
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BuildType {
    Daily {
        latest: bool,
        archive: bool,
        name: String,
    },
    Experimental {
        latest: bool,
        archive: bool,
        name: String,
    },
    Patch {
        latest: bool,
        archive: bool,
        name: String,
    },
    Stable {
        latest: bool,
        archive: bool,
        lts: bool,
    },
    None,
}

impl BuildType {
    pub fn update(&mut self, build: &Build) {
        match self {
            BuildType::Daily {
                latest,
                archive,
                name: _,
            } => match build {
                Build::DailyLatest(_) => *latest = true,
                Build::DailyArchive(_) => *archive = true,
                _ => (),
            },
            BuildType::Experimental {
                latest,
                archive,
                name: _,
            } => match build {
                Build::ExperimentalLatest(_) => *latest = true,
                Build::ExperimentalArchive(_) => *archive = true,
                _ => (),
            },
            BuildType::Patch {
                latest,
                archive,
                name: _,
            } => match build {
                Build::PatchLatest(_) => *latest = true,
                Build::PatchArchive(_) => *archive = true,
                _ => (),
            },
            BuildType::Stable {
                latest,
                archive,
                lts,
            } => match build {
                Build::StableLatest => *latest = true,
                Build::StableArchive => *archive = true,
                Build::Lts => *lts = true,
                _ => (),
            },
            BuildType::None => match build {
                Build::DailyLatest(name) => {
                    *self = BuildType::Daily {
                        latest: true,
                        archive: false,
                        name: name.to_string(),
                    };
                }
                Build::DailyArchive(name) => {
                    *self = BuildType::Daily {
                        latest: false,
                        archive: true,
                        name: name.to_string(),
                    };
                }
                Build::ExperimentalLatest(name) => {
                    *self = BuildType::Experimental {
                        latest: true,
                        archive: false,
                        name: name.to_string(),
                    };
                }
                Build::ExperimentalArchive(name) => {
                    *self = BuildType::Experimental {
                        latest: false,
                        archive: true,
                        name: name.to_string(),
                    };
                }
                Build::PatchLatest(name) => {
                    *self = BuildType::Patch {
                        latest: true,
                        archive: false,
                        name: name.to_string(),
                    };
                }
                Build::PatchArchive(name) => {
                    *self = BuildType::Patch {
                        latest: false,
                        archive: true,
                        name: name.to_string(),
                    };
                }
                Build::StableLatest => {
                    *self = BuildType::Stable {
                        latest: true,
                        archive: false,
                        lts: false,
                    };
                }
                Build::StableArchive => {
                    *self = BuildType::Stable {
                        latest: false,
                        archive: true,
                        lts: false,
                    };
                }
                Build::Lts => {
                    *self = BuildType::Stable {
                        latest: false,
                        archive: false,
                        lts: true,
                    };
                }
            },
        }
    }
}

impl Default for BuildType {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for BuildType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildType::Daily {
                latest,
                archive,
                name,
            } => {
                if *latest && !archive {
                    write!(f, "Daily (latest): {}", name)
                } else if !latest && *archive {
                    write!(f, "Daily (archive): {}", name)
                } else {
                    write!(f, "Daily (latest and archive): {}", name)
                }
            }
            BuildType::Experimental {
                latest,
                archive,
                name,
            } => {
                if *latest && !archive {
                    write!(f, "Experimental (latest): {}", name)
                } else if !latest && *archive {
                    write!(f, "Experimental (archive): {}", name)
                } else {
                    write!(f, "Experimental (latest and archive): {}", name)
                }
            }
            BuildType::Patch {
                latest,
                archive,
                name,
            } => {
                if *latest && !archive {
                    write!(f, "Patch (latest): {}", name)
                } else if !latest && *archive {
                    write!(f, "Patch (archive): {}", name)
                } else {
                    write!(f, "Patch (latest and archive): {}", name)
                }
            }
            BuildType::Stable {
                latest,
                archive,
                lts,
            } => {
                let mut text = String::new();

                if *latest && *archive {
                    write!(text, "Stable (latest and archive)").unwrap();
                } else if *latest {
                    write!(text, "Stable (latest)").unwrap();
                } else if *archive {
                    write!(text, "Stable (archive)").unwrap();
                }
                if *lts {
                    if !text.is_empty() {
                        write!(text, " | ").unwrap();
                    }
                    write!(text, "Long-term Support").unwrap();
                }

                write!(f, "{}", text)
            }
            BuildType::None => unreachable!("uninitialised build type"),
        }
    }
}
