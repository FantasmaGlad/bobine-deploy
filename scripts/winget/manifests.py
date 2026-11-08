"""Manifestes winget (format 1.12.0) des installeurs NSIS par utilisateur, écrits à partir de SHA256SUMS."""
from __future__ import annotations

import re

REPO = "FantasmaGlad/bobine-deploy"
IDENTIFIER = "FantasmaGlad.BobineDeploy"
MANIFEST_VERSION = "1.12.0"
ARCHES = ("x64", "arm64")
DESCRIPTIONS = {
    "en-US": "Install Bobine on a mini PC of your network over SSH, from a window or the command line.",
    "fr-FR": "Installer Bobine sur un mini PC du réseau par SSH, en fenêtre ou en ligne de commande.",
}


def version_of(tag: str) -> str:
    match = re.fullmatch(r"v(\d+\.\d+\.\d+)", tag)
    if not match:
        raise SystemExit(f"tag vX.Y.Z attendu : {tag}")
    return match.group(1)


def read_sums(text: str) -> dict[str, str]:
    sums = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([0-9a-fA-F]{64}) [ *](\S+)", line.strip())
        if match:
            sums[match.group(2)] = match.group(1).upper()
    return sums


def quoted(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def schema(kind: str) -> str:
    return f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{MANIFEST_VERSION}.schema.json\n\n"


def installers(sums: dict[str, str], tag: str, version: str) -> str:
    lines = []
    for arch in ARCHES:
        name = f"Bobine-Deploy-Setup-{version}-{arch}.exe"
        if name not in sums:
            raise SystemExit(f"installeur absent de SHA256SUMS : {name}")
        lines.append(
            f"  - Architecture: {arch}\n"
            f"    InstallerUrl: https://github.com/{REPO}/releases/download/{tag}/{name}\n"
            f"    InstallerSha256: {sums[name]}\n"
        )
    return "".join(lines)


def locale(locale_name: str, kind: str, tag: str) -> str:
    return (
        f"PackageLocale: {locale_name}\nPublisher: FantasmaGlad\nPublisherUrl: https://github.com/FantasmaGlad\n"
        f"PackageName: Bobine Deploy\nPackageUrl: https://github.com/{REPO}\nLicense: AGPL-3.0-or-later\n"
        f"LicenseUrl: https://github.com/{REPO}/blob/main/LICENSE\n"
        f"ReleaseNotesUrl: https://github.com/{REPO}/releases/tag/{tag}\n"
        f"ShortDescription: {quoted(DESCRIPTIONS[locale_name])}\nManifestType: {kind}\n"
    )


def manifests(sums_text: str, tag: str) -> dict[str, str]:
    version = version_of(tag)
    head = f"PackageIdentifier: {IDENTIFIER}\nPackageVersion: {version}\n"
    foot = f"ManifestVersion: {MANIFEST_VERSION}\n"
    bodies = {
        "version": (f"{IDENTIFIER}.yaml", "DefaultLocale: en-US\nManifestType: version\n"),
        "installer": (
            f"{IDENTIFIER}.installer.yaml",
            "InstallerType: nullsoft\nScope: user\nUpgradeBehavior: install\nInstallers:\n"
            + installers(read_sums(sums_text), tag, version)
            + "ManifestType: installer\n",
        ),
        "defaultLocale": (f"{IDENTIFIER}.locale.en-US.yaml", locale("en-US", "defaultLocale", tag)),
        "locale": (f"{IDENTIFIER}.locale.fr-FR.yaml", locale("fr-FR", "locale", tag)),
    }
    return {name: schema(kind) + head + body + foot for kind, (name, body) in bodies.items()}
