//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    package::{Build, Package},
    settings::{get_setting, save_settings, set_setting},
};
use bincode;
use derive_deref::{Deref, DerefMut};
use std::fs::{read_dir, remove_dir_all, File};

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Installed(Vec<Package>);

impl Installed {
    pub fn fetch(&mut self) {
        self.clear();

        for entry in read_dir(&get_setting().packages_dir).unwrap() {
            let dir = entry.unwrap();
            let mut package_info = dir.path();
            package_info.push("package_info.bin");

            if package_info.exists() {
                let file = File::open(&package_info).unwrap();
                match bincode::deserialize_from(file) {
                    Ok(package) => {
                        self.push(package);
                    }
                    Err(_) => {
                        remove_dir_all(package_info.parent().unwrap()).unwrap();
                    }
                }
            }
        }

        self.sort_by_key(|x| x.date.clone());
        self.reverse();
    }

    pub fn update_default(&self) {
        if get_setting().use_latest_as_default && get_setting().default_package.is_some() {
            let default_package = get_setting().default_package.clone().unwrap();
            let new_default = self
                .iter()
                .find(|package| package.build == default_package.build)
                .unwrap();

            if new_default.version == default_package.version
                && new_default.date > default_package.date
            {
                set_setting().default_package = Some(new_default.clone());
                save_settings();

                println!(
                    "Installed an update for the default package, switched from:\n{} | {}\nTo:\n{} | {}",
                    default_package.name, default_package.date, new_default.name, new_default.date
                );
            }
        }
    }

    pub fn remove_old_packages(&self) -> (bool, bool) {
        let mut daily_removed = false;
        let mut branched_removed = false;

        if get_setting().keep_only_latest_daily
            || get_setting().keep_only_latest_branched
            || get_setting().keep_only_latest_stable
            || get_setting().keep_only_latest_lts
        {
            let mut daily_count = Vec::new();
            let mut branched_count = Vec::new();
            let mut stable_count = 0;
            let mut lts_count = Vec::new();
            for package in self.iter() {
                match &package.build {
                    Build::Daily(s) if get_setting().keep_only_latest_daily => {
                        daily_count.push((package.version.clone(), s.clone()));
                        if daily_count
                            .iter()
                            .filter(|(v, n)| v == &package.version && n == s)
                            .count()
                            > 1
                        {
                            package.remove();
                            daily_removed = true;
                        }
                    }
                    Build::Branched(s) if get_setting().keep_only_latest_branched => {
                        branched_count.push((package.version.clone(), s.clone()));
                        if branched_count
                            .iter()
                            .filter(|(v, n)| v == &package.version && n == s)
                            .count()
                            > 1
                        {
                            package.remove();
                            branched_removed = true;
                        }
                    }
                    Build::Stable if get_setting().keep_only_latest_stable => {
                        stable_count += 1;
                        if stable_count > 1 {
                            package.remove();
                        }
                    }
                    Build::Lts if get_setting().keep_only_latest_lts => {
                        lts_count.push(&package.version);
                        if lts_count
                            .iter()
                            .filter(|v| {
                                v.nth(0).unwrap() == package.version.nth(0).unwrap()
                                    && v.nth(1).unwrap() == package.version.nth(1).unwrap()
                            })
                            .count()
                            > 1
                        {
                            package.remove();
                        }
                    }
                    _ => continue,
                }
            }
        }

        (daily_removed, branched_removed)
    }

    pub fn remove_all(&mut self) {
        for package in self.iter() {
            package.remove();
        }
    }

    pub fn remove_daily(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::Daily { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_branched(&mut self) {
        for package in self.iter() {
            if matches!(package.build, Build::Branched { .. }) {
                package.remove();
            }
        }
    }

    pub fn remove_stable(&mut self) {
        for package in self.iter() {
            if package.build == Build::Stable {
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

    pub fn remove_archived(&mut self) {
        for package in self.iter() {
            if package.build == Build::Archived {
                package.remove();
            }
        }
    }
}
