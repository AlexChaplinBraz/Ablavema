//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{package::Package, settings::SETTINGS};
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
}
