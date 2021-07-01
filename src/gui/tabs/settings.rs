use super::Tabs;
use crate::{
    gui::{
        extra::{BuildTypeSettings, Choice, Location},
        message::Message,
        style::Theme,
    },
    package::Build,
    releases::{ReleaseType, Releases},
    settings::{get_setting, ModifierKey, CONFIG_FILE_ENV, PORTABLE, PROJECT_DIRS, TEXT_SIZE},
};
use fs2::available_space;
use fs_extra::dir;
use iced::{
    button, scrollable, Align, Button, Column, Container, Element, HorizontalAlignment, Length,
    Radio, Row, Rule, Scrollable, Space, Text,
};
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub struct SettingsState {
    pub plus_1_button: button::State,
    pub plus_10_button: button::State,
    pub plus_100_button: button::State,
    pub minus_1_button: button::State,
    pub minus_10_button: button::State,
    pub minus_100_button: button::State,
    pub change_databases_location_button: button::State,
    pub reset_databases_location_button: button::State,
    pub change_packages_location_button: button::State,
    pub reset_packages_location_button: button::State,
    pub change_cache_location_button: button::State,
    pub reset_cache_location_button: button::State,
    pub remove_all_dbs_button: button::State,
    pub remove_daily_latest_db_button: button::State,
    pub remove_daily_archive_db_button: button::State,
    pub remove_experimental_latest_db_button: button::State,
    pub remove_experimental_archive_db_button: button::State,
    pub remove_patch_latest_db_button: button::State,
    pub remove_patch_archive_db_button: button::State,
    pub remove_stable_latest_db_button: button::State,
    pub remove_stable_archive_db_button: button::State,
    pub remove_lts_db_button: button::State,
    pub remove_all_packages_button: button::State,
    pub remove_daily_latest_packages_button: button::State,
    pub remove_daily_archive_packages_button: button::State,
    pub remove_experimental_latest_packages_button: button::State,
    pub remove_experimental_archive_packages_button: button::State,
    pub remove_patch_latest_packages_button: button::State,
    pub remove_patch_archive_packages_button: button::State,
    pub remove_stable_latest_packages_button: button::State,
    pub remove_stable_archive_packages_button: button::State,
    pub remove_lts_packages_button: button::State,
    pub remove_cache_button: button::State,
    pub scroll: scrollable::State,
}

impl Tabs {
    pub fn settings_body(&mut self, releases: &Releases) -> Element<'_, Message> {
        let settings_block_intro = |title, description| {
            Column::new()
                .spacing(10)
                .push(
                    Text::new(title)
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(TEXT_SIZE * 3)
                        .color(get_setting().theme.highlight_text()),
                )
                .push(
                    Text::new(description)
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
        };

        let separator = || Rule::horizontal(0).style(get_setting().theme);

        macro_rules! choice_setting {
            ($title:expr, $description:expr, &$array:expr, $option:expr, $message:expr,) => {
                Row::new()
                    .align_items(Align::Center)
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
        let min_button = |label, amount, state| {
            Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .on_press(Message::MinutesBetweenUpdatesChanged(amount))
            .width(Length::Fill)
            .style(get_setting().theme.tab_button())
        };

        let change_location_button = |label, location, state| {
            Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(get_setting().theme.tab_button())
            .on_press(Message::ChangeLocation(location))
        };

        let reset_location_button = |location, default, state| {
            let button =
                Button::new(state, Text::new("[R]")).style(get_setting().theme.tab_button());

            if default {
                button
            } else {
                button.on_press(Message::ResetLocation(location))
            }
        };

        let remove_db_button = |label, build_type, exists, state| {
            let button = Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(get_setting().theme.tab_button());

            if exists {
                Row::new().push(button.on_press(Message::RemoveDatabases(build_type)))
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
        let any_dbs_exist = daily_latest_db_exists
            || daily_archive_db_exists
            || experimental_latest_db_exists
            || experimental_archive_db_exists
            || patch_latest_db_exists
            || patch_archive_db_exists
            || stable_latest_db_exists
            || stable_archive_db_exists
            || lts_db_exists;

        let remove_packages_button = |label, build_type, exists, state| {
            let button = Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(get_setting().theme.tab_button());

            if exists {
                Row::new().push(button.on_press(Message::RemovePackages(build_type)))
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
        let any_packages_exist = daily_latest_packages_exist
            || daily_archive_packages_exist
            || experimental_latest_packages_exist
            || experimental_archive_packages_exist
            || patch_latest_packages_exist
            || patch_archive_packages_exist
            || stable_latest_packages_exist
            || stable_archive_packages_exist
            || lts_packages_exist;

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
            Message::CheckUpdatesAtLaunch,
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
                        .align_items(Align::Center)
                        .width(Length::Units(150))
                        .spacing(3)
                        .push(
                            Row::new()
                                .push(min_button("+1", 1, &mut self.settings_state.plus_1_button))
                                .push(min_button(
                                    "+10",
                                    10,
                                    &mut self.settings_state.plus_10_button,
                                ))
                                .push(min_button(
                                    "+100",
                                    100,
                                    &mut self.settings_state.plus_100_button,
                                )),
                        )
                        .push(Text::new(get_setting().minutes_between_updates.to_string()))
                        .push(
                            Row::new()
                                .push(min_button(
                                    "-1",
                                    -1,
                                    &mut self.settings_state.minus_1_button,
                                ))
                                .push(min_button(
                                    "-10",
                                    -10,
                                    &mut self.settings_state.minus_10_button,
                                ))
                                .push(min_button(
                                    "-100",
                                    -100,
                                    &mut self.settings_state.minus_100_button,
                                )),
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
            Message::UpdateDailyLatest,
        );

        let check_experimental_latest = choice_setting!(
            "Check latest experimental packages",
            "\
Look for new latest experimental packages. Each branch is considered a separate build and will \
look for updates for itself.",
            &Choice::ALL,
            Some(choice(get_setting().update_experimental_latest).unwrap()),
            Message::UpdateExperimentalLatest,
        );

        let check_patch_latest = choice_setting!(
            "Check latest patched packages",
            "Look for new latest patched packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_patch_latest).unwrap()),
            Message::UpdatePatchLatest,
        );

        let check_stable_latest = choice_setting!(
            "Check latest stable packages",
            "Look for new latest stable packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_stable_latest).unwrap()),
            Message::UpdateStableLatest,
        );

        let check_lts = choice_setting!(
            "Check Long-term Support packages",
            "Look for new Long-term Support packages.",
            &Choice::ALL,
            Some(choice(get_setting().update_lts).unwrap()),
            Message::UpdateLts,
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
            Message::BypassLauncher,
        );

        let modifier_key = choice_setting!(
            "Modifier key",
            "\
You can start holding the modifier key even before double clicking on a .blend file or Ablavema \
shortcut, but you are able to change it if there's any interference.",
            &ModifierKey::ALL,
            Some(get_setting().modifier_key),
            Message::ModifierKey,
        );

        let use_latest_as_default = choice_setting!(
            "Use latest as default",
            "\
Change to the latest package of the same build type and version (except the patch number, which \
can be higher) when installing an update.",
            &Choice::ALL,
            Some(choice(get_setting().use_latest_as_default).unwrap()),
            Message::UseLatestAsDefault,
        );

        let choose_theme = choice_setting!(
            "Choose the theme",
            "Both try to mimic Blender's colour schemes as much as possible.",
            &Theme::ALL,
            Some(get_setting().theme),
            Message::ThemeChanged,
        );

        let change_location = Row::new()
            .align_items(Align::Center)
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
                                            &mut self
                                                .settings_state
                                                .change_databases_location_button,
                                        ))
                                        .push(reset_location_button(
                                            Location::Databases,
                                            get_setting().databases_dir
                                                == PROJECT_DIRS.config_dir(),
                                            &mut self
                                                .settings_state
                                                .reset_databases_location_button,
                                        ))
                                        .push(Space::with_width(Length::Units(15)))
                                        .push(change_location_button(
                                            "Packages",
                                            Location::Packages,
                                            &mut self
                                                .settings_state
                                                .change_packages_location_button,
                                        ))
                                        .push(reset_location_button(
                                            Location::Packages,
                                            get_setting().packages_dir
                                                == PROJECT_DIRS.data_local_dir(),
                                            &mut self.settings_state.reset_packages_location_button,
                                        ))
                                        .push(Space::with_width(Length::Units(15)))
                                        .push(change_location_button(
                                            "Cache",
                                            Location::Cache,
                                            &mut self.settings_state.change_cache_location_button,
                                        ))
                                        .push(reset_location_button(
                                            Location::Cache,
                                            get_setting().cache_dir == PROJECT_DIRS.cache_dir(),
                                            &mut self.settings_state.reset_cache_location_button,
                                        )),
                                ),
                        )
                    }),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_databases = Row::new()
            .align_items(Align::Center)
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
Keep in mind that any installed package that's no longer available will not reapear.",
                    ))
                    .push(
                        Column::new()
                            .spacing(20)
                            .push(remove_db_button(
                                "All",
                                BuildTypeSettings::All,
                                any_dbs_exist,
                                &mut self.settings_state.remove_all_dbs_button,
                            ))
                            .push(remove_db_button(
                                "Daily (latest)",
                                BuildTypeSettings::DailyLatest,
                                daily_latest_db_exists,
                                &mut self.settings_state.remove_daily_latest_db_button,
                            ))
                            .push(remove_db_button(
                                "Daily (archive)",
                                BuildTypeSettings::DailyArchive,
                                daily_archive_db_exists,
                                &mut self.settings_state.remove_daily_archive_db_button,
                            ))
                            .push(remove_db_button(
                                "Experimental (latest)",
                                BuildTypeSettings::ExperimentalLatest,
                                experimental_latest_db_exists,
                                &mut self.settings_state.remove_experimental_latest_db_button,
                            ))
                            .push(remove_db_button(
                                "Experimental (archive)",
                                BuildTypeSettings::ExperimentalArchive,
                                experimental_archive_db_exists,
                                &mut self.settings_state.remove_experimental_archive_db_button,
                            ))
                            .push(remove_db_button(
                                "Patch (latest)",
                                BuildTypeSettings::PatchLatest,
                                patch_latest_db_exists,
                                &mut self.settings_state.remove_patch_latest_db_button,
                            ))
                            .push(remove_db_button(
                                "Patch (archive)",
                                BuildTypeSettings::PatchArchive,
                                patch_archive_db_exists,
                                &mut self.settings_state.remove_patch_archive_db_button,
                            ))
                            .push(remove_db_button(
                                "Stable (latest)",
                                BuildTypeSettings::StableLatest,
                                stable_latest_db_exists,
                                &mut self.settings_state.remove_stable_latest_db_button,
                            ))
                            .push(remove_db_button(
                                "Stable (archive)",
                                BuildTypeSettings::StableArchive,
                                stable_archive_db_exists,
                                &mut self.settings_state.remove_stable_archive_db_button,
                            ))
                            .push(remove_db_button(
                                "LTS",
                                BuildTypeSettings::Lts,
                                lts_db_exists,
                                &mut self.settings_state.remove_lts_db_button,
                            )),
                    ),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_packages = Row::new()
            .align_items(Align::Center)
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
                        dir::get_size(get_setting().packages_dir.clone()).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0,
                        available_space(get_setting().packages_dir.clone()).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0
                    )))
                    .push(
                        Column::new()
                            .spacing(20)
                            .push(remove_packages_button(
                                "All",
                                BuildTypeSettings::All,
                                any_packages_exist,
                                &mut self.settings_state.remove_all_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Daily (latest)",
                                BuildTypeSettings::DailyLatest,
                                daily_latest_packages_exist,
                                &mut self.settings_state.remove_daily_latest_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Daily (archive)",
                                BuildTypeSettings::DailyArchive,
                                daily_archive_packages_exist,
                                &mut self.settings_state.remove_daily_archive_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Experimental (latest)",
                                BuildTypeSettings::ExperimentalLatest,
                                experimental_latest_packages_exist,
                                &mut self
                                    .settings_state
                                    .remove_experimental_latest_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Experimental (archive)",
                                BuildTypeSettings::ExperimentalArchive,
                                experimental_archive_packages_exist,
                                &mut self
                                    .settings_state
                                    .remove_experimental_archive_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Patch (latest)",
                                BuildTypeSettings::PatchLatest,
                                patch_latest_packages_exist,
                                &mut self.settings_state.remove_patch_latest_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Patch (archive)",
                                BuildTypeSettings::PatchArchive,
                                patch_archive_packages_exist,
                                &mut self.settings_state.remove_patch_archive_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Stable (latest)",
                                BuildTypeSettings::StableLatest,
                                stable_latest_packages_exist,
                                &mut self.settings_state.remove_stable_latest_packages_button,
                            ))
                            .push(remove_packages_button(
                                "Stable (archive)",
                                BuildTypeSettings::StableArchive,
                                stable_archive_packages_exist,
                                &mut self.settings_state.remove_stable_archive_packages_button,
                            ))
                            .push(remove_packages_button(
                                "LTS",
                                BuildTypeSettings::Lts,
                                lts_packages_exist,
                                &mut self.settings_state.remove_lts_packages_button,
                            )),
                    ),
            )
            .push(Space::with_width(Length::Units(10)));

        let remove_cache = Row::new()
            .align_items(Align::Center)
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
                        dir::get_size(get_setting().cache_dir.clone()).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0,
                        available_space(get_setting().cache_dir.clone()).unwrap() as f64
                            / 1024.0
                            / 1024.0
                            / 1024.0
                    )))
                    .push(
                        Row::new().push(
                            // TODO: Disable button while installing.
                            // Also disable the buttons for the databases and stuff.
                            Button::new(
                                &mut self.settings_state.remove_cache_button,
                                Text::new("Remove all cache")
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(Message::RemoveCache)
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
if Ablavema is installed through a package manager, the laucher should be updated through it.

Though if made use of even if installed through a package manager, upon updating Ablavema the \
executable would simply be replaced with the newer one, same as if done through the built-in \
self-updater. In this way, making use of this feature is helpful when trying out older versions \
to see if a bug was there before or whatnot.",
            &Choice::ALL,
            Some(choice(get_setting().self_updater).unwrap()),
            Message::SelfUpdater,
        );

        let items = [
            iced::Element::from(checking_for_updates_block),
            check_updates_at_launch.into(),
            minutes_between_updates.into(),
            check_daily_latest.into(),
            check_experimental_latest.into(),
            check_patch_latest.into(),
            check_stable_latest.into(),
            check_lts.into(),
            others_block.into(),
            bypass_launcher.into(),
            modifier_key.into(),
            use_latest_as_default.into(),
            choose_theme.into(),
            change_location.into(),
            remove_databases.into(),
            remove_packages.into(),
            remove_cache.into(),
            self_updater.into(),
        ];

        let num_items = items.len();
        let mut settings = Column::new().padding(10).spacing(10);
        for (i, setting) in std::array::IntoIter::new(items).enumerate() {
            settings = settings.push(setting);
            if i + 1 < num_items {
                settings = settings.push(separator());
            }
        }

        Container::new(Scrollable::new(&mut self.settings_state.scroll).push(
            if get_setting().self_updater {
                settings.push(separator()).push(choice_setting!(
                    "Check for Ablavema updates at launch",
                    "\
This uses the same delay as the normal updates. Keep in mind that, at the moment, if you \
downgrade you will be prompted to update Ablavema every time updates are checked.",
                    &Choice::ALL,
                    Some(choice(get_setting().check_self_updates_at_launch).unwrap()),
                    Message::CheckSelfUpdatesAtLaunch,
                ))
            } else {
                settings
            },
        ))
        .height(Length::Fill)
        .width(Length::Fill)
        .style(get_setting().theme)
        .into()
    }
}
