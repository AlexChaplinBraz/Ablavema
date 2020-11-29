//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{helpers::*, settings::*};
use bzip2::read::BzDecoder;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::{self, header, Client};
use select::{
    document::Document,
    predicate::{Attr, Class, Name},
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{create_dir_all, File},
    io::{Read, Write},
};
use tar::Archive;
use tokio::{
    fs::{self, remove_dir_all, remove_file},
    io::AsyncWriteExt,
    task::JoinHandle,
};
use xz2::read::XzDecoder;
use zip::{read::ZipFile, ZipArchive};

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Releases {
    pub daily: Vec<Package>,
    pub experimental: Vec<Package>,
    pub lts: Vec<Package>,
    pub official: Vec<Package>,
    pub stable: Vec<Package>,
}

impl Releases {
    pub fn new() -> Releases {
        Releases {
            daily: Vec::new(),
            experimental: Vec::new(),
            lts: Vec::new(),
            official: Vec::new(),
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

    pub async fn fetch_experimental(&mut self) -> Result<(), Box<dyn Error>> {
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

            fetched.experimental.push(package);
        }

        if self.experimental != fetched.experimental {
            self.experimental = fetched.experimental;

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

    pub async fn fetch_official(&mut self) -> Result<(), Box<dyn Error>> {
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

                    package.name = format!("{}-official", get_file_stem(name));

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
            fetched.official.append(&mut handle.await.unwrap());
        }

        fetched.official.sort_by_key(|x| x.version.clone());
        fetched.official.reverse();

        if self.official != fetched.official {
            self.official = fetched.official;

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
        multi_progress: &MultiProgress,
        flags: &(bool, bool),
    ) -> Result<Option<JoinHandle<()>>, Box<dyn Error>> {
        let information_style = ProgressStyle::default_bar()
            .template("{wide_msg}")
            .progress_chars("---");
        let download_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.cyan/red}] {bytes}/{total_bytes} {bytes_per_sec} ({eta}) => {wide_msg}")
            .progress_chars("#>-");
        let extraction_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.green/red}] {percent}% => {wide_msg}")
            .progress_chars("#>-");

        let package = SETTINGS.read().unwrap().packages_dir.join(&self.name);

        if package.exists() && !flags.0 {
            let progress_bar = multi_progress.add(ProgressBar::new(0));
            progress_bar.set_style(information_style.clone());

            let msg = format!("Package '{}' is already installed.", self.name);
            progress_bar.finish_with_message(&msg);

            return Ok(None);
        } else if package.exists() && flags.0 {
            remove_dir_all(&package).await?;
        }

        let download_handle;

        let file = SETTINGS
            .read()
            .unwrap()
            .cache_dir
            .join(self.url.split_terminator('/').last().unwrap());

        if file.exists() && !flags.1 {
            let progress_bar = multi_progress.add(ProgressBar::new(0));
            progress_bar.set_style(information_style.clone());

            let msg = format!(
                "Found downloaded archive '{}'.",
                self.url.split_terminator('/').last().unwrap()
            );
            progress_bar.finish_with_message(&msg);

            download_handle = Option::None;
        } else {
            if file.exists() {
                remove_file(&file).await?;
            }

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

            download_handle = Some(tokio::task::spawn(async move {
                while let Some(chunk) = source.chunk().await.unwrap() {
                    dest.write_all(&chunk).await.unwrap();
                    progress_bar.inc(chunk.len() as u64);
                }

                progress_bar.finish_with_message(&msg);
            }));
        }

        // This value is hardcoded because the cost of calculating it is way too high
        // to justify it (up to 30 seconds). Setting it a bit higher so that it's not stuck
        // at 100%. It'll jump to 100% from around 95% for recent packages,
        // but it's jumpy throughout the whole process so it doesn't stand out.
        // It is, however, noticeable with older packages with less files.
        // TODO: Implement a fast way of calculating it.
        let progress_bar = multi_progress.add(ProgressBar::new(5000));
        progress_bar.set_style(extraction_style.clone());

        let extraction_handle = tokio::task::spawn(async move {
            if let Some(handle) = download_handle {
                handle.await.unwrap();
            }

            let msg = format!("Extracting {}", file.file_name().unwrap().to_str().unwrap());
            progress_bar.set_message(&msg);
            progress_bar.reset_elapsed();
            progress_bar.enable_steady_tick(250);

            if file.extension().unwrap() == "xz" {
                // This counting implementation adds up to 10 seconds to the extraction
                // on the latest .tar.xz archives.
                /*
                let tar_xz_len = File::open(&file).unwrap();
                let tar_len = XzDecoder::new(tar_xz_len);
                let mut archive_len = Archive::new(tar_len);
                let plen = archive_len.entries().unwrap().count();
                progress_bar.set_length(plen as u64);
                */

                let tar_xz = File::open(&file).unwrap();
                let tar = XzDecoder::new(tar_xz);
                let mut archive = Archive::new(tar);

                for entry in archive.entries().unwrap() {
                    progress_bar.inc(1);
                    let mut file = entry.unwrap();
                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                }

                let msg = format!("Extracted {}", file.file_name().unwrap().to_str().unwrap());
                progress_bar.finish_with_message(&msg);
            } else if file.extension().unwrap() == "bz2" {
                // This counting implementation adds up to 30 seconds to the extraction
                // on the latest .tar.bz2 archives.
                /*
                let tar_bz2_len = File::open(&file).unwrap();
                let tar_len = BzDecoder::new(tar_bz2_len);
                let mut archive_len = Archive::new(tar_len);
                let plen = archive_len.entries().unwrap().count();
                progress_bar.set_length(plen as u64);
                */

                let tar_bz2 = File::open(&file).unwrap();
                let tar = BzDecoder::new(tar_bz2);
                let mut archive = Archive::new(tar);

                for entry in archive.entries().unwrap() {
                    progress_bar.inc(1);
                    let mut file = entry.unwrap();
                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                }

                let msg = format!("Extracted {}", file.file_name().unwrap().to_str().unwrap());
                progress_bar.finish_with_message(&msg);
            } else if file.extension().unwrap() == "gz" {
                // Here it's fine since the old archives are really small.
                let tar_gz_len = File::open(&file).unwrap();
                let tar_len = GzDecoder::new(tar_gz_len);
                let mut archive_len = Archive::new(tar_len);
                let plen = archive_len.entries().unwrap().count();
                progress_bar.set_length(plen as u64);

                let tar_gz = File::open(&file).unwrap();
                let tar = GzDecoder::new(tar_gz);
                let mut archive = Archive::new(tar);

                for entry in archive.entries().unwrap() {
                    progress_bar.inc(1);
                    let mut file = entry.unwrap();
                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                }

                let msg = format!("Extracted {}", file.file_name().unwrap().to_str().unwrap());
                progress_bar.finish_with_message(&msg);
            } else if file.extension().unwrap() == "zip" {
                // TODO: Improve extraction speed. The slowness is caused by Windows Defender
                // and adding the program to the exclusions makes it extract around 6 times faster.
                // Tried spawning threads for each file and it sped up by around 3 times,
                // but it makes the progress bar meaningless. Possible clues for improvements here:
                // https://github.com/rust-lang/rustup/pull/1850
                let zip = File::open(&file).unwrap();
                let mut archive = ZipArchive::new(zip).unwrap();

                progress_bar.set_length(archive.len() as u64);

                // This handles some archives that don't have an inner directory.
                let extraction_dir = match file.file_name().unwrap().to_str().unwrap() {
                    "blender-2.49-win64.zip" => SETTINGS
                        .read()
                        .unwrap()
                        .cache_dir
                        .join("blender-2.49-win64"),
                    "blender-2.49a-win64-python26.zip" => SETTINGS
                        .read()
                        .unwrap()
                        .cache_dir
                        .join("blender-2.49a-win64-python26"),
                    "blender-2.49b-win64-python26.zip" => SETTINGS
                        .read()
                        .unwrap()
                        .cache_dir
                        .join("blender-2.49b-win64-python26"),
                    _ => SETTINGS.read().unwrap().cache_dir.clone(),
                };

                for file_index in 0..archive.len() {
                    progress_bar.inc(1);
                    let mut entry: ZipFile = archive.by_index(file_index).unwrap();
                    let name = entry.name().to_owned();

                    if entry.is_dir() {
                        let extracted_dir_path = extraction_dir.join(name);
                        create_dir_all(extracted_dir_path).unwrap();
                    } else if entry.is_file() {
                        let mut buffer: Vec<u8> = Vec::new();
                        let _bytes_read = entry.read_to_end(&mut buffer).unwrap();
                        let extracted_file_path = extraction_dir.join(name);
                        create_dir_all(extracted_file_path.parent().unwrap()).unwrap();
                        let mut file = File::create(extracted_file_path).unwrap();
                        file.write(&buffer).unwrap();
                    }
                }

                let msg = format!("Extracted {}", file.file_name().unwrap().to_str().unwrap());
                progress_bar.finish_with_message(&msg);
            } else if file.extension().unwrap() == "dmg" {
                todo!("macos extraction");
            } else {
                panic!("Unknown archive extension");
            }
        });

        let package = (*self).clone();

        let final_tasks = tokio::task::spawn(async move {
            extraction_handle.await.unwrap();

            let mut package_path = SETTINGS.read().unwrap().packages_dir.join(&package.name);

            std::fs::rename(
                SETTINGS
                    .read()
                    .unwrap()
                    .cache_dir
                    .join(get_extracted_name(&package)),
                &package_path,
            )
            .unwrap();

            package_path.push("package_info.bin");
            let file = File::create(&package_path).unwrap();
            bincode::serialize_into(file, &package).unwrap();
        });

        Ok(Some(final_tasks))
    }

    pub async fn remove(&self) -> Result<(), Box<dyn Error>> {
        let path = SETTINGS.read().unwrap().packages_dir.join(&self.name);

        remove_dir_all(path).await?;

        println!("Removed: {}", self.name);

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
            Build::None => unreachable!("Unexpected build type"),
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
