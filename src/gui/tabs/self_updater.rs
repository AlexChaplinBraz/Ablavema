use super::Tab;
use crate::{
    gui::{extra::GuiState, message::GuiMessage},
    settings::{get_setting, CAN_CONNECT, TEXT_SIZE},
};
use clap::crate_version;
use iced::{
    pure::{
        widget::{Button, Column, Container, PickList, Row, Scrollable, Text},
        Element,
    },
    Alignment, Length, Space,
};
use self_update::update::Release;
use std::sync::atomic::Ordering;

impl<'a> Tab {
    pub fn self_updater_body(
        gui_state: &'a GuiState,
        self_releases: &'a Option<Vec<Release>>,
    ) -> Element<'a, GuiMessage> {
        let release_index =
            match &self_releases {
                Some(releases) => {
                    match releases.iter().enumerate().find(|(_, release)| {
                        release.version == gui_state.pick_list_selected_releases
                    }) {
                        Some((index, _)) => index,
                        None => 0,
                    }
                }
                None => 0,
            };

        Container::new(
            Column::new()
                .align_items(Alignment::Center)
                .push(
                    Row::new()
                        .align_items(Alignment::Center)
                        .padding(10)
                        .spacing(10)
                        .push(Text::new(format!("Current version: {}", crate_version!())))
                        .push(Text::new("Select version:"))
                        .push(
                            PickList::new(
                                &gui_state.release_versions,
                                Some(gui_state.pick_list_selected_releases.clone()),
                                GuiMessage::PickListVersionSelected,
                            )
                            .width(Length::Units(60))
                            .style(get_setting().theme.normal_pick_list()),
                        )
                        .push(if gui_state.installed_release {
                            Container::new(Text::new("Restart Ablavema."))
                        } else if gui_state.installing_release {
                            Container::new(Text::new("Installing..."))
                        } else if self_releases.is_none() {
                            Container::new({
                                let button = Button::new(Text::new("Fetch releases"))
                                    .style(get_setting().theme);
                                if CAN_CONNECT.load(Ordering::Relaxed)
                                    && !gui_state.fetching_releases
                                {
                                    // TODO: Check connectivity on press.
                                    button.on_press(GuiMessage::FetchSelfReleases)
                                } else {
                                    button
                                }
                            })
                        } else {
                            Container::new({
                                let button = Button::new(Text::new("Install this version"))
                                    .style(get_setting().theme);
                                if gui_state.pick_list_selected_releases == crate_version!()
                                    || !CAN_CONNECT.load(Ordering::Relaxed)
                                {
                                    button
                                } else {
                                    // TODO: Check connectivity on press.
                                    button.on_press(GuiMessage::ChangeVersion)
                                }
                            })
                        }),
                )
                .push(match self_releases {
                    Some(releases) => Container::new(Scrollable::new(
                        Row::new()
                            .push(Space::with_width(Length::Fill))
                            .push(
                                Column::new()
                                    .padding(10)
                                    .spacing(20)
                                    .align_items(Alignment::Center)
                                    .width(Length::FillPortion(50))
                                    .push(
                                        Text::new(&releases[release_index].name)
                                            .size(TEXT_SIZE * 2),
                                    )
                                    .push(Text::new(
                                        releases[release_index].body.as_deref().unwrap_or_default(),
                                    )),
                            )
                            .push(Space::with_width(Length::Fill)),
                    ))
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
