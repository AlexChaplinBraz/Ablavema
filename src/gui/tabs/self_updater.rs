use super::TabState;
use crate::{
    gui::message::Message,
    settings::{get_setting, CAN_CONNECT, TEXT_SIZE},
};
use clap::crate_version;
use iced::{
    button, pick_list, scrollable, Align, Button, Column, Container, Element, Length, PickList,
    Row, Scrollable, Space, Text,
};
use self_update::update::Release;
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub struct SelfUpdaterState {
    pub release_versions: Vec<String>,
    pub fetch_button: button::State,
    pub fetching: bool,
    pub pick_list: pick_list::State<String>,
    pub pick_list_selected: String,
    pub install_button: button::State,
    pub installing: bool,
    pub installed: bool,
    pub scroll: scrollable::State,
}

impl SelfUpdaterState {
    pub fn new() -> Self {
        Self {
            pick_list_selected: crate_version!().to_string(),
            ..Default::default()
        }
    }
}

impl TabState {
    pub fn self_updater_body(
        &mut self,
        self_releases: &mut Option<Vec<Release>>,
    ) -> Element<'_, Message> {
        let self_updater_pick_list_selected = self.self_updater.pick_list_selected.clone();

        let release_index = match &self_releases {
            Some(releases) => {
                match releases
                    .iter()
                    .enumerate()
                    .find(|(_, release)| release.version == self_updater_pick_list_selected)
                {
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
                                &mut self.self_updater.pick_list,
                                &self.self_updater.release_versions,
                                Some(self_updater_pick_list_selected),
                                Message::PickListVersionSelected,
                            )
                            .width(Length::Units(60))
                            .style(get_setting().theme),
                        )
                        .push(if self.self_updater.installed {
                            Container::new(Text::new("Restart Ablavema."))
                        } else if self.self_updater.installing {
                            Container::new(Text::new("Installing..."))
                        } else if self_releases.is_none() {
                            Container::new({
                                let button = Button::new(
                                    &mut self.self_updater.fetch_button,
                                    Text::new("Fetch releases"),
                                )
                                .style(get_setting().theme);
                                if CAN_CONNECT.load(Ordering::Relaxed)
                                    && !self.self_updater.fetching
                                {
                                    // TODO: Check connectivity on press.
                                    button.on_press(Message::FetchSelfReleases)
                                } else {
                                    button
                                }
                            })
                        } else {
                            Container::new({
                                let button = Button::new(
                                    &mut self.self_updater.install_button,
                                    Text::new("Install this version"),
                                )
                                .style(get_setting().theme);
                                if self.self_updater.pick_list_selected == crate_version!()
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
                .push(match &self_releases {
                    Some(releases) => Container::new(
                        Scrollable::new(&mut self.self_updater.scroll).push(
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
                    .style(get_setting().theme),
                    None => Container::new(Space::new(Length::Fill, Length::Fill))
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .style(get_setting().theme),
                }),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .style(get_setting().theme.sidebar_container())
        .into()
    }
}
