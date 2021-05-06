//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    helpers::{get_document, get_file_stem},
    package::{Build, Os, Package},
    releases::ReleaseType,
    settings::SETTINGS,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_deref::{Deref, DerefMut};
use regex::Regex;
use select::predicate::Name;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use versions::Versioning;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Archived(Vec<Package>);

#[async_trait]
impl ReleaseType for Archived {
    async fn fetch() -> Self {
        let mut archived = Archived::default();
        let mut versions = Vec::new();

        {
            let document = get_document("https://ftp.nluug.nl/pub/graphics/blender/release/").await;

            for node in document.find(Name("a")) {
                let url_path = node.attr("href").unwrap();
                versions.push(url_path.to_string());
            }
        }

        versions.retain(|x| x.contains("Blender") && x.ends_with('/') && !x.contains("Benchmark"));
        versions.push("Blender2.79/latest/".to_string());

        let mut handles = Vec::new();
        for ver in versions {
            let handle = tokio::task::spawn(async move {
                let mut packages = Vec::new();

                let version = ver.strip_prefix("Blender").unwrap().replace("/", "");

                let url = format!(
                    "{}{}",
                    "https://ftp.nluug.nl/pub/graphics/blender/release/", ver
                );

                let document = get_document(url.as_str()).await;
                let mut builds = Vec::new();

                let re = Regex::new(r"\d{2}-\w{3}-\d{4}\s\d{2}:\d{2}").unwrap();
                let mut dates = Vec::new();
                for node in document.find(Name("pre")).next().unwrap().children() {
                    if let Some(text) = node.as_text() {
                        if text.chars().filter(|&c| c == '-').count() > 2 {
                            continue;
                        }

                        if let Some(date) = re.find(text) {
                            dates.push(format!("{}:00", date.as_str()));
                        }
                    }
                }

                for node in document.find(Name("a")) {
                    builds.push(node.attr("href").unwrap());
                }

                builds.retain(|x| !x.ends_with('/') && !x.contains("?"));
                builds.reverse();

                for name in builds {
                    let date = dates.pop().unwrap();

                    if name.contains(".msi")
                        || name.contains(".md")
                        || name.contains(".sha256")
                        || name.contains(".msix")
                        || name.contains(".exe")
                        || name.contains(".txt")
                        || name.contains(".rpm")
                        || name.contains(".deb")
                        || name.contains(".tbz")
                        || name.contains(".7z")
                        || name.contains("md5sums")
                        || name.contains("source")
                        || name.contains("demo")
                        || name.contains("script")
                        || name.contains("manual")
                        || name.contains("files")
                        || name.contains("beos")
                        || name.contains("static")
                        || name.contains("irix")
                        || name.contains("solaris")
                        || name.contains("powerpc")
                        || name.contains("-ppc")
                        || name.contains("_ppc")
                        || name.contains("freebsd")
                        || name.contains("FreeBSD")
                    //|| name.contains("i386")
                    //|| name.contains("i686")
                    //|| name.contains("-win32")
                    //|| name.contains("-windows32")
                    {
                        continue;
                    }

                    let targ_os = if cfg!(target_os = "linux") {
                        "linux"
                    } else if cfg!(target_os = "windows") {
                        "win"
                    } else if cfg!(target_os = "macos") {
                        "OS"
                    } else {
                        unreachable!("Unsupported OS config");
                    };

                    if !name.contains(targ_os) {
                        continue;
                    }

                    let mut package = Package::default();

                    package.name = format!("{}-archived", get_file_stem(name));

                    package.build = Build::Archived;

                    package.version = match version.as_ref() {
                        "1.0" => Versioning::new("1.0").unwrap(),
                        "1.60" => Versioning::new("1.60").unwrap(),
                        "1.73" => Versioning::new("1.73").unwrap(),
                        "1.80" => {
                            if package.name.contains("alpha") {
                                Versioning::new("1.80alpha").unwrap()
                            } else {
                                Versioning::new("1.80a").unwrap()
                            }
                        }
                        "2.04" => {
                            if package.name.contains("alpha") {
                                Versioning::new("2.04alpha").unwrap()
                            } else {
                                Versioning::new("2.04").unwrap()
                            }
                        }
                        "2.39" => {
                            if package.name.contains("alpha") {
                                Versioning::new("2.40alpha1").unwrap()
                            } else {
                                Versioning::new("2.40alpha2").unwrap()
                            }
                        }
                        "2.50alpha" => {
                            if package.name.contains("alpha0") {
                                Versioning::new("2.50alpha0").unwrap()
                            } else if package.name.contains("alpha1") {
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
                        _ => Versioning::new(
                            package.name.split_terminator("-").skip(1).next().unwrap(),
                        )
                        .unwrap(),
                    };

                    package.date = NaiveDateTime::parse_from_str(&date, "%d-%b-%Y %T").unwrap();

                    package.url = format!("{}{}", url, name);

                    package.os = {
                        if name.contains("linux") {
                            Os::Linux
                        } else if name.contains("win") {
                            Os::Windows
                        } else if name.contains("OS") {
                            Os::MacOs
                        } else {
                            unreachable!("Unexpected OS");
                        }
                    };

                    packages.push(package);
                }

                packages
            });

            handles.push(handle);
        }

        for handle in handles {
            archived.append(&mut handle.await.unwrap());
        }

        archived.sort();
        archived
    }

    fn get_name(&self) -> String {
        String::from("archived")
    }

    fn get_db_path(&self) -> PathBuf {
        SETTINGS.read().unwrap().databases_dir.join("archived.bin")
    }
}
