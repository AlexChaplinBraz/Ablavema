use super::TabState;
use crate::{
    gui::message::GuiMessage,
    settings::{get_setting, TEXT_SIZE},
};
use clap::crate_version;
use iced::{button, Align, Button, Column, Container, Element, Length, Row, Space, Text};

#[derive(Debug, Default)]
pub struct AboutState {
    pub repository_link_button: button::State,
    pub discord_link_button: button::State,
    pub contact_link_button: button::State,
    pub donation_link_button: button::State,
}

impl TabState {
    pub fn about_body(&mut self) -> Element<'_, GuiMessage> {
        let link = |label, url, state| {
            Row::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(
                    Text::new(label)
                        .width(Length::Units(100))
                        .color(get_setting().theme.highlight_text()),
                )
                .push(
                    Button::new(state, Text::new(&url))
                        .on_press(GuiMessage::OpenBrowser(url))
                        .style(get_setting().theme),
                )
        };

        Container::new(
            Column::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(Space::with_height(Length::Units(10)))
                .push(
                    Row::new()
                        .spacing(10)
                        .align_items(Align::End)
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
                            &mut self.about.repository_link_button,
                        ))
                        .push(link(
                            "Discord:",
                            String::from("https://discord.gg/D6gmhMUrrH"),
                            &mut self.about.discord_link_button,
                        ))
                        .push(link(
                            "Contact me:",
                            String::from("https://alexchaplinbraz.com/contact"),
                            &mut self.about.contact_link_button,
                        ))
                        .push(link(
                            "Donate:",
                            String::from("https://donate.alexchaplinbraz.com"),
                            &mut self.about.donation_link_button,
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
