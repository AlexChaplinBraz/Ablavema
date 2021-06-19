use crate::settings::{get_setting, CAN_CONNECT};
use clap::crate_version;
use fs_extra::file::{move_file, CopyOptions};
use reqwest::{self, ClientBuilder};
use select::document::Document;
use self_update::{backends::github::ReleaseList, update::Release};
use std::{
    env::current_exe,
    fs::{rename, File},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::Ordering,
    time::Duration,
};

/// Check whether there's a working connection to the download servers.
pub async fn check_connection() {
    // TODO: Fix rare false negative.
    // Seems to happen randomly, where one of the servers is momentarily unresponsive.
    // Could be fixed by looping through the check once more if there was an error,
    // since this just gets fixed if you retry manually right away.

    let urls = [
        "https://builder.blender.org/download/",
        "https://www.blender.org/download/",
        "https://ftp.nluug.nl/pub/graphics/blender/release/",
        "https://github.com/AlexChaplinBraz/Ablavema",
    ];

    let client = ClientBuilder::new()
        .connect_timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    for url in urls.iter() {
        match client.get(*url).send().await {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    CAN_CONNECT.store(false, Ordering::Relaxed);
                    return;
                }
            }
            Err(_) => {
                CAN_CONNECT.store(false, Ordering::Relaxed);
                return;
            }
        }
    }

    CAN_CONNECT.store(true, Ordering::Relaxed);
}

pub async fn get_document(url: &str) -> Document {
    // TODO: Fix hang on getting temp banned mid fetching.
    // Should be resolved by adding a timeout, but the requirement is being
    // able to pass an error around and handle it.
    let resp = reqwest::get(url).await.unwrap();
    assert!(resp.status().is_success());
    let resp = resp.bytes().await.unwrap();
    Document::from_read(&resp[..]).unwrap()
}

pub fn fetch_self_releases() -> Option<Vec<Release>> {
    let releases = ReleaseList::configure()
        .repo_owner("AlexChaplinBraz")
        .repo_name("Ablavema")
        .with_target(&self_update::get_target())
        .build()
        .unwrap()
        .fetch()
        .unwrap();

    if releases.is_empty() {
        None
    } else {
        Some(releases)
    }
}

pub fn check_self_updates(releases_option: &Option<Vec<Release>>) -> Option<usize> {
    match releases_option {
        Some(releases) => match releases
            .iter()
            .enumerate()
            .find(|(_, release)| release.version == crate_version!())
        {
            Some((index, _)) => index,
            // TODO: Might need to handle cases where the current release doesn't exist,
            // like if it was deleted. I made it at least so it doesn't crash,
            // but it shouldn't happen in practice anyway.
            None => return None,
        }
        .return_option(),
        None => None,
    }
}

pub fn change_self_version(releases: Vec<Release>, version: String) {
    let asset = releases
        .iter()
        .find(|release| release.version == version)
        .unwrap()
        .asset_for(&self_update::get_target())
        .unwrap();

    let archive_path = get_setting().cache_dir.join(asset.name);
    let archive = File::create(&archive_path).unwrap();

    self_update::Download::from_url(&asset.download_url)
        .set_header(
            reqwest::header::ACCEPT,
            "application/octet-stream".parse().unwrap(),
        )
        .download_to(&archive)
        .unwrap();

    let bin_archive_path = PathBuf::from(if cfg!(target_os = "linux") {
        format!(
            "ablavema-{}-{}/ablavema",
            version,
            self_update::get_target()
        )
    } else if cfg!(target_os = "windows") {
        format!(
            "ablavema-{}-{}/ablavema.exe",
            version,
            self_update::get_target()
        )
    } else if cfg!(target_os = "macos") {
        todo!("macos bin_name");
    } else {
        unreachable!("Unsupported OS");
    });

    self_update::Extract::from_source(&archive_path)
        .extract_file(&get_setting().cache_dir, &bin_archive_path)
        .unwrap();

    let bin_path = get_setting().cache_dir.join(bin_archive_path);
    let temp_path = current_exe().unwrap().parent().unwrap().join("temp");

    move_file(bin_path, &temp_path, &CopyOptions::new()).unwrap();
    rename(temp_path, current_exe().unwrap()).unwrap();
}

pub fn open_blender(package: String, file_path: Option<String>) {
    let mut cmd = Command::new(get_setting().packages_dir.join(package).join({
        if cfg!(target_os = "linux") {
            "blender"
        } else if cfg!(target_os = "windows") {
            "blender.exe"
        } else if cfg!(target_os = "macos") {
            todo!("macos executable");
        } else {
            unreachable!("Unsupported OS");
        }
    }));
    if let Some(path) = file_path {
        cmd.arg(path);
    }
    // TODO: Consider handling possible errors when launching Blender.
    // I've seen this panic inside a Windows VM with:
    // "The application has failed to start because its side-by-side configuration is incorrect.
    // Please see the application event log or use the command-line sxstrace.exe tool for more detail."
    // Which is the same message that appears on a dialog if I try to launch that same package from
    // the explorer, so it should probably be displayed as a dialog as well.
    //
    // The problem is that there are also messages like these:
    //
    // ---------------------------
    // Blender - Can't detect 3D hardware accelerated Driver!
    // ---------------------------
    // Your system does not use 3D hardware acceleration.
    // Blender requires a graphics driver with OpenGL 2.1 support.
    //
    // This may be caused by:
    // * A missing or faulty graphics driver installation.
    //   Blender needs a graphics card driver to work correctly.
    // * Accessing Blender through a remote connection.
    // * Using Blender through a virtual machine.
    //
    // The program will now close.
    // ---------------------------
    // OK
    // ---------------------------
    //
    // Which do not panic here and just seem to be eaten up, not displaying the dialog at all.
    cmd.spawn().unwrap();
}

pub fn get_file_stem(filename: &str) -> &str {
    if filename.contains(".tar.") {
        let f = Path::new(filename).file_stem().unwrap().to_str().unwrap();
        Path::new(f).file_stem().unwrap().to_str().unwrap()
    } else {
        Path::new(filename).file_stem().unwrap().to_str().unwrap()
    }
}

pub fn is_time_to_update() -> bool {
    get_setting()
        .last_update_time
        .elapsed()
        .unwrap()
        .as_secs()
        .checked_div(60)
        .unwrap()
        >= get_setting().minutes_between_updates
}

pub trait ReturnOption: Default + PartialEq {
    fn return_option(self) -> Option<Self> {
        if self == Self::default() {
            None
        } else {
            Some(self)
        }
    }
}

impl ReturnOption for usize {}
