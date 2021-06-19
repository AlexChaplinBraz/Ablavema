use crate::{helpers::ReturnOption, settings::get_setting};
use clap::crate_version;
use fs_extra::file::{move_file, CopyOptions};
use self_update::{backends::github::ReleaseList, update::Release};
use std::{
    env::current_exe,
    fs::{rename, File},
    path::PathBuf,
};

pub struct SelfUpdater;

impl SelfUpdater {
    pub fn fetch() -> Option<Vec<Release>> {
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

    pub fn count_new(releases_option: &Option<Vec<Release>>) -> Option<usize> {
        match releases_option {
            Some(releases) => match releases
                .iter()
                .enumerate()
                .find(|(_, release)| release.version == crate_version!())
            {
                Some((index, _)) => index,
                None => return None,
            }
            .return_option(),
            None => None,
        }
    }

    pub fn change(releases: Vec<Release>, version: String) {
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
}
