use wgpu::{Backends, DeviceType, Instance, InstanceDescriptor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renderer {
    Gpu { adapter: String, backend: String },
    Cpu { reason: &'static str },
}

pub fn select() -> Renderer {
    let renderer = decide(
        std::env::var("BOBINE_DEPLOY_RENDERER").ok().as_deref(),
        hardware_adapter,
    );
    match &renderer {
        Renderer::Gpu { adapter, backend } => eprintln!("bobine-deploy : rendu GPU ({backend}, {adapter})"),
        Renderer::Cpu { reason } => {
            eprintln!("bobine-deploy : rendu logiciel ({reason})");
            std::env::set_var("ICED_BACKEND", "tiny-skia");
        }
    }
    renderer
}

pub fn relaunch_on_cpu() -> i32 {
    eprintln!("bobine-deploy : le rendu GPU a échoué, relance en rendu logiciel");
    let Ok(exe) = std::env::current_exe() else { return 1 };
    std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env("BOBINE_DEPLOY_RENDERER", "cpu")
        .status()
        .ok()
        .and_then(|status| status.code())
        .unwrap_or(1)
}

fn decide(forced: Option<&str>, probe: impl FnOnce() -> Option<(String, String)>) -> Renderer {
    match forced.map(str::trim) {
        Some("cpu") => Renderer::Cpu {
            reason: "imposé par BOBINE_DEPLOY_RENDERER",
        },
        Some("gpu") => Renderer::Gpu {
            adapter: "imposé".into(),
            backend: "BOBINE_DEPLOY_RENDERER".into(),
        },
        _ => match probe() {
            Some((adapter, backend)) => Renderer::Gpu { adapter, backend },
            None => Renderer::Cpu {
                reason: "aucun adaptateur graphique matériel",
            },
        },
    }
}

fn hardware_adapter() -> Option<(String, String)> {
    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::all(),
        ..InstanceDescriptor::from_env_or_default()
    });
    instance
        .enumerate_adapters(Backends::all())
        .into_iter()
        .map(|adapter| adapter.get_info())
        .find(|info| {
            matches!(
                info.device_type,
                DeviceType::IntegratedGpu | DeviceType::DiscreteGpu | DeviceType::VirtualGpu
            )
        })
        .map(|info| (info.name, format!("{:?}", info.backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu() -> Option<(String, String)> {
        Some(("Carte".into(), "Vulkan".into()))
    }

    #[test]
    fn hardware_adapter_means_gpu_rendering() {
        assert!(matches!(decide(None, gpu), Renderer::Gpu { .. }));
        assert!(matches!(decide(None, || None), Renderer::Cpu { .. }));
    }

    #[test]
    fn the_environment_variable_wins() {
        assert!(matches!(decide(Some("cpu"), gpu), Renderer::Cpu { .. }));
        assert!(matches!(decide(Some("gpu"), || None), Renderer::Gpu { .. }));
        assert!(matches!(decide(Some("autre"), || None), Renderer::Cpu { .. }));
    }
}
