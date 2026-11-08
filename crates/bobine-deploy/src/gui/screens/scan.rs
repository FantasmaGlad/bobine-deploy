//! Recherche : tout ce qui répond sur le réseau, les cibles probables en tête.

use bobine_deploy_proto::{Command, Device, Event, Output, ScanRequest};
use iced::widget::{button, column, progress_bar, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

use crate::gui::engine;
use crate::gui::message::{Job, Message, Screen};
use crate::gui::state::App;
use crate::gui::style::{self, SPACING};
use crate::text::printable;

pub fn start(app: &mut App) -> Task<Message> {
    app.scan.running = true;
    app.scan.devices.clear();
    app.scan.progress = None;
    app.scan.error = None;
    let subnet = Some(app.scan.subnet.trim().to_string()).filter(|s| !s.is_empty());
    engine::run(Job::Scan, Command::Scan(ScanRequest { subnet }))
}

pub fn on_event(app: &mut App, event: Event) {
    match event {
        Event::ScanProgress(progress) => app.scan.progress = Some(progress),
        Event::ScanDevice(device) if !app.scan.devices.iter().any(|d| d.ip == device.ip) => {
            app.scan.devices.push(device);
        }
        _ => {}
    }
}

pub fn on_finished(app: &mut App, result: Result<Output, bobine_deploy_proto::Error>) {
    app.scan.running = false;
    match result {
        Ok(Output::Scan { devices }) => app.scan.devices = devices,
        Ok(_) => {}
        Err(error) => app.scan.error = Some(error.message),
    }
}

pub fn select(app: &mut App, ip: String) {
    app.form.host = ip;
    app.target = Default::default();
    app.screen = Screen::Connect;
}

pub fn view(app: &App) -> Element<'_, Message> {
    let controls = row![
        text_input("Réseau à sonder (facultatif) : 192.168.2.0/24", &app.scan.subnet)
            .on_input(Message::SubnetChanged)
            .on_submit(Message::StartScan),
        button(if app.scan.running { "Recherche…" } else { "Rechercher" })
            .on_press_maybe((!app.scan.running).then_some(Message::StartScan))
            .style(button::primary),
        button("Saisir une adresse")
            .on_press(Message::Show(Screen::Connect))
            .style(button::secondary),
    ]
    .spacing(SPACING);

    let mut body = column![controls].spacing(SPACING);
    if let Some(progress) = &app.scan.progress {
        let value = if progress.total == 0 {
            0.0
        } else {
            progress.done as f32 / progress.total as f32
        };
        body = body.push(progress_bar(0.0..=1.0, value)).push(
            text(format!(
                "{}/{} adresses, {} appareil(s) · {}",
                progress.done,
                progress.total,
                progress.found,
                progress.networks.join(", ")
            ))
            .style(text::secondary),
        );
    }
    body = body.push(style::error(&app.scan.error));
    body = body.push(scrollable(column(app.scan.devices.iter().map(device_row)).spacing(6)).height(Length::Fill));
    style::page(
        "Trouver la machine",
        "Bobine Deploy sonde le réseau local ; choisissez le mini PC sur lequel installer Bobine.",
        body.into(),
    )
}

fn device_row(device: &Device) -> Element<'_, Message> {
    let name = printable(device.hostname.as_deref().unwrap_or("Nom inconnu"));
    let mut tags = Vec::new();
    if device.bobine_open {
        tags.push("Bobine");
    }
    if device.ssh_open {
        tags.push("SSH");
    }
    let detail = format!(
        "{}{}{}",
        device.os_hint.as_deref().map(printable).unwrap_or_default(),
        if tags.is_empty() {
            String::new()
        } else {
            format!("  ·  {}", tags.join(", "))
        },
        device.mac.as_deref().map(|m| format!("  ·  {m}")).unwrap_or_default()
    );
    let content = row![
        column![text(device.ip.clone()).size(16), text(name).style(text::secondary)].width(Length::Fixed(260.0)),
        text(detail).style(text::secondary).width(Length::Fill),
    ]
    .spacing(SPACING);
    button(content)
        .width(Length::Fill)
        .on_press(Message::SelectDevice(device.ip.clone()))
        .style(if device.is_wyse_or_bobine {
            button::secondary
        } else {
            button::text
        })
        .into()
}
