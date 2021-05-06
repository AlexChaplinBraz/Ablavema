//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{
    gui::GuiFlags,
    helpers::{
        check_self_updates, cli_install, cli_list_narrow, cli_list_wide, get_self_releases,
        is_time_to_update, process_bool_arg,
    },
    releases::{ReleaseType, Releases},
    settings::{
        get_setting, save_settings, set_setting, ModifierKey, CAN_CONNECT, LAUNCH_GUI, ONLY_CLI,
    },
};
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgGroup,
    SubCommand,
};
use device_query::{DeviceQuery, DeviceState};
use std::{str::FromStr, sync::atomic::Ordering};
use tokio::fs::remove_dir_all;

pub async fn run_cli() -> GuiFlags {
    let (mut releases, initialised) = Releases::init().await;
    let mut self_releases = None;

    // Workaround for 'clap' not supporting colours on Windows,
    // even though 'indicatif' does display colours on Windows.
    // TODO: Upgrade to version 3 once it's out of beta.
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
        match get_setting().default_package.clone() {
            Some(package) => package.name,
            None => String::default(),
        },
        right_ansi_code
    );
    let help_bypass_launcher = format!(
        "If default package is set, only open launcher when the set modifier key is held down [current: {}{}{}]",
        left_ansi_code,
        get_setting().bypass_launcher,
        right_ansi_code
    );
    let help_modifier_key = format!(
        "Modifier key to use for opening launcher [current: {}{}{}]",
        left_ansi_code,
        get_setting().modifier_key,
        right_ansi_code
    );
    let help_use_latest_as_default = format!(
        "Change to the latest package of the same build type when updating [current: {}{}{}]",
        left_ansi_code,
        get_setting().use_latest_as_default,
        right_ansi_code
    );
    let help_check_updates_at_launch = format!(
        "Check for updates at launch [current: {}{}{}]",
        left_ansi_code,
        get_setting().check_updates_at_launch,
        right_ansi_code
    );
    let help_minutes_between_updates = format!(
        "Amount of minutes to wait between update checks [current: {}{}{}]",
        left_ansi_code,
        get_setting().minutes_between_updates,
        right_ansi_code
    );
    let help_update_daily = format!(
        "Download the latest daily package [current: {}{}{}]",
        left_ansi_code,
        get_setting().update_daily,
        right_ansi_code
    );
    let help_update_branched = format!(
        "Download the latest branched package [current: {}{}{}]",
        left_ansi_code,
        get_setting().update_branched,
        right_ansi_code
    );
    let help_update_stable = format!(
        "Download the latest stable package [current: {}{}{}]",
        left_ansi_code,
        get_setting().update_stable,
        right_ansi_code
    );
    let help_update_lts = format!(
        "Download the latest LTS package [current: {}{}{}]",
        left_ansi_code,
        get_setting().update_lts,
        right_ansi_code
    );
    let help_keep_only_latest_daily = format!(
        "Remove all daily packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        get_setting().keep_only_latest_daily,
        right_ansi_code
    );
    let help_keep_only_latest_branched = format!(
        "Remove all branched packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        get_setting().keep_only_latest_branched,
        right_ansi_code
    );
    let help_keep_only_latest_stable = format!(
        "Remove all stable packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        get_setting().keep_only_latest_stable,
        right_ansi_code
    );
    let help_keep_only_latest_lts = format!(
        "Remove all LTS packages other than the newest [current: {}{}{}]",
        left_ansi_code,
        get_setting().keep_only_latest_lts,
        right_ansi_code
    );

    // TODO: Add the path changing to the CLI.
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
                    Arg::with_name("bypass_launcher")
                        .display_order(13)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("y")
                        .long("bypass-launcher")
                        .help(&help_bypass_launcher),
                )
                .arg(
                    Arg::with_name("modifier_key")
                        .display_order(17)
                        .takes_value(true)
                        .value_name("KEY")
                        .possible_values(&["shift", "ctrl", "alt"])
                        .short("k")
                        .long("modifier-key")
                        .help(&help_modifier_key),
                )
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
                    Arg::with_name("update_branched")
                        .display_order(40)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("b")
                        .long("update-branched")
                        .help(&help_update_branched),
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
                        .long("keep-only-latest-daily")
                        .help(&help_keep_only_latest_daily),
                )
                .arg(
                    Arg::with_name("keep_only_latest_branched")
                        .display_order(80)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("B")
                        .long("keep-only-latest-branched")
                        .help(&help_keep_only_latest_branched),
                )
                .arg(
                    Arg::with_name("keep_only_latest_stable")
                        .display_order(90)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("S")
                        .long("keep-only-latest-stable")
                        .help(&help_keep_only_latest_stable),
                )
                .arg(
                    Arg::with_name("keep_only_latest_lts")
                        .display_order(100)
                        .takes_value(true)
                        .value_name("BOOL")
                        .possible_values(&["t", "f", "true", "false"])
                        .short("L")
                        .long("keep-only-latest-lts")
                        .help(&help_keep_only_latest_lts),
                )
                .group(
                    ArgGroup::with_name("config_group")
                        .args(&[
                            "bypass_launcher",
                            "modifier_key",
                            "use_latest_as_default",
                            "check_updates_at_launch",
                            "minutes_between_updates",
                            "update_daily",
                            "update_branched",
                            "update_stable",
                            "update_lts",
                            "keep_only_latest_daily",
                            "keep_only_latest_branched",
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
                        .short("A")
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
                    Arg::with_name("branched")
                        .short("b")
                        .long("branched")
                        .help("Fetch branched packages"),
                )
                .arg(
                    Arg::with_name("stable")
                        .short("s")
                        .long("stable")
                        .help("Fetch stable packages"),
                )
                .arg(
                    Arg::with_name("lts")
                        .short("l")
                        .long("lts")
                        .help("Fetch LTS packages"),
                )
                .arg(
                    Arg::with_name("archived")
                        .short("a")
                        .long("archived")
                        .help("Fetch archived packages"),
                )
                .group(
                    ArgGroup::with_name("fetch_group")
                        .args(&["all", "daily", "branched", "stable", "lts", "archived"])
                        .required(true)
                        .multiple(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .about("Install packages")
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("redownload")
                        .global(true)
                        .short("D")
                        .long("redownload")
                        .help("Redownload packages even if already cached"),
                )
                .arg(
                    Arg::with_name("reinstall")
                        .global(true)
                        .short("I")
                        .long("reinstall")
                        .help("Reinstall packages even if already installed"),
                )
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
                    SubCommand::with_name("branched")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install branched packages")
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
                    SubCommand::with_name("archived")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .about("Install archived packages")
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
                .arg(
                    Arg::with_name("invert")
                        .global(true)
                        .short("i")
                        .long("invert")
                        .help("Invert the table")
                        .long_help("Invert the table so it's possible to start reading from ID 0 from the command prompt. Useful for long tables like the one for the archived packages if you're looking for the latest ones and don't want to scroll up a lot."),
                )
                .arg(
                    Arg::with_name("wide")
                        .global(true)
                        .short("w")
                        .long("wide")
                        .help("Use the wide version of the table")
                        .long_help("Use the wide version of the table. The wider table can be better for listing archived packages, as they take three times more space vertically using the narrow table."),
                )
                .subcommand(
                    SubCommand::with_name("daily")
                        .about("List daily packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("branched")
                        .about("List branched packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("stable")
                        .about("List stable packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("lts")
                        .about("List lts packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("archived")
                        .about("List archived packages")
                        .help_message("Print help and exit"),
                )
                .subcommand(
                    SubCommand::with_name("installed")
                        .about("List installed packages")
                        .help_message("Print help and exit"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("Remove packages and cached files")
                .help_message("Print help and exit")
                .arg(
                    Arg::with_name("id")
                        .value_name("ID")
                        .multiple(true)
                        .help("A list of packages to remove"),
                )
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .help("Use the name of the package instead of the ID")
                        .long_help("Use the name of the package instead of the ID. This can be useful for scripting, since the ID may change but the name will not."),
                )
                .arg(
                    Arg::with_name("cache")
                        .short("c")
                        .long("cache")
                        .help("Remove all cache files"),
                )
                .arg(
                    Arg::with_name("packages")
                        .short("p")
                        .long("packages")
                        .help("Remove all packages"),
                )
                .group(
                    ArgGroup::with_name("remove_group")
                        .args(&["id", "cache", "packages"])
                        .required(true)
                        .multiple(true)
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
                        .required_unless("unset")
                        .help("Default package to use for opening .blend files"),
                )
                .arg(
                    Arg::with_name("unset")
                        .conflicts_with("id")
                        .short("u")
                        .long("unset")
                        .help("Unset the default package"),
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
            process_bool_arg(a, "bypass_launcher");

            if a.is_present("modifier_key") {
                let new_arg = a.value_of("modifier_key").unwrap();
                let old_arg = get_setting().modifier_key.clone();

                if new_arg == old_arg.to_string() {
                    println!("'modifier-key' is unchanged from '{}'.", old_arg);
                } else {
                    set_setting().modifier_key = match new_arg {
                        "shift" => ModifierKey::Shift,
                        "ctrl" => ModifierKey::Control,
                        "alt" => ModifierKey::Alt,
                        _ => unreachable!("Unknown ModifierKey"),
                    };

                    println!(
                        "'modifier-key' changed from '{}' to '{}'.",
                        old_arg, new_arg
                    );
                }
            }

            process_bool_arg(a, "use_latest_as_default");
            process_bool_arg(a, "check_updates_at_launch");

            if a.is_present("minutes_between_updates") {
                let new_arg =
                    u64::from_str(a.value_of("minutes_between_updates").unwrap()).unwrap();
                let old_arg = get_setting().minutes_between_updates;

                if new_arg == old_arg {
                    println!("'minutes-between-updates' is unchanged from '{}'.", old_arg);
                } else {
                    set_setting().minutes_between_updates = new_arg;

                    println!(
                        "'minutes-between-updates' changed from '{}' to '{}'.",
                        old_arg, new_arg
                    );
                }
            }

            process_bool_arg(a, "update_daily");
            process_bool_arg(a, "update_branched");
            process_bool_arg(a, "update_stable");
            process_bool_arg(a, "update_lts");
            process_bool_arg(a, "keep_only_latest_daily");
            process_bool_arg(a, "keep_only_latest_branched");
            process_bool_arg(a, "keep_only_latest_stable");
            process_bool_arg(a, "keep_only_latest_lts");

            save_settings();
        }
        ("fetch", Some(a)) => {
            if CAN_CONNECT.load(Ordering::Relaxed) {
                if a.is_present("all") {
                    releases.daily = Releases::check_daily_updates(releases.daily).await.1;
                    releases.branched = Releases::check_branched_updates(releases.branched).await.1;
                    releases.stable = Releases::check_stable_updates(releases.stable).await.1;
                    releases.lts = Releases::check_lts_updates(releases.lts).await.1;
                    releases.archived = Releases::check_archived_updates(releases.archived).await.1;
                } else {
                    if a.is_present("daily") {
                        releases.daily =
                            Releases::check_daily_updates(releases.daily.take()).await.1;
                    }
                    if a.is_present("branched") {
                        releases.branched =
                            Releases::check_branched_updates(releases.branched.take())
                                .await
                                .1;
                    }
                    if a.is_present("stable") {
                        releases.stable = Releases::check_stable_updates(releases.stable.take())
                            .await
                            .1;
                    }
                    if a.is_present("lts") {
                        releases.lts = Releases::check_lts_updates(releases.lts.take()).await.1;
                    }
                    if a.is_present("archived") {
                        releases.archived =
                            Releases::check_archived_updates(releases.archived.take())
                                .await
                                .1;
                    }
                }
            } else {
                println!("Error: Failed to connect to server.");
            }
        }
        ("install", Some(a)) => match a.subcommand() {
            ("daily", Some(b)) => cli_install(b, &releases.daily, "daily").await,
            ("branched", Some(b)) => cli_install(b, &releases.branched, "branched").await,
            ("stable", Some(b)) => cli_install(b, &releases.stable, "stable").await,
            ("lts", Some(b)) => cli_install(b, &releases.lts, "LTS").await,
            ("archived", Some(b)) => cli_install(b, &releases.archived, "archived").await,
            _ => unreachable!("Install subcommand"),
        },
        ("list", Some(a)) => match a.subcommand() {
            ("daily", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.daily, "daily", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.daily, "daily", b.is_present("invert"));
                }
            }
            ("branched", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.branched, "branched", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.branched, "branched", b.is_present("invert"));
                }
            }
            ("stable", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.stable, "stable", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.stable, "stable", b.is_present("invert"));
                }
            }
            ("lts", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.lts, "LTS", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.lts, "LTS", b.is_present("invert"));
                }
            }
            ("archived", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.archived, "archived", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.archived, "archived", b.is_present("invert"));
                }
            }
            ("installed", Some(b)) => {
                if b.is_present("wide") {
                    cli_list_wide(&releases.installed, "installed", b.is_present("invert"));
                } else {
                    cli_list_narrow(&releases.installed, "installed", b.is_present("invert"));
                }
            }
            _ => unreachable!("List subcommand"),
        },
        ("remove", Some(a)) => {
            if a.is_present("cache") {
                remove_dir_all(get_setting().cache_dir.clone())
                    .await
                    .unwrap();

                println!("Removed all cache files.");
            }

            if a.is_present("packages") {
                remove_dir_all(get_setting().packages_dir.clone())
                    .await
                    .unwrap();

                println!("Removed all packages.");

                if !get_setting().default_package.is_none() {
                    set_setting().default_package = None;
                    save_settings();

                    println!("All packages removed. Please install and select a new package.");
                }
            }

            if a.is_present("id") && !a.is_present("packages") {
                let mut values = Vec::new();

                for build in a.values_of("id").unwrap() {
                    if values.contains(&build.to_string()) {
                        continue;
                    }
                    values.push(build.to_string());

                    if a.is_present("name") {
                        match releases
                            .installed
                            .iter()
                            .find(|package| package.name == build)
                        {
                            Some(a) => a.remove(),
                            None => {
                                println!("No installed package named '{}' found.", build);
                                continue;
                            }
                        };
                    } else {
                        let build = usize::from_str(build).unwrap();

                        match releases
                            .installed
                            .iter()
                            .enumerate()
                            .find(|(index, _)| *index == build)
                        {
                            Some(a) => a.1.remove(),
                            None => {
                                println!("No installed package with ID '{}' found.", build);
                                continue;
                            }
                        };
                    }
                }

                if get_setting().default_package.is_some() {
                    releases.installed.fetch();

                    let old_default = get_setting().default_package.clone().unwrap();

                    if releases
                        .installed
                        .iter()
                        .find(|package| package.name == old_default.name)
                        .is_none()
                    {
                        set_setting().default_package = None;
                        save_settings();

                        println!(
                            "Default package '{}' was removed. Please select a new package.",
                            old_default.name
                        );
                    }
                }
            }
        }
        ("select", Some(a)) => {
            if a.is_present("unset") {
                match get_setting().default_package.clone() {
                    Some(package) => {
                        set_setting().default_package = None;
                        save_settings();
                        println!("Default package '{}' was unset.", package.name);
                    }
                    None => {
                        println!("No default package to unset.");
                    }
                }
            } else {
                set_setting().default_package = {
                    if a.is_present("name") {
                        match releases
                            .installed
                            .iter()
                            .find(|package| package.name == a.value_of("id").unwrap())
                        {
                            Some(package) => Some(package.clone()),
                            None => panic!("No installed package with this name found"),
                        }
                    } else {
                        let id = usize::from_str(a.value_of("id").unwrap()).unwrap();

                        match releases
                            .installed
                            .iter()
                            .enumerate()
                            .find(|(index, _)| *index == id)
                        {
                            Some((_, package)) => Some(package.clone()),
                            None => panic!("No installed package with this ID found"),
                        }
                    }
                };

                save_settings();
                println!(
                    "Selected: {}",
                    get_setting().default_package.clone().unwrap().name
                );
            }
        }
        ("update", Some(_a)) => {
            if CAN_CONNECT.load(Ordering::Relaxed) {
                let packages = Releases::check_updates(releases.take()).await;
                releases.add_new_packages(packages);
                releases.cli_install_updates().await;
            } else {
                println!("Error: Failed to connect to server.");
            }
        }
        _ => {
            ONLY_CLI.store(false, Ordering::Relaxed);

            if get_setting().check_updates_at_launch && !initialised {
                if is_time_to_update() {
                    if CAN_CONNECT.load(Ordering::Relaxed) {
                        let packages = Releases::check_updates(releases.take()).await;

                        // This only launches the GUI when new packages were found for the first time.
                        // Meaning it won't pop the GUI again if the user chose to ignore them.
                        if packages.0 {
                            LAUNCH_GUI.store(true, Ordering::Relaxed);
                        }

                        releases.add_new_packages(packages);
                    } else {
                        println!("Failed to connect to server and check for updates.");
                    }
                } else {
                    println!("Not the time to check for updates yet.");
                }
            }

            // TODO: Add the self-updater to CLI.
            // TODO: Add setting to notify only on newer version when downgraded.
            // This would make it possible to downgrade to 0.2.1 from 0.2.2
            // and not get prompted until a newer version than 0.2.2 is released.
            if get_setting().check_self_updates_at_launch
                && is_time_to_update()
                && CAN_CONNECT.load(Ordering::Relaxed)
            {
                self_releases = get_self_releases();

                if let Some(updates) = check_self_updates(&self_releases) {
                    println!(
                        "Found {} Ablavema update{}.",
                        updates,
                        if updates > 1 { "s" } else { "" }
                    );
                    LAUNCH_GUI.store(true, Ordering::Relaxed);
                }
            }

            if get_setting().bypass_launcher && !LAUNCH_GUI.load(Ordering::Relaxed) {
                let device_state = DeviceState::new();
                let keys = device_state.get_keys();

                if keys.contains(&get_setting().modifier_key.get_keycode()) {
                    LAUNCH_GUI.store(true, Ordering::Relaxed);
                }
            } else {
                LAUNCH_GUI.store(true, Ordering::Relaxed);
            }
        }
    }

    GuiFlags {
        releases,
        file_path: if args.is_present("path") {
            Some(String::from(args.value_of("path").unwrap()))
        } else {
            None
        },
        self_releases,
    }
}
