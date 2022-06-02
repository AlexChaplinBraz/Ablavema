use crate::{
    package::{Build, Package},
    settings::{get_setting, save_settings, set_setting},
};
use derive_deref::{Deref, DerefMut};
use ron::from_str;
use std::fs::{read_dir, read_to_string, remove_dir_all};

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Installed(Vec<Package>);

impl Installed {
    pub fn fetch(&mut self) {
        self.clear();

        for entry in read_dir(&get_setting().packages_dir).unwrap() {
            let dir = entry.unwrap();
            let mut package_info = dir.path();
            package_info.push("package_info.ron");

            if package_info.exists() {
                if let Ok(package_string) = read_to_string(&package_info) {
                    match from_str(&package_string) {
                        Ok(package) => self.push(package),
                        Err(e) => {
                            eprintln!(
                                "Error reading package info file: {}.\nRemoving installed package.",
                                e
                            );
                            remove_dir_all(package_info.parent().unwrap()).unwrap();
                        }
                    }
                }
            }
        }

        self.sort_by_key(|x| x.date);
        self.reverse();
    }

    pub fn update_default(&self) {
        if get_setting().use_latest_as_default && get_setting().default_package.is_some() {
            let default_package = get_setting().default_package.clone().unwrap();
            // TODO: Fix build comparison.
            // It's comparing Build, which may not be accurate because it may have been filtered
            // and installed with another Build due to BuildType. I could save the BuildType as
            // well and compare that, but it could get out of sync so I'm not sure what to do.
            if let Some(new_default) = self.iter().find(|package| {
                package.build == default_package.build
                    && package.version.nth(0).unwrap() == default_package.version.nth(0).unwrap()
                    && package.version.nth(1).unwrap() == default_package.version.nth(1).unwrap()
                    && package.version.nth(2).unwrap() >= default_package.version.nth(2).unwrap()
            }) {
                if new_default.date > default_package.date {
                    set_setting().default_package = Some(new_default.clone());
                    save_settings();

                    println!(
                            "Installed an update for the default package, switched from:\n{} | {}\nTo:\n{} | {}",
                            default_package.name, default_package.date, new_default.name, new_default.date
                        );
                }
            }
        }
    }

    pub fn remove_all(&mut self) {
        for package in self.iter() {
            package.remove();
        }
    }

    pub fn remove_daily_latest(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::DailyLatest { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_daily_archive(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::DailyArchive { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_experimental_latest(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::ExperimentalLatest { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_experimental_archive(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::ExperimentalArchive { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_patch_latest(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::PatchLatest { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_patch_archive(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::PatchArchive { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_stable_latest(&mut self) {
        for package in self.iter() {
            if package.build == Build::StableLatest {
                package.remove();
            }
        }
    }

    pub fn remove_stable_archive(&mut self) {
        for package in self.iter() {
            if package.build == Build::StableArchive {
                package.remove();
            }
        }
    }

    pub fn remove_lts(&mut self) {
        for package in self.iter() {
            if package.build == Build::Lts {
                package.remove();
            }
        }
    }
}
