//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{helpers::*, package::*, settings::*};
use chrono::{Datelike, NaiveDateTime, Utc};
use regex::Regex;
use reqwest;
use select::{
    document::Document,
    predicate::{Attr, Class, Name},
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Releases {
    pub daily: Vec<Package>,
    pub branched: Vec<Package>,
    pub lts: Vec<Package>,
    pub archived: Vec<Package>,
    pub stable: Vec<Package>,
}

impl Releases {
    pub fn new() -> Releases {
        Releases {
            daily: Vec::new(),
            branched: Vec::new(),
            lts: Vec::new(),
            archived: Vec::new(),
            stable: Vec::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn Error>> {
        if SETTINGS.read().unwrap().releases_db.exists() {
            let file = File::open(&SETTINGS.read().unwrap().releases_db)?;
            let bin: Releases = bincode::deserialize_from(file)?;
            *self = bin;
        }

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let file = File::create(&SETTINGS.read().unwrap().releases_db)?;
        bincode::serialize_into(file, self)?;

        Ok(())
    }

    pub async fn fetch_daily(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "https://builder.blender.org/download/";
        let resp = reqwest::get(url).await.unwrap();
        assert!(resp.status().is_success());
        let resp = resp.bytes().await.unwrap();
        let document = Document::from_read(&resp[..]).unwrap();

        let current_year = Utc::today().year();
        let current_year = format!("-{}", current_year);

        let mut fetched = Releases::new();

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

            let mut package = Package::new();

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
            date.push_str(&current_year);
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

            fetched.daily.push(package);
        }

        if self.daily != fetched.daily {
            self.daily = fetched.daily;

            self.save()?;
        }

        Ok(())
    }

    pub async fn fetch_branched(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "https://builder.blender.org/download/branches/";
        let resp = reqwest::get(url).await.unwrap();
        assert!(resp.status().is_success());
        let resp = resp.bytes().await.unwrap();
        let document = Document::from_read(&resp[..]).unwrap();

        let current_year = Utc::today().year();
        let current_year = format!("-{}", current_year);

        let mut fetched = Releases::new();

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

            let mut package = Package::new();

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

            let mut date = build.find(Name("small")).next().unwrap().text();
            let mut date: String = date.drain(..date.find('-').unwrap()).collect();
            date.push_str(&current_year);
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

            fetched.branched.push(package);
        }

        if self.branched != fetched.branched {
            self.branched = fetched.branched;

            self.save()?;
        }

        Ok(())
    }

    pub async fn fetch_lts(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "https://www.blender.org/download/lts/";
        let resp = reqwest::get(url).await.unwrap();
        assert!(resp.status().is_success());
        let resp = resp.bytes().await.unwrap();
        let document = Document::from_read(&resp[..]).unwrap();

        let mut fetched = Releases::new();

        // Can be done so it works off a vector of LTS releases, but by that time the website will
        // probably change anyway so I'll wait until then. Maybe by then it won't require me to do
        // it so stupidly since the layout is hard to parse.
        let lts = String::from("283");
        for rev in 0.. {
            let mut package = Package::new();

            let lts_id = format!("lts-release-{}{}", lts, rev);
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

            let lts_date_id = format!("faq-lts-release-{}{}-1", lts, rev);
            let section = document
                .find(Attr("id", lts_date_id.as_str()))
                .next()
                .unwrap();

            let mut date = section
                .find(Name("p"))
                .next()
                .unwrap()
                .text()
                .strip_prefix("Released on ")
                .unwrap()
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

                package.name = get_file_stem(node.text().as_str()).to_string();

                package.build = Build::LTS;

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

            let lts_changelog_id = format!("faq-lts-release-{}{}-2", lts, rev);
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

            fetched.lts.push(package);
        }

        fetched.lts.reverse();

        if self.lts != fetched.lts {
            self.lts = fetched.lts;

            self.save()?;
        }

        Ok(())
    }

    pub async fn fetch_archived(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "https://ftp.nluug.nl/pub/graphics/blender/release/";
        let resp = reqwest::get(url).await.unwrap();
        assert!(resp.status().is_success());
        let resp = resp.bytes().await.unwrap();
        let document = Document::from_read(&resp[..]).unwrap();

        let mut fetched = Releases::new();

        let mut versions = Vec::new();

        for node in document.find(Name("a")) {
            let url_path = node.attr("href").unwrap();
            versions.push(url_path.to_string());
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

                let resp = reqwest::get(url.as_str()).await.unwrap();
                assert!(resp.status().is_success());
                let resp = resp.bytes().await.unwrap();
                let document = Document::from_read(&resp[..]).unwrap();
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

                    let mut package = Package::new();

                    package.name = format!("{}-archived", get_file_stem(name));

                    package.build = Build::Archived;

                    package.version = match version.as_ref() {
                        "1.0" => String::from("1.0"),
                        "1.60" => String::from("1.60"),
                        "1.73" => String::from("1.73"),
                        "1.80" => {
                            let v = {
                                if package.name.contains("alpha") {
                                    "alpha"
                                } else {
                                    "a"
                                }
                            };
                            format!("1.80{}", v)
                        }
                        "2.04" => {
                            let v = {
                                if package.name.contains("alpha") {
                                    "alpha"
                                } else {
                                    ""
                                }
                            };
                            format!("2.04{}", v)
                        }
                        "2.39" => {
                            let v = {
                                if package.name.contains("alpha1") {
                                    "alpha1"
                                } else {
                                    "alpha2"
                                }
                            };
                            format!("2.40{}", v)
                        }
                        "2.50alpha" => {
                            let v = {
                                if package.name.contains("alpha0") {
                                    "alpha0"
                                } else if package.name.contains("alpha1") {
                                    "alpha1"
                                } else {
                                    "alpha2"
                                }
                            };
                            format!("2.50{}", v)
                        }
                        "2.53beta" => String::from("2.53beta"),
                        "2.54beta" => String::from("2.54beta"),
                        "2.55beta" => String::from("2.55beta"),
                        "2.56beta" => String::from("2.56beta"),
                        "2.56abeta" => String::from("2.56abeta"),
                        "2.79latest" => String::from("2.79latest"),
                        _ => package
                            .name
                            .split_terminator("-")
                            .skip(1)
                            .next()
                            .unwrap()
                            .to_string(),
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
            fetched.archived.append(&mut handle.await.unwrap());
        }

        fetched.archived.sort_by_key(|x| x.version.clone());
        fetched.archived.reverse();

        if self.archived != fetched.archived {
            self.archived = fetched.archived;

            self.save()?;
        }

        Ok(())
    }

    pub async fn fetch_stable(&mut self) -> Result<(), Box<dyn Error>> {
        let url = "https://www.blender.org/download/";
        let resp = reqwest::get(url).await.unwrap();
        assert!(resp.status().is_success());
        let resp = resp.bytes().await.unwrap();
        let document = Document::from_read(&resp[..]).unwrap();

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
        let mut package = Package::new();

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

        let mut fetched = Releases::new();

        fetched.stable.push(package);

        if self.stable != fetched.stable {
            self.stable = fetched.stable;

            self.save()?;
        }

        Ok(())
    }
}
