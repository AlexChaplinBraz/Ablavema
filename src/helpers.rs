//#![allow(dead_code, unused_imports, unused_variables, unused_macros)]
use crate::{
    package::{Build, Package, PackageState},
    settings::{get_setting, set_setting, CAN_CONNECT},
};
use clap::{crate_version, ArgMatches};
use indicatif::MultiProgress;
use lazy_static::lazy_static;
use prettytable::{
    cell,
    format::{self, FormatBuilder},
    row, table, Table,
};
use reqwest::{self, ClientBuilder};
use select::document::Document;
use self_update::{backends::github::ReleaseList, update::Release};
use std::{
    collections::HashMap,
    env::current_exe,
    fs::File,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
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

    // TODO: Offer an option to restore previous version.
    // Could maybe even save them with their versions in the name so it'd be possible
    // to quickly swich them around without having to redownload.
    let tmp_file = get_setting().cache_dir.join("ablavema_backup");
    let bin_path = get_setting().cache_dir.join(bin_archive_path);
    self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_file)
        .to_dest(&current_exe().unwrap())
        .unwrap();
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
    cmd.spawn().unwrap();
}

pub fn process_bool_arg(arg: &ArgMatches<'_>, name: &str) {
    if arg.is_present(name) {
        let new_arg = expand_bool(arg.value_of(name).unwrap());
        let old_arg = read_bool_setting(name);
        if new_arg == old_arg {
            println!(
                "'{}' is unchanged from '{}'.",
                name.replace("_", "-"),
                old_arg
            );
        } else {
            write_bool_setting(name, new_arg);
            println!(
                "'{}' changed from '{}' to '{}'.",
                name.replace("_", "-"),
                old_arg,
                new_arg
            );
        }
    }
}

fn expand_bool(boolean: &str) -> bool {
    match boolean {
        "t" => true,
        "f" => false,
        "true" => true,
        "false" => false,
        _ => unreachable!("Unexpected boolean value"),
    }
}

fn read_bool_setting(name: &str) -> bool {
    match name {
        "bypass_launcher" => get_setting().bypass_launcher,
        "use_latest_as_default" => get_setting().use_latest_as_default,
        "check_updates_at_launch" => get_setting().check_updates_at_launch,
        "update_daily" => get_setting().update_daily,
        "update_branched" => get_setting().update_branched,
        "update_stable" => get_setting().update_stable,
        "update_lts" => get_setting().update_lts,
        "keep_only_latest_daily" => get_setting().keep_only_latest_daily,
        "keep_only_latest_branched" => get_setting().keep_only_latest_branched,
        "keep_only_latest_stable" => get_setting().keep_only_latest_stable,
        "keep_only_latest_lts" => get_setting().keep_only_latest_lts,
        _ => panic!("Unknown boolean field"),
    }
}

fn write_bool_setting(name: &str, value: bool) {
    match name {
        "bypass_launcher" => set_setting().bypass_launcher = value,
        "use_latest_as_default" => set_setting().use_latest_as_default = value,
        "check_updates_at_launch" => set_setting().check_updates_at_launch = value,
        "update_daily" => set_setting().update_daily = value,
        "update_branched" => set_setting().update_branched = value,
        "update_stable" => set_setting().update_stable = value,
        "update_lts" => set_setting().update_lts = value,
        "keep_only_latest_daily" => set_setting().keep_only_latest_daily = value,
        "keep_only_latest_branched" => set_setting().keep_only_latest_branched = value,
        "keep_only_latest_stable" => set_setting().keep_only_latest_stable = value,
        "keep_only_latest_lts" => set_setting().keep_only_latest_lts = value,
        _ => panic!("Unknown boolean field"),
    }
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
    if get_setting()
        .last_update_time
        .elapsed()
        .unwrap()
        .as_secs()
        .checked_div(60)
        .unwrap()
        >= get_setting().minutes_between_updates
    {
        true
    } else {
        false
    }
}

pub async fn cli_install(args: &ArgMatches<'_>, packages: &Vec<Package>, name: &str) {
    if CAN_CONNECT.load(Ordering::Relaxed) {
        let multi_progress = MultiProgress::new();
        let flags = (args.is_present("reinstall"), args.is_present("redownload"));
        let mut values = Vec::new();

        for build in args.values_of("id").unwrap() {
            if values.contains(&build.to_string()) {
                continue;
            }
            values.push(build.to_string());

            if args.is_present("name") {
                match packages.iter().find(|package| package.name == build) {
                    Some(a) => a.cli_install(&multi_progress, &flags).await,
                    None => {
                        println!("No {} package named '{}' found.", name, build);
                        continue;
                    }
                }
            } else {
                let build = usize::from_str(build).unwrap();

                match packages
                    .iter()
                    .enumerate()
                    .find(|(index, _)| *index == build)
                {
                    Some(a) => a.1.cli_install(&multi_progress, &flags).await,
                    None => {
                        println!("No {} package with ID '{}' found.", name, build);
                        continue;
                    }
                }
            };
        }

        multi_progress.join().unwrap();
    } else {
        println!("Error: Failed to connect to servers.");
    }
}

pub fn cli_list_narrow(packages: &Vec<Package>, name: &str, invert: bool) {
    let mut table = Table::new();
    table.set_titles(row!["ID", "Package"]);

    for (index, package) in packages.iter().enumerate() {
        if !matches!(package.state, PackageState::Installed { .. }) {
            // This is a workaround for the issue of prettytable having a weird behaviour when a cell
            // has hspan > 1, affecting the other cells and making them uneven based on the content
            // length of the cell with hspan > 1.
            let details = format!("{} | {} | {}", package.date, package.version, package.build);
            let mut package = table!([package.name], [details]);

            let inner_format = FormatBuilder::new().padding(0, 0).build();
            package.set_format(inner_format);

            table.add_row(row![index, package]);
        }
    }

    if table.is_empty() {
        eprintln!("No {} packages found.", name);
    } else if invert {
        let mut inverted_table = Table::new();
        inverted_table.set_titles(row!["ID", "Package"]);

        for r in table.row_iter().rev() {
            inverted_table.add_row(r.to_owned());
        }

        inverted_table.printstd();
    } else {
        table.printstd();
    }
}

pub fn cli_list_wide(packages: &Vec<Package>, name: &str, invert: bool) {
    let mut table = Table::new();
    table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    for (index, package) in packages.iter().enumerate() {
        if !matches!(package.state, PackageState::Installed { .. }) {
            table.add_row(row![
                index,
                package.name,
                package.version,
                package.build,
                package.date
            ]);
        }
    }

    if table.is_empty() {
        eprintln!("No {} packages found.", name);
    } else if invert {
        let mut inverted_table = Table::new();
        inverted_table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
        inverted_table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

        for r in table.row_iter().rev() {
            inverted_table.add_row(r.to_owned());
        }

        inverted_table.printstd();
    } else {
        table.printstd();
    }
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

/// Handles cases where the extracted directory isn't named the same
/// as the downloaded archive from which the name of the package is taken.
pub fn get_extracted_name(package: &Package) -> &str {
    match EXTRACTED_NAMES.get(&package.name.as_ref()) {
        Some(s) => *s,
        None => {
            if package.build == Build::Stable {
                package.name.trim_end_matches("-stable")
            } else if package.build == Build::Lts {
                package.name.trim_end_matches("-lts")
            } else if package.build == Build::Archived {
                package.name.trim_end_matches("-archived")
            } else {
                &package.name
            }
        }
    }
}

/// Handles getting the hardcoded entry count for archives. Exists mostly because there's
/// no length method for tar archives and calculating it is too costly (up to 30 seconds).
/// This deals mainly with older packages and gives an approximate default for newer ones.
pub fn get_count(file: &str) -> u64 {
    match ARCHIVE_ITEM_COUNT.get(file) {
        Some(v) => *v,
        None => 4800,
    }
}

lazy_static! {
    static ref EXTRACTED_NAMES: HashMap<&'static str, &'static str> = [
        (
            "blender-2.27.NewPy1-linux-glibc2.3.2-i386-archived",
            "blender-2.27-linux-glibc2.3.2-i386"
        ),
        (
            // Seems like a wrongly packaged version.
            // This actually conflicts if the 2.35b archive is being installed
            // at the same time as the actual 2.35a archive.
            // There's no error, but the files end up distributed between
            // the two directories, breaking both packages.
            // TODO: I don't even.
            "blender-2.35b-linux-glibc2.2.5-i386-archived",
            "blender-2.35a-linux-glibc2.2.5-i386"
        ),
        ("blender-2.5-alpha1-linux-glibc27-x86_64-archived", "blender-2.50-alpha1-linux-glibc27-x86_64"),
        ("blender-2.5-alpha1-linux-glibc27-i686-archived", "blender-2.50-alpha1-linux-glibc27-i686"),
        (
            "blender-2.27.NewPy1-windows-archived",
            "blender-2.27-windows"
        ),
        ("blender-2.47-windows-law-archived", "blender-2.47-windows"),
        ("blender-2.48-windows64-archived", "Blender248"),
        ("blender-2.48a-windows64-archived", "Blender248a"),
        ("blender-2.5-alpha1-win64-archived", "blender25-win64-26982"),
        ("blender-2.5-alpha2-win64-archived", "Release"),
        (
            "blender-2.79-e045fe53f1b0-win64-archived",
            "blender-2.79.0-git.e045fe53f1b0-windows64"
        ),
        (
            "blender-2.79-e045fe53f1b0-win32-archived",
            "blender-2.79.0-git.e045fe53f1b0-windows32"
        ),
    ]
    .iter()
    .copied()
    .collect();
    static ref ARCHIVE_ITEM_COUNT: HashMap<&'static str, u64> = [
        ("blender1.80a-linux-glibc2.1.2-i386.tar.gz", 51),
        ("blender1.80-linux-glibc2.1.3-alpha.tar.gz", 47),
        ("blender2.04-linux-glibc2.1.2-i386.tar.gz", 26),
        ("blender2.04-linux-glibc2.1.3-alpha.tar.gz", 26),
        ("blender-2.26-linux-glibc2.3.1-i386.tar.gz", 27),
        ("blender-2.26-linux-glibc2.2.5-i386.tar.gz", 27),
        ("blender-2.27-linux-glibc2.2.5-i386.tar.gz", 56),
        ("blender-2.27.NewPy1-linux-glibc2.3.2-i386.tar.gz", 56),
        ("blender-2.28-linux-glibc2.3.2-i386.tar.gz", 56),
        ("blender-2.28a-linux-glibc2.2.5-i386.tar.gz", 56),
        ("blender-2.28-linux-glibc2.2.5-i386.tar.gz", 56),
        ("blender-2.28c-linux-glibc2.2.5-i386.tar.gz", 60),
        ("blender-2.30-linux-glibc2.2.5-i386.tar.gz", 61),
        ("blender-2.31-linux-glibc2.2.5-i386.tar.gz", 62),
        ("blender-2.31a-linux-glibc2.2.5-i386.tar.gz", 62),
        ("blender-2.32-linux-glibc2.2.5-i386.tar.gz", 84),
        ("blender-2.33-linux-glibc2.2.5-i386.tar.gz", 86),
        ("blender-2.33a-linux-glibc2.2.5-i386.tar.gz", 87),
        ("blender-2.34-linux-glibc2.2.5-i386.tar.gz", 141),
        ("blender-2.35a-linux-glibc2.2.5-i386.tar.gz", 145),
        ("blender-2.35b-linux-glibc2.2.5-i386.tar.gz", 145),
        ("blender-2.36-linux-glibc2.2.5-i386-1.tar.gz", 155),
        ("blender-2.37-linux-glibc2.2.5-i386.tar.bz2", 155),
        ("blender-2.37a-linux-glibc2.2.5-i386.tar.bz2", 175),
        ("blender-2.40-linux-glibc232-py24-i386.tar.bz2", 169),
        ("blender-2.40-linux-glibc232-py23-i386.tar.bz2", 169),
        ("blender-2.40-linux-glibc2.3.2-x86_64-py24.tar.bz2", 185),
        ("blender-2.40-linux-glibc2.3.2-x86_64-py23.tar.bz2", 185),
        ("blender-2.40alpha1-linux-glibc232-py24-i386.tar.bz2", 179),
        ("blender-2.40alpha1-linux-glibc232-py23-i386.tar.bz2", 179),
        ("blender-2.40alpha2-linux-glibc232-py24-i386.tar.bz2", 294),
        ("blender-2.40alpha2-linux-glibc232-py23-i386.tar.bz2", 150),
        ("blender-2.41-linux-glibc232-py24-i386.tar.bz2", 190),
        ("blender-2.41-linux-glibc232-py23-i386.tar.bz2", 190),
        ("blender-2.41-linux-glibc2.3.2-x86_64-py24.tar.gz", 204),
        ("blender-2.41-linux-glibc2.3.2-x86_64-py23.tar.gz", 204),
        ("blender-2.42-linux-glibc232-py24-i386.tar.bz2", 225),
        ("blender-2.42-linux-glibc232-py23-i386.tar.bz2", 225),
        ("blender-2.42a-linux-glibc232-py24-i386.tar.bz2", 225),
        ("blender-2.42a-linux-glibc232-py23-i386.tar.bz2", 225),
        ("blender-2.43-linux-glibc232-py24-i386.tar.bz2", 249),
        ("blender-2.44-linux-glibc236-py25-x86_64.tar.bz2", 258),
        ("blender-2.44-linux-glibc236-py24-x86_64.tar.bz2", 256),
        ("blender-2.44-linux-glibc232-py25-i386.tar.bz2", 262),
        ("blender-2.44-linux-glibc232-py24-i386.tar.bz2", 262),
        ("blender-2.45-linux-glibc236-py25-x86_64.tar.bz2", 253),
        ("blender-2.45-linux-glibc236-py25-i386.tar.bz2", 250),
        ("blender-2.45-linux-glibc236-py24-x86_64.tar.bz2", 253),
        ("blender-2.45-linux-glibc236-py24-i386.tar.bz2", 250),
        ("blender-2.46-linux-glibc236-py25-x86_64.tar.bz2", 273),
        ("blender-2.46-linux-glibc236-py25-i386.tar.bz2", 274),
        ("blender-2.46-linux-glibc236-py24-x86_64.tar.bz2", 273),
        ("blender-2.46-linux-glibc236-py24-i386.tar.bz2", 273),
        ("blender-2.47-linux-glibc236-py25-x86_64.tar.bz2", 275),
        ("blender-2.47-linux-glibc236-py25-i386.tar.bz2", 275),
        ("blender-2.47-linux-glibc236-py24-x86_64.tar.bz2", 275),
        ("blender-2.47-linux-glibc236-py24-i386.tar.bz2", 275),
        ("blender-2.48-linux-glibc236-py25-x86_64.tar.bz2", 304),
        ("blender-2.48-linux-glibc236-py25-i386.tar.bz2", 305),
        ("blender-2.48-linux-glibc236-py24-x86_64.tar.bz2", 304),
        ("blender-2.48-linux-glibc236-py24-i386.tar.bz2", 304),
        ("blender-2.48a-linux-glibc236-py25-x86_64.tar.bz2", 305),
        ("blender-2.48a-linux-glibc236-py25-i386.tar.bz2", 305),
        ("blender-2.48a-linux-glibc236-py24-x86_64.tar.bz2", 305),
        ("blender-2.48a-linux-glibc236-py24-i386.tar.bz2", 305),
        ("blender-2.49-linux-glibc236-py26-x86_64.tar.bz2", 324),
        ("blender-2.49-linux-glibc236-py26-i386.tar.bz2", 321),
        ("blender-2.49-linux-glibc236-py25-x86_64.tar.bz2", 321),
        ("blender-2.49-linux-glibc236-py25-i386.tar.bz2", 321),
        ("blender-2.49a-linux-glibc236-py26-x86_64.tar.bz2", 319),
        ("blender-2.49a-linux-glibc236-py26-i386.tar.bz2", 319),
        ("blender-2.49a-linux-glibc236-py25-x86_64.tar.bz2", 319),
        ("blender-2.49a-linux-glibc236-py25-i386.tar.bz2", 319),
        ("blender-2.49b-linux-glibc236-py26-x86_64.tar.bz2", 320),
        ("blender-2.49b-linux-glibc236-py26-i386.tar.bz2", 320),
        ("blender-2.49b-linux-glibc236-py25-x86_64.tar.bz2", 320),
        ("blender-2.49b-linux-glibc236-py25-i386.tar.bz2", 320),
        ("blender-2.5-alpha0-linux-glibc27-x86_64.tar.bz2", 802),
        ("blender-2.5-alpha0-linux-glibc27-i686.tar.bz2", 803),
        ("blender-2.5-alpha1-linux-glibc27-x86_64.tar.bz2", 787),
        ("blender-2.5-alpha1-linux-glibc27-i686.tar.bz2", 787),
        ("blender-2.5-alpha2-linux-glibc27-x86_64.tar.bz2", 794),
        ("blender-2.5-alpha2-linux-glibc27-i686.tar.bz2", 794),
        ("blender-2.53-beta-linux-glibc27-x86_64.tar.bz2", 1163),
        ("blender-2.53-beta-linux-glibc27-i686.tar.bz2", 1163),
        ("blender-2.54-beta-linux-glibc27-x86_64.tar.bz2", 1361),
        ("blender-2.54-beta-linux-glibc27-i686.tar.bz2", 1361),
        ("blender-2.55-beta-linux-glibc27-x86_64.tar.bz2", 829),
        ("blender-2.55-beta-linux-glibc27-i686.tar.bz2", 831),
        ("blender-2.56a-beta-linux-glibc27-x86_64.tar.bz2", 897),
        ("blender-2.56a-beta-linux-glibc27-i686.tar.bz2", 897),
        ("blender-2.56-beta-linux-glibc27-x86_64.tar.bz2", 1218),
        ("blender-2.56-beta-linux-glibc27-i686.tar.bz2", 895),
        ("blender-2.57-linux-glibc27-x86_64.tar.bz2", 930),
        ("blender-2.57-linux-glibc27-i686.tar.bz2", 930),
        ("blender-2.57a-linux-glibc27-x86_64.tar.bz2", 926),
        ("blender-2.57a-linux-glibc27-i686.tar.bz2", 926),
        ("blender-2.57b-linux-glibc27-x86_64.tar.bz2", 929),
        ("blender-2.57b-linux-glibc27-i686.tar.bz2", 929),
        ("blender-2.58-linux-glibc27-x86_64.tar.bz2", 938),
        ("blender-2.58-linux-glibc27-i686.tar.bz2", 938),
        ("blender-2.58a-linux-glibc27-x86_64.tar.bz2", 971),
        ("blender-2.58a-linux-glibc27-i686.tar.bz2", 971),
        ("blender-2.59-linux-glibc27-x86_64.tar.bz2", 946),
        ("blender-2.59-linux-glibc27-i686.tar.bz2", 946),
        ("blender-2.60-linux-glibc27-x86_64.tar.bz2", 1032),
        ("blender-2.60-linux-glibc27-i686.tar.bz2", 1032),
        ("blender-2.60a-linux-glibc27-x86_64.tar.bz2", 1032),
        ("blender-2.60a-linux-glibc27-i686.tar.bz2", 1032),
        ("blender-2.61-linux-glibc27-x86_64.tar.bz2", 1199),
        ("blender-2.61-linux-glibc27-i686.tar.bz2", 1199),
        ("blender-2.62-linux-glibc27-x86_64.tar.bz2", 1253),
        ("blender-2.62-linux-glibc27-i686.tar.bz2", 1253),
        ("blender-2.63-linux-glibc27-x86_64.tar.bz2", 1265),
        ("blender-2.63-linux-glibc27-i686.tar.bz2", 1319),
        ("blender-2.63a-linux-glibc27-x86_64.tar.bz2", 1269),
        ("blender-2.63a-linux-glibc27-i686.tar.bz2", 1323),
        ("blender-2.64-linux-glibc27-x86_64.tar.bz2", 1372),
        ("blender-2.64-linux-glibc27-i686.tar.bz2", 1426),
        ("blender-2.64a-linux-glibc27-x86_64.tar.bz2", 1372),
        ("blender-2.64a-linux-glibc27-i686.tar.bz2", 1426),
        ("blender-2.65-linux-glibc211-x86_64.tar.bz2", 1456),
        ("blender-2.65-linux-glibc211-i686.tar.bz2", 1456),
        ("blender-2.65-linux-glibc27-x86_64.tar.bz2", 1458),
        ("blender-2.65-linux-glibc27-i686.tar.bz2", 1458),
        ("blender-2.65a-linux-glibc211-x86_64.tar.bz2", 1456),
        ("blender-2.65a-linux-glibc211-i686.tar.bz2", 1456),
        ("blender-2.65a-linux-glibc27-x86_64.tar.bz2", 1458),
        ("blender-2.65a-linux-glibc27-i686.tar.bz2", 1458),
        ("blender-2.66-linux-glibc211-x86_64.tar.bz2", 1496),
        ("blender-2.66-linux-glibc211-i686.tar.bz2", 1496),
        ("blender-2.66a-linux-glibc211-x86_64.tar.bz2", 1497),
        ("blender-2.66a-linux-glibc211-i686.tar.bz2", 1497),
        ("blender-2.67-linux-glibc211-x86_64.tar.bz2", 1845),
        ("blender-2.67-linux-glibc211-i686.tar.bz2", 1845),
        ("blender-2.67a-linux-glibc211-x86_64.tar.bz2", 2172),
        ("blender-2.67a-linux-glibc211-i686.tar.bz2", 2172),
        ("blender-2.67b-linux-glibc211-x86_64.tar.bz2", 1845),
        ("blender-2.67b-linux-glibc211-i686.tar.bz2", 1845),
        ("blender-2.68-linux-glibc211-x86_64.tar.bz2", 1858),
        ("blender-2.68-linux-glibc211-i686.tar.bz2", 1858),
        ("blender-2.68a-linux-glibc211-x86_64.tar.bz2", 1858),
        ("blender-2.68a-linux-glibc211-i686.tar.bz2", 1858),
        ("blender-2.69-linux-glibc211-x86_64.tar.bz2", 1949),
        ("blender-2.69-linux-glibc211-i686.tar.bz2", 1949),
        ("blender-2.70-linux-glibc211-x86_64.tar.bz2", 2184),
        ("blender-2.70-linux-glibc211-i686.tar.bz2", 2184),
        ("blender-2.70a-linux-glibc211-x86_64.tar.bz2", 2184),
        ("blender-2.70a-linux-glibc211-i686.tar.bz2", 2184),
        ("blender-2.71-linux-glibc211-x86_64.tar.bz2", 2242),
        ("blender-2.71-linux-glibc211-i686.tar.bz2", 2241),
        ("blender-2.72-linux-glibc211-x86_64.tar.bz2", 2328),
        ("blender-2.72-linux-glibc211-i686.tar.bz2", 2327),
        ("blender-2.72a-linux-glibc211-x86_64.tar.bz2", 2328),
        ("blender-2.72a-linux-glibc211-i686.tar.bz2", 2327),
        ("blender-2.72b-linux-glibc211-i686.tar.bz2", 2327),
        ("blender-2.72b-linux-glibc211-x86_64.tar.bz2", 2328),
        ("blender-2.73-linux-glibc211-x86_64.tar.bz2", 2178),
        ("blender-2.73-linux-glibc211-i686.tar.bz2", 2178),
        ("blender-2.73a-linux-glibc211-x86_64.tar.bz2", 2178),
        ("blender-2.73a-linux-glibc211-i686.tar.bz2", 2178),
        ("blender-2.74-linux-glibc211-x86_64.tar.bz2", 2189),
        ("blender-2.74-linux-glibc211-i686.tar.bz2", 2189),
        ("blender-2.75-linux-glibc211-x86_64.tar.bz2", 2270),
        ("blender-2.75-linux-glibc211-i686.tar.bz2", 2270),
        ("blender-2.75a-linux-glibc211-x86_64.tar.bz2", 2270),
        ("blender-2.75a-linux-glibc211-i686.tar.bz2", 2270),
        ("blender-2.76-linux-glibc211-x86_64.tar.bz2", 2260),
        ("blender-2.76-linux-glibc211-i686.tar.bz2", 2260),
        ("blender-2.76a-linux-glibc211-i686.tar.bz2", 2260),
        ("blender-2.76a-linux-glibc211-x86_64.tar.bz2", 2260),
        ("blender-2.76b-linux-glibc211-x86_64.tar.bz2", 2260),
        ("blender-2.76b-linux-glibc211-i686.tar.bz2", 2260),
        ("blender-2.77-linux-glibc211-x86_64.tar.bz2", 2183),
        ("blender-2.77-linux-glibc211-i686.tar.bz2", 2183),
        ("blender-2.77a-linux-glibc211-i686.tar.bz2", 2184),
        ("blender-2.77a-linux-glibc211-x86_64.tar.bz2", 2184),
        ("blender-2.78-linux-glibc219-x86_64.tar.bz2", 2503),
        ("blender-2.78-linux-glibc219-i686.tar.bz2", 2503),
        ("blender-2.78-linux-glibc211-x86_64.tar.bz2", 2503),
        ("blender-2.78-linux-glibc211-i686.tar.bz2", 2503),
        ("blender-2.78a-linux-glibc219-x86_64.tar.bz2", 2504),
        ("blender-2.78a-linux-glibc219-i686.tar.bz2", 2504),
        ("blender-2.78a-linux-glibc211-x86_64.tar.bz2", 2504),
        ("blender-2.78a-linux-glibc211-i686.tar.bz2", 2504),
        ("blender-2.78b-linux-glibc219-x86_64.tar.bz2", 2521),
        ("blender-2.78b-linux-glibc219-i686.tar.bz2", 2520),
        ("blender-2.78c-linux-glibc219-x86_64.tar.bz2", 2509),
        ("blender-2.78c-linux-glibc219-i686.tar.bz2", 2508),
        ("blender-2.79-linux-glibc219-x86_64.tar.bz2", 3081),
        ("blender-2.79-linux-glibc219-i686.tar.bz2", 3081),
        ("blender-2.79a-linux-glibc219-x86_64.tar.bz2", 3081),
        ("blender-2.79a-linux-glibc219-i686.tar.bz2", 3081),
        ("blender-2.79b-linux-glibc219-x86_64.tar.bz2", 3081),
        ("blender-2.79b-linux-glibc219-i686.tar.bz2", 3081),
        (
            "blender-2.79-e045fe53f1b0-linux-glibc217-x86_64.tar.bz2",
            3070
        ),
        (
            "blender-2.79-e045fe53f1b0-linux-glibc224-i686.tar.bz2",
            3053
        ),
        ("blender-2.80-linux-glibc224-i686.tar.bz2", 3209),
        ("blender-2.80-linux-glibc217-x86_64.tar.bz2", 3227),
        ("blender-2.80rc1-linux-glibc217-x86_64.tar.bz2", 3233),
        ("blender-2.80rc1-linux-glibc224-i686.tar.bz2", 3215),
        ("blender-2.80rc2-linux-glibc217-x86_64.tar.bz2", 3233),
        ("blender-2.80rc2-linux-glibc224-i686.tar.bz2", 3215),
        ("blender-2.80rc3-linux-glibc217-x86_64.tar.bz2", 3227),
        ("blender-2.80rc3-linux-glibc224-i686.tar.bz2", 3209),
        ("blender-2.81-linux-glibc217-x86_64.tar.bz2", 4410),
        ("blender-2.81a-linux-glibc217-x86_64.tar.bz2", 3445),
        ("blender-2.82-linux64.tar.xz", 4642),
        ("blender-2.82a-linux64.tar.xz", 4642),
        ("blender-2.83.0-linux64.tar.xz", 4672),
        ("blender-2.83.1-linux64.tar.xz", 4672),
        ("blender-2.83.2-linux64.tar.xz", 4672),
        ("blender-2.83.3-linux64.tar.xz", 4672),
        ("blender-2.83.4-linux64.tar.xz", 4672),
        ("blender-2.83.5-linux64.tar.xz", 4672),
        ("blender-2.83.6-linux64.tar.xz", 4672),
        ("blender-2.83.7-linux64.tar.xz", 4672),
        ("blender-2.83.8-linux64.tar.xz", 4672),
        ("blender-2.83.9-linux64.tar.xz", 4672),
        ("blender-2.90.0-linux64.tar.xz", 4649),
        ("blender-2.90.1-linux64.tar.xz", 4649),
        ("blender-2.91.0-linux64.tar.xz", 4644),
    ]
    .iter()
    .copied()
    .collect();
}
