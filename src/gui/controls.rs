use super::{sort_by::SortBy, GuiMessage};
use crate::{
    releases::UpdateCount,
    settings::{get_setting, CAN_CONNECT, FETCHING, INSTALLING},
};
use iced::{
    pure::widget::{Button, Checkbox, Column, Container, PickList, Row, Scrollable, Text},
    Alignment, Length, Rule, Space,
};
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub struct Controls {
    pub checking_connection: bool,
}

impl Controls {
    pub fn view(&self, update_count: UpdateCount) -> Container<'_, GuiMessage> {
        let update_button = {
            let button = Button::new(Text::new("[C] Check for updates")).style(get_setting().theme);

            if CAN_CONNECT.load(Ordering::Relaxed)
                && !INSTALLING.load(Ordering::Relaxed)
                && !FETCHING.load(Ordering::Relaxed)
            {
                button.on_press(GuiMessage::CheckForUpdates)
            } else {
                button
            }
        };

        let filter_row = |filter,
                          label,
                          checkbox_message: fn(bool) -> GuiMessage,
                          fetch_button,
                          button_message: Option<GuiMessage>| {
            let row = Row::new()
                .height(Length::Units(25))
                .align_items(Alignment::Center)
                .push(
                    Checkbox::new(filter, label, checkbox_message)
                        .width(Length::Fill)
                        .style(get_setting().theme),
                );
            if fetch_button {
                let button = Button::new(Text::new("[F]")).style(get_setting().theme);

                match button_message {
                    Some(button_message) => {
                        if CAN_CONNECT.load(Ordering::Relaxed)
                            && !INSTALLING.load(Ordering::Relaxed)
                            && !FETCHING.load(Ordering::Relaxed)
                        {
                            row.push(button.on_press(button_message))
                        } else {
                            row.push(button)
                        }
                    }
                    None => row.push(button),
                }
            } else {
                row
            }
        };

        let filters = Column::new()
            .spacing(5)
            .push(filter_row(
                get_setting().filters.updates,
                match update_count.all {
                    Some(count) => {
                        format!("Updates [{}]", count)
                    }
                    None => String::from("Updates"),
                },
                GuiMessage::FilterUpdatesChanged,
                false,
                None,
            ))
            .push(filter_row(
                get_setting().filters.bookmarks,
                String::from("Bookmarks"),
                GuiMessage::FilterBookmarksChanged,
                false,
                None,
            ))
            .push(filter_row(
                get_setting().filters.installed,
                String::from("Installed"),
                GuiMessage::FilterInstalledChanged,
                false,
                None,
            ))
            .push(Rule::horizontal(5).style(get_setting().theme))
            .push(filter_row(
                get_setting().filters.all,
                String::from("All"),
                GuiMessage::FilterAllChanged,
                true,
                Some(GuiMessage::FetchAll),
            ))
            .push(filter_row(
                get_setting().filters.daily_latest,
                match update_count.daily {
                    Some(count) => {
                        format!("Daily (latest) [{}]", count)
                    }
                    None => String::from("Daily (latest)"),
                },
                GuiMessage::FilterDailyLatestChanged,
                true,
                Some(GuiMessage::FetchDailyLatest),
            ))
            .push(filter_row(
                get_setting().filters.daily_archive,
                String::from("Daily (archive)"),
                GuiMessage::FilterDailyArchiveChanged,
                true,
                Some(GuiMessage::FetchDailyArchive),
            ))
            .push(filter_row(
                get_setting().filters.experimental_latest,
                match update_count.experimental {
                    Some(count) => {
                        format!("Experimental (latest) [{}]", count)
                    }
                    None => String::from("Experimental (latest)"),
                },
                GuiMessage::FilterExperimentalLatestChanged,
                true,
                Some(GuiMessage::FetchExperimentalLatest),
            ))
            .push(filter_row(
                get_setting().filters.experimental_archive,
                String::from("Experimental (archive)"),
                GuiMessage::FilterExperimentalArchiveChanged,
                true,
                Some(GuiMessage::FetchExperimentalArchive),
            ))
            .push(filter_row(
                get_setting().filters.patch_latest,
                match update_count.patch {
                    Some(count) => {
                        format!("Patch (latest) [{}]", count)
                    }
                    None => String::from("Patch (latest)"),
                },
                GuiMessage::FilterPatchLatestChanged,
                true,
                Some(GuiMessage::FetchPatchLatest),
            ))
            .push(filter_row(
                get_setting().filters.patch_archive,
                String::from("Patch (archive)"),
                GuiMessage::FilterPatchArchiveChanged,
                true,
                Some(GuiMessage::FetchPatchArchive),
            ))
            .push(filter_row(
                get_setting().filters.stable_latest,
                match update_count.stable {
                    Some(count) => {
                        format!("Stable (latest) [{}]", count)
                    }
                    None => String::from("Stable (latest)"),
                },
                GuiMessage::FilterStableLatestChanged,
                true,
                Some(GuiMessage::FetchStableLatest),
            ))
            .push(filter_row(
                get_setting().filters.stable_archive,
                String::from("Stable (archive)"),
                GuiMessage::FilterStableArchiveChanged,
                true,
                Some(GuiMessage::FetchStableArchive),
            ))
            .push(filter_row(
                get_setting().filters.lts,
                match update_count.lts {
                    Some(count) => {
                        format!("Long-term Support [{}]", count)
                    }
                    None => String::from("Long-term Support"),
                },
                GuiMessage::FilterLtsChanged,
                true,
                Some(GuiMessage::FetchLts),
            ));

        let sorting = Row::new()
            .spacing(8)
            .align_items(Alignment::Center)
            .push(Text::new("Sort by"))
            .push(
                PickList::new(
                    &SortBy::ALL[..],
                    Some(get_setting().sort_by),
                    GuiMessage::SortingChanged,
                )
                .width(Length::Fill)
                .style(get_setting().theme),
            );

        let scrollable = Scrollable::new(
            Column::new()
                .spacing(5)
                .padding(10)
                .align_items(Alignment::Center)
                .push(update_button)
                .push(filters)
                .push(Space::with_height(Length::Units(3)))
                .push(sorting),
        );

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Container::new(scrollable)
                // TODO: Can't get it to shrink around its content for some reason.
                // It always fills the whole space unless I set a specific width.
                .width(Length::Units(230))
                .height(Length::Fill)
                .style(get_setting().theme.sidebar_container())
        } else {
            Container::new(
                Column::new().push(scrollable.height(Length::Fill)).push(
                    Container::new(
                        Row::new()
                            .padding(1)
                            .align_items(Alignment::Center)
                            .push(Space::with_width(Length::Units(9)))
                            .push(Text::new("CANNOT CONNECT").width(Length::Fill))
                            .push({
                                let button = Button::new(Text::new("[R]"))
                                    .style(get_setting().theme.tab_button());

                                if self.checking_connection {
                                    button
                                } else {
                                    button.on_press(GuiMessage::CheckConnection)
                                }
                            })
                            .push(Space::with_width(Length::Units(9))),
                    )
                    .width(Length::Fill)
                    .style(get_setting().theme.status_container()),
                ),
            )
            .width(Length::Units(230))
            .height(Length::Fill)
            .style(get_setting().theme.sidebar_container())
        }
    }
}
