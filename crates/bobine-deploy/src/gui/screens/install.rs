use bobine_deploy_proto::{Channel, Command, Elevation, Error, Event, InstallPhase, InstallRequest, Output};
use iced::widget::{button, checkbox, column, progress_bar, radio, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

use crate::gui::engine;
use crate::gui::message::{Job, Message, Screen};
use crate::gui::state::App;
use crate::gui::style::{self, SPACING};
use crate::text::printable;

const MAX_LOG_LINES: usize = 2000;

pub fn start(app: &mut App, simulate: bool) -> Task<Message> {
    let needs_root = app.target.inspection.as_ref().is_some_and(|i| i.needs_root_password);
    let request = InstallRequest {
        credentials: app.form.credentials(),
        channel: app.options.channel,
        elevation: match app.target.inspection.as_ref().map(|i| i.elevation) {
            Some(elevation) if !simulate => elevation,
            _ => Elevation::Sudo,
        },
        root_password: Some(app.options.root_password.clone()).filter(|p| needs_root && !p.is_empty()),
        no_kiosk: !app.options.kiosk,
        skip_packages: app.options.skip_packages,
        script_path: None,
        simulate,
    };
    app.install = Default::default();
    app.install.running = true;
    app.install.simulated = simulate;
    engine::run(Job::Install, Command::Install(request))
}

pub fn on_event(app: &mut App, event: Event) {
    match event {
        Event::InstallProgress(progress) => app.install.progress = Some(progress),
        Event::InstallLog(line) => {
            app.install.log.push(printable(&line));
            let excess = app.install.log.len().saturating_sub(MAX_LOG_LINES);
            app.install.log.drain(..excess);
        }
        _ => {}
    }
}

pub fn on_finished(app: &mut App, result: Result<Output, Error>) {
    app.install.running = false;
    app.install.outcome = Some(match result {
        Ok(Output::Install(outcome)) => Ok(outcome),
        Ok(_) => Err("Réponse inattendue du moteur.".to_string()),
        Err(error) => Err(error.message),
    });
}

pub fn copy_log(app: &App) -> Task<Message> {
    iced::clipboard::write(app.install.log.join("\n"))
}

pub fn view(app: &App) -> Element<'_, Message> {
    let mut body = column![style::card(options(app))].spacing(SPACING);
    if let Some(progress) = &app.install.progress {
        let failed = progress.phase == InstallPhase::Failed;
        body = body
            .push(progress_bar(0.0..=100.0, f32::from(progress.pct)).style(if failed {
                progress_bar::danger
            } else {
                progress_bar::primary
            }))
            .push(text(format!(
                "{} % · étape {} sur {} : {}",
                progress.pct,
                progress.step_index,
                progress.total_steps,
                printable(&progress.step_title)
            )));
    }
    body = body.push(outcome(app));
    let log = column(app.install.log.iter().map(|line| text(line.clone()).size(13).into()));
    body = body.push(style::card(scrollable(log).height(Length::Fill).anchor_bottom()));
    style::page(
        "Installer Bobine",
        "Bobine Deploy déroule l'installateur de Bobine sur le mini PC.",
        body.into(),
    )
}

fn options(app: &App) -> Element<'_, Message> {
    let channel = |label, value| radio(label, value, Some(app.options.channel), Message::ChannelSelected);
    let needs_root = app.target.inspection.as_ref().is_some_and(|i| i.needs_root_password);
    let busy = app.install.running;
    let ready = app.target.inspection.is_some() && (!needs_root || !app.options.root_password.is_empty());
    let mut content = column![
        row![
            channel("Stable", Channel::Stable),
            channel("Bêta", Channel::Beta),
            channel("Canary", Channel::Canary)
        ]
        .spacing(24),
        checkbox(app.options.kiosk)
            .label("Kiosque plein écran au démarrage")
            .on_toggle(Message::KioskToggled),
        checkbox(app.options.skip_packages)
            .label("Paquets système déjà installés")
            .on_toggle(Message::SkipPackagesToggled),
    ]
    .spacing(SPACING);
    if needs_root {
        content = content.push(
            text_input(
                "Mot de passe root de la machine (élévation par su)",
                &app.options.root_password,
            )
            .on_input(Message::RootPasswordChanged)
            .secure(true),
        );
    }
    content
        .push(
            row![
                button("Retour")
                    .on_press(Message::Show(Screen::Connect))
                    .style(button::secondary),
                button("Installer")
                    .on_press_maybe((ready && !busy).then_some(Message::StartInstall))
                    .style(button::primary),
                button("Simuler")
                    .on_press_maybe((!busy).then_some(Message::Simulate))
                    .style(button::text),
            ]
            .spacing(SPACING),
        )
        .into()
}

fn outcome(app: &App) -> Element<'_, Message> {
    let simulated = if app.install.simulated {
        " (simulation, aucune machine modifiée)"
    } else {
        ""
    };
    let status = match &app.install.outcome {
        None => return column![].into(),
        Some(Ok(done)) => text(match &done.access_url {
            Some(url) => format!("Installation terminée{simulated} : {url}"),
            None => format!("Installation terminée{simulated}."),
        })
        .style(text::success),
        Some(Err(message)) => text(format!("L'installation n'a pas abouti : {message}")).style(text::danger),
    };
    row![
        status.width(Length::Fill),
        button("Copier le journal")
            .on_press(Message::CopyLog)
            .style(button::secondary)
    ]
    .spacing(SPACING)
    .into()
}
