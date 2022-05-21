use super::{install::Progress, Gui, GuiMessage};
use crate::{
    package::{Package, PackageState, PackageStatus},
    settings::{get_setting, save_settings, set_setting, CAN_CONNECT, FETCHING, TEXT_SIZE},
};
use iced::{
    alignment::Horizontal,
    pure::{
        widget::{Button, Column, Container, Row, Text},
        Element,
    },
    Alignment, Command, Length, ProgressBar,
};
use std::sync::atomic::Ordering;

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
    pub fn update(&mut self, message: PackageMessage) -> Command<GuiMessage> {
        match message {
            PackageMessage::Install => Command::perform(
                Gui::check_availability(true, self.clone()),
                GuiMessage::CheckAvailability,
            ),
            PackageMessage::InstallationProgress(progress) => match progress {
                Progress::Started => {
                    self.state = PackageState::Downloading { progress: 0.0 };
                    Command::none()
                }
                Progress::DownloadProgress(progress) => {
                    if let PackageState::Downloading { .. } = self.state {
                        self.state = PackageState::Downloading { progress };
                    }
                    Command::none()
                }
                Progress::FinishedDownloading => {
                    self.state = PackageState::Extracting { progress: 0.0 };
                    Command::none()
                }
                Progress::ExtractionProgress(progress) => {
                    if let PackageState::Extracting { .. } = self.state {
                        self.state = PackageState::Extracting { progress };
                    }
                    Command::none()
                }
                Progress::FinishedExtracting => Command::none(),
                Progress::FinishedInstalling => {
                    self.state = PackageState::Installed;
                    Command::perform(
                        Gui::pass_package(self.clone()),
                        GuiMessage::PackageInstalled,
                    )
                }
                Progress::Errored(message) => {
                    self.state = PackageState::Errored { message };
                    Command::perform(Gui::pass_package(self.clone()), GuiMessage::CancelInstall)
                }
            },
            PackageMessage::Cancel => {
                self.state = PackageState::default();
                Command::perform(Gui::pass_package(self.clone()), GuiMessage::CancelInstall)
            }
            PackageMessage::Remove => {
                self.remove();
                Command::perform(Gui::pass_package(self.clone()), GuiMessage::PackageRemoved)
            }
            PackageMessage::OpenBlender => {
                Command::perform(Gui::pass_string(self.name.clone()), GuiMessage::OpenBlender)
            }

            PackageMessage::OpenBlenderWithFile => Command::perform(
                Gui::pass_string(self.name.clone()),
                GuiMessage::OpenBlenderWithFile,
            ),
            PackageMessage::SetDefault => {
                set_setting().default_package = Some(self.clone());
                save_settings();
                Command::none()
            }
            PackageMessage::UnsetDefault => {
                set_setting().default_package = None;
                save_settings();
                Command::none()
            }
            PackageMessage::Bookmark => {
                Command::perform(Gui::pass_package(self.clone()), GuiMessage::Bookmark)
            }
        }
    }

    pub fn view(&self, file_exists: bool, is_odd: bool) -> Element<'_, PackageMessage> {
        let is_default_package = get_setting().default_package.is_some()
            && get_setting().default_package.clone().unwrap() == *self;

        let date_time = self.get_formatted_date_time();

        let name = Row::new()
            .spacing(10)
            .push(
                Text::new(&self.name)
                    .color(get_setting().theme.highlight_text())
                    .size(TEXT_SIZE + 10)
                    .width(Length::Fill),
            )
            .push(
                Button::new(Text::new(if get_setting().bookmarks.contains(&self.name) {
                    "[B]"
                } else {
                    "[M]"
                }))
                .on_press(PackageMessage::Bookmark)
                .style(get_setting().theme),
            );

        let details = Column::new()
            .push(
                Row::new()
                    .align_items(Alignment::End)
                    .push(Text::new("Date: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(date_time)
                            .color(get_setting().theme.highlight_text())
                            .width(Length::Fill),
                    ),
            )
            .push(
                Row::new()
                    .push(
                        Row::new()
                            .width(Length::Fill)
                            .align_items(Alignment::End)
                            .push(Text::new("Version: ").size(TEXT_SIZE - 4))
                            .push(
                                Text::new(self.version.to_string())
                                    .color(get_setting().theme.highlight_text()),
                            ),
                    )
                    .push(
                        Text::new(match self.status {
                            PackageStatus::Update => "UPDATE   ",
                            PackageStatus::New => "NEW   ",
                            PackageStatus::Old => "",
                        })
                        .color(get_setting().theme.highlight_text())
                        .size(TEXT_SIZE + 4),
                    ),
            )
            .push(
                Row::new()
                    .align_items(Alignment::End)
                    .push(Text::new("Build: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(self.build_type.to_string())
                            .color(get_setting().theme.highlight_text()),
                    ),
            );

        let button = |label, package_message: Option<PackageMessage>| {
            let button = Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Fill)
                .style(get_setting().theme);

            match package_message {
                Some(package_message) => button.on_press(package_message),
                None => button,
            }
        };

        let controls: Element<'_, PackageMessage> = match &self.state {
            PackageState::Fetched => Row::new()
                .push(button(
                    "[#] Install",
                    if CAN_CONNECT.load(Ordering::Relaxed) && !FETCHING.load(Ordering::Relaxed) {
                        Some(PackageMessage::Install)
                    } else {
                        None
                    },
                ))
                .into(),
            PackageState::Downloading { progress } => Row::new()
                .spacing(10)
                .align_items(Alignment::Center)
                .push(Text::new(format!("Downloading... {:.2}%", progress)))
                .push(
                    ProgressBar::new(0.0..=100.0, *progress)
                        .width(Length::Fill)
                        .style(get_setting().theme),
                )
                .push(
                    Button::new(Text::new("Cancel"))
                        .on_press(PackageMessage::Cancel)
                        .style(get_setting().theme),
                )
                .into(),
            PackageState::Extracting { progress } => {
                // TODO: Figure out why cancelling doesn't work for extraction.
                // It does visually get cancelled, but the extraction keeps going in the
                // background, ultimately getting installed. But since the package was supposedly
                // removed from the installation process, the program crashes at the end when it
                // tries that, since it's no longer there. The same behaviour happens on Windows,
                // where the extraction works differently. I thought maybe the download kept going
                // as well, but no, that stops as intended when cancelled.
                if cfg!(target_os = "linux") {
                    Row::new()
                        .align_items(Alignment::Center)
                        .push(
                            Text::new("Extracting...")
                                .width(Length::Fill)
                                .horizontal_alignment(Horizontal::Center),
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
                        .align_items(Alignment::Center)
                        .push(Text::new(format!("Extracting... {:.2}%", progress)))
                        .push(
                            ProgressBar::new(0.0..=100.0, *progress)
                                .width(Length::Fill)
                                .style(get_setting().theme),
                        )
                        /* .push(
                            Button::new(cancel_button, Text::new("Cancel"))
                                .on_press(PackageMessage::Cancel)
                                .style(theme),
                        ) */
                        .into()
                }
            }
            PackageState::Installed => {
                let button1 =
                    Row::new().push(button("[=] Open", Some(PackageMessage::OpenBlender)));

                let button2 = button1.push(button(
                    "[+] Open file",
                    if file_exists {
                        Some(PackageMessage::OpenBlenderWithFile)
                    } else {
                        None
                    },
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
                ));

                button3
                    .spacing(10)
                    .push(button("[X] Uninstall", Some(PackageMessage::Remove)))
                    .into()
            }
            PackageState::Errored {
                message: error_message,
            } => Row::new()
                .spacing(10)
                .align_items(Alignment::Center)
                .push(Text::new(format!("Error: {}.", error_message)).width(Length::Fill))
                .push(
                    Button::new(Text::new("Retry"))
                        // TODO: Disable if can't connect or fetching.
                        .on_press(PackageMessage::Install)
                        .style(get_setting().theme),
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
                get_setting().theme.odd_container()
            } else {
                get_setting().theme.even_container()
            }
        })
        .padding(10)
        .into()
    }
}
