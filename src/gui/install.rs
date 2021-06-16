use super::{Message, PackageMessage};
use crate::{package::Package, settings::get_setting};
use bincode;
use glob::glob;
use iced_futures::{
    futures::stream::{unfold, BoxStream},
    subscription,
};
use reqwest;
use std::{
    fs::{create_dir_all, rename, File},
    hash::{Hash, Hasher},
    path::PathBuf,
};
use tokio::fs::{remove_dir_all, remove_file};

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
    io::{Read, Write},
    thread::sleep,
    time::Duration,
};
#[cfg(target_os = "windows")]
use zip::{read::ZipFile, ZipArchive};

macro_rules! unwrap_or_return {
    ($index:expr, $result:expr) => {
        match $result {
            Ok(x) => x,
            Err(e) => {
                eprintln!(
                    "GUI install error at {}:{}, which was:\n{:#?}",
                    file!(),
                    line!(),
                    e
                );
                return Some((
                    ($index, Progress::Errored(e.to_string())),
                    State::FinishedInstalling,
                ));
            }
        }
    };
}

pub struct Install {
    package: Package,
    index: usize,
}

impl Install {
    pub fn package(package: Package, index: usize) -> iced::Subscription<Message> {
        iced::Subscription::from_recipe(Install { package, index }).map(|(index, progress)| {
            Message::PackageMessage(index, PackageMessage::InstallationProgress(progress))
        })
    }
}

impl<H, I> subscription::Recipe<H, I> for Install
where
    H: Hasher,
{
    type Output = (usize, Progress);

    fn hash(&self, state: &mut H) {
        std::any::TypeId::of::<Self>().hash(state);
        self.package.name.hash(state);
        self.package.date.hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(unfold(
            State::ReadyToInstall {
                package: self.package,
                index: self.index,
            },
            |state| async move {
                match state {
                    State::ReadyToInstall { package, index } => {
                        let response = reqwest::get(&package.url).await;

                        match response {
                            Ok(response) => {
                                if let Some(total) = response.content_length() {
                                    create_dir_all(&get_setting().cache_dir).unwrap();

                                    let file = get_setting()
                                        .cache_dir
                                        .join(package.url.split_terminator('/').last().unwrap());

                                    // TODO: Give option to reuse previously downloaded packages.
                                    // Could have an extra button [Install from cache].
                                    // Useful for reinstalling. Will need to make sure to show an
                                    // error if that package is corrupted due to stopping the
                                    // download midway. Or make it delete whatever was downloaded
                                    // if the install was canceled. Though it's possible the file
                                    // will still be left there if the program crashed, so having
                                    // both is recommended.
                                    // Also worth considering not deleting the entry from the
                                    // database if there's a valid downloaded archive, so the user
                                    // can reinstall it even if it becomes unavailable like is the
                                    // case with daily and experimental packages.
                                    if file.exists() {
                                        unwrap_or_return!(index, remove_file(&file).await);
                                    }

                                    let package_dir =
                                        get_setting().packages_dir.join(&package.name);

                                    if package_dir.exists() {
                                        unwrap_or_return!(
                                            index,
                                            remove_dir_all(&package_dir).await
                                        );
                                    }

                                    let destination = unwrap_or_return!(
                                        index,
                                        tokio::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(&file)
                                            .await
                                    );

                                    Some((
                                        (index, Progress::Started),
                                        State::Downloading {
                                            package,
                                            response,
                                            file,
                                            destination,
                                            total,
                                            downloaded: 0,
                                            index,
                                        },
                                    ))
                                } else {
                                    Some((
                                        (
                                            index,
                                            Progress::Errored(String::from(
                                                "cannot find content length",
                                            )),
                                        ),
                                        State::FinishedInstalling,
                                    ))
                                }
                            }
                            Err(e) => Some((
                                (index, Progress::Errored(e.to_string())),
                                State::FinishedInstalling,
                            )),
                        }
                    }
                    State::Downloading {
                        package,
                        mut response,
                        file,
                        mut destination,
                        total,
                        downloaded,
                        index,
                    } => match response.chunk().await {
                        // TODO: Handle case when temporarily banned for making too many requests.
                        // I had this happen when testing too frequently. Probably not an issue for
                        // normal users, but it may make the download hang, in which case there's a
                        // need to report it to the user as an error.
                        // TODO: Handle losing connection while downloading.
                        // The good thing is that it keeps downloading when reconnected,
                        // but meanwhile it's just stuck. So maybe add a timeout.
                        Ok(Some(chunk)) => {
                            unwrap_or_return!(
                                index,
                                tokio::io::AsyncWriteExt::write_all(&mut destination, &chunk).await
                            );

                            let downloaded = downloaded + chunk.len() as u64;
                            let percentage = (downloaded as f32 / total as f32) * 100.0;

                            Some((
                                (index, Progress::DownloadProgress(percentage)),
                                State::Downloading {
                                    package,
                                    response,
                                    file,
                                    destination,
                                    total,
                                    downloaded,
                                    index,
                                },
                            ))
                        }
                        Ok(None) => Some((
                            (index, Progress::FinishedDownloading),
                            State::FinishedDownloading {
                                package,
                                file,
                                index,
                            },
                        )),
                        Err(e) => Some((
                            (index, Progress::Errored(e.to_string())),
                            State::FinishedInstalling,
                        )),
                    },
                    State::FinishedDownloading {
                        package,
                        file,
                        index,
                    } => {
                        // TODO: Figure out a way to show extraction progress on Linux.
                        // I can't pass it around due to the use of Cell and the like inside it.

                        let extraction_dir = get_setting().cache_dir.join(&package.name);

                        let archive = if file.extension().unwrap() == "xz" {
                            #[cfg(not(target_os = "linux"))]
                            unreachable!("Linux extraction on non-Linux OS");
                            #[cfg(target_os = "linux")]
                            DownloadedArchive::TarXz { extraction_dir }
                        } else if file.extension().unwrap() == "bz2" {
                            #[cfg(not(target_os = "linux"))]
                            unreachable!("Linux extraction on non-Linux OS");
                            #[cfg(target_os = "linux")]
                            DownloadedArchive::TarBz { extraction_dir }
                        } else if file.extension().unwrap() == "gz" {
                            #[cfg(not(target_os = "linux"))]
                            unreachable!("Linux extraction on non-Linux OS");
                            #[cfg(target_os = "linux")]
                            DownloadedArchive::TarGz { extraction_dir }
                        } else if file.extension().unwrap() == "zip" {
                            #[cfg(not(target_os = "windows"))]
                            unreachable!("Windows extraction on non-Windows OS");
                            #[cfg(target_os = "windows")]
                            {
                                // This is a workaround for a relatively common extraction error
                                // where apparently the extraction starts just before the file
                                // was completely written, so it was giving an "invalid Zip
                                // archive" error.
                                sleep(Duration::from_millis(250));

                                let zip = unwrap_or_return!(index, File::open(&file));
                                let archive = unwrap_or_return!(index, ZipArchive::new(zip));

                                // This handles some archives that don't have an inner directory.
                                let extraction_dir =
                                    match file.file_name().unwrap().to_str().unwrap() {
                                        "blender-2.49-win64.zip"
                                        | "blender-2.49a-win64-python26.zip"
                                        | "blender-2.49b-win64-python26.zip" => {
                                            extraction_dir.join("inner")
                                        }
                                        _ => extraction_dir,
                                    };

                                let total = archive.len() as u64;

                                DownloadedArchive::Zip {
                                    archive,
                                    extraction_dir,
                                    total,
                                    extracted: 0,
                                }
                            }
                        } else if file.extension().unwrap() == "dmg" {
                            todo!("macos extraction");
                        } else {
                            panic!("Unknown archive extension");
                        };

                        Some((
                            (index, Progress::ExtractionProgress(0.0)),
                            State::Extracting {
                                package,
                                file,
                                archive,
                                index,
                            },
                        ))
                    }
                    State::Extracting {
                        package,
                        file,
                        archive,
                        index,
                    } => match archive {
                        #[cfg(target_os = "linux")]
                        DownloadedArchive::TarXz { extraction_dir } => {
                            let tar_xz = unwrap_or_return!(index, File::open(&file));
                            let tar = XzDecoder::new(tar_xz);
                            let mut archive = Archive::new(tar);

                            for entry in unwrap_or_return!(index, archive.entries()) {
                                let mut file = unwrap_or_return!(index, entry);
                                unwrap_or_return!(index, file.unpack_in(&extraction_dir));
                            }

                            Some((
                                (index, Progress::FinishedExtracting),
                                State::FinishedExtracting { package, index },
                            ))
                        }
                        #[cfg(target_os = "linux")]
                        DownloadedArchive::TarGz { extraction_dir } => {
                            let tar_gz = unwrap_or_return!(index, File::open(&file));
                            let tar = GzDecoder::new(tar_gz);
                            let mut archive = Archive::new(tar);

                            for entry in unwrap_or_return!(index, archive.entries()) {
                                let mut file = unwrap_or_return!(index, entry);
                                unwrap_or_return!(index, file.unpack_in(&extraction_dir));
                            }

                            Some((
                                (index, Progress::FinishedExtracting),
                                State::FinishedExtracting { package, index },
                            ))
                        }
                        #[cfg(target_os = "linux")]
                        DownloadedArchive::TarBz { extraction_dir } => {
                            let tar_bz2 = unwrap_or_return!(index, File::open(&file));
                            let tar = BzDecoder::new(tar_bz2);
                            let mut archive = Archive::new(tar);

                            for entry in unwrap_or_return!(index, archive.entries()) {
                                let mut file = unwrap_or_return!(index, entry);
                                unwrap_or_return!(index, file.unpack_in(&extraction_dir));
                            }

                            Some((
                                (index, Progress::FinishedExtracting),
                                State::FinishedExtracting { package, index },
                            ))
                        }
                        #[cfg(target_os = "windows")]
                        DownloadedArchive::Zip {
                            mut archive,
                            extraction_dir,
                            total,
                            extracted,
                        } => {
                            {
                                // TODO: Show progress with bytes to avoid looking stuck.
                                let mut entry: ZipFile<'_> =
                                    unwrap_or_return!(index, archive.by_index(extracted as usize));
                                let entry_name = entry.name().to_owned();

                                if entry.is_dir() {
                                    let extracted_dir_path = extraction_dir.join(entry_name);
                                    unwrap_or_return!(index, create_dir_all(extracted_dir_path));
                                } else if entry.is_file() {
                                    let mut buffer: Vec<u8> = Vec::new();
                                    unwrap_or_return!(index, entry.read_to_end(&mut buffer));
                                    let extracted_file_path = extraction_dir.join(entry_name);
                                    unwrap_or_return!(
                                        index,
                                        create_dir_all(extracted_file_path.parent().unwrap())
                                    );
                                    let mut file =
                                        unwrap_or_return!(index, File::create(extracted_file_path));
                                    unwrap_or_return!(index, file.write(&buffer));
                                }
                            }

                            let extracted = extracted + 1;
                            let percentage = (extracted as f32 / total as f32) * 100.0;

                            let archive = DownloadedArchive::Zip {
                                archive,
                                extraction_dir,
                                total,
                                extracted,
                            };

                            if extracted == total {
                                Some((
                                    (index, Progress::FinishedExtracting),
                                    State::FinishedExtracting { package, index },
                                ))
                            } else {
                                Some((
                                    (index, Progress::ExtractionProgress(percentage)),
                                    State::Extracting {
                                        package,
                                        file,
                                        archive,
                                        index,
                                    },
                                ))
                            }
                        }
                    },
                    State::FinishedExtracting { package, index } => {
                        let extracted_path = glob(&format!(
                            "{}/*",
                            get_setting()
                                .cache_dir
                                .join(&package.name)
                                .to_str()
                                .unwrap()
                        ))
                        .unwrap()
                        .next()
                        .unwrap()
                        .unwrap();

                        let mut package_path = get_setting().packages_dir.join(&package.name);

                        // TODO: Fix moving directories across filesystems.
                        // Can probably use the `fs_extra` crate, which I'm already depending on.
                        unwrap_or_return!(index, rename(extracted_path, &package_path));

                        package_path.push("package_info.bin");
                        let file = unwrap_or_return!(index, File::create(&package_path));
                        unwrap_or_return!(index, bincode::serialize_into(file, &package));

                        Some((
                            (index, Progress::FinishedInstalling),
                            State::FinishedInstalling,
                        ))
                    }
                    State::FinishedInstalling => {
                        let _: () = iced::futures::future::pending().await;

                        None
                    }
                }
            },
        ))
    }
}

#[derive(Clone, Debug)]
pub enum Progress {
    Started,
    DownloadProgress(f32),
    FinishedDownloading,
    ExtractionProgress(f32),
    FinishedExtracting,
    FinishedInstalling,
    Errored(String),
}

enum State {
    ReadyToInstall {
        package: Package,
        index: usize,
    },
    Downloading {
        package: Package,
        response: reqwest::Response,
        file: PathBuf,
        destination: tokio::fs::File,
        total: u64,
        downloaded: u64,
        index: usize,
    },
    FinishedDownloading {
        package: Package,
        file: PathBuf,
        index: usize,
    },
    Extracting {
        package: Package,
        file: PathBuf,
        archive: DownloadedArchive,
        index: usize,
    },
    FinishedExtracting {
        package: Package,
        index: usize,
    },
    FinishedInstalling,
}

enum DownloadedArchive {
    #[cfg(target_os = "linux")]
    TarXz { extraction_dir: PathBuf }, // { entries: Entries<XzDecoder<File>> },
    #[cfg(target_os = "linux")]
    TarBz { extraction_dir: PathBuf }, // { entries: Entries<BzDecoder<File>> },
    #[cfg(target_os = "linux")]
    TarGz { extraction_dir: PathBuf }, // { entries: Entries<GzDecoder<File>> },
    #[cfg(target_os = "windows")]
    Zip {
        archive: ZipArchive<File>,
        extraction_dir: PathBuf,
        total: u64,
        extracted: u64,
    },
}
