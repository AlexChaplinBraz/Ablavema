//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Change, Os, Package},
    releases::ReleaseType,
    settings::CONFIG_PATH,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use select::predicate::{Attr, Name};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Lts(Vec<Package>);

#[async_trait]
impl ReleaseType for Lts {
    async fn fetch() -> Self {
        let document = get_document("https://www.blender.org/download/lts/").await;
        let mut lts = Lts::default();

        // Can be done so it works off a vector of LTS releases, but by that time the website will
        // probably change anyway so I'll wait until then. Maybe by then it won't require me to do
        // it so stupidly since the layout is hard to parse.
        let lts_ver = String::from("283");
        for rev in 0.. {
            let mut package = Package::default();

            let lts_id = format!("lts-release-{}{}", lts_ver, rev);
            let version = match document.find(Attr("id", lts_id.as_str())).next() {
                Some(a) => a,
                _ => break,
            }
            .text();

            package.version = version
                .split_whitespace()
                .skip(2)
                .next()
                .unwrap()
                .to_string();

            let lts_date_id = format!("faq-lts-release-{}{}-1", lts_ver, rev);
            let section = document
                .find(Attr("id", lts_date_id.as_str()))
                .next()
                .unwrap();

            let mut date = match section
                .find(Name("p"))
                .next()
                .unwrap()
                .text()
                .strip_prefix("Released on ")
            {
                Some(a) => a,
                None => continue,
            }
            .strip_suffix(".")
            .unwrap()
            .to_string();
            date.push_str("-00:00:00");
            package.date = NaiveDateTime::parse_from_str(&date, "%B %d, %Y-%T").unwrap();

            for node in section.find(Name("a")) {
                let name = node.text();
                if name.is_empty() || name.contains(".msi") {
                    continue;
                }

                let targ_os = if cfg!(target_os = "linux") {
                    "linux"
                } else if cfg!(target_os = "windows") {
                    "win"
                } else if cfg!(target_os = "macos") {
                    "mac"
                } else {
                    unreachable!("Unsupported OS config");
                };

                if !name.contains(targ_os) {
                    continue;
                }

                package.name = format!("{}-lts", get_file_stem(node.text().as_str()).to_string());

                package.build = Build::Lts;

                let download_path =
                    "https://ftp.nluug.nl/pub/graphics/blender/release/Blender2.83/";
                package.url = format!("{}{}", download_path, name);

                package.os = {
                    if name.contains("linux") {
                        Os::Linux
                    } else if name.contains("win") {
                        Os::Windows
                    } else if name.contains("mac") {
                        Os::MacOs
                    } else {
                        unreachable!("Unexpected OS");
                    }
                };
            }

            let lts_changelog_id = format!("faq-lts-release-{}{}-2", lts_ver, rev);
            let section = document
                .find(Attr("id", lts_changelog_id.as_str()))
                .next()
                .unwrap();

            for node in section.find(Name("li")) {
                let text = node.text();

                let url = match node.find(Name("a")).next() {
                    Some(a) => a.attr("href").unwrap_or_default().to_string(),
                    _ => String::from("N/A"),
                };

                let change = Change { text, url };
                package.changelog.push(change);
            }

            lts.push(package);
        }

        lts.sort();
        lts
    }

    fn get_name(&self) -> String {
        String::from("LTS")
    }

    fn get_db_path(&self) -> PathBuf {
        CONFIG_PATH.parent().unwrap().join("lts_db.bin")
    }
}
