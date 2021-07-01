use super::{install::Progress, Gui, Message};
use crate::{
    package::{Package, PackageState, PackageStatus},
    settings::{get_setting, save_settings, set_setting, CAN_CONNECT, FETCHING, TEXT_SIZE},
};
use iced::{
    Align, Button, Column, Command, Container, Element, HorizontalAlignment, Length, ProgressBar,
    Row, Text,
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
    pub fn update(&mut self, message: PackageMessage) -> Command<Message> {
        match message {
            PackageMessage::Install => Command::perform(
                Gui::check_availability(true, self.clone()),
                Message::CheckAvailability,
            ),
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
                Command::perform(Gui::pass_package(self.clone()), Message::Bookmark)
            }
        }
    }

    pub fn view(&mut self, file_exists: bool, is_odd: bool) -> Element<'_, PackageMessage> {
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
                Button::new(
                    &mut self.bookmark_button,
                    Text::new(if get_setting().bookmarks.contains(&self.name) {
                        "[B]"
                    } else {
                        "[M]"
                    }),
                )
                .on_press(PackageMessage::Bookmark)
                .style(get_setting().theme),
            );

        let details = Column::new()
            .push(
                Row::new()
                    .align_items(Align::End)
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
                            .align_items(Align::End)
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
                    .align_items(Align::End)
                    .push(Text::new("Build: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(self.build_type.to_string())
                            .color(get_setting().theme.highlight_text()),
                    ),
            );

        let button = |label, package_message: Option<PackageMessage>, state| {
            let button = Button::new(
                state,
                Text::new(label).horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::Fill)
            .style(get_setting().theme);

            match package_message {
                Some(package_message) => button.on_press(package_message),
                None => button,
            }
        };

        let controls: Element<'_, PackageMessage> = match &mut self.state {
            PackageState::Fetched { install_button } => Row::new()
                .push(button(
                    "[#] Install",
                    if CAN_CONNECT.load(Ordering::Relaxed) && !FETCHING.load(Ordering::Relaxed) {
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
                        .style(get_setting().theme),
                )
                .push(
                    Button::new(cancel_button, Text::new("Cancel"))
                        .on_press(PackageMessage::Cancel)
                        .style(get_setting().theme),
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
                            Text::new("Extracting...")
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
