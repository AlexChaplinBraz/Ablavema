use super::Tab;
use crate::{
    gui::{
        extra::{BuildTypeSettings, Choice, Location},
        message::GuiMessage,
        style::Theme,
    },
    package::Build,
    releases::{ReleaseType, Releases},
    settings::{get_setting, ModifierKey, CONFIG_FILE_ENV, PORTABLE, PROJECT_DIRS, TEXT_SIZE},
};
use fs2::available_space;
use fs_extra::dir;
use iced::{
    alignment::Horizontal,
    pure::{
        widget::{Button, Column, Container, Radio, Row, Scrollable, Text},
        Element,
    },
    Alignment, Length, Rule, Space,
};
use std::sync::atomic::Ordering;

impl Tab {
    pub fn settings_body(releases: &Releases) -> Element<'_, GuiMessage> {
        let settings_block_intro = |title, description| {
            Column::new()
                .spacing(10)
                .push(
                    Text::new(title)
                        .width(Length::Fill)
                        .horizontal_alignment(Horizontal::Center)
                        .size(TEXT_SIZE * 3)
                        .color(get_setting().theme.highlight_text()),
                )
                .push(
                    Text::new(description)
                        .width(Length::Fill)
                        .horizontal_alignment(Horizontal::Center),
                )
        };

        let separator = || Rule::horizontal(0).style(get_setting().theme);

        macro_rules! choice_setting {
            ($title:expr, $description:expr, &$array:expr, $option:expr, $message:expr,) => {
                Row::new()
                    .align_items(Alignment::Center)
                    .push(Space::with_width(Length::Units(10)))
                    .push(
                        Column::new()
                            .spacing(10)
                            .width(Length::Fill)
                            .push(
                                Text::new($title)
                                    .color(get_setting().theme.highlight_text())
                                    .size(TEXT_SIZE * 2),
                            )
                            .push(Text::new($description)),
                    )
                    .push(Space::with_width(Length::Units(20)))
                    .push($array.iter().fold(
                        Column::new().spacing(10).width(Length::Units(110)),
                        |col, value| {
                            col.push(
                                Radio::new(*value, &format!("{:?}", value), $option, $message)
                                    .style(get_setting().theme),
                            )
                        },
                    ))
                    .push(Space::with_width(Length::Units(10)))
            };
        }

        let choice = |flag| match flag {
            true => Some(Choice::Enable),
            false => Some(Choice::Disable),
        };

        // TODO: Change to a stepper when available.
        // A proper stepper would be better, but this will do for now.
        // At least it's much better than a slider.
        let min_button = |label, amount| {
            Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .on_press(GuiMessage::MinutesBetweenUpdatesChanged(amount))
                .width(Length::Fill)
                .style(get_setting().theme.tab_button())
        };

        let change_location_button = |label, location| {
            Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Fill)
                .style(get_setting().theme.tab_button())
                .on_press(GuiMessage::ChangeLocation(location))
        };

        let reset_location_button = |location, default| {
            let button = Button::new(Text::new("[R]")).style(get_setting().theme.tab_button());

            if default {
                button
            } else {
                button.on_press(GuiMessage::ResetLocation(location))
            }
        };

        let remove_db_button = |label, build_type, exists| {
            let button = Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Fill)
                .style(get_setting().theme.tab_button());

            if exists {
                Row::new().push(button.on_press(GuiMessage::RemoveDatabases(build_type)))
            } else {
                Row::new().push(button)
            }
        };

        let daily_latest_db_exists = releases.daily_latest.get_db_path().exists();
        let daily_archive_db_exists = releases.daily_archive.get_db_path().exists();
        let experimental_latest_db_exists = releases.experimental_latest.get_db_path().exists();
        let experimental_archive_db_exists = releases.experimental_archive.get_db_path().exists();
        let patch_latest_db_exists = releases.patch_latest.get_db_path().exists();
        let patch_archive_db_exists = releases.patch_archive.get_db_path().exists();
        let stable_latest_db_exists = releases.stable_latest.get_db_path().exists();
        let stable_archive_db_exists = releases.stable_archive.get_db_path().exists();
        let lts_db_exists = releases.lts.get_db_path().exists();

        let remove_packages_button = |label, build_type, exists| {
            let button = Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Fill)
                .style(get_setting().theme.tab_button());

            if exists {
                Row::new().push(button.on_press(GuiMessage::RemovePackages(build_type)))
            } else {
                Row::new().push(button)
            }
        };

        let daily_latest_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::DailyLatest { .. }))
            .count()
            > 0;
        let daily_archive_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::DailyArchive { .. }))
            .count()
            > 0;
        let experimental_latest_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::ExperimentalLatest { .. }))
            .count()
            > 0;
        let experimental_archive_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::ExperimentalArchive { .. }))
            .count()
            > 0;
        let patch_latest_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::PatchArchive { .. }))
            .count()
            > 0;
        let patch_archive_packages_exist = releases
            .installed
            .iter()
            .filter(|package| matches!(package.build, Build::PatchArchive { .. }))
            .count()
            > 0;
        let stable_latest_packages_exist = releases
            .installed
            .iter()
            .filter(|package| package.build == Build::StableLatest)
            .count()
            > 0;
        let stable_archive_packages_exist = releases
            .installed
            .iter()
            .filter(|package| package.build == Build::StableArchive)
            .count()
            > 0;
        let lts_packages_exist = releases
            .installed
            .iter()
            .filter(|package| package.build == Build::Lts)
            .count()
            > 0;

        let checking_for_updates_block = settings_block_intro(
            "Checking for updates",
            "\
These settings affect how checking for updates works. Enabling specific build types also marks \
the newest package of that build as an update. Keep in mind that you need to first have one \
installed package of that build type for any newer ones to be marked as an update, even if \
you're checking for their updates. It is recommended to disable checking for updates for builds \
that aren't installed to reduce launch time.",
        );

        let check_updates_at_launch = choice_setting!(
            "Check at launch",
            "Increases Ablavema's launch time for about a second or two.",
            &Choice::ALL,
            Some(choice(get_setting().check_updates_at_launch).unwrap()),
            GuiMessage::CheckUpdatesAtLaunch,
        );

        let minutes_between_updates = {
            Row::new()
                .push(Space::with_width(Length::Units(10)))
                .push(
                    Column::new()
                        .width(Length::Fill)
                        .spacing(10)
                        .push(
                            Text::new("Delay between checks")
                                .color(get_setting().theme.highlight_text())
                                .size(TEXT_SIZE * 2),
                        )
                        .push(Text::new(
                            "\
Minutes to wait between update checks. Setting it to 0 will make it check every time. \
Maximum is a day (1440 minutes).",
                        )),
                )
                .push(Space::with_width(Length::Units(10)))
                .push(
                    Column::new()
                        .align_items(Alignment::Center)
                        .width(Length::Units(150))
                        .spacing(3)
                        .push(
                            Row::new()
                                .push(min_button("+1", 1))
                                .push(min_button("+10", 10))
                                .push(min_button("+100", 100)),
                        )
                        .push(Text::new(get_setting().minutes_between_updates.to_string()))
                        .push(
                            Row::new()
                                .push(min_button("-1", -1))
                                .push(min_button("-10", -10))
                                .push(min_button("-100", -100)),
                        ),
                )
                .push(Space::with_width(Length::Units(10)))
        };

        let check_daily_latest = choice_setting!(
            "Check latest daily packages",
            "\
Look for new latest daily packages. Each build, like Alpha and Beta, is considered a separate \
build and will look for updates for itself.",
            &Choice::ALL,
            Some(choice(get_setting().update_daily_latest).unwrap()),
            GuiMessage::UpdateDailyLatest,
        );

        let check_experimental_latest = choice_setting!(
            "Check latest experimental packages",
            "\
Look for new latest experimental packages. Each branch is considered a separate build and will \
look for updates for itself.",
            &Choice::ALL,
            Some(choice(get_setting().update_experimental_latest).unwrap()),
            GuiMessage::UpdateExperimentalLatest,
        );

        let check_patch_latest = choice_setting!(
            "Check latest patched packages",
            "Look for new latest patched packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_patch_latest).unwrap()),
            GuiMessage::UpdatePatchLatest,
        );

        let check_stable_latest = choice_setting!(
            "Check latest stable packages",
            "Look for new latest stable packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_stable_latest).unwrap()),
            GuiMessage::UpdateStableLatest,
        );

        let check_lts = choice_setting!(
            "Check Long-term Support packages",
            "Look for new Long-term Support packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_lts).unwrap()),
            GuiMessage::UpdateLts,
        );

        let others_block =
            settings_block_intro("Miscelaneous", "A few miscellaneous but useful settings.");

        let bypass_launcher = choice_setting!(
            "Bypass launcher",
            "\
The preferred way to use this launcher. If a default package is set and no updates were found, \
only open launcher when the selected modifier key is held down. This way the launcher only makes \
itself known if there's an update or if you want to launch a different package.",
            &Choice::ALL,
            Some(choice(get_setting().bypass_launcher).unwrap()),
            GuiMessage::BypassLauncher,
        );

        let modifier_key = choice_setting!(
            "Modifier key",
            "\
You can start holding the modifier key even before double clicking on a .blend file or Ablavema \
shortcut, but you are able to change it if there's any interference.",
            &ModifierKey::ALL,
            Some(get_setting().modifier_key),
            GuiMessage::ModifierKey,
        );

        let use_latest_as_default = choice_setting!(
            "Use latest as default",
            "\
Change to the latest package of the same build type and version (except the patch number, which \
can be higher) when installing an update.",
            &Choice::ALL,
            Some(choice(get_setting().use_latest_as_default).unwrap()),
            GuiMessage::UseLatestAsDefault,
        );

        let choose_theme = choice_setting!(
            "Choose the theme",
            "Both try to mimic Blender's colour schemes as much as possible.",
            &Theme::ALL,
            Some(get_setting().theme),
            GuiMessage::ThemeChanged,
        );

        let change_location = Row::new()
            .align_items(Alignment::Center)
            .push(Space::with_width(Length::Units(10)))
            .push(
                Column::new()
                    .spacing(10)
                    .width(Length::Fill)
                    .push(
                        Text::new("Change locations")
                            .color(get_setting().theme.highlight_text())
                            .size(TEXT_SIZE * 2),
                    )
                    .push(if PORTABLE.load(Ordering::Relaxed) {
                        Container::new(Text::new(
                            "\
Can't change locations because portable mode is enabled. Delete the \"portable\" file in the \
executable's directory to disable it.",
                        ))
                        .width(Length::Fill)
                    } else {
                        Container::new(
                            Column::new()
                                .spacing(10)
                                .width(Length::Fill)
                                .push(Text::new(
                                    "\
Ablavema's files are stored in the recommended default locations for every platform, but \
changing them is possible.",
                                ))
                                .push(Text::new(&format!(
                                    "\
To change the location of the configuration file, which is located by default at '{}' you can \
set the environment variable {} and it will create that file and use it as the config file, \
whatever its name is.",
                                    PROJECT_DIRS.config_dir().display(),
                                    CONFIG_FILE_ENV
                                )))
                                .push(Text::new(&format!(
                                    "Databases: {}\nPackages: {}\nCache: {}",
                                    get_setting().databases_dir.display(),
                                    get_setting().packages_dir.display(),
                                    get_setting().cache_dir.display()
                                )))
                                .push(
                                    Row::new()
                                        .spacing(5)
                                        .push(change_location_button(
                                            "Databases",
                                            Location::Databases,
                                        ))
                                        .push(reset_location_button(
                                            Location::Databases,
                                            get_setting().databases_dir
                                                == PROJECT_DIRS.config_dir(),
                                        ))
                                        .push(Space::with_width(Length::Units(15)))
                                        .push(change_location_button(
                                            "Packages",
                                            Location::Packages,
                                        ))
                                        .push(reset_location_button(
                                            Location::Packages,
                                            get_setting().packages_dir
                                                == PROJECT_DIRS.data_local_dir(),
                                        ))
                                        .push(Space::with_width(Length::Units(15)))
                                        .push(change_location_button("Cache", Location::Cache))
                                        .push(reset_location_button(
                                            Location::Cache,
                                            get_setting().cache_dir == PROJECT_DIRS.cache_dir(),
                                        )),
                                ),
                        )
                    }),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_databases = Row::new()
            .align_items(Alignment::Center)
            .push(Space::with_width(Length::Units(10)))
            .push(
                Column::new()
                    .spacing(10)
                    .width(Length::Fill)
                    .push(
                        Text::new("Remove databases")
                            .color(get_setting().theme.highlight_text())
                            .size(TEXT_SIZE * 2),
                    )
                    .push(Text::new(
                        "\
Keep in mind that any installed package that's no longer available will not reappear.",
                    ))
                    .push(
                        Column::new()
                            .spacing(20)
                            .push(remove_db_button(
                                "Remove whole directory",
                                BuildTypeSettings::All,
                                true,
                            ))
                            .push(remove_db_button(
                                "Daily (latest)",
                                BuildTypeSettings::DailyLatest,
                                daily_latest_db_exists,
                            ))
                            .push(remove_db_button(
                                "Daily (archive)",
                                BuildTypeSettings::DailyArchive,
                                daily_archive_db_exists,
                            ))
                            .push(remove_db_button(
                                "Experimental (latest)",
                                BuildTypeSettings::ExperimentalLatest,
                                experimental_latest_db_exists,
                            ))
                            .push(remove_db_button(
                                "Experimental (archive)",
                                BuildTypeSettings::ExperimentalArchive,
                                experimental_archive_db_exists,
                            ))
                            .push(remove_db_button(
                                "Patch (latest)",
                                BuildTypeSettings::PatchLatest,
                                patch_latest_db_exists,
                            ))
                            .push(remove_db_button(
                                "Patch (archive)",
                                BuildTypeSettings::PatchArchive,
                                patch_archive_db_exists,
                            ))
                            .push(remove_db_button(
                                "Stable (latest)",
                                BuildTypeSettings::StableLatest,
                                stable_latest_db_exists,
                            ))
                            .push(remove_db_button(
                                "Stable (archive)",
                                BuildTypeSettings::StableArchive,
                                stable_archive_db_exists,
                            ))
                            .push(remove_db_button(
                                "Long-term Support",
                                BuildTypeSettings::Lts,
                                lts_db_exists,
                            )),
                    ),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_packages = Row::new()
            .align_items(Alignment::Center)
            .push(Space::with_width(Length::Units(10)))
            .push(
                Column::new()
                    .spacing(10)
                    .width(Length::Fill)
                    .push(
                        Text::new("Remove packages")
                            .color(get_setting().theme.highlight_text())
                            .size(TEXT_SIZE * 2),
                    )
                    .push(Text::new(
                        "\
Useful for getting rid of a large quantity of packages at the same time.",
                    ))
                    // TODO: Fix slowdowns due to calculating packages' size.
                    .push(Text::new(format!(
                        "Space used by packages: {:.2} GB\nAvailable space: {:.2} GB",
                        dir::get_size(&get_setting().packages_dir).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0,
                        available_space(&get_setting().packages_dir).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0
                    )))
                    .push(
                        Column::new()
                            .spacing(20)
                            .push(remove_packages_button(
                                "Remove whole directory",
                                BuildTypeSettings::All,
                                true,
                            ))
                            .push(remove_packages_button(
                                "Daily (latest)",
                                BuildTypeSettings::DailyLatest,
                                daily_latest_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Daily (archive)",
                                BuildTypeSettings::DailyArchive,
                                daily_archive_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Experimental (latest)",
                                BuildTypeSettings::ExperimentalLatest,
                                experimental_latest_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Experimental (archive)",
                                BuildTypeSettings::ExperimentalArchive,
                                experimental_archive_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Patch (latest)",
                                BuildTypeSettings::PatchLatest,
                                patch_latest_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Patch (archive)",
                                BuildTypeSettings::PatchArchive,
                                patch_archive_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Stable (latest)",
                                BuildTypeSettings::StableLatest,
                                stable_latest_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Stable (archive)",
                                BuildTypeSettings::StableArchive,
                                stable_archive_packages_exist,
                            ))
                            .push(remove_packages_button(
                                "Long-term Support",
                                BuildTypeSettings::Lts,
                                lts_packages_exist,
                            )),
                    ),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_cache = Row::new()
            .align_items(Alignment::Center)
            .push(Space::with_width(Length::Units(10)))
            .push(
                Column::new()
                    .spacing(10)
                    .width(Length::Fill)
                    .push(
                        Text::new("Remove cache")
                            .color(get_setting().theme.highlight_text())
                            .size(TEXT_SIZE * 2),
                    )
                    .push(Text::new(
                        "\
Useful for getting rid of the accumulated cache (mainly downloaded packages) since at the moment \
cache isn't being automatically removed.",
                    ))
                    // TODO: Fix slowdowns due to calculating cache size.
                    .push(Text::new(format!(
                        "Space used by cache: {:.2} GB\nAvailable space: {:.2} GB",
                        dir::get_size(&get_setting().cache_dir).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0,
                        available_space(&get_setting().cache_dir).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0
                    )))
                    .push(
                        Row::new().push(
                            // TODO: Disable button while installing.
                            // Also disable the buttons for the databases and stuff.
                            Button::new(
                                Text::new("Remove all cache")
                                    .horizontal_alignment(Horizontal::Center),
                            )
                            .on_press(GuiMessage::RemoveCache)
                            .width(Length::Fill)
                            .style(get_setting().theme.tab_button()),
                        ),
                    ),
            )
            .push(Space::with_width(Length::Units(10)));

        let self_updater = choice_setting!(
            "Self-updater",
            "\
Update the launcher itself through the built-in system. This enables a hidden tab dedicated to \
updating, which can also be used to read the release notes of every version. Keep in mind that \
if Ablavema is installed through a package manager, the launcher should be updated through it.

Though if made use of even if installed through a package manager, upon updating Ablavema the \
executable would simply be replaced with the newer one, same as if done through the built-in \
self-updater. In this way, making use of this feature is helpful when trying out older versions \
to see if a bug was there before or whatnot.",
            &Choice::ALL,
            Some(choice(get_setting().self_updater).unwrap()),
            GuiMessage::SelfUpdater,
        );

        let settings = Column::new()
            .padding(10)
            .spacing(10)
            .push(checking_for_updates_block)
            .push(separator())
            .push(check_updates_at_launch)
            .push(separator())
            .push(minutes_between_updates)
            .push(separator())
            .push(check_daily_latest)
            .push(separator())
            .push(check_experimental_latest)
            .push(separator())
            .push(check_patch_latest)
            .push(separator())
            .push(check_stable_latest)
            .push(separator())
            .push(check_lts)
            .push(separator())
            .push(others_block)
            .push(separator())
            .push(bypass_launcher)
            .push(separator())
            .push(modifier_key)
            .push(separator())
            .push(use_latest_as_default)
            .push(separator())
            .push(choose_theme)
            .push(separator())
            .push(change_location)
            .push(separator())
            .push(remove_databases)
            .push(separator())
            .push(remove_packages)
            .push(separator())
            .push(remove_cache)
            .push(separator())
            .push(self_updater);

        Container::new(Scrollable::new(if get_setting().self_updater {
            settings.push(separator()).push(choice_setting!(
                "Check for Ablavema updates at launch",
                "\
This uses the same delay as the normal updates. Keep in mind that, at the moment, if you \
downgrade you will be prompted to update Ablavema every time updates are checked.",
                &Choice::ALL,
                Some(choice(get_setting().check_self_updates_at_launch).unwrap()),
                GuiMessage::CheckSelfUpdatesAtLaunch,
            ))
        } else {
            settings
        }))
        .height(Length::Fill)
        .width(Length::Fill)
        .style(get_setting().theme)
        .into()
    }
}
