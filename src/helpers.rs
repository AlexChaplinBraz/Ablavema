//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables, unused_macros)]
use crate::{releases::*, settings::*};
use clap::ArgMatches;
use indicatif::MultiProgress;
use std::{error::Error, path::Path, str::FromStr};

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
