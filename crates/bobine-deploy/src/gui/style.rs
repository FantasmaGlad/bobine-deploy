use iced::widget::{column, container, text, Column};
use iced::{Element, Length};

use super::message::Message;

pub const SPACING: u32 = 12;
pub const PADDING: u16 = 20;
pub const TITLE: u32 = 22;
pub const SUBTITLE: u32 = 15;

pub fn page<'a>(title: &'a str, subtitle: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text(title).size(TITLE),
        text(subtitle).size(SUBTITLE).style(text::secondary),
        body
    ]
    .spacing(SPACING)
    .width(Length::Fill)
    .into()
}

pub fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(PADDING)
        .width(Length::Fill)
        .style(container::bordered_box)
        .into()
}

pub fn error<'a>(message: &Option<String>) -> Column<'a, Message> {
    match message {
        Some(message) => column![text(message.clone()).style(text::danger)],
        None => column![],
    }
}
