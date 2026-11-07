#!/usr/bin/env python3
"""Construit les paquets de Bobine Deploy du système courant et les range sous des noms stables.

Usage : bundle.py --out DOSSIER [--version V] [--gpg-sign EMPREINTE]

- version : `--version`, sinon `VERSION` à la racine du dépôt ;
- paquets selon le système : `.deb` et `.rpm` sous Linux, installeur NSIS sous Windows, `.dmg` sous macOS
  (pas d'AppImage : sa construction télécharge des outils non épinglés) ;
- CLI Tauri épinglée (`TAURI_CLI`), même version mineure que la caisse `tauri` du `Cargo.lock` ;
- `--gpg-sign` : signature OpenPGP des `.rpm` (exigée par DNF).
"""
from __future__ import annotations

import argparse
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

TAURI_CLI = "@tauri-apps/cli@2.12.4"
ROOT = Path(__file__).resolve().parents[1]


def host() -> tuple[str, str]:
    """(système, architecture) au sens des noms de paquets : linux/windows/macos, x64/arm64."""
    system = {"Linux": "linux", "Windows": "windows", "Darwin": "macos"}.get(platform.system())
    if system is None:
        raise SystemExit(f"système non pris en charge : {platform.system()}")
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "x64" if machine in ("x86_64", "amd64") else None
    if arch is None:
        raise SystemExit(f"architecture non prise en charge : {machine}")
    return system, arch


def bundles_for(system: str) -> list[str]:
    return {"linux": ["deb", "rpm"], "windows": ["nsis"], "macos": ["app", "dmg"]}[system]


def target_names(system: str, arch: str, version: str) -> dict[str, str]:
    """Extension produite par Tauri -> nom publié."""
    linux_arch = {"x64": ("amd64", "x86_64"), "arm64": ("arm64", "aarch64")}[arch]
    suffix = "-x64" if arch == "x64" else "-arm64"
    return {
        "linux": {
            ".deb": f"bobine-deploy_{version}_{linux_arch[0]}.deb",
            ".rpm": f"bobine-deploy-{version}.{linux_arch[1]}.rpm",
        },
        "windows": {".exe": f"Bobine-Deploy-Setup-{version}{suffix}.exe"},
        "macos": {".dmg": f"Bobine-Deploy-{version}{suffix}.dmg"},
    }[system]


def collect(bundle_dir: Path, names: dict[str, str], out: Path) -> list[Path]:
    """Copie chaque paquet produit sous son nom publié ; exactement un paquet par extension attendue."""
    out.mkdir(parents=True, exist_ok=True)
    written = []
    for extension, name in names.items():
        found = [p for p in bundle_dir.rglob(f"*{extension}") if p.is_file()]
        if len(found) != 1:
            raise SystemExit(f"{extension} : {len(found)} paquet(s) produit(s) dans {bundle_dir}, 1 attendu")
        target = out / name
        shutil.copy2(found[0], target)
        written.append(target)
    return written


def gpg_sign_rpm(paths: list[Path], fingerprint: str) -> None:
    gpg = shutil.which("gpg")
    for path in paths:
        if path.suffix != ".rpm":
            continue
        subprocess.run(["rpmsign", "--define", f"_gpg_name {fingerprint}", "--define", f"_openpgp_sign_id {fingerprint}",
                        "--define", f"__gpg {gpg}", "--addsign", str(path)], check=True)
        check = subprocess.run(["rpmkeys", "--checksig", str(path)], check=True, capture_output=True, text=True).stdout
        if "digests signatures OK" not in check:
            raise SystemExit(f"signature OpenPGP invalide pour {path.name} : {check.strip()}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--version", help="version du paquet (défaut : VERSION)")
    parser.add_argument("--gpg-sign", metavar="EMPREINTE", help="signer les .rpm avec cette clé OpenPGP (déjà importée)")
    args = parser.parse_args(argv)

    version = args.version or (ROOT / "VERSION").read_text(encoding="utf-8").strip()
    system, arch = host()
    bundle_dir = ROOT / "target" / "release" / "bundle"
    shutil.rmtree(bundle_dir, ignore_errors=True)
    npx = shutil.which("npx") or shutil.which("npx.cmd")
    if npx is None:
        raise SystemExit("npx introuvable : Node.js est requis pour la CLI Tauri")

    with tempfile.TemporaryDirectory() as tmp:
        override = Path(tmp) / "version.json"
        override.write_text(json.dumps({"version": version}), encoding="utf-8")
        command = [npx, "--yes", TAURI_CLI, "build", "--bundles", ",".join(bundles_for(system)), "--config", str(override)]
        print("+", " ".join(command), flush=True)
        subprocess.run(command, cwd=ROOT / "src-tauri", check=True)

    written = collect(bundle_dir, target_names(system, arch, version), args.out)
    if args.gpg_sign:
        gpg_sign_rpm(written, args.gpg_sign)
    for path in sorted(args.out.iterdir()):
        print(f"{path.name} ({path.stat().st_size / 1_048_576:.1f} Mio)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
