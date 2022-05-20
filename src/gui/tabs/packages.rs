use super::TabState;
use crate::{
    gui::{controls::Controls, message::GuiMessage},
    package::Package,
    releases::UpdateCount,
    settings::{get_setting, FETCHING, TEXT_SIZE},
};
use iced::{
    button, scrollable, Align, Button, Column, Container, Element, Length, Row, Scrollable, Space,
    Text,
};
use itertools::Itertools;
use std::sync::atomic::Ordering;

#[derive(Debug, Default)]
pub struct PackagesState {
    pub open_default_button: button::State,
    pub open_default_with_file_button: button::State,
    pub select_file_button: button::State,
    pub scroll: scrollable::State,
}

impl<'a> TabState {
    pub fn packages_body(
        &'a mut self,
        packages: &'a mut [Package],
        file_path: Option<String>,
        update_count: UpdateCount,
        file_exists: bool,
        controls: &'a mut Controls,
    ) -> Element<'a, GuiMessage> {
        // TODO: Use real icons for the buttons.
        // TODO: Add tooltips.
        let button = |label, message: Option<GuiMessage>, state| {
            let button = Button::new(state, Text::new(label)).style(get_setting().theme);

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
                        .align_items(Align::Center)
                        .push(button(
                            "[=]",
                            if get_setting().default_package.is_some() {
                                Some(GuiMessage::OpenBlender(
                                    get_setting().default_package.clone().unwrap().name,
                                ))
                            } else {
                                None
                            },
                            &mut self.packages.open_default_button,
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
                        .align_items(Align::Center)
                        .push(button(
                            "[+]",
                            if file_path.is_some() && get_setting().default_package.is_some() {
                                Some(GuiMessage::OpenBlenderWithFile(
                                    get_setting().default_package.clone().unwrap().name,
                                ))
                            } else {
                                None
                            },
                            &mut self.packages.open_default_with_file_button,
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
                            Button::new(
                                &mut self.packages.select_file_button,
                                Text::new("Select file"),
                            )
                            .on_press(GuiMessage::SelectFile)
                            .style(get_setting().theme),
                        ),
                ),
        )
        .width(Length::Fill)
        .style(get_setting().theme.info_container())
        .into();

        let packages: Element<'_, GuiMessage> = {
            let mut package_count: u16 = 0;
            let filtered_packages = Container::new(
                packages
                    .iter_mut()
                    .filter(|package| get_setting().filters.matches(package))
                    .sorted_by(|a, b| get_setting().sort_by.get_ordering(a, b))
                    .fold(Column::new(), |column, package| {
                        package_count += 1;
                        let index = package.index;
                        let element = package.view(file_exists, package_count & 1 != 0);
                        column.push(
                            element
                                .map(move |message| GuiMessage::PackageMessage((index, message))),
                        )
                    })
                    .width(Length::Fill),
            );

            let scrollable = Scrollable::new(&mut self.packages.scroll).push(filtered_packages);

            if package_count == 0 {
                Container::new(
                    Text::new({
                        if FETCHING.load(Ordering::Relaxed) {
                            "Fetching..."
                        } else {
                            "No packages"
                        }
                    })
                    .size(TEXT_SIZE * 2),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .center_y()
                .style(get_setting().theme)
                .into()
            } else {
                Container::new(scrollable)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(get_setting().theme)
                    .into()
            }
        };

        Container::new(
            Column::new()
                .push(info)
                .push(Row::new().push(controls.view(update_count)).push(packages)),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .style(get_setting().theme)
        .into()
    }
}
