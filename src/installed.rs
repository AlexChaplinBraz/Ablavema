//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![allow(dead_code, unused_imports, unused_variables)]
use crate::releases::*;
use crate::settings::*;
use std::{error::Error, fs};
use std::{
    fs::File,
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

        self.sort_by_key(|x| x.version.clone());
        self.reverse();

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
