//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
pub use crate::settings::Settings;
use reqwest;
use select::document::Document;
use select::predicate::{Attr, Class, Name};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::copy;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Releases {
    pub official_releases: Vec<Release>,
    pub lts_releases: Vec<Release>,
    pub experimental_branches: Vec<Package>,
    pub latest_daily: Vec<Package>,
    pub latest_stable: Vec<Package>,
    // TODO: Add fields to hold previously downloaded packages:
    // pub previous_experimental: Vec<Package>,
    // pub previous_daily: Vec<Package>,
    // pub previous_stable: Vec<Package>,
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
                let mut release = Release::new();

                //TODO: Find some way to get the date from that horrible plain site.
                release.date = String::from("N/A");

                release.version = ver.strip_prefix("Blender").unwrap().replace("/", "");

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

                    package.build = String::from("Official Release");

                    package.version = match release.version.as_ref() {
                        "1.0" => String::from("1.0"),
                        "1.60" => String::from("1.60"),
                        "1.73" => String::from("1.73"),
                        "1.80" => package
                            .name
                            .split_terminator("-")
                            .next()
                            .unwrap()
                            .strip_prefix("blender")
                            .unwrap()
                            .to_string(),
                        "2.04" => String::from("2.04"),
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
                        "2.79latest" => String::from("2.79latest"),
                        _ => package
                            .name
                            .split_terminator("-")
                            .skip(1)
                            .next()
                            .unwrap()
                            .to_string(),
                    };

                    package.date = release.date.clone();

                    package.url = format!("{}{}", url, name);

                    package.os = {
                        if name.contains("linux") {
                            Os::Linux
                        } else if name.contains("win") {
                            Os::Windows
                        } else if name.contains("OS") {
                            Os::MacOs
                        } else {
                            unreachable!();
                        }
                    };

                    release.packages.push(package);
                }

                release
            });

            handles.push(handle);
        }

        for handle in handles {
            fetched.official_releases.push(handle.await.unwrap());
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
            let mut release = Release::new();

            let lts_id = format!("lts-release-{}{}", lts, rev);
            let version = match document.find(Attr("id", lts_id.as_str())).next() {
                Some(a) => a,
                _ => break,
            }
            .text();

            release.version = version
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

            release.date = section
                .find(Name("p"))
                .next()
                .unwrap()
                .text()
                .strip_prefix("Released on ")
                .unwrap()
                .strip_suffix(".")
                .unwrap()
                .to_string();

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

                let mut package = Package::new();

                package.version = release.version.clone();

                package.name = get_file_stem(node.text().as_str()).to_string();

                package.build = String::from("LTS Release");

                package.date = release.date.clone();

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
                        unreachable!();
                    }
                };

                release.packages.push(package);
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
                release.changelog.push(change);
            }

            fetched.lts_releases.push(release);
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

        // TODO: Think about how to rename this when a new latest stable release is out.
        package.build = String::from("Latest Stable Release");

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

        package.date = node
            .find(Class("dl-header-info-platform"))
            .next()
            .unwrap()
            .find(Name("small"))
            .next()
            .unwrap()
            .text();
        package.date = package.date.split_off(package.date.find("on").unwrap() + 3);

        package.os = {
            if o == "linux" {
                Os::Linux
            } else if o == "windows" {
                Os::Windows
            } else if o == "macos" {
                Os::MacOs
            } else {
                unreachable!();
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

            package.build = build.find(Class("build-var")).next().unwrap().text();

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

            package.date = build.find(Name("small")).next().unwrap().text();
            package.date = package
                .date
                .drain(..package.date.find('-').unwrap())
                .collect();

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
                    unreachable!();
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

            package.build = build
                .find(Class("build-var"))
                .next()
                .unwrap()
                .text()
                .split_whitespace()
                .next()
                .unwrap()
                .to_string();

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

            package.date = build.find(Name("small")).next().unwrap().text();
            package.date = package
                .date
                .drain(..package.date.find('-').unwrap())
                .collect();

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
                    unreachable!();
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

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Release {
    version: String,
    date: String,
    packages: Vec<Package>,
    changelog: Vec<Change>,
}

impl Release {
    fn new() -> Release {
        Release {
            version: String::new(),
            date: String::new(),
            packages: Vec::new(),
            changelog: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Package {
    version: String,
    name: String,
    build: String,
    date: String,
    commit: String,
    url: String,
    os: Os,
}

impl Package {
    fn new() -> Package {
        Package {
            version: String::new(),
            name: String::new(),
            build: String::new(),
            date: String::new(),
            commit: String::new(),
            url: String::new(),
            os: Os::None,
        }
    }

    pub async fn download(&self, settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
        let response = reqwest::get(&self.url).await?;

        let dest = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .unwrap();

            std::fs::create_dir_all(&settings.temp_dir)?;
            settings.temp_dir.join(fname)
        };

        if !dest.exists() {
            let mut file = File::create(&dest)?;

            let content = response.bytes().await?;
            copy(&mut content.as_ref(), &mut file)?;
        }

        Package::extract(&self, &settings)?;

        Ok(())
    }

    fn extract(package: &Package, settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
        let file = format!(
            "{}/{}",
            settings.temp_dir.to_str().unwrap(),
            package.url.split_terminator('/').last().unwrap()
        );

        std::fs::create_dir_all(&settings.packages_dir)?;

        let pack = format!(
            "{}/{}",
            settings.packages_dir.to_str().unwrap(),
            package.name
        );

        let p = Path::new(&pack);
        if p.exists() {
            return Ok(());
        }

        if cfg!(target_os = "linux") {
            use bzip2::read::BzDecoder;
            use flate2::read::GzDecoder;
            use tar::Archive;
            use xz2::read::XzDecoder;

            if file.ends_with(".xz") {
                let tar_xz = File::open(file)?;
                let tar = XzDecoder::new(tar_xz);
                let mut archive = Archive::new(tar);
                archive.unpack(&settings.packages_dir)?;
            } else if file.ends_with(".bz2") {
                let tar_bz2 = File::open(file)?;
                let tar = BzDecoder::new(tar_bz2);
                let mut archive = Archive::new(tar);
                archive.unpack(&settings.packages_dir)?;
            } else if file.ends_with(".gz") {
                let tar_gz = File::open(file)?;
                let tar = GzDecoder::new(tar_gz);
                let mut archive = Archive::new(tar);
                archive.unpack(&settings.packages_dir)?;
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

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
struct Change {
    text: String,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
enum Os {
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
