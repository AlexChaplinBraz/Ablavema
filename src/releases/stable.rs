//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Os, Package},
    releases::ReleaseType,
    settings::CONFIG_PATH,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use select::predicate::{Attr, Class, Name};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Stable(Vec<Package>);

#[async_trait]
impl ReleaseType for Stable {
    async fn fetch() -> Self {
        let url = "https://www.blender.org/download/";
        let document = get_document(url).await;
        let mut stable = Stable::default();
        let mut package = Package::default();

        let o = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            unreachable!("Unsupported OS config");
        };

        let node = document.find(Attr("id", o)).next().unwrap();

        package.version = node.find(Name("a")).next().unwrap().text();
        package
            .version
            .retain(|c| c.is_numeric() || c.is_ascii_punctuation());

        package.build = Build::Stable;

        package.url = format!(
            "https://ftp.nluug.nl/pub/graphics/blender/release/{}",
            node.find(Name("a"))
                .next()
                .unwrap()
                .attr("href")
                .unwrap()
                .strip_prefix(&url)
                .unwrap()
                .strip_suffix("/")
                .unwrap()
                .replace(".msi", ".zip")
        );

        package.name = get_file_stem(&package.url).to_string();

        let mut date = node
            .find(Class("dl-header-info-platform"))
            .next()
            .unwrap()
            .find(Name("small"))
            .next()
            .unwrap()
            .text();
        let mut date = date.split_off(date.find("on").unwrap() + 3);
        date.push_str("-00:00:00");
        package.date = NaiveDateTime::parse_from_str(&date, "%B %d, %Y-%T").unwrap();

        package.os = {
            if o == "linux" {
                Os::Linux
            } else if o == "windows" {
                Os::Windows
            } else if o == "macos" {
                Os::MacOs
            } else {
                unreachable!("Unexpected OS");
            }
        };

        stable.push(package);

        stable
    }

    fn get_name(&self) -> String {
        String::from("stable")
    }

    fn get_db_path(&self) -> PathBuf {
        CONFIG_PATH.parent().unwrap().join("stable_db.bin")
    }
}
