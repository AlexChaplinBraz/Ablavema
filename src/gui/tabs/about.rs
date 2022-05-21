use super::Tab;
use crate::{
    gui::message::GuiMessage,
    settings::{get_setting, TEXT_SIZE},
};
use clap::crate_version;
use iced::{
    pure::{
        widget::{Button, Column, Container, Row, Text},
        Element,
    },
    Alignment, Length, Space,
};

impl Tab {
    pub fn about_body() -> Element<'static, GuiMessage> {
        let link = |label, url| {
            Row::new()
                .spacing(10)
                .align_items(Alignment::Center)
                .push(
                    Text::new(label)
                        .width(Length::Units(100))
                        .color(get_setting().theme.highlight_text()),
                )
                .push(
                    Button::new(Text::new(&url))
                        .on_press(GuiMessage::OpenBrowser(url))
                        .style(get_setting().theme),
                )
        };

        Container::new(
            Column::new()
                .spacing(10)
                .align_items(Alignment::Center)
                .push(Space::with_height(Length::Units(10)))
                .push(
                    Row::new()
                        .spacing(10)
                        .align_items(Alignment::End)
                        .push(Text::new("Ablavema").size(TEXT_SIZE * 3))
                        .push(Text::new(crate_version!()).size(TEXT_SIZE * 2)),
                )
                .push(Text::new("A Blender Launcher and Version Manager").size(TEXT_SIZE * 2))
                .push(
                    Column::new()
                        .spacing(10)
                        .push(Space::with_height(Length::Units(30)))
                        .push(link(
                            "Repository:",
                            String::from("https://github.com/AlexChaplinBraz/Ablavema"),
                        ))
                        .push(link(
                            "Discord:",
                            String::from("https://discord.gg/D6gmhMUrrH"),
                        ))
                        .push(link(
                            "Contact me:",
                            String::from("https://alexchaplinbraz.com/contact"),
                        ))
                        .push(link(
                            "Donate:",
                            String::from("https://donate.alexchaplinbraz.com"),
                        )),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .style(get_setting().theme)
        .into()
    }
}
