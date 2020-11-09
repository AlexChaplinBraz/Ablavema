//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::settings::*;
use bzip2::read::BzDecoder;
use chrono::{Date, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest;
use reqwest::{header, Client};
use select::document::Document;
use select::predicate::{Attr, Class, Name};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{error::Error, fs::File};
use tar::Archive;
use tokio::{fs, fs::create_dir_all, fs::remove_dir_all, fs::remove_file, io::AsyncWriteExt};
use xz2::read::XzDecoder;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Releases {
    pub official_releases: Vec<Package>,
    pub lts_releases: Vec<Package>,
    pub experimental_branches: Vec<Package>,
    pub latest_daily: Vec<Package>,
    pub latest_stable: Vec<Package>,
}

impl Releases {
    pub fn new() -> Releases {
        Releases {
            official_releases: Vec::new(),
            lts_releases: Vec::new(),
            experimental_branches: Vec::new(),
            latest_daily: Vec::new(),
            latest_stable: Vec::new(),
        }
    }

    pub fn load(&mut self, settings: &Settings) {
        if settings.releases_db.exists() {
            let file = File::open(&settings.releases_db).unwrap();
            let bin: Releases = bincode::deserialize_from(file).unwrap();
            *self = bin;
        }
    }

    pub fn save(&mut self, settings: &Settings) {
        let file = File::create(&settings.releases_db).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub async fn fetch_official_releases(&mut self, settings: &Settings) {
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

                for node in document.find(Name("a")) {
                    builds.push(node.attr("href").unwrap());
                }

                builds.retain(|x| {
                    !x.ends_with('/')
                        && !x.contains(".msi")
                        && !x.contains(".md")
                        && !x.contains(".sha256")
                        && !x.contains(".msix")
                        && !x.contains(".exe")
                        && !x.contains(".txt")
                        && !x.contains(".rpm")
                        && !x.contains(".deb")
                        && !x.contains(".tbz")
                        && !x.contains("md5sums")
                        && !x.contains("source")
                        && !x.contains("demo")
                        && !x.contains("script")
                        && !x.contains("manual")
                        && !x.contains("files")
                        && !x.contains("beos")
                        && !x.contains("static")
                        && !x.contains("irix")
                        && !x.contains("solaris")
                        && !x.contains("powerpc")
                        && !x.contains("-ppc")
                        && !x.contains("_ppc")
                        && !x.contains("freebsd")
                        && !x.contains("FreeBSD")
                        && !x.contains("?")
                    //&& !x.contains("i386")
                    //&& !x.contains("i686")
                    //&& !x.contains("-win32")
                    //&& !x.contains("-windows32")
                });
                builds.reverse();

                for name in builds {
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

                    package.name = get_file_stem(name).to_string();

                    package.build = Build::Official;

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

                    // TODO: Find some way to get the date from that horrible plain site.
                    //package.date = ?

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
            fetched.official_releases.append(&mut handle.await.unwrap());
        }

        fetched.official_releases.sort_by_key(|x| x.version.clone());
        fetched.official_releases.reverse();

        if self.official_releases != fetched.official_releases {
            self.official_releases = fetched.official_releases;

            self.save(&settings);
        }
    }

    pub async fn fetch_lts_releases(&mut self, settings: &Settings) {
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

            fetched.lts_releases.push(package);
        }

        fetched.lts_releases.reverse();

        if self.lts_releases != fetched.lts_releases {
            self.lts_releases = fetched.lts_releases;

            self.save(&settings);
        }
    }

    pub async fn fetch_latest_stable(&mut self, settings: &Settings) {
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

        fetched.latest_stable.push(package);

        if self.latest_stable != fetched.latest_stable {
            self.latest_stable = fetched.latest_stable;

            self.save(&settings);
        }
    }

    pub async fn fetch_latest_daily(&mut self, settings: &Settings) {
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

            fetched.latest_daily.push(package);
        }

        if self.latest_daily != fetched.latest_daily {
            self.latest_daily = fetched.latest_daily;

            self.save(&settings);
        }
    }

    pub async fn fetch_experimental_branches(&mut self, settings: &Settings) {
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

            package.build = Build::Experimental(
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

            fetched.experimental_branches.push(package);
        }

        if self.experimental_branches != fetched.experimental_branches {
            self.experimental_branches = fetched.experimental_branches;

            self.save(&settings);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Clone)]
pub struct Package {
    pub version: String,
    pub name: String,
    pub build: Build,
    pub date: NaiveDateTime,
    pub commit: String,
    pub url: String,
    pub os: Os,
    pub changelog: Vec<Change>,
}

impl Package {
    fn new() -> Package {
        Package {
            version: String::new(),
            name: String::new(),
            build: Build::None,
            date: NaiveDateTime::new(
                NaiveDate::from_ymd(1999, 12, 31),
                NaiveTime::from_hms(23, 59, 59),
            ),
            commit: String::new(),
            url: String::new(),
            os: Os::None,
            changelog: Vec::new(),
        }
    }

    pub async fn install(
        &self,
        settings: &Settings,
        multi_progress: &MultiProgress,
    ) -> Result<(), Box<dyn Error>> {
        let download_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.cyan/red}] {bytes}/{total_bytes} {bytes_per_sec} ({eta}) => {wide_msg}")
            .progress_chars("#>-");
        let extraction_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.green/red}] {percent}% => {wide_msg}")
            .progress_chars("#>-");

        let client = Client::new();

        let total_size = {
            let resp = client.head(&self.url).send().await.unwrap();
            if resp.status().is_success() {
                resp.headers()
                    .get(header::CONTENT_LENGTH)
                    .and_then(|ct_len| ct_len.to_str().ok())
                    .and_then(|ct_len| ct_len.parse().ok())
                    .unwrap_or(0)
            } else {
                let error = format!(
                    "Couldn't download URL: {}. Error: {:?}",
                    self.url,
                    resp.status(),
                );
                panic!(error);
            }
        };

        let progress_bar = multi_progress.add(ProgressBar::new(total_size));
        progress_bar.set_style(download_style.clone());

        let msg = format!(
            "Downloading {}",
            self.url.split_terminator('/').last().unwrap()
        );
        progress_bar.set_message(&msg);

        let url = self.url.clone();
        let request = client.get(&url);

        create_dir_all(&settings.temp_dir).await.unwrap();

        let f = format!(
            "{}/{}",
            settings.temp_dir.to_str().unwrap(),
            self.url.split_terminator('/').last().unwrap()
        );
        let file = Path::new(&f);

        // TODO: Prompt/option for re-download.
        if file.exists() {
            remove_file(file).await?;
        }

        let mut source = request.send().await.unwrap();
        let mut dest = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .await
            .unwrap();

        let msg = format!(
            "Downloaded {}",
            self.url.split_terminator('/').last().unwrap()
        );

        let download_handle = tokio::task::spawn(async move {
            while let Some(chunk) = source.chunk().await.unwrap() {
                dest.write_all(&chunk).await.unwrap();
                progress_bar.inc(chunk.len() as u64);
            }

            progress_bar.finish_with_message(&msg);
        });

        create_dir_all(&settings.packages_dir).await?;

        // TODO: Prompt/option for re-extraction.
        let package = format!("{}/{}", settings.packages_dir.to_str().unwrap(), self.name);
        let path = Path::new(&package);
        if path.exists() {
            remove_dir_all(path).await?;
        }

        let file = format!(
            "{}/{}",
            settings.temp_dir.to_str().unwrap(),
            self.url.split_terminator('/').last().unwrap()
        );

        let packages_dir = settings.packages_dir.clone();

        // This value is hardcoded because the cost of calculating it is way too high
        // to justify it (around 4 seconds). Setting it a bit higher so that it's not stuck
        // at 100%. It'll jump to 100% from around 95% for recent packages,
        // but it's jumpy throughout the whole process so it doesn't stand out.
        // It is, however, noticeable with older packages with smaller size.
        // TODO: Implement a fast way of calculating it.
        let progress_bar = multi_progress.add(ProgressBar::new(5000));
        progress_bar.set_style(extraction_style.clone());

        let extraction_handle = tokio::task::spawn(async move {
            download_handle.await.unwrap();

            let msg = format!("Extracting {}", file.split_terminator('/').last().unwrap());
            progress_bar.set_message(&msg);
            progress_bar.reset_elapsed();
            progress_bar.enable_steady_tick(250);

            if cfg!(target_os = "linux") {
                if file.ends_with(".xz") {
                    let tar_xz = File::open(&file).unwrap();
                    let tar = XzDecoder::new(tar_xz);
                    let mut archive = Archive::new(tar);

                    for entry in archive.entries().unwrap() {
                        progress_bar.inc(1);
                        let mut file = entry.unwrap();
                        file.unpack_in(&packages_dir).unwrap();
                    }

                    let msg = format!("Extracted {}", file.split_terminator('/').last().unwrap());
                    progress_bar.finish_with_message(&msg);
                } else if file.ends_with(".bz2") {
                    let tar_bz2 = File::open(&file).unwrap();
                    let tar = BzDecoder::new(tar_bz2);
                    let mut archive = Archive::new(tar);

                    for entry in archive.entries().unwrap() {
                        progress_bar.inc(1);
                        let mut file = entry.unwrap();
                        file.unpack_in(&packages_dir).unwrap();
                    }

                    let msg = format!("Extracted {}", file.split_terminator('/').last().unwrap());
                    progress_bar.finish_with_message(&msg);
                } else if file.ends_with(".gz") {
                    let tar_gz = File::open(&file).unwrap();
                    let tar = GzDecoder::new(tar_gz);
                    let mut archive = Archive::new(tar);

                    for entry in archive.entries().unwrap() {
                        progress_bar.inc(1);
                        let mut file = entry.unwrap();
                        file.unpack_in(&packages_dir).unwrap();
                    }

                    let msg = format!("Extracted {}", file.split_terminator('/').last().unwrap());
                    progress_bar.finish_with_message(&msg);
                } else {
                    unreachable!("Unknown compression extension");
                }
            } else if cfg!(target_os = "windows") {
                todo!("windows extraction");
            } else if cfg!(target_os = "macos") {
                todo!("macos extraction");
            } else {
                unreachable!("Unsupported OS extraction");
            }
        });

        // TODO: Wrap them in Arc<Mutex<>> to avoid unnecessary cloning.
        let package = (*self).clone();
        let packages_dir = settings.packages_dir.clone();

        let _ = tokio::task::spawn(async move {
            extraction_handle.await.unwrap();

            let mut path = packages_dir.join(&package.name);
            path.push("package_info.bin");
            let file = File::create(&path).unwrap();
            bincode::serialize_into(file, &package).unwrap();
        });

        Ok(())
    }

    pub async fn remove(&self, settings: &Settings) -> Result<(), Box<dyn Error>> {
        let path = settings.packages_dir.join(&self.name);
        remove_dir_all(path).await?;

        // TODO: Add this type of reporting to other commands like fetch.
        println!("Removed {}", self.name);

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Clone)]
pub enum Build {
    Official,
    Stable,
    LTS,
    Daily(String),
    Experimental(String),
    None,
}

impl std::fmt::Display for Build {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let printable = match self {
            Build::Official => "Official Release",
            Build::Stable => "Stable Release",
            Build::LTS => "LTS Release",
            Build::Daily(s) | Build::Experimental(s) => s,
            Build::None => unreachable!("Unexpected release type"),
        };
        write!(f, "{}", printable)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Clone)]
pub struct Change {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Clone)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
    None,
}

fn get_file_stem(filename: &str) -> &str {
    if filename.contains(".tar.") {
        let f = Path::new(filename).file_stem().unwrap().to_str().unwrap();
        Path::new(f).file_stem().unwrap().to_str().unwrap()
    } else {
        Path::new(filename).file_stem().unwrap().to_str().unwrap()
    }
}
