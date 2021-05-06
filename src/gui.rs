//#![allow(dead_code, unused_imports, unused_variables)]
mod install;
pub mod style;
use self::{
    install::{Install, Progress},
    style::Theme,
};
use crate::{
    helpers::{
        change_self_version, check_connection, check_self_updates, get_self_releases, open_blender,
    },
    package::{Build, Package, PackageState, PackageStatus},
    releases::{
        archived::Archived, branched::Branched, daily::Daily, lts::Lts, stable::Stable,
        ReleaseType, Releases,
    },
    settings::{
        ModifierKey, CAN_CONNECT, CONFIG_FILE_ENV, PORTABLE, PROJECT_DIRS, SETTINGS, TEXT_SIZE,
    },
};
use clap::crate_version;
use fs2::available_space;
use fs_extra::dir;
use iced::{
    button, pick_list, scrollable, Align, Application, Button, Checkbox, Clipboard, Column,
    Command, Container, Element, Executor, HorizontalAlignment, Length, PickList, ProgressBar,
    Radio, Row, Rule, Scrollable, Space, Subscription, Text,
};
use itertools::Itertools;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use reqwest;
use self_update::update::Release;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    iter, process,
    sync::atomic::Ordering,
};
use webbrowser;

#[derive(Debug)]
pub struct Gui {
    releases: Releases,
    file_path: Option<String>,
    installing: Vec<(Package, usize)>,
    state: GuiState,
    tab: Tab,
    theme: Theme,
    self_releases: Option<Vec<Release>>,
}

impl Gui {
    /// A tuple is returned where:
    /// (true_if_available, true_if_for_install, package)
    async fn check_availability(
        for_install: bool,
        package: Package,
    ) -> Option<(bool, bool, Package)> {
        match reqwest::get(&package.url).await {
            Ok(response) => {
                if response.status().is_client_error() {
                    Some((false, for_install, package))
                } else {
                    Some((true, for_install, package))
                }
            }
            Err(_) => {
                CAN_CONNECT.store(false, Ordering::Relaxed);
                None
            }
        }
    }

    async fn pass_package(package: Package) -> Package {
        package
    }

    async fn check_for_updates(
        packages: (Daily, Branched, Stable, Lts),
    ) -> (bool, Daily, Branched, Stable, Lts) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_updates(packages).await
        } else {
            (false, packages.0, packages.1, packages.2, packages.3)
        }
    }

    async fn check_all(
        daily: Daily,
        branched: Branched,
        stable: Stable,
        lts: Lts,
        archived: Archived,
    ) -> (bool, Daily, Branched, Stable, Lts, Archived) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            (
                true,
                Releases::check_daily_updates(daily).await.1,
                Releases::check_branched_updates(branched).await.1,
                Releases::check_stable_updates(stable).await.1,
                Releases::check_lts_updates(lts).await.1,
                Releases::check_archived_updates(archived).await.1,
            )
        } else {
            (false, daily, branched, stable, lts, archived)
        }
    }

    async fn check_daily(packages: Daily) -> (bool, Daily) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_daily_updates(packages).await
        } else {
            (false, packages)
        }
    }

    async fn check_branched(packages: Branched) -> (bool, Branched) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_branched_updates(packages).await
        } else {
            (false, packages)
        }
    }

    async fn check_stable(packages: Stable) -> (bool, Stable) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_stable_updates(packages).await
        } else {
            (false, packages)
        }
    }

    async fn check_lts(packages: Lts) -> (bool, Lts) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_lts_updates(packages).await
        } else {
            (false, packages)
        }
    }

    async fn check_archived(packages: Archived) -> (bool, Archived) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_archived_updates(packages).await
        } else {
            (false, packages)
        }
    }

    async fn check_connection() {
        check_connection().await;
    }

    async fn change_self_version(releases: Vec<Release>, version: String) {
        change_self_version(releases, version);
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

        let mut state = GuiState::new();

        let self_releases = flags.self_releases;

        if let Some(s_releases) = &self_releases {
            state.release_versions = s_releases
                .iter()
                .map(|release| release.version.clone())
                .collect();
        }

        (
            Gui {
                releases,
                file_path: flags.file_path,
                installing: Vec::default(),
                state,
                // TODO: Save tab in user settings.
                // Will be useful when the recent files tab is introduced.
                tab: Tab::Packages,
                theme: SETTINGS.read().unwrap().theme,
                self_releases,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        match self.releases.count_updates().0 {
            Some(count) => format!(
                "Ablavema - {} update{} available!",
                count,
                if count > 1 { "s" } else { "" }
            ),
            None => String::from("Ablavema"),
        }
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
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
                    // TODO: Consider disabling the Install button instead of opening this MessageDialog.
                    // And show a tooltip explaining why it's disabled.
                    let message = format!(
                        "Can't install '{}' because the setting to keep only latest {} is enabled.",
                        package.name, message
                    );
                    if let Err(_) = MessageDialog::new()
                        .set_type(MessageType::Info)
                        .set_title("Ablavema")
                        .set_text(&message)
                        .show_alert()
                    {
                        if cfg!(target_os = "linux") {
                            println!(
                                "Error: install 'zenity' or 'kdialog' for a graphical dialog.\nThe message was: {}",
                                &message
                            );
                        } else {
                            unreachable!("unknown OS dialog error");
                        }
                    }
                    Command::none()
                }
            }
            Message::CheckAvailability(option) => match option {
                Some(tuple) => {
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
                            let message =
                                format!("Package '{}' is no longer available.", package.name);
                            if let Err(_) = MessageDialog::new()
                                .set_type(MessageType::Info)
                                .set_title("Ablavema")
                                .set_text(&message)
                                .show_alert()
                            {
                                if cfg!(target_os = "linux") {
                                    // TODO: Show a tooltip if dependencies not found.
                                    println!(
                                    "Error: install 'zenity' or 'kdialog' for a graphical dialog.\nThe message was: {}",
                                    &message
                                );
                                } else {
                                    unreachable!("unknown OS dialog error");
                                }
                            }
                        }
                        self.releases.sync();
                        Command::none()
                    }
                }
                None => {
                    self.releases.sync();
                    Command::none()
                }
            },
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
            Message::CancelInstall(package) => {
                let index = self
                    .installing
                    .iter()
                    .enumerate()
                    .find(|(_, (a_package, _))| *a_package == package)
                    .unwrap()
                    .0;
                self.installing.remove(index);
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
            Message::OpenBrowser(url) => {
                let _ = webbrowser::open(&url);
                Command::none()
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
                self.releases.sync();
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
                self.releases.sync();
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
                self.releases.sync();
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
                self.releases.sync();
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
                self.releases.sync();
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
                self.releases.sync();
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
            Message::MinutesBetweenUpdatesChanged(change) => {
                if change.is_positive() {
                    let mut current = SETTINGS.read().unwrap().minutes_between_updates;
                    current += change as u64;
                    if current > 1440 {
                        SETTINGS.write().unwrap().minutes_between_updates = 1440;
                    } else {
                        SETTINGS.write().unwrap().minutes_between_updates = current;
                    }
                } else {
                    let current = SETTINGS.read().unwrap().minutes_between_updates;
                    SETTINGS.write().unwrap().minutes_between_updates =
                        current.saturating_sub(change.abs() as u64);
                }
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
            Message::ChangeLocation(location) => {
                match location {
                    Location::Databases => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            SETTINGS.write().unwrap().databases_dir = directory;
                            SETTINGS.read().unwrap().save();
                        }
                    }
                    Location::Packages => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            SETTINGS.write().unwrap().packages_dir = directory;
                            SETTINGS.read().unwrap().save();
                            self.releases.sync();
                        }
                    }
                    Location::Cache => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            SETTINGS.write().unwrap().cache_dir = directory;
                            SETTINGS.read().unwrap().save();
                        }
                    }
                }
                Command::none()
            }
            Message::ResetLocation(location) => {
                match location {
                    Location::Databases => {
                        SETTINGS.write().unwrap().databases_dir =
                            PROJECT_DIRS.config_dir().to_path_buf();
                        SETTINGS.read().unwrap().save();
                    }
                    Location::Packages => {
                        SETTINGS.write().unwrap().packages_dir =
                            PROJECT_DIRS.data_local_dir().to_path_buf();
                        SETTINGS.read().unwrap().save();
                        self.releases.sync();
                    }
                    Location::Cache => {
                        SETTINGS.write().unwrap().cache_dir =
                            PROJECT_DIRS.cache_dir().to_path_buf();
                        SETTINGS.read().unwrap().save();
                    }
                }
                Command::none()
            }
            Message::RemoveDatabases(build_type) => {
                match build_type {
                    BuildType::All => {
                        self.releases.daily.remove_db();
                        self.releases.branched.remove_db();
                        self.releases.stable.remove_db();
                        self.releases.lts.remove_db();
                        self.releases.archived.remove_db();
                    }
                    BuildType::Daily => {
                        self.releases.daily.remove_db();
                    }
                    BuildType::Branched => {
                        self.releases.branched.remove_db();
                    }
                    BuildType::Stable => {
                        self.releases.stable.remove_db();
                    }
                    BuildType::Lts => {
                        self.releases.lts.remove_db();
                    }
                    BuildType::Archived => {
                        self.releases.archived.remove_db();
                    }
                }
                Command::none()
            }
            Message::RemovePackages(build_type) => {
                match build_type {
                    BuildType::All => {
                        self.releases.installed.remove_all();
                    }
                    BuildType::Daily => {
                        self.releases.installed.remove_daily();
                    }
                    BuildType::Branched => {
                        self.releases.installed.remove_branched();
                    }
                    BuildType::Stable => {
                        self.releases.installed.remove_stable();
                    }
                    BuildType::Lts => {
                        self.releases.installed.remove_lts();
                    }
                    BuildType::Archived => {
                        self.releases.installed.remove_archived();
                    }
                }
                self.releases.sync();
                Command::none()
            }
            Message::RemoveCache => {
                remove_dir_all(SETTINGS.read().unwrap().cache_dir.clone()).unwrap();
                println!("All cache removed.");
                create_dir_all(SETTINGS.read().unwrap().cache_dir.clone()).unwrap();
                Command::none()
            }
            Message::SelfUpdater(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().self_updater = true,
                    Choice::Disable => SETTINGS.write().unwrap().self_updater = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::CheckSelfUpdatesAtLaunch(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().check_self_updates_at_launch = true,
                    Choice::Disable => {
                        SETTINGS.write().unwrap().check_self_updates_at_launch = false
                    }
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::FetchSelfReleases => {
                self.self_releases = get_self_releases();
                if let Some(releases) = &self.self_releases {
                    self.state.release_versions = releases
                        .iter()
                        .map(|release| release.version.clone())
                        .collect();
                }
                Command::none()
            }
            Message::PickListVersionSelected(version) => {
                self.state.self_updater_pick_list_selected = version;
                Command::none()
            }
            Message::ChangeVersion => {
                self.state.installing_self_version = true;
                Command::perform(
                    Gui::change_self_version(
                        self.self_releases.clone().unwrap(),
                        self.state.self_updater_pick_list_selected.clone(),
                    ),
                    Message::VersionChanged,
                )
            }
            Message::VersionChanged(()) => {
                self.state.installing_self_version = false;
                self.state.installed_self_version = true;
                Command::none()
            }
            Message::CheckConnection => {
                self.state.controls.checking_connection = true;
                Command::perform(Gui::check_connection(), Message::ConnectionChecked)
            }
            Message::ConnectionChecked(()) => {
                self.state.controls.checking_connection = false;
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
                Container::new(button).padding(2)
            } else {
                Container::new(button.on_press(Message::TabChanged(tab))).padding(2)
            }
        };

        let self_update_tab_label = format!(
            "Self-updater{}",
            match check_self_updates(&self.self_releases) {
                Some(count) => {
                    format!(" [{}]", count)
                }
                None => {
                    String::new()
                }
            }
        );

        let tabs = Container::new(
            Row::new()
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
                .push(if SETTINGS.read().unwrap().self_updater {
                    tab_button(
                        &self_update_tab_label,
                        Tab::SelfUpdater,
                        &mut self.state.self_updater_button,
                    )
                } else {
                    Container::new(Space::with_width(Length::Units(0)))
                })
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
                // TODO: Add tooltips.
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
                            .align_items(Align::Center)
                            .push(Space::with_width(Length::Units(10)))
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
                            .push(Space::with_width(Length::Units(20)))
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
                    .style(theme.tab_button())
                };

                let change_location_button = |label, location, state| {
                    Button::new(state, Text::new(label))
                        .style(theme.tab_button())
                        .on_press(Message::ChangeLocation(location))
                };

                let reset_location_button = |location, state| {
                    // TODO: Disable button if path is default.
                    Button::new(state, Text::new("[R]"))
                        .style(theme.tab_button())
                        .on_press(Message::ResetLocation(location))
                };

                let remove_db_button = |label, build_type, exists, state| {
                    let button = Button::new(
                        state,
                        Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .width(Length::Fill)
                    .style(theme.tab_button());

                    if exists {
                        button.on_press(Message::RemoveDatabases(build_type))
                    } else {
                        button
                    }
                };

                let daily_db_exists = self.releases.daily.get_db_path().exists();
                let branched_db_exists = self.releases.branched.get_db_path().exists();
                let stable_db_exists = self.releases.stable.get_db_path().exists();
                let lts_db_exists = self.releases.lts.get_db_path().exists();
                let archived_db_exists = self.releases.archived.get_db_path().exists();
                let any_dbs_exist = daily_db_exists
                    || branched_db_exists
                    || stable_db_exists
                    || lts_db_exists
                    || archived_db_exists;

                let remove_packages_button = |label, build_type, exists, state| {
                    let button = Button::new(
                        state,
                        Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .width(Length::Fill)
                    .style(theme.tab_button());

                    if exists {
                        button.on_press(Message::RemovePackages(build_type))
                    } else {
                        button
                    }
                };

                let daily_packages_exist = if self
                    .releases
                    .installed
                    .iter()
                    .filter(|package| matches!(package.build, Build::Daily { .. }))
                    .count()
                    > 0
                {
                    true
                } else {
                    false
                };
                let branched_packages_exist = if self
                    .releases
                    .installed
                    .iter()
                    .filter(|package| matches!(package.build, Build::Branched { .. }))
                    .count()
                    > 0
                {
                    true
                } else {
                    false
                };
                let stable_packages_exist = if self
                    .releases
                    .installed
                    .iter()
                    .filter(|package| package.build == Build::Stable)
                    .count()
                    > 0
                {
                    true
                } else {
                    false
                };
                let lts_packages_exist = if self
                    .releases
                    .installed
                    .iter()
                    .filter(|package| package.build == Build::Lts)
                    .count()
                    > 0
                {
                    true
                } else {
                    false
                };
                let archived_packages_exist = if self
                    .releases
                    .installed
                    .iter()
                    .filter(|package| package.build == Build::Archived)
                    .count()
                    > 0
                {
                    true
                } else {
                    false
                };
                let any_packages_exist = daily_packages_exist
                    || branched_packages_exist
                    || stable_packages_exist
                    || lts_packages_exist
                    || archived_packages_exist;

                let installed_packages_space =
                    dir::get_size(SETTINGS.read().unwrap().packages_dir.clone()).unwrap() as f64
                        / 1024.0
                        / 1024.0
                        / 1024.0;
                let packages_dir_available_space =
                    available_space(SETTINGS.read().unwrap().packages_dir.clone()).unwrap() as f64
                        / 1024.0
                        / 1024.0
                        / 1024.0;
                let cache_space = dir::get_size(SETTINGS.read().unwrap().cache_dir.clone()).unwrap()
                    as f64
                    / 1024.0
                    / 1024.0
                    / 1024.0;
                let cache_dir_available_space =
                    available_space(SETTINGS.read().unwrap().cache_dir.clone()).unwrap() as f64
                        / 1024.0
                        / 1024.0
                        / 1024.0;

                let settings = Column::new()
                    .padding(10)
                    .spacing(10)
                    .push(Text::new("Checking for updates")
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(TEXT_SIZE * 3)
                        .color(theme.highlight_text())
                    )
                    .push(Text::new("These settings affect how checking for updates works. Enabling specific build types also marks the newest package of that build as an update. Keep in mind that you need to first have one installed package of that build type for any newer ones to be marked as an update, even if you're checking for their updates. It is recommended to disable checking for updates for builds that aren't installed to reduce launch time."))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Check at launch",
                        "Increases launch time for about a second or two. Having a delay between checks improves launch speed.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().check_updates_at_launch).unwrap()),
                        Message::CheckUpdatesAtLaunch,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Row::new()
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .width(Length::Fill)
                            .spacing(10)
                            .push(Text::new("Delay between checks")
                                .color(theme.highlight_text())
                                .size(TEXT_SIZE * 2)
                            )
                            .push(Text::new("Minutes to wait between update checks. Setting it to 0 will make it check every time. Maximum is a day (1440 minutes)."))
                        )
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .align_items(Align::Center)
                            .width(Length::Units(150))
                            .spacing(3)
                            .push(Row::new()
                                .push(min_button("+1", 1, &mut self.state.plus_1_button))
                                .push(min_button("+10", 10, &mut self.state.plus_10_button))
                                .push(min_button("+100", 100, &mut self.state.plus_100_button))
                            )
                            .push(Text::new(SETTINGS.read().unwrap().minutes_between_updates.to_string()))
                            .push(Row::new()
                                .push(min_button("-1", -1, &mut self.state.minus_1_button))
                                .push(min_button("-10", -10, &mut self.state.minus_10_button))
                                .push(min_button("-100", -100, &mut self.state.minus_100_button))
                            )
                        )
                        .push(Space::with_width(Length::Units(10)))
                    )
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Check daily packages",
                        "Look for new daily packages. Each build, like Alpha and Beta, is considered separate.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().update_daily).unwrap()),
                        Message::UpdateDaily,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Check branched packages",
                        "Look for new branched packages. Each branch is considered a separate build.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().update_branched).unwrap()),
                        Message::UpdateBranched,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Check stable packages",
                        "Look for new stable packages.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().update_stable).unwrap()),
                        Message::UpdateStable,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Check LTS packages",
                        "Look for new LTS packages.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().update_lts).unwrap()),
                        Message::UpdateLts,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Text::new("Installing updates")
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(TEXT_SIZE * 3)
                        .color(theme.highlight_text())
                    )
                    .push(Text::new("These settings affect what happens when an update is installed. Turning on old package removal for a build type means not being able to install an older version of the same build, like older LTS versions. So if needed, install those from the Archived packages."))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Use latest as default",
                        "Change to the latest package of the same build type when installing an update.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().use_latest_as_default).unwrap()),
                        Message::UseLatestAsDefault,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Keep only newest daily package",
                        "Remove all older daily packages of its build type when installing an update.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().keep_only_latest_daily).unwrap()),
                        Message::KeepOnlyLatestDaily,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Keep only newest branched package",
                        "Remove all older branched packages of its build type when installing an update.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().keep_only_latest_branched).unwrap()),
                        Message::KeepOnlyLatestBranched,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Keep only newest stable package",
                        "Remove all older stable packages when installing an update.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().keep_only_latest_stable).unwrap()),
                        Message::KeepOnlyLatestStable,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Keep only newest LTS package",
                        "Remove all older LTS packages when installing an update.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().keep_only_latest_lts).unwrap()),
                        Message::KeepOnlyLatestLts,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Text::new("Others")
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(TEXT_SIZE * 3)
                        .color(theme.highlight_text())
                    )
                    .push(Text::new("A few miscellaneous but useful settings."))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Bypass launcher",
                        "The preferred way to use this launcher. If a default package is set and no updates were found, only open launcher when the selected modifier key is held down. This way the launcher only makes itself known if there's an update or if you want to launch a different package.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().bypass_launcher).unwrap()),
                        Message::BypassLauncher,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Modifier key",
                        "You can start holding the modifier key even before double clicking on a .blend file or Ablavema shortcut, but you are able to change it if there's any interference.",
                        &ModifierKey::ALL,
                        Some(SETTINGS.read().unwrap().modifier_key),
                        Message::ModifierKey,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Choose the theme",
                        "Both try to mimic Blender's colour schemes as much as possible.",
                        &Theme::ALL,
                        Some(theme),
                        Message::ThemeChanged,
                    ))
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Row::new()
                        .align_items(Align::Center)
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .spacing(10)
                            .width(Length::Fill)
                            .push(Text::new("Change locations")
                                .color(theme.highlight_text())
                                .size(TEXT_SIZE * 2),
                            ).push(if PORTABLE.load(Ordering::Relaxed) {
                                Container::new(Text::new("Can't change locations because portable mode is enabled. Delete the \"portable\" file in the executable's directory to disable it.")).width(Length::Fill)
                            } else {
                                Container::new(Column::new()
                                    .spacing(10)
                                    .width(Length::Fill)
                                        .push(Text::new(&format!("Ablavema's files are stored in the recommended default locations for every platform, but changing them is possible. To change the location of the configuration file, which is stored by default at '{}' you can set the environment variable {} and it will create that file and use it as the config file, whatever its name is.", PROJECT_DIRS.config_dir().display(), CONFIG_FILE_ENV)))
                                        .push(Text::new(&format!("Default databases' location: {}\nCurrent databases' location: {}", PROJECT_DIRS.config_dir().display(), SETTINGS.read().unwrap().databases_dir.display())))
                                        .push(Text::new(&format!("Default packages' location: {}\nCurrent packages' location: {}", PROJECT_DIRS.data_local_dir().display(), SETTINGS.read().unwrap().packages_dir.display())))
                                        .push(Text::new(&format!("Default cache location: {}\nCurrent cache location: {}", PROJECT_DIRS.cache_dir().display(), SETTINGS.read().unwrap().cache_dir.display())))
                                        .push(Row::new()
                                            .spacing(5)
                                                .push(change_location_button(
                                                    "Databases",
                                                    Location::Databases,
                                                    &mut self.state.change_databases_location_button
                                                ))
                                                .push(reset_location_button(
                                                    Location::Databases,
                                                    &mut self.state.reset_databases_location_button
                                                ))
                                                .push(Space::with_width(Length::Units(15)))
                                                .push(change_location_button(
                                                    "Packages",
                                                    Location::Packages,
                                                    &mut self.state.change_packages_location_button
                                                ))
                                                .push(reset_location_button(
                                                    Location::Packages,
                                                    &mut self.state.reset_packages_location_button
                                                ))
                                                .push(Space::with_width(Length::Units(15)))
                                                .push(change_location_button(
                                                    "Cache",
                                                    Location::Cache,
                                                    &mut self.state.change_cache_location_button
                                                ))
                                                .push(reset_location_button(
                                                    Location::Cache,
                                                    &mut self.state.reset_cache_location_button
                                                ))
                                        )
                                )
                            }
                            )
                        )
                        .push(Space::with_width(Length::Units(10)))
                    )
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Row::new()
                        .align_items(Align::Center)
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .spacing(10)
                            .width(Length::Fill)
                            .push(Text::new("Remove databases")
                                .color(theme.highlight_text())
                                .size(TEXT_SIZE * 2),
                            )
                            .push(Text::new("Useful for when a release candidate is still available even though it doesn't appear in the website anymore. Keep in mind that bookmarks are stored in the databases, so they will be lost. Also, any installed package that's no longer available, like with old daily and branched packages, won't reapear."))
                            .push(Row::new()
                                .spacing(5)
                                .push(remove_db_button(
                                    "All",
                                    BuildType::All,
                                    any_dbs_exist,
                                    &mut self.state.remove_all_dbs_button
                                ))
                                .push(remove_db_button(
                                    "Daily",
                                    BuildType::Daily,
                                    daily_db_exists,
                                    &mut self.state.remove_daily_db_button
                                ))
                                .push(remove_db_button(
                                    "Branched",
                                    BuildType::Branched,
                                    branched_db_exists,
                                    &mut self.state.remove_branched_db_button
                                ))
                                .push(remove_db_button(
                                    "Stable",
                                    BuildType::Stable,
                                    stable_db_exists,
                                    &mut self.state.remove_stable_db_button
                                ))
                                .push(remove_db_button(
                                    "LTS",
                                    BuildType::Lts,
                                    lts_db_exists,
                                    &mut self.state.remove_lts_db_button
                                ))
                                .push(remove_db_button(
                                    "Archived",
                                    BuildType::Archived,
                                    archived_db_exists,
                                    &mut self.state.remove_archived_db_button
                                ))
                            )
                        )
                        .push(Space::with_width(Length::Units(10)))
                    )
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Row::new()
                        .align_items(Align::Center)
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .spacing(10)
                            .width(Length::Fill)
                            .push(Text::new("Remove packages")
                                .color(theme.highlight_text())
                                .size(TEXT_SIZE * 2),
                            )
                            .push(Text::new("Useful for getting rid of a large quantity of packages at once."))
                            // TODO: Fix slowdowns due to calculating packages' size.
                            .push(Text::new(format!("Space used by installed packages: {:.2} GB\nAvailable space: {:.2} GB", installed_packages_space, packages_dir_available_space)))
                            .push(Row::new()
                                .spacing(5)
                                .push(remove_packages_button(
                                    "All",
                                    BuildType::All,
                                    any_packages_exist,
                                    &mut self.state.remove_all_packages_button
                                ))
                                .push(remove_packages_button(
                                    "Daily",
                                    BuildType::Daily,
                                    daily_packages_exist,
                                    &mut self.state.remove_daily_packages_button
                                ))
                                .push(remove_packages_button(
                                    "Branched",
                                    BuildType::Branched,
                                    branched_packages_exist,
                                    &mut self.state.remove_branched_packages_button
                                ))
                                .push(remove_packages_button(
                                    "Stable",
                                    BuildType::Stable,
                                    stable_packages_exist,
                                    &mut self.state.remove_stable_packages_button
                                ))
                                .push(remove_packages_button(
                                    "LTS",
                                    BuildType::Lts,
                                    lts_packages_exist,
                                    &mut self.state.remove_lts_packages_button
                                ))
                                .push(remove_packages_button(
                                    "Archived",
                                    BuildType::Archived,
                                    archived_packages_exist,
                                    &mut self.state.remove_archived_packages_button
                                ))
                            )
                        )
                        .push(Space::with_width(Length::Units(10)))
                    )
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(Row::new()
                        .align_items(Align::Center)
                        .push(Space::with_width(Length::Units(10)))
                        .push(Column::new()
                            .spacing(10)
                            .width(Length::Fill)
                            .push(Text::new("Remove cache")
                                .color(theme.highlight_text())
                                .size(TEXT_SIZE * 2),
                            )
                            .push(Text::new("Useful for getting rid of the accumulated cache (mainly downloaded packages) since at the moment cache isn't being automatically removed."))
                            // TODO: Fix slowdowns due to calculating cache size.
                            .push(Text::new(format!("Space used by cache: {:.2} GB\nAvailable space: {:.2} GB", cache_space, cache_dir_available_space)))
                            .push(Button::new(&mut self.state.remove_cache_button,Text::new("Remove all cache"))
                                .on_press(Message::RemoveCache)
                                .style(self.theme.tab_button())
                            )
                        )
                        .push(Space::with_width(Length::Units(10)))
                    )
                    .push(Rule::horizontal(0).style(self.theme))
                    .push(choice_setting!(
                        "Self-updater",
                        "Update the launcher itself through the built-in system. This enables a hidden tab dedicated to updating, which can also be used to read the release notes of every version. Keep in mind that if Ablavema is installed through a package manager, the laucher should be updated through it. Though if made use of even if installed through a package manager, upon updating it through the package manager the executable would simply be replaced with the newer one, same as if done through the built-in system. In this way, making use of this feature is helpful when trying out older versions to see if a bug was there before or whatnot.",
                        &Choice::ALL,
                        Some(choice(SETTINGS.read().unwrap().self_updater).unwrap()),
                        Message::SelfUpdater,
                    ));

                Container::new(Scrollable::new(&mut self.state.settings_scroll).push(
                    if SETTINGS.read().unwrap().self_updater {
                        settings
                            .push(Rule::horizontal(0).style(self.theme))
                            .push(choice_setting!(
                                "Check for Ablavema updates at launch",
                                "This uses the same delay as the normal updates. Keep in mind that, at the moment, if you downgrade you will be prompted to update Ablavema every time updates are checked.",
                                &Choice::ALL,
                                Some(
                                    choice(SETTINGS.read().unwrap().check_self_updates_at_launch)
                                        .unwrap()
                                ),
                                Message::CheckSelfUpdatesAtLaunch,
                            ))
                    } else {
                        settings
                    },
                ))
                .height(Length::Fill)
                .width(Length::Fill)
                .style(self.theme)
                .into()
            }
            Tab::SelfUpdater => {
                let self_updater_pick_list_selected =
                    self.state.self_updater_pick_list_selected.clone();

                let release_index =
                    match &self.self_releases {
                        Some(releases) => {
                            match releases.iter().enumerate().find(|(_, release)| {
                                release.version == self_updater_pick_list_selected
                            }) {
                                Some((index, _)) => index,
                                None => 0,
                            }
                        }
                        None => 0,
                    };

                Container::new(
                    Column::new()
                        .align_items(Align::Center)
                        .push(
                            Row::new()
                                .align_items(Align::Center)
                                .padding(10)
                                .spacing(10)
                                .push(Text::new(format!("Current version: {}", crate_version!())))
                                .push(Text::new("Select version:"))
                                .push(
                                    PickList::new(
                                        &mut self.state.self_updater_pick_list,
                                        &self.state.release_versions,
                                        Some(self_updater_pick_list_selected.clone()),
                                        Message::PickListVersionSelected,
                                    )
                                    .width(Length::Units(60))
                                    .style(theme),
                                )
                                .push(if self.state.installed_self_version {
                                    Container::new(Text::new("Restart Ablavema."))
                                } else if self.state.installing_self_version {
                                    Container::new(Text::new("Installing..."))
                                } else if self.self_releases.is_none() {
                                    Container::new({
                                        let button = Button::new(
                                            &mut self.state.fetch_self_releases_button,
                                            Text::new("Fetch releases"),
                                        )
                                        .style(theme);
                                        if CAN_CONNECT.load(Ordering::Relaxed) {
                                            // TODO: Check connectivity on press.
                                            button.on_press(Message::FetchSelfReleases)
                                        } else {
                                            button
                                        }
                                    })
                                } else {
                                    Container::new({
                                        let button = Button::new(
                                            &mut self.state.install_self_version_button,
                                            Text::new("Install this version"),
                                        )
                                        .style(theme);
                                        if self.state.self_updater_pick_list_selected
                                            == crate_version!()
                                            || !CAN_CONNECT.load(Ordering::Relaxed)
                                        {
                                            button
                                        } else {
                                            // TODO: Check connectivity on press.
                                            button.on_press(Message::ChangeVersion)
                                        }
                                    })
                                }),
                        )
                        .push(match &self.self_releases {
                            Some(releases) => Container::new(
                                Scrollable::new(&mut self.state.self_updater_scroll).push(
                                    Row::new()
                                        .push(Space::with_width(Length::Fill))
                                        .push(
                                            Column::new()
                                                .padding(10)
                                                .spacing(20)
                                                .align_items(Align::Center)
                                                .width(Length::FillPortion(50))
                                                .push(
                                                    Text::new(&releases[release_index].name)
                                                        .size(TEXT_SIZE * 2),
                                                )
                                                .push(Text::new(
                                                    releases[release_index]
                                                        .body
                                                        .as_deref()
                                                        .unwrap_or_default(),
                                                )),
                                        )
                                        .push(Space::with_width(Length::Fill)),
                                ),
                            )
                            .height(Length::Fill)
                            .style(theme),
                            None => Container::new(Space::new(Length::Fill, Length::Fill))
                                .height(Length::Fill)
                                .width(Length::Fill)
                                .style(theme),
                        }),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .style(theme.sidebar_container())
                .into()
            }
            Tab::About => {
                let link = |label, url, state| {
                    Row::new()
                        .spacing(10)
                        .align_items(Align::Center)
                        .push(
                            Text::new(label)
                                .width(Length::Units(70))
                                .color(theme.highlight_text()),
                        )
                        .push(
                            Button::new(state, Text::new(&url))
                                .on_press(Message::OpenBrowser(url))
                                .style(theme),
                        )
                };

                Container::new(
                    Column::new()
                        .spacing(10)
                        .align_items(Align::Center)
                        .push(Space::with_height(Length::Units(10)))
                        .push(
                            Row::new()
                                .spacing(10)
                                .align_items(Align::End)
                                .push(Text::new("Ablavema").size(TEXT_SIZE * 3))
                                .push(Text::new(crate_version!()).size(TEXT_SIZE * 2)),
                        )
                        .push(
                            Text::new("A Blender Launcher and Version Manager").size(TEXT_SIZE * 2),
                        )
                        .push(
                            Column::new()
                                .spacing(10)
                                .push(Space::with_height(Length::Units(30)))
                                .push(link(
                                    "Repository:",
                                    String::from("https://github.com/AlexChaplinBraz/Ablavema"),
                                    &mut self.state.repository_link_button,
                                ))
                                .push(link(
                                    "Discord:",
                                    String::from("https://discord.gg/D6gmhMUrrH"),
                                    &mut self.state.discord_link_button,
                                ))
                                .push(link(
                                    "Contact me:",
                                    String::from("https://alexchaplinbraz.com/contact"),
                                    &mut self.state.contact_link_button,
                                ))
                                .push(link(
                                    "Donate:",
                                    String::from("https://donate.alexchaplinbraz.com"),
                                    &mut self.state.donation_link_button,
                                )),
                        ),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .style(theme)
                .into()
            }
        };

        Column::new().push(tabs).push(body).into()
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
    CheckAvailability(Option<(bool, bool, Package)>),
    InstallPackage(Package),
    CancelInstall(Package),
    PackageInstalled(Package),
    PackageRemoved(Package),
    OpenBlender(Package),
    OpenBlenderWithFile(Package),
    OpenBrowser(String),
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
    MinutesBetweenUpdatesChanged(i64),
    UpdateDaily(Choice),
    UpdateBranched(Choice),
    UpdateStable(Choice),
    UpdateLts(Choice),
    KeepOnlyLatestDaily(Choice),
    KeepOnlyLatestBranched(Choice),
    KeepOnlyLatestStable(Choice),
    KeepOnlyLatestLts(Choice),
    ThemeChanged(Theme),
    ChangeLocation(Location),
    ResetLocation(Location),
    RemoveDatabases(BuildType),
    RemovePackages(BuildType),
    RemoveCache,
    SelfUpdater(Choice),
    CheckSelfUpdatesAtLaunch(Choice),
    FetchSelfReleases,
    PickListVersionSelected(String),
    ChangeVersion,
    VersionChanged(()),
    CheckConnection,
    ConnectionChecked(()),
}

#[derive(Debug)]
pub struct GuiFlags {
    pub releases: Releases,
    pub file_path: Option<String>,
    pub self_releases: Option<Vec<Release>>,
}

#[derive(Debug, Default)]
struct GuiState {
    controls: Controls,
    packages_scroll: scrollable::State,
    settings_scroll: scrollable::State,
    self_updater_scroll: scrollable::State,
    about_scroll: scrollable::State,
    open_default_button: button::State,
    open_default_with_file_button: button::State,
    packages_button: button::State,
    settings_button: button::State,
    self_updater_button: button::State,
    about_button: button::State,
    plus_1_button: button::State,
    plus_10_button: button::State,
    plus_100_button: button::State,
    minus_1_button: button::State,
    minus_10_button: button::State,
    minus_100_button: button::State,
    change_databases_location_button: button::State,
    reset_databases_location_button: button::State,
    change_packages_location_button: button::State,
    reset_packages_location_button: button::State,
    change_cache_location_button: button::State,
    reset_cache_location_button: button::State,
    remove_all_dbs_button: button::State,
    remove_daily_db_button: button::State,
    remove_branched_db_button: button::State,
    remove_stable_db_button: button::State,
    remove_lts_db_button: button::State,
    remove_archived_db_button: button::State,
    remove_all_packages_button: button::State,
    remove_daily_packages_button: button::State,
    remove_branched_packages_button: button::State,
    remove_stable_packages_button: button::State,
    remove_lts_packages_button: button::State,
    remove_archived_packages_button: button::State,
    remove_cache_button: button::State,
    release_versions: Vec<String>,
    fetch_self_releases_button: button::State,
    self_updater_pick_list: pick_list::State<String>,
    self_updater_pick_list_selected: String,
    install_self_version_button: button::State,
    installing_self_version: bool,
    installed_self_version: bool,
    repository_link_button: button::State,
    discord_link_button: button::State,
    contact_link_button: button::State,
    donation_link_button: button::State,
}

impl GuiState {
    fn new() -> Self {
        Self {
            controls: Controls {
                filters: SETTINGS.read().unwrap().filters,
                sort_by: SETTINGS.read().unwrap().sort_by,
                ..Controls::default()
            },
            self_updater_pick_list_selected: crate_version!().to_owned(),
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
    check_connection_button: button::State,
    checking_connection: bool,
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

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Container::new(scrollable)
                // TODO: Can't get it to shrink around its content for some reason.
                // It always fills the whole space unless I set a specific width.
                .width(Length::Units(190))
                .height(Length::Fill)
                .style(theme.sidebar_container())
                .into()
        } else {
            Container::new(
                Column::new().push(scrollable.height(Length::Fill)).push(
                    Container::new(
                        Row::new()
                            .padding(1)
                            .align_items(Align::Center)
                            .push(Space::with_width(Length::Units(9)))
                            .push(Text::new("CANNOT CONNECT").width(Length::Fill))
                            .push({
                                let button = Button::new(
                                    &mut self.check_connection_button,
                                    Text::new("[R]"),
                                )
                                .style(theme.tab_button());

                                if self.checking_connection {
                                    button
                                } else {
                                    button.on_press(Message::CheckConnection)
                                }
                            })
                            .push(Space::with_width(Length::Units(9))),
                    )
                    .width(Length::Fill)
                    .style(theme.status_container()),
                ),
            )
            .width(Length::Units(190))
            .height(Length::Fill)
            .style(theme.sidebar_container())
            .into()
        }
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
            SortBy::VersionAscending => Ord::cmp(&a.version, &b.version),
            SortBy::VersionDescending => Ord::cmp(&a.version, &b.version).reverse(),
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
    SelfUpdater,
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
pub enum BuildType {
    All,
    Daily,
    Branched,
    Stable,
    Lts,
    Archived,
}

#[derive(Clone, Debug)]
pub enum Location {
    Databases,
    Packages,
    Cache,
}

#[derive(Clone, Debug)]
pub enum PackageMessage {
    Install,
    InstallationProgress(Progress),
    Cancel,
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
                    self.state = PackageState::Downloading {
                        progress: 0.0,
                        cancel_button: Default::default(),
                    };
                    Command::none()
                }
                Progress::DownloadProgress(progress) => {
                    if let PackageState::Downloading { cancel_button, .. } = self.state {
                        self.state = PackageState::Downloading {
                            progress,
                            cancel_button,
                        };
                    }
                    Command::none()
                }
                Progress::FinishedDownloading => {
                    self.state = PackageState::Extracting {
                        progress: 0.0,
                        cancel_button: Default::default(),
                    };
                    Command::none()
                }
                Progress::ExtractionProgress(progress) => {
                    if let PackageState::Extracting { cancel_button, .. } = self.state {
                        self.state = PackageState::Extracting {
                            progress,
                            cancel_button,
                        };
                    }
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
                Progress::Errored(error_message) => {
                    self.state = PackageState::Errored {
                        error_message,
                        retry_button: Default::default(),
                    };
                    Command::perform(Gui::pass_package(self.clone()), Message::CancelInstall)
                }
            },
            PackageMessage::Cancel => {
                self.state = PackageState::default();
                Command::perform(Gui::pass_package(self.clone()), Message::CancelInstall)
            }
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
                            .push(
                                Text::new(self.version.to_string()).color(theme.highlight_text()),
                            ),
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
            PackageState::Downloading {
                progress,
                cancel_button,
            } => Row::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(Text::new(format!("Downloading... {:.2}%", progress)))
                .push(
                    ProgressBar::new(0.0..=100.0, *progress)
                        .width(Length::Fill)
                        .style(theme),
                )
                .push(
                    Button::new(cancel_button, Text::new("Cancel"))
                        .on_press(PackageMessage::Cancel)
                        .style(theme),
                )
                .into(),
            PackageState::Extracting {
                progress,
                cancel_button: _,
            } => {
                // TODO: Figure out why cancelling doesn't work for extraction.
                // It does visually get cancelled, but the extraction keeps going in the
                // background, ultimately getting installed. But since the package was supposedly
                // removed from the installation process, the program crashes at the end when it
                // tries that, since it's no longer there. The same behaviour happens on Windows,
                // where the extraction works differently. I thought maybe the download kept going
                // as well, but no, that stops as intended when cancelled.
                if cfg!(target_os = "linux") {
                    Row::new()
                        .align_items(Align::Center)
                        .push(
                            Text::new(format!("Extracting..."))
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        /* .push(
                            Button::new(cancel_button, Text::new("Cancel"))
                                .on_press(PackageMessage::Cancel)
                                .style(theme),
                        ) */
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
                        /* .push(
                            Button::new(cancel_button, Text::new("Cancel"))
                                .on_press(PackageMessage::Cancel)
                                .style(theme),
                        ) */
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
            PackageState::Errored {
                error_message,
                retry_button,
            } => Row::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(Text::new(format!("Error: {}.", error_message)).width(Length::Fill))
                .push(
                    Button::new(retry_button, Text::new("Retry"))
                        .on_press(PackageMessage::Install)
                        .style(theme),
                )
                .into(),
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
