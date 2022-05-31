use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Os, Package},
    releases::{stable_archive::fetch_stable_archive_version, ReleaseType},
    settings::get_setting,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use select::predicate::{Attr, Class, Name, Predicate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use versions::Versioning;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct StableLatest(Vec<Package>);

#[async_trait]
impl ReleaseType for StableLatest {
    async fn fetch() -> Self {
        let (mut package, version_path) = {
            let url = "https://www.blender.org/download/";
            let document = get_document(url).await;

            let (os, targ_os) = {
                if cfg!(target_os = "linux") {
                    (Os::Linux, "linux")
                } else if cfg!(target_os = "windows") {
                    (Os::Windows, "windows")
                } else if cfg!(target_os = "macos") {
                    (Os::MacOs, "macos")
                } else {
                    unreachable!("Unexpected OS");
                }
            };

            let node = document.find(Attr("id", targ_os)).next().unwrap();

            let (version_path, _) = node
                .find(Name("a"))
                .next()
                .unwrap()
                .attr("href")
                .unwrap()
                .strip_prefix(&url)
                .unwrap()
                .strip_prefix("release/")
                .unwrap()
                .split_once('/')
                .unwrap();

            let version = {
                let mut version = node.find(Name("a")).next().unwrap().text();
                version.retain(|c| c.is_numeric() || c.is_ascii_punctuation());
                Versioning::new(&version).unwrap()
            };

            let url = format!(
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

            let date = {
                let mut date = node
                    .find(Class("dl-build-details-popup"))
                    .next()
                    .unwrap()
                    .find(Name("small"))
                    .nth(0)
                    .unwrap()
                    .text();
                let mut date = date.split_off(date.find("on").unwrap() + 3);
                date.truncate(date.find(" ·").unwrap());
                date.push_str("-00:00:00");
                NaiveDateTime::parse_from_str(&date, "%B %d, %Y-%T").unwrap()
            };

            let package = Package {
                version,
                name: get_file_stem(&url).to_string(),
                build: Build::StableLatest,
                date,
                url,
                os,
                ..Default::default()
            };

            (package, version_path.to_string())
        };

        let stable_archive_packages =
            fetch_stable_archive_version(format!("{}/", version_path)).await;
        if let Some(a_package) = stable_archive_packages
            .iter()
            .find(|a_package| a_package.url == package.url)
        {
            package.date = a_package.date;
        }

        Self(vec![package])
    }

    fn get_db_path(&self) -> PathBuf {
        get_setting().databases_dir.join("stable_latest.bin")
    }
}
