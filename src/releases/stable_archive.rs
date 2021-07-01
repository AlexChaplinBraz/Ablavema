use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Os, Package},
    releases::ReleaseType,
    settings::{get_setting, ARCHIVE_DATE_RE},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use select::predicate::Name;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::spawn;
use versions::Versioning;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct StableArchive(Vec<Package>);

#[async_trait]
impl ReleaseType for StableArchive {
    async fn fetch() -> Self {
        let mut stable_archive = Self::default();

        let versions = {
            let mut versions = Vec::new();
            let document = get_document("https://ftp.nluug.nl/pub/graphics/blender/release/").await;

            for node in document.find(Name("a")) {
                let url_path = node.attr("href").unwrap();
                versions.push(url_path.to_string());
            }

            versions
                .retain(|x| x.contains("Blender") && x.ends_with('/') && !x.contains("Benchmark"));
            versions.push("Blender2.79/latest/".to_string());
            versions
        };

        let mut handles = Vec::new();
        for version in versions {
            handles.push(spawn(
                async move { fetch_stable_archive_version(version).await },
            ));
        }

        for handle in handles {
            stable_archive.append(&mut handle.await.unwrap());
        }

        stable_archive.sort();
        stable_archive
    }

    fn get_db_path(&self) -> PathBuf {
        get_setting().databases_dir.join("stable_archive.bin")
    }
}

pub async fn fetch_stable_archive_version(version: String) -> Vec<Package> {
    let mut packages = Vec::new();

    let url = format!(
        "{}{}",
        "https://ftp.nluug.nl/pub/graphics/blender/release/", version
    );

    let document = get_document(url.as_str()).await;

    let version = version.strip_prefix("Blender").unwrap().replace("/", "");

    let (os, targ_os) = {
        if cfg!(target_os = "linux") {
            (Os::Linux, "linux")
        } else if cfg!(target_os = "windows") {
            (Os::Windows, "win")
        } else if cfg!(target_os = "macos") {
            (Os::MacOs, "OS")
        } else {
            unreachable!("Unexpected OS");
        }
    };

    let mut dates = {
        let mut dates = Vec::new();
        for node in document.find(Name("pre")).next().unwrap().children() {
            if let Some(text) = node.as_text() {
                if text.chars().filter(|&c| c == '-').count() > 2 {
                    continue;
                }

                if let Some(date) = ARCHIVE_DATE_RE.find(text) {
                    dates.push(format!("{}:00", date.as_str()));
                }
            }
        }
        dates
    };

    let builds = {
        let mut builds = Vec::new();
        for node in document.find(Name("a")) {
            builds.push(node.attr("href").unwrap());
        }
        builds.retain(|x| !x.ends_with('/') && !x.contains('?'));
        builds.reverse();
        builds
    };

    for build in builds {
        let date = dates.pop().unwrap();

        if !build.contains(targ_os)
            || build.contains(".msi")
            || build.contains(".md")
            || build.contains(".sha256")
            || build.contains(".msix")
            || build.contains(".exe")
            || build.contains(".txt")
            || build.contains(".rpm")
            || build.contains(".deb")
            || build.contains(".tbz")
            || build.contains(".7z")
            || build.contains("md5sums")
            || build.contains("source")
            || build.contains("demo")
            || build.contains("script")
            || build.contains("manual")
            || build.contains("files")
            || build.contains("beos")
            || build.contains("static")
            || build.contains("irix")
            || build.contains("solaris")
            || build.contains("powerpc")
            || build.contains("-ppc")
            || build.contains("_ppc")
            || build.contains("freebsd")
            || build.contains("FreeBSD")
        // TODO: Consider disabling old architectures.
        //|| name.contains("i386")
        //|| name.contains("i686")
        //|| name.contains("-win32")
        //|| name.contains("-windows32")
        {
            continue;
        }

        let version = match version.as_ref() {
            "1.0" => Versioning::new("1.0").unwrap(),
            "1.60" => Versioning::new("1.60").unwrap(),
            "1.73" => Versioning::new("1.73").unwrap(),
            "1.80" => {
                if build.contains("alpha") {
                    Versioning::new("1.80alpha").unwrap()
                } else {
                    Versioning::new("1.80a").unwrap()
                }
            }
            "2.04" => {
                if build.contains("alpha") {
                    Versioning::new("2.04alpha").unwrap()
                } else {
                    Versioning::new("2.04").unwrap()
                }
            }
            "2.39" => {
                if build.contains("alpha") {
                    Versioning::new("2.40alpha1").unwrap()
                } else {
                    Versioning::new("2.40alpha2").unwrap()
                }
            }
            "2.50alpha" => {
                if build.contains("alpha0") {
                    Versioning::new("2.50alpha0").unwrap()
                } else if build.contains("alpha1") {
                    Versioning::new("2.50alpha1").unwrap()
                } else {
                    Versioning::new("2.50alpha2").unwrap()
                }
            }
            "2.53beta" => Versioning::new("2.53beta").unwrap(),
            "2.54beta" => Versioning::new("2.54beta").unwrap(),
            "2.55beta" => Versioning::new("2.55beta").unwrap(),
            "2.56beta" => Versioning::new("2.56beta").unwrap(),
            "2.56abeta" => Versioning::new("2.56abeta").unwrap(),
            "2.79latest" => Versioning::new("2.79latest").unwrap(),
            _ => Versioning::new(build.split_terminator('-').nth(1).unwrap()).unwrap(),
        };

        let package = Package {
            version,
            name: get_file_stem(build).to_string(),
            build: Build::StableArchive,
            date: NaiveDateTime::parse_from_str(&date, "%d-%b-%Y %T").unwrap(),
            url: format!("{}{}", url, build),
            os,
            ..Default::default()
        };

        packages.push(package);
    }

    packages
}
