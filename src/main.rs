//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod installed;
mod releases;
mod settings;
use crate::installed::*;
use crate::releases::*;
use crate::settings::*;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgGroup,
    SubCommand,
};
use indicatif::MultiProgress;
use prettytable::{cell, format, row, Table};
use std::process::Command;
use std::str::FromStr;
use std::{error::Error, process::exit};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}.", e);
        exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let (mut settings, config_path) = Settings::new()?;

    let mut releases = Releases::new();
    releases.load(&settings);

    let mut installed = Installed::new(&settings)?;

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
            Arg::with_name("config")
                .global(true)
                .short("c")
                .long("config")
                .value_name("PATH")
                .help("Use a different configuration file")
                .long_help("Use a different configuration file. Must be TOML formatted. Name and extension don't matter.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("path")
                .value_name("PATH")
                .help("Path to .blend file"),
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
        ("fetch", Some(a)) => {
            if a.is_present("all") {
                releases.fetch_official_releases(&settings).await;
                releases.fetch_lts_releases(&settings).await;
                releases.fetch_experimental_branches(&settings).await;
                releases.fetch_latest_daily(&settings).await;
                releases.fetch_latest_stable(&settings).await;
            } else {
                if a.is_present("daily") {
                    releases.fetch_latest_daily(&settings).await;
                }

                if a.is_present("experimental") {
                    releases.fetch_experimental_branches(&settings).await;
                }

                if a.is_present("lts") {
                    releases.fetch_lts_releases(&settings).await;
                }

                if a.is_present("official") {
                    releases.fetch_official_releases(&settings).await;
                }

                if a.is_present("stable") {
                    releases.fetch_latest_stable(&settings).await;
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                            .install(&settings, &multi_progress)
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
                        .remove(&settings)
                        .await?;
                }
                installed.check(&settings)?;
            } else {
                for build in a.values_of("id").unwrap() {
                    installed
                        .iter()
                        .enumerate()
                        .find(|(i, _)| *i == usize::from_str(build).unwrap())
                        .unwrap()
                        .1
                        .remove(&settings)
                        .await?;
                }
                installed.check(&settings)?;
            }
        }
        ("select", Some(a)) => {
            settings.default_package = {
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
            settings.save(&config_path)?;
            println!("Selected: {}", settings.default_package);
        }
        ("update", Some(_a)) => {
            installed
                .update(&mut settings, &config_path, &mut releases)
                .await?
        }
        _ => {
            if args.is_present("path") {
                let _blender = Command::new({
                    if cfg!(target_os = "linux") {
                        format!(
                            "{}/{}/blender",
                            settings.packages_dir.to_str().unwrap(),
                            settings.default_package
                        )
                    } else if cfg!(target_os = "windows") {
                        todo!("windows command");
                    } else if cfg!(target_os = "macos") {
                        todo!("macos command");
                    } else {
                        unreachable!("Unsupported OS command");
                    }
                })
                .arg(args.value_of("path").unwrap())
                .status()?;
            } else {
                todo!("Launch interface");
            }
        }
    }

    Ok(())
}
