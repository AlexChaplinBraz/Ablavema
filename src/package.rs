//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{helpers::*, settings::*};
use bzip2::read::BzDecoder;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use flate2::read::GzDecoder;
use iced::button;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{self, header, Client};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub version: String,
    pub name: String,
    pub build: Build,
    pub date: NaiveDateTime,
    pub commit: String,
    pub url: String,
    pub os: Os,
    pub changelog: Vec<Change>,
    #[serde(skip)]
    pub state: PackageState,
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.name == other.name
            && self.build == other.build
            && self.date == other.date
            && self.os == other.os
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PackageState {
    Fetched {
        install_button: button::State,
    },
    Downloading {
        progress: f32,
    },
    Extracting {
        progress: f32,
    },
    Installed {
        open_button: button::State,
        open_file_button: button::State,
        set_default_button: button::State,
        remove_button: button::State,
    },
    Errored {
        retry_button: button::State,
    },
}

impl Default for PackageState {
    fn default() -> Self {
        Self::Fetched {
            install_button: button::State::new(),
        }
    }
}

impl Package {
    pub fn new() -> Package {
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
            state: PackageState::default(),
        }
    }

    pub async fn cli_install(
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

        let progress_bar = multi_progress.add(ProgressBar::new(get_count(
            file.file_name().unwrap().to_str().unwrap(),
        )));
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

    pub async fn cli_remove(&self) -> Result<(), Box<dyn Error>> {
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
