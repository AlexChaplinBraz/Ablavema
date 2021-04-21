//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    helpers::{get_count, get_extracted_name},
    settings::SETTINGS,
};
use bincode;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use iced::button;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::{
    fs::{rename, File},
    mem,
};
use timeago::{self, TimeUnit::Minutes};
use tokio::{
    fs::{self, remove_dir_all, remove_file},
    io::AsyncWriteExt,
    task::{spawn, JoinHandle},
};
use versions::Versioning;

#[cfg(target_os = "linux")]
use bzip2::read::BzDecoder;
#[cfg(target_os = "linux")]
use flate2::read::GzDecoder;
#[cfg(target_os = "linux")]
use tar::Archive;
#[cfg(target_os = "linux")]
use xz2::read::XzDecoder;

#[cfg(target_os = "windows")]
use std::{
    fs::create_dir_all,
    io::{Read, Write},
};
#[cfg(target_os = "windows")]
use zip::{read::ZipFile, ZipArchive};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Package {
    // TODO: Add "label" field so users can describe what a package is for if needed.
    pub version: Versioning,
    pub name: String,
    pub build: Build,
    pub date: NaiveDateTime,
    pub commit: String,
    pub url: String,
    pub os: Os,
    pub changelog: Vec<Change>,
    pub bookmarked: bool,
    #[serde(skip)]
    pub bookmark_button: button::State,
    #[serde(skip)]
    pub state: PackageState,
    #[serde(skip)]
    pub status: PackageStatus,
}

impl Package {
    pub async fn cli_install(
        &self,
        multi_progress: &MultiProgress,
        flags: &(bool, bool),
    ) -> Option<JoinHandle<()>> {
        // TODO: Consider reusing the Install module for the CLI as well.

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

            return None;
        } else if package.exists() && flags.0 {
            remove_dir_all(&package).await.unwrap();
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
                remove_file(&file).await.unwrap();
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
                    panic!("{}", error);
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

            download_handle = Some(spawn(async move {
                while let Some(chunk) = source.chunk().await.unwrap() {
                    dest.write_all(&chunk).await.unwrap();
                    progress_bar.inc(chunk.len() as u64);
                }

                progress_bar.finish_with_message(&msg);
            }));
        }

        let progress_bar = multi_progress.add(ProgressBar::new(get_count(
            file.file_name().unwrap().to_str().unwrap(),
        )));
        progress_bar.set_style(extraction_style.clone());

        let extraction_handle = spawn(async move {
            if let Some(handle) = download_handle {
                handle.await.unwrap();
            }

            let msg = format!("Extracting {}", file.file_name().unwrap().to_str().unwrap());
            progress_bar.set_message(&msg);
            progress_bar.reset_elapsed();
            progress_bar.enable_steady_tick(250);

            if file.extension().unwrap() == "xz" {
                // This can be used to calculate entry count for newer archives.
                // Leaving it here just in case.
                /*
                let tar_xz_len = File::open(&file).unwrap();
                let tar_len = XzDecoder::new(tar_xz_len);
                let mut archive_len = Archive::new(tar_len);
                let plen = archive_len.entries().unwrap().count();
                let mut count_file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("counts.txt")
                    .unwrap();
                writeln!(
                    count_file,
                    "(\"{}\", {}),",
                    file.file_name().unwrap().to_str().unwrap(),
                    plen
                )
                .unwrap();
                */

                #[cfg(target_os = "linux")]
                {
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
                }
            } else if file.extension().unwrap() == "bz2" {
                #[cfg(target_os = "linux")]
                {
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
                }
            } else if file.extension().unwrap() == "gz" {
                #[cfg(target_os = "linux")]
                {
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
                }
            } else if file.extension().unwrap() == "zip" {
                #[cfg(target_os = "windows")]
                {
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
                        let mut entry: ZipFile<'_> = archive.by_index(file_index).unwrap();
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
                }
            } else if file.extension().unwrap() == "dmg" {
                todo!("macos extraction");
            } else {
                panic!("Unknown archive extension");
            }

            true
        });

        let package = (*self).clone();

        let final_tasks = spawn(async move {
            extraction_handle.await.unwrap();

            let mut package_path = SETTINGS.read().unwrap().packages_dir.join(&package.name);

            rename(
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

        Some(final_tasks)
    }

    pub fn get_formatted_date_time(&self) -> String {
        let mut formatter = timeago::Formatter::new();
        formatter.num_items(2);
        formatter.min_unit(Minutes);
        let duration = Utc::now().naive_utc().signed_duration_since(self.date);
        format!(
            "{} ({})",
            self.date.format("%B %d, %Y - %T"),
            formatter.convert(duration.to_std().unwrap())
        )
    }

    pub fn remove(&self) {
        let path = SETTINGS.read().unwrap().packages_dir.join(&self.name);
        std::fs::remove_dir_all(path).unwrap();
        println!("Removed: {}", self.name);
    }

    pub fn take(&mut self) -> Self {
        mem::take(self)
    }
}

impl Default for Package {
    fn default() -> Self {
        Package {
            version: Versioning::default(),
            name: String::default(),
            build: Build::Archived,
            date: NaiveDateTime::new(
                NaiveDate::from_ymd(1999, 12, 31),
                NaiveTime::from_hms(23, 59, 59),
            ),
            commit: String::default(),
            url: String::default(),
            os: Os::Linux,
            changelog: Vec::default(),
            bookmarked: false,
            bookmark_button: Default::default(),
            state: PackageState::default(),
            status: PackageStatus::default(),
        }
    }
}

impl Eq for Package {}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.build {
            Build::Daily(_) | Build::Branched(_) => self
                .build
                .cmp(&other.build)
                .then(self.date.cmp(&other.date).reverse()),
            Build::Stable | Build::Lts | Build::Archived => {
                Ord::cmp(&self.version, &other.version).reverse()
            }
        }
    }
}

impl PartialEq for Package {
    // TODO: Consider what to do in case of having the same package name but different date.
    // Not really to be solved here, but I remember once where there were no commits in the
    // daily build for an entire day so it was the same package name but with a different date.
    // This would only bring trouble when trying to have both of them installed, but ultimately
    // being the same package means the worst that could happen is that it updates for no gain
    // whatsoever, removing the older package.
    fn eq(&self, other: &Self) -> bool {
        match self.build {
            Build::Daily(_) | Build::Branched(_) => {
                self.build == other.build && self.date == other.date
            }
            Build::Stable | Build::Lts | Build::Archived => {
                self.name == other.name && self.date == other.date
            }
        }
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Build {
    Daily(String),
    Branched(String),
    Stable,
    Lts,
    Archived,
}

impl std::fmt::Display for Build {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let printable = match self {
            Build::Daily(s) | Build::Branched(s) => s,
            Build::Stable => "Stable Release",
            Build::Lts => "LTS Release",
            Build::Archived => "Archived Release",
        };
        write!(f, "{}", printable)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Change {
    pub text: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub enum PackageState {
    Fetched {
        install_button: button::State,
    },
    Downloading {
        progress: f32,
        cancel_button: button::State,
    },
    Extracting {
        progress: f32,
        cancel_button: button::State,
    },
    Installed {
        open_button: button::State,
        open_file_button: button::State,
        set_default_button: button::State,
        remove_button: button::State,
    },
    Errored {
        error_message: String,
        retry_button: button::State,
    },
}

impl PackageState {
    pub fn default_installed() -> Self {
        PackageState::Installed {
            open_button: Default::default(),
            open_file_button: Default::default(),
            set_default_button: Default::default(),
            remove_button: Default::default(),
        }
    }
}

impl Default for PackageState {
    fn default() -> Self {
        Self::Fetched {
            install_button: button::State::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PackageStatus {
    Update,
    New,
    Old,
}

impl Default for PackageStatus {
    fn default() -> Self {
        Self::Old
    }
}
