use crate::{
    package::{BuildType, Package, PackageState, PackageStatus},
    settings::get_setting,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Filters {
    pub updates: bool,
    pub bookmarks: bool,
    pub installed: bool,
    pub all: bool,
    pub daily_latest: bool,
    pub daily_archive: bool,
    pub experimental_latest: bool,
    pub experimental_archive: bool,
    pub patch_latest: bool,
    pub patch_archive: bool,
    pub stable_latest: bool,
    pub stable_archive: bool,
    pub lts: bool,
}

impl Filters {
    pub fn matches(&self, package: &Package) -> bool {
        let mut matches = match &package.build_type {
            BuildType::Daily {
                latest,
                archive,
                name: _,
            } => self.daily_latest && *latest || self.daily_archive && *archive,
            BuildType::Experimental {
                latest,
                archive,
                name: _,
            } => self.experimental_latest && *latest || self.experimental_archive && *archive,
            BuildType::Patch {
                latest,
                archive,
                name: _,
            } => self.patch_latest && *latest || self.patch_archive && *archive,
            BuildType::Stable {
                latest,
                archive,
                lts,
            } => {
                self.stable_latest && *latest || self.stable_archive && *archive || self.lts && *lts
            }
            BuildType::None => unreachable!("uninitialised build type"),
        };

        if !matches {
            return false;
        }

        if self.updates {
            matches = package.status == PackageStatus::Update;
        }

        if !matches {
            return false;
        }

        if self.installed {
            matches = matches!(package.state, PackageState::Installed { .. });
        }

        if !matches {
            return false;
        }

        if self.bookmarks {
            matches = get_setting().bookmarks.contains(&package.name);
        }

        matches
    }

    pub fn refresh_all(&mut self) {
        self.all = self.daily_latest
            && self.daily_archive
            && self.experimental_latest
            && self.experimental_archive
            && self.patch_latest
            && self.patch_archive
            && self.stable_latest
            && self.stable_archive
            && self.lts
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            updates: false,
            bookmarks: false,
            installed: false,
            all: true,
            daily_latest: true,
            daily_archive: true,
            experimental_latest: true,
            experimental_archive: true,
            patch_latest: true,
            patch_archive: true,
            stable_latest: true,
            stable_archive: true,
            lts: true,
        }
    }
}
