use crate::settings::{get_setting, CAN_CONNECT};
use reqwest::{self, ClientBuilder};
use select::document::Document;
use std::{path::Path, process::Command, sync::atomic::Ordering, time::Duration};

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
