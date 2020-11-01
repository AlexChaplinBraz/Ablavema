//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod releases;
mod settings;
pub use crate::releases::Releases;
pub use crate::settings::Settings;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand,
};
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
    let settings = Settings::new()?;

    let mut releases = Releases::new();

    releases.load(&settings);

    let args = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("config")
                .global(true)
                .short("c")
                .long("config")
                .value_name("PATH")
                .help("Set a custom config directory")
                .takes_value(true)
                .default_value({
                    // TODO: Consider EnvVars.
                    if cfg!(target_os = "linux") {
                        "~/.config/BlenderLauncher/config.toml"
                    } else if cfg!(target_os = "windows") {
                        todo!("Decide Windows path");
                    } else if cfg!(target_os = "macos") {
                        todo!("Decide MacOs path");
                    } else {
                        unreachable!("Unsupported OS");
                    }
                }),
        )
        .subcommand(
            SubCommand::with_name("install")
                .setting(AppSettings::ColoredHelp)
                .version(crate_version!())
                .author(crate_authors!())
                .about("Install packages")
                .subcommand(
                    SubCommand::with_name("daily")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("Install daily packages")
                        .arg(
                            Arg::with_name("build")
                                .value_name("BUILD")
                                .required(true)
                                .multiple(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the build instead of the ID"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("exper")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("Install experimental packages")
                        .arg(
                            Arg::with_name("build")
                                .value_name("BUILD")
                                .required(true)
                                .multiple(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the build instead of the ID"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("stable")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("Install stable packages")
                        .arg(
                            Arg::with_name("build")
                                .value_name("BUILD")
                                .required(true)
                                .multiple(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the build instead of the ID"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("lts")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("Install lts packages")
                        .arg(
                            Arg::with_name("build")
                                .value_name("BUILD")
                                .required(true)
                                .multiple(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the build instead of the ID"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("official")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("Install official packages")
                        .arg(
                            Arg::with_name("build")
                                .value_name("BUILD")
                                .required(true)
                                .multiple(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .short("n")
                                .long("name")
                                .help("Use the name of the build instead of the ID"),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("fetch")
                .setting(AppSettings::ColoredHelp)
                .version(crate_version!())
                .author(crate_authors!())
                .about("Fetch new packages")
                .arg(
                    Arg::with_name("lts")
                        .short("l")
                        .long("lts")
                        .help("Fetch LTS releases")
                        .required_unless_one(&["stable", "daily", "exper", "official", "all"]),
                )
                .arg(
                    Arg::with_name("stable")
                        .short("s")
                        .long("stable")
                        .help("Fetch stable releases")
                        .required_unless_one(&["lts", "daily", "exper", "official", "all"]),
                )
                .arg(
                    Arg::with_name("daily")
                        .short("d")
                        .long("daily")
                        .help("Fetch daily releases")
                        .required_unless_one(&["stable", "lts", "exper", "official", "all"]),
                )
                .arg(
                    Arg::with_name("exper")
                        .short("e")
                        .long("exper")
                        .help("Fetch experimental releases")
                        .required_unless_one(&["stable", "daily", "lts", "official", "all"]),
                )
                .arg(
                    Arg::with_name("official")
                        .short("o")
                        .long("official")
                        .help("Fetch official releases")
                        .required_unless_one(&["stable", "daily", "exper", "lts", "all"]),
                )
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Fetch all releases")
                        .conflicts_with_all(&["stable", "daily", "exper", "lts", "official"])
                        .required_unless_one(&["stable", "daily", "exper", "lts", "official"]),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::ColoredHelp)
                .version(crate_version!())
                .author(crate_authors!())
                .about("List packages")
                .subcommand(
                    SubCommand::with_name("daily")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("List daily packages"),
                )
                .subcommand(
                    SubCommand::with_name("exper")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("List experimental packages"),
                )
                .subcommand(
                    SubCommand::with_name("stable")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("List stable packages"),
                )
                .subcommand(
                    SubCommand::with_name("lts")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("List lts packages"),
                )
                .subcommand(
                    SubCommand::with_name("official")
                        .setting(AppSettings::ColoredHelp)
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about("List official packages"),
                ),
        )
        .subcommand(
            SubCommand::with_name("update")
                .setting(AppSettings::ColoredHelp)
                .version(crate_version!())
                .author(crate_authors!())
                .about("Update installed packages"),
        )
        .get_matches();

    match args.subcommand() {
        ("fetch", Some(a)) => {
            if a.is_present("all") {
                releases.fetch_official_releases(&settings).await;
                releases.fetch_lts_releases(&settings).await;
                releases.fetch_latest_stable(&settings).await;
                releases.fetch_latest_daily(&settings).await;
                releases.fetch_experimental_branches(&settings).await;
            } else {
                if a.is_present("official") {
                    releases.fetch_official_releases(&settings).await;
                }

                if a.is_present("lts") {
                    releases.fetch_lts_releases(&settings).await;
                }

                if a.is_present("stable") {
                    releases.fetch_latest_stable(&settings).await;
                }

                if a.is_present("daily") {
                    releases.fetch_latest_daily(&settings).await;
                }

                if a.is_present("exper") {
                    releases.fetch_experimental_branches(&settings).await;
                }
            }
        }
        ("list", Some(a)) => match a.subcommand() {
            ("daily", Some(b)) => {
                println!("ID    Build");
                for (i, p) in releases.latest_daily.iter().enumerate() {
                    println!("{}    {}", i, p.name);
                }
            }
            ("exper", Some(b)) => {
                println!("ID    Build");
                for (i, p) in releases.experimental_branches.iter().enumerate() {
                    println!("{}    {}", i, p.name);
                }
            }
            ("stable", Some(b)) => {
                println!("ID    Build");
                for (i, p) in releases.latest_stable.iter().enumerate() {
                    println!("{}    {}", i, p.name);
                }
            }
            ("lts", Some(b)) => {
                println!("ID    Build");
                for (i, r) in releases.lts_releases.iter().enumerate() {
                    for p in &r.packages {
                        println!("{}    {}", i, p.name);
                    }
                }
            }
            ("official", Some(b)) => {
                println!("ID    Build");
                for (i, r) in releases.official_releases.iter().enumerate() {
                    for (u, p) in r.packages.iter().enumerate() {
                        println!("{}.{}    {}", i, u, p.name);
                    }
                }
            }
            _ => (),
        },
        ("install", Some(a)) => match a.subcommand() {
            ("daily", Some(b)) => {
                if b.is_present("name") {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .latest_daily
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .download(&settings)
                            .await?;
                    }
                } else {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .latest_daily
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .download(&settings)
                            .await?;
                    }
                }
            }
            ("exper", Some(b)) => {
                if b.is_present("name") {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .experimental_branches
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .download(&settings)
                            .await?;
                    }
                } else {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .experimental_branches
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .download(&settings)
                            .await?;
                    }
                }
            }
            ("stable", Some(b)) => {
                if b.is_present("name") {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .latest_stable
                            .iter()
                            .find(|p| p.name == build)
                            .unwrap()
                            .download(&settings)
                            .await?;
                    }
                } else {
                    for build in b.values_of("build").unwrap() {
                        releases
                            .latest_stable
                            .iter()
                            .enumerate()
                            .find(|(i, _)| *i == usize::from_str(build).unwrap())
                            .unwrap()
                            .1
                            .download(&settings)
                            .await?;
                    }
                }
            }
            ("lts", Some(b)) => {
                if b.is_present("name") {
                    for build in b.values_of("build").unwrap() {
                        for r in &releases.lts_releases {
                            for p in &r.packages {
                                if p.name == build {
                                    p.download(&settings).await?;
                                }
                            }
                        }
                    }
                } else {
                    for build in b.values_of("build").unwrap() {
                        for (i, r) in releases.lts_releases.iter().enumerate() {
                            if i == usize::from_str(build).unwrap() {
                                r.packages
                                    .iter()
                                    .next()
                                    .unwrap()
                                    .download(&settings)
                                    .await?;
                            }
                        }
                    }
                }
            }
            ("official", Some(b)) => {
                if b.is_present("name") {
                    for build in b.values_of("build").unwrap() {
                        for r in &releases.official_releases {
                            for p in &r.packages {
                                if p.name == build {
                                    p.download(&settings).await?;
                                }
                            }
                        }
                    }
                } else {
                    for build in b.values_of("build").unwrap() {
                        for (i, r) in releases.official_releases.iter().enumerate() {
                            for (u, p) in r.packages.iter().enumerate() {
                                if format!("{}.{}", i, u) == build {
                                    p.download(&settings).await?;
                                }
                            }
                        }
                    }
                }
            }
            _ => (),
        },
        _ => (), // TODO: Other subcommands.
    }

    println!("{:#?}", releases);

    Ok(())
}
