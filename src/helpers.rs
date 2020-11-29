//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables, unused_macros)]
use crate::{releases::*, settings::*};
use clap::ArgMatches;
use indicatif::MultiProgress;
use lazy_static::lazy_static;
use prettytable::{
    cell,
    format::{self, FormatBuilder},
    row, table, Table,
};
use std::{collections::HashMap, error::Error, path::Path, str::FromStr};

pub fn process_bool_arg(arg: &ArgMatches, name: &str) -> Result<(), Box<dyn Error>> {
    if arg.is_present(name) {
        let new_arg = expand_bool(arg.value_of(name).unwrap());
        let old_arg = read_bool_setting(name);
        if new_arg == old_arg {
            println!("'{}' is unchanged from '{}'.", name, old_arg);
        } else {
            write_bool_setting(name, new_arg);
            println!("'{}' changed from '{}' to '{}'.", name, old_arg, new_arg);
        }
    }

    Ok(())
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
        "use_latest_as_default" => SETTINGS.read().unwrap().use_latest_as_default,
        "check_updates_at_launch" => SETTINGS.read().unwrap().check_updates_at_launch,
        "update_daily" => SETTINGS.read().unwrap().update_daily,
        "update_experimental" => SETTINGS.read().unwrap().update_experimental,
        "update_stable" => SETTINGS.read().unwrap().update_stable,
        "update_lts" => SETTINGS.read().unwrap().update_lts,
        "keep_only_latest_daily" => SETTINGS.read().unwrap().keep_only_latest_daily,
        "keep_only_latest_experimental" => SETTINGS.read().unwrap().keep_only_latest_experimental,
        "keep_only_latest_stable" => SETTINGS.read().unwrap().keep_only_latest_stable,
        "keep_only_latest_lts" => SETTINGS.read().unwrap().keep_only_latest_lts,
        _ => panic!("Unknown bool field"),
    }
}

fn write_bool_setting(name: &str, value: bool) {
    match name {
        "use_latest_as_default" => SETTINGS.write().unwrap().use_latest_as_default = value,
        "check_updates_at_launch" => SETTINGS.write().unwrap().check_updates_at_launch = value,
        "update_daily" => SETTINGS.write().unwrap().update_daily = value,
        "update_experimental" => SETTINGS.write().unwrap().update_experimental = value,
        "update_stable" => SETTINGS.write().unwrap().update_stable = value,
        "update_lts" => SETTINGS.write().unwrap().update_lts = value,
        "keep_only_latest_daily" => SETTINGS.write().unwrap().keep_only_latest_daily = value,
        "keep_only_latest_experimental" => {
            SETTINGS.write().unwrap().keep_only_latest_experimental = value
        }
        "keep_only_latest_stable" => SETTINGS.write().unwrap().keep_only_latest_stable = value,
        "keep_only_latest_lts" => SETTINGS.write().unwrap().keep_only_latest_lts = value,
        _ => panic!("Unknown bool field"),
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
    if SETTINGS
        .read()
        .unwrap()
        .last_update_time
        .elapsed()
        .unwrap()
        .as_secs()
        .checked_div(60)
        .unwrap()
        >= SETTINGS.read().unwrap().minutes_between_updates
    {
        true
    } else {
        false
    }
}

pub async fn cli_install(
    args: &ArgMatches<'_>,
    packages: &Vec<Package>,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    let multi_progress = MultiProgress::new();
    let flags = (args.is_present("reinstall"), args.is_present("redownload"));
    let mut values = Vec::new();

    for build in args.values_of("id").unwrap() {
        if values.contains(&build.to_string()) {
            continue;
        }
        values.push(build.to_string());

        if args.is_present("name") {
            match packages.iter().find(|p| p.name == build) {
                Some(a) => a.install(&multi_progress, &flags).await?,
                None => {
                    println!("No {} package named '{}' found.", name, build);
                    continue;
                }
            }
        } else {
            let build = usize::from_str(build)?;

            match packages.iter().enumerate().find(|(i, _)| *i == build) {
                Some(a) => a.1.install(&multi_progress, &flags).await?,
                None => {
                    println!("No {} package with ID '{}' found.", name, build);
                    continue;
                }
            }
        };
    }

    multi_progress.join().unwrap();

    Ok(())
}

pub fn cli_list_narrow(packages: &Vec<Package>, name: &str, invert: bool) {
    let mut table = Table::new();
    table.set_titles(row!["ID", "Package"]);

    for (i, p) in packages.iter().enumerate() {
        // This is a workaround for the issue of prettytable having a weird behaviour when a cell
        // has hspan > 1, affecting the other cells and making them uneven based on the content
        // length of the cell with hspan > 1.
        let details = format!("{} | {} | {}", p.date, p.version, p.build);
        let mut package = table!([p.name], [details]);

        let inner_format = FormatBuilder::new().padding(0, 0).build();
        package.set_format(inner_format);

        table.add_row(row![i, package]);
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

    for (i, p) in packages.iter().enumerate() {
        table.add_row(row![i, p.name, p.version, p.build, p.date]);
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

lazy_static! {
    static ref EXTRACTED_NAMES: HashMap<&'static str, &'static str> = [
        (
            "blender-2.27.NewPy1-linux-glibc2.3.2-i386-official",
            "blender-2.27-linux-glibc2.3.2-i386"
        ),
        (
            // Seems like a wrongly packaged version.
            // This actually conflicts if the 2.35b archive is being installed
            // at the same time as the actual 2.35a archive.
            // There's no error, but the files end up distributed between
            // the two directories, breaking both packages.
            // TODO: I don't even.
            "blender-2.35b-linux-glibc2.2.5-i386-official",
            "blender-2.35a-linux-glibc2.2.5-i386"
        ),
        ("blender-2.5-alpha1-linux-glibc27-x86_64-official", "blender-2.50-alpha1-linux-glibc27-x86_64"),
        ("blender-2.5-alpha1-linux-glibc27-i686-official", "blender-2.50-alpha1-linux-glibc27-i686"),
        (
            "blender-2.27.NewPy1-windows-official",
            "blender-2.27-windows"
        ),
        ("blender-2.47-windows-law-official", "blender-2.47-windows"),
        ("blender-2.48-windows64-official", "Blender248"),
        ("blender-2.48a-windows64-official", "Blender248a"),
        ("blender-2.5-alpha1-win64-official", "blender25-win64-26982"),
        ("blender-2.5-alpha2-win64-official", "Release"),
        (
            "blender-2.79-e045fe53f1b0-win64-official",
            "blender-2.79.0-git.e045fe53f1b0-windows64"
        ),
        (
            "blender-2.79-e045fe53f1b0-win32-official",
            "blender-2.79.0-git.e045fe53f1b0-windows32"
        ),
    ]
    .iter()
    .copied()
    .collect();
}

/// Handles cases where the extracted directory isn't named the same
/// as the downloaded archive from which the name of the package is taken.
pub fn get_extracted_name(package: &Package) -> &str {
    match EXTRACTED_NAMES.get(&package.name.as_ref()) {
        Some(s) => *s,
        None => {
            if package.build == Build::Official {
                package.name.trim_end_matches("-official")
            } else {
                &package.name
            }
        }
    }
}
