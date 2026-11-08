//! Connexion : clé d'hôte vérifiée avant tout mot de passe, puis audit de la cible.

use bobine_deploy_proto::{
    Command, Error, HostKeyInfo, HostKeyRequest, HostKeyStatus, Inspection, Output, TrustHostKeyRequest,
};
use iced::widget::{button, checkbox, column, row, text, text_input};
use iced::{Element, Length, Task};

use crate::gui::engine;
use crate::gui::message::{Job, Message, Screen};
use crate::gui::state::App;
use crate::gui::style::{self, SPACING};
use crate::text::printable;

pub fn check_host_key(app: &mut App) -> Task<Message> {
    app.target = Default::default();
    app.target.busy = true;
    let request = HostKeyRequest {
        host: app.form.host.trim().to_string(),
        port: app.form.port(),
    };
    engine::run(Job::HostKey, Command::HostKey(request))
}

pub fn trust(app: &mut App) -> Task<Message> {
    let Some(info) = &app.target.host_key else {
        return Task::none();
    };
    let request = TrustHostKeyRequest {
        host: info.host.clone(),
        port: info.port,
        fingerprint: info.fingerprint.clone(),
    };
    app.target.busy = true;
    engine::run(Job::TrustHostKey, Command::TrustHostKey(request))
}

fn inspect(app: &mut App) -> Task<Message> {
    app.target.busy = true;
    engine::run(Job::Inspect, Command::Inspect(app.form.credentials()))
}

pub fn on_finished(app: &mut App, job: Job, result: Result<Output, Error>) -> Task<Message> {
    app.target.busy = false;
    match result {
        Ok(Output::HostKey(info)) => {
            let known = info.status == HostKeyStatus::Known;
            app.target.host_key = Some(info);
            if known {
                return inspect(app);
            }
        }
        Ok(Output::Inspect(report)) => {
            app.options.root_password.clear();
            app.target.inspection = Some(report);
        }
        Ok(_) => {}
        Err(error) => {
            app.target.error = Some(error.message);
            if job == Job::Inspect {
                app.target.inspection = None;
            }
        }
    }
    Task::none()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let form = &app.form;
    let key_label = form
        .key_path
        .as_deref()
        .map(|p| format!("Clé : {p}"))
        .unwrap_or_else(|| "Aucune clé choisie".into());
    let fields = column![
        row![
            text_input("Adresse ou nom (bobine.local)", &form.host).on_input(Message::HostChanged),
            text_input("Port", &form.port)
                .on_input(Message::PortChanged)
                .width(Length::Fixed(90.0)),
        ]
        .spacing(SPACING),
        text_input("Utilisateur SSH", &form.user).on_input(Message::UserChanged),
        text_input("Mot de passe SSH (ou phrase de passe de la clé)", &form.password)
            .on_input(Message::PasswordChanged)
            .secure(true),
        row![
            button("Choisir une clé…")
                .on_press(Message::PickKey)
                .style(button::secondary),
            button("Retirer")
                .on_press_maybe(form.key_path.as_ref().map(|_| Message::ClearKey))
                .style(button::text),
            text(key_label).style(text::secondary),
        ]
        .spacing(SPACING),
        checkbox(form.local_keys)
            .label("Présenter aussi l'agent SSH et les clés de ~/.ssh")
            .on_toggle(Message::LocalKeysToggled),
        row![
            button("Retour")
                .on_press(Message::Show(Screen::Scan))
                .style(button::secondary),
            button(if app.target.busy {
                "Vérification…"
            } else {
                "Vérifier la machine"
            })
            .on_press_maybe((form.is_complete() && !app.target.busy).then_some(Message::CheckHostKey))
            .style(button::primary),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING);

    let mut body = column![style::card(fields)].spacing(SPACING);
    if let Some(info) = &app.target.host_key {
        body = body.push(host_key_card(info, app.target.busy));
    }
    body = body.push(style::error(&app.target.error));
    if let Some(report) = &app.target.inspection {
        body = body.push(inspection_card(report));
    }
    style::page(
        "Se connecter au mini PC",
        "La clé d'hôte est vérifiée avant l'envoi de tout mot de passe.",
        body.into(),
    )
}

fn host_key_card(info: &HostKeyInfo, busy: bool) -> Element<'_, Message> {
    let content = match info.status {
        HostKeyStatus::Known => column![text(format!("Machine connue : {} {}", info.key_type, info.fingerprint))],
        HostKeyStatus::Changed => column![text(format!(
            "La clé de cette machine a changé ({}) : connexion refusée. Si elle a été réinstallée, retirez l'ancienne \
             entrée de {} puis recommencez.",
            info.fingerprint, info.known_hosts
        ))
        .style(text::danger)],
        HostKeyStatus::Unknown => column![
            text("Machine inconnue : comparez cette empreinte avec celle affichée sur le mini PC."),
            text(format!("{}  {}", info.key_type, info.fingerprint)).size(16),
            text("Sur le mini PC : ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub").style(text::secondary),
            button("L'empreinte est identique : faire confiance")
                .on_press_maybe((!busy).then_some(Message::TrustHostKey))
                .style(button::primary),
        ]
        .spacing(SPACING),
    };
    style::card(content)
}

fn inspection_card(report: &Inspection) -> Element<'_, Message> {
    let line =
        |label: &'static str, value: String| row![text(label).width(Length::Fixed(140.0)), text(printable(&value))];
    let elevation = if report.needs_root_password {
        "su (mot de passe root requis)".to_string()
    } else {
        "sudo".to_string()
    };
    style::card(
        column![
            line("Système", report.os_name.clone()),
            line("Architecture", report.arch.clone()),
            line("Processeur", report.cpu_model.clone()),
            line(
                "Mémoire",
                format!("{} Mo, disque libre {} Go", report.memory_total_mb, report.disk_free_gb)
            ),
            line("Graphismes", report.gpu_info.clone()),
            line("Élévation", elevation),
            button("Continuer vers l'installation")
                .on_press(Message::Show(Screen::Install))
                .style(button::primary),
        ]
        .spacing(8),
    )
}
