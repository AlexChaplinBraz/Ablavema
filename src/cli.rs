use crate::{
    gui::extra::GuiFlags,
    helpers::is_time_to_update,
    releases::Releases,
    self_updater::SelfUpdater,
    settings::{get_setting, CAN_CONNECT, LAUNCH_GUI},
};
use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};
use device_query::{DeviceQuery, DeviceState};
use std::sync::atomic::Ordering;

pub async fn run_cli() -> GuiFlags {
    let args = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .global_setting(AppSettings::ColoredHelp)
        .help_message("Print help and exit")
        .version_message("Print version and exit")
        .version_short("v")
        .arg(
            Arg::with_name("path")
                .value_name("PATH")
                .help("Path to .blend file"),
        )
        .get_matches();

    let mut releases = Releases::init().await;
    let mut self_releases = None;

    if get_setting().check_updates_at_launch {
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

    // TODO: Add setting to notify only on newer version when downgraded.
    // This would make it possible to downgrade to 0.2.1 from 0.2.2
    // and not get prompted until a newer version than 0.2.2 is released.
    if get_setting().check_self_updates_at_launch
        && is_time_to_update()
        && CAN_CONNECT.load(Ordering::Relaxed)
    {
        self_releases = SelfUpdater::fetch();

        if let Some(updates) = SelfUpdater::count_new(&self_releases) {
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

    GuiFlags {
        releases,
        file_path: args.value_of("path").map(|file_path| file_path.to_string()),
        self_releases,
    }
}
