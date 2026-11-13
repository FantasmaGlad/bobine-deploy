mod engine;
mod message;
mod renderer;
mod screens;
mod state;
mod style;

use bobine_deploy_proto::{Command, Event, Output};
use iced::widget::{column, container, image, row, text};
use iced::{window, Element, Length, Size, Task, Theme};

use message::{Job, Message, Screen};
use screens::{connect, install, scan};
use state::{App, TargetState};

#[cfg(test)]
pub const COMMANDS: &[&str] = &["scan", "host-key", "trust-host-key", "inspect", "install", "version"];

const LOGO: &[u8] = include_bytes!("../../assets/icons/256x256.png");

pub fn main() -> i32 {
    let renderer = renderer::select();
    let launched = std::panic::catch_unwind(launch);
    match (launched, renderer) {
        (Ok(code), _) => code,
        (Err(_), renderer::Renderer::Gpu { .. }) => renderer::relaunch_on_cpu(),
        (Err(_), renderer::Renderer::Cpu { .. }) => 1,
    }
}

fn launch() -> i32 {
    let result = iced::application(boot, update, view)
        .title("Bobine Deploy")
        .theme(theme)
        .window(window::Settings {
            size: Size::new(1040.0, 760.0),
            min_size: Some(Size::new(860.0, 620.0)),
            icon: window_icon(),
            ..Default::default()
        })
        .run();
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("bobine-deploy : interface indisponible ({error}) ; la ligne de commande reste utilisable.");
            1
        }
    }
}

fn window_icon() -> Option<window::Icon> {
    let pixels = ::image::load_from_memory_with_format(LOGO, ::image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let (width, height) = pixels.dimensions();
    window::icon::from_rgba(pixels.into_raw(), width, height).ok()
}

fn boot() -> (App, Task<Message>) {
    (App::default(), engine::run(Job::Version, Command::Version))
}

fn theme(_: &App) -> Theme {
    Theme::Light
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Event(event @ (Event::ScanProgress(_) | Event::ScanDevice(_))) => scan::on_event(app, event),
        Message::Event(event) => install::on_event(app, event),
        Message::Finished(job, result) => return finished(app, job, result),
        Message::Show(screen) => app.screen = screen,
        Message::SubnetChanged(subnet) => app.scan.subnet = subnet,
        Message::StartScan => return scan::start(app),
        Message::SelectDevice(ip) => scan::select(app, ip),
        Message::HostChanged(host) => {
            app.form.host = host;
            app.target = TargetState::default();
        }
        Message::PortChanged(port) => {
            app.form.port = port.chars().filter(char::is_ascii_digit).take(5).collect();
            app.target = TargetState::default();
        }
        Message::UserChanged(user) => {
            app.form.user = user;
            app.target = TargetState::default();
        }
        Message::PasswordChanged(password) => app.form.password = password,
        Message::PickKey => return pick_key(),
        Message::KeyPicked(path) => {
            if let Some(path) = path {
                app.form.key_path = Some(path.display().to_string());
            }
        }
        Message::ClearKey => app.form.key_path = None,
        Message::LocalKeysToggled(on) => app.form.local_keys = on,
        Message::CheckHostKey => return connect::check_host_key(app),
        Message::TrustHostKey => return connect::trust(app),
        Message::ChannelSelected(channel) => app.options.channel = channel,
        Message::KioskToggled(on) => app.options.kiosk = on,
        Message::SkipPackagesToggled(on) => app.options.skip_packages = on,
        Message::RootPasswordChanged(password) => app.options.root_password = password,
        Message::StartInstall => return install::start(app, false),
        Message::Simulate => return install::start(app, true),
        Message::CopyLog => return install::copy_log(app),
    }
    Task::none()
}

fn finished(app: &mut App, job: Job, result: Result<Output, bobine_deploy_proto::Error>) -> Task<Message> {
    match job {
        Job::Version => {
            if let Ok(Output::Version(info)) = result {
                app.version = Some(info);
            }
        }
        Job::Scan => scan::on_finished(app, result),
        Job::HostKey | Job::TrustHostKey | Job::Inspect => return connect::on_finished(app, job, result),
        Job::Install => install::on_finished(app, result),
    }
    Task::none()
}

fn pick_key() -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Choisir une clé privée SSH")
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        },
        Message::KeyPicked,
    )
}

fn view(app: &App) -> Element<'_, Message> {
    let screen = match app.screen {
        Screen::Scan => scan::view(app),
        Screen::Connect => connect::view(app),
        Screen::Install => install::view(app),
    };
    column![
        header(app),
        container(screen).padding(style::PADDING).height(Length::Fill)
    ]
    .into()
}

fn header(app: &App) -> Element<'_, Message> {
    let step = |label: &'static str, screen: Screen| {
        let label = text(label);
        if app.screen == screen {
            label
        } else {
            label.style(text::secondary)
        }
    };
    let version = app
        .version
        .as_ref()
        .map(|v| format!("v{}", v.version))
        .unwrap_or_default();
    container(
        row![
            image(image::Handle::from_bytes(LOGO)).width(32).height(32),
            text("Bobine Deploy").size(18).width(Length::Fill),
            step("1. Machine", Screen::Scan),
            step("2. Connexion", Screen::Connect),
            step("3. Installation", Screen::Install),
            text(version).style(text::secondary),
        ]
        .spacing(style::SPACING * 2)
        .align_y(iced::Alignment::Center),
    )
    .padding([12, style::PADDING])
    .style(container::bordered_box)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use bobine_deploy_proto::{HostKeyInfo, HostKeyStatus};

    use super::*;

    #[test]
    fn editing_the_target_forgets_what_was_learned_about_the_previous_one() {
        let mut app = App::default();
        app.form.host = "ancienne.local".into();
        app.target.host_key = Some(HostKeyInfo {
            host: "ancienne.local".into(),
            port: 22,
            fingerprint: "SHA256:x".into(),
            key_type: "ssh-ed25519".into(),
            status: HostKeyStatus::Known,
            known_hosts: String::new(),
        });
        let _ = update(&mut app, Message::HostChanged("nouvelle.local".into()));
        assert!(app.target.host_key.is_none() && app.target.inspection.is_none());
    }
}
