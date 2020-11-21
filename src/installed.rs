//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::releases::*;
use crate::settings::*;
use indicatif::MultiProgress;
use std::{
    error::Error,
    fs,
    fs::File,
    ops::{Deref, DerefMut},
    process::Command,
    time::SystemTime,
};

#[derive(Debug)]
pub struct Installed(Vec<Package>);

impl Installed {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut installed = Installed(Vec::new());

        installed.check()?;

        Ok(installed)
    }

    pub fn check(&mut self) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(&SETTINGS.read().unwrap().packages_dir)? {
            let dir = entry?;
            let mut package_info = dir.path();
            package_info.push("package_info.bin");

            if package_info.exists() {
                let file = File::open(&package_info)?;
                let package: Package = bincode::deserialize_from(file)?;
                if !self.contains(&package) {
                    self.push(package);
                }
            }
        }

        self.retain(|package| {
            let mut package_info = SETTINGS.read().unwrap().packages_dir.join(&package.name);
            package_info.push("package_info.bin");

            package_info.exists()
        });

        self.sort_by_key(|x| x.date.clone());
        self.reverse();

        Ok(())
    }

    pub async fn update(&mut self, releases: &mut Releases) -> Result<(), Box<dyn Error>> {
        SETTINGS.write().unwrap().last_update_time = SystemTime::now();
        SETTINGS.read().unwrap().save();

        let mut packages_to_install = Vec::new();

        if SETTINGS.read().unwrap().update_stable {
            releases.fetch_latest_stable().await?;

            let latest_stable = releases.latest_stable.iter().next().unwrap();
            if !self.contains(latest_stable)
                && self
                    .iter()
                    .find(|p| p.build == latest_stable.build)
                    .is_some()
            {
                packages_to_install.push(latest_stable.clone());
                println!("Found: {} | {}", latest_stable.name, latest_stable.date);
            }
        }

        if SETTINGS.read().unwrap().update_lts {
            releases.fetch_lts_releases().await?;

            let latest_lts = releases.lts_releases.iter().next().unwrap();
            if !self.contains(latest_lts)
                && self.iter().find(|p| p.build == latest_lts.build).is_some()
            {
                packages_to_install.push(latest_lts.clone());
                println!("Found: {} | {}", latest_lts.name, latest_lts.date);
            }
        }

        if SETTINGS.read().unwrap().update_daily {
            releases.fetch_latest_daily().await?;

            for fetched_package in &releases.latest_daily {
                if !self.contains(fetched_package)
                    && self
                        .iter()
                        .find(|p| p.build == fetched_package.build)
                        .is_some()
                {
                    packages_to_install.push(fetched_package.clone());
                    println!("Found: {} | {}", fetched_package.name, fetched_package.date);
                }
            }
        }

        if SETTINGS.read().unwrap().update_experimental {
            releases.fetch_experimental_branches().await?;

            for fetched_package in &releases.experimental_branches {
                if !self.contains(fetched_package)
                    && self
                        .iter()
                        .find(|p| p.build == fetched_package.build)
                        .is_some()
                {
                    packages_to_install.push(fetched_package.clone());
                    println!("Found: {} | {}", fetched_package.name, fetched_package.date);
                }
            }
        }

        if packages_to_install.is_empty() {
            println!("No new packages found.");
        } else {
            let multi_progress = MultiProgress::new();
            let mut install_completion = Vec::new();
            for package in packages_to_install {
                install_completion.push(package.install(&multi_progress).await?);
            }
            multi_progress.join().unwrap();
            for handle in install_completion {
                handle.await.unwrap();
            }

            self.check()?;

            if SETTINGS.read().unwrap().default_package.is_empty() {
                println!(
                    "No default package found, please select a package to open .blend files with."
                );
            } else if SETTINGS.read().unwrap().use_latest_as_default {
                let old_default = self
                    .iter()
                    .find(|p| p.name == SETTINGS.read().unwrap().default_package)
                    .unwrap();
                let new_default = self.iter().find(|p| p.build == old_default.build).unwrap();

                if old_default == new_default {
                    println!(
                        "No updates found for the default package:\n{} | {}",
                        old_default.name, old_default.date
                    );
                } else {
                    SETTINGS.write().unwrap().default_package = new_default.name.clone();
                    SETTINGS.read().unwrap().save();

                    println!(
                        "Found an update for the default package, switched from:\n{} | {}\nTo:\n{} | {}",
                        old_default.name, old_default.date, new_default.name, new_default.date
                    );
                }
            }

            let mut stable_count = 0;
            let mut lts_count = 0;
            let mut daily_count = Vec::new();
            let mut experimental_count = Vec::new();
            for package in &**self {
                match &package.build {
                    Build::Official => continue,
                    Build::Stable => {
                        stable_count += 1;
                        if stable_count > 1 && SETTINGS.read().unwrap().keep_only_latest_stable {
                            package.remove().await?;
                        }
                    }
                    Build::LTS => {
                        lts_count += 1;
                        if lts_count > 1 && SETTINGS.read().unwrap().keep_only_latest_lts {
                            package.remove().await?;
                        }
                    }
                    Build::Daily(s) => {
                        daily_count.push(s.clone());
                        if daily_count.iter().filter(|&n| n == s).count() > 1
                            && SETTINGS.read().unwrap().keep_only_latest_daily
                        {
                            package.remove().await?;
                        }
                    }
                    Build::Experimental(s) => {
                        experimental_count.push(s.clone());
                        if experimental_count.iter().filter(|&n| n == s).count() > 1
                            && SETTINGS.read().unwrap().keep_only_latest_experimental
                        {
                            package.remove().await?;
                        }
                    }
                    Build::None => unreachable!("Unexpected build type"),
                }
            }

            if !daily_count.is_empty()
                || !experimental_count.is_empty()
                || !lts_count == 0
                || !stable_count == 0
            {
                self.check()?;
            }
        }

        Ok(())
    }

    pub fn open_blender() -> Result<(), Box<dyn Error>> {
        Command::new(
            SETTINGS
                .read()
                .unwrap()
                .packages_dir
                .join(&SETTINGS.read().unwrap().default_package)
                .join({
                    if cfg!(target_os = "linux") {
                        "blender"
                    } else if cfg!(target_os = "windows") {
                        "blender.exe"
                    } else if cfg!(target_os = "macos") {
                        todo!("macos executable");
                    } else {
                        unreachable!("Unsupported OS");
                    }
                }),
        )
        .status()?;

        Ok(())
    }

    pub fn open_blender_with_file(file: &str) -> Result<(), Box<dyn Error>> {
        Command::new(
            SETTINGS
                .read()
                .unwrap()
                .packages_dir
                .join(&SETTINGS.read().unwrap().default_package)
                .join({
                    if cfg!(target_os = "linux") {
                        "blender"
                    } else if cfg!(target_os = "windows") {
                        "blender.exe"
                    } else if cfg!(target_os = "macos") {
                        todo!("macos executable");
                    } else {
                        unreachable!("Unsupported OS");
                    }
                }),
        )
        .arg(file)
        .status()?;

        Ok(())
    }
}

impl Deref for Installed {
    type Target = Vec<Package>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Installed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
