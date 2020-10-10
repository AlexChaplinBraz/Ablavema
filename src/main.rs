//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![allow(dead_code, unused_imports, unused_variables)]
use reqwest;
//use scraper::{Html, Selector};
use rayon::prelude::*;
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};
use std::path::Path;
use std::sync::{Arc, Mutex};

fn main() {
    let mut releases = Releases::new();
    releases.fetch_official_releases();
    releases.fetch_lts_releases();
    releases.fetch_latest_stable();
    releases.fetch_latest_daily();
    releases.fetch_experimental_branches();
    println!("{:#?}", releases);
}

#[derive(Debug)]
struct Releases {
    official_releases: Arc<Mutex<Vec<Release>>>,
    lts_releases: Vec<Release>,
    experimental_branches: Vec<Package>,
    latest_daily: Vec<Package>,
    latest_stable: Vec<Package>,
}

impl Releases {
    fn new() -> Releases {
        Releases {
            official_releases: Arc::new(Mutex::new(Vec::new())),
            lts_releases: Vec::new(),
            experimental_branches: Vec::new(),
            latest_daily: Vec::new(),
            latest_stable: Vec::new(),
        }
    }

    fn fetch_official_releases(&mut self) {
        let url = "https://download.blender.org/release/";
        let resp = reqwest::blocking::get(url).unwrap();
        assert!(resp.status().is_success());
        let document = Document::from_read(resp).unwrap();

        let mut versions = Vec::new();

        for node in document.find(Name("a")) {
            let url_path = node.attr("href").unwrap();
            versions.push(url_path);
        }

        versions.retain(|x| x.contains("Blender") && x.ends_with('/') && !x.contains("Benchmark"));
        versions.push("Blender2.79/latest/");

        versions.par_iter().for_each(|ver| {
            let mut release = Release::new();

            //TODO: Find some way to get the date from that horrible plain site.
            release.date = String::from("N/A");

            release.version = ver.strip_prefix("Blender").unwrap().replace("/", "");

            let url = format!("{}{}", "https://download.blender.org/release/", ver);

            let resp = reqwest::blocking::get(url.as_str()).unwrap();
            assert!(resp.status().is_success());
            let document = Document::from_read(resp).unwrap();
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
                //&& !x.contains("i386")
                //&& !x.contains("i686")
                //&& !x.contains("-win32")
                //&& !x.contains("-windows32")
            });
            builds.reverse();

            for name in builds {
                let mut package = Package::new();

                package.name = get_file_stem(name).to_string();

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

            self.official_releases.lock().unwrap().push(release);
        });

        self.official_releases
            .lock()
            .unwrap()
            .sort_by_key(|x| x.version.clone());
        self.official_releases.lock().unwrap().reverse();
    }

    fn fetch_lts_releases(&mut self) {
        let url = "https://www.blender.org/download/lts/";
        let resp = reqwest::blocking::get(url).unwrap();
        assert!(resp.status().is_success());
        let document = Document::from_read(resp).unwrap();

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

                let mut package = Package::new();

                package.version = release.version.clone();

                package.name = get_file_stem(node.text().as_str()).to_string();

                package.date = release.date.clone();

                // Hardcoded due to the stupid redirect. Could follow it dynamically,
                // but seems unnecessary.
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

            self.lts_releases.push(release);
        }

        self.lts_releases.reverse();
    }

    fn fetch_latest_stable(&mut self) {
        let url = "https://www.blender.org/download/";
        let resp = reqwest::blocking::get(url).unwrap();
        assert!(resp.status().is_success());
        let document = Document::from_read(resp).unwrap();

        for o in vec!["linux", "windows", "macos"] {
            let node = document.find(Attr("id", o)).next().unwrap();
            let mut package = Package::new();

            package.version = node.find(Name("a")).next().unwrap().text();
            package
                .version
                .retain(|c| c.is_numeric() || c.is_ascii_punctuation());

            package.name = String::from("Latest Stable");

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

            self.latest_stable.push(package);
        }
    }

    fn fetch_latest_daily(&mut self) {
        let url = "https://builder.blender.org/download/";
        let resp = reqwest::blocking::get(url).unwrap();
        assert!(resp.status().is_success());
        let document = Document::from_read(resp).unwrap();

        for build in document.find(Class("os")) {
            let mut package = Package::new();

            package.name = build.find(Class("build-var")).next().unwrap().text();

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

            package.os = {
                let o = build.find(Class("build")).next().unwrap().text();
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

            self.latest_daily.push(package);
        }
    }

    fn fetch_experimental_branches(&mut self) {
        let url = "https://builder.blender.org/download/branches/";
        let resp = reqwest::blocking::get(url).unwrap();
        assert!(resp.status().is_success());
        let document = Document::from_read(resp).unwrap();

        for build in document.find(Class("os")) {
            let mut package = Package::new();

            package.name = build
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

            package.os = {
                let o = build.find(Class("build")).next().unwrap().text();
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

            self.experimental_branches.push(package);
        }
    }
}

#[derive(Debug)]
struct Release {
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

#[derive(Debug)]
struct Package {
    version: String,
    name: String,
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
            date: String::new(),
            commit: String::new(),
            url: String::new(),
            os: Os::None,
        }
    }
}

#[derive(Debug)]
struct Change {
    text: String,
    url: String,
}

#[derive(Debug)]
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
