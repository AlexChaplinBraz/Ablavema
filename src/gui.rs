//#![allow(dead_code, unused_imports, unused_variables)]
mod install;
pub mod style;
use self::{
    install::{Install, Progress},
    style::Theme,
};
use crate::{
    helpers::open_blender,
    package::{Build, Package, PackageState, PackageStatus},
    releases::{
        archived::Archived, branched::Branched, daily::Daily, lts::Lts, stable::Stable,
        ReleaseType, Releases,
    },
    settings::{ModifierKey, CAN_CONNECT, SETTINGS, TEXT_SIZE},
};
use iced::{
    button, pick_list, scrollable, slider, Align, Application, Button, Checkbox, Column, Command,
    Container, Element, Executor, HorizontalAlignment, Length, PickList, ProgressBar, Radio, Row,
    Rule, Scrollable, Slider, Subscription, Text,
};
use itertools::Itertools;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, iter, process, sync::atomic::Ordering};

#[derive(Debug)]
pub struct Gui {
    releases: Releases,
    file_path: Option<String>,
    installing: Vec<(Package, usize)>,
    state: GuiState,
    tab: Tab,
    theme: Theme,
}

impl Gui {
    /// A tuple is returned where:
    /// (true_if_available, true_if_for_install, package)
    async fn check_availability(for_install: bool, package: Package) -> (bool, bool, Package) {
        match reqwest::get(&package.url).await {
            Ok(response) => {
                if response.status().is_client_error() {
                    (false, for_install, package)
                } else {
                    (true, for_install, package)
                }
            }
            Err(_) => panic!("Failed to connect to server"),
        }
    }

    async fn pass_package(package: Package) -> Package {
        package
    }

    async fn check_for_updates(
        packages: (Daily, Branched, Stable, Lts),
    ) -> (bool, Daily, Branched, Stable, Lts) {
        Releases::check_updates(packages).await
    }

    async fn check_all(
        daily: Daily,
        branched: Branched,
        stable: Stable,
        lts: Lts,
        archived: Archived,
    ) -> (bool, Daily, Branched, Stable, Lts, Archived) {
        (
            true,
            Releases::check_daily_updates(daily).await.1,
            Releases::check_branched_updates(branched).await.1,
            Releases::check_stable_updates(stable).await.1,
            Releases::check_lts_updates(lts).await.1,
            Releases::check_archived_updates(archived).await.1,
        )
    }

    async fn check_daily(packages: Daily) -> (bool, Daily) {
        Releases::check_daily_updates(packages).await
    }

    async fn check_branched(packages: Branched) -> (bool, Branched) {
        Releases::check_branched_updates(packages).await
    }

    async fn check_stable(packages: Stable) -> (bool, Stable) {
        Releases::check_stable_updates(packages).await
    }

    async fn check_lts(packages: Lts) -> (bool, Lts) {
        Releases::check_lts_updates(packages).await
    }

    async fn check_archived(packages: Archived) -> (bool, Archived) {
        Releases::check_archived_updates(packages).await
    }
}

impl Application for Gui {
    type Executor = GlobalTokio;
    type Message = Message;
    type Flags = GuiFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let releases = flags.releases;

        let default_package = SETTINGS.read().unwrap().default_package.clone();
        if let Some(package) = default_package {
            if !releases.installed.contains(&package) {
                SETTINGS.write().unwrap().default_package = None;
                SETTINGS.read().unwrap().save();
            }
        }

        (
            Gui {
                releases,
                file_path: flags.file_path,
                installing: Vec::default(),
                state: GuiState::new(),
                tab: Tab::Packages,
                theme: SETTINGS.read().unwrap().theme,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!(
            "BlenderLauncher{}",
            match self.releases.count_updates().0 {
                Some(count) => format!(
                    " - {} {} available!",
                    count,
                    if count == 1 { "update" } else { "updates" }
                ),
                None => String::new(),
            }
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PackageMessage(index, package_message) => {
                match iter::empty()
                    .chain(&mut self.releases.daily.iter_mut())
                    .chain(&mut self.releases.branched.iter_mut())
                    .chain(&mut self.releases.stable.iter_mut())
                    .chain(&mut self.releases.lts.iter_mut())
                    .chain(&mut self.releases.archived.iter_mut())
                    .collect::<Vec<_>>()
                    .get_mut(index)
                {
                    Some(package) => package.update(package_message),
                    None => unreachable!("Index out of bounds"),
                }
            }
            Message::Bookmark(package) => {
                match package.build {
                    Build::Daily(_) => {
                        match self
                            .releases
                            .daily
                            .iter_mut()
                            .find(|a_package| **a_package == package)
                        {
                            Some(found_package) => {
                                found_package.bookmarked = !found_package.bookmarked;
                                self.releases.daily.save();
                            }
                            None => {
                                unreachable!("Couldn't find daily package to bookmark");
                            }
                        }
                    }
                    Build::Branched(_) => {
                        match self
                            .releases
                            .branched
                            .iter_mut()
                            .find(|a_package| **a_package == package)
                        {
                            Some(found_package) => {
                                found_package.bookmarked = !found_package.bookmarked;
                                self.releases.branched.save();
                            }
                            None => {
                                unreachable!("Couldn't find branched package to bookmark");
                            }
                        }
                    }
                    Build::Stable => {
                        match self
                            .releases
                            .stable
                            .iter_mut()
                            .find(|a_package| **a_package == package)
                        {
                            Some(found_package) => {
                                found_package.bookmarked = !found_package.bookmarked;
                                self.releases.stable.save();
                            }
                            None => {
                                unreachable!("Couldn't find stable package to bookmark");
                            }
                        }
                    }
                    Build::Lts => {
                        match self
                            .releases
                            .lts
                            .iter_mut()
                            .find(|a_package| **a_package == package)
                        {
                            Some(found_package) => {
                                found_package.bookmarked = !found_package.bookmarked;
                                self.releases.lts.save();
                            }
                            None => {
                                unreachable!("Couldn't find LTS package to bookmark");
                            }
                        }
                    }
                    Build::Archived => {
                        match self
                            .releases
                            .archived
                            .iter_mut()
                            .find(|a_package| **a_package == package)
                        {
                            Some(found_package) => {
                                found_package.bookmarked = !found_package.bookmarked;
                                self.releases.archived.save();
                            }
                            None => {
                                unreachable!("Couldn't find archived package to bookmark");
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::TryToInstall(package) => {
                let message = match package.build {
                    Build::Daily(_) => {
                        if SETTINGS.read().unwrap().keep_only_latest_daily
                            && package.status != PackageStatus::Update
                            && self
                                .releases
                                .installed
                                .iter()
                                .find(|p| p.build == package.build)
                                .is_some()
                        {
                            "daily package of its build type"
                        } else {
                            ""
                        }
                    }
                    Build::Branched(_) => {
                        if SETTINGS.read().unwrap().keep_only_latest_branched
                            && package.status != PackageStatus::Update
                            && self
                                .releases
                                .installed
                                .iter()
                                .find(|p| p.build == package.build)
                                .is_some()
                        {
                            "branched package of its build type"
                        } else {
                            ""
                        }
                    }
                    Build::Stable => {
                        if SETTINGS.read().unwrap().keep_only_latest_stable
                            && package.status != PackageStatus::Update
                            && self
                                .releases
                                .installed
                                .iter()
                                .find(|p| p.build == package.build)
                                .is_some()
                        {
                            "stable package"
                        } else {
                            ""
                        }
                    }
                    Build::Lts => {
                        if SETTINGS.read().unwrap().keep_only_latest_lts
                            && package.status != PackageStatus::Update
                            && self
                                .releases
                                .installed
                                .iter()
                                .find(|p| p.build == package.build)
                                .is_some()
                        {
                            "LTS package"
                        } else {
                            ""
                        }
                    }
                    Build::Archived => "",
                };
                if message.is_empty() {
                    Command::perform(
                        Gui::check_availability(true, package),
                        Message::CheckAvailability,
                    )
                } else {
                    // TODO: Consider disabling the Install button instead of opening this msgbox.
                    msgbox::create(
                        "BlenderLauncher",
                        &format!("Can't install '{}' because the setting to keep only latest {} is enabled.", package.name, message),
                        msgbox::IconType::Info,
                    )
                    .unwrap();
                    Command::none()
                }
            }
            Message::CheckAvailability(tuple) => {
                let (available, for_install, package) = tuple;
                if available {
                    if for_install {
                        Command::perform(Gui::pass_package(package), Message::InstallPackage)
                    } else {
                        self.releases.sync();
                        Command::none()
                    }
                } else {
                    match package.build {
                        Build::Daily(_) => {
                            let index = self
                                .releases
                                .daily
                                .iter()
                                .position(|a_package| *a_package == package)
                                .unwrap();
                            self.releases.daily.remove(index);
                            self.releases.daily.save();
                        }
                        Build::Branched(_) => {
                            let index = self
                                .releases
                                .branched
                                .iter()
                                .position(|a_package| *a_package == package)
                                .unwrap();
                            self.releases.branched.remove(index);
                            self.releases.branched.save();
                        }
                        Build::Stable => {
                            let index = self
                                .releases
                                .stable
                                .iter()
                                .position(|a_package| *a_package == package)
                                .unwrap();
                            self.releases.stable.remove(index);
                            self.releases.stable.save();
                        }
                        Build::Lts => {
                            let index = self
                                .releases
                                .lts
                                .iter()
                                .position(|a_package| *a_package == package)
                                .unwrap();
                            self.releases.lts.remove(index);
                            self.releases.lts.save();
                        }
                        Build::Archived => {
                            let index = self
                                .releases
                                .archived
                                .iter()
                                .position(|a_package| *a_package == package)
                                .unwrap();
                            self.releases.archived.remove(index);
                            self.releases.archived.save();
                        }
                    }
                    if for_install {
                        msgbox::create(
                            "BlenderLauncher",
                            &format!("Package '{}' is no longer available.", package.name),
                            msgbox::IconType::Info,
                        )
                        .unwrap();
                    }
                    self.releases.sync();
                    Command::none()
                }
            }
            Message::InstallPackage(package) => {
                let (index, package) = iter::empty()
                    .chain(self.releases.daily.iter())
                    .chain(self.releases.branched.iter())
                    .chain(self.releases.stable.iter())
                    .chain(self.releases.lts.iter())
                    .chain(self.releases.archived.iter())
                    .enumerate()
                    .find(|(_, a_package)| **a_package == package)
                    .unwrap();
                self.installing.push(((*package).clone(), index));
                Command::none()
            }
            Message::PackageInstalled(package) => {
                let index = self
                    .installing
                    .iter()
                    .enumerate()
                    .find(|(_, (a_package, _))| *a_package == package)
                    .unwrap()
                    .0;
                self.installing.remove(index);
                self.releases.installed.fetch();
                self.releases.installed.update_default();
                self.releases.installed.remove_old_packages();
                self.releases.sync();
                Command::none()
            }
            Message::PackageRemoved(package) => {
                let default_package_option = SETTINGS.read().unwrap().default_package.clone();
                if let Some(default_package) = default_package_option {
                    if default_package == package {
                        SETTINGS.write().unwrap().default_package = None;
                        SETTINGS.read().unwrap().save();
                    }
                }
                Command::perform(
                    Gui::check_availability(false, package),
                    Message::CheckAvailability,
                )
            }
            Message::OpenBlender(package) => {
                open_blender(package.name, None);
                process::exit(0);
            }
            Message::OpenBlenderWithFile(package) => {
                open_blender(package.name, Some(self.file_path.clone().unwrap()));
                process::exit(0);
            }
            Message::CheckForUpdates => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_for_updates(self.releases.take()),
                    Message::UpdatesChecked,
                )
            }
            Message::UpdatesChecked(tuple) => {
                self.releases.add_new_packages(tuple);
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchAll => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_all(
                        self.releases.daily.take(),
                        self.releases.branched.take(),
                        self.releases.stable.take(),
                        self.releases.lts.take(),
                        self.releases.archived.take(),
                    ),
                    Message::AllFetched,
                )
            }
            Message::AllFetched((_, daily, branched, stable, lts, archived)) => {
                self.releases.daily = daily;
                self.releases.branched = branched;
                self.releases.stable = stable;
                self.releases.lts = lts;
                self.releases.archived = archived;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchDaily => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_daily(self.releases.daily.take()),
                    Message::DailyFetched,
                )
            }
            Message::DailyFetched((_, daily)) => {
                self.releases.daily = daily;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchBranched => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_branched(self.releases.branched.take()),
                    Message::BranchedFetched,
                )
            }
            Message::BranchedFetched((_, branched)) => {
                self.releases.branched = branched;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchStable => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_stable(self.releases.stable.take()),
                    Message::StableFetched,
                )
            }
            Message::StableFetched((_, stable)) => {
                self.releases.stable = stable;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchLts => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_lts(self.releases.lts.take()),
                    Message::LtsFetched,
                )
            }
            Message::LtsFetched((_, lts)) => {
                self.releases.lts = lts;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FetchArchived => {
                self.state.controls.checking_for_updates = true;
                Command::perform(
                    Gui::check_archived(self.releases.archived.take()),
                    Message::ArchivedFetched,
                )
            }
            Message::ArchivedFetched((_, archived)) => {
                self.releases.archived = archived;
                self.state.controls.checking_for_updates = false;
                Command::none()
            }
            Message::FilterUpdatesChanged(change) => {
                if change {
                    self.state.controls.filters.updates = true;
                    self.state.controls.filters.bookmarks = false;
                    self.state.controls.filters.installed = false;
                } else {
                    self.state.controls.filters.updates = false;
                }
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }

            Message::FilterBookmarksChanged(change) => {
                if change {
                    self.state.controls.filters.updates = false;
                    self.state.controls.filters.bookmarks = true;
                    self.state.controls.filters.installed = false;
                } else {
                    self.state.controls.filters.bookmarks = false;
                }
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterInstalledChanged(change) => {
                if change {
                    self.state.controls.filters.updates = false;
                    self.state.controls.filters.bookmarks = false;
                    self.state.controls.filters.installed = true;
                } else {
                    self.state.controls.filters.installed = false;
                }
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterAllChanged(change) => {
                self.state.controls.filters.all = change;
                self.state.controls.filters.daily = change;
                self.state.controls.filters.branched = change;
                self.state.controls.filters.stable = change;
                self.state.controls.filters.lts = change;
                self.state.controls.filters.archived = change;
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterDailyChanged(change) => {
                self.state.controls.filters.daily = change;
                self.state.controls.filters.refresh_all();
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterBranchedChanged(change) => {
                self.state.controls.filters.branched = change;
                self.state.controls.filters.refresh_all();
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterStableChanged(change) => {
                self.state.controls.filters.stable = change;
                self.state.controls.filters.refresh_all();
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterLtsChanged(change) => {
                self.state.controls.filters.lts = change;
                self.state.controls.filters.refresh_all();
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FilterArchivedChanged(change) => {
                self.state.controls.filters.archived = change;
                self.state.controls.filters.refresh_all();
                SETTINGS.write().unwrap().filters = self.state.controls.filters;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::SortingChanged(sort_by) => {
                self.state.controls.sort_by = sort_by;
                SETTINGS.write().unwrap().sort_by = sort_by;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::TabChanged(tab) => {
                self.tab = tab;
                Command::none()
            }
            Message::BypassLauncher(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().bypass_launcher = true,
                    Choice::Disable => SETTINGS.write().unwrap().bypass_launcher = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::ModifierKey(modifier_key) => {
                SETTINGS.write().unwrap().modifier_key = modifier_key;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::UseLatestAsDefault(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().use_latest_as_default = true,
                    Choice::Disable => SETTINGS.write().unwrap().use_latest_as_default = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::CheckUpdatesAtLaunch(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().check_updates_at_launch = true,
                    Choice::Disable => SETTINGS.write().unwrap().check_updates_at_launch = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::MinutesBetweenUpdatesChanged(minutes) => {
                self.state.minute_value = minutes;
                Command::none()
            }
            Message::SaveMinutesBetweenUpdates(minutes) => {
                SETTINGS.write().unwrap().minutes_between_updates = minutes as u64;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::UpdateDaily(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_daily = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_daily = false,
                }
                SETTINGS.read().unwrap().save();
                self.releases.sync();
                Command::none()
            }
            Message::UpdateBranched(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_branched = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_branched = false,
                }
                SETTINGS.read().unwrap().save();
                self.releases.sync();
                Command::none()
            }
            Message::UpdateStable(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_stable = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_stable = false,
                }
                SETTINGS.read().unwrap().save();
                self.releases.sync();
                Command::none()
            }
            Message::UpdateLts(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_lts = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_lts = false,
                }
                SETTINGS.read().unwrap().save();
                self.releases.sync();
                Command::none()
            }
            Message::KeepOnlyLatestDaily(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().keep_only_latest_daily = true,
                    Choice::Disable => SETTINGS.write().unwrap().keep_only_latest_daily = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::KeepOnlyLatestBranched(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().keep_only_latest_branched = true,
                    Choice::Disable => SETTINGS.write().unwrap().keep_only_latest_branched = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::KeepOnlyLatestStable(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().keep_only_latest_stable = true,
                    Choice::Disable => SETTINGS.write().unwrap().keep_only_latest_stable = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::KeepOnlyLatestLts(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().keep_only_latest_lts = true,
                    Choice::Disable => SETTINGS.write().unwrap().keep_only_latest_lts = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::ThemeChanged(theme) => {
                self.theme = theme;
                SETTINGS.write().unwrap().theme = theme;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            self.installing
                .iter()
                .map(|(package, index)| Install::package(package.to_owned(), index.to_owned())),
        )
    }

    fn view(&mut self) -> Element<'_, Message> {
        let file_exists = self.file_path.is_some();
        let self_tab = self.tab;
        let filters = self.state.controls.filters;
        let sort_by = self.state.controls.sort_by;
        let theme = self.theme;
        let update_count = self.releases.count_updates();

        let tab_button = |label, tab, state| {
            let button = Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Units(100))
            .style(theme.tab_button());

            if tab == self_tab {
                button
            } else {
                button.on_press(Message::TabChanged(tab))
            }
        };

        let tabs = Container::new(
            Row::new()
                .padding(2)
                .spacing(2)
                .push(tab_button(
                    "Packages",
                    Tab::Packages,
                    &mut self.state.packages_button,
                ))
                .push(tab_button(
                    "Settings",
                    Tab::Settings,
                    &mut self.state.settings_button,
                ))
                .push(tab_button(
                    "About",
                    Tab::About,
                    &mut self.state.about_button,
                )),
        )
        .width(Length::Fill)
        .center_x()
        .style(self.theme.tab_container());

        let body: Element<'_, Message> = match self.tab {
            Tab::Packages => {
                // TODO: Use icons for the buttons.
                // I could use just the icon here without text if the same icon is used
                // on the packages but accompanied with text to teach the user what they represent.
                // Tooltips would be nice too, if `iced` finally implements them.
                let button = |label, package_message: Option<Message>, state| {
                    let button = Button::new(state, Text::new(label)).style(theme);

                    if package_message.is_some() {
                        button.on_press(package_message.unwrap())
                    } else {
                        button
                    }
                };

                let info: Element<'_, Message> = Container::new(
                    Column::new()
                        .padding(10)
                        .spacing(5)
                        .push(
                            Row::new()
                                .spacing(10)
                                .align_items(Align::Center)
                                .push(button(
                                    "[=]",
                                    match SETTINGS.read().unwrap().default_package.clone() {
                                        Some(package) => Some(Message::OpenBlender(package)),
                                        None => None,
                                    },
                                    &mut self.state.open_default_button,
                                ))
                                .push(Text::new("Default package:"))
                                .push(
                                    Text::new(
                                        match SETTINGS.read().unwrap().default_package.clone() {
                                            Some(package) => {
                                                format!("{}", package.name)
                                            }
                                            None => String::from("not set"),
                                        },
                                    )
                                    .color(theme.highlight_text()),
                                ),
                        )
                        .push(
                            Row::new()
                                .spacing(10)
                                .align_items(Align::Center)
                                .push(button(
                                    "[+]",
                                    if self.file_path.is_some()
                                        && SETTINGS.read().unwrap().default_package.is_some()
                                    {
                                        Some(Message::OpenBlenderWithFile(
                                            SETTINGS
                                                .read()
                                                .unwrap()
                                                .default_package
                                                .clone()
                                                .unwrap(),
                                        ))
                                    } else {
                                        None
                                    },
                                    &mut self.state.open_default_with_file_button,
                                ))
                                .push(Text::new("File:"))
                                .push(
                                    Text::new(match &self.file_path {
                                        Some(file_path) => {
                                            format!("{}", file_path)
                                        }
                                        None => String::from("none"),
                                    })
                                    .color(theme.highlight_text()),
                                ),
                        ),
                )
                .width(Length::Fill)
                .style(self.theme.info_container())
                .into();

                let packages: Element<'_, Message> = {
                    let mut package_count: u16 = 0;
                    let filtered_packages =
                        Container::new(
                            iter::empty()
                                .chain(&mut self.releases.daily.iter_mut())
                                .chain(&mut self.releases.branched.iter_mut())
                                .chain(&mut self.releases.stable.iter_mut())
                                .chain(&mut self.releases.lts.iter_mut())
                                .chain(&mut self.releases.archived.iter_mut())
                                .enumerate()
                                .sorted_by(|a, b| sort_by.get_ordering(&a.1, &b.1))
                                .filter(|(_, package)| filters.matches(package))
                                .fold(Column::new(), |col, (index, package)| {
                                    package_count += 1;
                                    let element =
                                        package.view(file_exists, theme, package_count & 1 != 0);
                                    col.push(element.map(move |message| {
                                        Message::PackageMessage(index, message)
                                    }))
                                })
                                .width(Length::Fill),
                        );

                    let scrollable =
                        Scrollable::new(&mut self.state.packages_scroll).push(filtered_packages);

                    if package_count == 0 {
                        Container::new(
                            Text::new(if self.state.controls.checking_for_updates {
                                "Checking for updates..."
                            } else {
                                "No packages"
                            })
                            .size(TEXT_SIZE * 2),
                        )
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .center_x()
                        .center_y()
                        .style(self.theme)
                        .into()
                    } else {
                        Container::new(scrollable)
                            .height(Length::Fill)
                            .width(Length::Fill)
                            .style(self.theme)
                            .into()
                    }
                };

                Container::new(
                    Column::new().push(info).push(
                        Row::new()
                            .push(self.state.controls.view(update_count, theme))
                            .push(packages),
                    ),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .style(theme)
                .into()
            }
            Tab::Settings => {
                macro_rules! choice_setting {
                    ($title:expr, $description:expr, &$array:expr, $option:expr, $message:expr,) => {
                        Row::new()
                            .spacing(40)
                            .align_items(Align::Center)
                            .push(
                                Column::new()
                                    .spacing(10)
                                    .width(Length::Fill)
                                    .push(
                                        Text::new($title)
                                            .color(theme.highlight_text())
                                            .size(TEXT_SIZE * 2),
                                    )
                                    .push(Text::new($description)),
                            )
                            .push($array.iter().fold(
                                Column::new().spacing(10).width(Length::Units(110)),
                                |col, value| {
                                    col.push(
                                        Radio::new(
                                            *value,
                                            &format!("{:?}", value),
                                            $option,
                                            $message,
                                        )
                                        .style(theme),
                                    )
                                },
                            ))
                    };
                }

                let choice = |flag| match flag {
                    true => Some(Choice::Enable),
                    false => Some(Choice::Disable),
                };

                // TODO: Rewrite descriptions to better explain the behaviour of checking for updates.
                // Maybe try to group settings with a general description about them.
                let settings = Column::new()
                    .padding(10)
                    .push(
                        choice_setting!(
                            "Bypass launcher",
                            "If a default package is set and no updates were found, only open launcher when the selected modifier key is held down.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().bypass_launcher).unwrap()),
                            Message::BypassLauncher,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Modifier key",
                            "Change the modifier key if there's any interference when opening the launcher or a .blend file while holding it down.",
                            &ModifierKey::ALL,
                            Some(SETTINGS.read().unwrap().modifier_key),
                            Message::ModifierKey,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Use latest as default",
                            "Change to the latest package of the same build type when updating.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().use_latest_as_default).unwrap()),
                            Message::UseLatestAsDefault,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check updates at launch",
                            "Increases launch time for about a second or two. Having a delay between checks improves launch speed.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().check_updates_at_launch).unwrap()),
                            Message::CheckUpdatesAtLaunch,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(Column::new()
                        .spacing(10)
                        .push(Text::new("Delay between update checking")
                            .color(theme.highlight_text())
                            .size(TEXT_SIZE * 2))
                        .push(Text::new("Minutes to wait between update checks. Setting it to 0 will make it check every time. Maximum is 24 hours."))
                        .push(Row::new()
                            .push(Text::new(format!("Current: {}", self.state.minute_value)).width(Length::Units(130)))
                            .push(Slider::new(
                                &mut self.state.minute_slider,
                                0.0..=1440.0,
                                self.state.minute_value,
                                Message::MinutesBetweenUpdatesChanged)
                                    .on_release(Message::SaveMinutesBetweenUpdates(self.state.minute_value))
                                    .style(self.theme)))
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for daily packages",
                            "When updating, check for new daily packages. This setting also affects whether the newest daily package is counted as an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_daily).unwrap()),
                            Message::UpdateDaily,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for branched packages",
                            "When updating, check for new branched packages. This setting also affects whether the newest branched package is counted as an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_branched).unwrap()),
                            Message::UpdateBranched,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for stable packages",
                            "When updating, check for new stable packages. This setting also affects whether the newest stable package is counted as an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_stable).unwrap()),
                            Message::UpdateStable,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for LTS packages",
                            "When updating, check for new LTS packages. This setting also affects whether the newest LTS package is counted as an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_lts).unwrap()),
                            Message::UpdateLts,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest daily package",
                            "Remove all older daily packages of its build type when installing an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_daily).unwrap()),
                            Message::KeepOnlyLatestDaily,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest branched package",
                            "Remove all older branched packages of its build type when installing an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_branched).unwrap()),
                            Message::KeepOnlyLatestBranched,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest stable package",
                           "Remove all older stable packages when installing an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_stable).unwrap()),
                            Message::KeepOnlyLatestStable,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest LTS package",
                            "Remove all older LTS packages when installing an update.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_lts).unwrap()),
                            Message::KeepOnlyLatestLts,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Choose the theme",
                            "Both are simple light and dark colour schemes.",
                            &Theme::ALL,
                            Some(theme),
                            Message::ThemeChanged,
                        )
                    );

                Container::new(Scrollable::new(&mut self.state.settings_scroll).push(settings))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(self.theme)
                    .into()
            }
            Tab::About => Container::new(
                Text::new("About tab not yet implemented")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .width(Length::Fill)
                    .size(TEXT_SIZE * 2),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .style(theme)
            .into(),
        };

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Column::new().push(tabs).push(body).into()
        } else {
            Column::new()
                .push(tabs)
                .push(body)
                .push(
                    // TODO: Add button for checking connection.
                    Container::new(
                        Container::new(Text::new("CANNOT CONNECT").size(TEXT_SIZE - 5)).padding(2),
                    )
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .center_x()
                    .center_y()
                    .style(self.theme.status_container()),
                )
                .into()
        }
    }
}

pub struct GlobalTokio;

impl Executor for GlobalTokio {
    fn new() -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        tokio::task::spawn(future);
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    PackageMessage(usize, PackageMessage),
    Bookmark(Package),
    TryToInstall(Package),
    CheckAvailability((bool, bool, Package)),
    InstallPackage(Package),
    PackageInstalled(Package),
    PackageRemoved(Package),
    OpenBlender(Package),
    OpenBlenderWithFile(Package),
    CheckForUpdates,
    UpdatesChecked((bool, Daily, Branched, Stable, Lts)),
    FetchAll,
    AllFetched((bool, Daily, Branched, Stable, Lts, Archived)),
    FetchDaily,
    DailyFetched((bool, Daily)),
    FetchBranched,
    BranchedFetched((bool, Branched)),
    FetchStable,
    StableFetched((bool, Stable)),
    FetchLts,
    LtsFetched((bool, Lts)),
    FetchArchived,
    ArchivedFetched((bool, Archived)),
    FilterUpdatesChanged(bool),
    FilterBookmarksChanged(bool),
    FilterInstalledChanged(bool),
    FilterAllChanged(bool),
    FilterDailyChanged(bool),
    FilterBranchedChanged(bool),
    FilterStableChanged(bool),
    FilterLtsChanged(bool),
    FilterArchivedChanged(bool),
    SortingChanged(SortBy),
    TabChanged(Tab),
    BypassLauncher(Choice),
    ModifierKey(ModifierKey),
    UseLatestAsDefault(Choice),
    CheckUpdatesAtLaunch(Choice),
    MinutesBetweenUpdatesChanged(f64),
    SaveMinutesBetweenUpdates(f64),
    UpdateDaily(Choice),
    UpdateBranched(Choice),
    UpdateStable(Choice),
    UpdateLts(Choice),
    KeepOnlyLatestDaily(Choice),
    KeepOnlyLatestBranched(Choice),
    KeepOnlyLatestStable(Choice),
    KeepOnlyLatestLts(Choice),
    ThemeChanged(Theme),
}

#[derive(Debug)]
pub struct GuiFlags {
    pub releases: Releases,
    pub file_path: Option<String>,
}

#[derive(Debug, Default)]
struct GuiState {
    controls: Controls,
    packages_scroll: scrollable::State,
    settings_scroll: scrollable::State,
    about_scroll: scrollable::State,
    open_default_button: button::State,
    open_default_with_file_button: button::State,
    packages_button: button::State,
    settings_button: button::State,
    about_button: button::State,
    minute_slider: slider::State,
    minute_value: f64,
}

impl GuiState {
    fn new() -> Self {
        Self {
            controls: Controls {
                filters: SETTINGS.read().unwrap().filters,
                sort_by: SETTINGS.read().unwrap().sort_by,
                ..Controls::default()
            },
            minute_value: SETTINGS.read().unwrap().minutes_between_updates as f64,
            ..Self::default()
        }
    }
}

#[derive(Debug, Default)]
struct Controls {
    check_for_updates_button: button::State,
    checking_for_updates: bool,
    filters: Filters,
    fetch_all_button: button::State,
    fetch_daily_button: button::State,
    fetch_branched_button: button::State,
    fetch_stable_button: button::State,
    fetch_lts_button: button::State,
    fetch_archived_button: button::State,
    sort_by: SortBy,
    sorting_pick_list: pick_list::State<SortBy>,
    scroll: scrollable::State,
}

impl Controls {
    fn view(
        &mut self,
        update_count: (
            Option<usize>,
            Option<usize>,
            Option<usize>,
            Option<usize>,
            Option<usize>,
        ),
        theme: Theme,
    ) -> Container<'_, Message> {
        let checking_for_updates = self.checking_for_updates;

        let update_button = {
            let button = Button::new(
                &mut self.check_for_updates_button,
                Text::new("[O] Check for updates"),
            )
            .style(theme);

            if CAN_CONNECT.load(Ordering::Relaxed) && !checking_for_updates {
                button.on_press(Message::CheckForUpdates)
            } else {
                button
            }
        };

        let filter_row = |filter,
                          label,
                          checkbox_message: fn(bool) -> Message,
                          state,
                          button_message: Option<Message>| {
            let row = Row::new()
                .height(Length::Units(30))
                .align_items(Align::Center)
                .push(
                    Checkbox::new(filter, label, checkbox_message)
                        .width(Length::Fill)
                        .style(theme),
                );
            match state {
                Some(state) => {
                    let button = Button::new(state, Text::new("[O]")).style(theme);

                    if button_message.is_some()
                        && CAN_CONNECT.load(Ordering::Relaxed)
                        && !checking_for_updates
                    {
                        row.push(button.on_press(button_message.unwrap()))
                    } else {
                        row.push(button)
                    }
                }
                None => row,
            }
        };

        let filters = Column::new()
            .spacing(5)
            .push(Text::new("Filters"))
            .push(filter_row(
                self.filters.updates,
                match update_count.0 {
                    Some(count) => {
                        format!("Updates [{}]", count)
                    }
                    None => String::from("Updates"),
                },
                Message::FilterUpdatesChanged,
                None,
                None,
            ))
            .push(filter_row(
                self.filters.bookmarks,
                String::from("Bookmarks"),
                Message::FilterBookmarksChanged,
                None,
                None,
            ))
            .push(filter_row(
                self.filters.installed,
                String::from("Installed"),
                Message::FilterInstalledChanged,
                None,
                None,
            ))
            .push(Rule::horizontal(5).style(theme))
            .push(filter_row(
                self.filters.all,
                String::from("All"),
                Message::FilterAllChanged,
                Some(&mut self.fetch_all_button),
                Some(Message::FetchAll),
            ))
            .push(filter_row(
                self.filters.daily,
                match update_count.1 {
                    Some(count) => {
                        format!("Daily [{}]", count)
                    }
                    None => String::from("Daily"),
                },
                Message::FilterDailyChanged,
                Some(&mut self.fetch_daily_button),
                Some(Message::FetchDaily),
            ))
            .push(filter_row(
                self.filters.branched,
                match update_count.2 {
                    Some(count) => {
                        format!("Branched [{}]", count)
                    }
                    None => String::from("Branched"),
                },
                Message::FilterBranchedChanged,
                Some(&mut self.fetch_branched_button),
                Some(Message::FetchBranched),
            ))
            .push(filter_row(
                self.filters.stable,
                match update_count.3 {
                    Some(count) => {
                        format!("Stable [{}]", count)
                    }
                    None => String::from("Stable"),
                },
                Message::FilterStableChanged,
                Some(&mut self.fetch_stable_button),
                Some(Message::FetchStable),
            ))
            .push(filter_row(
                self.filters.lts,
                match update_count.4 {
                    Some(count) => {
                        format!("LTS [{}]", count)
                    }
                    None => String::from("LTS"),
                },
                Message::FilterLtsChanged,
                Some(&mut self.fetch_lts_button),
                Some(Message::FetchLts),
            ))
            .push(filter_row(
                self.filters.archived,
                String::from("Archived"),
                Message::FilterArchivedChanged,
                Some(&mut self.fetch_archived_button),
                Some(Message::FetchArchived),
            ));

        let sorting = Row::new()
            .spacing(8)
            .align_items(Align::Center)
            .push(Text::new("Sort by"))
            .push(
                PickList::new(
                    &mut self.sorting_pick_list,
                    &SortBy::ALL[..],
                    Some(SETTINGS.read().unwrap().sort_by),
                    Message::SortingChanged,
                )
                .width(Length::Fill)
                .style(theme),
            );

        let scrollable = Scrollable::new(&mut self.scroll).push(
            Column::new()
                .spacing(10)
                .padding(10)
                .align_items(Align::Center)
                .push(update_button)
                .push(filters)
                .push(sorting),
        );

        Container::new(scrollable)
            // TODO: Can't get it to shrink aroind its content for some reason.
            // It always fills the whole space unless I set a specific width.
            .width(Length::Units(190))
            .height(Length::Fill)
            .style(theme.sidebar_container())
            .into()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Filters {
    updates: bool,
    bookmarks: bool,
    installed: bool,
    all: bool,
    daily: bool,
    branched: bool,
    lts: bool,
    stable: bool,
    archived: bool,
}

impl Filters {
    fn matches(&self, package: &Package) -> bool {
        if self.updates {
            match package.build {
                Build::Daily(_) if self.daily && package.status == PackageStatus::Update => true,
                Build::Branched(_) if self.branched && package.status == PackageStatus::Update => {
                    true
                }
                Build::Stable if self.stable && package.status == PackageStatus::Update => true,
                Build::Lts if self.lts && package.status == PackageStatus::Update => true,
                Build::Archived if self.archived && package.status == PackageStatus::Update => true,
                _ => false,
            }
        } else if self.bookmarks {
            match package.build {
                Build::Daily(_) if self.daily && package.bookmarked => true,
                Build::Branched(_) if self.branched && package.bookmarked => true,
                Build::Stable if self.stable && package.bookmarked => true,
                Build::Lts if self.lts && package.bookmarked => true,
                Build::Archived if self.archived && package.bookmarked => true,
                _ => false,
            }
        } else if self.installed {
            match package.build {
                Build::Daily(_)
                    if self.daily && matches!(package.state, PackageState::Installed { .. }) =>
                {
                    true
                }
                Build::Branched(_)
                    if self.branched && matches!(package.state, PackageState::Installed { .. }) =>
                {
                    true
                }
                Build::Stable
                    if self.stable && matches!(package.state, PackageState::Installed { .. }) =>
                {
                    true
                }
                Build::Lts
                    if self.lts && matches!(package.state, PackageState::Installed { .. }) =>
                {
                    true
                }
                Build::Archived
                    if self.archived && matches!(package.state, PackageState::Installed { .. }) =>
                {
                    true
                }
                _ => false,
            }
        } else {
            match package.build {
                Build::Daily(_) if self.daily => true,
                Build::Branched(_) if self.branched => true,
                Build::Stable if self.stable => true,
                Build::Lts if self.lts => true,
                Build::Archived if self.archived => true,
                _ => false,
            }
        }
    }

    fn refresh_all(&mut self) {
        if self.daily && self.branched && self.stable && self.lts && self.archived {
            self.all = true
        } else {
            self.all = false
        }
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            updates: false,
            bookmarks: false,
            installed: false,
            all: true,
            daily: true,
            branched: true,
            lts: true,
            stable: true,
            archived: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Deserialize, PartialEq, Serialize)]
pub enum SortBy {
    NameAscending,
    NameDescending,
    DateAscending,
    DateDescending,
    VersionAscending,
    VersionDescending,
}

impl SortBy {
    const ALL: [SortBy; 6] = [
        SortBy::NameAscending,
        SortBy::NameDescending,
        SortBy::DateAscending,
        SortBy::DateDescending,
        SortBy::VersionAscending,
        SortBy::VersionDescending,
    ];

    fn get_ordering(&self, a: &Package, b: &Package) -> std::cmp::Ordering {
        match self {
            SortBy::NameAscending => Ord::cmp(&a.name, &b.name),
            SortBy::NameDescending => Ord::cmp(&a.name, &b.name).reverse(),
            SortBy::DateAscending => Ord::cmp(&a.date, &b.date),
            SortBy::DateDescending => Ord::cmp(&a.date, &b.date).reverse(),
            SortBy::VersionAscending => natord::compare_ignore_case(&a.version, &b.version),
            SortBy::VersionDescending => {
                natord::compare_ignore_case(&a.version, &b.version).reverse()
            }
        }
    }
}

impl Default for SortBy {
    fn default() -> Self {
        Self::VersionDescending
    }
}

impl Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SortBy::NameAscending => " Name [A]",
                SortBy::NameDescending => " Name [D]",
                SortBy::DateAscending => " Date [A]",
                SortBy::DateDescending => " Date [D]",
                SortBy::VersionAscending => " Version [A]",
                SortBy::VersionDescending => " Version [D]",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tab {
    Packages,
    Settings,
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Enable,
    Disable,
}

impl Choice {
    const ALL: [Choice; 2] = [Choice::Enable, Choice::Disable];
}

#[derive(Clone, Debug)]
pub enum PackageMessage {
    Install,
    InstallationProgress(Progress),
    Remove,
    OpenBlender,
    OpenBlenderWithFile,
    SetDefault,
    UnsetDefault,
    Bookmark,
}

impl Package {
    fn update(&mut self, message: PackageMessage) -> Command<Message> {
        match message {
            PackageMessage::Install => {
                Command::perform(Gui::pass_package(self.clone()), Message::TryToInstall)
            }
            PackageMessage::InstallationProgress(progress) => match progress {
                Progress::Started => {
                    self.state = PackageState::Downloading { progress: 0.0 };
                    Command::none()
                }
                Progress::DownloadProgress(progress) => {
                    self.state = PackageState::Downloading { progress };
                    Command::none()
                }
                Progress::FinishedDownloading => Command::none(),
                Progress::ExtractionProgress(progress) => {
                    self.state = PackageState::Extracting { progress };
                    Command::none()
                }
                Progress::FinishedExtracting => Command::none(),
                Progress::FinishedInstalling => {
                    self.state = PackageState::Installed {
                        open_button: Default::default(),
                        open_file_button: Default::default(),
                        set_default_button: Default::default(),
                        remove_button: Default::default(),
                    };
                    Command::perform(Gui::pass_package(self.clone()), Message::PackageInstalled)
                }
                Progress::Errored => {
                    self.state = PackageState::Errored {
                        retry_button: Default::default(),
                    };
                    Command::none()
                }
            },
            PackageMessage::Remove => {
                self.remove();
                Command::perform(Gui::pass_package(self.clone()), Message::PackageRemoved)
            }
            PackageMessage::OpenBlender => {
                Command::perform(Gui::pass_package(self.clone()), Message::OpenBlender)
            }

            PackageMessage::OpenBlenderWithFile => Command::perform(
                Gui::pass_package(self.clone()),
                Message::OpenBlenderWithFile,
            ),
            PackageMessage::SetDefault => {
                SETTINGS.write().unwrap().default_package = Some(self.clone());
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            PackageMessage::UnsetDefault => {
                SETTINGS.write().unwrap().default_package = None;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            PackageMessage::Bookmark => {
                Command::perform(Gui::pass_package(self.clone()), Message::Bookmark)
            }
        }
    }

    fn view(
        &mut self,
        file_exists: bool,
        theme: Theme,
        is_odd: bool,
    ) -> Element<'_, PackageMessage> {
        let is_default_package = SETTINGS.read().unwrap().default_package.is_some()
            && SETTINGS.read().unwrap().default_package.clone().unwrap() == *self;

        let date_time = self.get_formatted_date_time();

        let name = Row::new()
            .spacing(10)
            .push(
                Text::new(&self.name)
                    .color(theme.highlight_text())
                    .size(TEXT_SIZE + 10)
                    .width(Length::Fill),
            )
            .push(
                Button::new(
                    &mut self.bookmark_button,
                    Text::new(if self.bookmarked { "[B]" } else { "[M]" }),
                )
                .on_press(PackageMessage::Bookmark)
                .style(theme),
            );

        let details = Column::new()
            .push(
                Row::new()
                    .align_items(Align::End)
                    .push(Text::new("Date: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(date_time)
                            .color(theme.highlight_text())
                            .width(Length::Fill),
                    ),
            )
            .push(
                Row::new()
                    .push(
                        Row::new()
                            .width(Length::Fill)
                            .align_items(Align::End)
                            .push(Text::new("Version: ").size(TEXT_SIZE - 4))
                            .push(Text::new(&self.version).color(theme.highlight_text())),
                    )
                    .push(
                        Text::new(match self.status {
                            PackageStatus::Update => "UPDATE   ",
                            PackageStatus::New => "NEW   ",
                            PackageStatus::Old => "",
                        })
                        .color(theme.highlight_text())
                        .size(TEXT_SIZE + 4),
                    ),
            )
            .push(
                Row::new()
                    .align_items(Align::End)
                    .push(Text::new("Build: ").size(TEXT_SIZE - 4))
                    .push(Text::new(self.build.to_string()).color(theme.highlight_text())),
            );

        let button = |label, package_message: Option<PackageMessage>, state| {
            let button = Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(theme);

            if package_message.is_some() {
                button.on_press(package_message.unwrap())
            } else {
                button
            }
        };

        let controls: Element<'_, PackageMessage> = match &mut self.state {
            PackageState::Fetched { install_button } => Row::new()
                .push(button(
                    "[#] Install",
                    if CAN_CONNECT.load(Ordering::Relaxed) {
                        Some(PackageMessage::Install)
                    } else {
                        None
                    },
                    install_button,
                ))
                .into(),
            PackageState::Downloading { progress } => Row::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(Text::new(format!("Downloading... {:.2}%", progress)))
                .push(
                    ProgressBar::new(0.0..=100.0, *progress)
                        .width(Length::Fill)
                        .style(theme),
                )
                // TODO: Cancel functionality.
                .into(),
            PackageState::Extracting { progress } => {
                if cfg!(target_os = "linux") {
                    Row::new()
                        .align_items(Align::Center)
                        .push(
                            Text::new(format!("Extracting..."))
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .into()
                } else {
                    Row::new()
                        .spacing(10)
                        .align_items(Align::Center)
                        .push(Text::new(format!("Extracting... {:.2}%", progress)))
                        .push(
                            ProgressBar::new(0.0..=100.0, *progress)
                                .width(Length::Fill)
                                .style(theme),
                        )
                        .into()
                }
            }
            PackageState::Installed {
                open_button,
                open_file_button,
                set_default_button,
                remove_button,
            } => {
                let button1 = Row::new().push(button(
                    "[=] Open",
                    Some(PackageMessage::OpenBlender),
                    open_button,
                ));

                let button2 = button1.push(button(
                    "[+] Open file",
                    if file_exists {
                        Some(PackageMessage::OpenBlenderWithFile)
                    } else {
                        None
                    },
                    open_file_button,
                ));

                let button3 = button2.push(button(
                    if is_default_package {
                        "[U] Unset"
                    } else {
                        "[S] Set"
                    },
                    if is_default_package {
                        Some(PackageMessage::UnsetDefault)
                    } else {
                        Some(PackageMessage::SetDefault)
                    },
                    set_default_button,
                ));

                button3
                    .spacing(10)
                    .push(button(
                        "[X] Uninstall",
                        Some(PackageMessage::Remove),
                        remove_button,
                    ))
                    .into()
            }
            // TODO: Retry functionality.
            PackageState::Errored { retry_button: _ } => Text::new("Error").into(),
        };

        Container::new(
            Column::new().spacing(10).push(name).push(details).push(
                Container::new(controls)
                    .height(Length::Units(40))
                    .center_y(),
            ),
        )
        .style({
            if is_odd {
                theme.odd_container()
            } else {
                theme.even_container()
            }
        })
        .padding(10)
        .into()
    }
}
