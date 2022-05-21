use super::Tab;
use crate::{
    gui::message::GuiMessage,
    settings::{get_setting, TEXT_SIZE},
};
use chrono::{DateTime, Local};
use derive_deref::{Deref, DerefMut};
use iced::{
    alignment::Horizontal,
    pure::{
        widget::{Button, Column, Container, Row, Scrollable, Text},
        Element,
    },
    Alignment, Length, Space,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default, Deref, DerefMut, Deserialize, Serialize)]
pub struct RecentFiles(HashMap<PathBuf, RecentFile>);

impl RecentFiles {
    pub fn to_vec(&self) -> Vec<RecentFile> {
        self.values()
            .cloned()
            .into_iter()
            .sorted_by_key(|recent_file| recent_file.last_opened_on)
            .rev()
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecentFile {
    pub name: String,
    pub path: PathBuf,
    pub last_opened_with: String,
    pub last_opened_on: DateTime<Local>,
}

impl RecentFile {
    pub fn new(path: PathBuf, last_opened_with: String) -> Self {
        Self {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path,
            last_opened_with,
            last_opened_on: Local::now(),
        }
    }

    pub fn view(&self, is_odd: bool) -> Element<'_, RecentFileMessage> {
        let name = Row::new().spacing(10).push(
            Text::new(&self.name)
                .color(get_setting().theme.highlight_text())
                .size(TEXT_SIZE + 10)
                .width(Length::Fill),
        );

        let date_time = {
            let mut formatter = timeago::Formatter::new();
            formatter.num_items(2);
            formatter.min_unit(timeago::TimeUnit::Minutes);
            let duration = Local::now().signed_duration_since(self.last_opened_on);
            format!(
                "{} ({})",
                self.last_opened_on.format("%B %d, %Y - %T"),
                formatter.convert(duration.to_std().unwrap_or_default())
            )
        };

        let details = Column::new()
            .push(
                Row::new().push(
                    Row::new()
                        .width(Length::Fill)
                        .align_items(Alignment::End)
                        .push(Text::new("Path: ").size(TEXT_SIZE - 4))
                        .push(
                            Text::new(self.path.to_str().unwrap().to_string())
                                .color(get_setting().theme.highlight_text()),
                        ),
                ),
            )
            .push(
                Row::new()
                    .align_items(Alignment::End)
                    .push(Text::new("Last opened on: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(date_time)
                            .color(get_setting().theme.highlight_text())
                            .width(Length::Fill),
                    ),
            )
            .push(
                Row::new()
                    .align_items(Alignment::End)
                    .push(Text::new("Last opened with: ").size(TEXT_SIZE - 4))
                    .push(
                        Text::new(self.last_opened_with.clone())
                            .color(get_setting().theme.highlight_text()),
                    ),
            );

        let button = |label, recent_file_message: Option<RecentFileMessage>| {
            let button = Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Fill)
                .style(get_setting().theme);

            match recent_file_message {
                Some(recent_file_message) => button.on_press(recent_file_message),
                None => button,
            }
        };

        let controls: Element<'_, RecentFileMessage> = {
            let button1 = Row::new().push(button(
                "[-] Open with last",
                Some(RecentFileMessage::OpenWithLastBlender(
                    self.last_opened_with.clone(),
                )),
            ));

            let button2 = button1.push(button(
                "[=] Open with default",
                if get_setting().default_package.is_some() {
                    Some(RecentFileMessage::OpenWithDefaultBlender)
                } else {
                    None
                },
            ));

            let button3 = button2.push(button("[S] Select", Some(RecentFileMessage::Select)));

            button3
                .spacing(10)
                .push(button("[X] Remove entry", Some(RecentFileMessage::Remove)))
                .into()
        };

        Container::new(
            Column::new()
                .spacing(10)
                .push(name)
                .push(details)
                .push(controls),
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

impl<'a> Tab {
    pub fn recent_files_body(
        file_path: Option<String>,
        recent_files: &'a [RecentFile],
    ) -> Element<'a, GuiMessage> {
        let button = |label, message: Option<GuiMessage>| {
            let button = Button::new(Text::new(label)).style(get_setting().theme);

            match message {
                Some(message) => button.on_press(message),
                None => button,
            }
        };

        let info: Element<'_, GuiMessage> = Container::new(
            Column::new()
                .padding(10)
                .spacing(5)
                .push(
                    Row::new()
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .push(button(
                            "[=]",
                            if get_setting().default_package.is_some() {
                                Some(GuiMessage::OpenBlender(
                                    get_setting().default_package.clone().unwrap().name,
                                ))
                            } else {
                                None
                            },
                        ))
                        .push(Text::new("Default package:"))
                        .push(
                            Text::new(match get_setting().default_package.clone() {
                                Some(package) => package.name,
                                None => String::from("not set"),
                            })
                            .color(get_setting().theme.highlight_text()),
                        ),
                )
                .push(
                    Row::new()
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .push(button(
                            "[+]",
                            if file_path.is_some() && get_setting().default_package.is_some() {
                                Some(GuiMessage::OpenBlenderWithFile(
                                    get_setting().default_package.clone().unwrap().name,
                                ))
                            } else {
                                None
                            },
                        ))
                        .push(Text::new("File:"))
                        .push(
                            Text::new(match &file_path {
                                Some(file_path) => file_path,
                                None => "none",
                            })
                            .color(get_setting().theme.highlight_text()),
                        )
                        .push(Space::with_width(Length::Fill))
                        .push(
                            Button::new(Text::new("Select file"))
                                .on_press(GuiMessage::SelectFile)
                                .style(get_setting().theme),
                        ),
                ),
        )
        .width(Length::Fill)
        .style(get_setting().theme.info_container())
        .into();

        let recent_files_view: Element<'_, GuiMessage> = {
            let mut file_count: u16 = 0;
            let files = Container::new(
                recent_files
                    .iter()
                    .fold(Column::new(), |column, recent_file| {
                        file_count += 1;
                        let path = recent_file.path.to_str().unwrap().to_string();
                        let element = recent_file.view(file_count & 1 != 0);
                        column.push(element.map(move |message| {
                            GuiMessage::RecentFileMessage((path.clone(), message))
                        }))
                    })
                    .width(Length::Fill),
            );

            if file_count == 0 {
                Container::new(Text::new("No recent files").size(TEXT_SIZE * 2))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(get_setting().theme)
                    .into()
            } else {
                Container::new(Scrollable::new(files))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(get_setting().theme.normal_container())
                    .into()
            }
        };

        Container::new(Column::new().push(info).push(recent_files_view))
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .style(get_setting().theme.normal_container())
            .into()
    }
}

#[derive(Clone, Debug)]
pub enum RecentFileMessage {
    OpenWithLastBlender(String),
    OpenWithDefaultBlender,
    Select,
    Remove,
}
