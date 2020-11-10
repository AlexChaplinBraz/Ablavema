//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::releases::*;
use crate::settings::*;
use indicatif::MultiProgress;
use std::{
    error::Error,
    fs::File,
    fs::{self, create_dir_all},
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct Installed(Vec<Package>);

impl Installed {
    pub fn new(settings: &Settings) -> Result<Self, Box<dyn Error>> {
        let mut installed = Installed(Vec::new());

        installed.check(&settings)?;

        Ok(installed)
    }

    pub fn check(&mut self, settings: &Settings) -> Result<(), Box<dyn Error>> {
        create_dir_all(&settings.packages_dir).unwrap();

        for entry in fs::read_dir(&settings.packages_dir)? {
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
            let mut package_info = settings.packages_dir.join(&package.name);
            package_info.push("package_info.bin");

            package_info.exists()
        });

        self.sort_by_key(|x| x.date.clone());
        self.reverse();

        Ok(())
    }

    pub async fn update(
        &mut self,
        settings: &mut Settings,
        releases: &mut Releases,
    ) -> Result<(), Box<dyn Error>> {
        let mut packages_to_install = Vec::new();

        if settings.update_stable {
            releases.fetch_latest_stable(&settings).await;

            let latest_stable = releases.latest_stable.iter().next().unwrap();
            if !self.contains(latest_stable) {
                packages_to_install.push(latest_stable.clone());
                println!("Found: {} | {}", latest_stable.name, latest_stable.date);
            }
        }

        if settings.update_lts {
            releases.fetch_lts_releases(&settings).await;

            let latest_lts = releases.lts_releases.iter().next().unwrap();
            if !self.contains(latest_lts) {
                packages_to_install.push(latest_lts.clone());
                println!("Found: {} | {}", latest_lts.name, latest_lts.date);
            }
        }

        if settings.update_daily {
            releases.fetch_latest_daily(&settings).await;

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

        if settings.update_experimental {
            releases.fetch_experimental_branches(&settings).await;

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
                install_completion.push(package.install(&settings, &multi_progress).await.unwrap());
            }
            multi_progress.join().unwrap();
            for handle in install_completion {
                handle.await.unwrap();
            }

            self.check(&settings).unwrap();

            if settings.default_package.is_empty() {
                println!(
                    "No default package found, please select a package to open .blend files with."
                );
            } else if settings.use_latest_as_default {
                let old_default = self
                    .iter()
                    .find(|p| p.name == settings.default_package)
                    .unwrap();
                let new_default = self.iter().find(|p| p.build == old_default.build).unwrap();

                if old_default == new_default {
                    println!(
                        "No updates found for the default package:\n{} | {}",
                        old_default.name, old_default.date
                    );
                } else {
                    settings.default_package = new_default.name.clone();
                    settings.save().unwrap();

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
                        if stable_count > 1 && settings.keep_only_latest_stable {
                            package.remove(&settings).await?;
                        }
                    }
                    Build::LTS => {
                        lts_count += 1;
                        if lts_count > 1 && settings.keep_only_latest_lts {
                            package.remove(&settings).await?;
                        }
                    }
                    Build::Daily(s) => {
                        daily_count.push(s.clone());
                        if daily_count.iter().filter(|&n| n == s).count() > 1
                            && settings.keep_only_latest_daily
                        {
                            package.remove(&settings).await?;
                        }
                    }
                    Build::Experimental(s) => {
                        experimental_count.push(s.clone());
                        if experimental_count.iter().filter(|&n| n == s).count() > 1
                            && settings.keep_only_latest_experimental
                        {
                            package.remove(&settings).await?;
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
                self.check(&settings).unwrap();
            }
        }

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
