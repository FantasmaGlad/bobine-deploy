<p align="center">
  <img src="crates/bobine-deploy/assets/icons/128x128.png" width="96" height="96" alt="Bobine Deploy">
</p>

<h1 align="center">Bobine Deploy</h1>

<p align="center">
  <b>Installer <a href="https://github.com/FantasmaGlad/Bobine">Bobine</a> sur un mini PC du réseau par SSH.</b>
</p>

<p align="center">
  <a href="https://github.com/FantasmaGlad/bobine-deploy/releases/latest"><b>Releases</b></a> &nbsp;·&nbsp;
  <a href="https://bobine.fit/fr"><b>bobine.fit</b></a> &nbsp;·&nbsp;
  <a href="README.md"><b>English</b></a>
</p>

<p align="center">
  <a href="https://github.com/FantasmaGlad/bobine-deploy/actions/workflows/ci.yml"><img src="https://github.com/FantasmaGlad/bobine-deploy/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://github.com/FantasmaGlad/bobine-deploy/releases/latest"><img src="https://img.shields.io/github/v/release/FantasmaGlad/bobine-deploy?label=release&color=brightgreen" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-blue.svg" alt="AGPL-3.0"></a>
</p>

---

## Téléchargement

| Plateforme | Téléchargement | Prérequis |
| :--- | :--- | :--- |
| <img src="https://raw.githubusercontent.com/FantasmaGlad/Bobine/main/Assets/Icons/os/windows.svg" width="20" height="20" valign="middle" alt="Windows"> **Windows** 10 et 11 | [x64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/Bobine-Deploy-Setup-0.1.0-x64.exe) · [ARM64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/Bobine-Deploy-Setup-0.1.0-arm64.exe) | installation par utilisateur, sans droits administrateur |
| <img src="https://raw.githubusercontent.com/FantasmaGlad/Bobine/main/Assets/Icons/os/apple.svg" width="20" height="20" valign="middle" alt="macOS"> **macOS** | [Apple Silicon](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/Bobine-Deploy-0.1.0-arm64.dmg) · [Intel](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/Bobine-Deploy-0.1.0-x64.dmg) | `.dmg`, ou `brew install --cask fantasmaglad/bobine/bobine-deploy` |
| <img src="https://raw.githubusercontent.com/FantasmaGlad/Bobine/main/Assets/Icons/distros/debian.svg" width="20" height="20" valign="middle" alt="Debian"> **Debian, Ubuntu** | [amd64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/bobine-deploy_0.1.0_amd64.deb) · [arm64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/bobine-deploy_0.1.0_arm64.deb) | glibc 2.39+ |
| <img src="https://raw.githubusercontent.com/FantasmaGlad/Bobine/main/Assets/Icons/distros/fedora.svg" width="20" height="20" valign="middle" alt="Fedora"> **Fedora, RHEL** | [x86_64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/bobine-deploy-0.1.0.x86_64.rpm) · [aarch64](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/bobine-deploy-0.1.0.aarch64.rpm) | glibc 2.39+ |

Empreintes : [SHA256SUMS](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/SHA256SUMS), signées avec la clé de publication de Bobine ([SHA256SUMS.asc](https://github.com/FantasmaGlad/bobine-deploy/releases/download/v0.1.0/SHA256SUMS.asc)).

## Utilisation

```bash
bobine-deploy                              # fenêtre
bobine-deploy scan                         # trouver les mini PC
bobine-deploy install --host bobine.local --yes
bobine-deploy rpc                          # JSON-RPC sur stdio
```

Chaque action de la fenêtre existe en ligne de commande (`--help`, `--json`). Alias court sous Linux et macOS : `bdp`.

---

Licence : [AGPL-3.0](LICENSE) · Sécurité : [docs/fr/SECURITY.md](docs/fr/SECURITY.md) · Contribuer : [docs/fr/CONTRIBUTING.md](docs/fr/CONTRIBUTING.md)

<sub>Dépôts liés : [Bobine](https://github.com/FantasmaGlad/Bobine) (application) · [BobineWeb](https://github.com/FantasmaGlad/BobineWeb) (site) · [bobine-packages](https://github.com/FantasmaGlad/bobine-packages) (dépôts de paquets Linux) · [homebrew-bobine](https://github.com/FantasmaGlad/homebrew-bobine) (tap Homebrew) · Politique de sécurité : [security.bobine.fit](https://security.bobine.fit/fr/), [`/.well-known/security.txt`](https://bobine.fit/.well-known/security.txt)</sub>
