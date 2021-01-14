//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Os, Package},
    releases::ReleaseType,
    settings::PROJECT_DIRS,
};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime, Utc};
use derive_deref::{Deref, DerefMut};
use select::predicate::{Class, Name};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Branched(Vec<Package>);

#[async_trait]
impl ReleaseType for Branched {
    async fn fetch() -> Self {
        let document = get_document("https://builder.blender.org/download/branches/").await;
        let mut branched = Branched::default();

        for build in document.find(Class("os")) {
            let targ_os = if cfg!(target_os = "linux") {
                "Linux"
            } else if cfg!(target_os = "windows") {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "macOS"
            } else {
                unreachable!("Unsupported OS config");
            };

            let o = build.find(Class("build")).next().unwrap().text();
            if !o.contains(targ_os) {
                continue;
            }

            let mut package = Package::default();

            package.build = Build::Branched(
                build
                    .find(Class("build-var"))
                    .next()
                    .unwrap()
                    .text()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .to_string(),
            );

            package.version = build
                .find(Class("name"))
                .next()
                .unwrap()
                .text()
                .split_whitespace()
                .skip(1)
                .next()
                .unwrap()
                .to_string();

            package.date = {
                let mut original_date = build.find(Name("small")).next().unwrap().text();
                let original_date: String = original_date
                    .drain(..original_date.find('-').unwrap())
                    .collect();

                let this_year = format!("{}-{}", original_date, Utc::today().year());
                let package_date =
                    NaiveDateTime::parse_from_str(&this_year, "%B %d, %T-%Y").unwrap();

                if package_date > Utc::now().naive_utc() {
                    let last_year = format!("{}-{}", original_date, Utc::today().year() - 1);
                    NaiveDateTime::parse_from_str(&last_year, "%B %d, %T-%Y").unwrap()
                } else {
                    package_date
                }
            };

            package.commit = build
                .find(Name("small"))
                .next()
                .unwrap()
                .text()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();

            package.url = format!(
                "https://builder.blender.org{}",
                build.find(Name("a")).next().unwrap().attr("href").unwrap()
            );

            package.name = get_file_stem(&package.url).to_string();

            package.os = {
                if o.contains("Linux") {
                    Os::Linux
                } else if o.contains("Windows") {
                    Os::Windows
                } else if o.contains("macOS") {
                    Os::MacOs
                } else {
                    unreachable!("Unexpected OS");
                }
            };

            branched.push(package);
        }

        branched.sort();
        branched
    }

    fn get_name(&self) -> String {
        String::from("branched")
    }

    fn get_db_path(&self) -> PathBuf {
        PROJECT_DIRS
            .config_dir()
            .to_path_buf()
            .join("branched_db.bin")
    }
}
