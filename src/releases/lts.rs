use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Change, Os, Package},
    releases::ReleaseType,
    settings::get_setting,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use select::predicate::{Attr, Name};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use versions::Versioning;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Lts(Vec<Package>);

#[async_trait]
impl ReleaseType for Lts {
    async fn fetch() -> Self {
        let mut lts = Lts::default();

        let download_path = "https://ftp.nluug.nl/pub/graphics/blender/release/";

        let lts_info = &[
            (
                "https://www.blender.org/download/lts/2-83/",
                "283",
                "Blender2.83",
            ),
            (
                "https://www.blender.org/download/lts/2-93/",
                "293",
                "Blender2.93",
            ),
        ];

        let (os, targ_os) = {
            if cfg!(target_os = "linux") {
                (Os::Linux, "linux")
            } else if cfg!(target_os = "windows") {
                (Os::Windows, "windows")
            } else if cfg!(target_os = "macos") {
                (Os::MacOs, "mac")
            } else {
                unreachable!("Unexpected OS");
            }
        };

        for (lts_url, lts_ver, lts_ver_path) in lts_info {
            let document = get_document(lts_url).await;

            for rev in 0.. {
                let lts_id = format!("lts-release-{}{}", lts_ver, rev);
                let version = match document.find(Attr("id", lts_id.as_str())).next() {
                    Some(a) => a,
                    None => break,
                }
                .text();

                let version =
                    Versioning::new(version.split_whitespace().skip(2).next().unwrap()).unwrap();

                let lts_date_id = format!("faq-lts-release-{}{}-1", lts_ver, rev);
                let section_1 = document
                    .find(Attr("id", lts_date_id.as_str()))
                    .next()
                    .unwrap();

                let mut date = match section_1
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
                let date = NaiveDateTime::parse_from_str(&date, "%B %d, %Y-%T").unwrap();

                let lts_changelog_id = format!("faq-lts-release-{}{}-2", lts_ver, rev);
                let section_2 = document
                    .find(Attr("id", lts_changelog_id.as_str()))
                    .next()
                    .unwrap();

                let mut changelog = Vec::new();
                for node in section_2.find(Name("li")) {
                    let text = node.text();

                    let url = match node.find(Name("a")).next() {
                        Some(a) => a.attr("href").unwrap_or_default().to_string(),
                        None => String::new(),
                    };

                    changelog.push(Change { text, url });
                }

                for node in section_1.find(Name("a")) {
                    let archive_name = node
                        .attr("href")
                        .unwrap()
                        .trim_end_matches("?x69806")
                        .split_terminator('/')
                        .last()
                        .unwrap();

                    if archive_name.ends_with(".msi") || !archive_name.contains(targ_os) {
                        continue;
                    }

                    let mut package = Package::default();

                    package.name = format!("{}-lts", get_file_stem(archive_name));

                    package.build = Build::Lts;

                    package.url = format!("{}{}/{}", download_path, lts_ver_path, archive_name);

                    package.os = os;

                    package.version = version.clone();

                    package.date = date.clone();

                    package.changelog = changelog.clone();

                    lts.push(package);
                }
            }
        }

        lts.sort();
        lts
    }

    fn get_name(&self) -> String {
        String::from("LTS")
    }

    fn get_db_path(&self) -> PathBuf {
        get_setting().databases_dir.join("lts.bin")
    }
}
