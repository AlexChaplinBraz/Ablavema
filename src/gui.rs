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
    settings::{ModifierKey, SETTINGS},
};
use iced::{
    button, scrollable, slider, Align, Application, Button, Column, Command, Container, Element,
    Executor, HorizontalAlignment, Length, ProgressBar, Radio, Row, Rule, Scrollable, Slider,
    Subscription, Text,
};
use reqwest;
use std::{iter, process};

#[derive(Debug)]
pub struct Gui {
    releases: Releases,
    file_path: Option<String>,
    installing: Vec<(Package, usize)>,
    state: GuiState,
    tab: Tab,
    filter: Filter,
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
        (
            Gui {
                releases: flags.releases,
                file_path: flags.file_path,
                installing: Vec::default(),
                state: GuiState::new(),
                tab: Tab::Packages,
                filter: Filter::Installed,
                theme: SETTINGS.read().unwrap().theme,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!(
            "BlenderLauncher{}",
            match self.releases.count_updates() {
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
            Message::UnsetDefault => {
                SETTINGS.write().unwrap().default_package = None;
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::RemoveDefault => {
                let package = SETTINGS.read().unwrap().default_package.clone().unwrap();
                package.remove();
                Command::perform(Gui::pass_package(package), Message::PackageRemoved)
            }
            Message::CheckForUpdates => Command::perform(
                Gui::check_for_updates(self.releases.take()),
                Message::UpdatesChecked,
            ),
            Message::UpdatesChecked(tuple) => {
                // TODO: Add some feedback once completed.
                // The packages disappear for a moment due to the use of take(), but that's only
                // if the user is on the same filter. There's also an indication of a package
                // being new or an update, but only if you're looking at them. It would be great
                // if iced had tooltips. For now, maybe showing the number of new packages on its
                // filter button may be best. But that still doesn't help with the fact that an
                // update check with no new results doesn't have much feedback if you're not
                // looking at the output in the terminal. Maybe use a msgbox.
                self.releases.add_new_packages(tuple);
                Command::none()
            }
            Message::FetchDaily => Command::perform(
                Gui::check_daily(self.releases.daily.take()),
                Message::DailyFetched,
            ),
            Message::DailyFetched(tuple) => {
                self.releases.daily = tuple.1;
                Command::none()
            }
            Message::FetchBranched => Command::perform(
                Gui::check_branched(self.releases.branched.take()),
                Message::BranchedFetched,
            ),
            Message::BranchedFetched(tuple) => {
                self.releases.branched = tuple.1;
                Command::none()
            }
            Message::FetchStable => Command::perform(
                Gui::check_stable(self.releases.stable.take()),
                Message::StableFetched,
            ),
            Message::StableFetched(tuple) => {
                self.releases.stable = tuple.1;
                Command::none()
            }
            Message::FetchLts => Command::perform(
                Gui::check_lts(self.releases.lts.take()),
                Message::LtsFetched,
            ),
            Message::LtsFetched(tuple) => {
                self.releases.lts = tuple.1;
                Command::none()
            }
            Message::FetchArchived => Command::perform(
                Gui::check_archived(self.releases.archived.take()),
                Message::ArchivedFetched,
            ),
            Message::ArchivedFetched(tuple) => {
                self.releases.archived = tuple.1;
                Command::none()
            }
            Message::TabChanged(tab) => {
                self.tab = tab;
                Command::none()
            }
            Message::FilterChanged(filter) => {
                self.filter = filter;
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
        let filter = self.filter;
        let theme = self.theme;

        let top_button = |label, tab, state| {
            let button = Button::new(
                state,
                Text::new(label)
                    .size(16)
                    .horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Units(100))
            .style(theme);

            if tab == self_tab {
                button
            } else {
                button.on_press(Message::TabChanged(tab))
            }
        };

        let tabs = Container::new(
            Row::new()
                .padding(5)
                .spacing(40)
                .push(top_button(
                    "Packages",
                    Tab::Packages,
                    &mut self.state.packages_button,
                ))
                .push(top_button(
                    "Settings",
                    Tab::Settings,
                    &mut self.state.settings_button,
                ))
                .push(top_button(
                    "About",
                    Tab::About,
                    &mut self.state.about_button,
                )),
        )
        .width(Length::Fill)
        .center_x()
        .style(self.theme.lighter_container());

        let body: Element<'_, Message> = match self.tab {
            Tab::Packages => {
                let controls: Element<'_, Message> = Container::new(
                    Column::new()
                        .padding(20)
                        .spacing(20)
                        .push(
                            Column::new()
                                .width(Length::Fill)
                                .spacing(5)
                                .push(Text::new(match &self.file_path {
                                    Some(file_path) => format!("File: {}", file_path),
                                    None => format!("File: no .blend file to open"),
                                }))
                                .push(match SETTINGS.read().unwrap().default_package.clone() {
                                    Some(package) => {
                                        let button =
                                            |label, package_message: Option<Message>, state| {
                                                let button = Button::new(
                                                    state,
                                                    Text::new(label).size(18).horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    ),
                                                )
                                                .width(Length::Fill)
                                                .style(theme);

                                                if package_message.is_some() {
                                                    button.on_press(package_message.unwrap())
                                                } else {
                                                    button
                                                }
                                            };
                                        let col = Column::new()
                                            .spacing(5)
                                            .push(Text::new(format!(
                                                "Default package: {}",
                                                package.name
                                            )))
                                            .push(
                                                Row::new()
                                                    .spacing(40)
                                                    .push(button(
                                                        "Open default",
                                                        Some(Message::OpenBlender(package.clone())),
                                                        &mut self.state.open_default_button,
                                                    ))
                                                    .push(button(
                                                        "Open default with file",
                                                        if file_exists {
                                                            Some(Message::OpenBlenderWithFile(
                                                                package.clone(),
                                                            ))
                                                        } else {
                                                            None
                                                        },
                                                        &mut self
                                                            .state
                                                            .open_default_with_file_button,
                                                    ))
                                                    .push(button(
                                                        "Unset default",
                                                        Some(Message::UnsetDefault),
                                                        &mut self.state.unset_default_button,
                                                    ))
                                                    .push(button(
                                                        "Remove default",
                                                        Some(Message::RemoveDefault),
                                                        &mut self.state.remove_default_button,
                                                    )),
                                            );
                                        Container::new(col)
                                    }
                                    None => Container::new(Text::new("Default package: not set")),
                                }),
                        )
                        .push(self.state.controls.view(
                            self.releases.count_updates(),
                            self.filter,
                            self.theme,
                        )),
                )
                .width(Length::Fill)
                .style(self.theme.light_container())
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
                                .filter(|(_, package)| filter.matches(package))
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

                    let scrollable = Scrollable::new(match self.filter {
                        Filter::Updates => &mut self.state.updates_scroll,
                        Filter::Installed => &mut self.state.installed_scroll,
                        Filter::Daily => &mut self.state.daily_scroll,
                        Filter::Branched => &mut self.state.branched_scroll,
                        Filter::Lts => &mut self.state.lts_scroll,
                        Filter::Stable => &mut self.state.stable_scroll,
                        Filter::Archived => &mut self.state.archived_scroll,
                    })
                    .push(filtered_packages);

                    if package_count == 0 {
                        Container::new(
                            Text::new(match filter {
                                Filter::Updates => "No updates found",
                                Filter::Installed => "No installed packages",
                                Filter::Daily => "No daily packages, please fetch first",
                                Filter::Branched => "No branched packages, please fetch first",
                                Filter::Lts => "No LTS packages, please fetch first",
                                Filter::Stable => "No stable packages, please fetch first",
                                Filter::Archived => "No archived packages, please fetch first",
                            })
                            .size(50),
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

                Column::new().push(controls).push(packages).into()
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
                                    .push(Text::new($title).size(30))
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
                let settings = Column::new()
                    .padding(20)
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
                        .push(Text::new("Delay between update checking").size(30))
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
            Tab::About => Container::new(Text::new("About tab not yet implemented").size(50))
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .center_y()
                .style(theme)
                .into(),
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
    TryToInstall(Package),
    CheckAvailability((bool, bool, Package)),
    InstallPackage(Package),
    PackageInstalled(Package),
    PackageRemoved(Package),
    OpenBlender(Package),
    OpenBlenderWithFile(Package),
    UnsetDefault,
    RemoveDefault,
    CheckForUpdates,
    UpdatesChecked((bool, Daily, Branched, Stable, Lts)),
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
    TabChanged(Tab),
    FilterChanged(Filter),
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
    updates_scroll: scrollable::State,
    installed_scroll: scrollable::State,
    daily_scroll: scrollable::State,
    branched_scroll: scrollable::State,
    stable_scroll: scrollable::State,
    lts_scroll: scrollable::State,
    archived_scroll: scrollable::State,
    settings_scroll: scrollable::State,
    about_scroll: scrollable::State,
    open_default_button: button::State,
    open_default_with_file_button: button::State,
    unset_default_button: button::State,
    remove_default_button: button::State,
    updates_button: button::State,
    installed_button: button::State,
    daily_button: button::State,
    branched_button: button::State,
    stable_button: button::State,
    lts_button: button::State,
    archived_button: button::State,
    packages_button: button::State,
    settings_button: button::State,
    about_button: button::State,
    minute_slider: slider::State,
    minute_value: f64,
}

impl GuiState {
    fn new() -> Self {
        Self {
            minute_value: SETTINGS.read().unwrap().minutes_between_updates as f64,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Controls {
    check_updates_button: button::State,
    fetch_daily_button: button::State,
    fetch_branched_button: button::State,
    fetch_stable_button: button::State,
    fetch_lts_button: button::State,
    fetch_archived_button: button::State,
    updates_button: button::State,
    installed_button: button::State,
    daily_button: button::State,
    branched_button: button::State,
    lts_button: button::State,
    stable_button: button::State,
    archived_button: button::State,
}

impl Controls {
    fn view(
        &mut self,
        update_count: Option<usize>,
        filter: Filter,
        theme: Theme,
    ) -> Container<'_, Message> {
        let fetch_button = |state, label, message| {
            let label = Text::new(label)
                .size(16)
                .horizontal_alignment(HorizontalAlignment::Center);
            Button::new(state, label)
                .width(Length::Fill)
                .on_press(message)
                .style(theme)
        };

        let filter_button = |state, label, filter, current_filter| {
            let label = Text::new(label)
                .size(16)
                .horizontal_alignment(HorizontalAlignment::Center);
            let button = Button::new(state, label).width(Length::Fill).style(theme);

            if filter == current_filter {
                button
            } else {
                button.on_press(Message::FilterChanged(filter))
            }
        };

        Container::new(
            Column::new()
                .spacing(5)
                .push(
                    Row::new()
                        .width(Length::Fill)
                        .spacing(20)
                        .push(fetch_button(
                            &mut self.check_updates_button,
                            "Check for updates",
                            Message::CheckForUpdates,
                        ))
                        .push(fetch_button(
                            &mut self.fetch_daily_button,
                            "Fetch daily",
                            Message::FetchDaily,
                        ))
                        .push(fetch_button(
                            &mut self.fetch_branched_button,
                            "Fetch branched",
                            Message::FetchBranched,
                        ))
                        .push(fetch_button(
                            &mut self.fetch_stable_button,
                            "Fetch stable",
                            Message::FetchStable,
                        ))
                        .push(fetch_button(
                            &mut self.fetch_lts_button,
                            "Fetch LTS",
                            Message::FetchLts,
                        ))
                        .push(fetch_button(
                            &mut self.fetch_archived_button,
                            "Fetch archived",
                            Message::FetchArchived,
                        )),
                )
                .push(
                    Row::new()
                        .width(Length::Fill)
                        .spacing(20)
                        .push(filter_button(
                            &mut self.updates_button,
                            match update_count {
                                Some(count) => format!("Updates [{}]", count),
                                None => String::from("Updates"),
                            },
                            Filter::Updates,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.installed_button,
                            String::from("Installed"),
                            Filter::Installed,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.daily_button,
                            String::from("Daily"),
                            Filter::Daily,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.branched_button,
                            String::from("Branched"),
                            Filter::Branched,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.stable_button,
                            String::from("Stable"),
                            Filter::Stable,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.lts_button,
                            String::from("LTS"),
                            Filter::Lts,
                            filter,
                        ))
                        .push(filter_button(
                            &mut self.archived_button,
                            String::from("Archived"),
                            Filter::Archived,
                            filter,
                        )),
                ),
        )
        .width(Length::Fill)
        .center_x()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tab {
    Packages,
    Settings,
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Filter {
    Updates,
    Installed,
    Daily,
    Branched,
    Lts,
    Stable,
    Archived,
}

impl Filter {
    fn matches(&self, package: &Package) -> bool {
        match self {
            Filter::Updates => package.status == PackageStatus::Update,
            Filter::Installed => matches!(package.state, PackageState::Installed { .. }),
            Filter::Daily => matches!(package.build, Build::Daily { .. }),
            Filter::Branched => matches!(package.build, Build::Branched { .. }),
            Filter::Lts => package.build == Build::Lts,
            Filter::Stable => package.build == Build::Stable,
            Filter::Archived => package.build == Build::Archived,
        }
    }
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
        }
    }

    fn view(
        &mut self,
        file_exists: bool,
        theme: Theme,
        is_odd: bool,
    ) -> Element<'_, PackageMessage> {
        let name = Text::new(&self.name).size(30);

        let details = Row::new()
            .push(
                Row::new()
                    .width(Length::Fill)
                    .align_items(Align::End)
                    .push(Text::new("Date: ").size(16))
                    .push(Text::new(self.date.to_string()).size(20))
                    .push(Text::new("        Version: ").size(16))
                    .push(Text::new(&self.version).size(20))
                    .push(Text::new("        Build: ").size(16))
                    .push(Text::new(self.build.to_string()).size(20)),
            )
            .push(
                Text::new(match self.status {
                    PackageStatus::Update => "UPDATE     ",
                    PackageStatus::New => "NEW     ",
                    PackageStatus::Old => "",
                })
                .size(20),
            );

        let button = |label, package_message: Option<PackageMessage>, state| {
            let button = Button::new(
                state,
                Text::new(label)
                    .size(18)
                    .horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(theme);

            if package_message.is_some() {
                button.on_press(package_message.unwrap())
            } else {
                button
            }
        };

        let is_default_package = SETTINGS.read().unwrap().default_package.is_some()
            && SETTINGS.read().unwrap().default_package.clone().unwrap() == *self;

        let controls: Element<'_, PackageMessage> = match &mut self.state {
            PackageState::Fetched { install_button } => Row::new()
                .push(button(
                    "Install",
                    Some(PackageMessage::Install),
                    install_button,
                ))
                .into(),
            PackageState::Downloading { progress } => Row::new()
                .align_items(Align::Center)
                .push(
                    Text::new(format!("Downloading... {:.2}%", progress)).width(Length::Units(220)),
                )
                .push(
                    ProgressBar::new(0.0..=100.0, *progress)
                        .width(Length::Fill)
                        .style(theme),
                )
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
                        .align_items(Align::Center)
                        .push(
                            Text::new(format!("Extracting... {:.2}%", progress))
                                .width(Length::Units(220)),
                        )
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
                // TODO: Add button for adding package to favourites.
                let button1 = Row::new().push(button(
                    "Open",
                    Some(PackageMessage::OpenBlender),
                    open_button,
                ));

                let button2 = button1.push(button(
                    "Open file",
                    if file_exists {
                        Some(PackageMessage::OpenBlenderWithFile)
                    } else {
                        None
                    },
                    open_file_button,
                ));

                let button3 = button2.push(button(
                    if is_default_package {
                        "Unset default"
                    } else {
                        "Set as default"
                    },
                    if is_default_package {
                        Some(PackageMessage::UnsetDefault)
                    } else {
                        Some(PackageMessage::SetDefault)
                    },
                    set_default_button,
                ));

                button3
                    .spacing(40)
                    .push(button(
                        "Uninstall",
                        Some(PackageMessage::Remove),
                        remove_button,
                    ))
                    .into()
            }
            // TODO: Retry functionality.
            PackageState::Errored { retry_button: _ } => Text::new("Error").into(),
        };

        Container::new(
            Column::new().push(name).push(details).push(
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
        .padding(20)
        .into()
    }
}
