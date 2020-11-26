//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{gui::*, helpers::*, installed::*, releases::*, settings::*};
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgGroup,
    SubCommand,
};
use indicatif::MultiProgress;
use prettytable::{cell, format, row, Table};
use std::{error::Error, str::FromStr, sync::atomic::Ordering};

pub async fn run_cli() -> Result<GuiArgs, Box<dyn Error>> {
    let mut releases = Releases::new();
    releases.load()?;

    let mut installed = Installed::new()?;

    // Workaround for 'clap' not supporting colours on Windows,
    // even though 'indicatif' does display colours on Windows.
    // It's also a workaround for showing the current values of SETTINGS
    // without it being named and parsed as "default".
    let left_ansi_code;
    let right_ansi_code;
    if cfg!(target_os = "linux") {
        left_ansi_code = "\x1b[32m";
        right_ansi_code = "\x1b[0m";
    } else if cfg!(target_os = "windows") {
        left_ansi_code = "";
        right_ansi_code = "";
    } else if cfg!(target_os = "macos") {
        todo!("macos ansi codes");
    } else {
        unreachable!("Unsupported OS");
    }

    let help_default_package = format!(
        "Select default package to use for opening .blend files [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().default_package,
        right_ansi_code
    );
    let help_use_latest_as_default = format!(
        "Change to the latest package of the same build type when updating [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().use_latest_as_default,
        right_ansi_code
    );
    let help_check_updates_at_launch = format!(
        "Check for updates at launch [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().check_updates_at_launch,
        right_ansi_code
    );
    let help_minutes_between_updates = format!(
        "Amount of minutes to wait between update checks [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().minutes_between_updates,
        right_ansi_code
    );
    let help_update_daily = format!(
        "Download the latest daily package [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().update_daily,
        right_ansi_code
    );
    let help_update_experimental = format!(
        "Download the latest experimental package [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().update_experimental,
        right_ansi_code
    );
    let help_update_stable = format!(
        "Download the latest stable package [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().update_stable,
        right_ansi_code
    );
    let help_update_lts = format!(
        "Download the latest LTS package [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().update_lts,
        right_ansi_code
    );
    let help_keep_only_latest_daily = format!(
        "Remove all daily packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().keep_only_latest_daily,
        right_ansi_code
    );
    let help_keep_only_latest_experimental = format!(
        "Remove all experimental packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().keep_only_latest_experimental,
        right_ansi_code
    );
    let help_keep_only_latest_stable = format!(
        "Remove all stable packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().keep_only_latest_stable,
        right_ansi_code
    );
    let help_keep_only_latest_lts = format!(
        "Remove all LTS packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        SETTINGS.read().unwrap().keep_only_latest_lts,
        right_ansi_code
    );

    let args = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::DisableHelpSubcommand)
        .global_setting(AppSettings::InferSubcommands)
        .global_setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::ArgsNegateSubcommands)
        .help_message("Print help and exit")
        .version_message("Print version and exit")
        .version_short("v")
        .arg(
            Arg::with_name("path")
                .value_name("PATH")
                .help("Path to .blend file"),
        )
        .subcommand(
            SubCommand::with_name("config")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::NextLineHelp)
                .about("Set configuration settings")
                .long_about("Set configuration settings. It's possible to make the program portable by creating an empty file named 'portable' in the same directory as the executable, which will make it store everything together.")
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("use_latest_as_default")
                        .display_order(20)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("u")
                        .long("use-latest-as-default")
                        .help(&help_use_latest_as_default),
                )
                .arg(
                    Arg::with_name("check_updates_at_launch")
                        .display_order(23)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("c")
                        .long("check-updates-at-launch")
                        .help(&help_check_updates_at_launch),
                )
                .arg(
                    Arg::with_name("minutes_between_updates")
                        .display_order(27)
                        .takes_value(true)
                        .value_name("INT")
                        .short("m")
                        .long("minutes-between-updates")
                        .help(&help_minutes_between_updates),
                )
                .arg(
                    Arg::with_name("update_daily")
                        .display_order(30)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("d")
                        .long("update-daily")
                        .help(&help_update_daily),
                )
                .arg(
                    Arg::with_name("update_experimental")
                        .display_order(40)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("e")
                        .long("update-experimental")
                        .help(&help_update_experimental),
                )
                .arg(
                    Arg::with_name("update_stable")
                        .display_order(50)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("s")
                        .long("update-stable")
                        .help(&help_update_stable),
                )
                .arg(
                    Arg::with_name("update_lts")
                        .display_order(60)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("l")
                        .long("update-lts")
                        .help(&help_update_lts),
                )
                .arg(
                    Arg::with_name("keep_only_latest_daily")
                        .display_order(70)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("D")
                        .long("keep_only_latest_daily")
                        .help(&help_keep_only_latest_daily),
                )
                .arg(
                    Arg::with_name("keep_only_latest_experimental")
                        .display_order(80)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("E")
                        .long("keep_only_latest_experimental")
                        .help(&help_keep_only_latest_experimental),
                )
                .arg(
                    Arg::with_name("keep_only_latest_stable")
                        .display_order(90)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("S")
                        .long("keep_only_latest_stable")
                        .help(&help_keep_only_latest_stable),
                )
                .arg(
                    Arg::with_name("keep_only_latest_lts")
                        .display_order(100)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("L")
                        .long("keep_only_latest_lts")
                        .help(&help_keep_only_latest_lts),
                )
                .group(
                    ArgGroup::with_name("config_group")
                        .args(&[
                            "use_latest_as_default",
                            "check_updates_at_launch",
                            "minutes_between_updates",
                            "update_daily",
                            "update_experimental",
                            "update_stable",
                            "update_lts",
                            "keep_only_latest_daily",
                            "keep_only_latest_experimental",
                            "keep_only_latest_stable",
                            "keep_only_latest_lts"
                        ])
                        .required(true)
                        .multiple(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("fetch")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("Fetch new packages")
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Fetch all packages"),
                )
                .arg(
                    Arg::with_name("daily")
                        .short("d")
                        .long("daily")
                        .help("Fetch daily packages"),
                )
                .arg(
                    Arg::with_name("experimental")
                        .short("e")
                        .long("experimental")
                        .help("Fetch experimental packages"),
                )
                .arg(
                    Arg::with_name("lts")
                        .short("l")
                        .long("lts")
                        .help("Fetch LTS packages"),
                )
                .arg(
                    Arg::with_name("official")
                        .short("o")
                        .long("official")
                        .help("Fetch official packages"),
                )
                .arg(
                    Arg::with_name("stable")
                        .short("s")
                        .long("stable")
                        .help("Fetch stable packages"),
                )
                .group(
                    ArgGroup::with_name("fetch_group")
                        .args(&["all", "daily", "experimental", "lts", "official", "stable"])
                        .required(true)
                        .multiple(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .about("Install packages")
                .help_message("Print help and exit")
                .subcommand(
                    SubCommand::with_name("daily")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install daily packages")
                        .help_message("Print help and exit")
                        .arg(
                            Arg::with_name("id")
                                .value_name("ID")
                                .required(true)
                                .multiple(true)
                                .help("A list of packages to install"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the package instead of the ID")
                                .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("experimental")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install experimental packages")
                        .help_message("Print help and exit")
                        .arg(
                            Arg::with_name("id")
                                .value_name("ID")
                                .required(true)
                                .multiple(true)
                                .help("A list of packages to install"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the package instead of the ID")
                                .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("lts")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install LTS packages")
                        .help_message("Print help and exit")
                        .arg(
                            Arg::with_name("id")
                                .value_name("ID")
                                .required(true)
                                .multiple(true)
                                .help("A list of packages to install"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the package instead of the ID")
                                .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("official")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install official packages")
                        .help_message("Print help and exit")
                        .arg(
                            Arg::with_name("id")
                                .value_name("ID")
                                .required(true)
                                .multiple(true)
                                .help("A list of packages to install"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the package instead of the ID")
                                .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("stable")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install stable packages")
                        .help_message("Print help and exit")
                        .arg(
                            Arg::with_name("id")
                                .value_name("ID")
                                .required(true)
                                .multiple(true)
                                .help("A list of packages to install"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the package instead of the ID")
                                .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .about("List packages")
                .help_message("Print help and exit")
                .subcommand(
                    SubCommand::with_name("daily")
                        .about("List daily packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("experimental")
                        .about("List experimental packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("installed")
                        .about("List installed packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("lts")
                        .about("List lts packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("official")
                        .about("List official packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("stable")
                        .about("List stable packages")
                        .help_message("Print help and exit"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("Remove packages")
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("id")
                        .value_name("ID")
                        .required(true)
                        .multiple(true)
                        .help("A list of packages to remove"),
                )
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("Use the name of the package instead of the ID")
                        .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                ),
        )
        .subcommand(
            SubCommand::with_name("select")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("Select default package")
                .long_about(&*help_default_package)
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("id")
                        .value_name("ID")
                        .required(true)
                        .help("Default package to use for opening .blend files"),
                )
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("Use the name of the package instead of the ID")
                        .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                ),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Update installed packages")
                .help_message("Print help and exit"),
        )
        .get_matches();

    match args.subcommand() {
        ("config", Some(a)) => {
            process_bool_arg(&a, "use_latest_as_default")?;
            process_bool_arg(&a, "check_updates_at_launch")?;

            if a.is_present("minutes_between_updates") {
                let new_arg =
                    u64::from_str(a.value_of("minutes_between_updates").unwrap()).unwrap();
                let old_arg = SETTINGS.read().unwrap().minutes_between_updates;

                if new_arg == old_arg {
                    println!(
                        "'{}' is unchanged from '{}'.",
                        "minutes_between_updates", old_arg
                    );
                } else {
                    SETTINGS.write().unwrap().minutes_between_updates = new_arg;

                    println!(
                        "'{}' changed from '{}' to '{}'.",
                        "minutes_between_updates", old_arg, new_arg
                    );
                }
            }

            process_bool_arg(&a, "update_daily")?;
            process_bool_arg(&a, "update_experimental")?;
            process_bool_arg(&a, "update_stable")?;
            process_bool_arg(&a, "update_lts")?;
            process_bool_arg(&a, "keep_only_latest_daily")?;
            process_bool_arg(&a, "keep_only_latest_experimental")?;
            process_bool_arg(&a, "keep_only_latest_stable")?;
            process_bool_arg(&a, "keep_only_latest_lts")?;

            SETTINGS.read().unwrap().save();
        }
        ("fetch", Some(a)) => {
            if a.is_present("all") {
                releases.fetch_official_releases().await?;
                releases.fetch_lts_releases().await?;
                releases.fetch_experimental_branches().await?;
                releases.fetch_latest_daily().await?;
                releases.fetch_latest_stable().await?;
            } else {
                if a.is_present("daily") {
                    releases.fetch_latest_daily().await?;
                }

                if a.is_present("experimental") {
                    releases.fetch_experimental_branches().await?;
                }

                if a.is_present("lts") {
                    releases.fetch_lts_releases().await?;
                }

                if a.is_present("official") {
                    releases.fetch_official_releases().await?;
                }

                if a.is_present("stable") {
                    releases.fetch_latest_stable().await?;
                }
            }
        }
        ("install", Some(a)) => match a.subcommand() {
            ("daily", Some(b)) => {
                if b.is_present("name") {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .latest_daily
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                } else {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .latest_daily
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                }
            }
            ("experimental", Some(b)) => {
                if b.is_present("name") {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .experimental_branches
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                } else {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .experimental_branches
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                }
            }
            ("lts", Some(b)) => {
                if b.is_present("name") {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .lts_releases
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                } else {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .lts_releases
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                }
            }
            ("official", Some(b)) => {
                if b.is_present("name") {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .official_releases
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                } else {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .official_releases
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                }
            }
            ("stable", Some(b)) => {
                if b.is_present("name") {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .latest_stable
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                } else {
                    let multi_progress = MultiProgress::new();
                    for build in b.values_of("id").unwrap() {
                        releases
                            .latest_stable
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .install(&multi_progress)
                            .await?;
                    }
                    multi_progress.join().unwrap();
                }
            }
            _ => unreachable!("Install subcommand"),
        },
        ("list", Some(a)) => match a.subcommand() {
            ("daily", Some(_b)) => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in releases.latest_daily.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No daily packages found. Try fetching first.");
                } else {
                    table.printstd();
                }
            }
            ("experimental", Some(_b)) => {
                // FIX: This table can be around 160 characters wide, which breaks formatting
                // on narrow terminals. Could be solved by checking terminal width and truncating
                // the package name since it holds repeated information. But even the other tables
                // have a chance of looking weird depending on how small their terminal window is.
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in releases.experimental_branches.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No experimental packages found. Try fetching first.");
                } else {
                    table.printstd();
                }
            }
            ("installed", Some(_b)) => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in installed.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No installed packages found. Try installing first.");
                } else {
                    table.printstd();
                }
            }
            ("lts", Some(_b)) => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in releases.lts_releases.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No LTS packages found. Try fetching first.");
                } else {
                    table.printstd();
                }
            }
            ("official", Some(_b)) => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in releases.official_releases.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No official packages found. Try fetching first.");
                } else {
                    table.printstd();
                }
            }
            ("stable", Some(_b)) => {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(row!["ID", "Package", "Version", "Build", "Date"]);
                for (i, p) in releases.latest_stable.iter().enumerate() {
                    table.add_row(row![i, p.name, p.version, p.build, p.date]);
                }
                if table.is_empty() {
                    eprintln!("No stable packages found. Try fetching first.");
                } else {
                    table.printstd();
                }
            }
            _ => unreachable!("List subcommand"),
        },
        ("remove", Some(a)) => {
            if a.is_present("name") {
                for build in a.values_of("id").unwrap() {
                    installed
                        .iter()
                        .find(|p| p.name == build)
                        .unwrap()
                        .remove()
                        .await?;
                }
            } else {
                for build in a.values_of("id").unwrap() {
                    installed
                        .iter()
                        .enumerate()
                        .find(|(i, _)| *i == usize::from_str(build).unwrap())
                        .unwrap()
                        .1
                        .remove()
                        .await?;
                }
            }

            if !SETTINGS.read().unwrap().default_package.is_empty() {
                installed.check()?;

                let old_default = SETTINGS.read().unwrap().default_package.clone();

                if installed.iter().find(|p| p.name == old_default).is_none() {
                    SETTINGS.write().unwrap().default_package = String::new();
                    SETTINGS.read().unwrap().save();

                    println!(
                        "Default package '{}' was removed. Please select a new package.",
                        old_default
                    );
                }
            }
        }
        ("select", Some(a)) => {
            SETTINGS.write().unwrap().default_package = {
                if a.is_present("name") {
                    installed
                        .iter()
                        .find(|p| p.name == a.value_of("id").unwrap())
                        .unwrap()
                        .name
                        .to_string()
                } else {
                    installed
                        .iter()
                        .enumerate()
                        .find(|(i, _)| *i == usize::from_str(a.value_of("id").unwrap()).unwrap())
                        .unwrap()
                        .1
                        .name
                        .to_string()
                }
            };
            SETTINGS.read().unwrap().save();
            println!("Selected: {}", SETTINGS.read().unwrap().default_package);
        }
        ("update", Some(_a)) => installed.update(&mut releases).await?,
        _ => {
            LAUNCH_GUI.store(true, Ordering::Relaxed);
        }
    }

    Ok(GuiArgs {
        releases,
        installed,
        file_path: {
            if args.is_present("path") {
                String::from(args.value_of("path").unwrap())
            } else {
                String::new()
            }
        },
    })
}
