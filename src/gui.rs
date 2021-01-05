//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use self::install::{Install, Progress};
use crate::{helpers::*, installed::*, package::*, releases::*, settings::*, style::*};
use iced::{
    button, executor, scrollable, slider, Align, Application, Button, Column, Command, Container,
    Element, HorizontalAlignment, Length, ProgressBar, Radio, Row, Rule, Scrollable, Slider,
    Subscription, Text,
};
use std::fs::remove_dir_all;

#[derive(Debug)]
pub struct GuiArgs {
    pub releases: Releases,
    pub installed: Installed,
    pub updates: Option<Vec<Package>>,
    pub file_path: Option<String>,
}

#[derive(Debug)]
pub struct Gui {
    releases: Releases,
    installed: Installed,
    installing: Vec<(Package, Tab, usize)>,
    updates: Option<Vec<Package>>,
    unpacked_updates: Vec<Package>,
    default_package: Option<Package>,
    file_path: Option<String>,
    tab: Tab,
    scroll: scrollable::State,
    updates_button: button::State,
    installed_button: button::State,
    daily_button: button::State,
    branched_button: button::State,
    lts_button: button::State,
    stable_button: button::State,
    archived_button: button::State,
    settings_button: button::State,
    about_button: button::State,
    minute_slider: slider::State,
    minute_value: f64,
    theme: Theme,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Tab {
    Updates,
    Installed,
    Daily,
    Branched,
    LTS,
    Stable,
    Archived,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    PackageMessage(Tab, usize, PackageMessage),
    PackageInstalled(Result<String, GuiError>),
    PackageInstall(Result<(String, Build), GuiError>),
    PackageRemoved(Result<String, GuiError>),
    ChangeTab(Tab),
    BypassLauncher(Choice),
    ModifierKey(ModifierKey),
    UseLatestAsDefault(Choice),
    CheckUpdatesAtLaunch(Choice),
    MinutesBetweenUpdatesChanged(f64),
    MinutesBetweenUpdates(f64),
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

#[derive(Debug, Clone)]
pub enum PackageMessage {
    Install,
    InstallProgress(Progress),
    Remove,
    Open(String),
    OpenWithFile(String, String),
    SetDefault,
    UnsetDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Enable,
    Disable,
}

impl Choice {
    pub const ALL: [Choice; 2] = [Choice::Enable, Choice::Disable];
}

#[derive(Debug, Clone)]
pub enum GuiError {
    Io,
}

impl Application for Gui {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = GuiArgs;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let installed = flags.installed;

        let default_package = if SETTINGS.read().unwrap().default_package.is_empty() {
            None
        } else {
            Some(
                installed
                    .iter()
                    .find(|p| p.name == SETTINGS.read().unwrap().default_package)
                    .unwrap()
                    .to_owned(),
            )
        };

        (
            Gui {
                releases: flags.releases,
                installed,
                installing: Vec::new(),
                updates: flags.updates,
                unpacked_updates: Vec::new(),
                default_package,
                file_path: flags.file_path,
                tab: Tab::Installed,
                scroll: scrollable::State::new(),
                updates_button: button::State::new(),
                installed_button: button::State::new(),
                daily_button: button::State::new(),
                branched_button: button::State::new(),
                lts_button: button::State::new(),
                stable_button: button::State::new(),
                archived_button: button::State::new(),
                settings_button: button::State::new(),
                about_button: button::State::new(),
                minute_slider: slider::State::new(),
                minute_value: SETTINGS.read().unwrap().minutes_between_updates as f64,
                theme: SETTINGS.read().unwrap().theme,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        let updates = match &self.updates {
            Some(updates) => {
                if updates.is_empty() {
                    String::new()
                } else {
                    let count = updates.iter().count();
                    format!(
                        " - {} {} found!",
                        count,
                        if count == 1 { "package" } else { "packages" }
                    )
                }
            }
            None => String::new(),
        };

        format!("BlenderLauncher{}", updates)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PackageMessage(tab, index, package_message) => match tab {
                Tab::Updates => match self.updates.clone().unwrap().get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Installed => match self.installed.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Daily => match self.releases.daily.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Branched => match self.releases.branched.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::LTS => match self.releases.lts.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Stable => match self.releases.stable.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Archived => match self.releases.archived.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!(),
                },
                Tab::Settings => unreachable!(),
                Tab::About => unreachable!(),
            },
            Message::ChangeTab(tab) => {
                self.tab = tab;

                Command::none()
            }
            Message::PackageInstalled(_package) => {
                /* if package.unwrap() == SETTINGS.read().unwrap().default_package {
                    SETTINGS.write().unwrap().default_package = String::new();
                    SETTINGS.read().unwrap().save();
                } */

                self.installed.check().unwrap();

                Command::none()
            }
            Message::PackageInstall(result) => {
                let (name, build) = result.unwrap();

                match self
                    .unpacked_updates
                    .iter()
                    .enumerate()
                    .find(|(_index, package)| package.name == name)
                {
                    Some((index, package)) => {
                        self.installing
                            .push((package.to_owned(), Tab::Updates, index));
                    }
                    None => match build {
                        Build::Archived => {
                            let (index, package) = self
                                .releases
                                .archived
                                .iter()
                                .enumerate()
                                .find(|(_index, package)| package.name == name)
                                .unwrap();

                            self.installing
                                .push((package.to_owned(), Tab::Archived, index));
                        }
                        Build::Stable => {
                            let (index, package) = self
                                .releases
                                .stable
                                .iter()
                                .enumerate()
                                .find(|(_index, package)| package.name == name)
                                .unwrap();

                            self.installing
                                .push((package.to_owned(), Tab::Stable, index));
                        }
                        Build::LTS => {
                            let (index, package) = self
                                .releases
                                .lts
                                .iter()
                                .enumerate()
                                .find(|(_index, package)| package.name == name)
                                .unwrap();

                            self.installing.push((package.to_owned(), Tab::LTS, index));
                        }
                        Build::Daily(_) => {
                            let (index, package) = self
                                .releases
                                .daily
                                .iter()
                                .enumerate()
                                .find(|(_index, package)| package.name == name)
                                .unwrap();

                            self.installing
                                .push((package.to_owned(), Tab::Daily, index));
                        }
                        Build::Branched(_) => {
                            let (index, package) = self
                                .releases
                                .branched
                                .iter()
                                .enumerate()
                                .find(|(_index, package)| package.name == name)
                                .unwrap();

                            self.installing
                                .push((package.to_owned(), Tab::Branched, index));
                        }
                        Build::None => unreachable!(),
                    },
                }

                Command::none()
            }
            Message::PackageRemoved(package) => {
                if package.unwrap() == SETTINGS.read().unwrap().default_package {
                    SETTINGS.write().unwrap().default_package = String::new();
                    SETTINGS.read().unwrap().save();
                }

                self.installed.check().unwrap();

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
                self.minute_value = minutes;
                Command::none()
            }
            Message::MinutesBetweenUpdates(minutes) => {
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
                Command::none()
            }
            Message::UpdateBranched(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_branched = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_branched = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::UpdateStable(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_stable = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_stable = false,
                }
                SETTINGS.read().unwrap().save();
                Command::none()
            }
            Message::UpdateLts(choice) => {
                match choice {
                    Choice::Enable => SETTINGS.write().unwrap().update_lts = true,
                    Choice::Disable => SETTINGS.write().unwrap().update_lts = false,
                }
                SETTINGS.read().unwrap().save();
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
        Subscription::batch(self.installing.iter().map(|(package, tab, index)| {
            Install::package(package.to_owned(), tab.to_owned(), index.to_owned())
        }))
    }

    fn view(&mut self) -> Element<'_, Message> {
        let self_tab = self.tab;
        let theme = self.theme;

        let top_button = |label, tab, state| {
            let button = Button::new(
                state,
                Text::new(label)
                    .size(16)
                    .horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(theme);

            if tab == self_tab {
                button
            } else {
                button.on_press(Message::ChangeTab(tab))
            }
        };

        let tabs = Container::new(
            Row::new()
                .padding(20)
                .spacing(5)
                .push(top_button(
                    "Updates",
                    Tab::Updates,
                    &mut self.updates_button,
                ))
                .push(top_button(
                    "Installed",
                    Tab::Installed,
                    &mut self.installed_button,
                ))
                .push(top_button("Daily", Tab::Daily, &mut self.daily_button))
                .push(top_button(
                    "Branched",
                    Tab::Branched,
                    &mut self.branched_button,
                ))
                .push(top_button("LTS", Tab::LTS, &mut self.lts_button))
                .push(top_button("Stable", Tab::Stable, &mut self.stable_button))
                .push(top_button(
                    "Archived",
                    Tab::Archived,
                    &mut self.archived_button,
                ))
                .push(top_button(
                    "Settings",
                    Tab::Settings,
                    &mut self.settings_button,
                ))
                .push(top_button("About", Tab::About, &mut self.about_button)),
        )
        .width(Length::Fill)
        .center_x()
        .center_y()
        .style(self.theme.darker_container());

        let info: Element<Message> = Container::new(
            Column::new()
                .width(Length::Fill)
                .padding(20)
                .push(Text::new({
                    match &self.updates {
                        Some(updates) => {
                            if updates.is_empty() {
                                String::from("Updates: no new packages found")
                            } else {
                                let count = updates.iter().count();
                                format!(
                                    "Updates: {} {} found!",
                                    count,
                                    if count == 1 { "package" } else { "packages" }
                                )
                            }
                        }
                        None => String::from("Updates: not the time to check yet"),
                    }
                }))
                .push(Text::new(match &self.file_path {
                    Some(file_path) => format!("File: {}", file_path),
                    None => format!("File: no .blend file to open"),
                }))
                .push(if SETTINGS.read().unwrap().default_package.is_empty() {
                    self.default_package = None;
                    Container::new(Text::new("No default package set").size(50))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .into()
                } else {
                    self.default_package = Some(
                        self.installed
                            .iter()
                            .find(|p| p.name == SETTINGS.read().unwrap().default_package)
                            .unwrap()
                            .clone(),
                    );

                    let (index, package) =
                        self.default_package.iter_mut().enumerate().next().unwrap();

                    let element = package.view(&self.file_path, &self.installed, self.theme);

                    element
                        .map(move |message| Message::PackageMessage(Tab::Installed, index, message))
                }),
        )
        .width(Length::Fill)
        .height(Length::Units(160))
        .style(self.theme.dark_container())
        .into();

        let top_bar = Container::new(Column::new().push(tabs).push(info)).style(self.theme);

        let body: Element<Message> = match self.tab {
            Tab::Updates => {
                if self.updates.is_some() {
                    self.unpacked_updates = self.updates.clone().unwrap();
                }

                packages_body(
                    &mut self.unpacked_updates,
                    Tab::Updates,
                    &self.file_path,
                    &self.installed,
                    &mut self.scroll,
                    self.theme,
                )
            }
            Tab::Installed => {
                let installed = self.installed.clone();

                packages_body(
                    &mut self.installed,
                    Tab::Installed,
                    &self.file_path,
                    &installed,
                    &mut self.scroll,
                    self.theme,
                )
            }
            Tab::Daily => packages_body(
                &mut self.releases.daily,
                Tab::Daily,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
            Tab::Branched => packages_body(
                &mut self.releases.branched,
                Tab::Branched,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
            Tab::LTS => packages_body(
                &mut self.releases.lts,
                Tab::LTS,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
            Tab::Stable => packages_body(
                &mut self.releases.stable,
                Tab::Stable,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
            Tab::Archived => packages_body(
                &mut self.releases.archived,
                Tab::Archived,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
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
                            .push(Text::new(format!("Current: {}", self.minute_value)).width(Length::Units(130)))
                            .push(Slider::new(
                                &mut self.minute_slider,
                                0.0..=1440.0,
                                self.minute_value,
                                Message::MinutesBetweenUpdatesChanged)
                                    .on_release(Message::MinutesBetweenUpdates(self.minute_value))
                                    .style(self.theme)))
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for daily packages",
                            "Check updates for daily packages.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_daily).unwrap()),
                            Message::UpdateDaily,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for branched packages",
                            "Check updates for branched packages.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_branched).unwrap()),
                            Message::UpdateBranched,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for LTS packages",
                            "Check updates for LTS packages.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_lts).unwrap()),
                            Message::UpdateLts,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Check for stable packages",
                            "Check updates for stable packages.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().update_stable).unwrap()),
                            Message::UpdateStable,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest daily package",
                            "Remove all daily packages other than the newest.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_daily).unwrap()),
                            Message::KeepOnlyLatestDaily,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest branched package",
                            "Remove all branched packages other than the newest.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_branched).unwrap()),
                            Message::KeepOnlyLatestBranched,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest LTS package",
                            "Remove all LTS packages other than the newest.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_lts).unwrap()),
                            Message::KeepOnlyLatestLts,
                        )
                    ).push(Rule::horizontal(20).style(self.theme)
                    ).push(
                        choice_setting!(
                            "Keep only newest stable package",
                            "Remove all stable packages other than the newest.",
                            &Choice::ALL,
                            Some(choice(SETTINGS.read().unwrap().keep_only_latest_stable).unwrap()),
                            Message::KeepOnlyLatestStable,
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

                Container::new(Scrollable::new(&mut self.scroll).push(settings))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(theme)
                    .into()
            }
            Tab::About => todo("About tab", self.theme),
        };

        Column::new().push(top_bar).push(body).into()
    }
}

impl Package {
    fn update(&mut self, message: PackageMessage) -> Command<Message> {
        match message {
            PackageMessage::Install => {
                self.state = PackageState::Downloading { progress: 0.0 };
                Command::perform(
                    Package::install(self.name.clone(), self.build.clone()),
                    Message::PackageInstall,
                )
            }
            PackageMessage::InstallProgress(progress) => match progress {
                Progress::Started => Command::none(),
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

                    Command::perform(
                        Package::installed(self.name.clone()),
                        Message::PackageInstalled,
                    )
                }
                Progress::Errored => {
                    self.state = PackageState::Errored {
                        retry_button: Default::default(),
                    };

                    Command::none()
                }
            },
            PackageMessage::Remove => {
                self.state = PackageState::Fetched {
                    install_button: Default::default(),
                };

                Command::perform(Package::remove(self.name.clone()), Message::PackageRemoved)
            }
            PackageMessage::Open(package) => {
                open_blender(package, None).unwrap();
                std::process::exit(0);
            }
            PackageMessage::OpenWithFile(package, file) => {
                open_blender(package, Some(file)).unwrap();
                std::process::exit(0);
            }
            PackageMessage::SetDefault => {
                SETTINGS.write().unwrap().default_package = self.name.clone();
                SETTINGS.read().unwrap().save();

                Command::none()
            }
            PackageMessage::UnsetDefault => {
                SETTINGS.write().unwrap().default_package = String::new();
                SETTINGS.read().unwrap().save();

                Command::none()
            }
        }
    }

    fn view(
        &mut self,
        file_path: &Option<String>,
        installed: &Vec<Package>,
        theme: Theme,
    ) -> Element<PackageMessage> {
        let package_name = Text::new(&self.name).size(30);

        let package_details = Row::new()
            .align_items(Align::End)
            .push(Text::new("Date: ").size(16))
            .push(Text::new(self.date.to_string()).size(20))
            .push(Text::new("        Version: ").size(16))
            .push(Text::new(&self.version).size(20))
            .push(Text::new("        Build: ").size(16))
            .push(Text::new(self.build.to_string()).size(20));

        let package_info = Column::new().push(package_name).push(package_details);

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

        if matches!(self.state, PackageState::Fetched { .. }) && installed.contains(self) {
            self.state = PackageState::Installed {
                open_button: Default::default(),
                open_file_button: Default::default(),
                set_default_button: Default::default(),
                remove_button: Default::default(),
            }
        } else if matches!(self.state, PackageState::Installed { .. }) && !installed.contains(self)
        {
            self.state = PackageState::Fetched {
                install_button: Default::default(),
            }
        }

        let controls: Element<PackageMessage> = match &mut self.state {
            PackageState::Fetched { install_button } => Row::new()
                .push(button(
                    "Install",
                    Some(PackageMessage::Install),
                    install_button,
                ))
                .into(),
            PackageState::Downloading { progress } => Row::new()
                .push(
                    Text::new(format!("Downloading... {:.2}%", progress)).width(Length::Units(200)),
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
                        .push(
                            Text::new(format!("Extracting..."))
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .into()
                } else {
                    Row::new()
                        .push(
                            Text::new(format!("Extracting... {:.2}%", progress))
                                .width(Length::Units(200)),
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
                let button1 = Row::new().push(button(
                    "Open package",
                    Some(PackageMessage::Open(self.name.clone())),
                    open_button,
                ));

                let button2;
                match file_path {
                    Some(file_path) => {
                        button2 = button1.push(button(
                            "Open file with package",
                            Some(PackageMessage::OpenWithFile(
                                self.name.clone(),
                                file_path.clone(),
                            )),
                            open_file_button,
                        ))
                    }
                    None => {
                        button2 =
                            button1.push(button("Open file with package", None, open_file_button))
                    }
                }

                let button3;
                if SETTINGS.read().unwrap().default_package == self.name {
                    button3 = button2.push(button(
                        "Unset default",
                        Some(PackageMessage::UnsetDefault),
                        set_default_button,
                    ));
                } else {
                    button3 = button2.push(button(
                        "Set as default",
                        Some(PackageMessage::SetDefault),
                        set_default_button,
                    ));
                }

                button3
                    .spacing(40)
                    .width(Length::Shrink)
                    .push(button(
                        "Remove",
                        Some(PackageMessage::Remove),
                        remove_button,
                    ))
                    .into()
            }
            PackageState::Errored { retry_button: _ } => Text::new("Error").into(),
        };

        Column::new().push(package_info).push(controls).into()
    }

    async fn install(name: String, build: Build) -> Result<(String, Build), GuiError> {
        Ok((name, build))
    }

    async fn installed(name: String) -> Result<String, GuiError> {
        Ok(name)
    }

    async fn remove(name: String) -> Result<String, GuiError> {
        let path = SETTINGS.read().unwrap().packages_dir.join(&name);

        remove_dir_all(path).map_err(|_| GuiError::Io)?;

        Ok(name)
    }
}

fn packages_body<'a>(
    packages: &'a mut Vec<Package>,
    tab: Tab,
    file_path: &'a Option<String>,
    installed: &Vec<Package>,
    scroll: &'a mut scrollable::State,
    theme: Theme,
) -> Element<'a, Message> {
    if packages.is_empty() {
        Container::new(Text::new("No packages").size(50))
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .style(theme)
            .into()
    } else {
        let packages = packages
            .iter_mut()
            .enumerate()
            .fold(Column::new(), |col, (index, package)| {
                let element = package.view(file_path, &installed, theme);
                col.push(element.map(move |message| Message::PackageMessage(tab, index, message)))
            })
            .width(Length::Fill)
            .spacing(10)
            .padding(20);

        let scrollable = Scrollable::new(scroll).push(packages);

        Container::new(scrollable)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(theme)
            .into()
    }
}

fn todo(unimplemented_part: &str, theme: Theme) -> Element<Message> {
    Container::new(Text::new(format!("{} not yet implemented", unimplemented_part)).size(50))
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .center_y()
        .style(theme)
        .into()
}

pub mod install {
    use crate::{helpers::get_extracted_name, package::*, settings::*};
    use bzip2::read::BzDecoder;
    use flate2::read::GzDecoder;
    use iced_futures::futures;
    use std::{fs::create_dir_all, fs::File, io::Read, io::Write, path::PathBuf};
    use tar::Archive;
    use tokio::fs::{remove_dir_all, remove_file};
    use xz2::read::XzDecoder;
    use zip::{read::ZipFile, ZipArchive};

    use super::{Message, PackageMessage, Tab};

    pub struct Install {
        package: Package,
        tab: Tab,
        index: usize,
    }

    impl Install {
        pub fn package(package: Package, tab: Tab, index: usize) -> iced::Subscription<Message> {
            iced::Subscription::from_recipe(Install {
                package,
                tab,
                index,
            })
            .map(|(tab, index, progress)| {
                Message::PackageMessage(tab, index, PackageMessage::InstallProgress(progress))
            })
        }
    }

    impl<H, I> iced_native::subscription::Recipe<H, I> for Install
    where
        H: std::hash::Hasher,
    {
        type Output = (Tab, usize, Progress);

        fn hash(&self, state: &mut H) {
            use std::hash::Hash;

            std::any::TypeId::of::<Self>().hash(state);
            self.package.name.hash(state);
            self.package.date.hash(state);
        }

        fn stream(
            self: Box<Self>,
            _input: futures::stream::BoxStream<'static, I>,
        ) -> futures::stream::BoxStream<'static, Self::Output> {
            Box::pin(futures::stream::unfold(
                State::ReadyToInstall {
                    package: self.package,
                    tab: self.tab,
                    index: self.index,
                },
                |state| async move {
                    match state {
                        State::ReadyToInstall {
                            package,
                            tab,
                            index,
                        } => {
                            let response = reqwest::get(&package.url).await;

                            match response {
                                Ok(response) => {
                                    if let Some(total) = response.content_length() {
                                        let file = SETTINGS.read().unwrap().cache_dir.join(
                                            package.url.split_terminator('/').last().unwrap(),
                                        );

                                        if file.exists() {
                                            remove_file(&file).await.unwrap();
                                        }

                                        let package_dir = SETTINGS
                                            .read()
                                            .unwrap()
                                            .packages_dir
                                            .join(&package.name);

                                        if package_dir.exists() {
                                            remove_dir_all(&package_dir).await.unwrap();
                                        }

                                        let destination = tokio::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(&file)
                                            .await
                                            .unwrap();

                                        Some((
                                            (tab, index, Progress::Started),
                                            State::Downloading {
                                                package,
                                                response,
                                                file,
                                                destination,
                                                total,
                                                downloaded: 0,
                                                tab,
                                                index,
                                            },
                                        ))
                                    } else {
                                        Some((
                                            (tab, index, Progress::Errored),
                                            State::FinishedInstalling,
                                        ))
                                    }
                                }
                                Err(_) => Some((
                                    (tab, index, Progress::Errored),
                                    State::FinishedInstalling,
                                )),
                            }
                        }
                        State::Downloading {
                            package,
                            mut response,
                            file,
                            mut destination,
                            total,
                            downloaded,
                            tab,
                            index,
                        } => match response.chunk().await {
                            Ok(Some(chunk)) => {
                                tokio::io::AsyncWriteExt::write_all(&mut destination, &chunk)
                                    .await
                                    .unwrap();

                                let downloaded = downloaded + chunk.len() as u64;
                                let percentage = (downloaded as f32 / total as f32) * 100.0;

                                Some((
                                    (tab, index, Progress::DownloadProgress(percentage)),
                                    State::Downloading {
                                        package,
                                        response,
                                        file,
                                        destination,
                                        total,
                                        downloaded,
                                        tab,
                                        index,
                                    },
                                ))
                            }
                            Ok(None) => Some((
                                (tab, index, Progress::FinishedDownloading),
                                State::FinishedDownloading {
                                    package,
                                    file,
                                    tab,
                                    index,
                                },
                            )),
                            Err(_) => {
                                Some(((tab, index, Progress::Errored), State::FinishedInstalling))
                            }
                        },
                        State::FinishedDownloading {
                            package,
                            file,
                            tab,
                            index,
                        } => {
                            let archive = if file.extension().unwrap() == "xz" {
                                DownloadedArchive::TarXz
                            } else if file.extension().unwrap() == "bz2" {
                                DownloadedArchive::TarBz
                            } else if file.extension().unwrap() == "gz" {
                                DownloadedArchive::TarGz
                            } else if file.extension().unwrap() == "zip" {
                                let zip = File::open(&file).unwrap();
                                let archive = ZipArchive::new(zip).unwrap();

                                // This handles some archives that don't have an inner directory.
                                let extraction_dir =
                                    match file.file_name().unwrap().to_str().unwrap() {
                                        "blender-2.49-win64.zip" => SETTINGS
                                            .read()
                                            .unwrap()
                                            .cache_dir
                                            .join("blender-2.49-win64"),
                                        "blender-2.49a-win64-python26.zip" => SETTINGS
                                            .read()
                                            .unwrap()
                                            .cache_dir
                                            .join("blender-2.49a-win64-python26"),
                                        "blender-2.49b-win64-python26.zip" => SETTINGS
                                            .read()
                                            .unwrap()
                                            .cache_dir
                                            .join("blender-2.49b-win64-python26"),
                                        _ => SETTINGS.read().unwrap().cache_dir.clone(),
                                    };

                                let total = archive.len() as u64;

                                DownloadedArchive::Zip {
                                    archive,
                                    extraction_dir,
                                    total,
                                    extracted: 0,
                                }
                            } else if file.extension().unwrap() == "dmg" {
                                todo!("macos extraction");
                            } else {
                                panic!("Unknown archive extension");
                            };

                            Some((
                                (tab, index, Progress::ExtractionProgress(0.0)),
                                State::Extracting {
                                    package,
                                    file,
                                    archive,
                                    tab,
                                    index,
                                },
                            ))
                        }
                        State::Extracting {
                            package,
                            file,
                            archive,
                            tab,
                            index,
                        } => match archive {
                            DownloadedArchive::TarXz => {
                                let tar_xz = File::open(&file).unwrap();
                                let tar = XzDecoder::new(tar_xz);
                                let mut archive = Archive::new(tar);

                                for entry in archive.entries().unwrap() {
                                    let mut file = entry.unwrap();
                                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                                }

                                Some((
                                    (tab, index, Progress::FinishedExtracting),
                                    State::FinishedExtracting {
                                        package,
                                        tab,
                                        index,
                                    },
                                ))
                            }
                            DownloadedArchive::TarGz => {
                                let tar_bz2 = File::open(&file).unwrap();
                                let tar = BzDecoder::new(tar_bz2);
                                let mut archive = Archive::new(tar);

                                for entry in archive.entries().unwrap() {
                                    let mut file = entry.unwrap();
                                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                                }

                                Some((
                                    (tab, index, Progress::FinishedExtracting),
                                    State::FinishedExtracting {
                                        package,
                                        tab,
                                        index,
                                    },
                                ))
                            }
                            DownloadedArchive::TarBz => {
                                let tar_gz = File::open(&file).unwrap();
                                let tar = GzDecoder::new(tar_gz);
                                let mut archive = Archive::new(tar);

                                for entry in archive.entries().unwrap() {
                                    let mut file = entry.unwrap();
                                    file.unpack_in(&SETTINGS.read().unwrap().cache_dir).unwrap();
                                }

                                Some((
                                    (tab, index, Progress::FinishedExtracting),
                                    State::FinishedExtracting {
                                        package,
                                        tab,
                                        index,
                                    },
                                ))
                            }
                            DownloadedArchive::Zip {
                                mut archive,
                                extraction_dir,
                                total,
                                extracted,
                            } => {
                                if extracted == total - 1 {
                                    Some((
                                        (tab, index, Progress::FinishedExtracting),
                                        State::FinishedExtracting {
                                            package,
                                            tab,
                                            index,
                                        },
                                    ))
                                } else {
                                    {
                                        let mut entry: ZipFile =
                                            archive.by_index(extracted as usize).unwrap();
                                        let entry_name = entry.name().to_owned();

                                        if entry.is_dir() {
                                            let extracted_dir_path =
                                                extraction_dir.join(entry_name);
                                            create_dir_all(extracted_dir_path).unwrap();
                                        } else if entry.is_file() {
                                            let mut buffer: Vec<u8> = Vec::new();
                                            let _bytes_read =
                                                entry.read_to_end(&mut buffer).unwrap();
                                            let extracted_file_path =
                                                extraction_dir.join(entry_name);
                                            create_dir_all(extracted_file_path.parent().unwrap())
                                                .unwrap();
                                            let mut file =
                                                File::create(extracted_file_path).unwrap();
                                            file.write(&buffer).unwrap();
                                        }
                                    }

                                    let extracted = extracted + 1;
                                    let percentage = (extracted as f32 / total as f32) * 100.0;

                                    let archive = DownloadedArchive::Zip {
                                        archive,
                                        extraction_dir,
                                        total,
                                        extracted,
                                    };

                                    Some((
                                        (tab, index, Progress::ExtractionProgress(percentage)),
                                        State::Extracting {
                                            package,
                                            file,
                                            archive,
                                            tab,
                                            index,
                                        },
                                    ))
                                }
                            }
                        },
                        State::FinishedExtracting {
                            package,
                            tab,
                            index,
                        } => {
                            let mut package_path =
                                SETTINGS.read().unwrap().packages_dir.join(&package.name);

                            std::fs::rename(
                                SETTINGS
                                    .read()
                                    .unwrap()
                                    .cache_dir
                                    .join(get_extracted_name(&package)),
                                &package_path,
                            )
                            .unwrap();

                            package_path.push("package_info.bin");
                            let file = File::create(&package_path).unwrap();
                            bincode::serialize_into(file, &package).unwrap();

                            Some((
                                (tab, index, Progress::FinishedInstalling),
                                State::FinishedInstalling,
                            ))
                        }
                        State::FinishedInstalling => {
                            let _: () = iced::futures::future::pending().await;

                            None
                        }
                    }
                },
            ))
        }
    }

    #[derive(Debug, Clone)]
    pub enum Progress {
        Started,
        DownloadProgress(f32),
        FinishedDownloading,
        ExtractionProgress(f32),
        FinishedExtracting,
        FinishedInstalling,
        Errored,
    }

    pub enum State {
        ReadyToInstall {
            package: Package,
            tab: Tab,
            index: usize,
        },
        Downloading {
            package: Package,
            response: reqwest::Response,
            file: PathBuf,
            destination: tokio::fs::File,
            total: u64,
            downloaded: u64,
            tab: Tab,
            index: usize,
        },
        FinishedDownloading {
            package: Package,
            file: PathBuf,
            tab: Tab,
            index: usize,
        },
        Extracting {
            package: Package,
            file: PathBuf,
            archive: DownloadedArchive,
            tab: Tab,
            index: usize,
        },
        FinishedExtracting {
            package: Package,
            tab: Tab,
            index: usize,
        },
        FinishedInstalling,
    }

    pub enum DownloadedArchive {
        TarXz, // { entries: Entries<XzDecoder<File>> },
        TarGz, // { entries: Entries<GzDecoder<File>> },
        TarBz, // { entries: Entries<BzDecoder<File>> },
        Zip {
            archive: ZipArchive<File>,
            extraction_dir: PathBuf,
            total: u64,
            extracted: u64,
        },
    }
}
