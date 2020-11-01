//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod releases;
mod settings;
pub use crate::releases::Releases;
pub use crate::settings::Settings;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand,
};
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
                .about("Install packages"),
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
                        .required_unless_one(&[
                            "stable",
                            "daily",
                            "experimental",
                            "official",
                            "all",
                        ]),
                )
                .arg(
                    Arg::with_name("stable")
                        .short("s")
                        .long("stable")
                        .help("Fetch stable releases")
                        .required_unless_one(&["lts", "daily", "experimental", "official", "all"]),
                )
                .arg(
                    Arg::with_name("daily")
                        .short("d")
                        .long("daily")
                        .help("Fetch daily releases")
                        .required_unless_one(&["stable", "lts", "experimental", "official", "all"]),
                )
                .arg(
                    Arg::with_name("experimental")
                        .short("e")
                        .long("experimental")
                        .help("Fetch experimental releases")
                        .required_unless_one(&["stable", "daily", "lts", "official", "all"]),
                )
                .arg(
                    Arg::with_name("official")
                        .short("o")
                        .long("official")
                        .help("Fetch official releases")
                        .required_unless_one(&["stable", "daily", "experimental", "lts", "all"]),
                )
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Fetch all releases")
                        .conflicts_with_all(&["stable", "daily", "experimental", "lts", "official"])
                        .required_unless_one(&[
                            "stable",
                            "daily",
                            "experimental",
                            "lts",
                            "official",
                        ]),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .setting(AppSettings::ColoredHelp)
                .version(crate_version!())
                .author(crate_authors!())
                .about("List packages"),
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

                if a.is_present("experimental") {
                    releases.fetch_experimental_branches(&settings).await;
                }
            }
        }
        _ => (), //todo!("Other subcommands"),
    }

    println!("{:#?}", releases);

    Ok(())
}
