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
pub struct Daily(Vec<Package>);

#[async_trait]
impl ReleaseType for Daily {
    async fn fetch() -> Self {
        let document = get_document("https://builder.blender.org/download/").await;
        let mut daily = Daily::default();

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

            package.build = Build::Daily(build.find(Class("build-var")).next().unwrap().text());

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

            let mut date = build.find(Name("small")).next().unwrap().text();
            let mut date: String = date.drain(..date.find('-').unwrap()).collect();
            date.push_str(&format!("-{}", Utc::today().year()));
            package.date = NaiveDateTime::parse_from_str(&date, "%B %d, %T-%Y").unwrap();

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

            daily.push(package);
        }

        daily.sort();
        daily
    }

    fn get_name(&self) -> String {
        String::from("daily")
    }

    fn get_db_path(&self) -> PathBuf {
        PROJECT_DIRS.config_dir().to_path_buf().join("daily_db.bin")
    }
}
