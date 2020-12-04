//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{helpers::*, installed::*, package::*, releases::*, settings::*, style::*};
use iced::{
    button, executor, scrollable, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Scrollable, Text,
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
    updates: Option<Vec<Package>>,
    unpacked_updates: Vec<Package>,
    default_package: Option<Package>,
    file_path: Option<String>,
    tab: Tab,
    scroll: scrollable::State,
    updates_button: button::State,
    installed_button: button::State,
    daily_button: button::State,
    experimental_button: button::State,
    lts_button: button::State,
    stable_button: button::State,
    official_button: button::State,
    settings_button: button::State,
    about_button: button::State,
    theme: Theme,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Tab {
    Updates,
    Installed,
    Daily,
    Experimental,
    LTS,
    Stable,
    Official,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    PackageMessage(Tab, usize, PackageMessage),
    PackageRemoved(Result<String, GuiError>),
    ChangeTab(Tab),
}

#[derive(Debug, Clone)]
pub enum PackageMessage {
    Install,
    Remove,
    Open(String),
    OpenWithFile(String, String),
    SetDefault,
    UnsetDefault,
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
                updates: flags.updates,
                unpacked_updates: Vec::new(),
                default_package,
                file_path: flags.file_path,
                tab: Tab::Installed,
                scroll: scrollable::State::new(),
                updates_button: button::State::new(),
                installed_button: button::State::new(),
                daily_button: button::State::new(),
                experimental_button: button::State::new(),
                lts_button: button::State::new(),
                stable_button: button::State::new(),
                official_button: button::State::new(),
                settings_button: button::State::new(),
                about_button: button::State::new(),
                theme: Theme::Dark,
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
                Tab::Experimental => match self.releases.experimental.get_mut(index) {
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
                Tab::Official => match self.releases.official.get_mut(index) {
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
            Message::PackageRemoved(package) => {
                if package.unwrap() == SETTINGS.read().unwrap().default_package {
                    SETTINGS.write().unwrap().default_package = String::new();
                    SETTINGS.read().unwrap().save();
                }

                self.installed.check().unwrap();

                Command::none()
            }
        }
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
                    "Experimental",
                    Tab::Experimental,
                    &mut self.experimental_button,
                ))
                .push(top_button("LTS", Tab::LTS, &mut self.lts_button))
                .push(top_button("Stable", Tab::Stable, &mut self.stable_button))
                .push(top_button(
                    "Official",
                    Tab::Official,
                    &mut self.official_button,
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
            Tab::Experimental => packages_body(
                &mut self.releases.experimental,
                Tab::Experimental,
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
            Tab::Official => packages_body(
                &mut self.releases.official,
                Tab::Official,
                &self.file_path,
                &self.installed,
                &mut self.scroll,
                self.theme,
            ),
            Tab::Settings => todo("Settings tab", self.theme),
            Tab::About => todo("About tab", self.theme),
        };

        Column::new().push(top_bar).push(body).into()
    }
}

impl Package {
    fn update(&mut self, message: PackageMessage) -> Command<Message> {
        match message {
            PackageMessage::Install => Command::none(),
            PackageMessage::Remove => {
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

        let buttons;
        if installed.contains(self) {
            let button1 = Row::new().push(button(
                "Open package",
                Some(PackageMessage::Open(self.name.clone())),
                &mut self.open_button,
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
                        &mut self.open_file_button,
                    ))
                }
                None => {
                    button2 = button1.push(button(
                        "Open file with package",
                        None,
                        &mut self.open_file_button,
                    ))
                }
            }

            let button3;
            if SETTINGS.read().unwrap().default_package == self.name {
                button3 = button2.push(button(
                    "Unset default",
                    Some(PackageMessage::UnsetDefault),
                    &mut self.set_default_button,
                ));
            } else {
                button3 = button2.push(button(
                    "Set as default",
                    Some(PackageMessage::SetDefault),
                    &mut self.set_default_button,
                ));
            }

            buttons = button3.push(button(
                "Remove",
                Some(PackageMessage::Remove),
                &mut self.remove_button,
            ));
        } else {
            buttons = Row::new().push(button(
                "Install",
                Some(PackageMessage::Install),
                &mut self.install_button,
            ));
        }

        Column::new()
            .push(package_info)
            .push(buttons.spacing(40).width(Length::Shrink))
            .into()
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
