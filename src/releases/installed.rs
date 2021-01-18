//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    package::{Build, Package},
    settings::SETTINGS,
};
use bincode;
use derive_deref::{Deref, DerefMut};
use std::fs::{read_dir, File};

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Installed(Vec<Package>);

impl Installed {
    pub fn fetch(&mut self) {
        self.clear();

        for entry in read_dir(&SETTINGS.read().unwrap().packages_dir).unwrap() {
            let dir = entry.unwrap();
            let mut package_info = dir.path();
            package_info.push("package_info.bin");

            if package_info.exists() {
                let file = File::open(&package_info).unwrap();
                // TODO: Remove directory if package_info.bin failed to deserialize.
                let package: Package = bincode::deserialize_from(file).unwrap();
                self.push(package);
            }
        }

        self.sort_by_key(|x| x.date.clone());
        self.reverse();
    }

    pub fn update_default(&self) {
        // TODO: Fix case where there's the same build but for different versions.
        // Seen it happen for daily builds, where there were two Alpha packages,
        // one for version 2.91.1 and another for 2.93.0 so it's something to consider.
        if SETTINGS.read().unwrap().use_latest_as_default
            && SETTINGS.read().unwrap().default_package.is_some()
        {
            let default_package = SETTINGS.read().unwrap().default_package.clone().unwrap();
            let new_default = self
                .iter()
                .find(|package| package.build == default_package.build)
                .unwrap();

            if new_default.date > default_package.date {
                SETTINGS.write().unwrap().default_package = Some(new_default.clone());
                SETTINGS.read().unwrap().save();

                println!(
                    "Installed an update for the default package, switched from:\n{} | {}\nTo:\n{} | {}",
                    default_package.name, default_package.date, new_default.name, new_default.date
                );
            }
        }
    }

    pub fn remove_old_packages(&self) {
        if SETTINGS.read().unwrap().keep_only_latest_daily
            || SETTINGS.read().unwrap().keep_only_latest_branched
            || SETTINGS.read().unwrap().keep_only_latest_stable
            || SETTINGS.read().unwrap().keep_only_latest_lts
        {
            let mut daily_count = Vec::new();
            let mut branched_count = Vec::new();
            let mut stable_count = 0;
            let mut lts_count = 0;
            for package in self.iter() {
                match &package.build {
                    Build::Daily(s) if SETTINGS.read().unwrap().keep_only_latest_daily => {
                        daily_count.push(s.clone());
                        if daily_count.iter().filter(|&n| n == s).count() > 1 {
                            package.remove();
                        }
                    }
                    Build::Branched(s) if SETTINGS.read().unwrap().keep_only_latest_branched => {
                        branched_count.push(s.clone());
                        if branched_count.iter().filter(|&n| n == s).count() > 1 {
                            package.remove();
                        }
                    }
                    Build::Stable if SETTINGS.read().unwrap().keep_only_latest_stable => {
                        stable_count += 1;
                        if stable_count > 1 {
                            package.remove();
                        }
                    }
                    Build::Lts if SETTINGS.read().unwrap().keep_only_latest_lts => {
                        lts_count += 1;
                        if lts_count > 1 {
                            package.remove();
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
}
