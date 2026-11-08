#!/usr/bin/env python3
"""Construit les paquets de Bobine Deploy pour le système courant, sous des noms stables.

Usage : bundle.py --out DOSSIER [--version V] [--gpg-sign EMPREINTE]

- Linux : `.deb` (cargo deb) et `.rpm` (cargo generate-rpm), `.rpm` signé en OpenPGP avec --gpg-sign ;
- Windows : installeur NSIS par utilisateur (packaging/windows/installer.nsi) ;
- macOS : `Bobine Deploy.app`, signature ad hoc, dans un `.dmg`.
"""
from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "bobine-deploy"
TARGET = ROOT / "target" / "release"
PACKAGING = ROOT / "packaging"


def run(*command: str | Path, cwd: Path = ROOT) -> None:
    print("+", " ".join(str(c) for c in command), flush=True)
    subprocess.run([str(c) for c in command], cwd=cwd, check=True)


def host() -> tuple[str, str]:
    system = {"Linux": "linux", "Windows": "windows", "Darwin": "macos"}.get(platform.system())
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "x64" if machine in ("x86_64", "amd64") else None
    if system is None or arch is None:
        raise SystemExit(f"plateforme non prise en charge : {platform.system()} {machine}")
    return system, arch


def linux(version: str, arch: str, out: Path, gpg: str | None) -> None:
    deb_arch, rpm_arch = {"x64": ("amd64", "x86_64"), "arm64": ("arm64", "aarch64")}[arch]
    deb = out / f"bobine-deploy_{version}_{deb_arch}.deb"
    rpm = out / f"bobine-deploy-{version}.{rpm_arch}.rpm"
    run("cargo", "deb", "--no-build", "--no-strip", "-p", "bobine-deploy", "--deb-version", version, "-o", deb)
    run("cargo", "generate-rpm", "-p", "crates/bobine-deploy", "-s", f"version = \"{version}\"", "-o", rpm)
    if gpg:
        rpmsign = ["rpmsign", "--define", f"_gpg_name {gpg}", "--define", f"_openpgp_sign_id {gpg}"]
        run(*rpmsign, "--define", f"__gpg {shutil.which('gpg')}", "--addsign", rpm)
        check = subprocess.run(["rpmkeys", "--checksig", str(rpm)], capture_output=True, text=True, check=True)
        if "digests signatures OK" not in check.stdout:
            raise SystemExit(f"signature OpenPGP invalide : {check.stdout.strip()}")


def windows(version: str, arch: str, out: Path) -> None:
    makensis = shutil.which("makensis") or r"C:\Program Files (x86)\NSIS\makensis.exe"
    target = out / f"Bobine-Deploy-Setup-{version}-{arch}.exe"
    run(makensis, f"/DVERSION={version}", f"/DBINARY={TARGET / 'bobine-deploy.exe'}", f"/DOUTFILE={target}",
        "installer.nsi", cwd=PACKAGING / "windows")


def macos(version: str, arch: str, out: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        staging = Path(tmp) / "Bobine Deploy"
        app = staging / "Bobine Deploy.app" / "Contents"
        (app / "MacOS").mkdir(parents=True)
        (app / "Resources").mkdir()
        shutil.copy2(TARGET / "bobine-deploy", app / "MacOS" / "bobine-deploy")
        shutil.copy2(CRATE / "assets" / "icons" / "icon.icns", app / "Resources" / "icon.icns")
        plist = (PACKAGING / "macos" / "Info.plist").read_text(encoding="utf-8").replace("@VERSION@", version)
        (app / "Info.plist").write_text(plist, encoding="utf-8")
        run("codesign", "--force", "--deep", "--sign", "-", staging / "Bobine Deploy.app")
        (staging / "Applications").symlink_to("/Applications")
        dmg = out / f"Bobine-Deploy-{version}-{arch}.dmg"
        hdiutil_create(staging, dmg)


def hdiutil_create(source: Path, dmg: Path) -> None:
    """hdiutil échoue parfois sur les runners (« Resource busy », image analysée par le système) : nouvel essai."""
    for attempt in range(1, 4):
        result = subprocess.run(["hdiutil", "create", "-volname", "Bobine Deploy", "-srcfolder", str(source), "-ov",
                                 "-format", "UDZO", str(dmg)])
        if result.returncode == 0:
            return
        time.sleep(10 * attempt)
    raise SystemExit("hdiutil : image disque impossible à créer")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--version", help="version des paquets (défaut : VERSION)")
    parser.add_argument("--gpg-sign", metavar="EMPREINTE", help="signer les .rpm avec cette clé OpenPGP")
    args = parser.parse_args()
    version = args.version or (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    system, arch = host()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    run("cargo", "build", "--release", "--locked", "-p", "bobine-deploy")
    if system == "linux":
        linux(version, arch, out, args.gpg_sign)
    elif system == "windows":
        windows(version, arch, out)
    else:
        macos(version, arch, out)
    for path in sorted(out.iterdir()):
        print(f"{path.name} ({path.stat().st_size / 1_048_576:.1f} Mio)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
